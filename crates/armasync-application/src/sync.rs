// SPDX-License-Identifier: AGPL-3.0-only
//! The sync use cases: [`plan`] a destination against a repository revision and [`apply`] the plan.
//!
//! Safety rules enforced here (ARCHITECTURE, "Gemeinsame Verträge"):
//! * a plan is **revalidated** against the current repository revision and the current local state before
//!   anything is changed; a stale plan is rejected,
//! * files are streamed into a **staging file**, verified (size and hash) while they arrive, and only then
//!   committed atomically; a corrupted, short, cancelled or failed transfer never touches the existing file
//!   (ADR-0007),
//! * deletions run last and only if the plan contains them,
//! * cancellation is checked between files and yields the `Cancelled` state, never success,
//! * every state change is reported as a typed [`OperationEvent`].

use std::io::{self, Seek, SeekFrom, Write};

use armasync_core::{
    ARCHIVE_SUFFIX, ArchiveReader, CancelToken, Digest, EVENT_SCHEMA_VERSION, Error, ErrorKind,
    EventSink, FileEntry, HashAlgorithm, Hasher, JobState, LocalFile, LocalStore, Manifest,
    OperationEvent, OperationId, Phase, PlanOptions, Progress, RelativePath, RepositorySource,
    StagedFile, SyncPlan, SyncStep, plan_sync,
};

/// Progress events during a transfer are emitted at most once per this many bytes (and at the end of each file).
const PROGRESS_STEP: u64 = 256 * 1024;

/// Space kept free on the destination on top of what a run needs (heuristic, MIG-032B).
pub const FREE_SPACE_RESERVE: u64 = 16 * 1024 * 1024;

/// Advance check of the free space (MIG-032B). It walks the plan in its order and assumes that each file is complete
/// before the next starts: a file needs its size, plus the archive for a packed file, on top of the free space at that
/// moment, and keeps [`FREE_SPACE_RESERVE`]; a replaced file gives its old size back when it is committed. Overflow
/// counts as not enough space. This is advisory: other processes may use the space after the check, and a full disk
/// still fails on write.
fn check_free_space(
    plan: &SyncPlan,
    manifest: &Manifest,
    local: &[LocalFile],
    free: u64,
) -> Result<(), Error> {
    let mut available = free;
    for step in &plan.steps {
        let (path, size, old_size) = match step {
            SyncStep::Download { path, size, .. } => (path, *size, 0),
            SyncStep::Replace { path, size, .. } => {
                let old = local
                    .iter()
                    .find(|file| file.path == *path)
                    .map_or(0, |file| file.size);
                (path, *size, old)
            }
            SyncStep::Keep { .. } | SyncStep::Delete { .. } => continue,
        };
        let archive = manifest.archive_size(path).unwrap_or(0);
        let needed = size
            .checked_add(archive)
            .and_then(|total| total.checked_add(FREE_SPACE_RESERVE));
        if needed.is_none_or(|needed| available < needed) {
            return Err(Error::new(
                ErrorKind::FileSystem,
                format!(
                    "not enough free space for {path}: {available} bytes free, a reserve of {FREE_SPACE_RESERVE} bytes is kept"
                ),
            ));
        }
        available = available.saturating_sub(size).saturating_add(old_size);
    }
    Ok(())
}

/// Computes a plan for the current repository revision and local state without changing anything.
///
/// # Errors
/// Errors of the repository source or the local store.
pub fn plan(
    source: &dyn RepositorySource,
    store: &dyn LocalStore,
    options: &PlanOptions,
) -> Result<SyncPlan, Error> {
    plan_cancellable(source, store, options, &CancelToken::new())
}

/// Like [`plan`], but the scan of the local files (which hashes them) can be cancelled.
///
/// # Errors
/// As [`plan`], plus [`ErrorKind::Cancelled`].
pub fn plan_cancellable(
    source: &dyn RepositorySource,
    store: &dyn LocalStore,
    options: &PlanOptions,
    cancel: &CancelToken,
) -> Result<SyncPlan, Error> {
    let manifest = source.manifest()?;
    let local = store.list(algorithm_of(&manifest), cancel)?;
    Ok(plan_sync(&manifest, &local, options))
}

