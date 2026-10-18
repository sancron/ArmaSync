// SPDX-License-Identifier: AGPL-3.0-only
//! Planning an import: a pure function of the existing library, the import document and the conflict policy.
//!
//! The plan is deterministic, so importing the same document twice yields the same ids and the second run changes
//! nothing. Existing entries are never modified except under [`OnConflict::Replace`], which the preview shows.

use armasync_core::{
    CancelToken, CredentialRef, Error, ErrorKind, Library, Profile, ProfileId, Repository,
    RepositoryId, RepositoryUrl,
};

use super::document::{ImportDocument, ImportNote, ImportSource};

/// What to do when an imported item collides with an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnConflict {
    /// Keep existing entries untouched and add the imported item next to them under a new id (default).
    #[default]
    KeepBoth,
    /// Stop without changing anything.
    Fail,
    /// Overwrite the existing entry that has the same id (shown in the preview).
    Replace,
}

impl OnConflict {
    /// Stable name used on the command line and in JSON.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::KeepBoth => "keep-both",
            Self::Fail => "fail",
            Self::Replace => "replace",
        }
    }

    /// The policy for a command-line value.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "keep-both" => Some(Self::KeepBoth),
            "fail" => Some(Self::Fail),
            "replace" => Some(Self::Replace),
            _ => None,
        }
    }
}

/// What happens to one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// A new entry is added.
    Add,
    /// An identical entry exists; nothing changes.
    Unchanged,
    /// An existing entry is overwritten (only with [`OnConflict::Replace`]).
    Replace,
}

impl Action {
    /// Stable name for reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Unchanged => "unchanged",
            Self::Replace => "replace",
        }
    }
}

/// The planned outcome for one imported repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryItem {
    /// Source key.
    pub key: String,
    /// Display name.
    pub name: String,
    /// The id the repository has (or gets) in the library.
    pub id: RepositoryId,
    /// What happens.
    pub action: Action,
    /// The id was changed because the preferred one is taken by a different repository.
    pub renamed_due_to_collision: bool,
}

/// The planned outcome for one imported profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileItem {
    /// Display name.
    pub name: String,
    /// The id the profile has (or gets).
    pub id: ProfileId,
    /// What happens.
    pub action: Action,
    /// The id was changed because the preferred one is taken by a different profile.
    pub renamed_due_to_collision: bool,
    /// Repository keys of this profile that could not be resolved and are left out.
    pub unresolved: Vec<String>,
}

/// Something the user should know before applying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    /// A profile refers to a repository that neither the import nor the library contains.
    UnresolvedRepository {
        /// Profile name.
        profile: String,
        /// Repository key.
        key: String,
    },
    /// The repository's source used credentials; they were not imported and must be entered again.
    CredentialsNeeded {
        /// Repository id.
        repository: RepositoryId,
    },
    /// The repository address is unencrypted (`http`/`ftp`).
    Unencrypted {
        /// Repository id.
        repository: RepositoryId,
    },
    /// An existing repository with the same address has a different name; it was kept (or renamed with replace).
    NameDiffers {
        /// Repository id.
        repository: RepositoryId,
    },
}

/// Totals of a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// Repositories added.
    pub repositories_added: usize,
    /// Repositories already present and identical.
    pub repositories_unchanged: usize,
    /// Repositories overwritten.
    pub repositories_replaced: usize,
    /// Profiles added.
    pub profiles_added: usize,
    /// Profiles already present and identical.
    pub profiles_unchanged: usize,
    /// Profiles overwritten.
    pub profiles_replaced: usize,
}

/// The result of planning: per-item outcomes, warnings and the library that applying would write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPlan {
    /// Repository outcomes in document order.
    pub repositories: Vec<RepositoryItem>,
    /// Profile outcomes in document order.
    pub profiles: Vec<ProfileItem>,
    /// Warnings.
    pub warnings: Vec<Warning>,
    /// Notes of the exporter, passed through for the report.
    pub notes: Vec<ImportNote>,
    /// Producer of the document.
    pub source: ImportSource,
    library: Library,
}

