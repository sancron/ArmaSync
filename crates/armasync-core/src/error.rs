// SPDX-License-Identifier: AGPL-3.0-only
//! Typed errors. The categories map one-to-one to the CLI exit codes (`docs/migration/CLI_CONTRACT.md`).

use std::fmt;

/// Error category shared by all frontends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// Invalid arguments or usage.
    Usage,
    /// Configuration, missing consent or credentials.
    Config,
    /// Network, authentication or trust failure.
    Network,
    /// File system, permissions or disk space.
    FileSystem,
    /// Integrity or verification failure (hash/size mismatch, stale plan).
    Integrity,
    /// Unknown, unsupported-version or malformed data format.
    Format,
    /// Unsupported platform or transport capability.
    Unsupported,
    /// Explicit cancellation.
    Cancelled,
}

impl ErrorKind {
    /// Stable machine-readable code for JSON and logs.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::Config => "config",
            Self::Network => "network",
            Self::FileSystem => "filesystem",
            Self::Integrity => "integrity",
            Self::Format => "format",
            Self::Unsupported => "unsupported",
            Self::Cancelled => "cancelled",
        }
    }
}

/// An error with a category and a human-readable message.
///
/// The message must never contain passwords, tokens, private URL query strings or other secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    /// Creates an error of `kind`.
    #[must_use]
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// The category.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The message (free of secrets by contract).
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.code(), self.message)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::{Error, ErrorKind};

    #[test]
    fn codes_are_unique_and_stable() {
        let kinds = [
            ErrorKind::Usage,
            ErrorKind::Config,
            ErrorKind::Network,
            ErrorKind::FileSystem,
            ErrorKind::Integrity,
            ErrorKind::Format,
            ErrorKind::Unsupported,
            ErrorKind::Cancelled,
        ];
        let codes: std::collections::HashSet<_> = kinds.iter().map(|kind| kind.code()).collect();
        assert_eq!(codes.len(), kinds.len());
        assert_eq!(ErrorKind::Integrity.code(), "integrity");
    }

    #[test]
    fn display_contains_category_and_message() {
        let error = Error::new(ErrorKind::Network, "connection refused");
        assert_eq!(error.to_string(), "network: connection refused");
        assert_eq!(error.kind(), ErrorKind::Network);
        assert_eq!(error.message(), "connection refused");
    }
}
