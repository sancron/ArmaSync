// SPDX-License-Identifier: AGPL-3.0-only
//! ArmaSync adapters.
//!
//! Persistence (versioned JSON/TOML documents, atomic writes, platform data directories) and credential adapters
//! live here since MIG-020; transport and file-system sync adapters follow behind core ports (P3 and later).
//! OS-specific code stays here, gated by `cfg(target_os)`. For now the crate also provides the SHA-256
//! [`Sha256Hasher`] and in-memory adapters used to prove the ports and to test use cases without I/O.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod archive;
pub mod credentials;
mod fsutil;
pub mod http;
pub mod import;
pub mod localfs;
pub mod memory;
pub mod paths;
pub mod store;
pub mod sync_meta;

pub use credentials::SessionCredentialStore;
pub use memory::{InMemoryRepository, InMemoryStore, MemoryEventSink, Sha256Hasher, ValidatorMode};
pub use paths::{DataRoots, PathMode};
pub use store::{LibraryStore, SettingsStore};

/// Operating systems the project targets first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Windows (synchronisation and game launch).
    Windows,
    /// Linux (synchronisation only for now).
    Linux,
    /// Any other system: not an implicit commitment.
    Other,
}

impl Platform {
    /// The platform this binary was compiled for.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Other
        }
    }

    /// Stable lower-case name used in diagnostics and JSON output.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::Other => "other",
        }
    }

    /// Whether launching the game is a capability of this platform (Windows only for now).
    #[must_use]
    pub const fn can_launch_game(self) -> bool {
        matches!(self, Self::Windows)
    }
}

#[cfg(test)]
mod tests {
    use super::Platform;

    #[test]
    fn current_platform_matches_target() {
        let platform = Platform::current();
        assert_eq!(platform == Platform::Windows, cfg!(target_os = "windows"));
        assert_eq!(platform == Platform::Linux, cfg!(target_os = "linux"));
        assert_eq!(platform.can_launch_game(), platform == Platform::Windows);
        assert!(!platform.name().is_empty());
    }
}
