use eframe::egui;
use crate::style::{ColorPalette, ThemeMode};

pub fn panel_colors(theme: ThemeMode) -> (egui::Color32, egui::Color32, egui::Color32) {
    if matches!(theme, ThemeMode::Dark) {
        (ColorPalette::ZINC_800, ColorPalette::ZINC_700, ColorPalette::ZINC_200)
    } else {
        (ColorPalette::GRAY_50, ColorPalette::GRAY_300, ColorPalette::GRAY_800)
    }
}

pub fn format_btn_colors(selected: bool, accent: egui::Color32, theme: ThemeMode) -> (egui::Color32, egui::Color32) {
    if selected { (accent, egui::Color32::WHITE) }
    else if matches!(theme, ThemeMode::Dark) { (ColorPalette::ZINC_700, ColorPalette::ZINC_300) }
    else { (ColorPalette::GRAY_200, ColorPalette::GRAY_800) }
}

pub fn label_col(theme: ThemeMode) -> egui::Color32 {
    if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 }
}

pub fn drop_zone_colors(hover: bool, theme: ThemeMode) -> (egui::Color32, egui::Color32) {
    let bg = if hover {
        if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_200 }
    } else {
        if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_900 } else { egui::Color32::WHITE }
    };
    let border = if hover { ColorPalette::BLUE_500 } else if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 };
    (bg, border)
}

pub fn error_panel_colors(theme: ThemeMode) -> (egui::Color32, egui::Color32, egui::Color32) {
    if matches!(theme, ThemeMode::Dark) {
        (ColorPalette::ZINC_800, ColorPalette::RED_900, ColorPalette::RED_300)
    } else {
        (ColorPalette::RED_50, ColorPalette::RED_400, ColorPalette::RED_800)
    }
}
