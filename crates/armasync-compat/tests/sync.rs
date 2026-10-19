// SPDX-License-Identifier: AGPL-3.0-only
//! Tests of the file tree and server information parsers: MIG-001 fixtures, rejected fixtures, crafted trees and
//! hostile structure. All data is synthetic.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use armasync_compat::{
    Limits, ParseErrorKind, SyncDirectory, parse_autoconfig, parse_serverinfo, parse_sync_tree,
};
use common::{Item, TestDir, TestFile, Text, gzip, read_fixture, tree_file, tree_stream};

fn tree(bytes: &[u8]) -> Result<armasync_compat::SyncTree, armasync_compat::ParseError> {
    parse_sync_tree(bytes, &Limits::SYNC)
}

fn root(items: Vec<Item>) -> TestDir {
    TestDir::new("racine", items)
}

fn kind_of(root: &TestDir) -> ParseErrorKind {
    tree(&tree_file(root)).unwrap_err().kind()
}

fn names(dir: &SyncDirectory) -> Vec<&str> {
    dir.directories
        .iter()
        .map(|d| d.name.as_str())
        .chain(dir.files.iter().map(|f| f.name.as_str()))
        .collect()
}

#[test]
fn reads_the_small_fixture_tree() {
    let parsed = tree(&read_fixture("valid/server/small/.a3s/sync")).unwrap();
    assert_eq!(parsed.root.name, "racine");
    assert_eq!(parsed.file_count, 3);
    assert_eq!(names(&parsed.root), ["@example_core", "@second"]);
    let core = &parsed.root.directories[0];
    assert!(core.mark_as_addon && !core.hidden);
    assert_eq!(names(core), ["addons", "keys"]);
    let addons = &core.directories[0];
    let pbo = &addons.files[0];
    assert_eq!(pbo.name, "example_core.pbo");
    assert_eq!(pbo.sha1, "7935137a8950813a6ac435aa2b4c9beea989d744");
    assert_eq!(pbo.size, 8193);
    assert!(!pbo.compressed);
    let zip = &addons.files[1];
    assert!(zip.compressed);
    assert_eq!((zip.size, zip.compressed_size), (100, 0));
    let second = &parsed.root.directories[1];
    assert!(second.hidden && second.mark_as_addon);
    assert!(second.files.is_empty() && second.directories.is_empty());
}

#[test]
fn reads_empty_and_hostile_fixture_trees_as_data() {
    let empty = tree(&read_fixture("valid/server/empty/.a3s/sync")).unwrap();
    assert_eq!((empty.file_count, empty.root.files.len()), (0, 0));
    let hostile = tree(&read_fixture("valid/server/hostile-paths/.a3s/sync")).unwrap();
    // Names are returned exactly as published; path safety is a separate, mandatory layer.
    let all: Vec<_> = hostile.root.files.iter().map(|f| f.name.as_str()).collect();
    for expected in [
        "..",
        "../escape.pbo",
        "/absolute.pbo",
        "back\\slash.pbo",
        "CON",
        "Case.pbo",
        "case.PBO",
    ] {
        assert!(all.contains(&expected), "{expected} missing in {all:?}");
    }
}

#[test]
fn reads_all_server_info_fixtures() {
    let small = parse_serverinfo(
        &read_fixture("valid/server/small/.a3s/serverinfo"),
        &Limits::SYNC,
    )
    .unwrap();
    assert_eq!(small.revision, 3);
    assert_eq!(small.number_of_files, 2);
    assert_eq!(small.total_files_size, 8313);
    assert_eq!(small.number_of_connections, 4);
    assert_eq!(small.hidden_folder_paths, ["@second"]);
    for scenario in ["empty", "hostile-paths"] {
        let info = parse_serverinfo(
            &read_fixture(&format!("valid/server/{scenario}/.a3s/serverinfo")),
            &Limits::SYNC,
        )
        .unwrap();
        assert!(info.revision >= 1, "{scenario}");
    }
}

#[test]
fn the_three_file_kinds_are_not_interchangeable() {
    let sync = read_fixture("valid/server/small/.a3s/sync");
    let info = read_fixture("valid/server/small/.a3s/serverinfo");
    let auto = read_fixture("valid/server/small/.a3s/autoconfig");
    assert!(parse_serverinfo(&sync, &Limits::SYNC).is_err());
    assert!(parse_serverinfo(&auto, &Limits::SYNC).is_err());
    assert!(tree(&info).is_err());
    assert!(tree(&auto).is_err());
    assert!(parse_autoconfig(&sync, &Limits::AUTOCONFIG).is_err());
    for other in ["changelogs", "events"] {
        assert!(tree(&read_fixture(&format!("valid/server/small/.a3s/{other}"))).is_err());
    }
}

#[test]
fn rejects_every_invalid_fixture_for_both_readers() {
    let mut checked = 0;
    for entry in std::fs::read_dir(common::fixture("invalid")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "zip") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        assert!(tree(&bytes).is_err(), "{}", path.display());
        assert!(
            parse_serverinfo(&bytes, &Limits::SYNC).is_err(),
            "{}",
            path.display()
        );
        checked += 1;
    }
    assert_eq!(checked, 15);
}

