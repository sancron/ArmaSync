// SPDX-License-Identifier: AGPL-3.0-only
//! View-model of the desktop GUI: all state and behaviour without any drawing.
//!
//! Everything the prototype shows (repository list with filter/sort/selection, the "add repository" dialog with
//! per-field errors, a sync job with progress/cancel, an error summary) lives here and is unit-tested without a
//! display. The job runs on a worker thread that uses the same `armasync_application::sync` use case as the CLI;
//! the UI thread only polls events, so it never blocks on I/O. The data source is the in-memory adapter plus a
//! small artificial delay per file: there is no real transport yet, and the GUI says so.

use std::fmt;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use armasync_adapters::archive::ZipUnpacker;
use armasync_adapters::{InMemoryRepository, InMemoryStore, Sha256Hasher};
use armasync_application::sync::{ApplyReport, SyncContext, apply, plan};
use armasync_core::{
    CancelToken, Error, ErrorKind, EventSink, JobState, Manifest, OperationEvent, OperationId,
    Phase, PlanOptions, Progress, RelativePath, RepositoryId, RepositorySource,
};

/// German UI texts in one place (translations later).
pub mod text {
    /// Name error: empty.
    pub const NAME_REQUIRED: &str = "Bitte einen Namen eingeben.";
    /// Name error: too long.
    pub const NAME_TOO_LONG: &str = "Der Name darf höchstens 80 Zeichen lang sein.";
    /// Name error: duplicate.
    pub const NAME_TAKEN: &str = "Ein Repository mit diesem Namen existiert bereits.";
    /// URL error: empty.
    pub const URL_REQUIRED: &str = "Bitte eine Adresse eingeben.";
    /// URL error: scheme.
    pub const URL_SCHEME: &str =
        "Die Adresse muss mit https://, http://, ftps://, ftp:// oder sftp:// beginnen.";
    /// URL error: credentials inside the URL.
    pub const URL_CREDENTIALS: &str =
        "Benutzername und Kennwort gehören in die eigenen Felder, nicht in die Adresse.";
    /// URL error: whitespace or missing host.
    pub const URL_INVALID: &str = "Die Adresse ist unvollständig oder enthält Leerzeichen.";
}

/// Status of a repository as shown in the list (always rendered as text, never by colour alone).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepoStatus {
    /// Local files match the repository.
    UpToDate,
    /// A newer revision is available.
    Outdated,
    /// The server cannot be reached.
    Offline,
    /// Credentials are needed.
    AuthRequired,
}

impl RepoStatus {
    /// Label for display and screen readers.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::UpToDate => "Aktuell",
            Self::Outdated => "Update verfügbar",
            Self::Offline => "Nicht erreichbar",
            Self::AuthRequired => "Anmeldung erforderlich",
        }
    }
}

/// One row of the repository list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRow {
    /// Stable id.
    pub id: RepositoryId,
    /// Display name.
    pub name: String,
    /// Address without credentials.
    pub url: String,
    /// Status.
    pub status: RepoStatus,
    /// Number of files in the repository.
    pub files: u32,
    /// Simulates a server that delivers a damaged file (prototype error path).
    pub damaged_transfer: bool,
}

/// Deterministic synthetic repositories with long names (the ADR-0002 prototype list).
#[must_use]
pub fn synthetic_repositories(count: usize) -> Vec<RepositoryRow> {
    (0..count)
        .map(|index| {
            let status = match index % 9 {
                0..=2 => RepoStatus::UpToDate,
                3..=6 => RepoStatus::Outdated,
                7 => RepoStatus::Offline,
                _ => RepoStatus::AuthRequired,
            };
            let long = if index % 50 == 0 {
                " – Community Modpack mit sehr langem Anzeigenamen für Layouttests"
            } else {
                ""
            };
            RepositoryRow {
                id: repository_id(index),
                name: format!("Repository {index:05}{long}"),
                url: format!(
                    "https://repo{:03}.example.invalid/arma3/sync-{index}",
                    index % 997
                ),
                status,
                files: 3 + (index % 7) as u32 * 2,
                damaged_transfer: status == RepoStatus::Outdated && index % 28 == 3,
            }
        })
        .collect()
}

