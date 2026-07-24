use eframe::egui;
use crate::style::ColorPalette;

pub fn header_bg(dark: bool) -> egui::Color32 { if dark { egui::Color32::from_rgb(30,30,36) } else { ColorPalette::GRAY_100 } }
pub fn header_text(dark: bool) -> egui::Color32 { if dark { ColorPalette::ZINC_300 } else { ColorPalette::GRAY_600 } }
pub fn grid_line(dark: bool) -> egui::Color32 { if dark { egui::Color32::from_rgb(48,48,56) } else { ColorPalette::GRAY_200 } }
pub fn row_alt(dark: bool) -> egui::Color32 { if dark { egui::Color32::from_rgb(24,24,30) } else { egui::Color32::from_rgb(249,250,251) } }
pub fn sel_fill(dark: bool) -> egui::Color32 { if dark { egui::Color32::from_rgba_unmultiplied(59,130,246,55) } else { egui::Color32::from_rgba_unmultiplied(59,130,246,35) } }
pub fn cell_bg(dark: bool, fmt_bg: Option<[u8;3]>, alt: bool) -> egui::Color32 { fmt_bg.map(|c| egui::Color32::from_rgb(c[0],c[1],c[2])).unwrap_or(if alt {row_alt(dark)} else if dark {egui::Color32::from_rgb(18,18,22)} else {egui::Color32::WHITE}) }
pub fn cell_text(dark: bool, fmt_color: Option<[u8;3]>) -> egui::Color32 { fmt_color.map(|c| egui::Color32::from_rgb(c[0],c[1],c[2])).unwrap_or(if dark {ColorPalette::SLATE_200} else {ColorPalette::GRAY_800}) }
