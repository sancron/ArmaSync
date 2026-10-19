// SPDX-License-Identifier: AGPL-3.0-only
//! Drawing code (egui). All behaviour lives in [`crate::model::Model`]; this module only maps state to widgets.
//!
//! Accessibility notes: standard egui widgets are used wherever possible so that AccessKit exposes role, name,
//! value and state; text edits are tied to their visible labels; errors are shown as text next to the field (with
//! a "Fehler:" prefix, not colour alone); status is text (the default fonts lack many symbols, so none are used); focus returns to the opener when the dialog
//! closes; progress is a real progress bar with a text value.

use eframe::egui::{self, Align, Color32, Id, Key, Layout, Modifiers, RichText, ThemePreference};
use egui_extras::{Column, TableBuilder};

use armasync_core::{JobState, Phase};

use crate::model::{Model, SortDirection, SortKey, synthetic_repositories};
use crate::sync_screen::SyncScreen;
use crate::sync_ui::show_sync;

/// Colour theme choice persisted by the user (not yet persisted to disk in the prototype).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChoice {
    /// Follow the operating system.
    System,
    /// Light theme.
    Light,
    /// Dark theme.
    Dark,
}

impl ThemeChoice {
    const fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Hell",
            Self::Dark => "Dunkel",
        }
    }

    const fn preference(self) -> ThemePreference {
        match self {
            Self::System => ThemePreference::System,
            Self::Light => ThemePreference::Light,
            Self::Dark => ThemePreference::Dark,
        }
    }
}

/// View settings that are not part of the domain model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewSettings {
    /// Theme.
    pub theme: ThemeChoice,
    /// UI scale (1.0 = 100 %).
    pub scale: f32,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::System,
            scale: 1.0,
        }
    }
}

/// Colour for error text that reaches at least 4.5:1 on the default panel colours of both themes.
#[must_use]
pub fn error_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0xFF, 0x8A, 0x80)
    } else {
        Color32::from_rgb(0xB0, 0x1E, 0x1E)
    }
}

/// Visuals of the product themes: egui defaults adjusted for contrast.
///
/// * secondary ("weak") text at least 4.5:1,
/// * widget outlines (also used as the focus outline) at least 3:1 against the panel,
/// * selection: the fill at least 3:1 against the panel, text on the selection fill at least 4.5:1 and the selection
///   outline (used by egui as the focus ring of text fields) at least 3:1. egui draws text in selected rows and the
///   outline with the same colour (`selection.stroke`), so the light theme uses a near-black one on a mid blue fill:
///   the narrow band that satisfies all three limits at once. A selected list row is still also announced as
///   `selected` and set in bold type, so the state is not conveyed by colour alone.
#[must_use]
pub fn themed_visuals(dark: bool) -> egui::Visuals {
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let (weak, outline, focus, fill, ring) = if dark {
        (
            Color32::from_rgb(0xB4, 0xB4, 0xB4),
            Color32::from_rgb(0x9A, 0x9A, 0x9A),
            Color32::from_rgb(0xFF, 0xFF, 0xFF),
            Color32::from_rgb(0x1E, 0x78, 0xBE),
            Color32::from_rgb(0xFF, 0xFF, 0xFF),
        )
    } else {
        (
            Color32::from_rgb(0x5A, 0x5A, 0x5A),
            Color32::from_rgb(0x6E, 0x6E, 0x6E),
            Color32::from_rgb(0x00, 0x2D, 0x64),
            Color32::from_rgb(0x5A, 0x8F, 0xCC),
            Color32::from_rgb(0x10, 0x10, 0x10),
        )
    };
    visuals.weak_text_color = Some(weak);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, outline);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, outline);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(2.0, focus);
    visuals.selection.bg_fill = fill;
    visuals.selection.stroke = egui::Stroke::new(2.0, ring);
    visuals
}

const ADD_BUTTON_LABEL: &str = "Repository hinzufügen…";
const ADD_DIALOG_TITLE: &str = "Repository hinzufügen";
const ROW_HEIGHT: f32 = 26.0;

/// Which view is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    /// The synchronisation of a real repository (MIG-031).
    Sync,
    /// The list prototype with simulated data (MIG-012).
    Prototype,
}

/// Application state for eframe.
#[derive(Debug)]
pub struct ArmaSyncApp {
    model: Model,
    screen: SyncScreen,
    view: AppView,
    settings: ViewSettings,
    applied: Option<ViewSettings>,
    focus_add_button: bool,
}

