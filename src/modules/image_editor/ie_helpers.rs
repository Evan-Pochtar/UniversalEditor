use eframe::egui;
use super::ie_main::{
    THandle,
    HANDLE_HIT, HANDLE_VIS
};

pub(super) fn rgb_to_hsv_f32(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max: f32 = r.max(g).max(b);
    let min: f32 = r.min(g).min(b);
    let delta: f32 = max - min;
    let v: f32 = max;
    let s: f32 = if max == 0.0 { 0.0 } else { delta / max };
    let h: f32 = if delta == 0.0 { 0.0 }
        else if max == r { 60.0 * (((g - b) / delta) % 6.0) }
        else if max == g { 60.0 * ((b - r) / delta + 2.0) }
        else { 60.0 * ((r - g) / delta + 4.0) };
    (if h < 0.0 { h + 360.0 } else { h }, s, v)
}

pub(super) fn hsv_to_rgb_f32(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c: f32 = v * s;
    let x: f32 = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m: f32 = v - c;
    let (r, g, b) = match h as u32 {
        0..=59   => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _         => (c, 0.0, x),
    };
    (r + m, g + m, b + m)
}

pub(super) fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max: f32 = r.max(g).max(b);
    let min: f32 = r.min(g).min(b);
    let delta: f32 = max - min;
    let v: f32 = max;
    let s: f32 = if max == 0.0 { 0.0 } else { delta / max };
    let h: f32 = if delta == 0.0 { 0.0 }
        else if max == r { 60.0 * (((g - b) / delta) % 6.0) }
        else if max == g { 60.0 * ((b - r) / delta + 2.0) }
        else { 60.0 * ((r - g) / delta + 4.0) };
    (if h < 0.0 { h + 360.0 } else { h }, s, v)
}

pub(super) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c: f32 = v * s;
    let x: f32 = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m: f32 = v - c;
    let (r, g, b) = match h as u32 {
        0..=59   => (c, x, 0.0), 60..=119 => (x, c, 0.0), 120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c), 240..=299 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}

pub(super) fn crop_handle_positions(r: egui::Rect) -> [(THandle, egui::Pos2); 9] {
    let (cx, cy) = (r.center().x, r.center().y);
    [
        (THandle::NW, r.left_top()),
        (THandle::N,  egui::pos2(cx, r.min.y)),
        (THandle::NE, r.right_top()),
        (THandle::E,  egui::pos2(r.max.x, cy)),
        (THandle::SE, r.right_bottom()),
        (THandle::S,  egui::pos2(cx, r.max.y)),
        (THandle::SW, r.left_bottom()),
        (THandle::W,  egui::pos2(r.min.x, cy)),
        (THandle::Move, r.center()),
    ]
}

pub(super) fn crop_hit_handle(pos: egui::Pos2, r: egui::Rect) -> Option<THandle> {
    for (h, hp) in crop_handle_positions(r) {
        if h == THandle::Move { continue; }
        if egui::Rect::from_center_size(hp, egui::vec2(HANDLE_HIT, HANDLE_HIT)).contains(pos) {
            return Some(h);
        }
    }
    if r.contains(pos) { return Some(THandle::Move); }
    None
}

pub(super) fn draw_crop_handles(painter: &egui::Painter, r: egui::Rect, color: egui::Color32) {
    for (h, hp) in crop_handle_positions(r) {
        if h == THandle::Move { continue; }
        painter.rect_filled(egui::Rect::from_center_size(hp, egui::vec2(HANDLE_VIS, HANDLE_VIS)), 2.0, color);
        painter.rect_stroke(egui::Rect::from_center_size(hp, egui::vec2(HANDLE_VIS, HANDLE_VIS)), 2.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Outside);
    }
}