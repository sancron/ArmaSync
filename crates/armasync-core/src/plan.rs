// SPDX-License-Identifier: AGPL-3.0-only
//! Sync planning: a pure function from (manifest, local state, options) to a [`SyncPlan`].
//!
//! The plan is revalidated against the repository revision and the local state before it is applied
//! (see `armasync-application`). Deleting is never the default: it needs `delete_extra` *and* a managed root.

use std::collections::BTreeMap;

use crate::hash::ContentHash;
use crate::manifest::Manifest;
use crate::path::RelativePath;

/// A file found in the local destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFile {
    /// Path relative to the destination root.
    pub path: RelativePath,
    /// Size in bytes.
    pub size: u64,
    /// Content hash if known; `None` means unknown content and forces a replace.
    pub hash: Option<ContentHash>,
}

/// Options that shape a plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanOptions {
    /// Plan deletion of local files that are not in the manifest (off by default).
    pub delete_extra: bool,
    /// Directories (or files) whose extra content may be deleted. Extras elsewhere are always kept.
    pub managed_roots: Vec<RelativePath>,
}

/// One planned action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStep {
    /// Fetch a file that does not exist locally.
    Download {
        /// Target path.
        path: RelativePath,
        /// Expected size.
        size: u64,
    },
    /// Replace a file whose content differs or is unknown.
    Replace {
        /// Target path.
        path: RelativePath,
        /// Expected size.
        size: u64,
    },
    /// Leave an identical file alone.
    Keep {
        /// The file.
        path: RelativePath,
    },
    /// Delete a local file that the repository does not contain (only with `delete_extra`).
    Delete {
        /// The file to delete.
        path: RelativePath,
    },
}

/// A non-fatal observation about the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanWarning {
    /// An extra local file was kept although deletion was requested, because it is outside the managed roots.
    ExtraKeptOutsideManagedRoots(RelativePath),
    /// A local file exists whose name differs from the manifest only by case; the manifest spelling is fetched.
    CaseMismatch {
        /// Local spelling.
        local: RelativePath,
        /// Manifest spelling.
        expected: RelativePath,
    },
}

/// Totals of a plan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanSummary {
    /// Files to download.
    pub download_files: u64,
    /// Bytes to download.
    pub download_bytes: u64,
    /// Files to replace.
    pub replace_files: u64,
    /// Bytes to fetch for replacements.
    pub replace_bytes: u64,
    /// Identical files left alone.
    pub keep_files: u64,
    /// Files to delete.
    pub delete_files: u64,
    /// Extra local files that are not touched.
    pub extra_untouched: u64,
}

/// The ordered actions to bring a destination to a repository revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPlan {
    /// Revision the plan was computed for.
    pub revision: String,
    /// Downloads/replacements/keeps in path order, then deletions in path order.
    pub steps: Vec<SyncStep>,
    /// Totals.
    pub summary: PlanSummary,
    /// Observations.
    pub warnings: Vec<PlanWarning>,
}

impl SyncPlan {
    /// Whether applying the plan changes nothing.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.steps
            .iter()
            .all(|step| matches!(step, SyncStep::Keep { .. }))
    }
}

