// SPDX-License-Identifier: AGPL-3.0-only
//! Tests of the local file-system adapter in real temporary folders: scan, staging, atomic replacement, deletion,
//! containment against links, cancellation, leftovers and error paths. All data is synthetic.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use armasync_adapters::localfs::{LocalDirectory, STAGING_PREFIX, STALE_STAGING_AGE};
use armasync_core::{CancelToken, ErrorKind, HashAlgorithm, LocalStore, RelativePath};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "armasync-localfs-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

fn put(root: &Path, relative: &str, data: &[u8]) {
    let full = root.join(relative);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(full, data).unwrap();
}

fn names(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if entry.path().is_dir() {
                walk(&entry.path(), &format!("{name}/"), out);
            } else {
                out.push(name);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, "", &mut out);
    out.sort();
    out
}

fn list(store: &LocalDirectory, algorithm: HashAlgorithm) -> Vec<armasync_core::LocalFile> {
    store.list(algorithm, &CancelToken::new()).unwrap()
}

#[test]
fn scan_lists_regular_files_with_sizes_and_hashes_of_the_requested_algorithm() {
    let root = scratch("scan");
    put(&root, "@m/addons/a.pbo", b"abc");
    put(&root, "@m/b.pbo", b"");
    put(&root, "top.txt", b"hello");
    put(&root, "ünï/çödé.pbo", b"x");
    let store = LocalDirectory::new(&root).unwrap();
    let sha1 = list(&store, HashAlgorithm::Sha1);
    let listed: Vec<_> = sha1.iter().map(|f| (f.path.as_str(), f.size)).collect();
    assert_eq!(
        listed,
        [
            ("@m/addons/a.pbo", 3),
            ("@m/b.pbo", 0),
            ("top.txt", 5),
            ("ünï/çödé.pbo", 1)
        ]
    );
    assert_eq!(
        sha1[0].hash.as_ref().unwrap().to_string(),
        "sha1:a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    assert_eq!(
        sha1[1].hash.as_ref().unwrap().to_string(),
        "sha1:da39a3ee5e6b4b0d3255bfef95601890afd80709"
    );
    let sha256 = list(&store, HashAlgorithm::Sha256);
    assert_eq!(
        sha256[0].hash.as_ref().unwrap().to_string(),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scan_is_cancellable() {
    let root = scratch("scan-cancel");
    put(&root, "a.pbo", &vec![1u8; 300_000]);
    let store = LocalDirectory::new(&root).unwrap();
    let cancel = CancelToken::new();
    cancel.cancel();
    assert_eq!(
        store.list(HashAlgorithm::Sha1, &cancel).unwrap_err().kind(),
        ErrorKind::Cancelled
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn staging_creates_parents_and_commits_the_exact_content() {
    let root = scratch("commit");
    let mut store = LocalDirectory::new(&root).unwrap();
    let mut staged = store.begin(&path("@m/addons/new.pbo"), 6).unwrap();
    staged.write_all(b"abc").unwrap();
    staged.write_all(b"def").unwrap();
    staged.commit().unwrap();
    assert_eq!(fs::read(root.join("@m/addons/new.pbo")).unwrap(), b"abcdef");
    assert_eq!(
        names(&root),
        ["@m/addons/new.pbo"],
        "no temporary file remains"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_target_keeps_its_old_content_until_commit_and_is_replaced_atomically() {
    let root = scratch("replace");
    put(&root, "a.pbo", b"old content");
    let mut store = LocalDirectory::new(&root).unwrap();
    {
        let mut staged = store.begin(&path("a.pbo"), 3).unwrap();
        staged.write_all(b"new").unwrap();
        // while the new content is being written, the old file is complete and unchanged
        assert_eq!(fs::read(root.join("a.pbo")).unwrap(), b"old content");
        let during: Vec<_> = names(&root);
        assert_eq!(
            during.len(),
            2,
            "target plus one temporary file: {during:?}"
        );
        assert!(during.iter().any(|n| n.starts_with(STAGING_PREFIX)));
        staged.commit().unwrap();
    }
    assert_eq!(fs::read(root.join("a.pbo")).unwrap(), b"new");
    assert_eq!(names(&root), ["a.pbo"]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn staging_files_are_invisible_to_the_scan_and_dropping_removes_them() {
    let root = scratch("drop");
    put(&root, "a.pbo", b"old");
    let mut store = LocalDirectory::new(&root).unwrap();
    {
        let mut staged = store.begin(&path("a.pbo"), 5).unwrap();
        staged.write_all(b"partial").unwrap();
    }
    assert_eq!(fs::read(root.join("a.pbo")).unwrap(), b"old");
    assert_eq!(
        names(&root),
        ["a.pbo"],
        "the temporary file is gone after a drop without commit"
    );
    // a leftover that a crash would have left behind is not listed either
    put(&root, ".armasync-1-1-1.tmp", b"stuff");
    let listed = list(&store, HashAlgorithm::Sha1);
    assert_eq!(
        listed.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(),
        ["a.pbo"]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_leftovers_are_removed_by_a_scan_but_fresh_ones_are_not() {
    let root = scratch("stale");
    let store = LocalDirectory::new(&root).unwrap();
    put(&root, ".armasync-old.tmp", b"x");
    put(&root, "sub/.armasync-fresh.tmp", b"y");
    let old = File::options()
        .write(true)
        .open(root.join(".armasync-old.tmp"))
        .unwrap();
    old.set_modified(SystemTime::now() - STALE_STAGING_AGE - Duration::from_secs(60))
        .unwrap();
    drop(old);
    list(&store, HashAlgorithm::Sha1);
    assert_eq!(names(&root), ["sub/.armasync-fresh.tmp"]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reserved_staging_names_cannot_be_repository_files() {
    let root = scratch("reserved");
    let mut store = LocalDirectory::new(&root).unwrap();
    let error = store
        .begin(&path("@m/.armasync-evil.tmp"), 1)
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::FileSystem);
    assert!(names(&root).is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn only_regular_files_are_replaced_or_deleted() {
    let root = scratch("kinds");
    fs::create_dir_all(root.join("dir.pbo")).unwrap();
    put(&root, "file.pbo", b"f");
    put(&root, "parentfile", b"p");
    let mut store = LocalDirectory::new(&root).unwrap();
    // a folder where a file is expected
    assert_eq!(
        store
            .begin(&path("dir.pbo"), 1)
            .map(|_| ())
            .unwrap_err()
            .kind(),
        ErrorKind::FileSystem
    );
    assert_eq!(
        store.delete(&path("dir.pbo")).unwrap_err().kind(),
        ErrorKind::FileSystem
    );
    assert!(root.join("dir.pbo").is_dir());
    // a file where a folder is expected
    assert_eq!(
        store
            .begin(&path("parentfile/x.pbo"), 1)
            .map(|_| ())
            .unwrap_err()
            .kind(),
        ErrorKind::FileSystem
    );
    // delete removes exactly that file; a missing file is an error
    store.delete(&path("file.pbo")).unwrap();
    assert!(!root.join("file.pbo").exists());
    assert_eq!(
        store.delete(&path("file.pbo")).unwrap_err().kind(),
        ErrorKind::FileSystem
    );
    assert!(root.join("parentfile").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_destination_must_be_an_existing_folder() {
    let root = scratch("root");
    assert_eq!(
        LocalDirectory::new(root.join("missing"))
            .unwrap_err()
            .kind(),
        ErrorKind::FileSystem
    );
    put(&root, "file", b"x");
    assert_eq!(
        LocalDirectory::new(root.join("file")).unwrap_err().kind(),
        ErrorKind::FileSystem
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn error_messages_name_the_relative_path_but_not_the_location() {
    let root = scratch("messages");
    let mut store = LocalDirectory::new(&root).unwrap();
    let error = store.delete(&path("nothing/here.pbo")).unwrap_err();
    assert!(error.message().contains("nothing/here.pbo"), "{error}");
    assert!(
        !error.message().contains(&*root.to_string_lossy()),
        "{error}"
    );
    fs::remove_dir_all(root).unwrap();
}

/// A link from `link` to `target`: a junction on Windows (no privileges needed), a symbolic link elsewhere.
fn make_link(link: &Path, target: &Path) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .is_ok_and(|o| o.status.success())
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

#[test]
fn links_never_lead_out_of_the_destination() {
    let root = scratch("links-root");
    let outside = scratch("links-outside");
    put(&outside, "secret.txt", b"outside data");
    put(&outside, "victim.pbo", b"victim");
    assert!(
        make_link(&root.join("@linked"), &outside),
        "the test needs to create a link"
    );
    put(&root, "real.pbo", b"real");
    let mut store = LocalDirectory::new(&root).unwrap();
    // the scan does not descend into the link
    let listed = list(&store, HashAlgorithm::Sha1);
    assert_eq!(
        listed.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(),
        ["real.pbo"]
    );
    // writing, replacing and deleting through the link are refused and change nothing
    for target in [
        "@linked/new.pbo",
        "@linked/victim.pbo",
        "@linked/deep/er.pbo",
    ] {
        let error = store.begin(&path(target), 1).map(|_| ()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::FileSystem, "{target}");
    }
    assert_eq!(
        store
            .delete(&path("@linked/victim.pbo"))
            .unwrap_err()
            .kind(),
        ErrorKind::FileSystem
    );
    assert_eq!(
        store
            .write(&path("@linked/new.pbo"), b"x")
            .unwrap_err()
            .kind(),
        ErrorKind::FileSystem
    );
    assert_eq!(
        names(&outside),
        ["secret.txt", "victim.pbo"],
        "nothing outside was touched"
    );
    assert_eq!(fs::read(outside.join("victim.pbo")).unwrap(), b"victim");
    // a link as the file itself is refused as well
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.join("victim.pbo"), root.join("linkfile.pbo")).unwrap();
        assert!(store.begin(&path("linkfile.pbo"), 1).map(|_| ()).is_err());
    }
    // remove the link without following it, then the folders
    #[cfg(windows)]
    fs::remove_dir(root.join("@linked")).unwrap();
    #[cfg(unix)]
    fs::remove_file(root.join("@linked")).unwrap();
    fs::remove_dir_all(root).unwrap();
    assert_eq!(names(&outside), ["secret.txt", "victim.pbo"]);
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn a_link_as_the_destination_is_refused() {
    let real = scratch("dest-real");
    let holder = scratch("dest-holder");
    assert!(make_link(&holder.join("dest"), &real));
    assert_eq!(
        LocalDirectory::new(holder.join("dest")).unwrap_err().kind(),
        ErrorKind::FileSystem
    );
    #[cfg(windows)]
    fs::remove_dir(holder.join("dest")).unwrap();
    #[cfg(unix)]
    fs::remove_file(holder.join("dest")).unwrap();
    fs::remove_dir_all(holder).unwrap();
    fs::remove_dir_all(real).unwrap();
}

#[cfg(windows)]
#[test]
fn a_locked_target_fails_the_commit_and_keeps_the_old_file() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = scratch("locked");
    put(&root, "a.pbo", b"old");
    let mut store = LocalDirectory::new(&root).unwrap();
    // another program holds the file open without sharing
    let lock = File::options()
        .read(true)
        .share_mode(0)
        .open(root.join("a.pbo"))
        .unwrap();
    // the scan still works: the file cannot be hashed, so it counts as changed
    let listed = list(&store, HashAlgorithm::Sha1);
    assert_eq!((listed[0].size, listed[0].hash.is_none()), (3, true));
    let mut staged = store.begin(&path("a.pbo"), 3).unwrap();
    staged.write_all(b"new").unwrap();
    let error = staged.commit().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::FileSystem);
    drop(lock);
    assert_eq!(fs::read(root.join("a.pbo")).unwrap(), b"old");
    assert_eq!(names(&root), ["a.pbo"], "the temporary file was removed");
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn unusual_names_are_not_listed_and_so_never_touched() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let root = scratch("odd-names");
    let odd = root.join(OsString::from_vec(vec![
        b'b', b'a', b'd', 0xFF, b'.', b'p', b'b', b'o',
    ]));
    fs::write(&odd, b"odd").unwrap();
    put(&root, "ok.pbo", b"ok");
    let store = LocalDirectory::new(&root).unwrap();
    let listed = list(&store, HashAlgorithm::Sha1);
    assert_eq!(
        listed.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(),
        ["ok.pbo"]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn write_is_a_staged_commit_for_small_files() {
    let root = scratch("write");
    let mut store = LocalDirectory::new(&root).unwrap();
    store.write(&path("a/b.pbo"), b"data").unwrap();
    store.write(&path("a/b.pbo"), b"newer").unwrap();
    assert_eq!(fs::read(root.join("a/b.pbo")).unwrap(), b"newer");
    assert_eq!(names(&root), ["a/b.pbo"]);
    fs::remove_dir_all(root).unwrap();
}