#[allow(clippy::expect_used)] // invariant: "repo-" followed by digits is always a valid RepositoryId
fn repository_id(index: usize) -> RepositoryId {
    RepositoryId::parse(&format!("repo-{index}")).expect("generated ids are valid")
}

/// Sort column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    /// By name.
    Name,
    /// By status.
    Status,
    /// By file count.
    Files,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending.
    Ascending,
    /// Descending.
    Descending,
}

/// A password that never appears in `Debug` output, logs or error messages.
#[derive(Clone, Default)]
pub struct Secret(String);

impl Secret {
    /// Mutable access for the text edit.
    pub fn buffer_mut(&mut self) -> &mut String {
        &mut self.0
    }

    /// Whether nothing was entered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// Field values of the "add repository" dialog.
#[derive(Debug, Clone, Default)]
pub struct AddDialog {
    /// Display name.
    pub name: String,
    /// Address (no credentials).
    pub url: String,
    /// Optional user name.
    pub username: String,
    /// Optional password (kept out of every log).
    pub password: Secret,
}

/// Validation messages shown next to the fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldErrors {
    /// Message for the name field.
    pub name: Option<&'static str>,
    /// Message for the address field.
    pub url: Option<&'static str>,
}

impl FieldErrors {
    /// Whether there is nothing to fix.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.url.is_none()
    }
}

/// Entry of the error summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorItem {
    /// Affected repository.
    pub repository: String,
    /// Category.
    pub kind: ErrorKind,
    /// What happened (no secrets).
    pub message: String,
    /// What the user can do next.
    pub hint: &'static str,
}

fn hint_for(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Network => "Verbindung prüfen und erneut versuchen.",
        ErrorKind::Integrity => {
            "Die Datei wurde nicht übernommen. Später erneut versuchen oder den Betreiber informieren."
        }
        ErrorKind::FileSystem => "Speicherplatz und Schreibrechte des Zielordners prüfen.",
        ErrorKind::Config => "Anmeldedaten in den Repository-Einstellungen ergänzen.",
        ErrorKind::Cancelled => {
            "Die Synchronisation wurde abgebrochen; bereits übertragene Dateien sind vollständig."
        }
        ErrorKind::Usage | ErrorKind::Format | ErrorKind::Unsupported => {
            "Details ansehen und die Eingabe prüfen."
        }
    }
}

/// What the job panel shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobView {
    /// Repository being synchronised.
    pub repository: String,
    /// Current state.
    pub state: JobState,
    /// Current phase.
    pub phase: Phase,
    /// Counters.
    pub progress: Progress,
    /// Error category when failed or cancelled.
    pub error: Option<ErrorKind>,
    /// True once a cancel request was sent but the job has not stopped yet.
    pub cancel_requested: bool,
}

enum WorkerMessage {
    Event(OperationEvent),
    Done(Result<ApplyReport, Error>),
}

struct ChannelSink {
    sender: Sender<WorkerMessage>,
    on_event: Box<dyn Fn() + Send>,
}

impl EventSink for ChannelSink {
    fn emit(&mut self, event: OperationEvent) {
        // If the UI is gone, the job simply keeps running to completion without observers.
        let _ = self.sender.send(WorkerMessage::Event(event));
        (self.on_event)();
    }
}

/// Repository wrapper that adds a small delay per file so progress and cancellation are observable.
struct SlowSource<S> {
    inner: S,
    delay: Duration,
}

impl<S: RepositorySource> RepositorySource for SlowSource<S> {
    fn manifest(&self) -> Result<Manifest, Error> {
        self.inner.manifest()
    }

