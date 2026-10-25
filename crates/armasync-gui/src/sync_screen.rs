// SPDX-License-Identifier: AGPL-3.0-only
//! State and behaviour of the synchronisation screen (MIG-031), without drawing.
//!
//! The screen walks the user through: choose a repository from the library, enter the destination folder, **look at
//! the plan** (what is downloaded, replaced and — only if explicitly chosen — deleted), confirm deletions, run with
//! progress and cancellation, and read the result. All work goes through `armasync-workflow` — the same wiring the
//! command line uses — on worker threads, so the window never blocks. Nothing here plans, transfers or verifies on
//! its own.
//!
//! Safety properties kept by this model (and tested):
//! * the preview always belongs to the current inputs; changing destination, repository or deletion choices
//!   invalidates it and starting is impossible until the plan was checked again;
//! * deleting extra files is off by default, needs explicit folders, lists every file to be deleted and needs a
//!   separate confirmation;
//! * on a platform that is not released, the plan can be viewed but not applied;
//! * protected and unsupported (compressed without a usable archive size) entries are shown as "stay unchanged" and are part of the plan; compressed entries with a usable archive size are installed like any other file (MIG-030E)
//!   options by construction of the workflow.

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};

use armasync_adapters::LibraryStore;
use armasync_adapters::http::HttpConfig;
use armasync_adapters::paths::{DataRoots, LIBRARY_FILE, PathMode};
use armasync_adapters::sync_meta::SyncWarning;
use armasync_application::sync::ApplyReport;
use armasync_core::{
    CancelToken, Error, ErrorKind, EventSink, JobState, OperationEvent, OperationId, Phase,
    PlanWarning, Progress, RelativePath, RepositoryId, SyncStep,
};
use armasync_workflow::{
    PlatformStatus, PreparedSync, SyncRequest, execute, parse_managed_roots, prepare,
};

use crate::model::{ErrorItem, JobView};

/// A repository of the user's library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryChoice {
    /// Id in the library.
    pub id: RepositoryId,
    /// Display name.
    pub name: String,
    /// Address (no credentials).
    pub url: String,
    /// The repository was marked as needing credentials (not supported yet; none are ever sent).
    pub needs_credentials: bool,
}

/// What a plan row does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanAction {
    /// A file that does not exist locally is downloaded.
    Download,
    /// A local file that differs is replaced.
    Replace,
    /// A local file the repository does not contain is deleted (only if chosen).
    Delete,
}

impl PlanAction {
    /// Text shown in the table (status is never conveyed by colour alone).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Download => "Neu laden",
            Self::Replace => "Ersetzen",
            Self::Delete => "Löschen",
        }
    }
}

/// One row of the plan table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRow {
    /// Action.
    pub action: PlanAction,
    /// Path relative to the destination.
    pub path: String,
    /// Size in bytes (none for deletions).
    pub size: Option<u64>,
}

/// Totals of a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlanTotals {
    /// Files to download.
    pub downloads: u64,
    /// Bytes to download.
    pub download_bytes: u64,
    /// Files to replace.
    pub replaces: u64,
    /// Bytes of replacements.
    pub replace_bytes: u64,
    /// Files that are up to date.
    pub unchanged: u64,
    /// Files to delete.
    pub deletes: u64,
    /// Extra local files that stay.
    pub extras_kept: u64,
}

/// The plan as the screen shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanView {
    /// Revision of the repository content.
    pub revision: String,
    /// Totals.
    pub totals: PlanTotals,
    /// Download, replace and delete rows (up-to-date files are not listed).
    pub rows: Vec<PlanRow>,
    /// Paths to be deleted (also in `rows`), listed again for the confirmation.
    pub deletes: Vec<String>,
    /// Notes for the user (German).
    pub notes: Vec<String>,
    /// Paths that stay untouched because the repository protects them.
    pub protected: Vec<String>,
    /// Paths of files the server holds compressed without a usable archive size; not installed and never deleted.
    pub unsupported: Vec<String>,
    /// The repository was marked as needing credentials.
    pub needs_credentials: bool,
}

