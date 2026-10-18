// SPDX-License-Identifier: AGPL-3.0-only
//! Credential adapters.
//!
//! [`SessionCredentialStore`] keeps credentials in process memory only: they vanish when the process ends and are
//! never written to disk. It serves headless runs (the CLI hands credentials over for one run) and tests. A
//! persistent adapter for the operating-system keyring is a separate, platform-tested step (ADR-0005); until then
//! nothing in ArmaSync stores a secret on disk.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use armasync_core::{Credential, CredentialRef, CredentialStore, Error};

/// In-memory credential store.
#[derive(Debug, Default)]
pub struct SessionCredentialStore {
    entries: Mutex<HashMap<CredentialRef, Credential>>,
}

impl SessionCredentialStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn entries(&self) -> MutexGuard<'_, HashMap<CredentialRef, Credential>> {
        // A panic while holding the lock cannot leave the map half-updated (single insert/remove calls).
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl CredentialStore for SessionCredentialStore {
    fn get(&self, reference: &CredentialRef) -> Result<Option<Credential>, Error> {
        Ok(self.entries().get(reference).cloned())
    }

    fn put(&self, reference: &CredentialRef, credential: Credential) -> Result<(), Error> {
        self.entries().insert(reference.clone(), credential);
        Ok(())
    }

    fn delete(&self, reference: &CredentialRef) -> Result<(), Error> {
        self.entries().remove(reference);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SessionCredentialStore;
    use armasync_core::{Credential, CredentialRef, CredentialStore, Secret};

    fn reference(name: &str) -> CredentialRef {
        CredentialRef::parse(name).unwrap()
    }

    #[test]
    fn put_get_replace_delete() {
        let store = SessionCredentialStore::new();
        let key = reference("cred-1");
        assert!(store.get(&key).unwrap().is_none());
        store
            .put(&key, Credential::new("alice", Secret::new("one")).unwrap())
            .unwrap();
        store
            .put(&key, Credential::new("alice", Secret::new("two")).unwrap())
            .unwrap();
        let found = store.get(&key).unwrap().unwrap();
        assert_eq!(found.username(), "alice");
        assert_eq!(found.secret().expose(), "two");
        assert!(store.get(&reference("other")).unwrap().is_none());
        store.delete(&key).unwrap();
        store.delete(&key).unwrap();
        assert!(store.get(&key).unwrap().is_none());
    }

    #[test]
    fn debug_output_of_the_store_never_shows_secrets() {
        let store = SessionCredentialStore::new();
        store
            .put(
                &reference("c"),
                Credential::new("bob", Secret::new("TOPSECRET")).unwrap(),
            )
            .unwrap();
        assert!(!format!("{store:?}").contains("TOPSECRET"));
    }

    #[test]
    fn the_store_is_shareable_between_threads() {
        let store = std::sync::Arc::new(SessionCredentialStore::new());
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let store = std::sync::Arc::clone(&store);
                std::thread::spawn(move || {
                    let key = reference(&format!("c{i}"));
                    store
                        .put(&key, Credential::new("u", Secret::new("s")).unwrap())
                        .unwrap();
                    assert!(store.get(&key).unwrap().is_some());
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        assert!(store.get(&reference("c3")).unwrap().is_some());
    }
}
