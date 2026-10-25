// SPDX-License-Identifier: AGPL-3.0-only
//! `armasync upload plan` and `armasync upload publish` (MIG-042, step 4; ADR-0014). The frontend only translates
//! arguments and results; the rules live in `armasync-workflow` and `armasync-application`.

use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use armasync_adapters::http::HttpConfig;
use armasync_application::revision::PublishReport;
use armasync_core::exclusion::ExclusionReason;
use armasync_core::{
    CancelToken, Error, ErrorKind, OperationId, UploadEvent, UploadEventKind, UploadEventSink,
};
use armasync_workflow::upload::{UploadRequest, preview, publish_observed};
use serde_json::{Value, json};

use crate::{ExitStatus, JSON_SCHEMA_VERSION};

/// The arguments of one upload command, as the frontend parsed them. The password is resolved by the caller from
/// the environment and is never printed.
#[derive(Debug, Clone)]
pub struct UploadArgs {
    /// `true` for `publish`, `false` for `plan`.
    pub publish: bool,
    /// The explicit confirmation of `publish` (`--yes`). Without it nothing is published.
    pub confirmed: bool,
    /// The local folder.
    pub source: PathBuf,
    /// The WebDAV folder address.
    pub target: String,
    /// The user name, if any.
    pub user: Option<String>,
    /// The password, if the caller found one.
    pub password: Option<String>,
    /// Explicit exclusions, relative to the source.
    pub exclusions: Vec<String>,
}

/// Runs the upload command and writes its result to `stdout` (text or JSON) and its errors to `stderr`/`stdout`.
pub fn run(
    args: &UploadArgs,
    json_output: bool,
    quiet: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    cancel: &CancelToken,
) -> ExitStatus {
    let command = if args.publish {
        "upload-publish"
    } else {
        "upload-plan"
    };
    if args.publish && !args.confirmed {
        let error = Error::new(
            ErrorKind::Config,
            "publishing needs --yes; this version asks no questions and never waits for input",
        );
        return report_error(command, &error, json_output, stdout, stderr);
    }
    let request = UploadRequest {
        source: args.source.clone(),
        target: args.target.clone(),
        user: args.user.clone(),
        password: args.password.clone(),
        exclusions: args.exclusions.clone(),
        http: HttpConfig::default(),
    };
    if args.publish {
        let show_progress = !json_output && !quiet;
        match publish_with_progress(&request, cancel, show_progress, stderr) {
            Ok(report) => {
                let result = json!({
                    "revision": report.revision,
                    "revision_was_complete": report.revision_was_complete,
                    "files_uploaded": report.files_uploaded,
                    "pointer_changed": report.pointer_changed,
                    "excluded": excluded_json(&report.exclusions),
                    "unused_rules": report.unused_rules.iter().map(|r| r.as_str()).collect::<Vec<_>>(),
                });
                write_success(
                    command,
                    result,
                    json_output,
                    quiet,
                    stdout,
                    publish_text(&report),
                )
            }
            Err(error) => report_error(command, &error, json_output, stdout, stderr),
        }
    } else {
        match preview(&request, cancel) {
            Ok(plan) => {
                let uploads: Vec<Value> = plan
                    .steps
                    .iter()
                    .filter_map(|step| match step {
                        armasync_application::upload::UploadStep::Upload {
                            path,
                            size,
                            replace,
                        } => Some(json!({"path": path.as_str(), "size": size, "replace": replace})),
                        armasync_application::upload::UploadStep::MakeFolder { .. } => None,
                    })
                    .collect();
                let result = json!({
                    "folders": plan.folders,
                    "files": plan.files,
                    "replacements": plan.replacements,
                    "bytes": plan.bytes,
                    "included": uploads,
                    "excluded": excluded_json(&plan.exclusions),
                    "unused_rules": plan.unused_rules.iter().map(|r| r.as_str()).collect::<Vec<_>>(),
                });
                write_success(
                    command,
                    result,
                    json_output,
                    quiet,
                    stdout,
                    plan_text(&plan),
                )
            }
            Err(error) => report_error(command, &error, json_output, stdout, stderr),
        }
    }
}

/// How often the waiting loop checks whether the publication has ended.
const POLL: Duration = Duration::from_millis(50);

/// Sends the events of one publication to the waiting thread (ADR-0016).
struct ChannelSink(mpsc::Sender<UploadEvent>);

impl UploadEventSink for ChannelSink {
    fn emit(&mut self, event: UploadEvent) {
        let _ = self.0.send(event);
    }
}

/// Runs the publication on its own thread and, when `show` is set, writes one line per file to `stderr` as the
/// events arrive. Stdout and the exit status never depend on the events.
fn publish_with_progress(
    request: &UploadRequest,
    cancel: &CancelToken,
    show: bool,
    stderr: &mut dyn Write,
) -> Result<PublishReport, Error> {
    let (sender, receiver) = mpsc::channel();
    thread::scope(|scope| {
        let worker = scope.spawn(move || {
            publish_observed(
                request,
                cancel,
                OperationId::new(1),
                Box::new(ChannelSink(sender)),
            )
        });
        wait_for_events(&worker, &receiver, show, stderr);
        worker.join().unwrap_or_else(|_| {
            Err(Error::new(
                ErrorKind::Integrity,
                "the publication stopped unexpectedly; the target may need a check",
            ))
        })
    })
}