/// Counters of a finished run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportView {
    /// Files downloaded.
    pub downloaded: u64,
    /// Files replaced.
    pub replaced: u64,
    /// Files deleted.
    pub deleted: u64,
    /// Files that were already up to date.
    pub kept: u64,
    /// Bytes transferred.
    pub bytes: u64,
}

/// How the screen ended up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// All verified and installed.
    Succeeded(ReportView),
    /// Stopped by the user.
    Cancelled,
    /// Failed; existing files were not damaged.
    Failed(ErrorItem),
}

/// Where the user is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Choosing repository, destination and options.
    Choose,
    /// Reading the repository's file list and comparing (changes nothing).
    Planning,
    /// The plan is shown.
    PlanReady,
    /// Transferring.
    Running,
    /// Finished (see [`Outcome`]).
    Finished,
}

/// Errors next to the fields they belong to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncFieldErrors {
    /// Repository choice.
    pub repository: Option<String>,
    /// Destination folder.
    pub destination: Option<String>,
    /// Folders for deleting.
    pub roots: Option<String>,
}

impl SyncFieldErrors {
    /// Whether there is no error.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.repository.is_none() && self.destination.is_none() && self.roots.is_none()
    }
}

/// The inputs a plan was made for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedInputs {
    repository: Option<RepositoryId>,
    destination: String,
    delete_extra: bool,
    roots: String,
}

enum Message {
    Prepared(Result<Box<PreparedSync>, Error>),
    Event(OperationEvent),
    Done(Result<ApplyReport, Error>),
}

struct ChannelSink {
    sender: Sender<Message>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl EventSink for ChannelSink {
    fn emit(&mut self, event: OperationEvent) {
        let _ = self.sender.send(Message::Event(event));
        (self.wake)();
    }
}

struct Worker {
    receiver: Receiver<Message>,
    cancel: CancelToken,
    thread: Option<JoinHandle<()>>,
}

/// The synchronisation screen.
pub struct SyncScreen {
    config: Option<PathBuf>,
    repositories: Vec<RepositoryChoice>,
    load_error: Option<String>,
    selected: Option<usize>,
    /// Destination folder as typed.
    pub destination: String,
    /// Delete extra local files (off by default).
    pub delete_extra: bool,
    /// Folders where extra files may be deleted, one per line (as typed).
    pub managed_roots: String,
    /// The user confirmed the listed deletions.
    pub confirm_deletes: bool,
    /// Errors next to the fields.
    pub field_errors: SyncFieldErrors,
    stage: Stage,
    plan: Option<PlanView>,
    prepared: Option<Arc<PreparedSync>>,
    planned_for: Option<PlannedInputs>,
    job: Option<JobView>,
    outcome: Option<Outcome>,
    notice: Option<String>,
    platform: PlatformStatus,
    http: HttpConfig,
    worker: Option<Worker>,
    next_operation: u64,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl fmt::Debug for SyncScreen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyncScreen")
            .field("stage", &self.stage)
            .field("repositories", &self.repositories.len())
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Formats a byte count for display ("8.0 KiB").
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    human_bytes(bytes)
}

fn hint_for(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Network => {
            "Verbindung, Adresse und Zertifikat des Servers prüfen. Dateien, die nicht vollständig geprüft waren, wurden nicht übernommen."
        }
        ErrorKind::Integrity => {
            "Eine Datei stimmte nicht mit der Prüfsumme des Servers überein und wurde nicht übernommen. Vorhandene Dateien sind unverändert. Später erneut versuchen oder den Betreiber informieren."
        }
        ErrorKind::FileSystem => {
            "Zielordner, Schreibrechte und freien Speicherplatz prüfen. Vorhandene Dateien sind unverändert."
        }
        ErrorKind::Config => {
            "Der Zielordner wird gerade von einem anderen ArmaSync-Lauf verwendet. Warten, bis dieser beendet ist, und erneut versuchen."
        }
        ErrorKind::Format => {
            "Die Daten des Servers sind nicht lesbar oder nicht unterstützt. Den Betreiber des Repositorys informieren."
        }
        ErrorKind::Unsupported => {
            "Dieser Vorgang wird auf diesem System oder für diese Adresse noch nicht unterstützt."
        }
        ErrorKind::Usage => "Eingaben prüfen.",
        ErrorKind::Cancelled => {
            "Abgebrochen. Fertig geprüfte Dateien sind vollständig; die unterbrochene Datei wurde nicht verändert."
        }
    }
}

