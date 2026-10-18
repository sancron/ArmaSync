// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned documents on disk: the library (`library.json`) and the settings (`settings.toml`).
//!
//! Both start with `kind` and `schema_version`. A reader checks them *before* interpreting the rest, so a file
//! from a newer ArmaSync is reported as such instead of as a pile of unknown fields, and a foreign file is never
//! mistaken for ours. The document structs below are the storage format (`docs/migration/DATA_MODEL.md`); the core
//! model types stay free of serialisation. Unknown fields, duplicate ids, unknown references and oversized files
//! are rejected, and a file that fails to load never changes what is on disk.

use std::path::{Path, PathBuf};

use armasync_core::{
    CredentialRef, DisplayName, Error, ErrorKind, Language, Library, Profile, ProfileId,
    Repository, RepositoryId, RepositoryUrl, Settings, Theme, UiScale,
};
use serde::{Deserialize, Serialize};

use crate::fsutil::{backup_path, read_limited, write_atomic};

/// Largest accepted document in bytes (a library at the model limits is far smaller).
pub const MAX_DOCUMENT_BYTES: u64 = 4 * 1024 * 1024;
/// `kind` of the library document.
pub const LIBRARY_KIND: &str = "armasync.library";
/// `kind` of the settings document.
pub const SETTINGS_KIND: &str = "armasync.settings";
/// Schema version written by this build and the newest one it can read.
pub const SCHEMA_VERSION: u32 = 1;

fn format_error(path: &Path, what: impl std::fmt::Display) -> Error {
    Error::new(ErrorKind::Format, format!("'{}': {what}", path.display()))
}

/// Checks `kind` and `schema_version` of a parsed document.
fn check_header(
    path: &Path,
    expected_kind: &str,
    kind: Option<&str>,
    version: Option<u64>,
) -> Result<(), Error> {
    if kind != Some(expected_kind) {
        return Err(format_error(
            path,
            format!("not an ArmaSync document of kind '{expected_kind}'"),
        ));
    }
    match version {
        Some(version) if version == u64::from(SCHEMA_VERSION) => Ok(()),
        Some(version) if version > u64::from(SCHEMA_VERSION) => Err(format_error(
            path,
            format!(
                "written by a newer ArmaSync (schema version {version}, this version reads up to {SCHEMA_VERSION}); \
                 update ArmaSync, or restore the backup next to the file"
            ),
        )),
        _ => Err(format_error(path, "missing or unsupported schema version")),
    }
}

/// Shortens a library message that is echoed from the document (defence against very long values).
fn short(message: &str) -> String {
    message.chars().take(200).collect()
}

