// SPDX-License-Identifier: AGPL-3.0-only
//! Persistent domain data: repository definitions, profiles, the library that holds them, and user settings.
//!
//! Everything here is validated on construction, so a value that exists is valid. The types carry no storage
//! format: JSON/TOML documents and their `schema_version` live in the adapters (`docs/migration/DATA_MODEL.md`).
//! Names are neutral; nothing here mirrors legacy field or class names (that is the job of the import tasks).

use std::collections::HashSet;
use std::fmt;

use crate::error::{Error, ErrorKind};
use crate::ids::{CredentialRef, ProfileId, RepositoryId};

/// Longest display name in characters.
pub const MAX_NAME_CHARS: usize = 120;
/// Longest repository URL in bytes.
pub const MAX_URL_BYTES: usize = 2048;
/// Most repositories in one library.
pub const MAX_REPOSITORIES: usize = 5_000;
/// Most profiles in one library.
pub const MAX_PROFILES: usize = 1_000;

/// Why a value was rejected. Messages never contain the rejected text (it may be a secret by mistake).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// Empty display name.
    EmptyName,
    /// Display name longer than [`MAX_NAME_CHARS`] characters.
    NameTooLong,
    /// Control character, or leading/trailing white space, in a display name.
    NameCharacters,
    /// The URL is empty.
    UrlEmpty,
    /// The URL is longer than [`MAX_URL_BYTES`].
    UrlTooLong,
    /// White space, control or backslash characters in the URL.
    UrlCharacters,
    /// Scheme other than `http`, `https`, `ftp`, `ftps` or `sftp`, or no scheme.
    UrlScheme,
    /// Missing or malformed host or port.
    UrlHost,
    /// User name or password inside the URL; credentials belong in a credential store.
    UrlEmbeddedCredentials,
    /// Query or fragment in the URL (may carry private tokens).
    UrlQueryOrFragment,
    /// Two repositories share an id.
    DuplicateRepository(RepositoryId),
    /// Two profiles share an id.
    DuplicateProfile(ProfileId),
    /// A profile lists a repository id that is not in the library.
    UnknownRepository {
        /// The profile.
        profile: ProfileId,
        /// The missing repository.
        repository: RepositoryId,
    },
    /// A profile lists the same repository twice.
    DuplicateInProfile {
        /// The profile.
        profile: ProfileId,
        /// The repeated repository.
        repository: RepositoryId,
    },
    /// More repositories or profiles than allowed (the name says which limit).
    TooMany(&'static str),
    /// UI scale outside the supported range.
    UiScale,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => f.write_str("the name is empty"),
            Self::NameTooLong => write!(f, "the name is longer than {MAX_NAME_CHARS} characters"),
            Self::NameCharacters => {
                f.write_str("the name contains control characters or leading/trailing spaces")
            }
            Self::UrlEmpty => f.write_str("the repository address is empty"),
            Self::UrlTooLong => write!(
                f,
                "the repository address is longer than {MAX_URL_BYTES} bytes"
            ),
            Self::UrlCharacters => {
                f.write_str("the repository address contains spaces, control characters or backslashes")
            }
            Self::UrlScheme => f.write_str(
                "the repository address must start with http://, https://, ftp://, ftps:// or sftp://",
            ),
            Self::UrlHost => f.write_str("the repository address has no valid host or port"),
            Self::UrlEmbeddedCredentials => f.write_str(
                "the repository address must not contain a user name or password; enter them separately",
            ),
            Self::UrlQueryOrFragment => {
                f.write_str("the repository address must not contain a query or fragment")
            }
            Self::DuplicateRepository(id) => write!(f, "repository id '{id}' is used twice"),
            Self::DuplicateProfile(id) => write!(f, "profile id '{id}' is used twice"),
            Self::UnknownRepository {
                profile,
                repository,
            } => write!(
                f,
                "profile '{profile}' refers to unknown repository '{repository}'"
            ),
            Self::DuplicateInProfile {
                profile,
                repository,
            } => write!(
                f,
                "profile '{profile}' lists repository '{repository}' twice"
            ),
            Self::TooMany(what) => write!(f, "too many {what}"),
            Self::UiScale => write!(
                f,
                "the UI scale must be between {} and {} percent",
                UiScale::MIN,
                UiScale::MAX
            ),
        }
    }
}