impl ArmaSyncApp {
    /// Creates the app: the synchronisation view on the user's library, and the simulated list prototype
    /// (10,000 synthetic repositories, ADR-0002) as a second view.
    #[must_use]
    pub fn new(ctx: &egui::Context) -> Self {
        let repaint = ctx.clone();
        let model = Model::new(synthetic_repositories(10_000), move || {
            repaint.request_repaint()
        });
        let repaint = ctx.clone();
        let screen = SyncScreen::new(None, armasync_workflow::platform_status(), move || {
            repaint.request_repaint()
        });
        Self {
            model,
            screen,
            view: AppView::Sync,
            settings: ViewSettings::default(),
            applied: None,
            focus_add_button: false,
        }
    }
}

impl eframe::App for ArmaSyncApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.model.poll();
        self.screen.poll();
        apply_view_settings(ui.ctx(), &self.settings, &mut self.applied);
        egui::Panel::top("navigation").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Ansicht:");
                ui.selectable_value(&mut self.view, AppView::Sync, "Synchronisation");
                ui.selectable_value(
                    &mut self.view,
                    AppView::Prototype,
                    "Repository-Liste (Prototyp, simuliert)",
                );
            });
        });
        match self.view {
            AppView::Sync => show_sync(&mut self.screen, &mut self.settings, ui),
            AppView::Prototype => show(
                &mut self.model,
                &mut self.settings,
                &mut self.focus_add_button,
                ui,
            ),
        }
        if self.model.is_busy() || self.screen.is_busy() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

/// Applies theme and scale when they changed.
pub fn apply_view_settings(
    ctx: &egui::Context,
    settings: &ViewSettings,
    applied: &mut Option<ViewSettings>,
) {
    if applied.as_ref() != Some(settings) {
        ctx.set_visuals_of(egui::Theme::Light, themed_visuals(false));
        ctx.set_visuals_of(egui::Theme::Dark, themed_visuals(true));
        ctx.set_theme(settings.theme.preference());
        ctx.set_zoom_factor(settings.scale);
        *applied = Some(*settings);
    }
}

fn state_text(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "Wartet",
        JobState::Running => "Läuft",
        JobState::Succeeded => "Abgeschlossen",
        JobState::Failed => "Fehlgeschlagen",
        JobState::Cancelled => "Abgebrochen",
    }
}

fn phase_text(phase: Phase) -> &'static str {
    match phase {
        Phase::Planning => "Änderungen werden geprüft",
        Phase::Transferring => "Dateien werden übertragen und geprüft",
        Phase::Cleaning => "Überzählige Dateien werden entfernt",
        Phase::Finished => "Beendet",
    }
}

/// The theme and scale selectors (shared by every view's toolbar).
pub fn settings_controls(ui: &mut egui::Ui, settings: &mut ViewSettings) {
    let label = ui.label("Darstellung:");
    egui::ComboBox::from_id_salt("theme")
        .selected_text(settings.theme.label())
        .show_ui(ui, |ui| {
            for choice in [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark] {
                ui.selectable_value(&mut settings.theme, choice, choice.label());
            }
        })
        .response
        .labelled_by(label.id);
    let label = ui.label("Skalierung:");
    egui::ComboBox::from_id_salt("scale")
        .selected_text(format!("{} %", (settings.scale * 100.0).round()))
        .show_ui(ui, |ui| {
            for scale in [1.0_f32, 1.25, 1.5, 2.0] {
                ui.selectable_value(
                    &mut settings.scale,
                    scale,
                    format!("{} %", (scale * 100.0).round()),
                );
            }
        })
        .response
        .labelled_by(label.id);
}

/// Draws the whole window content.
pub fn show(
    model: &mut Model,
    settings: &mut ViewSettings,
    focus_add_button: &mut bool,
    ui: &mut egui::Ui,
) {
    let dark = ui.visuals().dark_mode;
    let dialog_open = model.dialog.is_some();
    let busy = model.is_busy();

    egui::Panel::top("toolbar").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.heading("ArmaSync");
            ui.label("von Lynxnode.it");
            ui.separator();
            let add = ui.add_enabled(!dialog_open, egui::Button::new(ADD_BUTTON_LABEL));
            if *focus_add_button {
                add.request_focus();
                *focus_add_button = false;
            }
            if add.clicked() {
                model.open_dialog();
            }
            let can_sync = model.selected().is_some() && !busy && !dialog_open;
            if ui.add_enabled(can_sync, egui::Button::new("Synchronisieren")).clicked() {
                model.start_sync();
            }
            if ui.add_enabled(busy, egui::Button::new("Abbrechen")).clicked() {
                model.cancel_sync();
            }
            ui.separator();
            settings_controls(ui, settings);
        });
        ui.label("Prototyp: Alle Repositories und Übertragungen sind simuliert (Speicherdaten); es werden keine echten Server oder Dateien verwendet.");
    });

    egui::Panel::bottom("job").show(ui, |ui| show_job(model, ui, dark));

    egui::CentralPanel::default().show(ui, |ui| {
        show_list(model, ui, !dialog_open);
    });

    if model.dialog.is_some() {
        show_dialog(model, ui.ctx(), focus_add_button, dark);
    }
}

