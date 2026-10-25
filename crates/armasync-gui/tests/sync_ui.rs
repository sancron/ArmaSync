// SPDX-License-Identifier: AGPL-3.0-only
//! Headless UI tests of the synchronisation screen with the AccessKit tree: names, roles, states, keyboard
//! operation, confirmation of deletions, progress semantics, error display and the platform notice.
//!
//! These tests exercise the exact accessibility tree that NVDA (UI Automation) and Orca (AT-SPI) receive through
//! AccessKit. They are **not** a substitute for running the real screen readers, which has not been done (see
//! `docs/migration/GUI_SYNC_EVIDENCE.md`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use armasync_adapters::LibraryStore;
use armasync_core::{DisplayName, Library, Repository, RepositoryId, RepositoryUrl};
use armasync_gui::sync_screen::{Outcome, Stage, SyncScreen};
use armasync_gui::sync_ui::{CANCEL_LABEL, PLAN_LABEL, START_LABEL, show_sync};
use armasync_gui::ui::{ViewSettings, apply_view_settings};
use armasync_testkit::{Behavior, PBO, ZIP, serve};
use armasync_workflow::{PlatformStatus, platform_status_for};
use eframe::egui::accesskit::{Live, Role};
use eframe::egui::{self, Key};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

struct App {
    screen: SyncScreen,
    settings: ViewSettings,
    applied: Option<ViewSettings>,
}

struct Env {
    root: PathBuf,
    config: PathBuf,
    dest: PathBuf,
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn env_with(url: &str) -> Env {
    let base = std::env::temp_dir().join(format!(
        "armasync-gui-ui-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let mut root = base.clone();
    let mut attempt = 0u32;
    while fs::create_dir(&root).is_err() {
        attempt += 1;
        assert!(attempt < 1000);
        root = PathBuf::from(format!("{}-{attempt}", base.display()));
    }
    let (config, dest) = (root.join("config"), root.join("dest"));
    fs::create_dir_all(&config).unwrap();
    fs::create_dir_all(&dest).unwrap();
    let repository = Repository {
        id: RepositoryId::parse("example").unwrap(),
        name: DisplayName::parse("Example").unwrap(),
        url: RepositoryUrl::parse(url).unwrap(),
        credential: None,
    };
    LibraryStore::new(config.join("library.json"))
        .save(&Library::new(vec![repository], Vec::new()).unwrap())
        .unwrap();
    Env { root, config, dest }
}

fn harness_for(env: &Env, platform: PlatformStatus) -> Harness<'static, App> {
    let mut screen = SyncScreen::new(Some(env.config.clone()), platform, || {});
    screen.destination = env.dest.to_string_lossy().into_owned();
    Harness::builder()
        .with_size([1024.0, 900.0])
        .with_max_steps(8)
        .build_ui_state(
            |ui, state: &mut App| {
                state.screen.poll();
                apply_view_settings(ui.ctx(), &state.settings, &mut state.applied);
                show_sync(&mut state.screen, &mut state.settings, ui);
            },
            App {
                screen,
                settings: ViewSettings::default(),
                applied: None,
            },
        )
}

fn step_until(harness: &mut Harness<'_, App>, what: &str, condition: impl Fn(&App) -> bool) {
    let started = Instant::now();
    while !condition(harness.state()) {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "timed out waiting for: {what}"
        );
        thread::sleep(Duration::from_millis(15));
        harness.step();
    }
    harness.run();
}

fn plan_via_ui(harness: &mut Harness<'_, App>) {
    harness
        .get_by_role_and_label(Role::Button, PLAN_LABEL)
        .click();
    harness.run();
    step_until(harness, "the plan", |app| !app.screen.is_busy());
}

#[test]
fn the_initial_tree_names_every_control_and_announces_the_platform() {
    let server = serve();
    let env = env_with(&format!("http://{}/repo", server.addr));
    let mut harness = harness_for(&env, platform_status_for("windows"));
    harness.run();
    assert!(
        harness.unnamed_widgets().is_empty(),
        "every input widget needs an accessible name"
    );
    harness.get_by_role_and_label(Role::TextInput, "Zielordner (muss vorhanden sein):");
    harness.get_by_role_and_label(Role::Button, PLAN_LABEL);
    harness.get_by_role_and_label(Role::CheckBox, "Zusätzliche Dateien im Zielordner löschen");
    // headings structure the page
    for heading in ["1. Repository", "2. Zielordner und Optionen"] {
        harness.get_by_label(heading);
    }
    // before a plan exists there is no start button, and the platform note says what is and is not verified
    assert!(
        harness
            .query_by_role_and_label(Role::Button, START_LABEL)
            .is_none()
    );
    harness.get_by_label_contains("Plattform:");
    harness.get_by_label_contains("mit einem lokalen Testserver geprüft");
    harness.get_by_label_contains("noch nicht geprüft");
    // deletion is off by default and the folder field only appears when it is switched on
    let delete =
        harness.get_by_role_and_label(Role::CheckBox, "Zusätzliche Dateien im Zielordner löschen");
    assert_eq!(
        delete.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::False)
    );
    assert!(
        harness
            .query_by_label_contains("Löschen nur in diesen Ordnern")
            .is_none()
    );
    // the status line is a polite live region
    let status = harness.get_by_label("Bereit. Repository und Zielordner wählen.");
    assert_eq!(status.accesskit_node().live(), Live::Polite);
}

