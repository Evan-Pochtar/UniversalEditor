#![windows_subsystem = "windows"]

mod app;
mod modules;
mod style;

use app::UniversalEditor;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Universal Editor")
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("img/logo.png")).unwrap_or_default()),
        ..Default::default()
    };
    
    eframe::run_native(
        "Universal Editor",
        options,
        Box::new(|cc| {
            cc.egui_ctx.style_mut(|s| s.visuals.text_cursor.blink = false);
            Ok(Box::new(UniversalEditor::new(cc)))
        }),
    )
}
