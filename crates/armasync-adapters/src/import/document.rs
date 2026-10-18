// SPDX-License-Identifier: AGPL-3.0-only
//! The neutral import document (`armasync.import`, schema 1; see `docs/migration/IMPORT_SCHEMA.md`).
//!
//! It is produced by an offline exporter (or by hand) and contains nothing legacy-specific: no old field names,
//! no class names, and by construction no secrets. Parsing is strict: unknown fields, secret-looking keys,
//! credentials in addresses and oversized input are rejected before anything is planned.

use std::collections::HashSet;
use std::path::Path;

use armasync_core::{DisplayName, Error, ErrorKind, RepositoryUrl};
use serde::Deserialize;

/// `kind` of the import document.
pub const IMPORT_KIND: &str = "armasync.import";
/// The only schema version this build reads.
pub const IMPORT_SCHEMA_VERSION: u32 = 1;
/// Largest accepted import document in bytes.
pub const MAX_IMPORT_BYTES: u64 = 4 * 1024 * 1024;
/// Most notes in one document.
pub const MAX_NOTES: usize = 1_000;
const MAX_KEY_CHARS: usize = 200;
const MAX_TEXT_CHARS: usize = 120;

/// Key fragments (lower case) that mark a field as secret-like. The one allowed exception is
/// `needs_credentials`, which is only a flag.
const SECRET_KEY_PARTS: [&str; 8] = [
    "password",
    "passwort",
    "passwd",
    "login",
    "secret",
    "token",
    "apikey",
    "credential",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    kind: String,
    schema_version: u32,
    source: SourceDocument,
    #[serde(default)]
    repositories: Vec<RepositoryDocument>,
    #[serde(default)]
    profiles: Vec<ProfileDocument>,
    #[serde(default)]
    notes: Vec<NoteDocument>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceDocument {
    tool: String,
    tool_version: String,
    format: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepositoryDocument {
    key: String,
    name: String,
    url: String,
    #[serde(default)]
    needs_credentials: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileDocument {
    name: String,
    #[serde(default)]
    repositories: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteDocument {
    code: String,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    count: Option<u32>,
}

/// Who produced the document (informational, shown in reports and the journal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSource {
    /// Producing tool.
    pub tool: String,
    /// Tool version.
    pub tool_version: String,
    /// Description of the source format the tool read.
    pub format: String,
}

/// A repository from the document, validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRepository {
    /// Stable key of the source (profiles refer to it).
    pub key: String,
    /// Display name.
    pub name: DisplayName,
    /// Address without credentials.
    pub url: RepositoryUrl,
    /// The source repository used credentials; the user must enter them again.
    pub needs_credentials: bool,
}

/// A profile from the document, validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportProfile {
    /// Display name.
    pub name: DisplayName,
    /// Keys of the repositories in the user's order.
    pub repositories: Vec<String>,
}

/// An informational note from the exporter (for example an item that was skipped and why).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportNote {
    /// Machine-readable code, `[a-z0-9-]{1,40}`.
    pub code: String,
    /// What the note is about (a name; never a secret).
    pub subject: Option<String>,
    /// How many items it concerns.
    pub count: Option<u32>,
}

/// A validated import document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDocument {
    /// Producer.
    pub source: ImportSource,
    /// Repositories in document order.
    pub repositories: Vec<ImportRepository>,
    /// Profiles in document order.
    pub profiles: Vec<ImportProfile>,
    /// Exporter notes.
    pub notes: Vec<ImportNote>,
}

fn error(path: &Path, what: impl std::fmt::Display) -> Error {
    Error::new(ErrorKind::Format, format!("'{}': {what}", path.display()))
}

fn short(message: &str) -> String {
    message.chars().take(200).collect()
}

fn plain_text(value: &str, max_chars: usize) -> bool {
    !value.is_empty() && value.chars().count() <= max_chars && !value.chars().any(char::is_control)
}

/// Finds the first secret-looking object key anywhere in the document.
fn find_secret_key(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, inner) in map {
                let lower = key.to_lowercase();
                if lower != "needs_credentials"
                    && SECRET_KEY_PARTS.iter().any(|part| lower.contains(part))
                {
                    let printable = key.len() <= 40
                        && key
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
                    return Some(if printable {
                        key.clone()
                    } else {
                        "<invalid name>".to_owned()
                    });
                }
                if let Some(found) = find_secret_key(inner) {
                    return Some(found);
                }
            }
            None
        }
        serde_json::Value::Array(items) => items.iter().find_map(find_secret_key),
        _ => None,
    }
}

