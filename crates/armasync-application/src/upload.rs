// SPDX-License-Identifier: AGPL-3.0-only
//! Upload of a local folder into a folder tree on a remote target (MIG-042, step 1; ADR-0011).
//!
//! [`plan`] lists the source and asks the target what exists; it changes nothing. [`apply`] executes the plan step
//! by step, checks cancellation before every step and during every upload, and reports what it did. Nothing is ever
//! deleted, and paths are always [`RelativePath`]s.

use std::collections::BTreeSet;

use armasync_core::{CancelToken, Error, ErrorKind, LocalSource, RelativePath, RemoteTarget};

/// One step of an upload plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadStep {
    /// Create one folder that does not exist on the target yet.
    MakeFolder {
        /// Folder path on the target.
        path: RelativePath,
    },
    /// Upload one file.
    Upload {
        /// File path on the target and in the source.
        path: RelativePath,
        /// Size announced by the plan; apply refuses a file whose size changed.
        size: u64,
        /// Whether a file already exists at the target (it is replaced).
        replace: bool,
    },
}

/// What an upload would do. Computing it changes nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadPlan {
    /// Folders first (by depth, then by path), then files (by path).
    pub steps: Vec<UploadStep>,
    /// Folders to create.
    pub folders: u64,
    /// Files to upload.
    pub files: u64,
    /// Files among `files` that replace an existing target file.
    pub replacements: u64,
    /// Total bytes to send.
    pub bytes: u64,
}

/// What an upload did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UploadReport {
    /// Folders created on the target.
    pub folders_created: u64,
    /// Files created on the target.
    pub files_created: u64,
    /// Files that replaced an existing target file.
    pub files_replaced: u64,
    /// Bytes sent.
    pub bytes: u64,
}

/// Computes the upload plan for everything the source lists.
///
/// # Errors
/// [`ErrorKind::Cancelled`] if `cancel` was requested; source and target errors otherwise.
pub fn plan(
    source: &dyn LocalSource,
    target: &dyn RemoteTarget,
    cancel: &CancelToken,
) -> Result<UploadPlan, Error> {
    let files = source.files(cancel)?;
    let mut folder_paths: BTreeSet<String> = BTreeSet::new();
    for file in &files {
        folder_paths.extend(file.path.ancestors().map(str::to_owned));
    }
    let mut folders: Vec<String> = folder_paths.into_iter().collect();
    folders.sort_by_key(|path| (path.matches('/').count(), path.clone()));

    let mut steps = Vec::new();
    for folder in folders {
        check_cancel(cancel)?;
        let path = parse(&folder)?;
        if !target.exists(&path)? {
            steps.push(UploadStep::MakeFolder { path });
        }
    }
    for file in files {
        check_cancel(cancel)?;
        let replace = target.exists(&file.path)?;
        steps.push(UploadStep::Upload {
            path: file.path,
            size: file.size,
            replace,
        });
    }
    Ok(summarize(steps))
}

fn summarize(steps: Vec<UploadStep>) -> UploadPlan {
    let (mut folders, mut files, mut replacements, mut bytes) = (0, 0, 0, 0);
    for step in &steps {
        match step {
            UploadStep::MakeFolder { .. } => folders += 1,
            UploadStep::Upload { size, replace, .. } => {
                files += 1;
                bytes += size;
                if *replace {
                    replacements += 1;
                }
            }
        }
    }
    UploadPlan {
        steps,
        folders,
        files,
        replacements,
        bytes,
    }
}

/// Executes `plan`.
///
/// Cancellation is checked before every step and while a file is streamed. A failure after cancellation is reported
/// as [`ErrorKind::Cancelled`]. A file whose size differs from the plan is refused before anything is sent.
///
/// # Errors
/// [`ErrorKind::Cancelled`]; [`ErrorKind::Integrity`] when a source file changed since the plan; source and target
/// errors otherwise. Steps completed before an error stay in place.
pub fn apply(
    plan: &UploadPlan,
    source: &dyn LocalSource,
    target: &dyn RemoteTarget,
    cancel: &CancelToken,
) -> Result<UploadReport, Error> {
    let mut report = UploadReport::default();
    for step in &plan.steps {
        check_cancel(cancel)?;
        match step {
            UploadStep::MakeFolder { path } => {
                target
                    .make_folder(path)
                    .map_err(|error| stopped(cancel, error))?;
                report.folders_created += 1;
            }
            UploadStep::Upload {
                path,
                size,
                replace,
            } => {
                let opened = source.open(path).map_err(|error| stopped(cancel, error))?;
                if opened.size != *size {
                    return Err(Error::new(
                        ErrorKind::Integrity,
                        format!("'{path}' changed since the plan was made; create a new plan"),
                    ));
                }
                target
                    .upload(path, opened, cancel)
                    .map_err(|error| stopped(cancel, error))?;
                if *replace {
                    report.files_replaced += 1;
                } else {
                    report.files_created += 1;
                }
                report.bytes += size;
            }
        }
    }
    Ok(report)
}

fn check_cancel(cancel: &CancelToken) -> Result<(), Error> {
    if cancel.is_cancelled() {
        Err(Error::new(ErrorKind::Cancelled, "the upload was cancelled"))
    } else {
        Ok(())
    }
}

/// A failure after a cancellation request is reported as the cancellation.
fn stopped(cancel: &CancelToken, error: Error) -> Error {
    if cancel.is_cancelled() {
        Error::new(ErrorKind::Cancelled, "the upload was cancelled")
    } else {
        error
    }
}

fn parse(path: &str) -> Result<RelativePath, Error> {
    RelativePath::parse(path).map_err(|_| {
        Error::new(
            ErrorKind::Usage,
            "a folder of the source has an unusable name",
        )
    })
}
