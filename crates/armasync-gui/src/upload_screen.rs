// SPDX-License-Identifier: AGPL-3.0-only
//! State and behaviour of the upload screen (MIG-042, step 4b), without drawing.
//!
//! The screen walks the user through: enter the source folder, the WebDAV address and the exclusions, **check the
//! preview** (what is uploaded, what is excluded and why), confirm, publish, and read the result. All work goes
//! through `armasync-workflow::upload` (`preview`, `publish`) — the same functions the command line uses — on a
//! worker thread, so the window never blocks. Nothing here plans, uploads or verifies on its own.
//!
//! Safety properties kept by this model (and tested in `tests/upload_screen.rs`):
//! * publishing needs a preview of **the current inputs** and an explicit confirmation; changing any input after the
//!   preview makes publishing impossible until the preview is checked again;
//! * the password lives only in the form of this session: it is never printed, never persisted, and cleared when a
//!   publication has ended;
//! * an address with credentials, or credentials over plain HTTP, is refused by the workflow and shown as an error;
//! * cancellation is cooperative through the same `CancelToken` the command line uses; a cancelled publication keeps
//!   the previous one (ADR-0012).
//!
//! The screen shows an indeterminate "running" state only. A progress indicator per file needs the event channel of
//! step 4c and is not part of this module.

use std::fmt;
use std::hash::{Hash, Hasher as _};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};

use armasync_adapters::http::HttpConfig;
use armasync_application::revision::PublishReport;
use armasync_application::upload::{UploadPlan, UploadStep};
use armasync_core::exclusion::ExclusionReason;
use armasync_core::{CancelToken, Error, ErrorKind};
use armasync_workflow::upload::{UploadRequest, preview, publish};

use crate::sync_screen::format_bytes;

/// The values of the upload form. The password is held only here, for this session.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct UploadForm {
    /// The local source folder.
    pub source: String,
    /// The WebDAV address, `https://…`.
    pub target: String,
    /// The user name (optional).
    pub user: String,
    /// The password (optional, session only).
    pub password: String,
    /// Exclusions, one relative path per line.
    pub exclusions: String,
}

impl fmt::Debug for UploadForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UploadForm")
            .field("source", &self.source)
            .field("target", &self.target)
            .field("user", &self.user)
            .field("password", &"<redacted>")
            .field("exclusions", &self.exclusions)
            .finish()
    }
}

impl UploadForm {
    /// The request the workflow receives for these values.
    fn request(&self, http: &HttpConfig) -> UploadRequest {
        let trimmed = |text: &str| {
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_owned())
        };
        UploadRequest {
            source: PathBuf::from(self.source.trim()),
            target: self.target.trim().to_owned(),
            user: trimmed(&self.user),
            password: (!self.password.is_empty()).then(|| self.password.clone()),
            exclusions: self.exclusions.lines().filter_map(trimmed_line).collect(),
            http: http.clone(),
        }
    }

    fn fingerprint(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

fn unexpected_end() -> Error {
    Error::new(ErrorKind::Integrity, "the operation ended unexpectedly")
}

fn trimmed_line(line: &str) -> Option<String> {
    let line = line.trim();
    (!line.is_empty()).then(|| line.to_owned())
}

/// Where the screen is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadStage {
    /// Waiting for the user.
    Editing,
    /// The preview is being computed; nothing is written.
    Previewing,
    /// A preview is shown.
    Previewed,
    /// The publication runs.
    Publishing,
    /// The publication finished.
    Published,
    /// Cancelled by the user; the previous publication is unchanged.
    Cancelled,
    /// Ended with an error; the message is shown.
    Failed,
}

/// One file the preview includes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludedRow {
    /// Path below the source folder.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// Whether the target already holds a file there (it is replaced).
    pub replace: bool,
}

/// One file the preview excludes, with the reason in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcludedRow {
    /// Path below the source folder.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// The reason, for the user.
    pub reason: String,
}

