// SPDX-License-Identifier: AGPL-3.0-only
//! Typed reading of a repository's published file tree (`.a3s/sync`) and server information (`.a3s/serverinfo`).
//!
//! The tree says which files a repository contains (name, size, SHA-1) and in which folders. The contract is
//! `docs/migration/SYNC_METADATA.md`. As with the auto-configuration, the class names and version numbers below are
//! **wire data** of the existing format, compared with the descriptors in the stream and never loaded.
//!
//! Names are returned as they are in the file: they are data, not paths. Turning them into safe relative paths is
//! the caller's job (and a separate, mandatory layer). The reader never follows the back reference every node has
//! to its parent, rejects a node that appears twice in the tree (which also rules out cycles and shared-node
//! blow-ups), and bounds nesting depth and the number of nodes.

use std::collections::HashSet;

use crate::error::{ParseError, ParseErrorKind};
use crate::gzip::gunzip;
use crate::stream::{Document, Limits, Object, Value, read_stream};

const DIRECTORY_CLASS: &str = "fr.soe.a3s.domain.repository.SyncTreeDirectory";
const FILE_CLASS: &str = "fr.soe.a3s.domain.repository.SyncTreeLeaf";
const SERVER_INFO_CLASS: &str = "fr.soe.a3s.domain.repository.ServerInfo";
const LIST_CLASS: &str = "java.util.ArrayList";
const ROOT_NAME: &str = "racine";
const MAX_NAME_BYTES: usize = 4096;

/// Wire descriptors accepted in a file tree stream: (name, version number).
const TREE_CLASSES: [(&str, u64); 3] = [
    (DIRECTORY_CLASS, 0xd85f_eb3c_784f_59f8),
    (FILE_CLASS, 0x7ace_d84d_2427_12d7),
    (LIST_CLASS, 0x7881_d21d_99c7_619d),
];

/// Wire descriptors accepted in a server information stream.
const SERVER_INFO_CLASSES: [(&str, u64); 3] = [
    (SERVER_INFO_CLASS, 0x6ad2_1056_c345_a7f9),
    ("java.util.HashSet", 0xba44_8595_96b8_b734),
    ("java.util.Date", 0x686a_8101_4b59_7419),
];

/// One file of the repository as published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncFile {
    /// File name (one path component as published; **not** validated as a path).
    pub name: String,
    /// SHA-1 of the uncompressed content as 40 hexadecimal digits (as published; integrity of the old protocol,
    /// not authenticity).
    pub sha1: String,
    /// Size of the uncompressed content in bytes.
    pub size: u64,
    /// The server stores a compressed variant (`<name>.zip`) instead of the file itself.
    pub compressed: bool,
    /// Size of the compressed variant (0 if not published).
    pub compressed_size: u64,
}

/// A folder of the repository as published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncDirectory {
    /// Folder name (one path component as published; **not** validated as a path).
    pub name: String,
    /// The server marks this folder as hidden.
    pub hidden: bool,
    /// The folder is an addon (mod) folder.
    pub mark_as_addon: bool,
    /// Sub-folders in published order.
    pub directories: Vec<SyncDirectory>,
    /// Files in published order.
    pub files: Vec<SyncFile>,
}

/// The published tree. `root.name` is the format's fixed root marker and is not part of any path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncTree {
    /// The root folder.
    pub root: SyncDirectory,
    /// Number of files in the tree.
    pub file_count: usize,
}

/// Published server information. Only values this version uses; no credentials exist in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    /// Revision number of the repository content.
    pub revision: u64,
    /// Number of files the server reports (informational; it need not equal the tree's count).
    pub number_of_files: u64,
    /// Total size the server reports in bytes (informational).
    pub total_files_size: u64,
    /// The server's suggested number of connections.
    pub number_of_connections: u64,
    /// Folders the server hides (as published; not validated as paths).
    pub hidden_folder_paths: Vec<String>,
}

fn structure(detail: &'static str) -> ParseError {
    ParseError::new(ParseErrorKind::UnexpectedStructure, detail)
}

fn invalid(detail: &'static str) -> ParseError {
    ParseError::new(ParseErrorKind::InvalidValue, detail)
}

fn text<'a>(object: &'a Object, name: &str) -> Result<&'a str, ParseError> {
    match object.field(name) {
        Some(Value::Text(value)) => Ok(value),
        _ => Err(structure("a field is missing or has an unexpected type")),
    }
}

fn flag(object: &Object, name: &str) -> Result<bool, ParseError> {
    match object.field(name) {
        Some(Value::Bool(value)) => Ok(*value),
        _ => Err(structure("a field is missing or has an unexpected type")),
    }
}

fn number(object: &Object, name: &str) -> Result<u64, ParseError> {
    match object.field(name) {
        Some(Value::Int(value)) => {
            u64::try_from(*value).map_err(|_| invalid("a size or count is negative"))
        }
        _ => Err(structure("a field is missing or has an unexpected type")),
    }
}

fn name_of(object: &Object) -> Result<String, ParseError> {
    let name = text(object, "name")?;
    if name.is_empty() || name.len() > MAX_NAME_BYTES {
        return Err(invalid("a name is empty or too long"));
    }
    Ok(name.to_owned())
}

