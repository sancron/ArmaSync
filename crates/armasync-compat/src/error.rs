// SPDX-License-Identifier: AGPL-3.0-only
//! Parse errors. They carry a category and a fixed explanation, never a value from the input, so an error can be
//! shown or logged without leaking anything the file contained (credentials included).

use std::fmt;

/// Why a file was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// The input is not GZIP data.
    NotCompressed,
    /// The GZIP data is damaged (bad checksum, truncated, trailing bytes).
    CompressionCorrupt,
    /// The input or its decompressed form exceeds the limits.
    TooLarge,
    /// The decompressed data is not a serialization stream of the expected version.
    NotSerialization,
    /// The stream ends before the object is complete.
    Truncated,
    /// A structural limit (nesting, number of objects, array length) was exceeded.
    LimitExceeded,
    /// The stream breaks the format rules (bad reference, bad type code, cyclic class chain, ...).
    Malformed,
    /// The stream names a class or class version this reader does not know; nothing is loaded or guessed.
    UnknownClass,
    /// The stream is well-formed but is not the expected kind of document.
    UnexpectedStructure,
    /// A value is present but not acceptable (address, port, name).
    InvalidValue,
    /// The repository uses a transport that is not a repository transport this version can import.
    UnsupportedProtocol,
}

/// A rejected input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError {
    kind: ParseErrorKind,
    detail: &'static str,
}

impl ParseError {
    pub(crate) const fn new(kind: ParseErrorKind, detail: &'static str) -> Self {
        Self { kind, detail }
    }

    /// The category.
    #[must_use]
    pub const fn kind(&self) -> ParseErrorKind {
        self.kind
    }

    /// A fixed explanation (never contains input data).
    #[must_use]
    pub const fn detail(&self) -> &'static str {
        self.detail
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.detail)
    }
}

impl std::error::Error for ParseError {}