impl std::error::Error for ModelError {}

impl From<ModelError> for Error {
    /// Invalid persisted or entered data is a format problem of that data.
    fn from(error: ModelError) -> Self {
        Self::new(ErrorKind::Format, error.to_string())
    }
}

/// A validated display name (1 to [`MAX_NAME_CHARS`] characters, no control characters, no surrounding spaces).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayName(String);

impl DisplayName {
    /// Validates `input`.
    ///
    /// # Errors
    /// [`ModelError`] for an empty, too long or badly formed name.
    pub fn parse(input: &str) -> Result<Self, ModelError> {
        if input.is_empty() {
            return Err(ModelError::EmptyName);
        }
        if input.chars().count() > MAX_NAME_CHARS {
            return Err(ModelError::NameTooLong);
        }
        if input.chars().any(char::is_control) || input.trim() != input {
            return Err(ModelError::NameCharacters);
        }
        Ok(Self(input.to_owned()))
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DisplayName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A repository address without credentials, query or fragment.
///
/// Accepted schemes are the transports the project plans to support (`http`, `https`, `ftp`, `ftps`, `sftp`); the
/// scheme is stored in lower case. Whether a transport is implemented is decided elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryUrl(String);

impl RepositoryUrl {
    /// Validates `input`.
    ///
    /// # Errors
    /// [`ModelError`] for malformed addresses or addresses containing credentials, a query or a fragment.
    pub fn parse(input: &str) -> Result<Self, ModelError> {
        if input.is_empty() {
            return Err(ModelError::UrlEmpty);
        }
        if input.len() > MAX_URL_BYTES {
            return Err(ModelError::UrlTooLong);
        }
        if input
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
        {
            return Err(ModelError::UrlCharacters);
        }
        let (scheme, rest) = input.split_once("://").ok_or(ModelError::UrlScheme)?;
        let scheme = scheme.to_ascii_lowercase();
        if !matches!(scheme.as_str(), "http" | "https" | "ftp" | "ftps" | "sftp") {
            return Err(ModelError::UrlScheme);
        }
        let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        if authority.contains('@') {
            return Err(ModelError::UrlEmbeddedCredentials);
        }
        if rest.contains(['?', '#']) {
            return Err(ModelError::UrlQueryOrFragment);
        }
        validate_host_and_port(authority)?;
        Ok(Self(format!("{scheme}://{rest}")))
    }

    /// The address.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the transport is unencrypted (`http` or `ftp`); frontends should warn.
    #[must_use]
    pub fn is_unencrypted(&self) -> bool {
        self.0.starts_with("http://") || self.0.starts_with("ftp://")
    }
}

impl fmt::Display for RepositoryUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn validate_host_and_port(authority: &str) -> Result<(), ModelError> {
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let (inner, after) = rest.split_once(']').ok_or(ModelError::UrlHost)?;
        if inner.is_empty()
            || !inner
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.')
        {
            return Err(ModelError::UrlHost);
        }
        (inner, after.strip_prefix(':'))
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    let bracketed = authority.starts_with('[');
    if host.is_empty()
        || (!bracketed
            && !host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_'))
    {
        return Err(ModelError::UrlHost);
    }
    if bracketed
        && authority
            .split_once(']')
            .is_some_and(|(_, after)| !after.is_empty() && port.is_none())
    {
        return Err(ModelError::UrlHost);
    }
    if let Some(port) = port {
        let valid = !port.is_empty()
            && port.len() <= 5
            && port.bytes().all(|b| b.is_ascii_digit())
            && port.parse::<u32>().is_ok_and(|p| (1..=65_535).contains(&p));
        if !valid {
            return Err(ModelError::UrlHost);
        }
    }
    Ok(())
}

/// A repository definition: where it is and which credentials (by reference) belong to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    /// Stable id.
    pub id: RepositoryId,
    /// Display name (not a key).
    pub name: DisplayName,
    /// Address without credentials.
    pub url: RepositoryUrl,
    /// Reference to stored credentials, if the repository needs them.
    pub credential: Option<CredentialRef>,
}

/// A profile: a named, ordered selection of repositories. Further profile content (groups, priorities, launch
/// options) belongs to a later task (MIG-050) and must be added with a schema version bump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// Stable id.
    pub id: ProfileId,
    /// Display name (not a key).
    pub name: DisplayName,
    /// Repositories in the user's order, each at most once.
    pub repositories: Vec<RepositoryId>,
}

/// All repositories and profiles of a user. Ids are unique and every profile refers only to known repositories.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Library {
    repositories: Vec<Repository>,
    profiles: Vec<Profile>,
}

