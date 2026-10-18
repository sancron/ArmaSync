// SPDX-License-Identifier: AGPL-3.0-only
//! Credentials: the core only knows an opaque [`CredentialRef`](crate::ids::CredentialRef) and this port.
//!
//! Secrets are never serialised by the core, never part of data files, events or error messages, and are not
//! written to disk in clear text by any adapter unless the user explicitly chooses an adapter that does so
//! (none exists; see ADR-0005).

use std::fmt;

use crate::error::{Error, ErrorKind};
use crate::ids::CredentialRef;

/// Maximum length of a user name in bytes.
pub const MAX_USERNAME_BYTES: usize = 256;
/// Maximum length of a secret in bytes.
pub const MAX_SECRET_BYTES: usize = 4096;

/// A secret value (password or token). `Debug` and `Display` never show the content.
///
/// The content is only reachable through [`Secret::expose`]; callers must not log or serialise it. The memory is
/// not zeroised on drop (no unsafe code, no extra dependency); this is a documented limitation.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    /// Wraps `value`.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The secret text. Use only to hand it to the transport or keyring adapter.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Whether the secret is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

/// A user name (not secret) with its secret.
#[derive(Debug, Clone)]
pub struct Credential {
    username: String,
    secret: Secret,
}

impl Credential {
    /// Creates a credential.
    ///
    /// # Errors
    /// [`ErrorKind::Usage`] for a user name that is too long or contains control characters, or a secret that is
    /// empty or too long. The message never contains the values.
    pub fn new(username: impl Into<String>, secret: Secret) -> Result<Self, Error> {
        let username = username.into();
        if username.len() > MAX_USERNAME_BYTES || username.chars().any(char::is_control) {
            return Err(Error::new(
                ErrorKind::Usage,
                "the user name is too long or contains control characters",
            ));
        }
        if secret.is_empty() || secret.expose().len() > MAX_SECRET_BYTES {
            return Err(Error::new(
                ErrorKind::Usage,
                "the secret is empty or too long",
            ));
        }
        Ok(Self { username, secret })
    }

    /// The user name.
    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    /// The secret.
    #[must_use]
    pub fn secret(&self) -> &Secret {
        &self.secret
    }
}

/// Storage for credentials, addressed by [`CredentialRef`].
///
/// Implementations decide where the secret lives (process memory, OS keyring). All methods take `&self` so that a
/// store can be shared between threads; implementations synchronise internally.
pub trait CredentialStore {
    /// The credential for `reference`, or `None` if there is none.
    ///
    /// # Errors
    /// [`ErrorKind::Config`] if the store is unavailable (for example no keyring on a headless host).
    fn get(&self, reference: &CredentialRef) -> Result<Option<Credential>, Error>;

    /// Stores or replaces the credential for `reference`.
    ///
    /// # Errors
    /// [`ErrorKind::Config`] if the store is unavailable or refuses to persist.
    fn put(&self, reference: &CredentialRef, credential: Credential) -> Result<(), Error>;

    /// Removes the credential for `reference`. Idempotent: removing a missing entry succeeds.
    ///
    /// # Errors
    /// [`ErrorKind::Config`] if the store is unavailable.
    fn delete(&self, reference: &CredentialRef) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    use super::{Credential, Secret};
    use crate::error::ErrorKind;

    #[test]
    fn secret_is_redacted_in_debug_and_display() {
        let secret = Secret::new("hunter2-very-secret");
        assert!(!format!("{secret:?}").contains("hunter2"));
        assert!(!format!("{secret}").contains("hunter2"));
        let credential = Credential::new("alice", secret).unwrap();
        let debug = format!("{credential:?}");
        assert!(debug.contains("alice"));
        assert!(!debug.contains("hunter2"));
        assert_eq!(credential.secret().expose(), "hunter2-very-secret");
    }

    #[test]
    fn invalid_credentials_are_usage_errors_without_values() {
        let long_secret = format!("QQ{}", "p".repeat(5000));
        let long_user = format!("ZZ{}", "u".repeat(300));
        for (user, secret) in [
            ("a\nb", "SECRETVALUE"),
            ("alice", ""),
            ("alice", long_secret.as_str()),
            (long_user.as_str(), "SECRETVALUE"),
        ] {
            let error = Credential::new(user, Secret::new(secret)).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Usage);
            let message = error.message();
            assert!(
                !message.contains("SECRETVALUE")
                    && !message.contains("QQ")
                    && !message.contains("ZZ")
            );
        }
    }
}
