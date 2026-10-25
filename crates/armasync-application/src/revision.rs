// SPDX-License-Identifier: AGPL-3.0-only
//! Revisions and their publication (MIG-042, step 2; ADR-0012).
//!
//! A revision is the complete file set of a local folder, identified by its content. [`publish`] uploads the files
//! into `revisions/<id>/files/`, writes `revisions/<id>/manifest.json` **last** (its presence marks the revision as
//! complete), and only then replaces `current.json`. The previous pointer is copied to `history/` before it is
//! replaced. Nothing is deleted. WebDAV does not guarantee an atomic replacement, so none is claimed: readers verify
//! the pointer through its `manifest_sha256` and the manifest through the file hashes.

use std::io::Cursor;

use armasync_core::exclusion::ExclusionRules;
use armasync_core::{
    CancelToken, Error, ErrorKind, HashAlgorithm, Hasher, LocalSource, OpenedSource, RelativePath,
    RemoteTarget, SourceFile,
};

use crate::exclusion::{Exclusion, partition};
use crate::progress::UploadObserver;
use crate::upload;

/// Schema name of the revision manifest.
pub const REVISION_SCHEMA: &str = "armasync.revision";
/// Schema name of the publication pointer.
pub const CURRENT_SCHEMA: &str = "armasync.current";
/// Largest manifest [`publish`] reads back to compare it.
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
/// Largest pointer or history copy [`publish`] reads.
const MAX_POINTER_BYTES: u64 = 4096;

/// A revision: its identifier, the exact manifest bytes and the SHA-256 of those bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    /// `r-` plus 32 hexadecimal digits of the SHA-256 over the canonical file listing.
    pub id: String,
    /// The manifest as published (UTF-8 JSON).
    pub manifest: Vec<u8>,
    /// SHA-256 of `manifest`, lower-case hexadecimal.
    pub manifest_sha256: String,
    /// The files, sorted by path.
    pub files: Vec<SourceFile>,
}

impl Revision {
    /// Builds the revision for `files`. The same content always gives the same identifier and manifest.
    ///
    /// # Errors
    /// Never for valid input; the result type keeps the signature stable for future checks.
    pub fn build(files: &[SourceFile], hasher: &dyn Hasher) -> Result<Self, Error> {
        let mut files = files.to_vec();
        files.sort_by(|a, b| a.path.cmp(&b.path));

        let mut listing = String::new();
        for file in &files {
            listing.push_str(&format!(
                "{}\t{}\t{}\n",
                file.path,
                file.size,
                file.hash.hex()
            ));
        }
        let digest = hasher.hash(HashAlgorithm::Sha256, listing.as_bytes());
        let id = format!("r-{}", &digest.hex()[..32]);

        let mut manifest = format!(
            "{{\"schema\":\"{REVISION_SCHEMA}\",\"version\":1,\"revision\":\"{id}\",\"files\":["
        );
        for (index, file) in files.iter().enumerate() {
            if index > 0 {
                manifest.push(',');
            }
            manifest.push_str(&format!(
                "{{\"path\":{},\"size\":{},\"sha256\":\"{}\"}}",
                json_string(file.path.as_str()),
                file.size,
                file.hash.hex()
            ));
        }
        manifest.push_str("]}");
        let manifest_sha256 = hasher
            .hash(HashAlgorithm::Sha256, manifest.as_bytes())
            .hex()
            .to_owned();
        Ok(Self {
            id,
            manifest: manifest.into_bytes(),
            manifest_sha256,
            files,
        })
    }

    /// The publication pointer that names this revision.
    #[must_use]
    pub fn current_pointer(&self) -> Vec<u8> {
        format!(
            "{{\"schema\":\"{CURRENT_SCHEMA}\",\"version\":1,\"revision\":\"{}\",\"manifest_sha256\":\"{}\"}}",
            self.id, self.manifest_sha256
        )
        .into_bytes()
    }
}

/// What a publication did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishReport {
    /// The revision that is (now) published.
    pub revision: String,
    /// Whether the revision was complete on the target already, so no file was uploaded.
    pub revision_was_complete: bool,
    /// Files uploaded for this revision (created and replaced).
    pub files_uploaded: u64,
    /// Whether `current.json` was replaced by this run.
    pub pointer_changed: bool,
    /// Files of the source that are not part of the revision, with the reason (ADR-0013).
    pub exclusions: Vec<Exclusion>,
    /// Exclusion rules that match no file of the source.
    pub unused_rules: Vec<RelativePath>,
}

/// Publishes the content of `source` as a revision (ADR-0012).
///
/// # Errors
/// [`ErrorKind::Cancelled`] when cancelled; [`ErrorKind::Integrity`] when a complete-looking revision holds different
/// content or the published pointer cannot be verified; transport and file-system errors from the upload otherwise.
/// Until the pointer is written the previous publication stays as it was.
pub fn publish(
    source: &dyn LocalSource,
    target: &dyn RemoteTarget,
    hasher: &dyn Hasher,
    cancel: &CancelToken,
) -> Result<PublishReport, Error> {
    publish_with_rules(source, target, hasher, &ExclusionRules::default(), cancel)
}

