// SPDX-License-Identifier: AGPL-3.0-only
//! Drawing code of the upload screen (MIG-042, step 4b). All behaviour lives in [`UploadScreen`]; this module maps
//! its state to widgets.
//!
//! Accessibility: every text field is tied to its visible label; the preview states each file's action and each
//! exclusion's reason in words (no colour-only status); the confirmation is an ordinary checkbox that the publish
//! button depends on, so no hidden dialog takes focus; the running state is a spinner with text and an enabled
//! cancel button, and the status line is a polite live region that changes with the stage. Errors stand next to
//! their result with the prefix "Fehler:". The labels are checked in `tests/upload_ui.rs`; screen readers have not
//! been run against this screen.

use eframe::egui::{self, RichText};

use crate::ui::error_color;
use crate::upload_screen::{UploadScreen, UploadStage};

/// Title of the status line.
pub const STATUS_LABEL: &str = "Status:";
/// Label of the button that checks the preview.
pub const PREVIEW_LABEL: &str = "Vorschau prüfen";
/// Label of the button that publishes the checked preview.
pub const PUBLISH_LABEL: &str = "Veröffentlichen";
/// Label of the cancel button shown while an operation runs.
pub const CANCEL_LABEL: &str = "Abbrechen";
/// Label of the confirmation checkbox.
pub const CONFIRM_LABEL: &str = "Ich habe die Vorschau geprüft und bestätige die Veröffentlichung.";

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(10.0);
    ui.heading(title);
}

/// Draws the upload view.
pub fn show_upload(screen: &mut UploadScreen, ui: &mut egui::Ui) {
    let error = error_color(ui.visuals().dark_mode);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.heading("Upload");
            ui.label(
                "Ein lokaler Ordner wird als Revision auf einen WebDAV-Ordner veröffentlicht. \
                 Zuerst die Vorschau prüfen, dann bestätigen.",
            );
            show_status_line(screen, ui);
            show_form(screen, ui);
            show_actions(screen, ui);
            show_preview(screen, ui, error);
            show_confirmation(screen, ui);
            show_result(screen, ui, error);
        });
}

fn show_status_line(screen: &UploadScreen, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.strong(STATUS_LABEL);
        let response = ui.label(screen.announcement());
        // A polite live region: the screen reader announces the text when the stage changes.
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_live(egui::accesskit::Live::Polite);
        });
    });
}

fn show_form(screen: &mut UploadScreen, ui: &mut egui::Ui) {
    let busy = screen.is_busy();
    ui.add_enabled_ui(!busy, |ui| {
        let form = &mut screen.form;
        let label = ui.label("Quellordner (lokal; wird nicht verändert):");
        ui.add(
            egui::TextEdit::singleline(&mut form.source)
                .hint_text("z. B. D:\\Arma 3\\Mods\\Meine Mod")
                .desired_width(520.0),
        )
        .labelled_by(label.id);

        let label = ui.label("Zieladresse (https://…, ohne Benutzername und Kennwort):");
        ui.add(
            egui::TextEdit::singleline(&mut form.target)
                .hint_text("https://repo.example/dav/mods")
                .desired_width(520.0),
        )
        .labelled_by(label.id);

        let label = ui.label("Benutzername (optional, nur über HTTPS):");
        ui.add(egui::TextEdit::singleline(&mut form.user).desired_width(320.0))
            .labelled_by(label.id);

        let label = ui.label("Kennwort (nur für diese Sitzung, wird nicht gespeichert):");
        ui.add(
            egui::TextEdit::singleline(&mut form.password)
                .password(true)
                .desired_width(320.0),
        )
        .labelled_by(label.id);

        let label =
            ui.label("Ausschlüsse (je Zeile ein Datei- oder Ordnerpfad, relativ zur Quelle):");
        ui.add(
            egui::TextEdit::multiline(&mut form.exclusions)
                .desired_rows(3)
                .desired_width(520.0),
        )
        .labelled_by(label.id);
        ui.label("Interne ArmaSync-Daten und Teildateien werden immer ausgeschlossen.");
    });
}

fn show_actions(screen: &mut UploadScreen, ui: &mut egui::Ui) {
    let busy = screen.is_busy();
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!busy, egui::Button::new(PREVIEW_LABEL))
            .clicked()
        {
            screen.request_preview();
        }
        if busy {
            ui.spinner();
            ui.label("Vorgang läuft … Abbruch ist jederzeit möglich.");
            if ui.button(CANCEL_LABEL).clicked() {
                screen.cancel();
            }
        }
    });
    if busy {
        show_progress(screen, ui);
    }
}