    fn read(&self, path: &RelativePath) -> Result<Vec<u8>, Error> {
        thread::sleep(self.delay);
        self.inner.read(path)
    }
}

struct JobHandle {
    receiver: Receiver<WorkerMessage>,
    cancel: CancelToken,
    thread: Option<JoinHandle<()>>,
}

/// The complete GUI state.
pub struct Model {
    repositories: Vec<RepositoryRow>,
    filter: String,
    sort: (SortKey, SortDirection),
    visible: Vec<usize>,
    selected: Option<RepositoryId>,
    /// Open "add repository" dialog, if any.
    pub dialog: Option<AddDialog>,
    /// Field errors of the dialog.
    pub dialog_errors: FieldErrors,
    job: Option<JobHandle>,
    job_view: Option<JobView>,
    errors: Vec<ErrorItem>,
    next_operation: u64,
    file_delay: Duration,
    wake: std::sync::Arc<dyn Fn() + Send + Sync>,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Model")
            .field("repositories", &self.repositories.len())
            .field("filter", &self.filter)
            .field("selected", &self.selected)
            .field("dialog_open", &self.dialog.is_some())
            .field("busy", &self.is_busy())
            .finish_non_exhaustive()
    }
}

impl Model {
    /// A model showing `repositories`. `wake` is called from the worker thread when new events are available
    /// (the GUI passes `ctx.request_repaint`); tests pass a no-op.
    #[must_use]
    pub fn new(repositories: Vec<RepositoryRow>, wake: impl Fn() + Send + Sync + 'static) -> Self {
        let mut model = Self {
            repositories,
            filter: String::new(),
            sort: (SortKey::Name, SortDirection::Ascending),
            visible: Vec::new(),
            selected: None,
            dialog: None,
            dialog_errors: FieldErrors::default(),
            job: None,
            job_view: None,
            errors: Vec::new(),
            next_operation: 1,
            file_delay: Duration::from_millis(40),
            wake: std::sync::Arc::new(wake),
        };
        model.rebuild_visible();
        model
    }

    /// Delay per transferred file of the simulated source (tests use zero).
    pub fn set_file_delay(&mut self, delay: Duration) {
        self.file_delay = delay;
    }

    // ---- list, filter, sort, selection -------------------------------------------------------------------

    /// All repositories (unfiltered).
    #[must_use]
    pub fn repositories(&self) -> &[RepositoryRow] {
        &self.repositories
    }

    /// Number of rows that match the filter.
    #[must_use]
    pub fn visible_len(&self) -> usize {
        self.visible.len()
    }

    /// The `row`-th visible repository.
    #[must_use]
    pub fn visible_row(&self, row: usize) -> Option<&RepositoryRow> {
        self.visible
            .get(row)
            .and_then(|&index| self.repositories.get(index))
    }

    /// Current filter text.
    #[must_use]
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Sets the case-insensitive substring filter (name or address).
    pub fn set_filter(&mut self, filter: &str) {
        if self.filter != filter {
            filter.clone_into(&mut self.filter);
            self.rebuild_visible();
        }
    }

    /// Current sort.
    #[must_use]
    pub fn sort(&self) -> (SortKey, SortDirection) {
        self.sort
    }

    /// Sorts by `key`; selecting the active key again flips the direction.
    pub fn sort_by(&mut self, key: SortKey) {
        self.sort = if self.sort.0 == key {
            (
                key,
                if self.sort.1 == SortDirection::Ascending {
                    SortDirection::Descending
                } else {
                    SortDirection::Ascending
                },
            )
        } else {
            (key, SortDirection::Ascending)
        };
        self.rebuild_visible();
    }