fn plan_warning_text(warning: &PlanWarning) -> String {
    match warning {
        PlanWarning::ExtraKeptOutsideManagedRoots(path) => {
            format!("Zusätzliche Datei bleibt erhalten (außerhalb der Löschordner): {path}")
        }
        PlanWarning::ExtraKeptProtected(path) => {
            format!("Zusätzliche Datei bleibt erhalten (vom Repository geschützt): {path}")
        }
        PlanWarning::CaseMismatch { local, expected } => {
            format!(
                "„{local}“ unterscheidet sich von „{expected}“ nur durch Groß-/Kleinschreibung."
            )
        }
    }
}

fn metadata_warning_text(warning: &SyncWarning) -> String {
    match warning {
        SyncWarning::UnsafeFileName { shown, .. } => {
            format!("Eine Datei mit unsicherem Namen wurde ausgelassen: „{shown}“.")
        }
        SyncWarning::UnsafeFolderName {
            shown,
            files_left_out,
            ..
        } => format!(
            "Ein Ordner mit unsicherem Namen wurde mit {files_left_out} Datei(en) ausgelassen: „{shown}“."
        ),
        SyncWarning::CompressedFileUnsupported(path) => format!(
            "Komprimiert geführte Datei ohne nutzbare Archivgröße bleibt unverändert: {path}"
        ),
        SyncWarning::ServerInfoPathIgnored => {
            "Ein Ordnerpfad der Serverinformation wurde ignoriert (kein sicherer Pfad).".to_owned()
        }
    }
}

fn build_plan_view(prepared: &PreparedSync, needs_credentials: bool) -> PlanView {
    let plan = prepared.plan();
    let mut rows = Vec::new();
    let mut deletes = Vec::new();
    for step in &plan.steps {
        match step {
            SyncStep::Download { path, size } => rows.push(PlanRow {
                action: PlanAction::Download,
                path: path.as_str().to_owned(),
                size: Some(*size),
            }),
            SyncStep::Replace { path, size } => rows.push(PlanRow {
                action: PlanAction::Replace,
                path: path.as_str().to_owned(),
                size: Some(*size),
            }),
            SyncStep::Delete { path } => {
                deletes.push(path.as_str().to_owned());
                rows.push(PlanRow {
                    action: PlanAction::Delete,
                    path: path.as_str().to_owned(),
                    size: None,
                });
            }
            SyncStep::Keep { .. } => {}
        }
    }
    let metadata = prepared.metadata();
    let mut notes: Vec<String> = metadata
        .warnings
        .iter()
        .map(metadata_warning_text)
        .collect();
    notes.extend(plan.warnings.iter().map(plan_warning_text));
    let s = &plan.summary;
    PlanView {
        revision: plan.revision.clone(),
        totals: PlanTotals {
            downloads: s.download_files,
            download_bytes: s.download_bytes,
            replaces: s.replace_files,
            replace_bytes: s.replace_bytes,
            unchanged: s.keep_files,
            deletes: s.delete_files,
            extras_kept: s.extra_untouched,
        },
        rows,
        deletes,
        notes,
        protected: metadata
            .protected
            .iter()
            .map(|p| p.as_str().to_owned())
            .collect(),
        unsupported: metadata
            .unsupported
            .iter()
            .map(|p| p.as_str().to_owned())
            .collect(),
        needs_credentials,
    }
}