#[test]
fn plan_rows_state_their_action_in_words_and_start_appears_only_after_the_plan() {
    let server = serve();
    let env = env_with(&format!("http://{}/repo", server.addr));
    let mut harness = harness_for(&env, platform_status_for("windows"));
    harness.run();
    plan_via_ui(&mut harness);
    assert_eq!(harness.state().screen.stage(), Stage::PlanReady);
    harness.get_by_label("3. Vorschau");
    harness.get_by_label("Neu laden: 2 Dateien (8.0 KiB)");
    harness.get_by_label("Ersetzen: 0 Dateien (0 B)");
    harness.get_by_label("Löschen: 0 Dateien");
    assert!(
        harness.get_all_by_label("Neu laden").count() >= 2,
        "each row says what it does"
    );
    harness.get_by_label(PBO);
    harness.get_by_label_contains("Hinweis: Komprimiert geführte Datei ohne nutzbare Archivgröße");
    harness.get_by_label_contains("Bleibt unverändert (2)");
    let start = harness.get_by_role_and_label(Role::Button, START_LABEL);
    assert!(!start.accesskit_node().is_disabled());
    let status = harness.get_by_label("Vorschau bereit: 2 neu, 0 ersetzen, 0 löschen.");
    assert_eq!(status.accesskit_node().live(), Live::Polite);
    // nothing was changed by looking at the plan
    assert_eq!(fs::read_dir(&env.dest).unwrap().count(), 0);
}