fn show_list(model: &mut Model, ui: &mut egui::Ui, interactive: bool) {
    ui.horizontal(|ui| {
        let label = ui.label("Filter:");
        let mut filter = model.filter().to_owned();
        let edit = ui
            .add(
                egui::TextEdit::singleline(&mut filter)
                    .hint_text("Name oder Adresse")
                    .desired_width(320.0),
            )
            .labelled_by(label.id);
        if edit.changed() {
            model.set_filter(&filter);
        }
        ui.label(format!(
            "{} von {} Repositories",
            model.visible_len(),
            model.repositories().len()
        ));
    });

    ui.horizontal(|ui| {
        ui.label("Sortieren nach:");
        for (key, label) in [
            (SortKey::Name, "Name"),
            (SortKey::Status, "Status"),
            (SortKey::Files, "Dateien"),
        ] {
            let active = model.sort().0 == key;
            let arrow = match (active, model.sort().1) {
                (true, SortDirection::Ascending) => " (aufsteigend)",
                (true, SortDirection::Descending) => " (absteigend)",
                (false, _) => "",
            };
            if ui
                .selectable_label(active, format!("{label}{arrow}"))
                .clicked()
            {
                model.sort_by(key);
            }
        }
    });

    if interactive && !ui.ctx().egui_wants_keyboard_input() {
        let (down, up, page_down, page_up) = ui.input_mut(|input| {
            (
                input.consume_key(Modifiers::NONE, Key::ArrowDown),
                input.consume_key(Modifiers::NONE, Key::ArrowUp),
                input.consume_key(Modifiers::NONE, Key::PageDown),
                input.consume_key(Modifiers::NONE, Key::PageUp),
            )
        });
        if down {
            model.move_selection(1);
        }
        if up {
            model.move_selection(-1);
        }
        if page_down {
            model.move_selection(10);
        }
        if page_up {
            model.move_selection(-10);
        }
    }

    let selected_row = model.selected_row();
    let visible_total = model.visible_len();
    let mut clicked = None;
    let mut table = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(Column::remainder().at_least(160.0).clip(true))
        .column(Column::initial(200.0).at_least(120.0).clip(true))
        .column(Column::initial(80.0).at_least(60.0))
        .min_scrolled_height(120.0);
    if let Some(row) = selected_row {
        table = table.scroll_to_row(row, None);
    }
    table
        .header(ROW_HEIGHT, |mut header| {
            for title in ["Repository", "Status", "Dateien"] {
                header.col(|ui| {
                    ui.strong(title);
                });
            }
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, model.visible_len(), |mut row| {
                let index = row.index();
                let Some(repo) = model.visible_row(index) else {
                    return;
                };
                let is_selected = selected_row == Some(index);
                row.set_selected(is_selected);
                row.col(|ui| {
                    let response = ui
                        .selectable_label(is_selected, &repo.name)
                        .on_hover_text(format!("{}\n{}", repo.name, repo.url));
                    if response.clicked() && interactive {
                        clicked = Some(repo.id.clone());
                    }
                    // egui reports a selectable label as a pressed toggle button. A list row is announced better as
                    // a selectable list item with its position ("3 von 10000"), which also matters for the
                    // virtualised list where most rows do not exist as widgets.
                    let (position, total) = (index + 1, visible_total);
                    ui.ctx().accesskit_node_builder(response.id, |node| {
                        node.clear_toggled();
                        node.set_role(egui::accesskit::Role::ListBoxOption);
                        node.set_selected(is_selected);
                        node.set_position_in_set(position);
                        node.set_size_of_set(total);
                    });
                });
                row.col(|ui| {
                    ui.label(repo.status.label().to_owned());
                });
                row.col(|ui| {
                    ui.label(repo.files.to_string());
                });
            });
        });
    if let Some(id) = clicked {
        model.select(&id);
    }
}

