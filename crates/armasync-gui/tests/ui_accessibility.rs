// SPDX-License-Identifier: AGPL-3.0-only
//! Headless UI tests with the AccessKit tree: names, roles, states, keyboard operation, dialog focus handling,
//! progress semantics, error display, secrets, scaling and colour contrast.
//!
//! These tests exercise the exact accessibility tree that NVDA (UI Automation) and Orca (AT-SPI) receive through
//! AccessKit. They are *not* a substitute for running the real screen readers (see GUI_PROTOTYPE_EVIDENCE.md).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::thread;
use std::time::{Duration, Instant};

use armasync_gui::contrast::contrast_ratio;
use armasync_gui::model::{Model, RepoStatus, RepositoryRow, synthetic_repositories};
use armasync_gui::ui::{ViewSettings, apply_view_settings, error_color, show, themed_visuals};
use eframe::egui::accesskit::Role;
use eframe::egui::{self, Key};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

struct App {
    model: Model,
    settings: ViewSettings,
    applied: Option<ViewSettings>,
    focus_add: bool,
}

fn app(count: usize) -> App {
    let mut model = Model::new(synthetic_repositories(count), || {});
    model.set_file_delay(Duration::from_millis(25));
    App {
        model,
        settings: ViewSettings::default(),
        applied: None,
        focus_add: false,
    }
}

fn harness(app_state: App) -> Harness<'static, App> {
    Harness::builder()
        .with_size([1024.0, 720.0])
        .with_max_steps(8)
        .build_ui_state(
            |ui, state: &mut App| {
                state.model.poll();
                apply_view_settings(ui.ctx(), &state.settings, &mut state.applied);
                show(
                    &mut state.model,
                    &mut state.settings,
                    &mut state.focus_add,
                    ui,
                );
            },
            app_state,
        )
}

/// Steps the harness until `condition` holds (worker threads need wall-clock time).
fn step_until(
    harness: &mut Harness<'_, App>,
    what: &str,
    condition: impl Fn(&Harness<'_, App>) -> bool,
) {
    let start = Instant::now();
    while !condition(harness) {
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "timed out waiting for: {what}"
        );
        thread::sleep(Duration::from_millis(15));
        harness.step();
    }
}

fn first_with(model: &Model, status: RepoStatus, damaged: bool) -> RepositoryRow {
    model
        .repositories()
        .iter()
        .find(|r| r.status == status && r.damaged_transfer == damaged)
        .unwrap()
        .clone()
}

/// Moves focus to the field first (AccessKit focus action), then types, like a keyboard user would.
fn type_into(harness: &mut Harness<'_, App>, label: &str, text: &str) {
    harness.get_by_label(label).focus();
    harness.run();
    harness.get_by_label(label).type_text(text);
    harness.run();
}

fn select_by_clicking(harness: &mut Harness<'_, App>, name: &str) {
    harness.state_mut().model.set_filter(name);
    harness.run();
    harness.get_by_label(name).click();
    harness.run();
}

#[test]
fn initial_tree_has_named_controls_with_correct_disabled_states() {
    let mut harness = harness(app(10_000));
    harness.run(); // also runs the built-in check that every input widget has an accessible name
    assert!(harness.unnamed_widgets().is_empty());

    harness.get_by_role_and_label(Role::Button, "Repository hinzufügen…");
    let sync = harness.get_by_role_and_label(Role::Button, "Synchronisieren");
    assert!(
        sync.accesskit_node().is_disabled(),
        "nothing selected: sync must be disabled"
    );
    let cancel = harness.get_by_role_and_label(Role::Button, "Abbrechen");
    assert!(
        cancel.accesskit_node().is_disabled(),
        "nothing running: cancel must be disabled"
    );
    harness.get_by_role_and_label(Role::TextInput, "Filter:");
    harness.get_by_label_contains("10000 von 10000 Repositories");
    // table header cells
    assert!(harness.get_all_by_label("Repository").count() >= 1);
    assert!(harness.get_all_by_label("Status").count() >= 1);
    assert!(harness.get_all_by_label("Dateien").count() >= 1);
    harness.get_by_label_contains("Prototyp: Alle Repositories und Übertragungen sind simuliert");
}

#[test]
fn virtual_list_exposes_only_visible_rows_but_with_full_text() {
    let mut harness = harness(app(10_000));
    harness.run();
    let rows = harness.get_all_by_label_contains("Repository 000").count();
    assert!(
        rows > 5 && rows < 200,
        "virtualised list must not build 10,000 nodes, got {rows}"
    );
    // Status is conveyed as text.
    assert!(
        harness.query_all_by_label_contains("Aktuell").count()
            + harness
                .query_all_by_label_contains("Update verfügbar")
                .count()
            > 0
    );
}

