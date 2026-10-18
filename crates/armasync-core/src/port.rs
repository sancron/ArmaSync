// SPDX-License-Identifier: AGPL-3.0-only
//! Ports: what the application layer needs from the outside world. Adapters implement them.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::Error;
use crate::hash::{ContentHash, HashAlgorithm};
use crate::manifest::Manifest;
use crate::path::RelativePath;
use crate::plan::LocalFile;

/// Read-only source of a repository revision.
pub trait RepositorySource {
    /// The current manifest. May change between calls when the repository is updated.
    ///
    /// # Errors
    /// Network, format or trust failures.
    fn manifest(&self) -> Result<Manifest, Error>;

    /// The full content of one file of the current revision.
    ///
    /// # Errors
    /// Network failures or a missing file.
    fn read(&self, path: &RelativePath) -> Result<Vec<u8>, Error>;
}

/// The local destination.
pub trait LocalStore {
    /// All files below the destination root with their size and (if known) hash.
    ///
    /// # Errors
    /// File-system failures.
    fn list(&self) -> Result<Vec<LocalFile>, Error>;

    /// Writes `data` to `path` **atomically**: afterwards the file has either the old or the complete new content,
    /// never a partial one, even after a crash or cancellation. Missing parent directories are created.
    ///
    /// # Errors
    /// File-system failures; the previous content must remain intact.
    fn write(&mut self, path: &RelativePath, data: &[u8]) -> Result<(), Error>;

    /// Deletes one file below the destination root. Never follows links out of the root.
    ///
    /// # Errors
    /// File-system failures.
    fn delete(&mut self, path: &RelativePath) -> Result<(), Error>;
}

/// Computes content hashes (kept as a port so the core stays free of crypto dependencies).
pub trait Hasher {
    /// Hashes `data`.
    fn hash(&self, algorithm: HashAlgorithm, data: &[u8]) -> ContentHash;
}

/// Cooperative cancellation shared between a frontend and a running operation.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A fresh, not yet cancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation (idempotent, visible to all clones).
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether cancellation was requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::CancelToken;

    #[test]
    fn cancellation_is_shared_between_clones() {
        let token = CancelToken::new();
        let clone = token.clone();
        assert!(!clone.is_cancelled());
        token.cancel();
        token.cancel();
        assert!(clone.is_cancelled());
    }
}
