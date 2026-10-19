// SPDX-License-Identifier: AGPL-3.0-only
//! Drawing code of the synchronisation screen (MIG-031). All behaviour lives in [`SyncScreen`]; this module maps
//! state to widgets.
//!
//! Accessibility: standard widgets throughout; every text field and selector is tied to its visible label; errors
//! stand next to their field with the prefix "Fehler:"; every plan row states its action in words (no colour-only
//! status); deletions are listed and need a separate, labelled confirmation; the progress bar carries a text; one
//! status line is a polite live region and changes only when the stage changes, not per byte. The labels are part
//! of the accessibility tree checked in `tests/sync_ui.rs`; screen readers themselves have **not** been run
//! against this screen (see `docs/migration/GUI_SYNC_EVIDENCE.md`).

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};

use armasync_core::JobState;

use crate::model::JobView;
use crate::sync_screen::{Outcome, Stage, SyncScreen, format_bytes};
use crate::ui::{ViewSettings, error_color, settings_controls};

/// Title of the status region.
pub const STATUS_LABEL: &str = "Status:";
/// Label of the main action that starts the synchronisation.
pub const START_LABEL: &str = "Synchronisieren";
/// Label of the action that creates the plan.
pub const PLAN_LABEL: &str = "Änderungen prüfen";
/// Label of the cancel button.
pub const CANCEL_LABEL: &str = "Abbrechen";

fn phase_text(job: &JobView) -> &'static str {
    use armasync_core::Phase;
    match job.phase {
        Phase::Planning => "Änderungen werden geprüft",
        Phase::Transferring => "Dateien werden übertragen und geprüft",
        Phase::Cleaning => "Überzählige Dateien werden entfernt",
        Phase::Finished => "Beendet",
    }
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(10.0);
    ui.heading(title);
}

/// Draws the whole synchronisation view.
pub fn show_sync(screen: &mut SyncScreen, settings: &mut ViewSettings, ui: &mut egui::Ui) {
    let dark = ui.visuals().dark_mode;
    egui::Panel::top("sync-toolbar").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.heading("ArmaSync");
            ui.label("von Lynxnode.it");
            ui.separator();
            settings_controls(ui, settings);
        });
        let platform = screen.platform();
        let prefix = if platform.run_allowed() {
            "Plattform:"
        } else {
            "Eingeschränkt:"
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(prefix).strong());
            ui.label(platform.message());
        });
    });

    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| show_body(screen, ui, dark));
    });
}

fn show_status_line(screen: &SyncScreen, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.strong(STATUS_LABEL);
        let response = ui.label(screen.announcement());
        // A polite live region: the screen reader announces the text when the stage changes.
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_live(egui::accesskit::Live::Polite);
        });
    });
}

fn show_body(screen: &mut SyncScreen, ui: &mut egui::Ui, dark: bool) {
    let error = error_color(dark);
    let busy = screen.is_busy();
    show_status_line(screen, ui);

    section(ui, "1. Repository");
    show_repository(screen, ui, busy, error);

    section(ui, "2. Zielordner und Optionen");
    show_destination(screen, ui, busy, error);

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let can_plan = !busy && screen.stage() != Stage::Running;
        if ui
            .add_enabled(can_plan, egui::Button::new(PLAN_LABEL))
            .clicked()
        {
            screen.request_plan();
        }
        if screen.stage() == Stage::Planning {
            ui.label("Änderungen werden geprüft … (es wird nichts verändert)");
            if ui.button(CANCEL_LABEL).clicked() {
                screen.cancel();
            }
        }
    });

    if screen.plan().is_some() {
        section(ui, "3. Vorschau");
        show_plan(screen, ui, error);
    }

    match screen.stage() {
        Stage::Running => {
            section(ui, "Synchronisation läuft");
            show_running(screen, ui);
        }
        Stage::Finished => {
            section(ui, "Ergebnis");
            show_outcome(screen, ui, error);
        }
        _ => {}
    }
}

