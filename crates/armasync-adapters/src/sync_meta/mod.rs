// SPDX-License-Identifier: AGPL-3.0-only
//! Turns the published repository metadata (`.a3s/sync`, `.a3s/serverinfo`) into the core's [`Manifest`]
//! (MIG-030A, `docs/migration/SYNC_METADATA.md`).
//!
//! The readers in `armasync-compat` return names exactly as published; here every name goes through the core's
//! [`RelativePath`] validation. An entry with an unsafe name is **left out with a warning** and is never
//! downloaded; a structurally broken file rejects the whole document.
//!
//! Compressed entries (MIG-030E, ADR-0008) are installable when the published archive size is known and within
//! [`MAX_PACKED_BYTES`]: they stay in the manifest as the unpacked file and are marked with their archive size. A
//! compressed entry without a usable archive size is left out with a warning **and listed in
//! [`SyncMetadata::unsupported`]**: a caller that deletes extra local files must keep those paths, because the
//! uncompressed file may exist locally. Nothing here touches the network or the file system.

use std::collections::{BTreeMap, BTreeSet};

use armasync_compat::{
    Limits, ServerInfo, SyncDirectory, SyncTree, parse_serverinfo, parse_sync_tree,
};
use armasync_core::{
    ContentHash, Error, ErrorKind, FileEntry, HashAlgorithm, Manifest, PlanOptions, RelativePath,
};

use crate::archive::MAX_PACKED_BYTES;

impl SyncMetadata {
    /// Plan options for this repository: extra-file deletion as requested within `managed_roots`, **never** in the
    /// protected folders or at the paths of files this version cannot install (they may exist locally in
    /// uncompressed form). Always use these options (or equivalent protection) when `delete_extra` is on.
    #[must_use]
    pub fn plan_options(
        &self,
        delete_extra: bool,
        managed_roots: Vec<RelativePath>,
    ) -> PlanOptions {
        let mut protected_paths = self.protected.clone();
        protected_paths.extend(self.unsupported.iter().cloned());
        protected_paths.sort();
        protected_paths.dedup();
        PlanOptions {
            delete_extra,
            managed_roots,
            protected_paths,
        }
    }
}

/// Something the user should know about the published metadata. Names are shortened and stripped of control
/// characters; they are never used as paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncWarning {
    /// A file name is not a safe relative path component; the file is not part of the manifest.
    UnsafeFileName {
        /// The name as far as it is printable (shortened).
        shown: String,
        /// Why it was rejected.
        reason: String,
    },
    /// A folder name is not safe; the folder and everything below it are not part of the manifest.
    UnsafeFolderName {
        /// The name as far as it is printable (shortened).
        shown: String,
        /// Why it was rejected.
        reason: String,
        /// Number of files below it that were left out.
        files_left_out: usize,
    },
    /// The server stores this file compressed, but its archive size is unknown or too large; it is left out.
    CompressedFileUnsupported(RelativePath),
    /// A folder-path entry of the server information is not a safe relative path and is ignored.
    ServerInfoPathIgnored,
}

/// Server-reported values (informational).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSummary {
    /// Revision of the repository content.
    pub revision: u64,
    /// Number of files the server reports (need not equal the manifest's count).
    pub number_of_files: u64,
    /// Total size the server reports in bytes.
    pub total_files_size: u64,
    /// The server's suggested number of connections.
    pub number_of_connections: u64,
}

/// The manifest and what to know about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncMetadata {
    /// What a client can install: all safe, uncompressed files, with SHA-1 hashes.
    pub manifest: Manifest,
    /// Observations about left-out or ignored entries.
    pub warnings: Vec<SyncWarning>,
    /// Paths of files that exist on the server in a form this version cannot install (compressed without a usable
    /// archive size). They are not in the manifest; **extra-file deletion must keep them**.
    pub unsupported: Vec<RelativePath>,
    /// Folders in which local extras must never be deleted (folders the server marks hidden, and the hidden
    /// folder paths of the server information).
    pub protected: Vec<RelativePath>,
    /// Server-reported values.
    pub server: ServerSummary,
}