impl Library {
    /// An empty library (first start).
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Validates and builds a library, keeping the given order.
    ///
    /// # Errors
    /// [`ModelError`] for duplicate ids, unknown or repeated repository references, or too many entries.
    pub fn new(repositories: Vec<Repository>, profiles: Vec<Profile>) -> Result<Self, ModelError> {
        if repositories.len() > MAX_REPOSITORIES {
            return Err(ModelError::TooMany("repositories"));
        }
        if profiles.len() > MAX_PROFILES {
            return Err(ModelError::TooMany("profiles"));
        }
        let mut known = HashSet::with_capacity(repositories.len());
        for repository in &repositories {
            if !known.insert(&repository.id) {
                return Err(ModelError::DuplicateRepository(repository.id.clone()));
            }
        }
        let mut profile_ids = HashSet::with_capacity(profiles.len());
        for profile in &profiles {
            if !profile_ids.insert(&profile.id) {
                return Err(ModelError::DuplicateProfile(profile.id.clone()));
            }
            if profile.repositories.len() > MAX_REPOSITORIES {
                return Err(ModelError::TooMany("repositories in a profile"));
            }
            let mut listed = HashSet::with_capacity(profile.repositories.len());
            for repository in &profile.repositories {
                if !known.contains(repository) {
                    return Err(ModelError::UnknownRepository {
                        profile: profile.id.clone(),
                        repository: repository.clone(),
                    });
                }
                if !listed.insert(repository) {
                    return Err(ModelError::DuplicateInProfile {
                        profile: profile.id.clone(),
                        repository: repository.clone(),
                    });
                }
            }
        }
        Ok(Self {
            repositories,
            profiles,
        })
    }

    /// The repositories in order.
    #[must_use]
    pub fn repositories(&self) -> &[Repository] {
        &self.repositories
    }

    /// The profiles in order.
    #[must_use]
    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }

    /// The repository with `id`.
    #[must_use]
    pub fn repository(&self, id: &RepositoryId) -> Option<&Repository> {
        self.repositories.iter().find(|r| &r.id == id)
    }
}

/// Interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// Follow the operating system.
    #[default]
    System,
    /// German.
    German,
    /// English.
    English,
}

impl Language {
    /// Stable code used in the settings file.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::German => "de",
            Self::English => "en",
        }
    }

    /// The language for a settings code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "system" => Some(Self::System),
            "de" => Some(Self::German),
            "en" => Some(Self::English),
            _ => None,
        }
    }
}

/// Colour theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Follow the operating system.
    #[default]
    System,
    /// Light.
    Light,
    /// Dark.
    Dark,
}

impl Theme {
    /// Stable code used in the settings file.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// The theme for a settings code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "system" => Some(Self::System),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }
}

/// UI scale in percent (75 to 300).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiScale(u16);

