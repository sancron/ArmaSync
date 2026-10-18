// SPDX-License-Identifier: AGPL-3.0-only
//! Import of a neutral import document into the library (MIG-021, `docs/migration/IMPORT_SCHEMA.md`).
//!
//! Flow: **preview** (read-only: parse, validate, plan against the current library) and **apply** (plan again
//! against the library as it is *now*, check for cancellation, back up the current library, write the result in one
//! atomic commit, record a journal entry). The source file and the legacy data it came from are only ever read.
//! A damaged existing library is never overwritten. Nothing here handles secrets.

pub mod document;
pub mod plan;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use armasync_core::{CancelToken, Error, ErrorKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use document::{
    IMPORT_KIND, IMPORT_SCHEMA_VERSION, ImportDocument, ImportNote, ImportProfile,
    ImportRepository, ImportSource, MAX_IMPORT_BYTES, parse_import,
};
pub use plan::{
    Action, Counts, ImportPlan, OnConflict, ProfileItem, RepositoryItem, Warning, plan_import,
};

use crate::fsutil::{read_limited, write_atomic};
use crate::paths::LIBRARY_FILE;
use crate::store::LibraryStore;

/// File name of the import journal inside the config root.
pub const JOURNAL_FILE: &str = "import-journal.json";
const JOURNAL_KIND: &str = "armasync.import-journal";
const JOURNAL_SCHEMA_VERSION: u32 = 1;
const JOURNAL_MAX_ENTRIES: usize = 100;
const JOURNAL_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalDocument {
    kind: String,
    schema_version: u32,
    entries: Vec<JournalEntry>,
}

/// One completed import. Contains counts and hashes only, never names, addresses or secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalEntry {
    /// Seconds since the Unix epoch.
    pub imported_at_unix: u64,
    /// SHA-256 of the import file (hex).
    pub source_sha256: String,
    /// Producing tool of the import file.
    pub source_tool: String,
    /// Repositories added.
    pub repositories_added: usize,
    /// Repositories replaced.
    pub repositories_replaced: usize,
    /// Profiles added.
    pub profiles_added: usize,
    /// Profiles replaced.
    pub profiles_replaced: usize,
    /// File name of the pre-import backup, if the library existed.
    pub backup_file: Option<String>,
}

/// The result of a preview.
#[derive(Debug, Clone)]
pub struct Preview {
    /// The plan.
    pub plan: ImportPlan,
    /// SHA-256 of the import file (hex).
    pub source_sha256: String,
    /// A journal entry with the same file hash exists (the same file was imported before).
    pub already_imported: bool,
}

/// The result of applying an import.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The plan that was applied.
    pub plan: ImportPlan,
    /// SHA-256 of the import file (hex).
    pub source_sha256: String,
    /// Whether the library was written (false if everything was already present).
    pub changed: bool,
    /// The pre-import backup of the previous library, if one was made.
    pub backup: Option<PathBuf>,
    /// The journal could not be updated (the import itself is complete); the reason.
    pub journal_warning: Option<String>,
}

/// Imports into the library of one config root.
#[derive(Debug, Clone)]
pub struct Importer {
    config: PathBuf,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

impl Importer {
    /// An importer for the config root `config` (holding `library.json`, the journal and the backups).
    #[must_use]
    pub fn new(config: impl Into<PathBuf>) -> Self {
        Self {
            config: config.into(),
        }
    }

    fn library_store(&self) -> LibraryStore {
        LibraryStore::new(self.config.join(LIBRARY_FILE))
    }

    fn journal_path(&self) -> PathBuf {
        self.config.join(JOURNAL_FILE)
    }

    fn read_source(source: &Path) -> Result<(ImportDocument, String), Error> {
        let bytes = read_limited(source, MAX_IMPORT_BYTES)?.ok_or_else(|| {
            Error::new(
                ErrorKind::Usage,
                format!("the import file '{}' does not exist", source.display()),
            )
        })?;
        let hash = sha256_hex(&bytes);
        Ok((parse_import(source, &bytes)?, hash))
    }

    /// Plans importing `source` without writing anything.
    ///
    /// # Errors
    /// [`ErrorKind::Usage`] for a missing file; [`ErrorKind::Format`] for an invalid import file or a damaged
    /// library; [`ErrorKind::Config`] for a collision under [`OnConflict::Fail`]; [`ErrorKind::FileSystem`].
    pub fn preview(&self, source: &Path, policy: OnConflict) -> Result<Preview, Error> {
        let (document, hash) = Self::read_source(source)?;
        let existing = self.library_store().load()?;
        let plan = plan_import(&existing, &document, policy, &CancelToken::new())?;
        let already_imported = self
            .read_journal()
            .is_ok_and(|entries| entries.iter().any(|e| e.source_sha256 == hash));
        Ok(Preview {
            plan,
            source_sha256: hash,
            already_imported,
        })
    }

