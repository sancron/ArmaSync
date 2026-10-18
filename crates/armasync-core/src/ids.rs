// SPDX-License-Identifier: AGPL-3.0-only
//! Stable identifiers. Display names are never used as keys.

use std::fmt;

/// An invalid identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    /// Empty or longer than 64 characters.
    Length,
    /// Contains a character other than `a-z`, `0-9`, `-` or `_`.
    Character,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Length => "identifier must be 1 to 64 characters",
            Self::Character => "identifier may only contain a-z, 0-9, '-' and '_'",
        })
    }
}

impl std::error::Error for IdError {}

/// Shared identifier rule: `[a-z0-9_-]{1,64}`.
fn validate(input: &str) -> Result<(), IdError> {
    if input.is_empty() || input.len() > 64 {
        return Err(IdError::Length);
    }
    if !input.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
    }) {
        return Err(IdError::Character);
    }
    Ok(())
}

/// Stable identifier of a repository (not its display name).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RepositoryId(String);

impl RepositoryId {
    /// Validates `input` (`[a-z0-9_-]{1,64}`).
    ///
    /// # Errors
    /// [`IdError`] for wrong length or characters.
    pub fn parse(input: &str) -> Result<Self, IdError> {
        validate(input)?;
        Ok(Self(input.to_owned()))
    }

    /// The identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RepositoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Defines an identifier newtype with the shared `[a-z0-9_-]{1,64}` rule.
macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// Validates `input` (`[a-z0-9_-]{1,64}`).
            ///
            /// # Errors
            /// [`IdError`] for wrong length or characters.
            pub fn parse(input: &str) -> Result<Self, IdError> {
                validate(input)?;
                Ok(Self(input.to_owned()))
            }

            /// The identifier text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id! {
    /// Stable identifier of a profile (not its display name).
    ProfileId
}

string_id! {
    /// Opaque reference to credentials held by a [`CredentialStore`](crate::credential::CredentialStore).
    ///
    /// The reference itself is not a secret and may be written to data files; the secret never is.
    CredentialRef
}

/// Identifier of one running or finished operation (unique within a process or store).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationId(u64);

impl OperationId {
    /// Wraps a numeric id.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The numeric value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "op-{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{CredentialRef, IdError, OperationId, ProfileId, RepositoryId};

    #[test]
    fn repository_ids_are_restricted() {
        assert_eq!(
            RepositoryId::parse("main-repo_2").unwrap().as_str(),
            "main-repo_2"
        );
        assert_eq!(RepositoryId::parse("").unwrap_err(), IdError::Length);
        assert_eq!(
            RepositoryId::parse(&"a".repeat(65)).unwrap_err(),
            IdError::Length
        );
        assert_eq!(RepositoryId::parse("Main").unwrap_err(), IdError::Character);
        assert_eq!(RepositoryId::parse("a b").unwrap_err(), IdError::Character);
        assert_eq!(RepositoryId::parse("../x").unwrap_err(), IdError::Character);
    }

    #[test]
    fn profile_and_credential_ids_share_the_rule() {
        assert_eq!(ProfileId::parse("default").unwrap().to_string(), "default");
        assert_eq!(ProfileId::parse("Default").unwrap_err(), IdError::Character);
        assert_eq!(CredentialRef::parse("cred-1").unwrap().as_str(), "cred-1");
        assert_eq!(CredentialRef::parse("").unwrap_err(), IdError::Length);
        assert_eq!(
            CredentialRef::parse("user:pw").unwrap_err(),
            IdError::Character
        );
    }

    #[test]
    fn operation_id_display() {
        assert_eq!(OperationId::new(7).to_string(), "op-7");
        assert_eq!(OperationId::new(7).value(), 7);
    }
}