/// Parses and validates import bytes. `path` is used in messages only.
///
/// # Errors
/// [`ErrorKind::Format`] for anything that is not a valid, secret-free `armasync.import` document of schema 1.
pub fn parse_import(path: &Path, bytes: &[u8]) -> Result<ImportDocument, Error> {
    if bytes.len() as u64 > MAX_IMPORT_BYTES {
        return Err(error(path, "the import file is too large"));
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| error(path, format!("not valid JSON ({})", short(&e.to_string()))))?;
    if value.get("kind").and_then(serde_json::Value::as_str) != Some(IMPORT_KIND) {
        return Err(error(
            path,
            format!("not an ArmaSync import document (kind '{IMPORT_KIND}' expected)"),
        ));
    }
    match value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
    {
        Some(version) if version == u64::from(IMPORT_SCHEMA_VERSION) => {}
        Some(version) if version > u64::from(IMPORT_SCHEMA_VERSION) => {
            return Err(error(
                path,
                format!(
                    "written for a newer ArmaSync (import schema {version}, this version reads {IMPORT_SCHEMA_VERSION}); update ArmaSync"
                ),
            ));
        }
        _ => return Err(error(path, "missing or unsupported import schema version")),
    }
    if let Some(key) = find_secret_key(&value) {
        return Err(error(
            path,
            format!(
                "the import contains the field '{key}', which looks like a secret; import files must never contain credentials. Export again without them"
            ),
        ));
    }
    let document: Document =
        serde_json::from_value(value).map_err(|e| error(path, short(&e.to_string())))?;
    if document.kind != IMPORT_KIND || document.schema_version != IMPORT_SCHEMA_VERSION {
        return Err(error(path, "inconsistent header"));
    }
    validate(path, document)
}

fn validate(path: &Path, document: Document) -> Result<ImportDocument, Error> {
    let bad = |what: &str, index: usize| error(path, format!("invalid {what} #{}", index + 1));
    let source = &document.source;
    for text in [&source.tool, &source.tool_version, &source.format] {
        if !plain_text(text, MAX_TEXT_CHARS) {
            return Err(error(path, "invalid source description"));
        }
    }
    if document.repositories.len() > armasync_core::model::MAX_REPOSITORIES {
        return Err(error(path, "too many repositories"));
    }
    if document.profiles.len() > armasync_core::model::MAX_PROFILES {
        return Err(error(path, "too many profiles"));
    }
    if document.notes.len() > MAX_NOTES {
        return Err(error(path, "too many notes"));
    }
    let mut keys = HashSet::new();
    let mut repositories = Vec::with_capacity(document.repositories.len());
    for (index, repository) in document.repositories.iter().enumerate() {
        if !plain_text(&repository.key, MAX_KEY_CHARS) {
            return Err(bad("repository key", index));
        }
        if !keys.insert(repository.key.as_str()) {
            return Err(error(
                path,
                format!("repository key used twice (repository #{})", index + 1),
            ));
        }
        repositories.push(ImportRepository {
            key: repository.key.clone(),
            name: DisplayName::parse(&repository.name)
                .map_err(|e| error(path, format!("invalid repository #{} name: {e}", index + 1)))?,
            url: RepositoryUrl::parse(&repository.url).map_err(|e| {
                error(
                    path,
                    format!("invalid repository #{} address: {e}", index + 1),
                )
            })?,
            needs_credentials: repository.needs_credentials,
        });
    }
    let mut profile_names = HashSet::new();
    let mut profiles = Vec::with_capacity(document.profiles.len());
    for (index, profile) in document.profiles.iter().enumerate() {
        if profile.repositories.len() > armasync_core::model::MAX_REPOSITORIES {
            return Err(error(path, "too many repositories in a profile"));
        }
        let mut listed = HashSet::new();
        for key in &profile.repositories {
            if !plain_text(key, MAX_KEY_CHARS) {
                return Err(bad("repository reference in profile", index));
            }
            if !listed.insert(key.as_str()) {
                return Err(error(
                    path,
                    format!("profile #{} lists a repository twice", index + 1),
                ));
            }
        }
        if !profile_names.insert(profile.name.as_str()) {
            return Err(error(
                path,
                format!("profile name used twice (profile #{})", index + 1),
            ));
        }
        profiles.push(ImportProfile {
            name: DisplayName::parse(&profile.name)
                .map_err(|e| error(path, format!("invalid profile #{} name: {e}", index + 1)))?,
            repositories: profile.repositories.clone(),
        });
    }
    let mut notes = Vec::with_capacity(document.notes.len());
    for (index, note) in document.notes.iter().enumerate() {
        let code_ok = !note.code.is_empty()
            && note.code.len() <= 40
            && note
                .code
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        let subject_ok = note
            .subject
            .as_deref()
            .is_none_or(|s| plain_text(s, MAX_TEXT_CHARS));
        if !code_ok || !subject_ok {
            return Err(bad("note", index));
        }
        notes.push(ImportNote {
            code: note.code.clone(),
            subject: note.subject.clone(),
            count: note.count,
        });
    }
    Ok(ImportDocument {
        source: ImportSource {
            tool: document.source.tool,
            tool_version: document.source.tool_version,
            format: document.source.format,
        },
        repositories,
        profiles,
        notes,
    })
}
