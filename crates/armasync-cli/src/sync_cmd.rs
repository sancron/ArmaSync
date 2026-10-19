// SPDX-License-Identifier: AGPL-3.0-only
//! `armasync sync plan` and `armasync sync run` (MIG-030D, `docs/migration/CLI_CONTRACT.md`).
//!
//! This module only wires things together (composition root): the repository comes from the library, the HTTP(S)
//! adapter reads its published metadata once, the application use cases plan and apply against the local
//! destination adapter. No sync logic lives here. Rules enforced at this level:
//! * the protected and unsupported paths of the published metadata are **always** part of the plan options, so
//!   they survive any deletion;
//! * extra local files are deleted only with `--delete-extra` **and** at least one explicit `--managed-root`;
//! * `plan` and `run --dry-run` never change anything and open the destination read-only;
//! * the command never prompts, never writes a password anywhere, and never prints credentials.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use armasync_adapters::LibraryStore;
use armasync_adapters::http::HttpConfig;
use armasync_adapters::sync_meta::SyncWarning;
use armasync_application::sync::ApplyReport;
use armasync_core::{
    CancelToken, Error, ErrorKind, EventSink, OperationEvent, OperationId, Phase, PlanWarning,
    RelativePath, RepositoryId, SyncPlan, SyncStep,
};
use armasync_workflow::{SyncRequest, execute as execute_sync, prepare};
use serde_json::{Value, json};

use crate::import_cmd::{config_dir, status_for};
use crate::{ExitStatus, JSON_SCHEMA_VERSION};

/// Parsed arguments of both commands.
#[derive(Debug)]
pub(crate) struct SyncArgs {
    pub run: bool,
    pub repository: String,
    pub destination: PathBuf,
    pub delete_extra: bool,
    pub managed_roots: Vec<String>,
    pub dry_run: bool,
    pub allow_unreleased_platform: bool,
    pub config: Option<PathBuf>,
}

impl SyncArgs {
    const fn command(&self) -> &'static str {
        if self.run { "sync-run" } else { "sync-plan" }
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

/// Prints progress to stderr at most once a second (never in JSON or quiet mode).
struct ProgressPrinter<'a> {
    stderr: &'a mut dyn Write,
    enabled: bool,
    last: Option<Instant>,
}

impl EventSink for ProgressPrinter<'_> {
    fn emit(&mut self, event: OperationEvent) {
        if !self.enabled || event.phase != Phase::Transferring {
            return;
        }
        let due = self
            .last
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(1));
        let finished = event.progress.files_done == event.progress.files_total;
        if due || finished {
            self.last = Some(Instant::now());
            let _ = writeln!(
                self.stderr,
                "transferring: {}/{} files, {} of {}",
                event.progress.files_done,
                event.progress.files_total,
                human_bytes(event.progress.bytes_done),
                human_bytes(event.progress.bytes_total)
            );
        }
    }
}

fn load_repository_url(args: &SyncArgs) -> Result<(RepositoryId, String, bool), Error> {
    let id = RepositoryId::parse(&args.repository).map_err(|_| {
        Error::new(
            ErrorKind::Usage,
            "the repository id is not valid (see `armasync repository list`)",
        )
    })?;
    let config = config_dir(args.config.as_deref())?;
    let library = LibraryStore::new(config.join(armasync_adapters::paths::LIBRARY_FILE)).load()?;
    let repository = library.repository(&id).ok_or_else(|| {
        Error::new(
            ErrorKind::Usage,
            "no repository with this id in the library (see `armasync repository list`)",
        )
    })?;
    Ok((
        id,
        repository.url.as_str().to_owned(),
        repository.credential.is_some(),
    ))
}

fn parse_roots(args: &SyncArgs) -> Result<Vec<RelativePath>, Error> {
    if args.delete_extra && args.managed_roots.is_empty() {
        return Err(Error::new(
            ErrorKind::Usage,
            "--delete-extra needs at least one --managed-root <folder> naming where extra files may be deleted",
        ));
    }
    if !args.delete_extra && !args.managed_roots.is_empty() {
        return Err(Error::new(
            ErrorKind::Usage,
            "--managed-root only makes sense together with --delete-extra",
        ));
    }
    armasync_workflow::parse_managed_roots(&args.managed_roots, args.delete_extra)
}

fn step_json(step: &SyncStep) -> Value {
    match step {
        SyncStep::Download { path, size } => {
            json!({"action": "download", "path": path.as_str(), "size": size})
        }
        SyncStep::Replace { path, size } => {
            json!({"action": "replace", "path": path.as_str(), "size": size})
        }
        SyncStep::Keep { path } => json!({"action": "keep", "path": path.as_str()}),
        SyncStep::Delete { path } => json!({"action": "delete", "path": path.as_str()}),
    }
}