/// The hash algorithm of a manifest (all entries of a revision share one). An empty manifest needs no hashes.
fn algorithm_of(manifest: &Manifest) -> HashAlgorithm {
    manifest
        .entries()
        .first()
        .map_or(HashAlgorithm::Sha256, |entry| entry.hash.algorithm())
}

/// Forwards received bytes to the staging file and the digest, and refuses more than the announced size.
struct Tee<'a> {
    stage: &'a mut dyn StagedFile,
    digest: &'a mut dyn Digest,
    written: u64,
    limit: u64,
    overflow: bool,
}

impl Write for Tee<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.written + data.len() as u64 > self.limit {
            self.overflow = true;
            return Err(io::Error::other("more data than announced"));
        }
        self.stage.write_all(data)?;
        self.digest.update(data);
        self.written += data.len() as u64;
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stage.flush()
    }
}

/// Streams one file into staging, verifies size and hash, and commits it atomically.
///
/// A plain file is streamed from the repository straight into staging. A packed file (`archive_size` is `Some`,
/// ADR-0008) is first downloaded completely as `<path>.zip` into a spool file, then unpacked into staging by
/// `unpacker`; only the unpacked bytes are verified and committed.
///
/// Any failure (network, cancellation, wrong size or hash, damaged archive, full disk, failed commit) drops the
/// staging and spool files and leaves an existing target untouched.
#[allow(clippy::too_many_arguments)]
fn install(
    source: &dyn RepositorySource,
    store: &mut dyn LocalStore,
    hasher: &dyn Hasher,
    unpacker: &dyn ArchiveReader,
    cancel: &CancelToken,
    path: &RelativePath,
    entry: &FileEntry,
    archive_size: Option<u64>,
    on_bytes: &mut dyn FnMut(u64),
) -> Result<(), Error> {
    let Some(archive_size) = archive_size else {
        let mut stage = store.begin(path, entry.size)?;
        let mut digest = hasher.digest(entry.hash.algorithm());
        let (fetched, written, overflow) = {
            let mut tee = Tee {
                stage: &mut *stage,
                digest: &mut *digest,
                written: 0,
                limit: entry.size,
                overflow: false,
            };
            let fetched = source.fetch(path, entry.size, &mut tee, cancel, on_bytes);
            (fetched, tee.written, tee.overflow)
        };
        if let Err(error) = fetched {
            return Err(if overflow {
                integrity(path, "longer than announced")
            } else {
                error
            });
        }
        return verify_and_commit(stage, digest, entry, path, written);
    };

    let archive_path = RelativePath::parse(&format!("{path}{ARCHIVE_SUFFIX}"))
        .map_err(|_| Error::new(ErrorKind::Format, format!("no archive name for {path}")))?;
    let mut spool = store.spool(archive_size)?;
    source.fetch(&archive_path, archive_size, &mut spool, cancel, on_bytes)?;
    spool
        .seek(SeekFrom::Start(0))
        .map_err(|_| Error::new(ErrorKind::FileSystem, "cannot read back the archive"))?;

    let mut stage = store.begin(path, entry.size)?;
    let mut digest = hasher.digest(entry.hash.algorithm());
    let (unpacked, written, overflow) = {
        let mut tee = Tee {
            stage: &mut *stage,
            digest: &mut *digest,
            written: 0,
            limit: entry.size,
            overflow: false,
        };
        let unpacked = unpacker.extract_single(&mut *spool, entry.size, &mut tee, cancel);
        (unpacked, tee.written, tee.overflow)
    };
    if let Err(error) = unpacked {
        return Err(if overflow {
            integrity(path, "longer than announced")
        } else {
            error
        });
    }
    verify_and_commit(stage, digest, entry, path, written)
}

fn integrity(path: &RelativePath, what: &str) -> Error {
    Error::new(
        ErrorKind::Integrity,
        format!("verification failed for {path} ({what}); file left unchanged"),
    )
}