/// The preview as the screen shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewView {
    /// Files to upload.
    pub included: Vec<IncludedRow>,
    /// Files that are excluded, with their reasons.
    pub excluded: Vec<ExcludedRow>,
    /// Exclusion rules that match no file.
    pub unused_rules: Vec<String>,
    /// Folders to create on the target.
    pub folders: u64,
    /// Files that replace an existing target file.
    pub replacements: u64,
    /// Total bytes to send.
    pub bytes: u64,
}

impl PreviewView {
    fn from_plan(plan: &UploadPlan) -> Self {
        let included = plan
            .steps
            .iter()
            .filter_map(|step| match step {
                UploadStep::Upload {
                    path,
                    size,
                    replace,
                } => Some(IncludedRow {
                    path: path.as_str().to_owned(),
                    size: *size,
                    replace: *replace,
                }),
                UploadStep::MakeFolder { .. } => None,
            })
            .collect();
        let excluded = plan
            .exclusions
            .iter()
            .map(|e| ExcludedRow {
                path: e.path.as_str().to_owned(),
                size: e.size,
                reason: reason_text(&e.reason),
            })
            .collect();
        Self {
            included,
            excluded,
            unused_rules: plan
                .unused_rules
                .iter()
                .map(|rule| rule.as_str().to_owned())
                .collect(),
            folders: plan.folders,
            replacements: plan.replacements,
            bytes: plan.bytes,
        }
    }
}

/// The reason of an exclusion, in words.
#[must_use]
pub fn reason_text(reason: &ExclusionReason) -> String {
    match reason {
        ExclusionReason::PartialFile => "Teildatei eines unterbrochenen Downloads".to_owned(),
        ExclusionReason::InternalData => "Interne ArmaSync-Daten".to_owned(),
        ExclusionReason::UserRule(rule) => {
            format!("ausgeschlossen durch »{}«", rule.as_str())
        }
    }
}

/// The result of a publication as the screen shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishView {
    /// The published revision.
    pub revision: String,
    /// Files uploaded (created and replaced).
    pub files_uploaded: u64,
    /// Whether the pointer was moved by this run.
    pub pointer_changed: bool,
    /// Whether the revision was complete on the target already.
    pub revision_was_complete: bool,
}

impl PublishView {
    fn from_report(report: &PublishReport) -> Self {
        Self {
            revision: report.revision.clone(),
            files_uploaded: report.files_uploaded,
            pointer_changed: report.pointer_changed,
            revision_was_complete: report.revision_was_complete,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobKind {
    Preview,
    Publish,
}

enum Outcome {
    Plan(Result<UploadPlan, Error>),
    Published(Result<PublishReport, Error>),
}

struct Job {
    receiver: Receiver<Outcome>,
    cancel: CancelToken,
    handle: Option<JoinHandle<()>>,
    kind: JobKind,
    /// The form values the job works on (for the preview: what it was computed for).
    fingerprint: u64,
}

/// The upload screen's state.
pub struct UploadScreen {
    /// The form, edited by the view.
    pub form: UploadForm,
    /// Whether the user confirmed this preview's publication.
    confirmed: bool,
    http: HttpConfig,
    stage: UploadStage,
    preview: Option<PreviewView>,
    preview_fingerprint: Option<u64>,
    published: Option<PublishView>,
    error: Option<String>,
    job: Option<Job>,
    repaint: Arc<dyn Fn() + Send + Sync>,
}

impl fmt::Debug for UploadScreen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UploadScreen")
            .field("form", &self.form)
            .field("stage", &self.stage)
            .field("busy", &self.is_busy())
            .finish_non_exhaustive()
    }
}

impl UploadScreen {
    /// A new screen. `repaint` is called from the worker thread when a result is ready.
    pub fn new(repaint: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            form: UploadForm::default(),
            confirmed: false,
            http: HttpConfig::default(),
            stage: UploadStage::Editing,
            preview: None,
            preview_fingerprint: None,
            published: None,
            error: None,
            job: None,
            repaint: Arc::new(repaint),
        }
    }

    /// The current stage.
    #[must_use]
    pub fn stage(&self) -> UploadStage {
        self.stage
    }

    /// Whether a worker is running.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    /// The preview, if one exists.
    #[must_use]
    pub fn preview(&self) -> Option<&PreviewView> {
        self.preview.as_ref()
    }