    fn rebuild_visible(&mut self) {
        let needle = self.filter.to_lowercase();
        let mut visible: Vec<usize> = self
            .repositories
            .iter()
            .enumerate()
            .filter(|(_, repo)| {
                needle.is_empty()
                    || repo.name.to_lowercase().contains(&needle)
                    || repo.url.to_lowercase().contains(&needle)
            })
            .map(|(index, _)| index)
            .collect();
        let (key, direction) = self.sort;
        visible.sort_by(|&a, &b| {
            let (left, right) = (&self.repositories[a], &self.repositories[b]);
            let ordering = match key {
                SortKey::Name => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
                SortKey::Status => left.status.label().cmp(right.status.label()),
                SortKey::Files => left.files.cmp(&right.files),
            }
            .then_with(|| left.id.cmp(&right.id));
            if direction == SortDirection::Descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
        self.visible = visible;
    }

    /// Selects a repository by id (ignored if unknown).
    pub fn select(&mut self, id: &RepositoryId) {
        if self.repositories.iter().any(|repo| &repo.id == id) {
            self.selected = Some(id.clone());
        }
    }

    /// Clears the selection.
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// The selected repository, if any.
    #[must_use]
    pub fn selected(&self) -> Option<&RepositoryRow> {
        let id = self.selected.as_ref()?;
        self.repositories.iter().find(|repo| &repo.id == id)
    }

    /// Moves the selection by `delta` rows within the visible list (keyboard navigation).
    pub fn move_selection(&mut self, delta: isize) {
        if self.visible.is_empty() {
            return;
        }
        let current = self.selected.as_ref().and_then(|id| {
            self.visible
                .iter()
                .position(|&index| &self.repositories[index].id == id)
        });
        let target = match current {
            Some(position) => position
                .saturating_add_signed(delta)
                .min(self.visible.len() - 1),
            None if delta >= 0 => 0,
            None => self.visible.len() - 1,
        };
        self.selected = Some(self.repositories[self.visible[target]].id.clone());
    }

    /// Visible position of the selection, for scrolling it into view.
    #[must_use]
    pub fn selected_row(&self) -> Option<usize> {
        let id = self.selected.as_ref()?;
        self.visible
            .iter()
            .position(|&index| &self.repositories[index].id == id)
    }

    // ---- add dialog --------------------------------------------------------------------------------------

    /// Opens an empty dialog.
    pub fn open_dialog(&mut self) {
        self.dialog = Some(AddDialog::default());
        self.dialog_errors = FieldErrors::default();
    }

    /// Closes the dialog without saving.
    pub fn cancel_dialog(&mut self) {
        self.dialog = None;
        self.dialog_errors = FieldErrors::default();
    }

    /// Validates the dialog; on success adds the repository, selects it and closes the dialog.
    ///
    /// Returns `true` if the dialog was accepted. Otherwise `dialog_errors` explains what to fix.
    pub fn submit_dialog(&mut self) -> bool {
        let Some(dialog) = self.dialog.as_ref() else {
            return false;
        };
        let errors = self.validate(dialog);
        if !errors.is_empty() {
            self.dialog_errors = errors;
            return false;
        }
        let index = self.repositories.len();
        let row = RepositoryRow {
            id: unique_id(&self.repositories, index),
            name: dialog.name.trim().to_owned(),
            url: dialog.url.trim().to_owned(),
            status: RepoStatus::Outdated,
            files: 5,
            damaged_transfer: false,
        };
        let id = row.id.clone();
        self.repositories.push(row);
        self.dialog = None;
        self.dialog_errors = FieldErrors::default();
        self.rebuild_visible();
        self.selected = Some(id);
        true
    }

    fn validate(&self, dialog: &AddDialog) -> FieldErrors {
        let mut errors = FieldErrors::default();
        let name = dialog.name.trim();
        if name.is_empty() {
            errors.name = Some(text::NAME_REQUIRED);
        } else if name.chars().count() > 80 {
            errors.name = Some(text::NAME_TOO_LONG);
        } else if self
            .repositories
            .iter()
            .any(|repo| repo.name.eq_ignore_ascii_case(name))
        {
            errors.name = Some(text::NAME_TAKEN);
        }
        let url = dialog.url.trim();
        if url.is_empty() {
            errors.url = Some(text::URL_REQUIRED);
        } else if let Some(rest) = ["https://", "http://", "ftps://", "ftp://", "sftp://"]
            .iter()
            .find_map(|scheme| url.strip_prefix(scheme))
        {
            let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
            if authority.contains('@') {
                errors.url = Some(text::URL_CREDENTIALS);
            } else if authority.is_empty() || url.chars().any(char::is_whitespace) {
                errors.url = Some(text::URL_INVALID);
            }
        } else {
            errors.url = Some(text::URL_SCHEME);
        }
        errors
    }

    // ---- job ---------------------------------------------------------------------------------------------

    /// Whether a job is running.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    /// The job panel content, if a job ran or is running.
    #[must_use]
    pub fn job_view(&self) -> Option<&JobView> {
        self.job_view.as_ref()
    }

    /// The error summary (newest last).
    #[must_use]
    pub fn errors(&self) -> &[ErrorItem] {
        &self.errors
    }

    /// Removes all entries from the error summary.
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    /// Starts a synchronisation of the selected repository on a worker thread.
    ///
    /// Does nothing if nothing is selected or a job is already running. Repositories that need credentials are
    /// reported as a configuration error without starting a job.
    pub fn start_sync(&mut self) {
        if self.is_busy() {
            return;
        }
        let Some(row) = self.selected().cloned() else {
            return;
        };
        if row.status == RepoStatus::AuthRequired {
            let message = "Für dieses Repository sind Anmeldedaten erforderlich.".to_owned();
            self.push_error(&row.name, ErrorKind::Config, message);
            return;
        }
        let operation = OperationId::new(self.next_operation);
        self.next_operation += 1;
        let (sender, receiver) = mpsc::channel();
        let cancel = CancelToken::new();
        let worker_cancel = cancel.clone();
        let wake = std::sync::Arc::clone(&self.wake);
        let delay = self.file_delay;
        self.job_view = Some(JobView {
            repository: row.name.clone(),
            state: JobState::Queued,
            phase: Phase::Planning,
            progress: Progress::default(),
            error: None,
            cancel_requested: false,
        });
        let handle = thread::spawn(move || {
            let result = run_job(operation, &row, delay, &worker_cancel, &sender, &wake);
            let _ = sender.send(WorkerMessage::Done(result));
            wake();
        });
        self.job = Some(JobHandle {
            receiver,
            cancel,
            thread: Some(handle),
        });
    }

    /// Requests cancellation of the running job (idempotent).
    pub fn cancel_sync(&mut self) {
        if let Some(job) = &self.job {
            job.cancel.cancel();
            if let Some(view) = self.job_view.as_mut() {
                view.cancel_requested = true;
            }
        }
    }

    /// Applies pending worker messages. Call once per frame; never blocks.
    pub fn poll(&mut self) {
        let mut finished: Option<Result<ApplyReport, Error>> = None;
        if let Some(job) = &self.job {
            loop {
                match job.receiver.try_recv() {
                    Ok(WorkerMessage::Event(event)) => {
                        if let Some(view) = self.job_view.as_mut() {
                            view.state = event.state;
                            view.phase = event.phase;
                            view.progress = event.progress;
                            view.error = event.error;
                        }
                    }
                    Ok(WorkerMessage::Done(result)) => {
                        finished = Some(result);
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        finished = Some(Err(Error::new(
                            ErrorKind::Integrity,
                            "worker stopped unexpectedly",
                        )));
                        break;
                    }
                }
            }
        }
        if let Some(result) = finished {
            if let Some(mut job) = self.job.take()
                && let Some(thread) = job.thread.take()
            {
                let _ = thread.join();
            }
            self.finish_job(result);
        }
    }

    fn finish_job(&mut self, result: Result<ApplyReport, Error>) {
        let repository = self
            .job_view
            .as_ref()
            .map(|view| view.repository.clone())
            .unwrap_or_default();
        match result {
            Ok(_) => {
                if let Some(selected) = self.selected.clone() {
                    if let Some(repo) = self
                        .repositories
                        .iter_mut()
                        .find(|repo| repo.id == selected)
                    {
                        repo.status = RepoStatus::UpToDate;
                    }
                    self.rebuild_visible();
                }
            }
            Err(error) => {
                if let Some(view) = self.job_view.as_mut() {
                    view.state = if error.kind() == ErrorKind::Cancelled {
                        JobState::Cancelled
                    } else {
                        JobState::Failed
                    };
                    view.error = Some(error.kind());
                }
                self.push_error(&repository, error.kind(), error.message().to_owned());
            }
        }
    }

    fn push_error(&mut self, repository: &str, kind: ErrorKind, message: String) {
        self.errors.push(ErrorItem {
            repository: repository.to_owned(),
            kind,
            message,
            hint: hint_for(kind),
        });
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        if let Some(mut job) = self.job.take() {
            job.cancel.cancel();
            if let Some(thread) = job.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

fn unique_id(existing: &[RepositoryRow], mut index: usize) -> RepositoryId {
    loop {
        let id = repository_id(index);
        if existing.iter().all(|repo| repo.id != id) {
            return id;
        }
        index += 1;
    }
}

fn run_job(
    operation: OperationId,
    row: &RepositoryRow,
    delay: Duration,
    cancel: &CancelToken,
    sender: &Sender<WorkerMessage>,
    wake: &std::sync::Arc<dyn Fn() + Send + Sync>,
) -> Result<ApplyReport, Error> {
    let names: Vec<String> = (0..row.files.min(60))
        .map(|n| format!("@mod{}/addons/file{n}.pbo", n % 3))
        .collect();
    let contents: Vec<Vec<u8>> = (0..names.len())
        .map(|n| format!("content of file {n} of {}", row.name).into_bytes())
        .collect();
    let files: Vec<(&str, &[u8])> = names
        .iter()
        .zip(&contents)
        .map(|(name, data)| (name.as_str(), data.as_slice()))
        .collect();
    let mut repository = InMemoryRepository::new("rev-1", &files)?;
    if row.status == RepoStatus::Offline {
        repository.set_unreachable(true);
    }
    if row.damaged_transfer
        && let Some(name) = names.first()
    {
        let path = RelativePath::parse(name)
            .map_err(|error| Error::new(ErrorKind::Format, error.to_string()))?;
        repository.serve_corrupted(&path, b"damaged");
    }
    let source = SlowSource {
        inner: repository,
        delay,
    };
    // Pretend some files already exist locally so that the plan has downloads, replacements and kept files.
    let mut store = InMemoryStore::new();
    if row.status == RepoStatus::UpToDate {
        for (name, data) in names.iter().zip(&contents) {
            store = store.with_file(name, data)?;
        }
    }
    let options = PlanOptions::default();
    let planned = plan(&source, &store, &options)?;
    let hasher = Sha256Hasher;
    let unpacker = ZipUnpacker;
    let mut sink = ChannelSink {
        sender: sender.clone(),
        on_event: Box::new({
            let wake = std::sync::Arc::clone(wake);
            move || wake()
        }),
    };
    let mut context = SyncContext {
        source: &source,
        store: &mut store,
        hasher: &hasher,
        unpacker: &unpacker,
        sink: &mut sink,
        cancel,
    };
    apply(operation, &planned, &options, &mut context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn model(count: usize) -> Model {
        let mut model = Model::new(synthetic_repositories(count), || {});
        model.set_file_delay(Duration::ZERO);
        model
    }

    fn wait_until_idle(model: &mut Model) {
        let start = Instant::now();
        while model.is_busy() {
            model.poll();
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "job did not finish"
            );
            thread::sleep(Duration::from_millis(2));
        }
    }

    fn select_first_with(model: &mut Model, status: RepoStatus, damaged: bool) -> RepositoryId {
        let id = model
            .repositories()
            .iter()
            .find(|r| r.status == status && r.damaged_transfer == damaged)
            .unwrap()
            .id
            .clone();
        model.select(&id);
        id
    }

    #[test]
    fn ten_thousand_rows_with_filter_and_sort() {
        let mut model = model(10_000);
        assert_eq!(model.visible_len(), 10_000);
        model.set_filter("repository 00042");
        assert_eq!(model.visible_len(), 1);
        model.set_filter("");
        model.sort_by(SortKey::Files);
        let first = model.visible_row(0).unwrap().files;
        let last = model.visible_row(9_999).unwrap().files;
        assert!(first <= last);
        model.sort_by(SortKey::Files);
        assert_eq!(model.sort(), (SortKey::Files, SortDirection::Descending));
        assert!(model.visible_row(0).unwrap().files >= model.visible_row(9_999).unwrap().files);
        model.set_filter("EXAMPLE.INVALID/arma3/sync-7");
        assert!(model.visible_len() > 0 && model.visible_len() < 10_000);
        model.set_filter("no such repository");
        assert_eq!(model.visible_len(), 0);
        assert!(model.visible_row(0).is_none());
    }

    #[test]
    fn selection_and_keyboard_navigation() {
        let mut model = model(20);
        assert!(model.selected().is_none());
        model.move_selection(1);
        assert_eq!(model.selected_row(), Some(0));
        model.move_selection(3);
        assert_eq!(model.selected_row(), Some(3));
        model.move_selection(-100);
        assert_eq!(model.selected_row(), Some(0));
        model.move_selection(1000);
        assert_eq!(model.selected_row(), Some(19));
        model.clear_selection();
        assert!(model.selected().is_none());
        model.select(&RepositoryId::parse("unknown").unwrap());
        assert!(model.selected().is_none());
    }

    #[test]
    fn dialog_validation_reports_each_field() {
        let mut model = model(3);
        model.open_dialog();
        assert!(!model.submit_dialog());
        assert_eq!(model.dialog_errors.name, Some(text::NAME_REQUIRED));
        assert_eq!(model.dialog_errors.url, Some(text::URL_REQUIRED));

        let dialog = model.dialog.as_mut().unwrap();
        dialog.name = "Repository 00001".into();
        dialog.url = "https://user:secret@example.invalid/x".into();
        assert!(!model.submit_dialog());
        assert_eq!(model.dialog_errors.name, Some(text::NAME_TAKEN));
        assert_eq!(model.dialog_errors.url, Some(text::URL_CREDENTIALS));

        let dialog = model.dialog.as_mut().unwrap();
        dialog.url = "file:///etc/passwd".into();
        assert!(!model.submit_dialog());
        assert_eq!(model.dialog_errors.url, Some(text::URL_SCHEME));
        let dialog = model.dialog.as_mut().unwrap();
        dialog.url = "https://exa mple.invalid".into();
        assert!(!model.submit_dialog());
        assert_eq!(model.dialog_errors.url, Some(text::URL_INVALID));
        assert!(
            model.dialog.is_some(),
            "dialog stays open until the input is valid or cancelled"
        );
    }

    #[test]
    fn valid_dialog_adds_selects_and_closes_without_leaking_the_password() {
        let mut model = model(3);
        model.open_dialog();
        let dialog = model.dialog.as_mut().unwrap();
        dialog.name = "Neues Repository".into();
        dialog.url = "https://example.invalid/arma3/sync".into();
        dialog.username = "anna".into();
        dialog.password.buffer_mut().push_str("hunter2");
        assert!(
            !format!("{dialog:?}").contains("hunter2"),
            "Debug must not reveal the password"
        );
        assert!(model.submit_dialog());
        assert!(model.dialog.is_none());
        assert_eq!(model.repositories().len(), 4);
        assert_eq!(model.selected().unwrap().name, "Neues Repository");
        assert!(!format!("{model:?}").contains("hunter2"));
        assert!(
            !model
                .repositories()
                .iter()
                .any(|r| r.url.contains("anna") || r.url.contains("hunter2"))
        );
        model.open_dialog();
        model.cancel_dialog();
        assert!(model.dialog.is_none());
    }

    #[test]
    fn sync_of_an_outdated_repository_succeeds_with_progress_and_updates_the_status() {
        let mut model = model(40);
        let id = select_first_with(&mut model, RepoStatus::Outdated, false);
        model.start_sync();
        assert!(model.is_busy());
        assert!(model.job_view().is_some());
        wait_until_idle(&mut model);
        let view = model.job_view().unwrap();
        assert_eq!(view.state, JobState::Succeeded);
        assert_eq!(view.progress.files_done, view.progress.files_total);
        assert!(view.progress.files_total > 0);
        assert!(model.errors().is_empty());
        assert_eq!(
            model
                .repositories()
                .iter()
                .find(|r| r.id == id)
                .unwrap()
                .status,
            RepoStatus::UpToDate
        );
    }

    #[test]
    fn damaged_transfer_is_an_integrity_error_with_a_next_step() {
        let mut model = model(60);
        select_first_with(&mut model, RepoStatus::Outdated, true);
        model.start_sync();
        wait_until_idle(&mut model);
        assert_eq!(model.job_view().unwrap().state, JobState::Failed);
        let error = &model.errors()[0];
        assert_eq!(error.kind, ErrorKind::Integrity);
        assert!(!error.hint.is_empty());
        assert_ne!(model.selected().unwrap().status, RepoStatus::UpToDate);
    }

    #[test]
    fn offline_and_auth_required_are_reported_distinctly() {
        let mut model = model(30);
        select_first_with(&mut model, RepoStatus::Offline, false);
        model.start_sync();
        wait_until_idle(&mut model);
        assert_eq!(model.errors()[0].kind, ErrorKind::Network);

        select_first_with(&mut model, RepoStatus::AuthRequired, false);
        model.start_sync();
        assert!(
            !model.is_busy(),
            "no job for a repository that needs credentials"
        );
        assert_eq!(model.errors()[1].kind, ErrorKind::Config);
        model.clear_errors();
        assert!(model.errors().is_empty());
    }

    #[test]
    fn cancel_stops_the_job_in_the_cancelled_state() {
        let mut model = model(40);
        model.set_file_delay(Duration::from_millis(30));
        select_first_with(&mut model, RepoStatus::Outdated, false);
        model.start_sync();
        thread::sleep(Duration::from_millis(50));
        model.cancel_sync();
        assert!(model.job_view().unwrap().cancel_requested);
        wait_until_idle(&mut model);
        let view = model.job_view().unwrap();
        assert_eq!(view.state, JobState::Cancelled);
        assert_eq!(view.error, Some(ErrorKind::Cancelled));
        assert!(view.progress.files_done < view.progress.files_total);
        assert_eq!(model.errors()[0].kind, ErrorKind::Cancelled);
    }

    #[test]
    fn only_one_job_at_a_time_and_dropping_the_model_stops_the_worker() {
        let mut model = model(40);
        model.set_file_delay(Duration::from_millis(20));
        select_first_with(&mut model, RepoStatus::Outdated, false);
        model.start_sync();
        model.start_sync();
        assert!(model.is_busy());
        drop(model); // joins the worker after cancelling it; must not hang
    }

    #[test]
    fn up_to_date_repository_finishes_without_transfers() {
        let mut model = model(10);
        select_first_with(&mut model, RepoStatus::UpToDate, false);
        model.start_sync();
        wait_until_idle(&mut model);
        let view = model.job_view().unwrap();
        assert_eq!(view.state, JobState::Succeeded);
        assert_eq!(view.progress.files_total, 0);
    }
}