#[test]
fn crafted_tree_round_trips_in_any_order() {
    let mut file = TestFile::new("b.pbo", 10);
    file.compressed = true;
    file.compressed_size = 4;
    let parsed = tree(&tree_file(&root(vec![
        Item::File(file),
        Item::Dir(TestDir::new(
            "addons",
            vec![Item::File(TestFile::new("a.pbo", 1))],
        )),
        Item::File(TestFile::new("A.pbo", 2)),
    ])))
    .unwrap();
    assert_eq!(parsed.file_count, 3);
    assert_eq!(names(&parsed.root), ["addons", "b.pbo", "A.pbo"]);
    assert_eq!(parsed.root.files[0].compressed_size, 4);
}

#[test]
fn structure_errors_reject_the_whole_document() {
    // wrong root name
    assert_eq!(
        kind_of(&TestDir::new("notroot", vec![])),
        ParseErrorKind::UnexpectedStructure
    );
    // empty names
    assert_eq!(
        kind_of(&root(vec![Item::File(TestFile::new("", 1))])),
        ParseErrorKind::InvalidValue
    );
    assert_eq!(
        kind_of(&root(vec![Item::Dir(TestDir::new("", vec![]))])),
        ParseErrorKind::InvalidValue
    );
    // negative sizes
    assert_eq!(
        kind_of(&root(vec![Item::File(TestFile::new("x", -1))])),
        ParseErrorKind::InvalidValue
    );
    // bad checksums
    for bad in [
        "",
        "zz23456789abcdef0123456789abcdef01234567",
        "0123456789abcdef0123456789abcdef0123456",
        "0123456789abcdef0123456789abcdef012345678",
    ] {
        let mut file = TestFile::new("x", 1);
        file.sha1 = Text::Str(bad.to_owned());
        assert_eq!(
            kind_of(&root(vec![Item::File(file)])),
            ParseErrorKind::InvalidValue,
            "{bad:?}"
        );
    }
    let mut missing = TestFile::new("x", 1);
    missing.sha1 = Text::Null;
    assert_eq!(
        kind_of(&root(vec![Item::File(missing)])),
        ParseErrorKind::UnexpectedStructure
    );
    // an element that is neither folder nor file
    let junk = Item::Raw(vec![0x74, 0x00, 0x01, b'x']);
    assert_eq!(
        kind_of(&root(vec![junk])),
        ParseErrorKind::UnexpectedStructure
    );
}

#[test]
fn a_node_that_appears_twice_or_in_a_cycle_is_rejected() {
    // The root object is the fifth handle of the stream (class descriptor, three field type strings, object).
    let reference_to_root = Item::Raw(vec![0x71, 0x00, 0x7E, 0x00, 0x04]);
    assert_eq!(
        kind_of(&root(vec![reference_to_root])),
        ParseErrorKind::UnexpectedStructure
    );
}

#[test]
fn depth_and_size_limits_hold() {
    let mut nested = TestDir::new("leaf", vec![Item::File(TestFile::new("f", 1))]);
    for level in 0..120 {
        nested = TestDir::new(&format!("d{level}"), vec![Item::Dir(nested)]);
    }
    let deep = root(vec![Item::Dir(nested)]);
    assert!(matches!(kind_of(&deep), ParseErrorKind::LimitExceeded));
    let wide = root(
        (0..200)
            .map(|i| Item::File(TestFile::new(&format!("f{i}"), 1)))
            .collect(),
    );
    let file = tree_file(&wide);
    let few = Limits {
        max_handles: 50,
        ..Limits::SYNC
    };
    assert_eq!(
        parse_sync_tree(&file, &few).unwrap_err().kind(),
        ParseErrorKind::LimitExceeded
    );
    let small_output = Limits {
        max_decompressed: 100,
        ..Limits::SYNC
    };
    assert_eq!(
        parse_sync_tree(&file, &small_output).unwrap_err().kind(),
        ParseErrorKind::TooLarge
    );
    assert_eq!(tree(&file).unwrap().file_count, 200);
}

#[test]
fn unknown_class_version_is_rejected() {
    let mut stream = tree_stream(&root(vec![Item::File(TestFile::new("x", 1))]));
    // Change the last byte of the file class version number (appears after its name).
    let needle = b"fr.soe.a3s.domain.repository.SyncTreeLeaf";
    let at = stream
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap()
        + needle.len()
        + 7;
    stream[at] ^= 0x01;
    assert_eq!(
        tree(&gzip(&stream)).unwrap_err().kind(),
        ParseErrorKind::UnknownClass
    );
}

#[test]
fn every_truncation_is_rejected_and_every_corruption_survives() {
    let stream = tree_stream(&root(vec![
        Item::Dir(TestDir::new(
            "a",
            vec![Item::File(TestFile::new("f.pbo", 3))],
        )),
        Item::File(TestFile::new("g.pbo", 4)),
    ]));
    assert!(tree(&gzip(&stream)).is_ok());
    for end in 0..stream.len() {
        assert!(tree(&gzip(&stream[..end])).is_err(), "prefix {end}");
    }
    for index in 0..stream.len() {
        let mut changed = stream.clone();
        changed[index] ^= 0xFF;
        let _ = tree(&gzip(&changed));
    }
}

#[test]
fn errors_never_contain_names_or_checksums() {
    let mut file = TestFile::new("marker-name-xyz", 1);
    file.sha1 = "marker-checksum-xyz".into();
    let error = tree(&tree_file(&root(vec![Item::File(file)]))).unwrap_err();
    let shown = format!("{error} {error:?}");
    assert!(!shown.contains("marker"), "{shown}");
}