impl UiScale {
    /// Smallest scale.
    pub const MIN: u16 = 75;
    /// Largest scale.
    pub const MAX: u16 = 300;
    /// 100 percent.
    pub const DEFAULT: Self = Self(100);

    /// Validates `percent`.
    ///
    /// # Errors
    /// [`ModelError::UiScale`] outside 75 to 300.
    pub const fn new(percent: u16) -> Result<Self, ModelError> {
        if percent >= Self::MIN && percent <= Self::MAX {
            Ok(Self(percent))
        } else {
            Err(ModelError::UiScale)
        }
    }

    /// The percentage.
    #[must_use]
    pub const fn percent(self) -> u16 {
        self.0
    }
}

impl Default for UiScale {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// User settings. A setting is only added together with a visible effect and a schema version bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Settings {
    /// Interface language.
    pub language: Language,
    /// Colour theme.
    pub theme: Theme,
    /// UI scale.
    pub ui_scale: UiScale,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(id: &str, url: &str) -> Repository {
        Repository {
            id: RepositoryId::parse(id).unwrap(),
            name: DisplayName::parse(&format!("Repo {id}")).unwrap(),
            url: RepositoryUrl::parse(url).unwrap(),
            credential: None,
        }
    }

    fn profile(id: &str, repos: &[&str]) -> Profile {
        Profile {
            id: ProfileId::parse(id).unwrap(),
            name: DisplayName::parse(id).unwrap(),
            repositories: repos
                .iter()
                .map(|r| RepositoryId::parse(r).unwrap())
                .collect(),
        }
    }

    #[test]
    fn names_are_validated() {
        assert_eq!(
            DisplayName::parse("Main – Größe").unwrap().as_str(),
            "Main – Größe"
        );
        assert_eq!(DisplayName::parse("").unwrap_err(), ModelError::EmptyName);
        assert_eq!(
            DisplayName::parse(&"x".repeat(121)).unwrap_err(),
            ModelError::NameTooLong
        );
        assert_eq!(
            DisplayName::parse(&"ä".repeat(120))
                .unwrap()
                .as_str()
                .chars()
                .count(),
            120
        );
        for bad in [" a", "a ", "a\tb", "a\u{0}b"] {
            assert_eq!(
                DisplayName::parse(bad).unwrap_err(),
                ModelError::NameCharacters
            );
        }
    }

    #[test]
    fn urls_accept_plain_repository_addresses() {
        for ok in [
            "https://example.org/repo",
            "HTTPS://Example.org:8443/a/b/",
            "http://192.0.2.7/x",
            "ftp://ftp.example.org",
            "ftps://ftp.example.org:990/",
            "sftp://[2001:db8::1]:22/path",
            "https://xn--bcher-kva.example/",
        ] {
            assert!(RepositoryUrl::parse(ok).is_ok(), "{ok}");
        }
        assert_eq!(
            RepositoryUrl::parse("HTTPS://Example.org/Path")
                .unwrap()
                .as_str(),
            "https://Example.org/Path"
        );
        assert!(
            RepositoryUrl::parse("http://a.example/")
                .unwrap()
                .is_unencrypted()
        );
        assert!(
            !RepositoryUrl::parse("https://a.example/")
                .unwrap()
                .is_unencrypted()
        );
    }

