// SPDX-License-Identifier: AGPL-3.0-only
//! Headless UI tests of the upload view (MIG-042, step 4b) with the AccessKit tree: names of every field, the live
//! status line, the publish gate, keyboard activation, the preview's words, both themes, and the contrast of the
//! error colour. These exercise the tree that UI Automation and AT-SPI receive; they are **not** a substitute for
//! running a screen reader, which has not been done.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use armasync_gui::contrast::contrast_ratio;
use armasync_gui::ui::{
    ThemeChoice, ViewSettings, apply_view_settings, error_color, themed_visuals,
};
use armasync_gui::upload_screen::UploadScreen;
use armasync_gui::upload_ui::{PREVIEW_LABEL, PUBLISH_LABEL, show_upload};
use armasync_testkit::webdav::start;
use eframe::egui::accesskit::{Live, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

struct App {
    upload: UploadScreen,
    settings: ViewSettings,
    applied: Option<ViewSettings>,
}

fn harness(screen: UploadScreen, theme: ThemeChoice) -> Harness<'static, App> {
    let settings = ViewSettings {
        theme,
        ..ViewSettings::default()
    };
    Harness::builder()
        .with_size([1024.0, 900.0])
        .with_max_steps(8)
        .build_ui_state(
            |ui, state: &mut App| {
                state.upload.poll();
                apply_view_settings(ui.ctx(), &state.settings, &mut state.applied);
                show_upload(&mut state.upload, ui);
            },
            App {
                upload: screen,
                settings,
                applied: None,
            },
        )
}

fn tree(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "armasync-gui-upload-ui-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("a.txt"), b"alpha").unwrap();
    fs::write(root.join("docs/b.txt"), b"beta").unwrap();
    fs::write(root.join(".armasync-lock"), b"").unwrap();
    root
}

fn step_until(harness: &mut Harness<'_, App>, what: &str, condition: impl Fn(&App) -> bool) {
    let started = Instant::now();
    while !condition(harness.state()) {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "timed out: {what}"
        );
        thread::sleep(Duration::from_millis(10));
        harness.step();
    }
    harness.run();
}

#[test]
fn every_field_has_a_visible_name_and_publishing_is_not_offered_before_a_preview() {
    let mut harness = harness(UploadScreen::new(|| {}), ThemeChoice::Light);
    harness.run();
    assert!(
        harness.unnamed_widgets().is_empty(),
        "{:?}",
        harness.unnamed_widgets()
    );

    harness.get_by_role_and_label(
        Role::TextInput,
        "Quellordner (lokal; wird nicht verändert):",
    );
    harness.get_by_role_and_label(
        Role::TextInput,
        "Zieladresse (https://…, ohne Benutzername und Kennwort):",
    );
    harness.get_by_role_and_label(Role::TextInput, "Benutzername (optional, nur über HTTPS):");
    // The password field is masked; its accessibility role differs from plain text, so it is found by its label.
    harness.get_by_label("Kennwort (nur für diese Sitzung, wird nicht gespeichert):");
    harness.get_by_role_and_label(
        Role::MultilineTextInput,
        "Ausschlüsse (je Zeile ein Datei- oder Ordnerpfad, relativ zur Quelle):",
    );
    let preview = harness.get_by_role_and_label(Role::Button, PREVIEW_LABEL);
    assert!(!preview.accesskit_node().is_disabled());
    assert!(
        harness
            .query_by_role_and_label(Role::Button, PUBLISH_LABEL)
            .is_none(),
        "publishing is offered only for a checked preview"
    );
    harness.get_by_label("Bereit. Quellordner und Zieladresse angeben, dann die Vorschau prüfen.");
}

#[test]
fn the_status_line_is_a_polite_live_region() {
    let mut harness = harness(UploadScreen::new(|| {}), ThemeChoice::Light);
    harness.run();
    let status = harness
        .get_by_label("Bereit. Quellordner und Zieladresse angeben, dann die Vorschau prüfen.");
    assert_eq!(status.accesskit_node().live(), Live::Polite);
}

#[test]
fn the_preview_button_runs_by_keyboard_and_the_preview_names_each_file_in_words() {
    let root = tree("keyboard");
    let server = start(false, false, "");
    let mut screen = UploadScreen::new(|| {});
    screen.form.source = root.to_string_lossy().into_owned();
    screen.form.target = format!("http://{}/dav", server.addr);
    screen.form.exclusions = "docs".to_owned();
    let mut harness = harness(screen, ThemeChoice::Light);
    harness.run();

    harness
        .get_by_role_and_label(Role::Button, PREVIEW_LABEL)
        .focus();
    harness.key_press(eframe::egui::Key::Enter);
    step_until(&mut harness, "the preview", |app| !app.upload.is_busy());

    harness.get_by_label("Hochladen: a.txt (5 B)");
    harness.get_by_label("Ausgeschlossen: .armasync-lock (0 B), Grund: Interne ArmaSync-Daten");
    harness.get_by_label("Ausgeschlossen: docs/b.txt (4 B), Grund: ausgeschlossen durch »docs«");
    // The publish button appears only after the preview, and it stays disabled until the confirmation is ticked.
    let publish = harness.get_by_role_and_label(Role::Button, PUBLISH_LABEL);
    assert!(publish.accesskit_node().is_disabled());
    harness.get_by_role_and_label(
        Role::CheckBox,
        "Ich habe die Vorschau geprüft und bestätige die Veröffentlichung.",
    );
    assert_eq!(server.puts(), 0, "a preview writes nothing");
    fs::remove_dir_all(root).ok();
}

#[test]
fn the_view_renders_in_both_themes_with_the_same_labels() {
    for theme in [ThemeChoice::Light, ThemeChoice::Dark] {
        let mut harness = harness(UploadScreen::new(|| {}), theme);
        harness.run();
        assert!(harness.unnamed_widgets().is_empty(), "{theme:?}");
        harness.get_by_role_and_label(Role::Button, PREVIEW_LABEL);
        harness
            .get_by_label("Bereit. Quellordner und Zieladresse angeben, dann die Vorschau prüfen.");
    }
}

#[test]
fn the_error_colour_has_enough_contrast_in_both_themes() {
    let rgb = |c: eframe::egui::Color32| [c.r(), c.g(), c.b()];
    for dark in [false, true] {
        let background = rgb(themed_visuals(dark).panel_fill);
        let ratio = contrast_ratio(rgb(error_color(dark)), background);
        assert!(ratio >= 4.5, "dark={dark}: {ratio:.2}:1");
    }
}
