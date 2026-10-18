// SPDX-License-Identifier: AGPL-3.0-only
//! Safe repository-relative paths.
//!
//! A [`RelativePath`] is a normalised, `/`-separated path that cannot escape its root on Windows or Linux:
//! no absolute or drive paths, no `..`/`.` segments, no backslashes, no alternate data streams, no control
//! characters, no Windows reserved device names and no trailing dot/space segments. Every path that comes from
//! a repository, a legacy file or user input must pass through [`RelativePath::parse`] before it is used.

use std::fmt;

/// Longest accepted path in bytes.
pub const MAX_PATH_BYTES: usize = 1024;

/// Reason a path was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    /// The path is empty.
    Empty,
    /// The path is longer than [`MAX_PATH_BYTES`].
    TooLong,
    /// The path starts with `/` (absolute).
    Absolute,
    /// The path contains a backslash.
    Backslash,
    /// The path contains `:` (drive letter or alternate data stream).
    DriveOrStream,
    /// The path contains a control character (including NUL).
    ControlCharacter,
    /// A segment is empty (`//` or a trailing `/`).
    EmptySegment,
    /// A segment is `.`.
    DotSegment,
    /// A segment is `..`.
    ParentSegment,
    /// A segment ends with `.` or a space (not representable on Windows).
    TrailingDotOrSpace,
    /// A segment is a Windows reserved device name such as `CON` or `NUL` (with or without extension).
    ReservedName,
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Empty => "path is empty",
            Self::TooLong => "path is too long",
            Self::Absolute => "path is absolute",
            Self::Backslash => "path contains a backslash",
            Self::DriveOrStream => "path contains ':' (drive or data stream)",
            Self::ControlCharacter => "path contains a control character",
            Self::EmptySegment => "path contains an empty segment",
            Self::DotSegment => "path contains a '.' segment",
            Self::ParentSegment => "path contains a '..' segment",
            Self::TrailingDotOrSpace => "path segment ends with '.' or a space",
            Self::ReservedName => "path segment is a reserved device name",
        };
        f.write_str(text)
    }
}

impl std::error::Error for PathError {}

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// A validated repository-relative path using `/` as separator.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelativePath(String);

impl RelativePath {
    /// Validates and wraps `input`.
    ///
    /// # Errors
    /// Returns the first [`PathError`] found.
    pub fn parse(input: &str) -> Result<Self, PathError> {
        if input.is_empty() {
            return Err(PathError::Empty);
        }
        if input.len() > MAX_PATH_BYTES {
            return Err(PathError::TooLong);
        }
        if input.starts_with('/') {
            return Err(PathError::Absolute);
        }
        if input.contains('\\') {
            return Err(PathError::Backslash);
        }
        if input.contains(':') {
            return Err(PathError::DriveOrStream);
        }
        if input.chars().any(char::is_control) {
            return Err(PathError::ControlCharacter);
        }
        for segment in input.split('/') {
            match segment {
                "" => return Err(PathError::EmptySegment),
                "." => return Err(PathError::DotSegment),
                ".." => return Err(PathError::ParentSegment),
                _ => {}
            }
            if segment.ends_with('.') || segment.ends_with(' ') {
                return Err(PathError::TrailingDotOrSpace);
            }
            let stem = segment.split('.').next().unwrap_or(segment).to_uppercase();
            if RESERVED.contains(&stem.as_str()) {
                return Err(PathError::ReservedName);
            }
        }
        Ok(Self(input.to_owned()))
    }

    /// The path as `/`-separated text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Key for detecting names that collide on case-insensitive file systems (Windows, default macOS).
    #[must_use]
    pub fn collision_key(&self) -> String {
        self.0.to_lowercase()
    }

    /// Whether `prefix` is this path or one of its parent directories (segment-wise, not textual).
    #[must_use]
    pub fn is_within(&self, prefix: &Self) -> bool {
        self.0 == prefix.0
            || (self.0.starts_with(prefix.as_str()) && self.0[prefix.0.len()..].starts_with('/'))
    }

    /// All parent directories, shortest first (`a/b/c` yields `a`, `a/b`).
    pub fn ancestors(&self) -> impl Iterator<Item = &str> {
        self.0.match_indices('/').map(|(index, _)| &self.0[..index])
    }
}

impl fmt::Display for RelativePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{PathError, RelativePath};

    fn rejected(input: &str) -> PathError {
        RelativePath::parse(input).unwrap_err()
    }

    #[test]
    fn accepts_ordinary_repository_paths() {
        for ok in [
            "addons/cba_main.pbo",
            "@CBA_A3/addons/main.pbo",
            "keys/a.bikey",
            "a",
            "mods/@ace 3/readme.txt",
        ] {
            assert_eq!(RelativePath::parse(ok).unwrap().as_str(), ok);
        }
    }

    #[test]
    fn rejects_path_escapes() {
        assert_eq!(rejected(""), PathError::Empty);
        assert_eq!(rejected("/etc/passwd"), PathError::Absolute);
        assert_eq!(rejected("../outside.pbo"), PathError::ParentSegment);
        assert_eq!(rejected("a/../../b"), PathError::ParentSegment);
        assert_eq!(rejected("a/./b"), PathError::DotSegment);
        assert_eq!(rejected("a\\b"), PathError::Backslash);
        assert_eq!(rejected("..\\x"), PathError::Backslash);
        assert_eq!(rejected("C:/Windows/x"), PathError::DriveOrStream);
        assert_eq!(rejected("file.pbo:stream"), PathError::DriveOrStream);
        assert_eq!(rejected("a//b"), PathError::EmptySegment);
        assert_eq!(rejected("a/"), PathError::EmptySegment);
    }

    #[test]
    fn rejects_unrepresentable_names() {
        assert_eq!(rejected("a/b\u{0}c"), PathError::ControlCharacter);
        assert_eq!(rejected("a/b\nc"), PathError::ControlCharacter);
        assert_eq!(rejected("name."), PathError::TrailingDotOrSpace);
        assert_eq!(rejected("a/name "), PathError::TrailingDotOrSpace);
        assert_eq!(rejected("mods/CON"), PathError::ReservedName);
        assert_eq!(rejected("mods/nul.txt"), PathError::ReservedName);
        assert_eq!(rejected("com1.pbo"), PathError::ReservedName);
        assert!(RelativePath::parse("console.txt").is_ok());
        assert_eq!(rejected(&"a/".repeat(600)), PathError::TooLong);
    }

    #[test]
    fn within_is_segment_wise() {
        let prefix = RelativePath::parse("mods/a").unwrap();
        assert!(RelativePath::parse("mods/a").unwrap().is_within(&prefix));
        assert!(
            RelativePath::parse("mods/a/x.pbo")
                .unwrap()
                .is_within(&prefix)
        );
        assert!(
            !RelativePath::parse("mods/ab/x.pbo")
                .unwrap()
                .is_within(&prefix)
        );
        assert!(!RelativePath::parse("mods").unwrap().is_within(&prefix));
    }

    #[test]
    fn ancestors_and_collision_key() {
        let path = RelativePath::parse("A/b/C.pbo").unwrap();
        assert_eq!(path.ancestors().collect::<Vec<_>>(), ["A", "A/b"]);
        assert_eq!(path.collision_key(), "a/b/c.pbo");
    }
}