// ---- library --------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryDocument {
    kind: String,
    schema_version: u32,
    #[serde(default)]
    repositories: Vec<RepositoryDocument>,
    #[serde(default)]
    profiles: Vec<ProfileDocument>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepositoryDocument {
    id: String,
    name: String,
    url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    credential: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileDocument {
    id: String,
    name: String,
    #[serde(default)]
    repositories: Vec<String>,
}

impl From<&Library> for LibraryDocument {
    fn from(library: &Library) -> Self {
        Self {
            kind: LIBRARY_KIND.to_owned(),
            schema_version: SCHEMA_VERSION,
            repositories: library
                .repositories()
                .iter()
                .map(|r| RepositoryDocument {
                    id: r.id.to_string(),
                    name: r.name.to_string(),
                    url: r.url.to_string(),
                    credential: r.credential.as_ref().map(ToString::to_string),
                })
                .collect(),
            profiles: library
                .profiles()
                .iter()
                .map(|p| ProfileDocument {
                    id: p.id.to_string(),
                    name: p.name.to_string(),
                    repositories: p.repositories.iter().map(ToString::to_string).collect(),
                })
                .collect(),
        }
    }
}

fn library_from_document(path: &Path, document: LibraryDocument) -> Result<Library, Error> {
    let invalid = |what: &str, error: &dyn std::fmt::Display| {
        format_error(path, format!("invalid {what}: {error}"))
    };
    let mut repositories = Vec::with_capacity(document.repositories.len());
    for (index, repository) in document.repositories.iter().enumerate() {
        let what = format!("repository #{}", index + 1);
        repositories.push(Repository {
            id: RepositoryId::parse(&repository.id)
                .map_err(|e| invalid(&format!("{what} id"), &e))?,
            name: DisplayName::parse(&repository.name)
                .map_err(|e| invalid(&format!("{what} name"), &e))?,
            url: RepositoryUrl::parse(&repository.url)
                .map_err(|e| invalid(&format!("{what} address"), &e))?,
            credential: repository
                .credential
                .as_deref()
                .map(CredentialRef::parse)
                .transpose()
                .map_err(|e| invalid(&format!("{what} credential reference"), &e))?,
        });
    }
    let mut profiles = Vec::with_capacity(document.profiles.len());
    for (index, profile) in document.profiles.iter().enumerate() {
        let what = format!("profile #{}", index + 1);
        profiles.push(Profile {
            id: ProfileId::parse(&profile.id).map_err(|e| invalid(&format!("{what} id"), &e))?,
            name: DisplayName::parse(&profile.name)
                .map_err(|e| invalid(&format!("{what} name"), &e))?,
            repositories: profile
                .repositories
                .iter()
                .map(|id| RepositoryId::parse(id))
                .collect::<Result<_, _>>()
                .map_err(|e| invalid(&format!("{what} repository id"), &e))?,
        });
    }
    Library::new(repositories, profiles).map_err(|e| format_error(path, e))
}

/// Parses library bytes (for the file `path`, used in messages only).
fn parse_library(path: &Path, bytes: &[u8]) -> Result<Library, Error> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| format_error(path, format!("not valid JSON ({})", short(&e.to_string()))))?;
    check_header(
        path,
        LIBRARY_KIND,
        value.get("kind").and_then(serde_json::Value::as_str),
        value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64),
    )?;
    let document: LibraryDocument =
        serde_json::from_value(value).map_err(|e| format_error(path, short(&e.to_string())))?;
    library_from_document(path, document)
}

/// Reads and writes the library document.
#[derive(Debug, Clone)]
pub struct LibraryStore {
    path: PathBuf,
}

impl LibraryStore {
    /// A store for the document at `path` (nothing is touched until a method is called).
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The document path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The saved library; an empty one if there is no file yet (first start).
    ///
    /// # Errors
    /// [`ErrorKind::Format`] for a damaged, foreign, oversized or too-new document; [`ErrorKind::FileSystem`] for
    /// I/O failures. The file is left untouched; the previous version is at [`LibraryStore::backup_path`].
    pub fn load(&self) -> Result<Library, Error> {
        match read_limited(&self.path, MAX_DOCUMENT_BYTES)? {
            None => Ok(Library::empty()),
            Some(bytes) => parse_library(&self.path, &bytes),
        }
    }

    /// Saves `library` atomically. The previous version moves to the backup file, unless the current file is
    /// itself unreadable (a good backup is never replaced by a damaged file).
    ///
    /// # Errors
    /// [`ErrorKind::FileSystem`]; the previous document stays intact.
    pub fn save(&self, library: &Library) -> Result<(), Error> {
        let mut bytes =
            serde_json::to_vec_pretty(&LibraryDocument::from(library)).map_err(|e| {
                Error::new(ErrorKind::Format, format!("cannot encode the library: {e}"))
            })?;
        bytes.push(b'\n');
        write_atomic(&self.path, &bytes, self.current_is_valid())
    }

    /// Where the previous version is kept.
    #[must_use]
    pub fn backup_path(&self) -> PathBuf {
        backup_path(&self.path)
    }

    /// Loads the backup (the version before the last save), if there is one.
    ///
    /// # Errors
    /// As [`LibraryStore::load`]; `Ok(None)` if there is no backup.
    pub fn load_backup(&self) -> Result<Option<Library>, Error> {
        let path = self.backup_path();
        read_limited(&path, MAX_DOCUMENT_BYTES)?
            .map(|bytes| parse_library(&path, &bytes))
            .transpose()
    }

    fn current_is_valid(&self) -> bool {
        matches!(read_limited(&self.path, MAX_DOCUMENT_BYTES), Ok(Some(bytes)) if parse_library(&self.path, &bytes).is_ok())
    }
}

