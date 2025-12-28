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
            .with_icon(eframe::icon_data::from_png_bytes(&[]).unwrap_or_default()), // Add icon here later
        ..Default::default()
    };
    
    eframe::run_native(
        "Universal Editor",
        options,
        Box::new(|cc| Box::new(UniversalEditor::new(cc))),
    )
}
