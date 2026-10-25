// SPDX-License-Identifier: AGPL-3.0-only
//! Upload exclusions (MIG-042, step 3b; ADR-0013). Pure rules, no I/O.
//!
//! Three kinds of files are never uploaded:
//! * **partial files** (`.armasync-partial-…`) of an interrupted download,
//! * **internal ArmaSync data**: every path component that starts with [`INTERNAL_PREFIX`] (staging files, the lock
//!   file and everything else the adapters create),
//! * **explicitly excluded paths** given as validated [`RelativePath`]s: one file, or a folder with all its content.
//!
//! The user-facing syntax of exclusion rules (glob patterns, a rules file, an editor) is a product decision that is not
//! taken here; see ADR-0013. Only validated paths are accepted, so a malformed rule cannot be expressed.

use crate::path::RelativePath;

/// Prefix of every file and folder ArmaSync creates for itself (staging, lock file, partial files).
pub const INTERNAL_PREFIX: &str = ".armasync-";
/// Prefix of the resumable partial files of an interrupted download.
pub const PARTIAL_PREFIX: &str = ".armasync-partial-";

/// Why a file is excluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExclusionReason {
    /// A resumable partial file of an interrupted download.
    PartialFile,
    /// Internal ArmaSync data (a path component starts with [`INTERNAL_PREFIX`]).
    InternalData,
    /// Matched by an explicitly excluded path (the file itself or a folder above it).
    UserRule(RelativePath),
}

impl ExclusionReason {
    /// Stable machine-readable code for previews and JSON output.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::PartialFile => "partial-file",
            Self::InternalData => "internal-data",
            Self::UserRule(_) => "user-rule",
        }
    }
}

/// The explicitly excluded paths. Sorted and without duplicates.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExclusionRules {
    rules: Vec<RelativePath>,
}

impl ExclusionRules {
    /// Rules from already validated paths; duplicates are dropped.
    pub fn new(rules: impl IntoIterator<Item = RelativePath>) -> Self {
        let mut rules: Vec<RelativePath> = rules.into_iter().collect();
        rules.sort();
        rules.dedup();
        Self { rules }
    }

    /// The rules in path order.
    #[must_use]
    pub fn rules(&self) -> &[RelativePath] {
        &self.rules
    }
}

/// Whether any component of `path` is internal ArmaSync data.
#[must_use]
pub fn is_internal(path: &RelativePath) -> bool {
    path.as_str()
        .split('/')
        .any(|component| component.starts_with(INTERNAL_PREFIX))
}

/// Decides whether `path` is excluded and why. Partial files are reported as such before the other reasons; a user rule
/// matches the path itself or any path below it.
#[must_use]
pub fn decide(path: &RelativePath, rules: &ExclusionRules) -> Option<ExclusionReason> {
    let file_name = path.as_str().rsplit('/').next().unwrap_or_default();
    if file_name.starts_with(PARTIAL_PREFIX) {
        return Some(ExclusionReason::PartialFile);
    }
    if is_internal(path) {
        return Some(ExclusionReason::InternalData);
    }
    rules
        .rules()
        .iter()
        .find(|rule| path == *rule || path.is_within(rule))
        .map(|rule| ExclusionReason::UserRule(rule.clone()))
}

/// The rules that match none of `paths`. Such a rule is reported, never silently ignored.
#[must_use]
pub fn unused_rules<'a>(
    paths: impl IntoIterator<Item = &'a RelativePath> + Clone,
    rules: &ExclusionRules,
) -> Vec<RelativePath> {
    rules
        .rules()
        .iter()
        .filter(|rule| {
            !paths
                .clone()
                .into_iter()
                .any(|path| path == *rule || path.is_within(rule))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(text: &str) -> RelativePath {
        RelativePath::parse(text).unwrap()
    }

    #[test]
    fn partial_files_are_recognised_before_internal_data() {
        let rules = ExclusionRules::default();
        assert_eq!(
            decide(&path("mods/.armasync-partial-42.data"), &rules),
            Some(ExclusionReason::PartialFile)
        );
        assert_eq!(
            decide(&path(".armasync-lock"), &rules),
            Some(ExclusionReason::InternalData)
        );
        assert_eq!(decide(&path("mods/plain.pbo"), &rules), None);
    }

    #[test]
    fn an_internal_component_anywhere_in_the_path_excludes_the_file() {
        let rules = ExclusionRules::default();
        assert_eq!(
            decide(&path("a/.armasync-cache/x.txt"), &rules),
            Some(ExclusionReason::InternalData)
        );
        assert!(is_internal(&path(".armasync-x/y")));
        assert!(!is_internal(&path("armasync/y")));
    }

    #[test]
    fn a_user_rule_matches_the_file_or_a_folder_above_it() {
        let rules = ExclusionRules::new([path("docs/secret.txt"), path("tmp")]);
        assert_eq!(
            decide(&path("docs/secret.txt"), &rules),
            Some(ExclusionReason::UserRule(path("docs/secret.txt")))
        );
        assert_eq!(
            decide(&path("tmp/a/b.txt"), &rules),
            Some(ExclusionReason::UserRule(path("tmp")))
        );
        assert_eq!(decide(&path("docs/other.txt"), &rules), None);
        // A prefix of a name is not a folder: `tmpx` is not inside `tmp`.
        assert_eq!(decide(&path("tmpx/a.txt"), &rules), None);
    }

    #[test]
    fn rules_are_sorted_and_deduplicated() {
        let rules = ExclusionRules::new([path("b"), path("a"), path("b")]);
        assert_eq!(rules.rules(), [path("a"), path("b")]);
    }

    #[test]
    fn unused_rules_are_listed() {
        let rules = ExclusionRules::new([path("docs"), path("missing.txt")]);
        let files = [path("docs/a.txt"), path("other.txt")];
        assert_eq!(unused_rules(files.iter(), &rules), [path("missing.txt")]);
    }
}