fn show_job(model: &mut Model, ui: &mut egui::Ui, dark: bool) {
    let mut clear_errors = false;
    if let Some(view) = model.job_view() {
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!("Synchronisation: {}", view.repository));
            ui.label(format!("– {}", state_text(view.state)));
            if view.cancel_requested && !view.state.is_terminal() {
                ui.label("(Abbruch angefordert)");
            }
        });
        let total = view.progress.files_total;
        let fraction = if total == 0 {
            if view.state == JobState::Succeeded {
                1.0
            } else {
                0.0
            }
        } else {
            view.progress.files_done as f32 / total as f32
        };
        let summary = if total == 0 && view.state == JobState::Succeeded {
            "Bereits aktuell – nichts zu übertragen".to_owned()
        } else {
            format!(
                "{} von {} Dateien, {}",
                view.progress.files_done,
                total,
                if view.state.is_terminal() {
                    state_text(view.state)
                } else {
                    phase_text(view.phase)
                }
            )
        };
        ui.add(
            egui::ProgressBar::new(fraction)
                .text(summary)
                .animate(false),
        );
    } else {
        ui.label("Keine Synchronisation aktiv. Repository auswählen und „Synchronisieren“ wählen.");
    }

    if !model.errors().is_empty() {
        egui::CollapsingHeader::new(
            RichText::new(format!("Fehler ({})", model.errors().len())).strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            for item in model.errors() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("Fehler in „{}“:", item.repository))
                            .color(error_color(dark))
                            .strong(),
                    );
                    ui.label(&item.message);
                    ui.label(item.hint);
                });
            }
            if ui.button("Fehlerliste leeren").clicked() {
                clear_errors = true;
            }
        });
    }
    if clear_errors {
        model.clear_errors();
    }
}

fn show_dialog(model: &mut Model, ctx: &egui::Context, focus_add_button: &mut bool, dark: bool) {
    let mut submit = false;
    let mut cancel = false;
    let errors = model.dialog_errors.clone();
    let first_frame = !ctx.data(|data| {
        data.get_temp::<bool>(Id::unique("add-dialog-open"))
            .unwrap_or(false)
    });
    ctx.data_mut(|data| data.insert_temp(Id::unique("add-dialog-open"), true));

    let dialog_id = Id::unique("add-repository");
    let modal = egui::Modal::new(dialog_id)
        .area(egui::Modal::default_area(dialog_id).accessible_name(ADD_DIALOG_TITLE))
        .show(ctx, |ui| {
            ui.set_width(460.0);
            ui.heading(ADD_DIALOG_TITLE);
            let Some(dialog) = model.dialog.as_mut() else {
                return;
            };
            egui::Grid::new("add-form")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    let label = ui.label("Name:");
                    let name = ui
                        .add(egui::TextEdit::singleline(&mut dialog.name).desired_width(300.0))
                        .labelled_by(label.id);
                    if first_frame {
                        name.request_focus();
                    }
                    ui.end_row();
                    if let Some(message) = errors.name {
                        ui.label("");
                        ui.label(
                            RichText::new(format!("Fehler: {message}")).color(error_color(dark)),
                        );
                        ui.end_row();
                    }

                    let label = ui.label("Adresse:");
                    ui.add(
                        egui::TextEdit::singleline(&mut dialog.url)
                            .hint_text("https://…")
                            .desired_width(300.0),
                    )
                    .labelled_by(label.id);
                    ui.end_row();
                    if let Some(message) = errors.url {
                        ui.label("");
                        ui.label(
                            RichText::new(format!("Fehler: {message}")).color(error_color(dark)),
                        );
                        ui.end_row();
                    }

                    let label = ui.label("Benutzername (optional):");
                    ui.add(egui::TextEdit::singleline(&mut dialog.username).desired_width(300.0))
                        .labelled_by(label.id);
                    ui.end_row();

                    let label = ui.label("Kennwort (optional):");
                    ui.add(
                        egui::TextEdit::singleline(dialog.password.buffer_mut())
                            .password(true)
                            .desired_width(300.0),
                    )
                    .labelled_by(label.id);
                    ui.end_row();
                });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Hinzufügen").clicked() {
                    submit = true;
                }
                if ui.button("Abbrechen").clicked() {
                    cancel = true;
                }
            });
        });
    if modal.should_close() {
        cancel = true;
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter)) {
        submit = true;
    }
    if submit {
        model.submit_dialog();
    }
    if cancel {
        model.cancel_dialog();
    }
    if model.dialog.is_none() {
        ctx.data_mut(|data| data.remove_temp::<bool>(Id::unique("add-dialog-open")));
        *focus_add_button = true;
    }
}
