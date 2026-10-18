// SPDX-License-Identifier: AGPL-3.0-only
//! ArmaSync core.
//!
//! Holds the product identity today; domain models, ports, errors and events follow in MIG-011.
//! This crate must not depend on GUI, operating-system or network libraries (see ADR-0001).

/// Product identity shared by all frontends.
pub mod product {
    /// Visible product name.
    pub const NAME: &str = "ArmaSync";
    /// Visible publisher / brand.
    pub const PUBLISHER: &str = "Lynxnode.it";
    /// Stable technical identifier (package and settings namespace).
    pub const TECHNICAL_ID: &str = "it.lynxnode.armasync";
    /// Workspace version of the Rust implementation (not the legacy Java release version).
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}

#[cfg(test)]
mod tests {
    use super::product;

    #[test]
    fn identity_is_stable() {
        assert_eq!(product::NAME, "ArmaSync");
        assert_eq!(product::PUBLISHER, "Lynxnode.it");
        assert_eq!(product::TECHNICAL_ID, "it.lynxnode.armasync");
        assert!(!product::VERSION.is_empty());
    }
}