/// The checks shared by both transfer routes: size first, then hash, then the atomic commit.
fn verify_and_commit(
    stage: Box<dyn StagedFile + '_>,
    digest: Box<dyn Digest>,
    entry: &FileEntry,
    path: &RelativePath,
    written: u64,
) -> Result<(), Error> {
    if written != entry.size {
        return Err(integrity(path, "size differs"));
    }
    if digest.finish() != entry.hash {
        return Err(integrity(path, "checksum differs"));
    }
    stage.commit()
}

/// Everything [`apply`] needs from the outside world.
pub struct SyncContext<'a> {
    /// Repository to read from.
    pub source: &'a dyn RepositorySource,
    /// Destination to change.
    pub store: &'a mut dyn LocalStore,
    /// Hash implementation used to verify downloads.
    pub hasher: &'a dyn Hasher,
    /// Unpacks the packed copies the repository publishes (ADR-0008).
    pub unpacker: &'a dyn ArchiveReader,
    /// Receiver of operation events.
    pub sink: &'a mut dyn EventSink,
    /// Cooperative cancellation.
    pub cancel: &'a CancelToken,
}

impl std::fmt::Debug for SyncContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncContext")
            .field("cancelled", &self.cancel.is_cancelled())
            .finish_non_exhaustive()
    }
}

/// Result of a successful [`apply`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyReport {
    /// The operation.
    pub operation: OperationId,
    /// Always [`JobState::Succeeded`] for an `Ok` result.
    pub state: JobState,
    /// Files downloaded.
    pub downloaded: u64,
    /// Files replaced.
    pub replaced: u64,
    /// Files deleted.
    pub deleted: u64,
    /// Files left alone.
    pub kept: u64,
    /// Bytes written.
    pub bytes: u64,
}

struct Reporter<'a> {
    sink: &'a mut dyn EventSink,
    operation: OperationId,
    sequence: u64,
    state: JobState,
}

impl Reporter<'_> {
    fn emit(&mut self, phase: Phase, progress: Progress, error: Option<ErrorKind>) {
        self.sink.emit(OperationEvent {
            schema_version: EVENT_SCHEMA_VERSION,
            operation: self.operation,
            sequence: self.sequence,
            phase,
            state: self.state,
            progress,
            error,
        });
        self.sequence += 1;
    }

    fn enter(&mut self, next: JobState) {
        if let Ok(state) = self.state.transition(next) {
            self.state = state;
        }
    }

    /// Reports the terminal state for `error` and hands the error back.
    fn finish_with(&mut self, progress: Progress, error: Error) -> Error {
        let (state, kind) = match error.kind() {
            ErrorKind::Cancelled => (JobState::Cancelled, ErrorKind::Cancelled),
            other => (JobState::Failed, other),
        };
        self.enter(state);
        self.emit(Phase::Finished, progress, Some(kind));
        error
    }
}

