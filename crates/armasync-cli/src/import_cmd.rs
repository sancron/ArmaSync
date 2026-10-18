// SPDX-License-Identifier: AGPL-3.0-only
//! `armasync import <file> [--apply] [--on-conflict keep-both|fail|replace] [--config <dir>]`.
//!
//! Without `--apply` the command only shows what would happen (nothing is written, not even directories). The
//! command never prompts. Results go to stdout (text or a versioned JSON envelope), diagnostics to stderr.

use std::io::Write;
use std::path::{Path, PathBuf};

use armasync_adapters::import::{Action, ImportPlan, Importer, OnConflict, Warning};
use armasync_adapters::{DataRoots, PathMode};
use armasync_core::{CancelToken, Error, ErrorKind};
use serde_json::{Value, json};

use crate::{ExitStatus, JSON_SCHEMA_VERSION};

/// Parsed arguments of the command.
#[derive(Debug)]
pub(crate) struct ImportArgs {
    pub file: PathBuf,
    pub apply: bool,
    pub on_conflict: OnConflict,
    pub config: Option<PathBuf>,
}

pub(crate) fn status_for(kind: ErrorKind) -> ExitStatus {
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

fn warning_json(warning: &Warning) -> Value {
    match warning {
        Warning::UnresolvedRepository { profile, key } => {
            json!({"code": "unresolved-repository", "profile": profile, "repository_key": key})
        }
        Warning::CredentialsNeeded { repository } => {
            json!({"code": "credentials-needed", "repository": repository.as_str()})
        }
        Warning::Unencrypted { repository } => {
            json!({"code": "unencrypted-transport", "repository": repository.as_str()})
        }
        Warning::NameDiffers { repository } => {
            json!({"code": "name-differs", "repository": repository.as_str()})
        }
    }
}

fn warning_text(warning: &Warning) -> String {
    match warning {
        Warning::UnresolvedRepository { profile, key } => format!(
            "profile '{profile}' refers to repository '{key}', which is not part of the import or the library; it is left out (add the repository and import again with --on-conflict replace)"
        ),
        Warning::CredentialsNeeded { repository } => {
            format!(
                "repository '{repository}' needs credentials; they are never imported, enter them after the import"
            )
        }
        Warning::Unencrypted { repository } => {
            format!("repository '{repository}' uses an unencrypted address (http/ftp)")
        }
        Warning::NameDiffers { repository } => {
            format!(
                "an existing repository '{repository}' with the same address has a different name"
            )
        }
    }
}

fn plan_json(plan: &ImportPlan) -> Value {
    let counts = plan.counts();
    json!({
        "counts": {
            "repositories_added": counts.repositories_added,
            "repositories_unchanged": counts.repositories_unchanged,
            "repositories_replaced": counts.repositories_replaced,
            "profiles_added": counts.profiles_added,
            "profiles_unchanged": counts.profiles_unchanged,
            "profiles_replaced": counts.profiles_replaced,
        },
        "repositories": plan.repositories.iter().map(|i| json!({
            "key": i.key, "id": i.id.as_str(), "action": i.action.name(),
            "renamed_due_to_collision": i.renamed_due_to_collision,
        })).collect::<Vec<_>>(),
        "profiles": plan.profiles.iter().map(|i| json!({
            "name": i.name, "id": i.id.as_str(), "action": i.action.name(),
            "renamed_due_to_collision": i.renamed_due_to_collision,
            "unresolved_repository_keys": i.unresolved,
        })).collect::<Vec<_>>(),
        "warnings": plan.warnings.iter().map(warning_json).collect::<Vec<_>>(),
        "notes": plan.notes.iter().map(|n| json!({"code": n.code, "subject": n.subject, "count": n.count})).collect::<Vec<_>>(),
        "source": {"tool": plan.source.tool, "tool_version": plan.source.tool_version, "format": plan.source.format},
    })
}

fn plan_text(plan: &ImportPlan, out: &mut String) {
    use std::fmt::Write as _;
    let counts = plan.counts();
    let _ = writeln!(
        out,
        "Repositories: {} to add, {} unchanged, {} to replace. Profiles: {} to add, {} unchanged, {} to replace.",
        counts.repositories_added,
        counts.repositories_unchanged,
        counts.repositories_replaced,
        counts.profiles_added,
        counts.profiles_unchanged,
        counts.profiles_replaced
    );
    for item in &plan.repositories {
        let note = if item.renamed_due_to_collision {
            " (id taken, added next to the existing one)"
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "  repository {} -> {} [{}]{note}",
            item.key,
            item.id,
            item.action.name()
        );
    }
    for item in &plan.profiles {
        let note = if item.renamed_due_to_collision {
            " (id taken, added next to the existing one)"
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "  profile {} -> {} [{}]{note}",
            item.name,
            item.id,
            item.action.name()
        );
    }
    for warning in &plan.warnings {
        let _ = writeln!(out, "  warning: {}", warning_text(warning));
    }
    for note in &plan.notes {
        let subject = note
            .subject
            .as_deref()
            .map(|s| format!(" '{s}'"))
            .unwrap_or_default();
        let count = note.count.map(|c| format!(" ({c})")).unwrap_or_default();
        let _ = writeln!(
            out,
            "  note from the exporter: {}{subject}{count}",
            note.code
        );
    }
    let replaced = plan
        .repositories
        .iter()
        .filter(|i| i.action == Action::Replace)
        .count()
        + plan
            .profiles
            .iter()
            .filter(|i| i.action == Action::Replace)
            .count();
    if replaced > 0 {
        let _ = writeln!(out, "  {replaced} existing entries will be overwritten.");
    }
}

fn config_dir(config: Option<&Path>) -> Result<PathBuf, Error> {
    match config {
        Some(dir) => Ok(dir.to_owned()),
        None => Ok(DataRoots::for_current_process(&PathMode::Standard)?
            .config()
            .to_owned()),
    }
}

fn fail(
    error: &Error,
    json_output: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitStatus {
    let _ = writeln!(stderr, "error: {error}");
    if json_output {
        let envelope = json!({
            "schema_version": JSON_SCHEMA_VERSION, "command": "import", "operation_id": Value::Null,
            "status": "error", "error": {"kind": error.kind().code(), "message": error.message()},
        });
        let _ = writeln!(stdout, "{envelope}");
    }
    status_for(error.kind())
}

pub(crate) fn run(
    args: &ImportArgs,
    json_output: bool,
    quiet: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitStatus {
    let config = match config_dir(args.config.as_deref()) {
        Ok(dir) => dir,
        Err(error) => return fail(&error, json_output, stdout, stderr),
    };
    let importer = Importer::new(config);
    // Ctrl-C is not wired yet; the token keeps the cancellation path of the library in use.
    let cancel = CancelToken::new();
    let result = if args.apply {
        importer.apply(&args.file, args.on_conflict, &cancel).map(|outcome| {
            let extra = json!({
                "mode": "apply",
                "changed": outcome.changed,
                "backup_file": outcome.backup.as_deref().and_then(Path::file_name).map(|n| n.to_string_lossy().into_owned()),
                "journal_warning": outcome.journal_warning,
            });
            (outcome.plan, outcome.source_sha256, false, extra, outcome.changed, outcome.backup, outcome.journal_warning)
        })
    } else {
        importer
            .preview(&args.file, args.on_conflict)
            .map(|preview| {
                let extra = json!({"mode": "preview", "changed": false});
                (
                    preview.plan,
                    preview.source_sha256,
                    preview.already_imported,
                    extra,
                    false,
                    None,
                    None,
                )
            })
    };
    let (plan, sha, already, extra, changed, backup, journal_warning) = match result {
        Ok(value) => value,
        Err(error) => return fail(&error, json_output, stdout, stderr),
    };
    if let Some(warning) = &journal_warning {
        let _ = writeln!(
            stderr,
            "warning: the import is complete, but the journal could not be updated: {warning}"
        );
    }
    let written = if json_output {
        let mut result = plan_json(&plan);
        if let (Some(target), Some(extra)) = (result.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                target.insert(key.clone(), value.clone());
            }
            target.insert("source_sha256".to_owned(), json!(sha));
            target.insert("already_imported".to_owned(), json!(already));
            target.insert("on_conflict".to_owned(), json!(args.on_conflict.name()));
        }
        writeln!(
            stdout,
            "{}",
            json!({
                "schema_version": JSON_SCHEMA_VERSION, "command": "import", "operation_id": Value::Null,
                "status": "ok", "result": result,
            })
        )
    } else if quiet {
        Ok(())
    } else {
        let mut text = String::new();
        if args.apply {
            text.push_str(if changed {
                "Import applied.\n"
            } else {
                "Nothing to import: everything is already present.\n"
            });
            if let Some(path) = &backup {
                text.push_str(&format!("Previous library saved as {}\n", path.display()));
            }
        } else {
            text.push_str("Preview only; nothing was changed. Use --apply to import.\n");
            if already {
                text.push_str("This exact file was imported before.\n");
            }
        }
        plan_text(&plan, &mut text);
        write!(stdout, "{text}")
    };
    if written.is_ok() {
        ExitStatus::Success
    } else {
        ExitStatus::FileSystem
    }
}