    /// Imports `source`.
    ///
    /// The plan is computed again against the library as it is at this moment. Cancellation is honoured until the
    /// commit starts; a cancelled or failed import leaves the library, backups and journal as they were.
    ///
    /// # Errors
    /// As [`Importer::preview`], plus [`ErrorKind::Cancelled`].
    pub fn apply(
        &self,
        source: &Path,
        policy: OnConflict,
        cancel: &CancelToken,
    ) -> Result<Outcome, Error> {
        let (document, hash) = Self::read_source(source)?;
        let store = self.library_store();
        let existing = store.load()?;
        let plan = plan_import(&existing, &document, policy, cancel)?;
        if !plan.changes_library() {
            return Ok(Outcome {
                plan,
                source_sha256: hash,
                changed: false,
                backup: None,
                journal_warning: None,
            });
        }
        if cancel.is_cancelled() {
            return Err(Error::new(
                ErrorKind::Cancelled,
                "the import was cancelled; nothing was changed",
            ));
        }
        let backup = self.backup_library(store.path())?;
        store.save(plan.library())?;
        let journal_warning = self
            .append_journal(&plan, &hash, backup.as_deref())
            .err()
            .map(|e| e.to_string());
        Ok(Outcome {
            plan,
            source_sha256: hash,
            changed: true,
            backup,
            journal_warning,
        })
    }

    /// Copies the current library to `library.pre-import-<unix time>[-n].json.bak`. `None` if there is none yet.
    fn backup_library(&self, library: &Path) -> Result<Option<PathBuf>, Error> {
        let Some(bytes) = read_limited(library, crate::store::MAX_DOCUMENT_BYTES)? else {
            return Ok(None);
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let mut counter = 0u32;
        let target = loop {
            let name = if counter == 0 {
                format!("library.pre-import-{now}.json.bak")
            } else {
                format!("library.pre-import-{now}-{counter}.json.bak")
            };
            let candidate = self.config.join(name);
            if !candidate.exists() {
                break candidate;
            }
            counter += 1;
        };
        write_atomic(&target, &bytes, false)?;
        Ok(Some(target))
    }

    fn read_journal(&self) -> Result<Vec<JournalEntry>, Error> {
        let path = self.journal_path();
        let Some(bytes) = read_limited(&path, JOURNAL_MAX_BYTES)? else {
            return Ok(Vec::new());
        };
        let bad =
            |what: &str| Error::new(ErrorKind::Format, format!("'{}': {what}", path.display()));
        let document: JournalDocument =
            serde_json::from_slice(&bytes).map_err(|_| bad("not a valid import journal"))?;
        if document.kind != JOURNAL_KIND || document.schema_version != JOURNAL_SCHEMA_VERSION {
            return Err(bad("unsupported import journal"));
        }
        Ok(document.entries)
    }

    /// Appends an entry. A journal that cannot be read is left untouched and reported, never overwritten.
    fn append_journal(
        &self,
        plan: &ImportPlan,
        hash: &str,
        backup: Option<&Path>,
    ) -> Result<(), Error> {
        let mut entries = self.read_journal()?;
        let counts = plan.counts();
        entries.push(JournalEntry {
            imported_at_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            source_sha256: hash.to_owned(),
            source_tool: plan.source.tool.clone(),
            repositories_added: counts.repositories_added,
            repositories_replaced: counts.repositories_replaced,
            profiles_added: counts.profiles_added,
            profiles_replaced: counts.profiles_replaced,
            backup_file: backup
                .and_then(Path::file_name)
                .map(|n| n.to_string_lossy().into_owned()),
        });
        if entries.len() > JOURNAL_MAX_ENTRIES {
            let excess = entries.len() - JOURNAL_MAX_ENTRIES;
            entries.drain(..excess);
        }
        let mut bytes = serde_json::to_vec_pretty(&JournalDocument {
            kind: JOURNAL_KIND.to_owned(),
            schema_version: JOURNAL_SCHEMA_VERSION,
            entries,
        })
        .map_err(|e| Error::new(ErrorKind::Format, format!("cannot encode the journal: {e}")))?;
        bytes.push(b'\n');
        write_atomic(&self.journal_path(), &bytes, false)
    }

    /// The recorded imports, oldest first.
    ///
    /// # Errors
    /// [`ErrorKind::Format`] for a damaged journal.
    pub fn journal(&self) -> Result<Vec<JournalEntry>, Error> {
        self.read_journal()
    }

    /// The config root this importer works in.
    #[must_use]
    pub fn config(&self) -> &Path {
        &self.config
    }
}