/// Computes the plan. Pure and deterministic.
#[must_use]
pub fn plan_sync(manifest: &Manifest, local: &[LocalFile], options: &PlanOptions) -> SyncPlan {
    let by_path: BTreeMap<&RelativePath, &LocalFile> =
        local.iter().map(|file| (&file.path, file)).collect();
    let by_key: BTreeMap<String, &RelativePath> = local
        .iter()
        .map(|file| (file.path.collision_key(), &file.path))
        .collect();
    let mut steps = Vec::new();
    let mut warnings = Vec::new();
    let mut summary = PlanSummary::default();

    for entry in manifest.entries() {
        match by_path.get(&entry.path) {
            Some(file) if file.size == entry.size && file.hash.as_ref() == Some(&entry.hash) => {
                summary.keep_files += 1;
                steps.push(SyncStep::Keep {
                    path: entry.path.clone(),
                });
            }
            Some(_) => {
                summary.replace_files += 1;
                summary.replace_bytes += entry.size;
                steps.push(SyncStep::Replace {
                    path: entry.path.clone(),
                    size: entry.size,
                });
            }
            None => {
                if let Some(other) = by_key.get(&entry.path.collision_key()) {
                    warnings.push(PlanWarning::CaseMismatch {
                        local: (*other).clone(),
                        expected: entry.path.clone(),
                    });
                }
                summary.download_files += 1;
                summary.download_bytes += entry.size;
                steps.push(SyncStep::Download {
                    path: entry.path.clone(),
                    size: entry.size,
                });
            }
        }
    }

    for file in by_path.values() {
        if manifest.get(&file.path).is_some() {
            continue;
        }
        let case_variant = manifest
            .entries()
            .iter()
            .any(|entry| entry.path.collision_key() == file.path.collision_key());
        if case_variant {
            // Reported as CaseMismatch above; deleting it would race with the download on case-insensitive systems.
            summary.extra_untouched += 1;
            continue;
        }
        let managed = options
            .managed_roots
            .iter()
            .any(|root| file.path.is_within(root));
        if options.delete_extra && managed {
            summary.delete_files += 1;
            steps.push(SyncStep::Delete {
                path: file.path.clone(),
            });
        } else {
            summary.extra_untouched += 1;
            if options.delete_extra {
                warnings.push(PlanWarning::ExtraKeptOutsideManagedRoots(file.path.clone()));
            }
        }
    }

    SyncPlan {
        revision: manifest.revision().to_owned(),
        steps,
        summary,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalFile, PlanOptions, PlanWarning, SyncStep, plan_sync};
    use crate::hash::{ContentHash, HashAlgorithm};
    use crate::manifest::{FileEntry, Manifest};
    use crate::path::RelativePath;

    fn hash(digit: char) -> ContentHash {
        ContentHash::new(HashAlgorithm::Sha256, &digit.to_string().repeat(64)).unwrap()
    }

    fn path(text: &str) -> RelativePath {
        RelativePath::parse(text).unwrap()
    }

    fn manifest(files: &[(&str, u64, char)]) -> Manifest {
        let entries = files
            .iter()
            .map(|(p, size, h)| FileEntry {
                path: path(p),
                size: *size,
                hash: hash(*h),
            })
            .collect();
        Manifest::new("rev-1", entries).unwrap()
    }

    fn local(p: &str, size: u64, h: Option<char>) -> LocalFile {
        LocalFile {
            path: path(p),
            size,
            hash: h.map(hash),
        }
    }

    #[test]
    fn identical_changed_missing_and_unknown_content() {
        let manifest = manifest(&[
            ("same.pbo", 10, 'a'),
            ("changed.pbo", 20, 'b'),
            ("missing.pbo", 30, 'c'),
            ("unknown.pbo", 5, 'd'),
        ]);
        let local = [
            local("same.pbo", 10, Some('a')),
            local("changed.pbo", 20, Some('f')),
            local("unknown.pbo", 5, None),
        ];
        let plan = plan_sync(&manifest, &local, &PlanOptions::default());
        assert_eq!(
            plan.steps,
            [
                SyncStep::Replace {
                    path: path("changed.pbo"),
                    size: 20
                },
                SyncStep::Download {
                    path: path("missing.pbo"),
                    size: 30
                },
                SyncStep::Keep {
                    path: path("same.pbo")
                },
                SyncStep::Replace {
                    path: path("unknown.pbo"),
                    size: 5
                },
            ]
        );
        assert_eq!(plan.summary.download_files, 1);
        assert_eq!(plan.summary.download_bytes, 30);
        assert_eq!(plan.summary.replace_files, 2);
        assert_eq!(plan.summary.replace_bytes, 25);
        assert_eq!(plan.summary.keep_files, 1);
        assert_eq!(plan.revision, "rev-1");
        assert!(!plan.is_noop());
    }

    #[test]
    fn same_hash_but_different_size_is_replaced() {
        let plan = plan_sync(
            &manifest(&[("a.pbo", 10, 'a')]),
            &[local("a.pbo", 11, Some('a'))],
            &PlanOptions::default(),
        );
        assert!(matches!(plan.steps[0], SyncStep::Replace { .. }));
    }

    #[test]
    fn extras_are_kept_by_default() {
        let plan = plan_sync(
            &manifest(&[("a.pbo", 1, 'a')]),
            &[local("a.pbo", 1, Some('a')), local("extra.pbo", 9, None)],
            &PlanOptions::default(),
        );
        assert!(plan.is_noop());
        assert_eq!(plan.summary.delete_files, 0);
        assert_eq!(plan.summary.extra_untouched, 1);
        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn deletion_needs_the_flag_and_a_managed_root() {
        let local = [
            local("mods/@x/old.pbo", 1, None),
            local("userdata/save.dat", 1, None),
        ];
        let options = PlanOptions {
            delete_extra: true,
            managed_roots: vec![path("mods")],
        };
        let plan = plan_sync(&manifest(&[("mods/@x/new.pbo", 1, 'a')]), &local, &options);
        assert_eq!(
            plan.steps,
            [
                SyncStep::Download {
                    path: path("mods/@x/new.pbo"),
                    size: 1
                },
                SyncStep::Delete {
                    path: path("mods/@x/old.pbo")
                }
            ]
        );
        assert_eq!(plan.summary.delete_files, 1);
        assert_eq!(
            plan.warnings,
            [PlanWarning::ExtraKeptOutsideManagedRoots(path(
                "userdata/save.dat"
            ))]
        );

        let no_flag = PlanOptions {
            delete_extra: false,
            managed_roots: vec![path("mods")],
        };
        let plan = plan_sync(&manifest(&[("mods/@x/new.pbo", 1, 'a')]), &local, &no_flag);
        assert_eq!(plan.summary.delete_files, 0);
    }

    #[test]
    fn deletions_come_last() {
        let options = PlanOptions {
            delete_extra: true,
            managed_roots: vec![path("a")],
        };
        let plan = plan_sync(
            &manifest(&[("z/new.pbo", 1, 'a')]),
            &[local("a/old.pbo", 1, None)],
            &options,
        );
        assert!(matches!(plan.steps.last(), Some(SyncStep::Delete { .. })));
    }

    #[test]
    fn case_only_difference_is_downloaded_and_warned_not_deleted() {
        let options = PlanOptions {
            delete_extra: true,
            managed_roots: vec![path("mods")],
        };
        let plan = plan_sync(
            &manifest(&[("mods/A.pbo", 1, 'a')]),
            &[local("mods/a.pbo", 1, Some('a'))],
            &options,
        );
        assert_eq!(
            plan.steps,
            [SyncStep::Download {
                path: path("mods/A.pbo"),
                size: 1
            }]
        );
        assert_eq!(
            plan.warnings,
            [PlanWarning::CaseMismatch {
                local: path("mods/a.pbo"),
                expected: path("mods/A.pbo")
            }]
        );
        assert_eq!(plan.summary.delete_files, 0);
    }

    #[test]
    fn planning_is_deterministic_and_idempotent_when_in_sync() {
        let manifest = manifest(&[("b.pbo", 1, 'b'), ("a.pbo", 1, 'a')]);
        let local = [local("b.pbo", 1, Some('b')), local("a.pbo", 1, Some('a'))];
        let first = plan_sync(&manifest, &local, &PlanOptions::default());
        assert_eq!(first, plan_sync(&manifest, &local, &PlanOptions::default()));
        assert!(first.is_noop());
    }
}
