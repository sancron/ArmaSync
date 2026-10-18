// SPDX-License-Identifier: AGPL-3.0-only
//! ArmaSync application layer.
//!
//! Use cases (inspect, plan, apply, verify, ...) are added from MIG-011 on. Only the read-only
//! product description is available now; no synchronisation is implemented.

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
