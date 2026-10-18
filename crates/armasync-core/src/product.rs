// SPDX-License-Identifier: AGPL-3.0-only
//! Product identity shared by all frontends.

/// Visible product name.
pub const NAME: &str = "ArmaSync";
/// Visible publisher / brand.
pub const PUBLISHER: &str = "Lynxnode.it";
/// Stable technical identifier (package and settings namespace).
pub const TECHNICAL_ID: &str = "it.lynxnode.armasync";
/// Workspace version of the Rust implementation (not the legacy Java release version).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::{NAME, PUBLISHER, TECHNICAL_ID, VERSION};

    #[test]
    fn identity_is_stable() {
        assert_eq!(NAME, "ArmaSync");
        assert_eq!(PUBLISHER, "Lynxnode.it");
        assert_eq!(TECHNICAL_ID, "it.lynxnode.armasync");
        assert!(!VERSION.is_empty());
    }
}
