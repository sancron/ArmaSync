// SPDX-License-Identifier: AGPL-3.0-only
//! Splitting the source into files that are uploaded and files that are excluded, with the reason (MIG-042,
//! step 3b; ADR-0013). The decision itself lives in `armasync_core::exclusion`; this module applies it to a source.

use armasync_core::exclusion::{self, ExclusionReason, ExclusionRules};
use armasync_core::{CancelToken, Error, LocalSource, RelativePath, SourceFile};

/// A file of the source that is not uploaded, with its size and the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exclusion {
    /// Path below the source root.
    pub path: RelativePath,
    /// Size in bytes at listing time.
    pub size: u64,
    /// Why the file is excluded.
    pub reason: ExclusionReason,
}

/// The result of splitting a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    /// Files to upload, sorted by path, with their hashes.
    pub included: Vec<SourceFile>,
    /// Every excluded file of the source (internal, partial and user-excluded), sorted by path.
    pub excluded: Vec<Exclusion>,
    /// Rules that match no file of the source. They are reported, never silently ignored.
    pub unused_rules: Vec<RelativePath>,
}

/// Splits the source. Every regular file of the source appears either in `included` or in `excluded`.
///
/// # Errors
/// [`armasync_core::ErrorKind::Cancelled`] if `cancel` was requested; source errors otherwise.
pub fn partition(
    source: &dyn LocalSource,
    rules: &ExclusionRules,
    cancel: &CancelToken,
) -> Result<Partition, Error> {
    let inventory = source.inventory(cancel)?;
    let files = source.files(cancel)?;

    let mut excluded: Vec<Exclusion> = inventory
        .iter()
        .filter_map(|entry| {
            exclusion::decide(&entry.path, rules).map(|reason| Exclusion {
                path: entry.path.clone(),
                size: entry.size,
                reason,
            })
        })
        .collect();
    excluded.sort_by(|a, b| a.path.cmp(&b.path));

    let included = files
        .into_iter()
        .filter(|file| exclusion::decide(&file.path, rules).is_none())
        .collect();
    let unused_rules = exclusion::unused_rules(inventory.iter().map(|entry| &entry.path), rules);
    Ok(Partition {
        included,
        excluded,
        unused_rules,
    })
}