// ---- settings -------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsDocument {
    kind: String,
    schema_version: u32,
    #[serde(default = "default_language")]
    language: String,
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default = "default_scale")]
    ui_scale_percent: u16,
}

fn default_language() -> String {
    Language::default().code().to_owned()
}
fn default_theme() -> String {
    Theme::default().code().to_owned()
}
fn default_scale() -> u16 {
    UiScale::DEFAULT.percent()
}

fn parse_settings(path: &Path, bytes: &[u8]) -> Result<Settings, Error> {
    let text = std::str::from_utf8(bytes).map_err(|_| format_error(path, "not valid UTF-8"))?;
    let table: toml::Table = toml::from_str(text).map_err(|e| {
        let line = e.span().map_or(0, |span| {
            text[..span.start.min(text.len())].matches('\n').count() + 1
        });
        format_error(
            path,
            format!("not valid TOML (line {line}: {})", short(e.message())),
        )
    })?;
    check_header(
        path,
        SETTINGS_KIND,
        table.get("kind").and_then(toml::Value::as_str),
        table
            .get("schema_version")
            .and_then(toml::Value::as_integer)
            .and_then(|v| u64::try_from(v).ok()),
    )?;
    let document: SettingsDocument = table
        .try_into()
        .map_err(|e: toml::de::Error| format_error(path, short(e.message())))?;
    let invalid = |what: &str| format_error(path, format!("invalid {what}"));
    Ok(Settings {
        language: Language::from_code(&document.language).ok_or_else(|| invalid("language"))?,
        theme: Theme::from_code(&document.theme).ok_or_else(|| invalid("theme"))?,
        ui_scale: UiScale::new(document.ui_scale_percent).map_err(|e| format_error(path, e))?,
    })
}

/// Reads and writes the settings document.
#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    /// A store for the document at `path`.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The document path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The saved settings; the defaults if there is no file yet.
    ///
    /// # Errors
    /// As [`LibraryStore::load`].
    pub fn load(&self) -> Result<Settings, Error> {
        match read_limited(&self.path, MAX_DOCUMENT_BYTES)? {
            None => Ok(Settings::default()),
            Some(bytes) => parse_settings(&self.path, &bytes),
        }
    }

    /// Saves `settings` atomically (previous version kept as backup, as for the library).
    ///
    /// # Errors
    /// [`ErrorKind::FileSystem`]; the previous document stays intact.
    pub fn save(&self, settings: &Settings) -> Result<(), Error> {
        let document = SettingsDocument {
            kind: SETTINGS_KIND.to_owned(),
            schema_version: SCHEMA_VERSION,
            language: settings.language.code().to_owned(),
            theme: settings.theme.code().to_owned(),
            ui_scale_percent: settings.ui_scale.percent(),
        };
        let text = toml::to_string_pretty(&document).map_err(|e| {
            Error::new(
                ErrorKind::Format,
                format!("cannot encode the settings: {e}"),
            )
        })?;
        let valid = matches!(
            read_limited(&self.path, MAX_DOCUMENT_BYTES),
            Ok(Some(bytes)) if parse_settings(&self.path, &bytes).is_ok()
        );
        write_atomic(&self.path, text.as_bytes(), valid)
    }
}

