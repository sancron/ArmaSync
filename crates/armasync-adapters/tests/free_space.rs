// SPDX-License-Identifier: AGPL-3.0-only
//! Free space of the local destination (MIG-032B): a real value on Windows for an existing folder. The "unknown" case
//! (a query that fails) maps to `None` in the adapter and is tested through the sync use case, because no reliable way
//! to make the query fail on this platform was found. The value is advisory.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use armasync_adapters::localfs::LocalDirectory;
use armasync_core::LocalStore;

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("armasync-free-space-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn an_existing_destination_reports_its_free_space() {
    let dir = scratch("exists");
    let store = LocalDirectory::new(&dir).unwrap();

    let free = store.free_space().expect("the store answers");

    let bytes = free.expect("this platform reports free space for an existing folder");
    assert!(bytes > 0, "a working disk has free space");
    let _ = fs::remove_dir_all(&dir);
}