struct Walker<'a> {
    document: &'a Document,
    max_depth: usize,
    seen: HashSet<usize>,
    files: usize,
}

impl Walker<'_> {
    fn directory(&mut self, index: usize, depth: usize) -> Result<SyncDirectory, ParseError> {
        if depth > self.max_depth {
            return Err(ParseError::new(
                ParseErrorKind::LimitExceeded,
                "the tree is nested too deeply",
            ));
        }
        if !self.seen.insert(index) {
            return Err(structure("a node appears more than once in the tree"));
        }
        let object = &self.document.objects[index];
        let mut directory = SyncDirectory {
            name: name_of(object)?,
            hidden: flag(object, "hidden")?,
            mark_as_addon: flag(object, "markAsAddon")?,
            directories: Vec::new(),
            files: Vec::new(),
        };
        let items = match object.field("list") {
            Some(Value::Null) | None => Vec::new(),
            Some(value) => {
                let list = self
                    .document
                    .object(value)
                    .filter(|o| self.document.class_of(o).name == LIST_CLASS)
                    .ok_or_else(|| structure("the child list has an unexpected type"))?;
                list.items.clone()
            }
        };
        for item in &items {
            let Value::Object(child) = item else {
                return Err(structure("the tree contains an unexpected element"));
            };
            let child_object = &self.document.objects[*child];
            match self.document.class_of(child_object).name.as_str() {
                DIRECTORY_CLASS => {
                    directory
                        .directories
                        .push(self.directory(*child, depth + 1)?);
                }
                FILE_CLASS => {
                    if !self.seen.insert(*child) {
                        return Err(structure("a node appears more than once in the tree"));
                    }
                    directory.files.push(file(child_object)?);
                    self.files += 1;
                }
                _ => return Err(structure("the tree contains an unexpected element")),
            }
        }
        Ok(directory)
    }
}

fn file(object: &Object) -> Result<SyncFile, ParseError> {
    let sha1 = text(object, "sha1")?;
    if sha1.len() != 40 || !sha1.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid("a file checksum is not 40 hexadecimal digits"));
    }
    Ok(SyncFile {
        name: name_of(object)?,
        sha1: sha1.to_owned(),
        size: number(object, "size")?,
        compressed: flag(object, "compressed")?,
        compressed_size: number(object, "compressedSize")?,
    })
}

/// Parses the bytes of a `.a3s/sync` file.
///
/// # Errors
/// [`ParseError`] for anything that is not a well-formed, supported file tree within `limits`. Errors never
/// contain data from the input.
pub fn parse_sync_tree(bytes: &[u8], limits: &Limits) -> Result<SyncTree, ParseError> {
    let raw = gunzip(bytes, limits)?;
    let document = read_stream(&raw, limits)?;
    document.ensure_only(&TREE_CLASSES)?;
    let Value::Object(top) = &document.top else {
        return Err(structure("the file is not a repository file tree"));
    };
    let top = *top;
    if document.class_of(&document.objects[top]).name != DIRECTORY_CLASS {
        return Err(structure("the file is not a repository file tree"));
    }
    let mut walker = Walker {
        document: &document,
        max_depth: limits.max_depth,
        seen: HashSet::new(),
        files: 0,
    };
    let root = walker.directory(top, 0)?;
    if root.name != ROOT_NAME {
        return Err(structure("the tree does not start at the expected root"));
    }
    Ok(SyncTree {
        root,
        file_count: walker.files,
    })
}

/// Parses the bytes of a `.a3s/serverinfo` file.
///
/// # Errors
/// [`ParseError`] as for [`parse_sync_tree`].
pub fn parse_serverinfo(bytes: &[u8], limits: &Limits) -> Result<ServerInfo, ParseError> {
    let raw = gunzip(bytes, limits)?;
    let document = read_stream(&raw, limits)?;
    document.ensure_only(&SERVER_INFO_CLASSES)?;
    let top = document
        .object(&document.top)
        .filter(|o| document.class_of(o).name == SERVER_INFO_CLASS)
        .ok_or_else(|| structure("the file is not repository server information"))?;
    let hidden_folder_paths = match top.field("hiddenFolderPaths") {
        Some(Value::Null) | None => Vec::new(),
        Some(value) => {
            let set = document
                .object(value)
                .ok_or_else(|| structure("the hidden folder list has an unexpected type"))?;
            let mut paths = Vec::with_capacity(set.items.len());
            for item in &set.items {
                match item {
                    Value::Text(path) if path.len() <= MAX_NAME_BYTES => paths.push(path.clone()),
                    _ => {
                        return Err(structure(
                            "the hidden folder list has an unexpected element",
                        ));
                    }
                }
            }
            paths.sort();
            paths
        }
    };
    Ok(ServerInfo {
        revision: number(top, "revision")?,
        number_of_files: number(top, "numberOfFiles")?,
        total_files_size: number(top, "totalFilesSize")?,
        number_of_connections: number(top, "numberOfConnections")?,
        hidden_folder_paths,
    })
}