/// Publishes the content of `source` without the excluded files (ADR-0013): internal and partial files are never part
/// of a revision, and the files matched by `rules` are left out.
///
/// # Errors
/// As [`publish`].
pub fn publish_with_rules(
    source: &dyn LocalSource,
    target: &dyn RemoteTarget,
    hasher: &dyn Hasher,
    rules: &ExclusionRules,
    cancel: &CancelToken,
) -> Result<PublishReport, Error> {
    publish_inner(source, target, hasher, rules, cancel, None)
}

/// As [`publish_with_rules`], and reports the publication to `observer` (ADR-0016). Exactly one terminal event ends
/// the events: `Finished` on success, `Cancelled` on cancellation, `Failed` with the error category otherwise.
///
/// # Errors
/// As [`publish_with_rules`].
pub fn publish_observed(
    source: &dyn LocalSource,
    target: &dyn RemoteTarget,
    hasher: &dyn Hasher,
    rules: &ExclusionRules,
    cancel: &CancelToken,
    observer: &UploadObserver,
) -> Result<PublishReport, Error> {
    let result = publish_inner(source, target, hasher, rules, cancel, Some(observer));
    observer.end(&result);
    result
}

fn publish_inner(
    source: &dyn LocalSource,
    target: &dyn RemoteTarget,
    hasher: &dyn Hasher,
    rules: &ExclusionRules,
    cancel: &CancelToken,
    observer: Option<&UploadObserver>,
) -> Result<PublishReport, Error> {
    let split = partition(source, rules, cancel)?;
    let revision = Revision::build(&split.included, hasher)?;
    let directory = format!("revisions/{}", revision.id);
    let manifest_path = parse(&format!("{directory}/manifest.json"))?;
    let mut report = PublishReport {
        revision: revision.id.clone(),
        revision_was_complete: false,
        files_uploaded: 0,
        pointer_changed: false,
        exclusions: split.excluded.clone(),
        unused_rules: split.unused_rules.clone(),
    };

    match target.fetch(&manifest_path, MAX_MANIFEST_BYTES)? {
        Some(existing) if existing == revision.manifest => report.revision_was_complete = true,
        Some(_) => {
            return Err(Error::new(
                ErrorKind::Integrity,
                format!(
                    "revision {} exists on the target with different content; nothing was changed",
                    revision.id
                ),
            ));
        }
        None => {}
    }

    if !report.revision_was_complete {
        ensure_folder(target, "revisions")?;
        ensure_folder(target, &directory)?;
        let files_directory = format!("{directory}/files");
        ensure_folder(target, &files_directory)?;
        let scoped = PrefixedTarget {
            inner: target,
            prefix: format!("{files_directory}/"),
        };
        let plan = upload::plan_included(
            &split.included,
            split.excluded.clone(),
            split.unused_rules.clone(),
            &scoped,
            cancel,
        )?;
        if let Some(observer) = observer {
            observer.start(plan.files, plan.bytes);
        }
        let uploaded = upload::apply_observed(&plan, source, &scoped, cancel, observer)?;
        report.files_uploaded = uploaded.files_created + uploaded.files_replaced;
        check_cancel(cancel)?;
    } else if let Some(observer) = observer {
        observer.start(0, 0);
    }
    // Every file is read back and checked before anything that names the revision is written (step 3a, ADR-0012).
    verify_files(target, &directory, &revision.files, hasher, cancel)?;
    if !report.revision_was_complete {
        // The manifest comes last: its presence marks the revision as complete.
        upload_bytes(target, &manifest_path, &revision.manifest, cancel)?;
        verify_manifest(target, &manifest_path, &revision.manifest)?;
    }

    publish_pointer(
        target,
        &revision.current_pointer(),
        hasher,
        cancel,
        &mut report,
    )?;
    Ok(report)
}

/// Reads every file of the revision back from the target and checks its size and SHA-256 against the manifest.
///
/// Nothing is written by this function. Any missing, shortened, lengthened or altered file stops the publication with
/// [`ErrorKind::Integrity`]; a failed read is reported as the transport error it is.
fn verify_files(
    target: &dyn RemoteTarget,
    directory: &str,
    files: &[SourceFile],
    hasher: &dyn Hasher,
    cancel: &CancelToken,
) -> Result<(), Error> {
    for file in files {
        check_cancel(cancel)?;
        let path = parse(&format!("{directory}/files/{}", file.path))?;
        let bytes = match target.fetch(&path, file.size) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => {
                return Err(Error::new(
                    ErrorKind::Integrity,
                    format!(
                        "'{}' is missing on the target; the pointer was not changed",
                        file.path
                    ),
                ));
            }
            // The read is limited to the announced size: a longer file is refused by the adapter.
            Err(error) if error.kind() == ErrorKind::Format => {
                return Err(Error::new(
                    ErrorKind::Integrity,
                    format!(
                        "'{}' differs in size on the target; the pointer was not changed",
                        file.path
                    ),
                ));
            }
            Err(error) => return Err(error),
        };
        if bytes.len() as u64 != file.size {
            return Err(Error::new(
                ErrorKind::Integrity,
                format!(
                    "'{}' differs in size on the target; the pointer was not changed",
                    file.path
                ),
            ));
        }
        if hasher.hash(HashAlgorithm::Sha256, &bytes) != file.hash {
            return Err(Error::new(
                ErrorKind::Integrity,
                format!(
                    "'{}' differs in content on the target; the pointer was not changed",
                    file.path
                ),
            ));
        }
    }
    Ok(())
}