#[test]
fn deletions_are_listed_and_the_start_button_stays_disabled_until_they_are_confirmed() {
    let server = serve();
    let env = env_with(&format!("http://{}/repo", server.addr));
    fs::create_dir_all(env.dest.join("@example_core/addons")).unwrap();
    fs::write(env.dest.join("@example_core/addons/extra.pbo"), b"extra").unwrap();
    fs::write(env.dest.join(ZIP), b"local compressed-copy").unwrap();
    let mut harness = harness_for(&env, platform_status_for("windows"));
    harness.run();
    harness
        .get_by_role_and_label(Role::CheckBox, "Zusätzliche Dateien im Zielordner löschen")
        .click();
    harness.run();
    harness.get_by_label_contains("Löschen nur in diesen Ordnern");
    // without a folder the plan is refused with a message next to the field
    harness
        .get_by_role_and_label(Role::Button, PLAN_LABEL)
        .click();
    harness.run();
    harness.get_by_label_contains("Fehler: Zum Löschen mindestens einen Ordner angeben");
    harness
        .get_by_label("Löschen nur in diesen Ordnern (relativ zum Zielordner, einer pro Zeile):")
        .focus();
    harness.run();
    harness
        .get_by_label("Löschen nur in diesen Ordnern (relativ zum Zielordner, einer pro Zeile):")
        .type_text("@example_core");
    harness.run();
    plan_via_ui(&mut harness);
    harness.get_by_label("Löschungen bestätigen");
    harness.get_by_label_contains("Achtung: Diese 1 Dateien werden endgültig gelöscht");
    assert!(
        harness
            .get_all_by_label("@example_core/addons/extra.pbo")
            .count()
            >= 2,
        "the file is in the plan table and again in the confirmation list"
    );
    harness.get_by_label("Löschen");
    let start = harness.get_by_role_and_label(Role::Button, START_LABEL);
    assert!(
        start.accesskit_node().is_disabled(),
        "deleting needs the separate confirmation"
    );
    harness.get_by_label_contains("Nicht möglich: Bitte zuerst die Löschungen bestätigen");
    let confirm = harness.get_by_role_and_label(
        Role::CheckBox,
        "Ich habe die 1 Löschungen geprüft und möchte sie ausführen.",
    );
    assert_eq!(
        confirm.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::False),
        "unconfirmed by default"
    );
    confirm.click();
    harness.run();
    assert!(
        !harness
            .get_by_role_and_label(Role::Button, START_LABEL)
            .accesskit_node()
            .is_disabled()
    );
    harness
        .get_by_role_and_label(Role::Button, START_LABEL)
        .click();
    harness.run();
    step_until(&mut harness, "the run", |app| {
        app.screen.stage() == Stage::Finished
    });
    assert!(
        matches!(harness.state().screen.outcome(), Some(Outcome::Succeeded(r)) if r.deleted == 1)
    );
    harness.get_by_label("Abgeschlossen");
    assert!(harness.get_all_by_label_contains("1 gelöscht").count() >= 1);
    assert!(
        env.dest.join(ZIP).exists(),
        "protected and unsupported files stay"
    );
    assert!(!env.dest.join("@example_core/addons/extra.pbo").exists());
}

#[test]
fn an_unreleased_platform_shows_the_plan_but_the_start_button_is_disabled_with_a_reason() {
    let server = serve();
    let env = env_with(&format!("http://{}/repo", server.addr));
    let mut harness = harness_for(&env, platform_status_for("linux"));
    harness.run();
    harness.get_by_label_contains("Eingeschränkt:");
    harness.get_by_label_contains("Linux: die Synchronisation ist noch nicht freigegeben");
    harness.get_by_label_contains("Pfadgrenze");
    plan_via_ui(&mut harness);
    harness.get_by_label("Neu laden: 2 Dateien (8.0 KiB)");
    let start = harness.get_by_role_and_label(Role::Button, START_LABEL);
    assert!(start.accesskit_node().is_disabled());
    harness.get_by_label_contains(
        "Nicht möglich: Auf diesem System ist das Synchronisieren noch nicht freigegeben",
    );
    assert_eq!(fs::read_dir(&env.dest).unwrap().count(), 0);
}

#[test]
fn keyboard_only_flow_from_plan_to_result() {
    let server = serve();
    let env = env_with(&format!("http://{}/repo", server.addr));
    let mut harness = harness_for(&env, platform_status_for("windows"));
    harness.run();
    // Tab reaches the plan button; Enter activates it (no mouse involved)
    let mut reached = false;
    for _ in 0..12 {
        harness.key_press(Key::Tab);
        harness.run();
        if harness.root().children_recursive().any(|n| {
            n.accesskit_node().is_focused()
                && n.accesskit_node().label().as_deref() == Some(PLAN_LABEL)
        }) {
            reached = true;
            break;
        }
    }
    assert!(reached, "the plan button must be reachable with Tab");
    harness.key_press(Key::Enter);
    harness.run();
    step_until(&mut harness, "the plan", |app| !app.screen.is_busy());
    assert_eq!(harness.state().screen.stage(), Stage::PlanReady);
    // focus the start button via the keyboard path and press Enter
    harness
        .get_by_role_and_label(Role::Button, START_LABEL)
        .focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    step_until(&mut harness, "the run", |app| {
        app.screen.stage() == Stage::Finished
    });
    assert!(matches!(
        harness.state().screen.outcome(),
        Some(Outcome::Succeeded(_))
    ));
}

