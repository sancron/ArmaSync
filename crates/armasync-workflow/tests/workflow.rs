// SPDX-License-Identifier: AGPL-3.0-only
//! Tests of the workflows shared by CLI and GUI against the loopback repository of `armasync-testkit`
//! (real HTTP adapter, real file system, temporary folders): plan without changes, apply, protected paths, the
//! deletion rule, the exclusive lock, cancellation and the platform gate. No public server, no user data.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use armasync_adapters::MemoryEventSink;
use armasync_adapters::http::HttpConfig;
use armasync_core::{CancelToken, ErrorKind, JobState, OperationId, RelativePath, SyncStep};
use armasync_testkit::{BIKEY, Behavior, PBO, ZIP, bikey, pbo, serve};
use armasync_workflow::{
    PlatformStatus, SyncRequest, execute, parse_managed_roots, platform_status_for, prepare,
    require_run_allowed,
};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "armasync-workflow-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let mut candidate = dir.clone();
    let mut attempt = 0u32;
    while fs::create_dir(&candidate).is_err() {
        attempt += 1;
        assert!(attempt < 1000);
        candidate = PathBuf::from(format!("{}-{attempt}", dir.display()));
    }
    candidate
}

fn request(server: &armasync_testkit::Server, dest: &Path) -> SyncRequest {
    SyncRequest {
        repository_url: format!("http://{}/repo", server.addr),
        destination: dest.to_path_buf(),
        delete_extra: false,
        managed_roots: Vec::new(),
        http: HttpConfig::default(),
    }
}

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

