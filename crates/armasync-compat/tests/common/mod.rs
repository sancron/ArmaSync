// SPDX-License-Identifier: AGPL-3.0-only
//! Test helper for the compatibility tests: the synthetic stream writers live in `armasync-testkit`; this module adds
//! the paths of the MIG-001 fixtures.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

pub use armasync_testkit::synthetic::*;

/// Path of a MIG-001 fixture file below `fixtures/legacy-formats/generated`.
pub fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/legacy-formats/generated")
        .join(relative)
}

pub fn read_fixture(relative: &str) -> Vec<u8> {
    std::fs::read(fixture(relative)).unwrap_or_else(|e| panic!("fixture {relative}: {e}"))
}