#[cfg(test)]
mod tests {
    use super::{LibraryStore, MAX_DOCUMENT_BYTES, SettingsStore};
    use armasync_core::{
        CredentialRef, DisplayName, ErrorKind, Language, Library, Profile, ProfileId, Repository,
        RepositoryId, RepositoryUrl, Settings, Theme, UiScale,
    };
    use std::fs;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "armasync-store-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample() -> Library {
        let repo = |id: &str, url: &str, credential: Option<&str>| Repository {
            id: RepositoryId::parse(id).unwrap(),
            name: DisplayName::parse(&format!("Repository {id} – Größe")).unwrap(),
            url: RepositoryUrl::parse(url).unwrap(),
            credential: credential.map(|c| CredentialRef::parse(c).unwrap()),
        };
        Library::new(
            vec![
                repo("zeta", "https://zeta.example/repo", Some("cred-zeta")),
                repo("alpha", "sftp://alpha.example:2222/data", None),
            ],
            vec![Profile {
                id: ProfileId::parse("default").unwrap(),
                name: DisplayName::parse("Default").unwrap(),
                repositories: vec![
                    RepositoryId::parse("alpha").unwrap(),
                    RepositoryId::parse("zeta").unwrap(),
                ],
            }],
        )
        .unwrap()
    }

    fn header(extra: &str) -> String {
        format!("{{\"kind\":\"armasync.library\",\"schema_version\":1{extra}}}")
    }

    #[test]
    fn library_round_trips_with_order_and_optional_fields() {
        let dir = scratch("roundtrip");
        let store = LibraryStore::new(dir.join("cfg").join("library.json"));
        assert_eq!(store.load().unwrap(), Library::empty(), "first start");
        let library = sample();
        store.save(&library).unwrap();
        assert_eq!(store.load().unwrap(), library);
        let text = fs::read_to_string(store.path()).unwrap();
        assert!(text.contains("\"schema_version\": 1") && text.contains("armasync.library"));
        assert!(text.ends_with('\n'));
        assert_eq!(
            text.matches("credential").count(),
            1,
            "absent credential is omitted"
        );
        assert!(!text.to_lowercase().contains("password"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn second_save_keeps_the_previous_version_as_backup() {
        let dir = scratch("backup");
        let store = LibraryStore::new(dir.join("library.json"));
        assert_eq!(store.load_backup().unwrap(), None);
        store.save(&sample()).unwrap();
        store.save(&Library::empty()).unwrap();
        assert_eq!(store.load().unwrap(), Library::empty());
        assert_eq!(store.load_backup().unwrap().unwrap(), sample());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn damaged_file_is_rejected_untouched_and_does_not_replace_a_good_backup() {
        let dir = scratch("damaged");
        let store = LibraryStore::new(dir.join("library.json"));
        store.save(&sample()).unwrap();
        store.save(&sample()).unwrap();
        fs::write(store.path(), b"{ this is not json").unwrap();
        let error = store.load().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Format);
        assert_eq!(fs::read(store.path()).unwrap(), b"{ this is not json");
        // Saving over a damaged file must not overwrite the good backup with it.
        store.save(&Library::empty()).unwrap();
        assert_eq!(store.load_backup().unwrap().unwrap(), sample());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_documents_are_rejected_with_a_format_error() {
        let dir = scratch("invalid");
        let store = LibraryStore::new(dir.join("library.json"));
        let repository =
            |id: &str, url: &str| format!("{{\"id\":\"{id}\",\"name\":\"N\",\"url\":\"{url}\"}}");
        let cases: Vec<(&str, String)> = vec![
            (
                "wrong kind",
                "{\"kind\":\"other\",\"schema_version\":1}".to_owned(),
            ),
            ("missing kind", "{\"schema_version\":1}".to_owned()),
            (
                "missing version",
                "{\"kind\":\"armasync.library\"}".to_owned(),
            ),
            (
                "version zero",
                "{\"kind\":\"armasync.library\",\"schema_version\":0}".to_owned(),
            ),
            ("not an object", "[1,2]".to_owned()),
            ("unknown field", header(",\"extra\":true")),
            (
                "unknown repository field",
                header(
                    ",\"repositories\":[{\"id\":\"a\",\"name\":\"N\",\"url\":\"https://a.example/\",\"password\":\"x\"}]",
                ),
            ),
            (
                "bad id",
                header(&format!(
                    ",\"repositories\":[{}]",
                    repository("A B", "https://a.example/")
                )),
            ),
            (
                "url with credentials",
                header(&format!(
                    ",\"repositories\":[{}]",
                    repository("a", "https://u:pw@a.example/")
                )),
            ),
            (
                "duplicate ids",
                header(&format!(
                    ",\"repositories\":[{},{}]",
                    repository("a", "https://a.example/"),
                    repository("a", "https://b.example/")
                )),
            ),
            (
                "unknown reference",
                header(",\"profiles\":[{\"id\":\"p\",\"name\":\"P\",\"repositories\":[\"nope\"]}]"),
            ),
            ("wrong type", header(",\"repositories\":\"not a list\"")),
        ];
        for (name, content) in cases {
            fs::write(store.path(), content).unwrap();
            let error = store.load().expect_err(name);
            assert_eq!(error.kind(), ErrorKind::Format, "{name}: {error}");
            assert!(!error.message().contains("pw@"), "{name}: {error}");
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn newer_schema_versions_are_reported_as_such_even_with_unknown_fields() {
        let dir = scratch("newer");
        let store = LibraryStore::new(dir.join("library.json"));
        fs::write(
            store.path(),
            "{\"kind\":\"armasync.library\",\"schema_version\":2,\"brand_new_field\":{\"x\":1}}",
        )
        .unwrap();
        let error = store.load().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Format);
        assert!(error.message().contains("newer ArmaSync"), "{error}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn oversized_and_non_regular_documents_are_refused() {
        let dir = scratch("size");
        let store = LibraryStore::new(dir.join("library.json"));
        let huge = vec![b' '; usize::try_from(MAX_DOCUMENT_BYTES).unwrap() + 1];
        fs::write(store.path(), huge).unwrap();
        assert_eq!(store.load().unwrap_err().kind(), ErrorKind::Format);
        fs::remove_file(store.path()).unwrap();
        fs::create_dir(store.path()).unwrap();
        assert_eq!(store.load().unwrap_err().kind(), ErrorKind::FileSystem);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn deeply_nested_json_is_rejected_without_crashing() {
        let dir = scratch("deep");
        let store = LibraryStore::new(dir.join("library.json"));
        fs::write(
            store.path(),
            format!("{}1{}", "[".repeat(100_000), "]".repeat(100_000)),
        )
        .unwrap();
        assert_eq!(store.load().unwrap_err().kind(), ErrorKind::Format);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn settings_round_trip_and_defaults() {
        let dir = scratch("settings");
        let store = SettingsStore::new(dir.join("settings.toml"));
        assert_eq!(store.load().unwrap(), Settings::default());
        let settings = Settings {
            language: Language::German,
            theme: Theme::Dark,
            ui_scale: UiScale::new(150).unwrap(),
        };
        store.save(&settings).unwrap();
        assert_eq!(store.load().unwrap(), settings);
        let text = fs::read_to_string(store.path()).unwrap();
        assert!(
            text.contains("schema_version = 1") && text.contains("language = \"de\""),
            "{text}"
        );
        // Missing optional keys fall back to defaults within the same schema version.
        fs::write(
            store.path(),
            "kind = \"armasync.settings\"\nschema_version = 1\ntheme = \"light\"\n",
        )
        .unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.theme, Theme::Light);
        assert_eq!(loaded.language, Language::System);
        assert_eq!(loaded.ui_scale, UiScale::DEFAULT);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_settings_are_rejected() {
        let dir = scratch("settings-invalid");
        let store = SettingsStore::new(dir.join("settings.toml"));
        let head = "kind = \"armasync.settings\"\nschema_version = 1\n";
        for (name, content) in [
            ("not toml", "this = = broken".to_owned()),
            (
                "wrong kind",
                "kind = \"armasync.library\"\nschema_version = 1\n".to_owned(),
            ),
            (
                "newer",
                "kind = \"armasync.settings\"\nschema_version = 9\nfuture = 1\n".to_owned(),
            ),
            (
                "negative version",
                "kind = \"armasync.settings\"\nschema_version = -1\n".to_owned(),
            ),
            ("unknown key", format!("{head}colour = \"red\"\n")),
            ("unknown language", format!("{head}language = \"fr\"\n")),
            ("unknown theme", format!("{head}theme = \"sepia\"\n")),
            ("scale too small", format!("{head}ui_scale_percent = 10\n")),
            (
                "scale wrong type",
                format!("{head}ui_scale_percent = \"big\"\n"),
            ),
        ] {
            fs::write(store.path(), content).unwrap();
            let error = store.load().expect_err(name);
            assert_eq!(error.kind(), ErrorKind::Format, "{name}: {error}");
        }
        fs::write(store.path(), [0xFF, 0xFE, 0x00]).unwrap();
        assert_eq!(store.load().unwrap_err().kind(), ErrorKind::Format);
        fs::remove_dir_all(dir).unwrap();
    }
}