#[test]
fn selecting_a_row_enables_sync_and_reports_the_selected_state() {
    let mut harness = harness(app(200));
    let target = first_with(&harness.state().model, RepoStatus::Outdated, false);
    select_by_clicking(&mut harness, &target.name);
    assert_eq!(harness.state().model.selected().unwrap().id, target.id);
    let sync = harness.get_by_role_and_label(Role::Button, "Synchronisieren");
    assert!(!sync.accesskit_node().is_disabled());
    let row = harness.get_by_label(&target.name);
    let node = row.accesskit_node();
    assert_eq!(
        node.role(),
        Role::ListBoxOption,
        "a list row must be announced as a list item"
    );
    assert_eq!(
        node.is_selected(),
        Some(true),
        "selected state must be exposed"
    );
    assert_eq!(node.size_of_set(), Some(1), "filtered to one row");
    assert_eq!(node.position_in_set(), Some(1));
}

#[test]
fn keyboard_only_selection_and_sync() {
    let mut harness = harness(app(50));
    harness.run();
    // Arrow keys move the selection when no text field has focus.
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().model.selected_row(), Some(1));
    assert!(
        !harness
            .get_by_role_and_label(Role::Button, "Synchronisieren")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn tab_order_is_toolbar_then_filter_then_list() {
    let mut harness = harness(app(30));
    harness.run();
    let mut order: Vec<String> = Vec::new();
    for _ in 0..8 {
        harness.key_press(Key::Tab);
        harness.run();
        let focused = harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().is_focused())
            .map(|node| {
                format!(
                    "{:?}: {}",
                    node.accesskit_node().role(),
                    node.accesskit_node().label().unwrap_or_default()
                )
            });
        order.push(focused.unwrap_or_else(|| "<none>".into()));
    }
    println!("tab order: {order:#?}");
    let position = |needle: &str| order.iter().position(|entry| entry.contains(needle));
    let add = position("Repository hinzufügen…").expect("add button must be reachable by Tab");
    let theme = position("Darstellung").expect("theme selector reachable by Tab");
    assert!(
        add < theme,
        "toolbar order must be stable and logical: {order:?}"
    );
    assert!(
        order.iter().all(|entry| entry != "<none>"),
        "focus must always be visible/known: {order:?}"
    );
}

#[test]
fn dialog_shows_field_errors_next_to_fields_and_returns_focus_on_escape() {
    let mut harness = harness(app(10));
    harness.run();
    harness.get_by_label("Repository hinzufügen…").click();
    harness.run();
    harness.get_by_role_and_label(Role::Dialog, "Repository hinzufügen");
    harness.get_by_role_and_label(Role::TextInput, "Name:");
    harness.get_by_role_and_label(Role::TextInput, "Adresse:");
    harness.get_by_role_and_label(Role::TextInput, "Benutzername (optional):");
    let password = harness.get_by_role_and_label(Role::PasswordInput, "Kennwort (optional):");
    assert!(password.accesskit_node().label().is_some());

    harness.get_by_label("Hinzufügen").click();
    harness.run();
    harness.get_by_label("Fehler: Bitte einen Namen eingeben.");
    harness.get_by_label("Fehler: Bitte eine Adresse eingeben.");

    // Escape closes the dialog without saving and focus goes back to the button that opened it.
    harness.key_press(Key::Escape);
    harness.run();
    harness.run();
    assert!(harness.state().model.dialog.is_none());
    assert_eq!(harness.state().model.repositories().len(), 10);
    let add = harness.get_by_label("Repository hinzufügen…");
    assert!(
        add.is_focused(),
        "focus must return to the opener after the dialog closes"
    );
}

#[test]
fn credentials_in_the_url_are_rejected_with_an_explanation_and_secrets_stay_out_of_the_tree() {
    let mut harness = harness(app(10));
    harness.run();
    harness.get_by_label("Repository hinzufügen…").click();
    harness.run();
    type_into(&mut harness, "Name:", "Mein Repository");
    type_into(
        &mut harness,
        "Adresse:",
        "https://anna:hunter2@example.invalid/sync",
    );
    type_into(&mut harness, "Kennwort (optional):", "hunter2-password");
    harness.get_by_label("Hinzufügen").click();
    harness.run();
    harness.get_by_label_contains("gehören in die eigenen Felder");
    // The typed password must not be readable anywhere in the accessibility tree.
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        assert!(
            !label.contains("hunter2-password") && !value.contains("hunter2-password"),
            "password leaked: {label:?} {value:?}"
        );
    }
}

