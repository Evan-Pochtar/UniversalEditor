use eframe::egui;
use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use super::ie_main::{THandle, BlendMode, HANDLE_HIT, HANDLE_VIS};

pub(super) fn config_path(filename: &str) -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("universal_editor");
    p.push(filename);
    p
}

pub(super) fn load_persisted<T: for<'de> Deserialize<'de> + Default>(filename: &str) -> T {
    fs::read_to_string(config_path(filename))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(super) fn save_persisted<T: Serialize>(filename: &str, val: &T) {
    let path = config_path(filename);
    if let Some(p) = path.parent() { let _ = fs::create_dir_all(p); }
    if let Ok(j) = serde_json::to_string(val) { let _ = fs::write(path, j); }
}

#[inline]
pub(super) fn blend_pixels_u8(dst: [u8; 4], src: [u8; 4], opacity: f32, mode: BlendMode) -> [u8; 4] {
    let sa = (src[3] as f32 / 255.0) * opacity;
    if sa < 1e-6 { return dst; }
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a < 1e-6 { return [0, 0, 0, 0]; }
    let sr = [src[0] as f32/255.0, src[1] as f32/255.0, src[2] as f32/255.0];
    let dr = [dst[0] as f32/255.0, dst[1] as f32/255.0, dst[2] as f32/255.0];
    let out = std::array::from_fn::<f32, 3, _>(|i| (mode.blend_channel(dr[i], sr[i]) * sa + dr[i] * da * (1.0 - sa)) / out_a);
    [
        (out[0]*255.0).clamp(0.0,255.0) as u8,
        (out[1]*255.0).clamp(0.0,255.0) as u8,
        (out[2]*255.0).clamp(0.0,255.0) as u8,
        (out_a*255.0).clamp(0.0,255.0) as u8,
    ]
}

#[inline]
pub(super) fn srgb_to_linear(c: u8) -> f32 { let c = c as f32 / 255.0; if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) } }

#[inline]
pub(super) fn linear_to_srgb_u8(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
    (s * 255.0).round() as u8
}

#[inline]
pub(super) fn blend_pixels_linear(dst: [u8; 4], src: [u8; 4], opacity: f32, mode: BlendMode) -> [u8; 4] {
    let sa = (src[3] as f32 / 255.0) * opacity;
    if sa < 1e-6 { return dst; }
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a < 1e-6 { return [0, 0, 0, 0]; }
    let sl = [srgb_to_linear(src[0]), srgb_to_linear(src[1]), srgb_to_linear(src[2])];
    let dl = [srgb_to_linear(dst[0]), srgb_to_linear(dst[1]), srgb_to_linear(dst[2])];
    let out = std::array::from_fn::<f32, 3, _>(|i| (mode.blend_channel(dl[i], sl[i]) * sa + dl[i] * da * (1.0 - sa)) / out_a);
    [
        linear_to_srgb_u8(out[0]),
        linear_to_srgb_u8(out[1]),
        linear_to_srgb_u8(out[2]),
        (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

pub(super) fn rgb_to_hsv_f32(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let h = if delta == 0.0 { 0.0 }
        else if max == r { 60.0 * (((g - b) / delta) % 6.0) }
        else if max == g { 60.0 * ((b - r) / delta + 2.0) }
        else { 60.0 * ((r - g) / delta + 4.0) };
    (if h < 0.0 { h + 360.0 } else { h }, s, max)
}

pub(super) fn hsv_to_rgb_f32(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (r + m, g + m, b + m)
}

pub(super) fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    rgb_to_hsv_f32(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

pub(super) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let (r, g, b) = hsv_to_rgb_f32(h, s, v);
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
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
        if egui::Rect::from_center_size(hp, egui::vec2(HANDLE_HIT, HANDLE_HIT)).contains(pos) { return Some(h); }
    }
    if r.contains(pos) { return Some(THandle::Move); }
    None
}

pub(super) fn draw_crop_handles(painter: &egui::Painter, r: egui::Rect, color: egui::Color32) {
    for (h, hp) in crop_handle_positions(r) {
        if h == THandle::Move { continue; }
        let hr = egui::Rect::from_center_size(hp, egui::vec2(HANDLE_VIS, HANDLE_VIS));
        painter.rect_filled(hr, 2.0, color);
        painter.rect_stroke(hr, 2.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Outside);
    }
}

pub(super) fn smooth_hash_2d(px: u32, py: u32, scale: u32, seed: u64) -> f32 {
    let s = scale.max(1);
    let (gx, gy) = (px/s, py/s);
    let (fx, fy) = ((px%s) as f32/s as f32, (py%s) as f32/s as f32);
    let (ux, uy) = (fx*fx*(3.0-2.0*fx), fy*fy*(3.0-2.0*fy));
    let h = |x: u32, y: u32| -> f32 {
        brush_rand((x as u64).wrapping_mul(0x517CC1B7) ^ (y as u64).wrapping_mul(0x9E3779B9) ^ seed)
    };
    let (n00, n10, n01, n11) = (h(gx,gy), h(gx+1,gy), h(gx,gy+1), h(gx+1,gy+1));
    let x0 = n00 + (n10-n00)*ux;
    let x1 = n01 + (n11-n01)*ux;
    x0 + (x1-x0)*uy
}

#[inline(always)]
pub(super) fn brush_rand(seed: u64) -> f32 {
    let x = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let x = x ^ (x >> 33);
    let x = x.wrapping_mul(0xff51afd7ed558ccd);
    let x = x ^ (x >> 33);
    let x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    let x = x ^ (x >> 33);
    (x >> 11) as f32 / (1u64 << 53) as f32
}

#[inline(always)]
pub(super) fn retouch_lerp_u8(a: u8, b: u8, t: f32) -> u8 { (a as f32 + (b as f32 - a as f32) * t).clamp(0.0, 255.0) as u8 }
