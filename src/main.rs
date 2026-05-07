#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod fonts;
mod html;
mod model;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Rapid",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_fonts(fonts::app_font_definitions());
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(8, 10, 16);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(16, 19, 28);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(24, 28, 40);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(28, 34, 48);
            visuals.extreme_bg_color = egui::Color32::from_rgb(10, 12, 18);
            visuals.faint_bg_color = egui::Color32::from_rgb(14, 17, 24);
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(app::EditorApp::default()))
        }),
    )
}