impl SyncScreen {
    /// A screen reading the library from `config` (the platform's config directory if `None`). `platform` decides
    /// whether changes are allowed; `wake` is called from worker threads when there is something to show.
    #[must_use]
    pub fn new(
        config: Option<PathBuf>,
        platform: PlatformStatus,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let mut screen = Self {
            config,
            repositories: Vec::new(),
            load_error: None,
            selected: None,
            destination: String::new(),
            delete_extra: false,
            managed_roots: String::new(),
            confirm_deletes: false,
            field_errors: SyncFieldErrors::default(),
            stage: Stage::Choose,
            plan: None,
            prepared: None,
            planned_for: None,
            job: None,
            outcome: None,
            notice: None,
            platform,
            http: HttpConfig::default(),
            worker: None,
            next_operation: 1,
            wake: Arc::new(wake),
        };
        screen.reload();
        screen
    }

    /// Replaces the HTTP client settings (tests inject trusted test roots; default is the system trust store).
    pub fn set_http_config(&mut self, http: HttpConfig) {
        self.http = http;
    }

    /// Reads the library again.
    pub fn reload(&mut self) {
        self.load_error = None;
        let path = match &self.config {
            Some(dir) => Ok(dir.join(LIBRARY_FILE)),
            None => DataRoots::for_current_process(&PathMode::Standard)
                .map(|roots| roots.config().join(LIBRARY_FILE)),
        };
        let loaded = path.and_then(|p| LibraryStore::new(p).load());
        match loaded {
            Ok(library) => {
                self.repositories = library
                    .repositories()
                    .iter()
                    .filter(|r| r.url.as_str().starts_with("http"))
                    .map(|r| RepositoryChoice {
                        id: r.id.clone(),
                        name: r.name.as_str().to_owned(),
                        url: r.url.as_str().to_owned(),
                        needs_credentials: r.credential.is_some(),
                    })
                    .collect();
                if self.selected.is_some_and(|i| i >= self.repositories.len()) {
                    self.selected = None;
                }
                if self.selected.is_none() && self.repositories.len() == 1 {
                    self.selected = Some(0);
                }
            }
            Err(error) => {
                self.repositories.clear();
                self.selected = None;
                self.load_error = Some(format!(
                    "Die Repository-Liste konnte nicht gelesen werden: {}",
                    error.message()
                ));
            }
        }
    }

    // ---- accessors ---------------------------------------------------------------------------------------

    /// Repositories that can be synchronised (HTTP and HTTPS addresses).
    #[must_use]
    pub fn repositories(&self) -> &[RepositoryChoice] {
        &self.repositories
    }

    /// Why the library could not be read, if it could not.
    #[must_use]
    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    /// The chosen repository.
    #[must_use]
    pub fn selected(&self) -> Option<&RepositoryChoice> {
        self.selected.and_then(|i| self.repositories.get(i))
    }

    /// Chooses a repository by position (changes invalidate an existing plan).
    pub fn select(&mut self, index: usize) {
        if index < self.repositories.len() && !self.is_busy() {
            self.selected = Some(index);
            self.field_errors.repository = None;
        }
    }

    /// Where the user is.
    #[must_use]
    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// The plan, if one was made.
    #[must_use]
    pub fn plan(&self) -> Option<&PlanView> {
        self.plan.as_ref()
    }

    /// Progress of the running or last job.
    #[must_use]
    pub fn job(&self) -> Option<&JobView> {
        self.job.as_ref()
    }

    /// How the last run ended.
    #[must_use]
    pub fn outcome(&self) -> Option<&Outcome> {
        self.outcome.as_ref()
    }