    #[test]
    fn urls_reject_credentials_queries_and_malformed_input_without_echoing_them() {
        let cases = [
            ("", ModelError::UrlEmpty),
            ("example.org/x", ModelError::UrlScheme),
            ("file:///etc/passwd", ModelError::UrlScheme),
            ("javascript://x", ModelError::UrlScheme),
            (
                "https://user:pw@example.org/",
                ModelError::UrlEmbeddedCredentials,
            ),
            (
                "https://user@example.org/",
                ModelError::UrlEmbeddedCredentials,
            ),
            (
                "https://example.org/a?token=SECRET",
                ModelError::UrlQueryOrFragment,
            ),
            ("https://example.org/a#frag", ModelError::UrlQueryOrFragment),
            ("https:///path", ModelError::UrlHost),
            ("https://exa mple.org/", ModelError::UrlCharacters),
            ("https://example.org\\x", ModelError::UrlCharacters),
            ("https://example.org:0/", ModelError::UrlHost),
            ("https://example.org:70000/", ModelError::UrlHost),
            ("https://example.org:/", ModelError::UrlHost),
            ("https://[::1/", ModelError::UrlHost),
            ("https://[]/", ModelError::UrlHost),
            ("https://[::1]x/", ModelError::UrlHost),
            ("https://exa%6dple.org/", ModelError::UrlHost),
        ];
        for (input, expected) in cases {
            let error = RepositoryUrl::parse(input).unwrap_err();
            assert_eq!(error, expected, "{input}");
            let message = error.to_string();
            assert!(
                !message.contains("SECRET") && !message.contains("pw@"),
                "{message}"
            );
        }
        assert_eq!(
            RepositoryUrl::parse(&format!("https://e.org/{}", "a".repeat(2048))).unwrap_err(),
            ModelError::UrlTooLong
        );
    }

    #[test]
    fn library_keeps_order_and_resolves_references() {
        let library = Library::new(
            vec![
                repo("b", "https://b.example/"),
                repo("a", "https://a.example/"),
            ],
            vec![profile("p", &["a", "b"])],
        )
        .unwrap();
        assert_eq!(library.repositories()[0].id.as_str(), "b");
        assert_eq!(library.profiles()[0].repositories[0].as_str(), "a");
        assert!(
            library
                .repository(&RepositoryId::parse("a").unwrap())
                .is_some()
        );
        assert!(
            library
                .repository(&RepositoryId::parse("zz").unwrap())
                .is_none()
        );
        assert_eq!(Library::empty(), Library::default());
    }

    #[test]
    fn library_rejects_duplicates_unknown_references_and_overflow() {
        let a = || repo("a", "https://a.example/");
        assert_eq!(
            Library::new(vec![a(), a()], vec![]).unwrap_err(),
            ModelError::DuplicateRepository(RepositoryId::parse("a").unwrap())
        );
        assert_eq!(
            Library::new(vec![a()], vec![profile("p", &[]), profile("p", &[])]).unwrap_err(),
            ModelError::DuplicateProfile(ProfileId::parse("p").unwrap())
        );
        assert!(matches!(
            Library::new(vec![a()], vec![profile("p", &["nope"])]).unwrap_err(),
            ModelError::UnknownRepository { .. }
        ));
        assert!(matches!(
            Library::new(vec![a()], vec![profile("p", &["a", "a"])]).unwrap_err(),
            ModelError::DuplicateInProfile { .. }
        ));
        let many = (0..=MAX_REPOSITORIES)
            .map(|i| repo(&format!("r{i}"), "https://a.example/"))
            .collect();
        assert_eq!(
            Library::new(many, vec![]).unwrap_err(),
            ModelError::TooMany("repositories")
        );
    }

    #[test]
    fn settings_codes_and_scale() {
        for language in [Language::System, Language::German, Language::English] {
            assert_eq!(Language::from_code(language.code()), Some(language));
        }
        for theme in [Theme::System, Theme::Light, Theme::Dark] {
            assert_eq!(Theme::from_code(theme.code()), Some(theme));
        }
        assert_eq!(Language::from_code("fr"), None);
        assert_eq!(Theme::from_code("sepia"), None);
        assert_eq!(UiScale::new(74).unwrap_err(), ModelError::UiScale);
        assert_eq!(UiScale::new(301).unwrap_err(), ModelError::UiScale);
        assert_eq!(UiScale::new(200).unwrap().percent(), 200);
        assert_eq!(Settings::default().ui_scale.percent(), 100);
    }

    #[test]
    fn model_errors_become_format_errors() {
        let error: Error = ModelError::UiScale.into();
        assert_eq!(error.kind(), ErrorKind::Format);
    }
}
