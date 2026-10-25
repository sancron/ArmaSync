// SPDX-License-Identifier: AGPL-3.0-only
//! Behaviour of the upload screen (MIG-042, step 4b) against the loopback WebDAV server of the test kit, anonymous
//! HTTP only. Preview, confirmation, staleness, cancellation, refusals before any connection, and that the password
//! never reaches the screen's messages. Synthetic data only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use armasync_gui::upload_screen::{UploadScreen, UploadStage};
use armasync_testkit::webdav::{Server, start};

fn tree(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("armasync-gui-upload-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("a.txt"), b"alpha").unwrap();
    fs::write(root.join("docs/b.txt"), b"beta").unwrap();
    fs::write(root.join(".armasync-lock"), b"").unwrap();
    root
}

fn screen_for(root: &Path, server: &Server) -> UploadScreen {
    let mut screen = UploadScreen::new(|| {});
    screen.form.source = root.to_string_lossy().into_owned();
    screen.form.target = format!("http://{}/dav", server.addr);
    screen
}

/// Polls the screen until no worker runs (a result is always produced; nothing waits forever).
fn settle(screen: &mut UploadScreen, what: &str) {
    let started = Instant::now();
    while screen.is_busy() {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "timed out: {what}"
        );
        thread::sleep(Duration::from_millis(10));
        screen.poll();
    }
}

fn preview_and_settle(screen: &mut UploadScreen) {
    screen.request_preview();
    settle(screen, "the preview");
}

#[test]
fn the_preview_lists_included_and_excluded_files_with_reasons_and_writes_nothing() {
    let root = tree("preview");
    let server = start(false, false, "");
    let mut screen = screen_for(&root, &server);
    screen.form.exclusions = "docs".to_owned();
    preview_and_settle(&mut screen);

    assert_eq!(
        screen.stage(),
        UploadStage::Previewed,
        "{:?}",
        screen.error()
    );
    let preview = screen.preview().unwrap();
    let included: Vec<&str> = preview.included.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(included, ["a.txt"]);
    let excluded: Vec<(&str, &str)> = preview
        .excluded
        .iter()
        .map(|r| (r.path.as_str(), r.reason.as_str()))
        .collect();
    assert_eq!(
        excluded,
        [
            (".armasync-lock", "Interne ArmaSync-Daten"),
            ("docs/b.txt", "ausgeschlossen durch »docs«"),
        ]
    );
    assert_eq!(server.puts(), 0, "a preview writes nothing");
    assert!(server.file_names().is_empty());
    fs::remove_dir_all(root).ok();
}

#[test]
fn publishing_needs_a_checked_and_confirmed_preview() {
    let root = tree("gate");
    let server = start(false, false, "");
    let mut screen = screen_for(&root, &server);
    assert!(screen.publish_blocker().unwrap().contains("Vorschau"));
    preview_and_settle(&mut screen);
    assert!(screen.publish_blocker().unwrap().contains("bestätigen"));
    screen.request_publish();
    assert_eq!(
        screen.stage(),
        UploadStage::Previewed,
        "not confirmed: nothing starts"
    );
    assert!(server.seen().iter().all(|s| s.method != "PUT"));

    screen.set_confirmed(true);
    assert!(screen.publish_blocker().is_none());
    screen.request_publish();
    settle(&mut screen, "the publication");
    assert_eq!(
        screen.stage(),
        UploadStage::Published,
        "{:?}",
        screen.error()
    );
    assert!(server.file("current.json").is_some());
    fs::remove_dir_all(root).ok();
}

#[test]
fn editing_an_input_after_the_preview_makes_it_stale() {
    let root = tree("stale");
    let server = start(false, false, "");
    let mut screen = screen_for(&root, &server);
    preview_and_settle(&mut screen);
    screen.set_confirmed(true);
    screen.form.exclusions = "docs".to_owned();
    assert!(!screen.preview_is_current());
    assert!(screen.publish_blocker().unwrap().contains("geändert"));
    screen.request_publish();
    assert_eq!(screen.stage(), UploadStage::Previewed);
    assert!(server.file("current.json").is_none());
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_cancelled_publication_keeps_the_previous_pointer() {
    let root = tree("cancel");
    let server = start(false, false, "");
    let mut screen = screen_for(&root, &server);
    preview_and_settle(&mut screen);
    screen.set_confirmed(true);
    screen.request_publish();
    screen.cancel();
    settle(&mut screen, "the cancelled publication");
    // The cancel may land before or after the last step; both are correct, a half-published pointer is not.
    match screen.stage() {
        UploadStage::Cancelled => {
            assert!(
                server.file("current.json").is_none(),
                "a cancelled run leaves no pointer"
            );
            assert!(screen.announcement().contains("bleibt unverändert"));
        }
        UploadStage::Published => assert!(server.file("current.json").is_some()),
        other => panic!("unexpected stage {other:?}: {:?}", screen.error()),
    }
    fs::remove_dir_all(root).ok();
}

#[test]
fn credentials_over_plain_http_are_reported_without_connecting() {
    let root = tree("plain-http");
    let server = start(false, false, "");
    let mut screen = screen_for(&root, &server);
    screen.form.user = "u".to_owned();
    screen.form.password = "pw-secret-value".to_owned();
    preview_and_settle(&mut screen);
    assert_eq!(screen.stage(), UploadStage::Failed);
    let message = screen.error().unwrap().to_owned();
    assert!(message.contains("HTTPS"), "{message}");
    assert!(!message.contains("pw-secret-value"), "{message}");
    assert!(server.seen().is_empty(), "no connection is made");
    fs::remove_dir_all(root).ok();
}

#[test]
fn an_address_with_credentials_is_refused_before_connecting() {
    let root = tree("userinfo");
    let server = start(false, false, "");
    let mut screen = screen_for(&root, &server);
    screen.form.target = format!("http://u:leak-me@{}/dav", server.addr);
    preview_and_settle(&mut screen);
    assert_eq!(screen.stage(), UploadStage::Failed);
    assert!(!screen.error().unwrap().contains("leak-me"));
    assert!(server.seen().is_empty());
    fs::remove_dir_all(root).ok();
}

#[test]
fn the_status_sentence_names_the_stage_in_words() {
    let root = tree("status");
    let server = start(false, false, "");
    let mut screen = screen_for(&root, &server);
    assert!(screen.announcement().starts_with("Bereit."));
    preview_and_settle(&mut screen);
    assert!(
        screen.announcement().starts_with("Vorschau bereit:"),
        "{}",
        screen.announcement()
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn reset_returns_to_editing_and_keeps_the_form() {
    let root = tree("reset");
    let server = start(false, false, "");
    let mut screen = screen_for(&root, &server);
    preview_and_settle(&mut screen);
    screen.reset();
    assert_eq!(screen.stage(), UploadStage::Editing);
    assert!(screen.preview().is_none());
    assert!(!screen.form.source.is_empty());
    fs::remove_dir_all(root).ok();
}