/// The progress of a running publication, in words and as a bar (ADR-0016). The words carry the information; the bar
/// only repeats it. The text is not a live region, so the screen reader does not announce every file.
fn show_progress(screen: &UploadScreen, ui: &mut egui::Ui) {
    let Some(progress) = screen.progress() else {
        ui.label("Die Veröffentlichung wird vorbereitet.");
        return;
    };
    let fraction = if progress.bytes_total == 0 {
        1.0
    } else {
        // The ratio only steers the bar; the exact figures are in the text below.
        (progress.bytes_done as f64 / progress.bytes_total as f64).clamp(0.0, 1.0) as f32
    };
    ui.add(
        egui::ProgressBar::new(fraction)
            .text(format!(
                "{} von {} übertragen",
                format_bytes(progress.bytes_done),
                format_bytes(progress.bytes_total)
            ))
            .desired_width(520.0),
    );
    ui.label(format!(
        "Dateien: {} von {} fertig.",
        progress.files_done, progress.files_total
    ));
    if let Some(current) = &progress.current {
        ui.label(format!("Gerade wird übertragen: {current}"));
    }
}

fn show_preview(screen: &mut UploadScreen, ui: &mut egui::Ui, error: egui::Color32) {
    let Some(preview) = screen.preview().cloned() else {
        return;
    };
    section(ui, "Vorschau");
    if !screen.preview_is_current() {
        ui.label(
            RichText::new("Die Eingaben haben sich geändert; diese Vorschau gilt nicht mehr.")
                .color(error),
        );
    }
    ui.label(format!(
        "Hochladen: {} Dateien ({}), {} Ordner anlegen, {} ersetzen.",
        preview.included.len(),
        format_bytes(preview.bytes),
        preview.folders,
        preview.replacements
    ));
    ui.label(RichText::new("Eingeschlossen:").strong());
    if preview.included.is_empty() {
        ui.label("Keine Datei wird hochgeladen.");
    }
    for row in &preview.included {
        let replace = if row.replace {
            ", ersetzt die vorhandene Datei"
        } else {
            ""
        };
        ui.label(format!(
            "Hochladen: {} ({}{replace})",
            row.path,
            format_bytes(row.size)
        ));
    }
    ui.label(RichText::new("Ausgeschlossen:").strong());
    if preview.excluded.is_empty() {
        ui.label("Keine Datei ist ausgeschlossen.");
    }
    for row in &preview.excluded {
        ui.label(format!(
            "Ausgeschlossen: {} ({}), Grund: {}",
            row.path,
            format_bytes(row.size),
            row.reason
        ));
    }
    for rule in &preview.unused_rules {
        ui.label(format!("Regel ohne Treffer: {rule}"));
    }
}

fn show_confirmation(screen: &mut UploadScreen, ui: &mut egui::Ui) {
    if screen.preview().is_none() {
        if let Some(text) = screen.publish_blocker() {
            ui.label(text);
        }
        return;
    }
    section(ui, "Veröffentlichen");
    ui.label(
        "Veröffentlichen lädt die aufgeführten Dateien auf das Ziel und setzt danach den Zeiger auf die neue \
         Revision. Die bisherige Veröffentlichung bleibt erhalten; sie wird nur bei Erfolg ersetzt.",
    );
    let busy = screen.is_busy();
    let mut confirmed = screen.confirmed();
    if ui
        .add_enabled(!busy, egui::Checkbox::new(&mut confirmed, CONFIRM_LABEL))
        .changed()
    {
        screen.set_confirmed(confirmed);
    }
    let blocker = screen.publish_blocker();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(blocker.is_none(), egui::Button::new(PUBLISH_LABEL))
            .clicked()
        {
            screen.request_publish();
        }
    });
    if let Some(text) = blocker {
        ui.label(text);
    }
}

fn show_result(screen: &mut UploadScreen, ui: &mut egui::Ui, error: egui::Color32) {
    match screen.stage() {
        UploadStage::Published => {
            let Some(published) = screen.published().cloned() else {
                return;
            };
            section(ui, "Ergebnis");
            ui.label(format!("Veröffentlicht: Revision {}.", published.revision));
            ui.label(format!(
                "Hochgeladene Dateien: {}.",
                published.files_uploaded
            ));
            if published.pointer_changed {
                ui.label("Der Zeiger zeigt jetzt auf diese Revision.");
            } else {
                ui.label("Der Zeiger zeigte bereits auf diese Revision.");
            }
            if published.revision_was_complete {
                ui.label("Die Revision war auf dem Ziel bereits vollständig.");
            }
        }
        UploadStage::Cancelled => {
            section(ui, "Ergebnis");
            ui.label("Abgebrochen. Die bisherige Veröffentlichung bleibt unverändert.");
        }
        UploadStage::Failed => {
            section(ui, "Ergebnis");
            if let Some(message) = screen.error() {
                ui.label(RichText::new(format!("Fehler: {message}")).color(error));
            }
            ui.label("Die Fehlermeldung sagt, ob die bisherige Veröffentlichung betroffen ist.");
        }
        _ => {}
    }
}

fn format_bytes(bytes: u64) -> String {
    crate::sync_screen::format_bytes(bytes)
}