fn plan_warning_text(warning: &PlanWarning) -> String {
    match warning {
        PlanWarning::ExtraKeptOutsideManagedRoots(path) => {
            format!("extra file kept (outside the managed folders): {path}")
        }
        PlanWarning::ExtraKeptProtected(path) => {
            format!("extra file kept (protected by the repository): {path}")
        }
        PlanWarning::CaseMismatch { local, expected } => {
            format!("'{local}' differs from the repository spelling '{expected}' only by case")
        }
    }
}

fn plan_warning_json(warning: &PlanWarning) -> Value {
    match warning {
        PlanWarning::ExtraKeptOutsideManagedRoots(path) => {
            json!({"code": "extra-kept-outside-managed-roots", "path": path.as_str()})
        }
        PlanWarning::ExtraKeptProtected(path) => {
            json!({"code": "extra-kept-protected", "path": path.as_str()})
        }
        PlanWarning::CaseMismatch { local, expected } => {
            json!({"code": "case-mismatch", "local": local.as_str(), "expected": expected.as_str()})
        }
    }
}

fn metadata_warning_text(warning: &SyncWarning) -> String {
    match warning {
        SyncWarning::UnsafeFileName { shown, reason } => {
            format!("a file with an unsafe name was left out: '{shown}' ({reason})")
        }
        SyncWarning::UnsafeFolderName {
            shown,
            reason,
            files_left_out,
        } => format!(
            "a folder with an unsafe name was left out with {files_left_out} file(s): '{shown}' ({reason})"
        ),
        SyncWarning::CompressedFileUnsupported(path) => {
            format!("compressed file not supported yet, left untouched: {path}")
        }
        SyncWarning::ServerInfoPathIgnored => {
            "a folder path of the server information was ignored (not a safe path)".to_owned()
        }
    }
}

fn metadata_warning_json(warning: &SyncWarning) -> Value {
    match warning {
        SyncWarning::UnsafeFileName { shown, reason } => {
            json!({"code": "unsafe-file-name", "name": shown, "reason": reason})
        }
        SyncWarning::UnsafeFolderName {
            shown,
            reason,
            files_left_out,
        } => {
            json!({"code": "unsafe-folder-name", "name": shown, "reason": reason, "files_left_out": files_left_out})
        }
        SyncWarning::CompressedFileUnsupported(path) => {
            json!({"code": "compressed-file-unsupported", "path": path.as_str()})
        }
        SyncWarning::ServerInfoPathIgnored => json!({"code": "server-info-path-ignored"}),
    }
}

fn summary_json(plan: &SyncPlan) -> Value {
    let s = &plan.summary;
    json!({
        "download_files": s.download_files, "download_bytes": s.download_bytes,
        "replace_files": s.replace_files, "replace_bytes": s.replace_bytes,
        "keep_files": s.keep_files, "delete_files": s.delete_files, "extra_untouched": s.extra_untouched,
    })
}