#[test]
fn a_running_transfer_exposes_a_progress_bar_with_text_and_a_cancel_button() {
    let server = serve();
    server.behavior.lock().unwrap().trickle_pbo = true;
    let env = env_with(&format!("http://{}/repo", server.addr));
    let mut harness = harness_for(&env, platform_status_for("windows"));
    harness.run();
    plan_via_ui(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, START_LABEL)
        .click();
    harness.run();
    step_until(&mut harness, "the transfer", |_| {
        server.requested("example_core.pbo") > 0
    });
    harness.run();
    harness.get_by_label("Synchronisation läuft");
    let bar = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().role() == Role::ProgressIndicator)
        .expect("a progress indicator must exist while transferring");
    let described = format!(
        "{:?} {:?}",
        bar.accesskit_node().label(),
        bar.accesskit_node().value()
    );
    assert!(
        described.contains("Dateien"),
        "the progress bar carries a text: {described}"
    );
    let cancel = harness.get_by_role_and_label(Role::Button, CANCEL_LABEL);
    assert!(!cancel.accesskit_node().is_disabled());
    cancel.click();
    harness.run();
    harness.get_by_label_contains("Abbruch angefordert");
    step_until(&mut harness, "the cancellation", |app| {
        app.screen.stage() == Stage::Finished
    });
    harness.get_by_label("Abgebrochen");
    harness.get_by_label_contains("vorhandene Dateien sind unverändert");
    let status = harness.get_by_label("Abgebrochen.");
    assert_eq!(status.accesskit_node().live(), Live::Polite);
}

#[test]
fn errors_stand_next_to_the_result_with_object_cause_and_next_step() {
    let server = serve();
    *server.behavior.lock().unwrap() = Behavior {
        corrupt_pbo: true,
        ..Behavior::default()
    };
    let env = env_with(&format!("http://{}/repo", server.addr));
    fs::create_dir_all(env.dest.join("@example_core/addons")).unwrap();
    fs::write(env.dest.join(PBO), b"previous").unwrap();
    let mut harness = harness_for(&env, platform_status_for("windows"));
    harness.run();
    plan_via_ui(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, START_LABEL)
        .click();
    harness.run();
    step_until(&mut harness, "the failure", |app| {
        app.screen.stage() == Stage::Finished
    });
    harness.get_by_label("Fehler bei „Example“:");
    assert!(
        harness
            .get_all_by_label_contains("verification failed")
            .count()
            >= 1
    );
    harness.get_by_label_contains("stimmte nicht mit der Prüfsumme des Servers überein");
    harness.get_by_label_contains("Fehler: ");
    assert_eq!(fs::read(env.dest.join(PBO)).unwrap(), b"previous");
    // back to the selection: the screen is usable again
    harness.get_by_label("Zurück zur Auswahl").click();
    harness.run();
    assert_eq!(harness.state().screen.stage(), Stage::Choose);
    harness.get_by_role_and_label(Role::Button, PLAN_LABEL);
}

#[test]
fn scaling_to_200_percent_keeps_every_action_reachable() {
    let server = serve();
    let env = env_with(&format!("http://{}/repo", server.addr));
    let mut harness = harness_for(&env, platform_status_for("windows"));
    harness.state_mut().settings.scale = 2.0;
    harness.run();
    harness.get_by_role_and_label(Role::Button, PLAN_LABEL);
    harness.get_by_role_and_label(Role::TextInput, "Zielordner (muss vorhanden sein):");
    plan_via_ui(&mut harness);
    harness.get_by_role_and_label(Role::Button, START_LABEL);
}