impl ImportPlan {
    /// The library as it would be after applying.
    #[must_use]
    pub fn library(&self) -> &Library {
        &self.library
    }

    /// Totals.
    #[must_use]
    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for item in &self.repositories {
            match item.action {
                Action::Add => counts.repositories_added += 1,
                Action::Unchanged => counts.repositories_unchanged += 1,
                Action::Replace => counts.repositories_replaced += 1,
            }
        }
        for item in &self.profiles {
            match item.action {
                Action::Add => counts.profiles_added += 1,
                Action::Unchanged => counts.profiles_unchanged += 1,
                Action::Replace => counts.profiles_replaced += 1,
            }
        }
        counts
    }

    /// Whether applying would change the library at all.
    #[must_use]
    pub fn changes_library(&self) -> bool {
        let c = self.counts();
        c.repositories_added + c.repositories_replaced + c.profiles_added + c.profiles_replaced > 0
    }
}

/// Lower-case ASCII slug of `input` for ids: letters and digits kept, everything else becomes a single `-`.
fn slug(input: &str) -> String {
    let mut out = String::new();
    let mut separator = true;
    for c in input.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            separator = false;
        } else if !separator {
            out.push('-');
            separator = true;
        }
    }
    out.truncate(48);
    let trimmed = out.trim_end_matches('-');
    if trimmed.is_empty() {
        "item".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Normalised address for equality: lower-case scheme and host, no trailing slash.
fn url_identity(url: &RepositoryUrl) -> String {
    let text = url.as_str();
    let (scheme, rest) = text.split_once("://").unwrap_or(("", text));
    let (authority, path) = rest.split_once('/').map_or((rest, ""), |(a, p)| (a, p));
    format!(
        "{}://{}/{}",
        scheme.to_ascii_lowercase(),
        authority.to_ascii_lowercase(),
        path.trim_end_matches('/')
    )
}

fn cancelled() -> Error {
    Error::new(
        ErrorKind::Cancelled,
        "the import was cancelled; nothing was changed",
    )
}

fn conflict(what: &str, id: &str) -> Error {
    Error::new(
        ErrorKind::Config,
        format!(
            "{what} '{id}' already exists with different content; choose --on-conflict keep-both or replace, or resolve it first"
        ),
    )
}

/// Plans importing `document` into `existing`.
///
/// # Errors
/// [`ErrorKind::Cancelled`] if `cancel` is triggered; [`ErrorKind::Config`] for a collision under
/// [`OnConflict::Fail`]; [`ErrorKind::Format`] if the result would violate the library limits.
pub fn plan_import(
    existing: &Library,
    document: &ImportDocument,
    policy: OnConflict,
    cancel: &CancelToken,
) -> Result<ImportPlan, Error> {
    let mut repositories: Vec<Repository> = existing.repositories().to_vec();
    let mut profiles: Vec<Profile> = existing.profiles().to_vec();
    let mut key_to_id: Vec<(String, RepositoryId)> = Vec::new();
    let mut repository_items = Vec::new();
    let mut warnings = Vec::new();

    for item in &document.repositories {
        if cancel.is_cancelled() {
            return Err(cancelled());
        }
        let identity = url_identity(&item.url);
        let same_url = repositories
            .iter()
            .position(|r| url_identity(&r.url) == identity);
        let (id, action, renamed) = if let Some(position) = same_url {
            let id = repositories[position].id.clone();
            let mut action = Action::Unchanged;
            if repositories[position].name != item.name {
                if policy == OnConflict::Replace {
                    repositories[position].name = item.name.clone();
                    action = Action::Replace;
                }
                warnings.push(Warning::NameDiffers {
                    repository: id.clone(),
                });
            }
            (id, action, false)
        } else {
            let base = slug(&item.key);
            let taken = repositories.iter().position(|r| r.id.as_str() == base);
            let (id, action, renamed) = match (taken, policy) {
                (None, _) => (base, Action::Add, false),
                (Some(_), OnConflict::Fail) => return Err(conflict("repository", &base)),
                (Some(position), OnConflict::Replace) => {
                    repositories[position].url = item.url.clone();
                    repositories[position].name = item.name.clone();
                    (base, Action::Replace, false)
                }
                (Some(_), OnConflict::KeepBoth) => {
                    let mut n = 2;
                    let free = loop {
                        let candidate = format!("{base}-{n}");
                        if repositories.iter().all(|r| r.id.as_str() != candidate) {
                            break candidate;
                        }
                        n += 1;
                    };
                    (free, Action::Add, true)
                }
            };
            let id = RepositoryId::parse(&id)
                .map_err(|e| Error::new(ErrorKind::Format, e.to_string()))?;
            if action == Action::Add {
                repositories.push(Repository {
                    id: id.clone(),
                    name: item.name.clone(),
                    url: item.url.clone(),
                    credential: None,
                });
            }
            (id, action, renamed)
        };
        if item.needs_credentials && action != Action::Unchanged {
            if let Some(repository) = repositories.iter_mut().find(|r| r.id == id) {
                if repository.credential.is_none() {
                    repository.credential = CredentialRef::parse(id.as_str()).ok();
                }
            }
            warnings.push(Warning::CredentialsNeeded {
                repository: id.clone(),
            });
        }
        if item.url.is_unencrypted() {
            warnings.push(Warning::Unencrypted {
                repository: id.clone(),
            });
        }
        key_to_id.push((item.key.clone(), id.clone()));
        repository_items.push(RepositoryItem {
            key: item.key.clone(),
            name: item.name.to_string(),
            id,
            action,
            renamed_due_to_collision: renamed,
        });
    }

    let mut profile_items = Vec::new();
    for item in &document.profiles {
        if cancel.is_cancelled() {
            return Err(cancelled());
        }
        let mut resolved: Vec<RepositoryId> = Vec::new();
        let mut unresolved = Vec::new();
        for key in &item.repositories {
            let found = key_to_id
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, id)| id.clone())
                .or_else(|| {
                    let by_id = slug(key);
                    repositories
                        .iter()
                        .find(|r| r.id.as_str() == by_id)
                        .or_else(|| {
                            let mut named = repositories.iter().filter(|r| r.name.as_str() == key);
                            match (named.next(), named.next()) {
                                (Some(only), None) => Some(only),
                                _ => None,
                            }
                        })
                        .map(|r| r.id.clone())
                });
            match found {
                Some(id) if !resolved.contains(&id) => resolved.push(id),
                Some(_) => {}
                None => {
                    warnings.push(Warning::UnresolvedRepository {
                        profile: item.name.to_string(),
                        key: key.clone(),
                    });
                    unresolved.push(key.clone());
                }
            }
        }
        let identical = profiles
            .iter()
            .find(|p| p.name == item.name && p.repositories == resolved)
            .map(|p| p.id.clone());
        let base = slug(item.name.as_str());
        let taken = profiles.iter().position(|p| p.id.as_str() == base);
        let (id, action, renamed) = if let Some(id) = identical {
            (id.to_string(), Action::Unchanged, false)
        } else {
            match (taken, policy) {
                (None, _) => (base, Action::Add, false),
                (Some(_), OnConflict::Fail) => return Err(conflict("profile", &base)),
                (Some(position), OnConflict::Replace) => {
                    profiles[position].name = item.name.clone();
                    profiles[position].repositories = resolved.clone();
                    let id = profiles[position].id.clone();
                    (id.to_string(), Action::Replace, false)
                }
                (Some(_), OnConflict::KeepBoth) => {
                    let mut n = 2;
                    let free = loop {
                        let candidate = format!("{base}-{n}");
                        if profiles.iter().all(|p| p.id.as_str() != candidate) {
                            break candidate;
                        }
                        n += 1;
                    };
                    (free, Action::Add, true)
                }
            }
        };
        let id = ProfileId::parse(&id).map_err(|e| Error::new(ErrorKind::Format, e.to_string()))?;
        if action == Action::Add {
            profiles.push(Profile {
                id: id.clone(),
                name: item.name.clone(),
                repositories: resolved,
            });
        }
        profile_items.push(ProfileItem {
            name: item.name.to_string(),
            id,
            action,
            renamed_due_to_collision: renamed,
            unresolved,
        });
    }

    let library = Library::new(repositories, profiles)?;
    Ok(ImportPlan {
        repositories: repository_items,
        profiles: profile_items,
        warnings,
        notes: document.notes.clone(),
        source: document.source.clone(),
        library,
    })
}