/// Reads the manifest back and compares it with the bytes that were written.
fn verify_manifest(
    target: &dyn RemoteTarget,
    path: &RelativePath,
    manifest: &[u8],
) -> Result<(), Error> {
    match target.fetch(path, MAX_MANIFEST_BYTES)? {
        Some(stored) if stored == manifest => Ok(()),
        _ => Err(Error::new(
            ErrorKind::Integrity,
            "the revision manifest could not be verified on the target; the pointer was not changed",
        )),
    }
}

/// Replaces `current.json` with `pointer`, keeping the previous pointer in `history/` first.
fn publish_pointer(
    target: &dyn RemoteTarget,
    pointer: &[u8],
    hasher: &dyn Hasher,
    cancel: &CancelToken,
    report: &mut PublishReport,
) -> Result<(), Error> {
    let current = parse("current.json")?;
    let previous = target.fetch(&current, MAX_POINTER_BYTES)?;
    if previous.as_deref() == Some(pointer) {
        return Ok(());
    }
    if let Some(old) = &previous {
        ensure_folder(target, "history")?;
        let key = hasher.hash(HashAlgorithm::Sha256, old).hex().to_owned();
        let copy = parse(&format!("history/{key}.json"))?;
        if target.fetch(&copy, MAX_POINTER_BYTES)?.is_none() {
            upload_bytes(target, &copy, old, cancel)?;
        }
    }
    check_cancel(cancel)?;
    match upload_bytes(target, &current, pointer, cancel) {
        Ok(()) => {}
        Err(error) => {
            // The write may have landed although the request failed: look at what is there now.
            let now = target.fetch(&current, MAX_POINTER_BYTES)?;
            if now.as_deref() == Some(pointer) {
                report.pointer_changed = true;
                return Ok(());
            }
            if now == previous {
                return Err(error);
            }
            return Err(Error::new(
                ErrorKind::Integrity,
                "the publication state is unknown; the previous pointer is kept in history/",
            ));
        }
    }
    let stored = target.fetch(&current, MAX_POINTER_BYTES)?;
    if stored.as_deref() != Some(pointer) {
        return Err(Error::new(
            ErrorKind::Integrity,
            "the published pointer could not be verified",
        ));
    }
    report.pointer_changed = true;
    Ok(())
}

/// Writes `bytes` to `path` as one upload.
fn upload_bytes(
    target: &dyn RemoteTarget,
    path: &RelativePath,
    bytes: &[u8],
    cancel: &CancelToken,
) -> Result<(), Error> {
    let source = OpenedSource {
        size: bytes.len() as u64,
        content: Box::new(Cursor::new(bytes.to_vec())),
    };
    target.upload(path, source, cancel)
}

fn ensure_folder(target: &dyn RemoteTarget, path: &str) -> Result<(), Error> {
    let path = parse(path)?;
    if !target.exists(&path)? {
        target.make_folder(&path)?;
    }
    Ok(())
}

fn check_cancel(cancel: &CancelToken) -> Result<(), Error> {
    if cancel.is_cancelled() {
        Err(Error::new(
            ErrorKind::Cancelled,
            "the publication was cancelled",
        ))
    } else {
        Ok(())
    }
}

fn parse(path: &str) -> Result<RelativePath, Error> {
    RelativePath::parse(path)
        .map_err(|_| Error::new(ErrorKind::Usage, "an internal publication path is unusable"))
}

/// Presents a target below a fixed folder prefix, so the revision files land in `revisions/<id>/files/`.
struct PrefixedTarget<'a> {
    inner: &'a dyn RemoteTarget,
    prefix: String,
}

impl PrefixedTarget<'_> {
    fn at(&self, path: &RelativePath) -> Result<RelativePath, Error> {
        parse(&format!("{}{}", self.prefix, path.as_str()))
    }
}

impl RemoteTarget for PrefixedTarget<'_> {
    fn exists(&self, path: &RelativePath) -> Result<bool, Error> {
        self.inner.exists(&self.at(path)?)
    }

    fn make_folder(&self, path: &RelativePath) -> Result<(), Error> {
        self.inner.make_folder(&self.at(path)?)
    }

    fn upload(
        &self,
        path: &RelativePath,
        source: OpenedSource,
        cancel: &CancelToken,
    ) -> Result<(), Error> {
        self.inner.upload(&self.at(path)?, source, cancel)
    }

    fn fetch(&self, path: &RelativePath, limit: u64) -> Result<Option<Vec<u8>>, Error> {
        self.inner.fetch(&self.at(path)?, limit)
    }
}

/// A JSON string literal: quotes, backslashes and control characters are escaped.
fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