    /// The result of the last publication, if it finished.
    #[must_use]
    pub fn published(&self) -> Option<&PublishView> {
        self.published.as_ref()
    }

    /// The error message of the last failed step, if any. It never contains a password.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether the user has confirmed the publication.
    #[must_use]
    pub fn confirmed(&self) -> bool {
        self.confirmed
    }

    /// Records the confirmation checkbox.
    pub fn set_confirmed(&mut self, confirmed: bool) {
        self.confirmed = confirmed;
    }

    /// Whether the preview was computed for the current form values.
    #[must_use]
    pub fn preview_is_current(&self) -> bool {
        self.preview.is_some() && self.preview_fingerprint == Some(self.form.fingerprint())
    }

    /// Starts the preview in the background. Nothing is written anywhere.
    pub fn request_preview(&mut self) {
        if self.is_busy() {
            return;
        }
        self.error = None;
        self.published = None;
        self.preview = None;
        self.preview_fingerprint = None;
        self.confirmed = false;
        let request = self.form.request(&self.http);
        self.start(JobKind::Preview, request, self.form.fingerprint());
        self.stage = UploadStage::Previewing;
    }

    /// Why publishing is not possible right now, in words; `None` when it is possible.
    #[must_use]
    pub fn publish_blocker(&self) -> Option<&'static str> {
        if self.is_busy() {
            return Some("Ein Vorgang läuft bereits.");
        }
        if self.preview.is_none() {
            return Some(
                "Zuerst die Vorschau prüfen: veröffentlicht wird nur, was in der Vorschau steht.",
            );
        }
        if !self.preview_is_current() {
            return Some(
                "Die Eingaben haben sich seit der Vorschau geändert. Bitte die Vorschau erneut prüfen.",
            );
        }
        if !self.confirmed {
            return Some("Bitte die Veröffentlichung ausdrücklich bestätigen.");
        }
        None
    }

    /// Starts the publication of the previewed files, if the preview is current and confirmed.
    pub fn request_publish(&mut self) {
        if self.publish_blocker().is_some() {
            return;
        }
        self.error = None;
        let request = self.form.request(&self.http);
        self.start(JobKind::Publish, request, self.form.fingerprint());
        self.stage = UploadStage::Publishing;
    }

    /// Asks the running operation to stop after the current step. The result arrives through [`Self::poll`].
    pub fn cancel(&mut self) {
        if let Some(job) = &self.job {
            job.cancel.cancel();
        }
    }

    /// Returns to editing: forgets the preview and the result, keeps the form.
    pub fn reset(&mut self) {
        if self.is_busy() {
            return;
        }
        self.stage = UploadStage::Editing;
        self.preview = None;
        self.preview_fingerprint = None;
        self.published = None;
        self.error = None;
        self.confirmed = false;
    }

    /// Takes a finished result from the worker, if there is one. Called once per frame.
    pub fn poll(&mut self) {
        let Some(job) = self.job.as_ref() else {
            return;
        };
        let received = job.receiver.try_recv();
        let kind = job.kind;
        let fingerprint = job.fingerprint;
        let outcome = match received {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => match kind {
                JobKind::Preview => Outcome::Plan(Err(unexpected_end())),
                JobKind::Publish => Outcome::Published(Err(unexpected_end())),
            },
        };
        let Some(mut job) = self.job.take() else {
            return;
        };
        if let Some(handle) = job.handle.take() {
            let _ = handle.join();
        }
        match outcome {
            Outcome::Plan(Ok(plan)) => {
                self.preview = Some(PreviewView::from_plan(&plan));
                self.preview_fingerprint = Some(fingerprint);
                self.stage = UploadStage::Previewed;
            }
            Outcome::Plan(Err(error)) => self.fail(&error),
            Outcome::Published(Ok(report)) => {
                self.published = Some(PublishView::from_report(&report));
                self.stage = UploadStage::Published;
                self.finish_publication();
            }
            Outcome::Published(Err(error)) => {
                self.fail(&error);
                self.finish_publication();
            }
        }
    }

    fn fail(&mut self, error: &Error) {
        if error.kind() == ErrorKind::Cancelled {
            self.stage = UploadStage::Cancelled;
            self.error = None;
        } else {
            self.stage = UploadStage::Failed;
            self.error = Some(error.message().to_owned());
        }
        if self.stage == UploadStage::Cancelled {
            self.preview = None;
            self.preview_fingerprint = None;
        }
    }

    /// After any publication: the preview is used up, the confirmation is withdrawn, the password is forgotten.
    fn finish_publication(&mut self) {
        self.preview = None;
        self.preview_fingerprint = None;
        self.confirmed = false;
        self.form.password.clear();
    }

    fn start(&mut self, kind: JobKind, request: UploadRequest, fingerprint: u64) {
        let (sender, receiver) = mpsc::channel();
        let cancel = CancelToken::new();
        let worker_cancel = cancel.clone();
        let repaint = Arc::clone(&self.repaint);
        let handle = thread::spawn(move || {
            let outcome = match kind {
                JobKind::Preview => Outcome::Plan(preview(&request, &worker_cancel)),
                JobKind::Publish => Outcome::Published(publish(&request, &worker_cancel)),
            };
            let _ = sender.send(outcome);
            repaint();
        });
        self.job = Some(Job {
            receiver,
            cancel,
            handle: Some(handle),
            kind,
            fingerprint,
        });
    }

    /// The status sentence for the status line.
    #[must_use]
    pub fn announcement(&self) -> String {
        match self.stage {
            UploadStage::Editing => {
                "Bereit. Quellordner und Zieladresse angeben, dann die Vorschau prüfen.".to_owned()
            }
            UploadStage::Previewing => {
                "Vorschau wird erstellt … (es wird nichts veröffentlicht)".to_owned()
            }
            UploadStage::Previewed => match &self.preview {
                Some(preview) => format!(
                    "Vorschau bereit: {} Dateien ({}), {} ausgeschlossen.",
                    preview.included.len(),
                    format_bytes(preview.bytes),
                    preview.excluded.len()
                ),
                None => "Vorschau bereit.".to_owned(),
            },
            UploadStage::Publishing => {
                "Veröffentlichung läuft … Abbruch ist möglich; die bisherige Veröffentlichung bleibt bis zum Ende unverändert."
                    .to_owned()
            }
            UploadStage::Published => match &self.published {
                Some(published) => format!(
                    "Veröffentlicht: Revision {}, {} Dateien hochgeladen.",
                    published.revision, published.files_uploaded
                ),
                None => "Veröffentlicht.".to_owned(),
            },
            UploadStage::Cancelled => {
                "Abgebrochen. Die bisherige Veröffentlichung bleibt unverändert.".to_owned()
            }
            UploadStage::Failed => match &self.error {
                Some(message) => format!("Fehler: {message}"),
                None => "Fehler.".to_owned(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_form_debug_output_hides_the_password() {
        let form = UploadForm {
            password: "topsecret-value".to_owned(),
            ..UploadForm::default()
        };
        let debug = format!("{form:?}");
        assert!(!debug.contains("topsecret-value"), "{debug}");
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn exclusion_lines_are_trimmed_and_empty_lines_dropped() {
        let form = UploadForm {
            exclusions: "  docs \n\n tmp/x.txt \n".to_owned(),
            ..UploadForm::default()
        };
        assert_eq!(
            form.request(&HttpConfig::default()).exclusions,
            ["docs", "tmp/x.txt"]
        );
    }

    #[test]
    fn publishing_is_blocked_until_a_preview_is_confirmed() {
        let screen = UploadScreen::new(|| {});
        assert_eq!(
            screen.publish_blocker(),
            Some("Zuerst die Vorschau prüfen: veröffentlicht wird nur, was in der Vorschau steht.")
        );
        assert_eq!(screen.stage(), UploadStage::Editing);
    }

    #[test]
    fn reasons_are_named_in_words() {
        assert_eq!(
            reason_text(&ExclusionReason::InternalData),
            "Interne ArmaSync-Daten"
        );
        assert_eq!(
            reason_text(&ExclusionReason::PartialFile),
            "Teildatei eines unterbrochenen Downloads"
        );
    }
}