#[cfg(test)]
mod tests {
    use super::{Action, OnConflict, Warning, plan_import, slug};
    use crate::import::document::parse_import;
    use armasync_core::{CancelToken, ErrorKind, Library};
    use std::path::Path;

    fn doc(json: &str) -> crate::import::document::ImportDocument {
        parse_import(Path::new("t.json"), json.as_bytes()).unwrap()
    }

    const BASE: &str = r#"{"kind":"armasync.import","schema_version":1,
        "source":{"tool":"t","tool_version":"1","format":"f"},
        "repositories":[
          {"key":"Main Repo","name":"Main Repo","url":"https://repo.example/a","needs_credentials":true},
          {"key":"Second","name":"Second","url":"http://second.example/x"}],
        "profiles":[{"name":"Alpha","repositories":["Main Repo","Second","Gone"]}]}"#;

    fn plan(existing: &Library, json: &str, policy: OnConflict) -> super::ImportPlan {
        plan_import(existing, &doc(json), policy, &CancelToken::new()).unwrap()
    }

    #[test]
    fn slugs_are_ascii_ids() {
        assert_eq!(slug("Main Repo"), "main-repo");
        assert_eq!(slug("  --A__b!!"), "a-b");
        assert_eq!(slug("Größe"), "gr-e");
        assert_eq!(slug("???"), "item");
        assert!(slug(&"x".repeat(100)).len() <= 48);
    }

    #[test]
    fn plan_adds_and_reports_warnings() {
        let plan = plan(&Library::empty(), BASE, OnConflict::KeepBoth);
        let counts = plan.counts();
        assert_eq!((counts.repositories_added, counts.profiles_added), (2, 1));
        assert!(plan.changes_library());
        assert_eq!(plan.library().repositories().len(), 2);
        let first = &plan.library().repositories()[0];
        assert_eq!(first.id.as_str(), "main-repo");
        assert_eq!(
            first
                .credential
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("main-repo")
        );
        assert!(plan.library().repositories()[1].credential.is_none());
        let profile = &plan.library().profiles()[0];
        assert_eq!(
            profile.repositories.len(),
            2,
            "unresolved reference is left out"
        );
        assert!(
            plan.warnings
                .iter()
                .any(|w| matches!(w, Warning::UnresolvedRepository { key, .. } if key == "Gone"))
        );
        assert!(
            plan.warnings
                .iter()
                .any(|w| matches!(w, Warning::CredentialsNeeded { .. }))
        );
        assert!(
            plan.warnings
                .iter()
                .any(|w| matches!(w, Warning::Unencrypted { .. }))
        );
        assert_eq!(plan.profiles[0].unresolved, ["Gone"]);
    }

    #[test]
    fn importing_the_same_document_twice_changes_nothing() {
        for policy in [OnConflict::KeepBoth, OnConflict::Fail, OnConflict::Replace] {
            let first = plan(&Library::empty(), BASE, policy);
            let second = plan(first.library(), BASE, policy);
            assert!(!second.changes_library(), "{policy:?}");
            assert_eq!(second.library(), first.library());
            assert!(
                second
                    .repositories
                    .iter()
                    .all(|i| i.action == Action::Unchanged)
            );
            assert!(
                second
                    .profiles
                    .iter()
                    .all(|i| i.action == Action::Unchanged)
            );
        }
    }

    #[test]
    fn id_collision_with_a_different_address_follows_the_policy() {
        let existing = plan(
            &Library::empty(),
            r#"{"kind":"armasync.import","schema_version":1,"source":{"tool":"t","tool_version":"1","format":"f"},
               "repositories":[{"key":"Second","name":"Mine","url":"https://mine.example/"}],
               "profiles":[{"name":"Alpha","repositories":["Second"]}]}"#,
            OnConflict::KeepBoth,
        );
        let library = existing.library();
        // keep-both: both survive, imported one gets a suffix; a second run is stable.
        let kept = plan(library, BASE, OnConflict::KeepBoth);
        assert!(
            kept.repositories
                .iter()
                .any(|i| i.id.as_str() == "second-2" && i.renamed_due_to_collision)
        );
        assert!(
            kept.profiles[0].renamed_due_to_collision && kept.profiles[0].id.as_str() == "alpha-2"
        );
        assert_eq!(kept.library().repositories().len(), 3);
        let again = plan(kept.library(), BASE, OnConflict::KeepBoth);
        assert!(!again.changes_library());
        assert_eq!(again.library(), kept.library());
        // the existing entries are untouched
        assert_eq!(kept.library().repositories()[0], library.repositories()[0]);
        assert_eq!(kept.library().profiles()[0], library.profiles()[0]);
        // fail: error, nothing planned
        let error =
            plan_import(library, &doc(BASE), OnConflict::Fail, &CancelToken::new()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Config);
        // replace: the existing entries are overwritten and shown as such
        let replaced = plan(library, BASE, OnConflict::Replace);
        assert_eq!(replaced.counts().repositories_replaced, 1);
        assert_eq!(replaced.counts().profiles_replaced, 1);
        assert_eq!(replaced.library().repositories().len(), 2);
        assert_eq!(
            replaced.library().repositories()[0].url.as_str(),
            "http://second.example/x"
        );
    }

    #[test]
    fn missing_repositories_can_be_added_later_and_resolve_the_profile() {
        let profile_only = r#"{"kind":"armasync.import","schema_version":1,"source":{"tool":"t","tool_version":"1","format":"f"},
            "profiles":[{"name":"Alpha","repositories":["Main Repo"]}]}"#;
        let first = plan(&Library::empty(), profile_only, OnConflict::KeepBoth);
        assert!(first.library().profiles()[0].repositories.is_empty());
        let repo_only = r#"{"kind":"armasync.import","schema_version":1,"source":{"tool":"t","tool_version":"1","format":"f"},
            "repositories":[{"key":"Main Repo","name":"Main Repo","url":"https://repo.example/a"}]}"#;
        let with_repo = plan(first.library(), repo_only, OnConflict::KeepBoth);
        // Re-importing the profile now resolves the reference through the library (replace updates it).
        let updated = plan(with_repo.library(), profile_only, OnConflict::Replace);
        assert_eq!(updated.counts().profiles_replaced, 1);
        assert_eq!(
            updated.library().profiles()[0].repositories[0].as_str(),
            "main-repo"
        );
    }

    #[test]
    fn url_identity_ignores_case_and_trailing_slash() {
        let existing = plan(
            &Library::empty(),
            r#"{"kind":"armasync.import","schema_version":1,"source":{"tool":"t","tool_version":"1","format":"f"},
               "repositories":[{"key":"A","name":"A","url":"https://Repo.Example/a/"}]}"#,
            OnConflict::KeepBoth,
        );
        let again = plan(
            existing.library(),
            r#"{"kind":"armasync.import","schema_version":1,"source":{"tool":"t","tool_version":"1","format":"f"},
               "repositories":[{"key":"Other key","name":"A","url":"https://repo.example/a"}]}"#,
            OnConflict::KeepBoth,
        );
        assert!(!again.changes_library());
        assert_eq!(again.repositories[0].id.as_str(), "a");
    }

    #[test]
    fn cancellation_stops_planning() {
        let cancel = CancelToken::new();
        cancel.cancel();
        let error =
            plan_import(&Library::empty(), &doc(BASE), OnConflict::KeepBoth, &cancel).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Cancelled);
    }

    #[test]
    fn policy_names_round_trip() {
        for policy in [OnConflict::KeepBoth, OnConflict::Fail, OnConflict::Replace] {
            assert_eq!(OnConflict::from_name(policy.name()), Some(policy));
        }
        assert_eq!(OnConflict::from_name("skip"), None);
        assert_eq!(OnConflict::default(), OnConflict::KeepBoth);
    }
}
