// SPDX-License-Identifier: AGPL-3.0-only
//! Import of a repository's published auto-configuration (`.a3s/autoconfig`, MIG-022,
//! `docs/migration/SERVER_METADATA_IMPORT.md`).
//!
//! The bytes are read by the bounded parser in `armasync-compat` (no class loading, no JVM) and turned into the
//! same neutral [`ImportDocument`] the file-based import uses, so planning, conflict handling, backup, atomic
//! commit and journal are shared. Only public connection data is taken over: the repository name and its address.
//! Credentials the file may contain are never read; the repository is flagged `needs_credentials` and the user
//! enters them again. This is not the per-user repository definition (that stays in the encrypted legacy files) and
//! it provides no settings or addon lists.

use std::path::Path;

use armasync_compat::{Limits, ParseErrorKind, ServerAutoConfig, parse_autoconfig};
use armasync_core::{DisplayName, Error, ErrorKind, RepositoryUrl};

use super::document::{ImportDocument, ImportNote, ImportRepository, ImportSource};

/// `source.format` of a document built from an auto-configuration file.
pub const SERVER_AUTOCONFIG_FORMAT: &str = "legacy-server-autoconfig";
/// `source.tool` of a document built from an auto-configuration file.
pub const SERVER_AUTOCONFIG_TOOL: &str = "armasync-server-metadata";
/// Largest accepted auto-configuration file in bytes.
pub const MAX_AUTOCONFIG_BYTES: u64 = Limits::AUTOCONFIG.max_compressed as u64;

fn error(path: &Path, what: impl std::fmt::Display) -> Error {
    Error::new(ErrorKind::Format, format!("'{}': {what}", path.display()))
}

/// The neutral repository address for the published connection data: scheme, host, port (omitted when it is the
/// scheme's default) and path.
fn address(config: &ServerAutoConfig) -> String {
    let (host, path) = config
        .location
        .split_once('/')
        .map_or((config.location.as_str(), None), |(host, path)| {
            (host, Some(path))
        });
    let mut url = format!("{}://{host}", config.protocol.scheme());
    if let Some(port) = config.port.filter(|p| *p != config.protocol.default_port()) {
        url.push_str(&format!(":{port}"));
    }
    if let Some(path) = path {
        url.push('/');
        url.push_str(path);
    }
    url
}

/// Builds the neutral import document from the bytes of an auto-configuration file. `path` is used in messages
/// only. Errors never contain data from the file.
///
/// # Errors
/// [`ErrorKind::Format`] for a file that is not a supported, well-formed auto-configuration (or exceeds the
/// limits); [`ErrorKind::Unsupported`] if it describes a transport that is not a repository transport.
pub fn document_from_autoconfig(path: &Path, bytes: &[u8]) -> Result<ImportDocument, Error> {
    let config = parse_autoconfig(bytes, &Limits::AUTOCONFIG).map_err(|e| {
        let kind = if e.kind() == ParseErrorKind::UnsupportedProtocol {
            ErrorKind::Unsupported
        } else {
            ErrorKind::Format
        };
        let hint = if e.kind() == ParseErrorKind::NotCompressed
            && bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{')
        {
            "; this looks like a neutral import file, use 'armasync import' for it"
        } else {
            ""
        };
        Error::new(kind, format!("'{}': {e}{hint}", path.display()))
    })?;
    let name = DisplayName::parse(&config.repository_name).map_err(|e| {
        error(
            path,
            format!("the published repository name is not usable ({e})"),
        )
    })?;
    let url = RepositoryUrl::parse(&address(&config)).map_err(|e| {
        error(
            path,
            format!("the published repository address is not usable ({e})"),
        )
    })?;
    let mut notes = Vec::new();
    if config.favorite_server_count > 0 {
        notes.push(ImportNote {
            code: "skipped-favorite-servers".to_owned(),
            subject: None,
            count: u32::try_from(config.favorite_server_count).ok(),
        });
    }
    if config.protocol.is_webdav() {
        notes.push(ImportNote {
            code: "webdav-recorded-as-address".to_owned(),
            subject: None,
            count: Some(1),
        });
    }
    Ok(ImportDocument {
        source: ImportSource {
            tool: SERVER_AUTOCONFIG_TOOL.to_owned(),
            tool_version: "1".to_owned(),
            format: SERVER_AUTOCONFIG_FORMAT.to_owned(),
        },
        repositories: vec![ImportRepository {
            key: config.repository_name.clone(),
            name,
            url,
            needs_credentials: config.uses_credentials,
        }],
        profiles: Vec::new(),
        notes,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn fixture(scenario: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/legacy-formats/generated/valid/server")
            .join(scenario)
            .join(".a3s/autoconfig");
        std::fs::read(path).unwrap()
    }

    #[test]
    fn fixture_becomes_a_credential_free_repository() {
        let document =
            document_from_autoconfig(Path::new("autoconfig"), &fixture("small")).unwrap();
        assert_eq!(document.source.tool, SERVER_AUTOCONFIG_TOOL);
        assert_eq!(document.source.format, SERVER_AUTOCONFIG_FORMAT);
        assert!(document.profiles.is_empty());
        let [repository] = document.repositories.as_slice() else {
            panic!("one repository expected");
        };
        assert_eq!(repository.key, "Example HTTP");
        assert_eq!(repository.name.as_str(), "Example HTTP");
        assert_eq!(repository.url.as_str(), "https://repo.example.invalid");
        assert!(repository.needs_credentials);
        let notes: Vec<_> = document
            .notes
            .iter()
            .map(|n| (n.code.as_str(), n.count))
            .collect();
        assert_eq!(notes, [("skipped-favorite-servers", Some(1))]);
        let shown = format!("{document:?}");
        assert!(!shown.contains("fixture-user") && !shown.contains("password"));
    }

    #[test]
    fn a_neutral_import_file_gets_a_pointer_to_the_right_command() {
        let error =
            document_from_autoconfig(Path::new("x"), br#"{"kind":"armasync.import"}"#).unwrap_err();
        assert!(error.message().contains("armasync import"), "{error}");
    }

    #[test]
    fn errors_name_the_file_but_not_its_content() {
        let error = document_from_autoconfig(Path::new("x/autoconfig"), b"not gzip").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Format);
        assert!(error.message().starts_with("'x/autoconfig'"), "{error}");
    }
}