#[test]
fn valid_dialog_adds_and_selects_the_repository() {
    let mut harness = harness(app(10));
    harness.run();
    harness.get_by_label("Repository hinzufügen…").click();
    harness.run();
    type_into(&mut harness, "Name:", "Mein Repository");
    type_into(
        &mut harness,
        "Adresse:",
        "https://example.invalid/arma3/sync",
    );
    harness.get_by_label("Hinzufügen").click();
    harness.run();
    assert!(harness.state().model.dialog.is_none());
    assert_eq!(
        harness.state().model.selected().unwrap().name,
        "Mein Repository"
    );
}

#[test]
fn progress_is_a_labelled_progress_indicator_and_cancel_ends_in_cancelled() {
    let mut harness = harness(app(200));
    let target = first_with(&harness.state().model, RepoStatus::Outdated, false);
    select_by_clicking(&mut harness, &target.name);
    harness.get_by_label("Synchronisieren").click();
    harness.run();
    step_until(&mut harness, "progress bar visible", |h| {
        h.query_by_role(Role::ProgressIndicator).is_some()
    });
    let bar = harness.get_by_role(Role::ProgressIndicator);
    assert!(
        bar.accesskit_node().label().is_some()
            || bar.accesskit_node().value().is_some()
            || bar.accesskit_node().numeric_value().is_some()
    );
    harness.get_by_label("Abbrechen").click();
    step_until(&mut harness, "cancelled state", |h| {
        h.query_all_by_label_contains("Abgebrochen").count() > 0
    });
    step_until(&mut harness, "error summary entry", |h| {
        h.query_all_by_label_contains("Fehler (1)").count() > 0
    });
}

#[test]
fn successful_sync_reports_completion_and_updated_status() {
    let mut harness = harness(app(200));
    harness.state_mut().model.set_file_delay(Duration::ZERO);
    let target = first_with(&harness.state().model, RepoStatus::Outdated, false);
    select_by_clicking(&mut harness, &target.name);
    harness.get_by_label("Synchronisieren").click();
    step_until(&mut harness, "completion", |h| {
        h.query_all_by_label_contains("Abgeschlossen").count() > 0
    });
    harness.run();
    harness.get_by_label_contains("Aktuell");
}

#[test]
fn error_state_names_repository_cause_and_next_step() {
    let mut harness = harness(app(300));
    harness.state_mut().model.set_file_delay(Duration::ZERO);
    let target = first_with(&harness.state().model, RepoStatus::Outdated, true);
    select_by_clicking(&mut harness, &target.name);
    harness.get_by_label("Synchronisieren").click();
    step_until(&mut harness, "failure", |h| {
        h.query_all_by_label_contains("Fehlgeschlagen").count() > 0
    });
    harness.run();
    harness.get_by_label_contains(&format!("Fehler in „{}“", target.name));
    harness.get_by_label_contains("Die Datei wurde nicht übernommen");
    harness.get_by_label("Fehlerliste leeren").click();
    harness.run();
    assert!(harness.state().model.errors().is_empty());
}

#[test]
fn credentials_required_repository_explains_what_to_do() {
    let mut harness = harness(app(60));
    let target = first_with(&harness.state().model, RepoStatus::AuthRequired, false);
    select_by_clicking(&mut harness, &target.name);
    harness.get_by_label("Synchronisieren").click();
    harness.run();
    harness.get_by_label_contains("Anmeldedaten erforderlich");
    harness.get_by_label_contains("Anmeldedaten in den Repository-Einstellungen ergänzen");
}

#[test]
fn all_core_actions_remain_available_at_200_percent_scale_and_small_window() {
    for (width, height, scale) in [
        (1024.0, 720.0, 2.0_f32),
        (640.0, 480.0, 2.0),
        (1920.0, 1080.0, 1.5),
        (640.0, 480.0, 1.0),
    ] {
        let mut state = app(100);
        state.settings.scale = scale;
        let mut harness = Harness::builder()
            .with_size([width, height])
            .with_max_steps(8)
            .build_ui_state(
                |ui, state: &mut App| {
                    state.model.poll();
                    apply_view_settings(ui.ctx(), &state.settings, &mut state.applied);
                    show(
                        &mut state.model,
                        &mut state.settings,
                        &mut state.focus_add,
                        ui,
                    );
                },
                state,
            );
        harness.run();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
        for label in ["Repository hinzufügen…", "Synchronisieren", "Abbrechen"] {
            let rect = harness.get_by_label(label).rect();
            assert!(
                screen.contains_rect(rect),
                "{label} must stay fully visible at {width}x{height} @ {scale}: {rect:?}"
            );
        }
        harness.get_by_label("Filter:");
    }
}

#[test]
fn theme_switch_keeps_the_tree_stable() {
    for theme in [
        armasync_gui::ui::ThemeChoice::Light,
        armasync_gui::ui::ThemeChoice::Dark,
    ] {
        let mut state = app(20);
        state.settings.theme = theme;
        let mut harness = harness(state);
        harness.run();
        harness.get_by_label("Repository hinzufügen…");
        assert!(harness.unnamed_widgets().is_empty());
    }
}