fn shown(name: &str) -> String {
    name.chars()
        .take(80)
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

/// One path component of a published name: safe on its own and not a path in disguise.
fn component(name: &str) -> Result<(), String> {
    if name.contains('/') || name.contains('\\') {
        return Err("the name contains a path separator".to_owned());
    }
    RelativePath::parse(name)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn count_files(dir: &SyncDirectory) -> usize {
    dir.files.len() + dir.directories.iter().map(count_files).sum::<usize>()
}

struct Collector {
    entries: Vec<FileEntry>,
    archives: BTreeMap<RelativePath, u64>,
    warnings: Vec<SyncWarning>,
    unsupported: Vec<RelativePath>,
    protected: Vec<RelativePath>,
}

impl Collector {
    fn walk(&mut self, dir: &SyncDirectory, prefix: &str) {
        for file in &dir.files {
            if let Err(reason) = component(&file.name) {
                self.warnings.push(SyncWarning::UnsafeFileName {
                    shown: shown(&file.name),
                    reason,
                });
                continue;
            }
            let Ok(path) = RelativePath::parse(&format!("{prefix}{}", file.name)) else {
                self.warnings.push(SyncWarning::UnsafeFileName {
                    shown: shown(&file.name),
                    reason: "the resulting path is not safe".to_owned(),
                });
                continue;
            };
            if file.compressed {
                if file.compressed_size == 0 || file.compressed_size > MAX_PACKED_BYTES {
                    self.warnings
                        .push(SyncWarning::CompressedFileUnsupported(path.clone()));
                    self.unsupported.push(path);
                    continue;
                }
                self.archives.insert(path.clone(), file.compressed_size);
            }
            // `parse_sync_tree` guarantees 40 hexadecimal digits.
            let Ok(hash) = ContentHash::new(HashAlgorithm::Sha1, &file.sha1) else {
                continue;
            };
            self.entries.push(FileEntry {
                path,
                size: file.size,
                hash,
            });
        }
        for child in &dir.directories {
            if let Err(reason) = component(&child.name) {
                self.warnings.push(SyncWarning::UnsafeFolderName {
                    shown: shown(&child.name),
                    reason,
                    files_left_out: count_files(child),
                });
                continue;
            }
            let path = format!("{prefix}{}", child.name);
            if child.hidden {
                if let Ok(parsed) = RelativePath::parse(&path) {
                    self.protected.push(parsed);
                }
            }
            self.walk(child, &format!("{path}/"));
        }
    }
}

/// Builds the manifest from an already parsed tree. `revision` identifies the repository state.
///
/// # Errors
/// [`ErrorKind::Format`] if the revision is empty or the safe entries collide (same path, names that differ only
/// in case or Unicode form, a file that is also a folder): nothing is merged silently.
pub fn metadata_from_tree(
    tree: &SyncTree,
    info: &ServerInfo,
    revision: &str,
) -> Result<SyncMetadata, Error> {
    let mut collector = Collector {
        entries: Vec::new(),
        archives: BTreeMap::new(),
        warnings: Vec::new(),
        unsupported: Vec::new(),
        protected: Vec::new(),
    };
    collector.walk(&tree.root, "");
    for path in &info.hidden_folder_paths {
        match RelativePath::parse(&path.replace('\\', "/")) {
            Ok(parsed) => collector.protected.push(parsed),
            Err(_) => collector.warnings.push(SyncWarning::ServerInfoPathIgnored),
        }
    }
    let manifest = Manifest::new(revision, collector.entries)
        .and_then(|manifest| manifest.with_archives(collector.archives))
        .map_err(|e| {
            Error::new(
                ErrorKind::Format,
                format!("the published file list is not usable: {e}"),
            )
        })?;
    let protected: BTreeSet<RelativePath> = collector.protected.into_iter().collect();
    Ok(SyncMetadata {
        manifest,
        warnings: collector.warnings,
        unsupported: collector.unsupported,
        protected: protected.into_iter().collect(),
        server: ServerSummary {
            revision: info.revision,
            number_of_files: info.number_of_files,
            total_files_size: info.total_files_size,
            number_of_connections: info.number_of_connections,
        },
    })
}

/// Reads the bytes of `.a3s/sync` and `.a3s/serverinfo` and builds the manifest. The revision is the server
/// information's revision number.
///
/// # Errors
/// [`ErrorKind::Format`] for files that are not supported, well-formed metadata within the limits or whose entries
/// collide. Messages never contain names or checksums from the files.
pub fn read_sync_metadata(sync: &[u8], serverinfo: &[u8]) -> Result<SyncMetadata, Error> {
    let tree = parse_sync_tree(sync, &Limits::SYNC)
        .map_err(|e| Error::new(ErrorKind::Format, format!("the file list: {e}")))?;
    let info = parse_serverinfo(serverinfo, &Limits::SYNC)
        .map_err(|e| Error::new(ErrorKind::Format, format!("the server information: {e}")))?;
    metadata_from_tree(&tree, &info, &info.revision.to_string())
}