fn show_repository(screen: &mut SyncScreen, ui: &mut egui::Ui, busy: bool, error: egui::Color32) {
    if let Some(message) = screen.load_error() {
        ui.label(RichText::new(format!("Fehler: {message}")).color(error));
        return;
    }
    if screen.repositories().is_empty() {
        ui.label(
            "Die Bibliothek enthält noch kein Repository mit http- oder https-Adresse. Fügen Sie eines in der Kommandozeile hinzu: „armasync repository import <Datei>“ oder „armasync import <Datei>“. Danach „Liste neu laden“ wählen.",
        );
        if ui.button("Liste neu laden").clicked() {
            screen.reload();
        }
        return;
    }
    ui.add_enabled_ui(!busy, |ui| {
        ui.horizontal(|ui| {
            let label = ui.label("Repository:");
            let selected_text = screen
                .selected()
                .map_or_else(|| "Bitte auswählen".to_owned(), |r| r.name.clone());
            let mut chosen = None;
            egui::ComboBox::from_id_salt("sync-repository")
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    for (index, repository) in screen.repositories().iter().enumerate() {
                        let active = screen.selected().map(|r| &r.id) == Some(&repository.id);
                        if ui.selectable_label(active, &repository.name).clicked() {
                            chosen = Some(index);
                        }
                    }
                })
                .response
                .labelled_by(label.id);
            if let Some(index) = chosen {
                screen.select(index);
            }
            if ui.button("Liste neu laden").clicked() {
                screen.reload();
            }
        });
    });
    if let Some(message) = &screen.field_errors.repository {
        ui.label(RichText::new(format!("Fehler: {message}")).color(error));
    }
    if let Some(repository) = screen.selected() {
        ui.horizontal_wrapped(|ui| {
            ui.label("Adresse:");
            ui.label(&repository.url);
        });
        if repository.needs_credentials {
            ui.label("Hinweis: Dieses Repository ist als anmeldepflichtig markiert. Die Anmeldung wird noch nicht unterstützt; es werden keine Zugangsdaten gesendet.");
        }
    }
}

fn show_destination(screen: &mut SyncScreen, ui: &mut egui::Ui, busy: bool, error: egui::Color32) {
    ui.add_enabled_ui(!busy, |ui| {
        let label = ui.label("Zielordner (muss vorhanden sein):");
        ui.add(
            egui::TextEdit::singleline(&mut screen.destination)
                .hint_text("z. B. D:\\Arma 3\\Mods")
                .desired_width(520.0),
        )
        .labelled_by(label.id);
        if let Some(message) = screen.field_errors.destination.clone() {
            ui.label(RichText::new(format!("Fehler: {message}")).color(error));
        }
        ui.add_space(4.0);
        ui.checkbox(
            &mut screen.delete_extra,
            "Zusätzliche Dateien im Zielordner löschen",
        );
        ui.label("Standardmäßig wird nichts gelöscht. Wenn Sie das Löschen einschalten, geben Sie genau an, wo, und bestätigen es in der Vorschau.");
        if screen.delete_extra {
            let label = ui.label(
                "Löschen nur in diesen Ordnern (relativ zum Zielordner, einer pro Zeile):",
            );
            ui.add(
                egui::TextEdit::multiline(&mut screen.managed_roots)
                    .desired_rows(3)
                    .desired_width(520.0),
            )
            .labelled_by(label.id);
            if let Some(message) = screen.field_errors.roots.clone() {
                ui.label(RichText::new(format!("Fehler: {message}")).color(error));
            }
            ui.label("Vom Repository geschützte Ordner und komprimiert geführte Dateien werden nie gelöscht.");
        }
    });
}