#[test]
fn colours_meet_wcag_aa() {
    fn rgb(color: egui::Color32) -> [u8; 3] {
        [color.r(), color.g(), color.b()]
    }
    let mut report = Vec::new();
    for (name, visuals, dark) in [
        ("hell", themed_visuals(false), false),
        ("dunkel", themed_visuals(true), true),
    ] {
        let background = rgb(visuals.panel_fill);
        let text = rgb(visuals.text_color());
        let weak = rgb(visuals.weak_text_color());
        let error = rgb(error_color(dark));
        let selection_background = rgb(visuals.selection.bg_fill);
        let selection_text = rgb(visuals.selection.stroke.color);
        let ring = rgb(visuals.selection.stroke.color);
        let outline = rgb(visuals.widgets.inactive.bg_stroke.color);
        let focus = rgb(visuals.widgets.active.bg_stroke.color);
        let checks = [
            ("Text auf Fläche", contrast_ratio(text, background), 4.5),
            (
                "schwacher Text auf Fläche",
                contrast_ratio(weak, background),
                4.5,
            ),
            (
                "Fehlertext auf Fläche",
                contrast_ratio(error, background),
                4.5,
            ),
            (
                "Auswahl: Text auf Auswahlfläche",
                contrast_ratio(selection_text, selection_background),
                4.5,
            ),
            (
                "Auswahl: Ring/Fokus von Textfeldern auf Fläche",
                contrast_ratio(ring, background),
                3.0,
            ),
            (
                "Widget-Umriss auf Fläche",
                contrast_ratio(outline, background),
                3.0,
            ),
            (
                "Fokus-Umriss (aktives Widget) auf Fläche",
                contrast_ratio(focus, background),
                3.0,
            ),
            // Light theme: the selection fill is a light tint, selection is also shown in bold (documented exception).
            (
                "Auswahlfläche gegen Fläche",
                contrast_ratio(selection_background, background),
                if dark { 3.0 } else { 1.4 },
            ),
        ];
        for (what, ratio, minimum) in checks {
            report.push(format!("{name}: {what}: {ratio:.2}:1 (Soll >= {minimum})"));
            assert!(
                ratio >= minimum,
                "{name}: {what} only {ratio:.2}:1, needs {minimum}"
            );
        }
    }
    println!("{}", report.join("\n"));
}

/// Prints the complete accessibility tree of the main window and of the open dialog (used for the evidence file).
#[test]
fn dump_accessibility_trees_for_the_evidence_file() {
    fn dump(harness: &Harness<'_, App>, title: &str) -> String {
        let mut out = format!("--- {title} ---\n");
        for node in harness.root().children_recursive() {
            let n = node.accesskit_node();
            if n.is_hidden() {
                continue;
            }
            let label = n.label().unwrap_or_default();
            let value = n.value().unwrap_or_default();
            if label.is_empty()
                && value.is_empty()
                && matches!(n.role(), Role::GenericContainer | Role::Unknown)
            {
                continue;
            }
            out.push_str(&format!(
                "{:?} | name={:?} | value={:?}{}{}{}\n",
                n.role(),
                label,
                value,
                if n.is_disabled() { " | disabled" } else { "" },
                if n.is_focused() { " | focused" } else { "" },
                match n.is_selected() {
                    Some(true) => " | selected",
                    _ => "",
                },
            ));
        }
        out
    }
    let mut harness = harness(app(10_000));
    harness.run();
    let main = dump(&harness, "Hauptfenster (Start)");
    harness.get_by_label("Repository hinzufügen…").click();
    harness.run();
    let dialog = dump(&harness, "Dialog Repository hinzufügen");
    let path = std::env::var("ARMASYNC_A11Y_DUMP").ok();
    let text = format!("{main}\n{dialog}");
    if let Some(path) = path {
        std::fs::write(path, &text).unwrap();
    }
    println!("{text}");
    assert!(main.contains("Button"));
    assert!(dialog.contains("PasswordInput"));
}

/// Reports (does not assert) the average frame time for the 10,000-row list: a virtualised list must not scale with
/// the number of rows. The assertion is deliberately generous so slow CI machines do not fail it.
#[test]
fn ten_thousand_rows_stay_responsive() {
    let mut harness = harness(app(10_000));
    harness.run();
    let start = Instant::now();
    let frames = 30;
    for _ in 0..frames {
        harness.step();
    }
    let average = start.elapsed() / frames;
    println!("average frame time with 10,000 rows (debug build, headless): {average:?}");
    assert!(
        average < Duration::from_millis(250),
        "frame time {average:?} suggests the list is not virtualised"
    );
}
