// SPDX-License-Identifier: AGPL-3.0-only
//! `armasync repository list` and `armasync profile list`: show what is in `library.json`.
//!
//! Read-only: nothing is written and no directory is created. A missing library is an empty one (first start); a
//! damaged library is an error and is never touched. Credentials are never shown, only whether a repository needs
//! them.

use std::io::Write;
use std::path::PathBuf;

use armasync_adapters::LibraryStore;
use armasync_adapters::paths::LIBRARY_FILE;
use armasync_core::{Error, Library};
use serde_json::{Value, json};

use crate::import_cmd::{config_dir, status_for};
use crate::{ExitStatus, JSON_SCHEMA_VERSION};

/// What to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Repositories,
    Profiles,
}

impl Kind {
    const fn command(self) -> &'static str {
        match self {
            Self::Repositories => "repository-list",
            Self::Profiles => "profile-list",
        }
    }
}

fn load(config: Option<&PathBuf>) -> Result<Library, Error> {
    let dir = config_dir(config.map(PathBuf::as_path))?;
    LibraryStore::new(dir.join(LIBRARY_FILE)).load()
}

fn result_json(kind: Kind, library: &Library) -> Value {
    match kind {
        Kind::Repositories => json!({
            "repositories": library.repositories().iter().map(|r| json!({
                "id": r.id.as_str(),
                "name": r.name.as_str(),
                "url": r.url.as_str(),
                "unencrypted": r.url.is_unencrypted(),
                "credentials_required": r.credential.is_some(),
            })).collect::<Vec<_>>(),
        }),
        Kind::Profiles => json!({
            "profiles": library.profiles().iter().map(|p| json!({
                "id": p.id.as_str(),
                "name": p.name.as_str(),
                "repositories": p.repositories.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        }),
    }
}

fn result_text(kind: Kind, library: &Library) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    match kind {
        Kind::Repositories => {
            if library.repositories().is_empty() {
                out.push_str("No repositories yet. Import one with `armasync repository import` or `armasync import`.\n");
            }
            for r in library.repositories() {
                let mut notes = Vec::new();
                if r.credential.is_some() {
                    notes.push("credentials required");
                }
                if r.url.is_unencrypted() {
                    notes.push("unencrypted");
                }
                let notes = if notes.is_empty() {
                    String::new()
                } else {
                    format!("  [{}]", notes.join(", "))
                };
                let _ = writeln!(out, "{}  {}  {}{notes}", r.id, r.name, r.url);
            }
        }
        Kind::Profiles => {
            if library.profiles().is_empty() {
                out.push_str("No profiles yet. Import them with `armasync import`.\n");
            }
            for p in library.profiles() {
                let repositories: Vec<_> = p.repositories.iter().map(|id| id.as_str()).collect();
                let _ = writeln!(
                    out,
                    "{}  {}  repositories: {}",
                    p.id,
                    p.name,
                    if repositories.is_empty() {
                        "none".to_owned()
                    } else {
                        repositories.join(", ")
                    }
                );
            }
        }
    }
    out
}

pub(crate) fn run(
    kind: Kind,
    config: Option<&PathBuf>,
    json_output: bool,
    quiet: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitStatus {
    let library = match load(config) {
        Ok(library) => library,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            if json_output {
                let _ = writeln!(
                    stdout,
                    "{}",
                    json!({
                        "schema_version": JSON_SCHEMA_VERSION, "command": kind.command(), "operation_id": Value::Null,
                        "status": "error", "error": {"kind": error.kind().code(), "message": error.message()},
                    })
                );
            }
            return status_for(error.kind());
        }
    };
    let written = if json_output {
        writeln!(
            stdout,
            "{}",
            json!({
                "schema_version": JSON_SCHEMA_VERSION, "command": kind.command(), "operation_id": Value::Null,
                "status": "ok", "result": result_json(kind, &library),
            })
        )
    } else if quiet {
        Ok(())
    } else {
        write!(stdout, "{}", result_text(kind, &library))
    };
    if written.is_ok() {
        ExitStatus::Success
    } else {
        ExitStatus::FileSystem
    }
}