/// Applies `plan` after revalidating it.
///
/// `options` must be the options the plan was created with; they are needed to recompute the plan.
///
/// # Errors
/// * [`ErrorKind::Cancelled`] if cancellation was requested (files already completed stay in place, no partial file),
/// * [`ErrorKind::Integrity`] if the plan is stale or a download fails verification,
/// * source/store errors otherwise.
pub fn apply(
    operation: OperationId,
    plan: &SyncPlan,
    options: &PlanOptions,
    context: &mut SyncContext<'_>,
) -> Result<ApplyReport, Error> {
    let mut reporter = Reporter {
        sink: &mut *context.sink,
        operation,
        sequence: 0,
        state: JobState::Queued,
    };
    let mut progress = Progress::default();
    reporter.emit(Phase::Planning, progress, None);

    if context.cancel.is_cancelled() {
        return Err(reporter.finish_with(
            progress,
            Error::new(ErrorKind::Cancelled, "cancelled before start"),
        ));
    }
    reporter.enter(JobState::Running);
    reporter.emit(Phase::Planning, progress, None);

    // Revalidate against the current repository revision and the current local state.
    let manifest = match context.source.manifest() {
        Ok(manifest) => manifest,
        Err(error) => return Err(reporter.finish_with(progress, error)),
    };
    let local = match context.store.list(algorithm_of(&manifest), context.cancel) {
        Ok(local) => local,
        Err(error) => return Err(reporter.finish_with(progress, error)),
    };
    if plan_sync(&manifest, &local, options) != *plan {
        let error = Error::new(
            ErrorKind::Integrity,
            "plan is stale: repository revision or local files changed since planning",
        );
        return Err(reporter.finish_with(progress, error));
    }

    // Advance check (MIG-032B) before anything is downloaded or changed. Unknown free space skips it.
    match context.store.free_space() {
        Ok(Some(free)) => {
            if let Err(error) = check_free_space(plan, &manifest, &local, free) {
                return Err(reporter.finish_with(progress, error));
            }
        }
        Ok(None) => {}
        Err(error) => return Err(reporter.finish_with(progress, error)),
    }

    for step in &plan.steps {
        match step {
            SyncStep::Download { path, size, .. } | SyncStep::Replace { path, size, .. } => {
                progress.files_total += 1;
                // A packed file transfers its archive, which is what the progress counts.
                progress.bytes_total += manifest.archive_size(path).unwrap_or(*size);
            }
            SyncStep::Delete { .. } => progress.files_total += 1,
            SyncStep::Keep { .. } => {}
        }
    }
    let mut report = ApplyReport {
        operation,
        state: JobState::Succeeded,
        downloaded: 0,
        replaced: 0,
        deleted: 0,
        kept: plan.summary.keep_files,
        bytes: 0,
    };

    reporter.emit(Phase::Transferring, progress, None);
    for step in &plan.steps {
        let (path, is_replace) = match step {
            SyncStep::Download { path, .. } => (path, false),
            SyncStep::Replace { path, .. } => (path, true),
            SyncStep::Keep { .. } | SyncStep::Delete { .. } => continue,
        };
        if context.cancel.is_cancelled() {
            return Err(
                reporter.finish_with(progress, Error::new(ErrorKind::Cancelled, "cancelled"))
            );
        }
        let Some(entry) = manifest.get(path) else {
            let error = Error::new(
                ErrorKind::Integrity,
                format!("{path} vanished from the manifest"),
            );
            return Err(reporter.finish_with(progress, error));
        };
        let mut file_bytes: u64 = 0;
        let mut since_emit: u64 = 0;
        let installed = install(
            context.source,
            &mut *context.store,
            context.hasher,
            context.unpacker,
            context.cancel,
            path,
            entry,
            manifest.archive_size(path),
            &mut |bytes| {
                file_bytes += bytes;
                since_emit += bytes;
                progress.bytes_done += bytes;
                if since_emit >= PROGRESS_STEP {
                    since_emit = 0;
                    reporter.emit(Phase::Transferring, progress, None);
                }
            },
        );
        if let Err(error) = installed {
            // Bytes of the unfinished file do not count as completed.
            progress.bytes_done -= file_bytes;
            return Err(reporter.finish_with(progress, error));
        }
        progress.files_done += 1;
        report.bytes += entry.size;
        if is_replace {
            report.replaced += 1;
        } else {
            report.downloaded += 1;
        }
        reporter.emit(Phase::Transferring, progress, None);
    }

    for step in &plan.steps {
        let SyncStep::Delete { path } = step else {
            continue;
        };
        if context.cancel.is_cancelled() {
            return Err(
                reporter.finish_with(progress, Error::new(ErrorKind::Cancelled, "cancelled"))
            );
        }
        if let Err(error) = context.store.delete(path) {
            return Err(reporter.finish_with(progress, error));
        }
        progress.files_done += 1;
        report.deleted += 1;
        reporter.emit(Phase::Cleaning, progress, None);
    }

    reporter.enter(JobState::Succeeded);
    reporter.emit(Phase::Finished, progress, None);
    Ok(report)
}