fn show_plan(screen: &mut SyncScreen, ui: &mut egui::Ui, error: egui::Color32) {
    let Some(plan) = screen.plan().cloned() else {
        return;
    };
    ui.label(format!("Stand des Repositorys: Revision {}", plan.revision));
    let t = plan.totals;
    ui.label(format!(
        "Neu laden: {} Dateien ({})",
        t.downloads,
        format_bytes(t.download_bytes)
    ));
    ui.label(format!(
        "Ersetzen: {} Dateien ({})",
        t.replaces,
        format_bytes(t.replace_bytes)
    ));
    ui.label(format!("Bereits aktuell: {} Dateien", t.unchanged));
    ui.label(format!("Löschen: {} Dateien", t.deletes));
    if t.extras_kept > 0 {
        ui.label(format!(
            "Zusätzliche Dateien, die bleiben: {}",
            t.extras_kept
        ));
    }
    if plan.rows.is_empty() {
        ui.label("Alles ist bereits aktuell. Es ist nichts zu übertragen.");
    } else {
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::initial(110.0).at_least(80.0))
            .column(Column::remainder().at_least(200.0).clip(true))
            .column(Column::initial(100.0).at_least(70.0))
            .max_scroll_height(240.0)
            .header(24.0, |mut header| {
                for title in ["Aktion", "Datei", "Größe"] {
                    header.col(|ui| {
                        ui.strong(title);
                    });
                }
            })
            .body(|body| {
                body.rows(24.0, plan.rows.len(), |mut row| {
                    let item = &plan.rows[row.index()];
                    row.col(|ui| {
                        ui.label(item.action.label());
                    });
                    row.col(|ui| {
                        ui.label(&item.path);
                    });
                    row.col(|ui| {
                        ui.label(item.size.map_or(String::new(), format_bytes));
                    });
                });
            });
    }
    for note in &plan.notes {
        ui.label(format!("Hinweis: {note}"));
    }
    if plan.needs_credentials {
        ui.label("Hinweis: Anmeldung wird noch nicht unterstützt; es wurden keine Zugangsdaten gesendet.");
    }
    if !plan.protected.is_empty() || !plan.unsupported.is_empty() {
        egui::CollapsingHeader::new(format!(
            "Bleibt unverändert ({})",
            plan.protected.len() + plan.unsupported.len()
        ))
        .default_open(false)
        .show(ui, |ui| {
            for path in &plan.protected {
                ui.label(format!("Vom Repository geschützt: {path}"));
            }
            for path in &plan.unsupported {
                ui.label(format!(
                    "Komprimiert geführt, noch nicht unterstützt (wird nie gelöscht): {path}"
                ));
            }
        });
    }

    if !plan.deletes.is_empty() {
        section(ui, "Löschungen bestätigen");
        ui.label(
            RichText::new(format!(
                "Achtung: Diese {} Dateien werden endgültig gelöscht:",
                plan.deletes.len()
            ))
            .strong()
            .color(error),
        );
        egui::ScrollArea::vertical()
            .id_salt("delete-list")
            .max_height(150.0)
            .show(ui, |ui| {
                for path in &plan.deletes {
                    ui.label(path);
                }
            });
        let text = format!(
            "Ich habe die {} Löschungen geprüft und möchte sie ausführen.",
            plan.deletes.len()
        );
        ui.checkbox(&mut screen.confirm_deletes, text);
    }

    if let Some(notice) = screen.notice() {
        ui.label(RichText::new(format!("Hinweis: {notice}")).strong());
    }
    ui.add_space(6.0);
    let blocker = screen.start_blocker();
    ui.horizontal(|ui| {
        let start = ui.add_enabled(blocker.is_ok(), egui::Button::new(START_LABEL));
        if start.clicked() {
            screen.start();
        }
        if let Err(reason) = blocker
            && screen.stage() == Stage::PlanReady
        {
            ui.label(format!("Nicht möglich: {reason}"));
        }
    });
}

fn show_running(screen: &mut SyncScreen, ui: &mut egui::Ui) {
    let mut cancel = false;
    if let Some(job) = screen.job() {
        let total_bytes = job.progress.bytes_total;
        let fraction = if total_bytes == 0 {
            if job.progress.files_total == 0 {
                0.0
            } else {
                job.progress.files_done as f32 / job.progress.files_total as f32
            }
        } else {
            job.progress.bytes_done as f32 / total_bytes as f32
        };
        let text = format!(
            "{} von {} Dateien, {} von {} – {}",
            job.progress.files_done,
            job.progress.files_total,
            format_bytes(job.progress.bytes_done),
            format_bytes(total_bytes),
            phase_text(job)
        );
        ui.add(
            egui::ProgressBar::new(fraction.clamp(0.0, 1.0))
                .text(text)
                .animate(false),
        );
        if job.cancel_requested && !job.state.is_terminal() {
            ui.label("Abbruch angefordert: Die laufende Datei wird verworfen, vorhandene Dateien bleiben unverändert.");
        }
        if ui
            .add_enabled(!job.cancel_requested, egui::Button::new(CANCEL_LABEL))
            .clicked()
        {
            cancel = true;
        }
    }
    if cancel {
        screen.cancel();
    }
}

fn show_outcome(screen: &mut SyncScreen, ui: &mut egui::Ui, error: egui::Color32) {
    match screen.outcome().cloned() {
        Some(Outcome::Succeeded(report)) => {
            ui.label(RichText::new("Abgeschlossen").strong());
            ui.label(format!(
                "{} neu geladen, {} ersetzt, {} gelöscht, {} unverändert; {} übertragen.",
                report.downloaded,
                report.replaced,
                report.deleted,
                report.kept,
                format_bytes(report.bytes)
            ));
            ui.label("Jede Datei wurde anhand der Prüfsumme des Servers geprüft und erst danach übernommen. Vom Repository geschützte und komprimiert geführte Dateien blieben unverändert.");
        }
        Some(Outcome::Cancelled) => {
            ui.label(RichText::new("Abgebrochen").strong());
            ui.label("Bereits fertig geprüfte Dateien sind vollständig übernommen. Die unterbrochene Datei wurde nicht verändert; vorhandene Dateien sind unverändert.");
        }
        Some(Outcome::Failed(item)) => {
            ui.label(
                RichText::new(format!("Fehler bei „{}“:", item.repository))
                    .strong()
                    .color(error),
            );
            ui.label(&item.message);
            ui.label(item.hint);
        }
        None => {}
    }
    if screen.job().is_some_and(|j| j.state == JobState::Running) {
        return;
    }
    ui.add_space(6.0);
    if ui.button("Zurück zur Auswahl").clicked() {
        screen.reset();
    }
}