    /// A short message for the user (for example why a plan was dropped).
    #[must_use]
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// The platform status shown in the window.
    #[must_use]
    pub fn platform(&self) -> PlatformStatus {
        self.platform
    }

    /// Whether a worker is running (planning or transferring).
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.worker.is_some()
    }

    fn current_inputs(&self) -> PlannedInputs {
        PlannedInputs {
            repository: self.selected().map(|r| r.id.clone()),
            destination: self.destination.trim().to_owned(),
            delete_extra: self.delete_extra,
            roots: self.managed_roots.trim().to_owned(),
        }
    }

    /// Whether the plan shown belongs to the current inputs.
    #[must_use]
    pub fn plan_is_current(&self) -> bool {
        self.planned_for.as_ref() == Some(&self.current_inputs())
    }

    // ---- planning ----------------------------------------------------------------------------------------

    fn root_lines(&self) -> Vec<String> {
        self.managed_roots
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect()
    }

    fn validate(&mut self) -> Option<SyncRequest> {
        self.field_errors = SyncFieldErrors::default();
        let Some(repository) = self.selected().cloned() else {
            self.field_errors.repository = Some("Bitte ein Repository auswählen.".to_owned());
            return None;
        };
        let destination = self.destination.trim();
        if destination.is_empty() {
            self.field_errors.destination = Some("Bitte den Zielordner angeben.".to_owned());
        } else if !std::path::Path::new(destination).is_dir() {
            self.field_errors.destination = Some(
                "Dieser Ordner existiert nicht. Bitte zuerst anlegen oder einen vorhandenen Ordner angeben."
                    .to_owned(),
            );
        }
        let mut roots: Vec<RelativePath> = Vec::new();
        if self.delete_extra {
            match parse_managed_roots(&self.root_lines(), true) {
                Ok(parsed) => roots = parsed,
                Err(error) => {
                    self.field_errors.roots = Some(match error.kind() {
                        ErrorKind::Usage if self.root_lines().is_empty() => {
                            "Zum Löschen mindestens einen Ordner angeben (relativ zum Zielordner, einer pro Zeile)."
                                .to_owned()
                        }
                        _ => format!(
                            "Ungültiger Ordner: {} Nur sichere relative Ordner sind erlaubt.",
                            error.message()
                        ),
                    });
                }
            }
        }
        if !self.field_errors.is_empty() {
            return None;
        }
        Some(SyncRequest {
            repository_url: repository.url,
            destination: PathBuf::from(destination),
            delete_extra: self.delete_extra,
            managed_roots: roots,
            http: self.http.clone(),
        })
    }

    /// Checks the inputs and starts reading the repository and comparing with the destination. Changes nothing.
    pub fn request_plan(&mut self) {
        if self.is_busy() {
            return;
        }
        let Some(request) = self.validate() else {
            return;
        };
        self.notice = None;
        self.plan = None;
        self.prepared = None;
        self.planned_for = None;
        self.outcome = None;
        self.confirm_deletes = false;
        self.stage = Stage::Planning;
        let inputs = self.current_inputs();
        let (sender, receiver) = mpsc::channel();
        let cancel = CancelToken::new();
        let worker_cancel = cancel.clone();
        let wake = Arc::clone(&self.wake);
        let handle = thread::spawn(move || {
            let result = prepare(&request, &worker_cancel).map(Box::new);
            let _ = sender.send(Message::Prepared(result));
            wake();
        });
        self.planned_for = Some(inputs);
        self.worker = Some(Worker {
            receiver,
            cancel,
            thread: Some(handle),
        });
    }

    // ---- starting ----------------------------------------------------------------------------------------

    /// Whether the number of deletions in the plan has been confirmed (or there are none).
    #[must_use]
    pub fn deletions_confirmed(&self) -> bool {
        self.plan
            .as_ref()
            .is_none_or(|p| p.deletes.is_empty() || self.confirm_deletes)
    }

    /// Whether "Synchronisieren" may be pressed, and if not, why.
    ///
    /// # Errors
    /// The reason in German.
    pub fn start_blocker(&self) -> Result<(), &'static str> {
        if self.is_busy() {
            return Err("Es läuft bereits ein Vorgang.");
        }
        if self.stage != Stage::PlanReady || self.prepared.is_none() {
            return Err("Zuerst die Änderungen prüfen.");
        }
        if !self.plan_is_current() {
            return Err("Die Eingaben wurden geändert. Bitte die Änderungen erneut prüfen.");
        }
        if !self.platform.run_allowed() {
            return Err("Auf diesem System ist das Synchronisieren noch nicht freigegeben.");
        }
        if !self.deletions_confirmed() {
            return Err("Bitte zuerst die Löschungen bestätigen.");
        }
        Ok(())
    }

    /// Starts the synchronisation of the plan that is shown.
    pub fn start(&mut self) {
        if self.start_blocker().is_err() {
            return;
        }
        let Some(prepared) = self.prepared.clone() else {
            return;
        };
        let repository = self.selected().map(|r| r.name.clone()).unwrap_or_default();
        let operation = OperationId::new(self.next_operation);
        self.next_operation += 1;
        self.job = Some(JobView {
            repository,
            state: JobState::Queued,
            phase: Phase::Planning,
            progress: Progress::default(),
            error: None,
            cancel_requested: false,
        });
        self.outcome = None;
        self.stage = Stage::Running;
        let (sender, receiver) = mpsc::channel();
        let cancel = CancelToken::new();
        let worker_cancel = cancel.clone();
        let wake = Arc::clone(&self.wake);
        let handle = thread::spawn(move || {
            let mut sink = ChannelSink {
                sender: sender.clone(),
                wake: Arc::clone(&wake),
            };
            let result = execute(&prepared, operation, &mut sink, &worker_cancel);
            let _ = sender.send(Message::Done(result));
            wake();
        });
        self.worker = Some(Worker {
            receiver,
            cancel,
            thread: Some(handle),
        });
    }

    /// Requests cancellation of planning or of the transfer (idempotent). Files already completed stay; the file
    /// in progress is discarded and the existing one stays unchanged.
    pub fn cancel(&mut self) {
        if let Some(worker) = &self.worker {
            worker.cancel.cancel();
            if let Some(job) = self.job.as_mut() {
                job.cancel_requested = true;
            }
        }
    }

    /// Back to choosing (drops the plan and the result).
    pub fn reset(&mut self) {
        if self.is_busy() {
            return;
        }
        self.stage = Stage::Choose;
        self.plan = None;
        self.prepared = None;
        self.planned_for = None;
        self.job = None;
        self.outcome = None;
        self.notice = None;
        self.confirm_deletes = false;
    }

    /// Applies worker messages and drops a plan that no longer matches the inputs. Call once per frame; never
    /// blocks.
    pub fn poll(&mut self) {
        let mut finished: Option<Message> = None;
        if let Some(worker) = &self.worker {
            loop {
                match worker.receiver.try_recv() {
                    Ok(Message::Event(event)) => {
                        if let Some(job) = self.job.as_mut() {
                            job.state = event.state;
                            job.phase = event.phase;
                            job.progress = event.progress;
                            job.error = event.error;
                        }
                    }
                    Ok(message) => {
                        finished = Some(message);
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        finished = Some(Message::Done(Err(Error::new(
                            ErrorKind::Integrity,
                            "der Hintergrundvorgang wurde unerwartet beendet",
                        ))));
                        break;
                    }
                }
            }
        }
        if let Some(message) = finished {
            if let Some(mut worker) = self.worker.take()
                && let Some(thread) = worker.thread.take()
            {
                let _ = thread.join();
            }
            match message {
                Message::Prepared(result) => self.finish_planning(result),
                Message::Done(result) => self.finish_running(result),
                Message::Event(_) => {}
            }
        }
        if self.stage == Stage::PlanReady && !self.plan_is_current() {
            self.notice = Some(
                "Die Eingaben wurden geändert. Die Vorschau gilt nicht mehr; bitte die Änderungen erneut prüfen."
                    .to_owned(),
            );
        } else if self.stage == Stage::PlanReady {
            self.notice = None;
        }
    }

    fn error_item(&self, error: &Error) -> ErrorItem {
        ErrorItem {
            repository: self.selected().map(|r| r.name.clone()).unwrap_or_default(),
            kind: error.kind(),
            message: error.message().to_owned(),
            hint: hint_for(error.kind()),
        }
    }

    fn finish_planning(&mut self, result: Result<Box<PreparedSync>, Error>) {
        match result {
            Ok(prepared) => {
                let needs = self.selected().is_some_and(|r| r.needs_credentials);
                self.plan = Some(build_plan_view(&prepared, needs));
                self.prepared = Some(Arc::new(*prepared));
                self.stage = Stage::PlanReady;
            }
            Err(error) => {
                self.stage = Stage::Finished;
                self.planned_for = None;
                self.outcome = Some(if error.kind() == ErrorKind::Cancelled {
                    Outcome::Cancelled
                } else {
                    Outcome::Failed(self.error_item(&error))
                });
            }
        }
    }

    fn finish_running(&mut self, result: Result<ApplyReport, Error>) {
        self.stage = Stage::Finished;
        match result {
            Ok(report) => {
                if let Some(job) = self.job.as_mut() {
                    job.state = JobState::Succeeded;
                }
                self.outcome = Some(Outcome::Succeeded(ReportView {
                    downloaded: report.downloaded,
                    replaced: report.replaced,
                    deleted: report.deleted,
                    kept: report.kept,
                    bytes: report.bytes,
                }));
            }
            Err(error) => {
                if let Some(job) = self.job.as_mut() {
                    job.state = if error.kind() == ErrorKind::Cancelled {
                        JobState::Cancelled
                    } else {
                        JobState::Failed
                    };
                    job.error = Some(error.kind());
                }
                self.outcome = Some(if error.kind() == ErrorKind::Cancelled {
                    Outcome::Cancelled
                } else {
                    Outcome::Failed(self.error_item(&error))
                });
            }
        }
        // The plan was applied or abandoned; a new one must be made before the next run.
        self.prepared = None;
        self.planned_for = None;
    }

    /// A sentence for screen readers announcing the current stage (changes only on stage changes, not per byte).
    #[must_use]
    pub fn announcement(&self) -> String {
        match (self.stage, &self.outcome) {
            (Stage::Choose, _) => "Bereit. Repository und Zielordner wählen.".to_owned(),
            (Stage::Planning, _) => "Änderungen werden geprüft.".to_owned(),
            (Stage::PlanReady, _) => match &self.plan {
                Some(plan) => format!(
                    "Vorschau bereit: {} neu, {} ersetzen, {} löschen.",
                    plan.totals.downloads, plan.totals.replaces, plan.totals.deletes
                ),
                None => "Vorschau bereit.".to_owned(),
            },
            (Stage::Running, _) => "Synchronisation läuft.".to_owned(),
            (Stage::Finished, Some(Outcome::Succeeded(r))) => format!(
                "Abgeschlossen: {} geladen, {} ersetzt, {} gelöscht.",
                r.downloaded, r.replaced, r.deleted
            ),
            (Stage::Finished, Some(Outcome::Cancelled)) => "Abgebrochen.".to_owned(),
            (Stage::Finished, Some(Outcome::Failed(e))) => format!("Fehler: {}", e.message),
            (Stage::Finished, None) => "Beendet.".to_owned(),
        }
    }
}

impl Drop for SyncScreen {
    fn drop(&mut self) {
        if let Some(mut worker) = self.worker.take() {
            worker.cancel.cancel();
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
    }
}
