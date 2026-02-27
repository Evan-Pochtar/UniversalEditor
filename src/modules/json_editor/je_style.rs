use eframe::egui;
use crate::style::{ColorPalette};
use super::je_tools::{ValType};

pub fn c_panel(dark: bool) -> egui::Color32 { if dark { egui::Color32::from_rgb(22, 22, 28) } else { ColorPalette::GRAY_50 } }
pub fn c_border(dark: bool) -> egui::Color32 { if dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_200 } }
pub fn c_row_alt(dark: bool) -> egui::Color32 { if dark { egui::Color32::from_rgb(24, 24, 30) } else { egui::Color32::from_rgb(248, 249, 251) } }
pub fn c_row_sel(dark: bool) -> egui::Color32 { if dark { egui::Color32::from_rgb(32, 52, 88) } else { ColorPalette::BLUE_100 } }
pub fn c_row_match(dark: bool) -> egui::Color32 { if dark { egui::Color32::from_rgb(60, 48, 16) } else { ColorPalette::AMBER_100 } }
pub fn c_row_match_active(dark: bool) -> egui::Color32 { if dark { egui::Color32::from_rgb(90, 72, 16) } else { ColorPalette::AMBER_200 } }
pub fn c_key(dark: bool) -> egui::Color32 { if dark { ColorPalette::BLUE_300 } else { ColorPalette::BLUE_700 } }
pub fn c_text(dark: bool) -> egui::Color32 { if dark { ColorPalette::SLATE_200 } else { ColorPalette::GRAY_800 } }
pub fn c_muted(dark: bool) -> egui::Color32 { if dark { ColorPalette::ZINC_500 } else { ColorPalette::GRAY_400 } }
pub fn c_error(dark: bool) -> egui::Color32 { if dark { ColorPalette::RED_400 } else { ColorPalette::RED_600 } }
pub fn c_string(dark: bool) -> egui::Color32 { if dark { ColorPalette::GREEN_400 } else { ColorPalette::GREEN_700 } }
pub fn c_number(dark: bool) -> egui::Color32 { if dark { ColorPalette::AMBER_300 } else { ColorPalette::AMBER_700 } }
pub fn c_bool_null(dark: bool) -> egui::Color32 { if dark { ColorPalette::PURPLE_400 } else { ColorPalette::PURPLE_600 } }
pub fn c_container(dark: bool) -> egui::Color32 { if dark { ColorPalette::TEAL_400 } else { ColorPalette::TEAL_600 } }

pub fn val_color(v: &ValType, dark: bool) -> egui::Color32 {
    match v {
        ValType::Null => c_bool_null(dark),
        ValType::Bool(_) => c_bool_null(dark),
        ValType::Number(_) => c_number(dark),
        ValType::Str(_) => c_string(dark),
        ValType::Array(_) => c_container(dark),
        ValType::Object(_) => c_container(dark),
    }
}

pub fn compact_button(ui: &mut egui::Ui, label: &str, dark: bool) -> egui::Response {
    let (bg, hov, txt) = if dark {
        (egui::Color32::from_rgb(36, 36, 44), egui::Color32::from_rgb(46, 46, 56), ColorPalette::SLATE_200)
    } else {
        (egui::Color32::WHITE, ColorPalette::GRAY_100, ColorPalette::GRAY_800)
    };
    ui.scope(|ui| {
        let s = ui.style_mut();
        s.visuals.widgets.inactive.bg_fill = bg;
        s.visuals.widgets.inactive.weak_bg_fill = bg;
        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, c_border(dark));
        s.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, txt);
        s.visuals.widgets.hovered.bg_fill = hov;
        s.visuals.widgets.hovered.weak_bg_fill = hov;
        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, c_border(dark));
        s.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, txt);
        s.visuals.widgets.active.bg_fill = hov;
        ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(txt))
            .min_size(egui::vec2(0.0, 24.0)))
    }).inner
}

pub fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.scope(|ui| {
        let s = ui.style_mut();
        let r = ColorPalette::RED_600;
        let rh = ColorPalette::RED_500;
        s.visuals.widgets.inactive.bg_fill = r;
        s.visuals.widgets.inactive.weak_bg_fill = r;
        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        s.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        s.visuals.widgets.hovered.bg_fill = rh;
        s.visuals.widgets.hovered.weak_bg_fill = rh;
        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        s.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        s.visuals.widgets.active.bg_fill = r;
        ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(egui::Color32::WHITE))
            .min_size(egui::vec2(0.0, 24.0)))
    }).inner
}

pub fn accent_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.scope(|ui| {
        let s = ui.style_mut();
        s.visuals.widgets.inactive.bg_fill = ColorPalette::BLUE_600;
        s.visuals.widgets.inactive.weak_bg_fill = ColorPalette::BLUE_600;
        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        s.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        s.visuals.widgets.hovered.bg_fill = ColorPalette::BLUE_500;
        s.visuals.widgets.hovered.weak_bg_fill = ColorPalette::BLUE_500;
        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        s.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        s.visuals.widgets.active.bg_fill = ColorPalette::BLUE_700;
        ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(egui::Color32::WHITE))
            .min_size(egui::vec2(0.0, 24.0)))
    }).inner
}

pub fn ghost_btn_small(ui: &mut egui::Ui, label: &str, dark: bool, enabled: bool) -> egui::Response {
    let txt = if enabled { c_text(dark) } else { c_muted(dark) };
    ui.scope(|ui| {
        if !enabled { ui.disable(); }
        let s = ui.style_mut();
        s.visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
        s.visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, if enabled { c_border(dark) } else { c_muted(dark) });
        s.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, txt);
        s.visuals.widgets.hovered.bg_fill = if dark { egui::Color32::from_rgb(34, 34, 42) } else { ColorPalette::GRAY_100 };
        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, c_border(dark));
        s.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, txt);
        s.visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
        ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(txt))
            .min_size(egui::vec2(0.0, 24.0)))
    }).inner
}

pub fn expand_triangle(ui: &mut egui::Ui, rect: egui::Rect, expanded: bool, dark: bool) -> bool {
    let c: egui::Pos2 = rect.center();
    let s: f32 = 4.5_f32;
    let color: egui::Color32 = c_muted(dark);
    let resp = ui.allocate_rect(rect, egui::Sense::click());
    if resp.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }

    let pts: Vec<egui::Pos2> = if expanded {vec![c + egui::vec2(-s, -s * 0.5), c + egui::vec2(s, -s * 0.5), c + egui::vec2(0.0, s * 0.8)] }
    else { vec![c + egui::vec2(-s * 0.5, -s), c + egui::vec2(-s * 0.5, s), c + egui::vec2(s * 0.8, 0.0)] };

    let tri_color = if resp.hovered() { c_text(dark) } else { color };
    ui.painter().add(egui::Shape::convex_polygon(pts, tri_color, egui::Stroke::NONE));
    
    resp.clicked()
}