fn fail(
    args: &SyncArgs,
    error: &Error,
    json_output: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitStatus {
    let _ = writeln!(stderr, "error: {error}");
    if json_output {
        let envelope = json!({
            "schema_version": JSON_SCHEMA_VERSION, "command": args.command(), "operation_id": Value::Null,
            "status": "error", "error": {"kind": error.kind().code(), "message": error.message()},
        });
        let _ = writeln!(stdout, "{envelope}");
    }
    status_for(error.kind())
}

struct Outcome {
    repository: RepositoryId,
    revision: String,
    plan: SyncPlan,
    warnings: Vec<SyncWarning>,
    unsupported: Vec<RelativePath>,
    protected: Vec<RelativePath>,
    needs_credentials: bool,
    report: Option<ApplyReport>,
}

fn execute(
    args: &SyncArgs,
    json_output: bool,
    quiet: bool,
    stderr: &mut dyn Write,
) -> Result<Outcome, Error> {
    let roots = parse_roots(args)?;
    let (id, url, needs_credentials) = load_repository_url(args)?;
    let changes = args.run && !args.dry_run;
    if changes {
        let status = armasync_workflow::platform_status();
        armasync_workflow::require_run_allowed(status, args.allow_unreleased_platform)?;
        if !status.run_allowed() {
            let _ = writeln!(
                stderr,
                "warning: continuing on a platform that is not released (--allow-unreleased-platform): {}",
                status.message()
            );
        }
    }
    let request = SyncRequest {
        repository_url: url,
        destination: args.destination.clone(),
        delete_extra: args.delete_extra,
        managed_roots: roots,
        http: HttpConfig::default(),
    };
    let cancel = CancelToken::new();
    let prepared = prepare(&request, &cancel)?;
    let report = if changes {
        let mut printer = ProgressPrinter {
            stderr,
            enabled: !json_output && !quiet,
            last: None,
        };
        Some(execute_sync(
            &prepared,
            OperationId::new(1),
            &mut printer,
            &cancel,
        )?)
    } else {
        None
    };
    let metadata = prepared.metadata();
    Ok(Outcome {
        repository: id,
        revision: prepared.plan().revision.clone(),
        plan: prepared.plan().clone(),
        warnings: metadata.warnings.clone(),
        unsupported: metadata.unsupported.clone(),
        protected: metadata.protected.clone(),
        needs_credentials,
        report,
    })
}

fn render_text(args: &SyncArgs, outcome: &Outcome, out: &mut String) {
    use std::fmt::Write as _;
    let s = &outcome.plan.summary;
    let _ = writeln!(
        out,
        "Repository {} (revision {}) -> {}",
        outcome.repository,
        outcome.revision,
        Path::new(&args.destination).display()
    );
    let _ = writeln!(
        out,
        "Plan: {} to download ({}), {} to replace ({}), {} up to date, {} to delete, {} extra file(s) kept.",
        s.download_files,
        human_bytes(s.download_bytes),
        s.replace_files,
        human_bytes(s.replace_bytes),
        s.keep_files,
        s.delete_files,
        s.extra_untouched
    );
    let mut shown = 0;
    for step in &outcome.plan.steps {
        let line = match step {
            SyncStep::Download { path, size } => {
                format!("  download {path} ({})", human_bytes(*size))
            }
            SyncStep::Replace { path, size } => {
                format!("  replace  {path} ({})", human_bytes(*size))
            }
            SyncStep::Delete { path } => format!("  delete   {path}"),
            SyncStep::Keep { .. } => continue,
        };
        if shown >= 100 {
            let _ = writeln!(out, "  ... more steps are listed with --json");
            break;
        }
        shown += 1;
        let _ = writeln!(out, "{line}");
    }
    for warning in &outcome.warnings {
        let _ = writeln!(out, "warning: {}", metadata_warning_text(warning));
    }
    for warning in &outcome.plan.warnings {
        let _ = writeln!(out, "warning: {}", plan_warning_text(warning));
    }
    if outcome.needs_credentials {
        let _ = writeln!(
            out,
            "note: this repository was marked as needing credentials; authentication is not supported yet and no credentials were sent"
        );
    }
    if let Some(report) = &outcome.report {
        let _ = writeln!(
            out,
            "Done: {} downloaded, {} replaced, {} deleted, {} unchanged, {} transferred.",
            report.downloaded,
            report.replaced,
            report.deleted,
            report.kept,
            human_bytes(report.bytes)
        );
    } else if args.run {
        let _ = writeln!(out, "Dry run: nothing was changed.");
    } else {
        let _ = writeln!(
            out,
            "Plan only: nothing was changed. Use `sync run` to apply it."
        );
    }
}

pub(crate) fn run(
    args: &SyncArgs,
    json_output: bool,
    quiet: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitStatus {
    let outcome = match execute(args, json_output, quiet, stderr) {
        Ok(outcome) => outcome,
        Err(error) => return fail(args, &error, json_output, stdout, stderr),
    };
    let written = if json_output {
        let report = outcome.report.as_ref().map(|r| {
            json!({"downloaded": r.downloaded, "replaced": r.replaced, "deleted": r.deleted,
                   "kept": r.kept, "bytes": r.bytes, "state": "succeeded"})
        });
        writeln!(
            stdout,
            "{}",
            json!({
                "schema_version": JSON_SCHEMA_VERSION, "command": args.command(), "operation_id": Value::Null,
                "status": "ok",
                "result": {
                    "repository": outcome.repository.as_str(),
                    "revision": outcome.revision,
                    "mode": if outcome.report.is_some() { "applied" } else if args.run { "dry-run" } else { "plan" },
                    "changed": outcome.report.is_some_and_changed(),
                    "summary": summary_json(&outcome.plan),
                    "steps": outcome.plan.steps.iter().filter(|s| !matches!(s, SyncStep::Keep { .. })).map(step_json).collect::<Vec<_>>(),
                    "warnings": outcome.plan.warnings.iter().map(plan_warning_json).collect::<Vec<_>>(),
                    "metadata_warnings": outcome.warnings.iter().map(metadata_warning_json).collect::<Vec<_>>(),
                    "unsupported_paths": outcome.unsupported.iter().map(RelativePath::as_str).collect::<Vec<_>>(),
                    "protected_paths": outcome.protected.iter().map(RelativePath::as_str).collect::<Vec<_>>(),
                    "needs_credentials": outcome.needs_credentials,
                    "report": report,
                },
            })
        )
    } else if quiet {
        Ok(())
    } else {
        let mut text = String::new();
        render_text(args, &outcome, &mut text);
        write!(stdout, "{text}")
    };
    if written.is_ok() {
        ExitStatus::Success
    } else {
        ExitStatus::FileSystem
    }
}

trait ReportExt {
    fn is_some_and_changed(&self) -> bool;
}

impl ReportExt for Option<ApplyReport> {
    fn is_some_and_changed(&self) -> bool {
        self.as_ref()
            .is_some_and(|r| r.downloaded + r.replaced + r.deleted > 0)
    }
}
