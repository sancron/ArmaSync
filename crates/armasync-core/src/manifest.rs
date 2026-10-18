// SPDX-License-Identifier: AGPL-3.0-only
//! Repository manifest: the neutral description of what a repository revision contains.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use crate::hash::ContentHash;
use crate::path::RelativePath;

/// One file of a repository revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// Safe repository-relative path.
    pub path: RelativePath,
    /// Size in bytes.
    pub size: u64,
    /// Expected content hash.
    pub hash: ContentHash,
}

/// A manifest that cannot be used safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// The revision identifier is empty.
    EmptyRevision,
    /// The same path occurs twice.
    DuplicatePath(RelativePath),
    /// Two paths differ only by case and would collide on Windows.
    CaseCollision(RelativePath, RelativePath),
    /// A path is used both as a file and as a directory of another entry.
    FileDirectoryConflict(RelativePath),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRevision => f.write_str("manifest revision is empty"),
            Self::DuplicatePath(path) => write!(f, "duplicate path '{path}'"),
            Self::CaseCollision(a, b) => write!(f, "paths '{a}' and '{b}' differ only by case"),
            Self::FileDirectoryConflict(path) => {
                write!(f, "'{path}' is both a file and a directory")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

/// An immutable, validated, path-sorted list of files for one revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    revision: String,
    entries: Vec<FileEntry>,
}

impl Manifest {
    /// Validates and sorts the entries.
    ///
    /// # Errors
    /// [`ManifestError`] for an empty revision, duplicate or case-colliding paths, or a file/directory conflict.
    pub fn new(
        revision: impl Into<String>,
        mut entries: Vec<FileEntry>,
    ) -> Result<Self, ManifestError> {
        let revision = revision.into();
        if revision.is_empty() {
            return Err(ManifestError::EmptyRevision);
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let mut by_key: BTreeMap<String, &RelativePath> = BTreeMap::new();
        let mut paths: HashSet<&str> = HashSet::new();
        for entry in &entries {
            if let Some(previous) = by_key.insert(entry.path.collision_key(), &entry.path) {
                return Err(if *previous == entry.path {
                    ManifestError::DuplicatePath(entry.path.clone())
                } else {
                    ManifestError::CaseCollision(previous.clone(), entry.path.clone())
                });
            }
            paths.insert(entry.path.as_str());
        }
        for entry in &entries {
            if entry
                .path
                .ancestors()
                .any(|ancestor| paths.contains(ancestor))
            {
                let parent = entry
                    .path
                    .ancestors()
                    .find(|ancestor| paths.contains(ancestor));
                let conflict = parent
                    .and_then(|text| RelativePath::parse(text).ok())
                    .unwrap_or_else(|| entry.path.clone());
                return Err(ManifestError::FileDirectoryConflict(conflict));
            }
        }
        Ok(Self { revision, entries })
    }

    /// The revision identifier.
    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// The entries, sorted by path.
    #[must_use]
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    /// Looks up an entry by exact path.
    #[must_use]
    pub fn get(&self, path: &RelativePath) -> Option<&FileEntry> {
        self.entries
            .binary_search_by(|entry| entry.path.cmp(path))
            .ok()
            .map(|index| &self.entries[index])
    }
}

#[cfg(test)]
mod tests {
    use super::{FileEntry, Manifest, ManifestError};
    use crate::hash::{ContentHash, HashAlgorithm};
    use crate::path::RelativePath;

    fn entry(path: &str) -> FileEntry {
        FileEntry {
            path: RelativePath::parse(path).unwrap(),
            size: 1,
            hash: ContentHash::new(HashAlgorithm::Sha256, &"0".repeat(64)).unwrap(),
        }
    }

    #[test]
    fn sorts_and_looks_up() {
        let manifest = Manifest::new("r1", vec![entry("b/x.pbo"), entry("a.pbo")]).unwrap();
        let paths: Vec<_> = manifest.entries().iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, ["a.pbo", "b/x.pbo"]);
        assert_eq!(manifest.revision(), "r1");
        assert!(
            manifest
                .get(&RelativePath::parse("b/x.pbo").unwrap())
                .is_some()
        );
        assert!(manifest.get(&RelativePath::parse("c").unwrap()).is_none());
    }

    #[test]
    fn rejects_unsafe_manifests() {
        assert_eq!(
            Manifest::new("", vec![]).unwrap_err(),
            ManifestError::EmptyRevision
        );
        assert!(matches!(
            Manifest::new("r", vec![entry("a.pbo"), entry("a.pbo")]).unwrap_err(),
            ManifestError::DuplicatePath(_)
        ));
        assert!(matches!(
            Manifest::new("r", vec![entry("Mods/A.pbo"), entry("mods/a.pbo")]).unwrap_err(),
            ManifestError::CaseCollision(_, _)
        ));
        assert!(matches!(
            Manifest::new("r", vec![entry("a"), entry("a/b.pbo")]).unwrap_err(),
            ManifestError::FileDirectoryConflict(_)
        ));
        assert!(Manifest::new("r", vec![entry("a-b"), entry("a/b.pbo")]).is_ok());
    }
}
