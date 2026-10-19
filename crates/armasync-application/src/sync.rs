// SPDX-License-Identifier: AGPL-3.0-only
//! The sync use cases: [`plan`] a destination against a repository revision and [`apply`] the plan.
//!
//! Safety rules enforced here (ARCHITECTURE, "Gemeinsame Verträge"):
//! * a plan is **revalidated** against the current repository revision and the current local state before
//!   anything is changed; a stale plan is rejected,
//! * downloaded bytes are verified (size and hash) **before** they are written, so a corrupted transfer never
//!   replaces a good file,
//! * deletions run last and only if the plan contains them,
//! * cancellation is checked between files and yields the `Cancelled` state, never success,
//! * every state change is reported as a typed [`OperationEvent`].

use armasync_core::{
    CancelToken, EVENT_SCHEMA_VERSION, Error, ErrorKind, EventSink, Hasher, JobState, LocalStore,
    OperationEvent, OperationId, Phase, PlanOptions, Progress, RepositorySource, SyncPlan,
    SyncStep, plan_sync,
};

/// Computes a plan for the current repository revision and local state without changing anything.
///
/// # Errors
/// Errors of the repository source or the local store.
pub fn plan(
    source: &dyn RepositorySource,
    store: &dyn LocalStore,
    options: &PlanOptions,
) -> Result<SyncPlan, Error> {
    let manifest = source.manifest()?;
    let local = store.list()?;
    Ok(plan_sync(&manifest, &local, options))
}

/// Everything [`apply`] needs from the outside world.
pub struct SyncContext<'a> {
    /// Repository to read from.
    pub source: &'a dyn RepositorySource,
    /// Destination to change.
    pub store: &'a mut dyn LocalStore,
    /// Hash implementation used to verify downloads.
    pub hasher: &'a dyn Hasher,
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
    let local = match context.store.list() {
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

    for step in &plan.steps {
        match step {
            SyncStep::Download { size, .. } | SyncStep::Replace { size, .. } => {
                progress.files_total += 1;
                progress.bytes_total += size;
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
        let data = match context.source.read(path) {
            Ok(data) => data,
            Err(error) => return Err(reporter.finish_with(progress, error)),
        };
        if data.len() as u64 != entry.size
            || context.hasher.hash(entry.hash.algorithm(), &data) != entry.hash
        {
            let error = Error::new(
                ErrorKind::Integrity,
                format!("verification failed for {path}; file left unchanged"),
            );
            return Err(reporter.finish_with(progress, error));
        }
        if let Err(error) = context.store.write(path, &data) {
            return Err(reporter.finish_with(progress, error));
        }
        progress.files_done += 1;
        progress.bytes_done += entry.size;
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