fn names(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if name == ".armasync-lock" {
                continue;
            }
            if entry.path().is_dir() {
                walk(&entry.path(), &format!("{name}/"), out);
            } else {
                out.push(name);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}

#[test]
fn prepare_plans_without_changing_anything_and_execute_installs_verified_files() {
    let server = serve();
    let dest = scratch("basic");
    let prepared = prepare(&request(&server, &dest), &CancelToken::new()).unwrap();
    assert_eq!(prepared.plan().summary.download_files, 2);
    assert_eq!(prepared.plan().revision, "3");
    assert!(
        fs::read_dir(&dest).unwrap().next().is_none(),
        "preparing changes nothing, not even a lock file"
    );
    assert_eq!(server.requested("example_core.pbo"), 0);
    assert_eq!(prepared.metadata().unsupported, [path(ZIP)]);
    assert_eq!(
        prepared.options().protected_paths.len(),
        2,
        "hidden folder and the compressed file"
    );

    let mut sink = MemoryEventSink::default();
    let report = execute(
        &prepared,
        OperationId::new(1),
        &mut sink,
        &CancelToken::new(),
    )
    .unwrap();
    assert_eq!((report.downloaded, report.bytes), (2, 8193 + 20));
    assert_eq!(fs::read(dest.join(PBO)).unwrap(), pbo());
    assert_eq!(fs::read(dest.join(BIKEY)).unwrap(), bikey());
    let last = sink.events.last().unwrap();
    assert_eq!(
        (last.state, last.progress.files_done),
        (JobState::Succeeded, 2)
    );
    // preparing again now finds everything up to date
    let again = prepare(&request(&server, &dest), &CancelToken::new()).unwrap();
    assert_eq!(
        (
            again.plan().summary.keep_files,
            again.plan().summary.download_files
        ),
        (2, 0)
    );
    fs::remove_dir_all(dest).unwrap();
}

#[test]
fn deletion_needs_folders_and_protected_paths_always_survive() {
    let server = serve();
    let dest = scratch("delete");
    fs::create_dir_all(dest.join("@example_core/addons")).unwrap();
    fs::write(dest.join("@example_core/addons/extra.pbo"), b"extra").unwrap();
    fs::write(dest.join(ZIP), b"local copy of a compressed file").unwrap();
    fs::create_dir_all(dest.join("@second")).unwrap();
    fs::write(dest.join("@second/mine.txt"), b"mine").unwrap();

    // the rule is enforced by the workflow itself, whatever the frontend checked before
    let mut bad = request(&server, &dest);
    bad.delete_extra = true;
    assert_eq!(
        prepare(&bad, &CancelToken::new()).unwrap_err().kind(),
        ErrorKind::Usage
    );
    assert_eq!(server.total(), 0, "refused before any network access");
    assert!(parse_managed_roots(&[], true).is_err());
    assert!(parse_managed_roots(&["x".to_owned()], false).is_err());
    assert!(parse_managed_roots(&["../out".to_owned()], true).is_err());
    assert!(parse_managed_roots(&["/abs".to_owned()], true).is_err());

    let mut deleting = request(&server, &dest);
    deleting.delete_extra = true;
    deleting.managed_roots =
        parse_managed_roots(&["@example_core".to_owned(), "@second".to_owned()], true).unwrap();
    let prepared = prepare(&deleting, &CancelToken::new()).unwrap();
    let deletes: Vec<_> = prepared
        .plan()
        .steps
        .iter()
        .filter_map(|s| match s {
            SyncStep::Delete { path } => Some(path.as_str().to_owned()),
            _ => None,
        })
        .collect();
    assert_eq!(deletes, ["@example_core/addons/extra.pbo"]);
    execute(
        &prepared,
        OperationId::new(1),
        &mut MemoryEventSink::default(),
        &CancelToken::new(),
    )
    .unwrap();
    assert!(!dest.join("@example_core/addons/extra.pbo").exists());
    assert!(
        dest.join(ZIP).exists(),
        "a file the server holds compressed is never deleted"
    );
    assert!(
        dest.join("@second/mine.txt").exists(),
        "a protected folder is never cleaned"
    );
    fs::remove_dir_all(dest).unwrap();
}

#[test]
fn a_second_execution_on_the_same_destination_is_refused_while_the_first_holds_it() {
    let server = serve();
    let dest = scratch("lock");
    let prepared = prepare(&request(&server, &dest), &CancelToken::new()).unwrap();
    let _held = armasync_adapters::localfs::LocalDirectory::open_exclusive(&dest).unwrap();
    let error = execute(
        &prepared,
        OperationId::new(1),
        &mut MemoryEventSink::default(),
        &CancelToken::new(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Config);
    assert!(names(&dest).is_empty());
    fs::remove_dir_all(dest).unwrap();
}

#[test]
fn failures_leave_no_partial_files_and_keep_existing_ones() {
    for (name, behavior) in [
        (
            "corrupt",
            Behavior {
                corrupt_pbo: true,
                ..Behavior::default()
            },
        ),
        (
            "status",
            Behavior {
                pbo_status: Some(500),
                ..Behavior::default()
            },
        ),
        (
            "short",
            Behavior {
                short_pbo: true,
                ..Behavior::default()
            },
        ),
    ] {
        let server = serve();
        *server.behavior.lock().unwrap() = behavior;
        let dest = scratch(name);
        fs::create_dir_all(dest.join("@example_core/addons")).unwrap();
        fs::write(dest.join(PBO), b"previous").unwrap();
        let prepared = prepare(&request(&server, &dest), &CancelToken::new()).unwrap();
        let error = execute(
            &prepared,
            OperationId::new(1),
            &mut MemoryEventSink::default(),
            &CancelToken::new(),
        )
        .unwrap_err();
        let expected = if name == "corrupt" {
            ErrorKind::Integrity
        } else {
            ErrorKind::Network
        };
        assert_eq!(error.kind(), expected, "{name}");
        assert_eq!(fs::read(dest.join(PBO)).unwrap(), b"previous", "{name}");
        assert_eq!(names(&dest), [PBO], "{name}: nothing partial, nothing else");
        fs::remove_dir_all(dest).unwrap();
    }
}

#[test]
fn cancellation_before_and_after_preparing_changes_nothing() {
    let server = serve();
    let dest = scratch("cancel");
    let cancel = CancelToken::new();
    cancel.cancel();
    assert_eq!(
        prepare(&request(&server, &dest), &cancel)
            .map(|_| ())
            .unwrap_err()
            .kind(),
        ErrorKind::Cancelled
    );
    let prepared = prepare(&request(&server, &dest), &CancelToken::new()).unwrap();
    let error = execute(
        &prepared,
        OperationId::new(1),
        &mut MemoryEventSink::default(),
        &cancel,
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert!(names(&dest).is_empty());
    fs::remove_dir_all(dest).unwrap();
}

#[test]
fn the_platform_gate_names_what_is_verified_and_blocks_what_is_not() {
    let windows = platform_status_for("windows");
    assert!(windows.run_allowed());
    assert!(windows.message().contains("lokalen Testserver"));
    assert!(
        windows.message().contains("noch nicht geprüft"),
        "it says what was NOT verified"
    );
    for os in ["linux", "macos", "freebsd"] {
        let status = platform_status_for(os);
        assert!(!status.run_allowed(), "{os}");
        assert_eq!(
            require_run_allowed(status, false).unwrap_err().kind(),
            ErrorKind::Unsupported,
            "{os}"
        );
        require_run_allowed(status, true).unwrap();
        assert!(matches!(status, PlatformStatus::NotReleased { .. }));
    }
    assert!(
        platform_status_for("linux")
            .message()
            .contains("Pfadgrenze")
    );
    require_run_allowed(windows, false).unwrap();
}

// ---- error cases of P3 / MIG-032 (part 1): the destination and the repository change under the run ----------------

#[test]
fn a_file_that_changes_on_the_server_after_planning_is_refused_and_the_old_file_stays() {
    let server = serve();
    let dest = scratch("changed-remote");
    fs::create_dir_all(dest.join("@example_core/addons")).unwrap();
    fs::write(dest.join(PBO), b"previous").unwrap();
    let prepared = prepare(&request(&server, &dest), &CancelToken::new()).unwrap();
    // the server now serves different bytes than the file list announced
    server.behavior.lock().unwrap().corrupt_pbo = true;
    let error = execute(
        &prepared,
        OperationId::new(1),
        &mut MemoryEventSink::default(),
        &CancelToken::new(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity);
    assert_eq!(fs::read(dest.join(PBO)).unwrap(), b"previous");
    assert_eq!(names(&dest), [PBO]);
    fs::remove_dir_all(dest).unwrap();
}

#[test]
fn a_local_change_between_planning_and_running_makes_the_plan_stale() {
    let server = serve();
    let dest = scratch("stale");
    let prepared = prepare(&request(&server, &dest), &CancelToken::new()).unwrap();
    // someone installs one of the files meanwhile: the plan no longer matches the destination
    fs::create_dir_all(dest.join("@example_core/keys")).unwrap();
    fs::write(dest.join(BIKEY), bikey()).unwrap();
    let error = execute(
        &prepared,
        OperationId::new(1),
        &mut MemoryEventSink::default(),
        &CancelToken::new(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity);
    assert!(error.message().contains("stale"), "{error}");
    assert!(
        !dest.join(PBO).exists(),
        "nothing was installed from a stale plan"
    );
    fs::remove_dir_all(dest).unwrap();
}

#[test]
fn a_folder_where_a_file_belongs_and_a_file_where_a_folder_belongs_are_path_conflicts() {
    for (name, conflict_path, is_dir) in [
        ("dir-for-file", PBO, true),
        ("file-for-dir", "@example_core", false),
    ] {
        let server = serve();
        let dest = scratch(name);
        let conflict = dest.join(conflict_path);
        if is_dir {
            fs::create_dir_all(&conflict).unwrap();
            fs::write(conflict.join("keep.txt"), b"mine").unwrap();
        } else {
            fs::write(&conflict, b"i am a file").unwrap();
        }
        let prepared = prepare(&request(&server, &dest), &CancelToken::new()).unwrap();
        let error = execute(
            &prepared,
            OperationId::new(1),
            &mut MemoryEventSink::default(),
            &CancelToken::new(),
        )
        .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::FileSystem, "{name}: {error}");
        if is_dir {
            assert_eq!(
                fs::read(conflict.join("keep.txt")).unwrap(),
                b"mine",
                "{name}"
            );
        } else {
            assert_eq!(fs::read(&conflict).unwrap(), b"i am a file", "{name}");
        }
        assert!(
            !names(&dest).iter().any(|n| n.contains(".armasync-")),
            "{name}: no temporary file"
        );
        fs::remove_dir_all(dest).unwrap();
    }
}

#[test]
fn a_read_only_target_file_is_not_replaced_and_nothing_is_left_behind() {
    let server = serve();
    let dest = scratch("readonly");
    fs::create_dir_all(dest.join("@example_core/addons")).unwrap();
    let target = dest.join(PBO);
    fs::write(&target, b"protected by the file system").unwrap();
    let mut permissions = fs::metadata(&target).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&target, permissions).unwrap();
    let prepared = prepare(&request(&server, &dest), &CancelToken::new()).unwrap();
    let outcome = execute(
        &prepared,
        OperationId::new(1),
        &mut MemoryEventSink::default(),
        &CancelToken::new(),
    );
    // Windows refuses to replace a read-only file; the behaviour on other systems depends on the folder rights
    // (a read-only file in a writable folder can be replaced there) and is part of the deferred platform checks.
    if cfg!(windows) {
        assert_eq!(outcome.unwrap_err().kind(), ErrorKind::FileSystem);
        assert_eq!(fs::read(&target).unwrap(), b"protected by the file system");
        assert!(!names(&dest).iter().any(|n| n.contains(".armasync-")));
    }
    let mut permissions = fs::metadata(&target).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(&target, permissions).unwrap();
    fs::remove_dir_all(dest).unwrap();
}
