// SPDX-License-Identifier: AGPL-3.0-only
//! `armasync-gui` binary: opens the desktop prototype window.

use armasync_gui::ui::ArmaSyncApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui_viewport(),
        ..Default::default()
    };
    eframe::run_native(
        "ArmaSync – Lynxnode.it",
        options,
        Box::new(|cc| Ok(Box::new(ArmaSyncApp::new(&cc.egui_ctx)))),
    )
}

fn egui_viewport() -> eframe::egui::ViewportBuilder {
    eframe::egui::ViewportBuilder::default()
        .with_inner_size([1024.0, 720.0])
        .with_min_inner_size([640.0, 480.0])
        .with_title("ArmaSync – Lynxnode.it")
}
