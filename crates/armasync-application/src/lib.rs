// SPDX-License-Identifier: AGPL-3.0-only
//! ArmaSync application layer.
//!
//! Use cases live in submodules: [`sync`] plans and applies a synchronisation against core ports (MIG-011).
//! More use cases (verify, build, upload, import) follow. The read-only product description is [`version_info`].
//! No real transport or file-system adapter exists yet, so nothing here syncs real data.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod exclusion;
pub mod revision;
pub mod sync;
pub mod upload;

use armasync_core::product;

/// Description of the running build, independent of any frontend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionInfo {
    /// Visible product name.
    pub name: &'static str,
    /// Visible publisher.
    pub publisher: &'static str,
    /// Stable technical identifier.
    pub technical_id: &'static str,
    /// Version of this build.
    pub version: &'static str,
}

/// Returns the description of this build.
#[must_use]
pub fn version_info() -> VersionInfo {
    VersionInfo {
        name: product::NAME,
        publisher: product::PUBLISHER,
        technical_id: product::TECHNICAL_ID,
        version: product::VERSION,
    }
}

#[cfg(test)]
mod tests {
    use super::version_info;

    #[test]
    fn version_info_uses_core_identity() {
        let info = version_info();
        assert_eq!(info.name, "ArmaSync");
        assert_eq!(info.publisher, "Lynxnode.it");
    }
}