fn wait_for_events(
    worker: &thread::ScopedJoinHandle<'_, Result<PublishReport, Error>>,
    receiver: &Receiver<UploadEvent>,
    show: bool,
    stderr: &mut dyn Write,
) {
    loop {
        match receiver.recv_timeout(POLL) {
            Ok(event) => report_event(&event, show, stderr),
            Err(RecvTimeoutError::Timeout) if worker.is_finished() => break,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    while let Ok(event) = receiver.try_recv() {
        report_event(&event, show, stderr);
    }
}

fn report_event(event: &UploadEvent, show: bool, stderr: &mut dyn Write) {
    if !show {
        return;
    }
    let line = match &event.kind {
        UploadEventKind::FileStarted { path, size } => format!(
            "{}/{} uploading {path} ({size} bytes)",
            event.progress.files_done + 1,
            event.progress.files_total
        ),
        UploadEventKind::FileFinished { path } => format!(
            "{}/{} uploaded {path}",
            event.progress.files_done, event.progress.files_total
        ),
        _ => return,
    };
    let _ = writeln!(stderr, "{line}");
}

fn excluded_json(exclusions: &[armasync_application::exclusion::Exclusion]) -> Vec<Value> {
    exclusions
        .iter()
        .map(|e| {
            let mut entry = json!({
                "path": e.path.as_str(),
                "size": e.size,
                "reason": e.reason.code(),
            });
            if let ExclusionReason::UserRule(rule) = &e.reason {
                entry["rule"] = Value::String(rule.as_str().to_owned());
            }
            entry
        })
        .collect()
}

fn plan_text(plan: &armasync_application::upload::UploadPlan) -> String {
    let mut text = format!(
        "Preview: {} files ({} bytes), {} folders to create, {} to replace.\n",
        plan.files, plan.bytes, plan.folders, plan.replacements
    );
    for exclusion in &plan.exclusions {
        let reason = match &exclusion.reason {
            ExclusionReason::PartialFile => "partial file".to_owned(),
            ExclusionReason::InternalData => "internal ArmaSync data".to_owned(),
            ExclusionReason::UserRule(rule) => format!("excluded by '{}'", rule.as_str()),
        };
        text.push_str(&format!("  excluded: {} ({reason})\n", exclusion.path));
    }
    for rule in &plan.unused_rules {
        text.push_str(&format!("  unused rule: '{rule}' matches no file\n"));
    }
    text
}

fn publish_text(report: &armasync_application::revision::PublishReport) -> String {
    let state = if report.revision_was_complete {
        "already complete on the target"
    } else {
        "uploaded"
    };
    let pointer = if report.pointer_changed {
        "the published pointer now names this revision"
    } else {
        "the published pointer was already current"
    };
    format!(
        "Published revision {}: {} files {state}; {pointer}.\n",
        report.revision, report.files_uploaded
    )
}

fn write_success(
    command: &str,
    result: Value,
    json_output: bool,
    quiet: bool,
    stdout: &mut dyn Write,
    text: String,
) -> ExitStatus {
    let written = if json_output {
        let envelope = json!({
            "schema_version": JSON_SCHEMA_VERSION,
            "command": command,
            "operation_id": Value::Null,
            "status": "ok",
            "result": result,
        });
        writeln!(stdout, "{envelope}")
    } else if quiet {
        Ok(())
    } else {
        write!(stdout, "{text}")
    };
    if written.is_ok() {
        ExitStatus::Success
    } else {
        ExitStatus::FileSystem
    }
}

fn status_of(kind: ErrorKind) -> ExitStatus {
    match kind {
        ErrorKind::Usage => ExitStatus::Usage,
        ErrorKind::Config => ExitStatus::Config,
        ErrorKind::Network => ExitStatus::Network,
        ErrorKind::FileSystem => ExitStatus::FileSystem,
        ErrorKind::Integrity => ExitStatus::Integrity,
        ErrorKind::Format => ExitStatus::Format,
        ErrorKind::Unsupported => ExitStatus::Unsupported,
        ErrorKind::Cancelled => ExitStatus::Cancelled,
    }
}

fn report_error(
    command: &str,
    error: &Error,
    json_output: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitStatus {
    let status = status_of(error.kind());
    if json_output {
        let envelope = json!({
            "schema_version": JSON_SCHEMA_VERSION,
            "command": command,
            "operation_id": Value::Null,
            "status": "error",
            "error": {"kind": error.kind().code(), "message": error.message()},
        });
        let _ = writeln!(stdout, "{envelope}");
    } else {
        let _ = writeln!(stderr, "error: {}", error.message());
    }
    status
}
