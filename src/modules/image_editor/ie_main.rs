use eframe::egui;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, ImageReader, Rgba};
use crate::modules::helpers::image_export::ExportFormat;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::style::ThemeMode;
use crate::modules::{EditorModule, MenuAction, MenuItem, MenuContribution};
use serde::{Deserialize, Serialize};
use super::ie_helpers::{load_persisted, save_persisted, blend_pixels_u8, blend_pixels_linear};

pub(super) const MAX_UNDO: usize = 20;
pub(super) const MAX_COLOR_HISTORY: usize = 20;
pub(super) const MAX_COLOR_FAVORITES: usize = 30;
pub(super) const COLOR_FAV_HOTKEYS: usize = 10;
pub(super) const HANDLE_HIT: f32 = 22.0;
pub(super) const HANDLE_VIS: f32 = 8.0;
pub(super) const ROTATE_DIST: f32 = 28.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(super) struct RgbaColor { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }

impl RgbaColor {
    pub(super) fn to_egui(&self) -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a) }
    pub(super) fn from_egui(c: egui::Color32) -> Self { Self { r: c.r(), g: c.g(), b: c.b(), a: c.a() } }

    pub(super) fn to_hex(&self) -> String {
        if self.a == 255 { format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b) }
        else { format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a) }
    }
    pub(super) fn from_hex(hex: &str) -> Option<Self> {
        let hex: &str = hex.trim_start_matches('#');
        match hex.len() {
            6 => Some(Self {
                r: u8::from_str_radix(&hex[0..2], 16).ok()?,
                g: u8::from_str_radix(&hex[2..4], 16).ok()?,
                b: u8::from_str_radix(&hex[4..6], 16).ok()?,
                a: 255,
            }),
            8 => Some(Self {
                r: u8::from_str_radix(&hex[0..2], 16).ok()?,
                g: u8::from_str_radix(&hex[2..4], 16).ok()?,
                b: u8::from_str_radix(&hex[4..6], 16).ok()?,
                a: u8::from_str_radix(&hex[6..8], 16).ok()?,
            }),
            _ => None,
        }
    }

    pub(super) fn to_rgb_string(&self) -> String {
        if self.a == 255 { format!("rgb({}, {}, {})", self.r, self.g, self.b) }
        else { format!("rgba({}, {}, {}, {:.2})", self.r, self.g, self.b, self.a as f32 / 255.0) }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub(super) struct ColorHistory { pub colors: VecDeque<RgbaColor> }

impl ColorHistory {
    pub(super) fn load() -> Self { load_persisted("color_history.json") }
    pub(super) fn save(&self) { save_persisted("color_history.json", self); }
    pub(super) fn add_color(&mut self, color: RgbaColor) {
        if let Some(pos) = self.colors.iter().position(|c| *c == color) { self.colors.remove(pos); }
        self.colors.push_front(color);
        if self.colors.len() > MAX_COLOR_HISTORY { self.colors.pop_back(); }
        self.save();
    }
    pub(super) fn get_colors(&self) -> &VecDeque<RgbaColor> { &self.colors }
}

#[derive(Serialize, Deserialize, Default)]
pub(super) struct ColorFavorites { pub colors: Vec<RgbaColor> }

impl ColorFavorites {
    pub(super) fn load() -> Self { load_persisted("color_favorites.json") }
    pub(super) fn save(&self) { save_persisted("color_favorites.json", self); }

    pub(super) fn toggle(&mut self, color: RgbaColor) -> bool {
        if let Some(pos) = self.colors.iter().position(|c| *c == color) {
            self.colors.remove(pos);
            self.save();
            false
        } else if self.colors.len() < MAX_COLOR_FAVORITES {
            self.colors.push(color);
            self.save();
            true
        } else {
            true
        }
    }

    pub(super) fn contains(&self, color: RgbaColor) -> bool {
        self.colors.iter().any(|c| *c == color)
    }

    pub(super) fn move_item(&mut self, from: usize, to: usize) {
        if from == to || from >= self.colors.len() || to >= self.colors.len() { return; }
        let item = self.colors.remove(from);
        self.colors.insert(to, item);
        self.save();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tool { Brush, Eraser, Fill, Text, Eyedropper, Crop, Pan, Retouch }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(super) enum RetouchMode { Blur, Sharpen, Smudge, Vibrance, Saturation, Temperature, Brightness, Pixelate, }

impl RetouchMode {
    pub(super) fn label(&self) -> &'static str {
        match self {
            Self::Blur => "Blur",
            Self::Sharpen => "Sharpen",
            Self::Smudge => "Smudge",
            Self::Vibrance => "Vibrance",
            Self::Saturation => "Saturation",
            Self::Temperature => "Temperature",
            Self::Brightness  => "Brightness",
            Self::Pixelate => "Pixelate",
        }
    }
    pub(super) fn strength_label(&self) -> &'static str {
        match self {
            Self::Blur => "Radius",
            Self::Sharpen => "Amount",
            Self::Smudge => "Strength",
            Self::Vibrance => "Boost",
            Self::Saturation => "Amount",
            Self::Temperature => "Shift",
            Self::Brightness => "Amount",
            Self::Pixelate => "Block Size",
        }
    }
    pub(super) fn all() -> &'static [RetouchMode] {
        &[
            Self::Blur, Self::Sharpen, Self::Smudge,
            Self::Vibrance, Self::Saturation, Self::Temperature, Self::Brightness,
            Self::Pixelate,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(super) enum BrushShape { Circle, Square, Diamond, CalligraphyFlat, }

impl BrushShape {
    pub(super) fn label(&self) -> &'static str {
        match self {
            BrushShape::Circle => "Circle",
            BrushShape::Square => "Square",
            BrushShape::Diamond => "Diamond",
            BrushShape::CalligraphyFlat => "Flat",
        }
    }
    pub(super) fn all() -> &'static [BrushShape] { &[BrushShape::Circle, BrushShape::Square, BrushShape::Diamond, BrushShape::CalligraphyFlat] }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(super) enum BrushTextureMode { None, Rough, Canvas, Paper, }

impl BrushTextureMode {
    pub(super) fn label(&self) -> &'static str { match self { Self::None => "None", Self::Rough => "Rough", Self::Canvas => "Canvas", Self::Paper => "Paper" } }
    pub(super) fn all() -> &'static [BrushTextureMode] { &[Self::None, Self::Rough, Self::Canvas, Self::Paper] }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct BrushSettings {
    pub size: f32,
    pub opacity: f32,
    pub softness: f32,
    pub step: f32,
    pub flow: f32,
    pub angle: f32,
    pub angle_jitter: f32,
    pub scatter: f32,
    pub aspect_ratio: f32,
    pub texture_mode: BrushTextureMode,
    pub texture_strength: f32,
    pub shape: BrushShape,
    pub spray_mode: bool,
    pub spray_particles: u32,
    pub wetness: f32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            size: 12.0, opacity: 1.0, softness: 0.7, step: 0.25, flow: 1.0,
            angle: 0.0, angle_jitter: 0.0, scatter: 0.0,
            aspect_ratio: 0.3, texture_mode: BrushTextureMode::None, texture_strength: 0.0,
            shape: BrushShape::Circle, spray_mode: false, spray_particles: 40, wetness: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) enum BrushPreset { Regular, Pencil, Pen, Crayon, Marker, Calligraphy, SprayPaint, Watercolor, Charcoal, Airbrush, }

impl BrushPreset {
    pub(super) fn label(&self) -> &'static str {
        match self {
            Self::Regular => "Regular",
            Self::Pencil => "Pencil",
            Self::Pen => "Pen",
            Self::Crayon => "Crayon",
            Self::Marker => "Marker",
            Self::Calligraphy => "Calligraphy",
            Self::SprayPaint => "Spray",
            Self::Watercolor => "Watercolor",
            Self::Charcoal => "Charcoal",
            Self::Airbrush => "Airbrush",
        }
    }

    pub(super) fn all() -> &'static [BrushPreset] {
        &[
            Self::Regular, Self::Pencil, Self::Pen, Self::Crayon, Self::Marker,
            Self::Calligraphy, Self::SprayPaint, Self::Watercolor, Self::Charcoal, Self::Airbrush,
        ]
    }

    pub(super) fn settings(&self, current_size: f32) -> BrushSettings {
        match self {
            Self::Regular => BrushSettings {
                size: current_size, opacity: 1.0, softness: 0.7, step: 0.25, flow: 1.0,
                shape: BrushShape::Circle, scatter: 0.0, angle: 0.0, angle_jitter: 0.0,
                aspect_ratio: 1.0, texture_mode: BrushTextureMode::None, texture_strength: 0.0,
                spray_mode: false, spray_particles: 40, wetness: 0.0,
            },
            Self::Pencil => BrushSettings {
                size: current_size, opacity: 0.85, softness: 0.0, step: 0.35, flow: 0.75,
                shape: BrushShape::Circle, scatter: current_size * 0.08, angle: 0.0, angle_jitter: 0.0,
                aspect_ratio: 1.0, texture_mode: BrushTextureMode::Rough, texture_strength: 0.45,
                spray_mode: false, spray_particles: 40, wetness: 0.0,
            },
            Self::Pen => BrushSettings {
                size: current_size, opacity: 1.0, softness: 0.0, step: 0.12, flow: 1.0,
                shape: BrushShape::Circle, scatter: 0.0, angle: 0.0, angle_jitter: 0.0,
                aspect_ratio: 1.0, texture_mode: BrushTextureMode::None, texture_strength: 0.0,
                spray_mode: false, spray_particles: 40, wetness: 0.0,
            },
            Self::Crayon => BrushSettings {
                size: current_size, opacity: 0.75, softness: 0.05, step: 0.20, flow: 0.65,
                shape: BrushShape::Square, scatter: current_size * 0.18, angle: 15.0, angle_jitter: 12.0,
                aspect_ratio: 1.0, texture_mode: BrushTextureMode::Rough, texture_strength: 0.55,
                spray_mode: false, spray_particles: 40, wetness: 0.0,
            },
            Self::Marker => BrushSettings {
                size: current_size, opacity: 0.90, softness: 0.0, step: 0.18, flow: 0.85,
                shape: BrushShape::Circle, scatter: 0.0, angle: 0.0, angle_jitter: 0.0,
                aspect_ratio: 1.0, texture_mode: BrushTextureMode::None, texture_strength: 0.0,
                spray_mode: false, spray_particles: 40, wetness: 0.4,
            },
            Self::Calligraphy => BrushSettings {
                size: current_size, opacity: 1.0, softness: 0.10, step: 0.18, flow: 1.0,
                shape: BrushShape::CalligraphyFlat, scatter: 0.0, angle: 45.0, angle_jitter: 0.0,
                aspect_ratio: 0.18, texture_mode: BrushTextureMode::None, texture_strength: 0.0,
                spray_mode: false, spray_particles: 40, wetness: 0.0,
            },
            Self::SprayPaint => BrushSettings {
                size: current_size.max(20.0), opacity: 0.85, softness: 1.0, step: 0.50, flow: 0.12,
                shape: BrushShape::Circle, scatter: current_size * 0.6, angle: 0.0, angle_jitter: 0.0,
                aspect_ratio: 1.0, texture_mode: BrushTextureMode::None, texture_strength: 0.0,
                spray_mode: true, spray_particles: 60, wetness: 0.0,
            },
            Self::Watercolor => BrushSettings {
                size: current_size, opacity: 0.70, softness: 0.90, step: 0.15, flow: 0.25,
                shape: BrushShape::Circle, scatter: current_size * 0.12, angle: 0.0, angle_jitter: 0.0,
                aspect_ratio: 1.0, texture_mode: BrushTextureMode::Paper, texture_strength: 0.30,
                spray_mode: false, spray_particles: 40, wetness: 0.40,
            },
            Self::Charcoal => BrushSettings {
                size: current_size, opacity: 0.80, softness: 0.08, step: 0.28, flow: 0.55,
                shape: BrushShape::Diamond, scatter: current_size * 0.14, angle: 30.0, angle_jitter: 18.0,
                aspect_ratio: 1.0, texture_mode: BrushTextureMode::Canvas, texture_strength: 0.50,
                spray_mode: false, spray_particles: 40, wetness: 0.0,
            },
            Self::Airbrush => BrushSettings {
                size: current_size, opacity: 0.80, softness: 1.0, step: 0.12, flow: 0.12,
                shape: BrushShape::Circle, scatter: 0.0, angle: 0.0, angle_jitter: 0.0,
                aspect_ratio: 1.0, texture_mode: BrushTextureMode::None, texture_strength: 0.0,
                spray_mode: false, spray_particles: 40, wetness: 0.0,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct SavedBrush { pub name: String, pub settings: BrushSettings, }

#[derive(Serialize, Deserialize, Default)]
pub(super) struct BrushFavorites { pub brushes: Vec<SavedBrush>, }

impl BrushFavorites {
    pub(super) fn load() -> Self { load_persisted("brush_favorites.json") }
    pub(super) fn save(&self) { save_persisted("brush_favorites.json", self); }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum FilterPanel { None, BrightnessContrast, HueSaturation, Blur, Sharpen, Resize, Export, Brush }

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum THandle { Move, N, S, E, W, NE, NW, SE, SW, Rotate }

pub(super) struct TransformHandleSet { pub rect: egui::Rect, pub angle_rad: f32 }

impl TransformHandleSet {
    pub(super) fn with_rotation(rect: egui::Rect, angle_rad: f32) -> Self { Self { rect, angle_rad } }
    fn rot(&self, p: egui::Pos2) -> egui::Pos2 {
        if self.angle_rad == 0.0 { return p; }
        let c: egui::Pos2 = self.rect.center();
        let d: egui::Vec2 = p - c;
        let (cos_a, sin_a) = (self.angle_rad.cos(), self.angle_rad.sin());
        c + egui::vec2(d.x * cos_a - d.y * sin_a, d.x * sin_a + d.y * cos_a)
    }

    pub(super) fn positions(&self) -> [(THandle, egui::Pos2); 9] {
        let r: &egui::Rect = &self.rect;
        let (cx, cy) = (r.center().x, r.center().y);
        let top_center: egui::Pos2 = self.rot(egui::pos2(cx, r.min.y));
        let (cos_a, sin_a) = (self.angle_rad.cos(), self.angle_rad.sin());
        let up_dir: egui::Vec2 = egui::vec2(sin_a, -cos_a);
        let rot_handle_pos: egui::Pos2 = top_center + up_dir * ROTATE_DIST;
        [
            (THandle::NW, self.rot(r.left_top())),
            (THandle::N, self.rot(egui::pos2(cx, r.min.y))),
            (THandle::NE, self.rot(r.right_top())),
            (THandle::E, self.rot(egui::pos2(r.max.x, cy))),
            (THandle::SE, self.rot(r.right_bottom())),
            (THandle::S, self.rot(egui::pos2(cx, r.max.y))),
            (THandle::SW, self.rot(r.left_bottom())),
            (THandle::W, self.rot(egui::pos2(r.min.x, cy))),
            (THandle::Rotate, rot_handle_pos),
        ]
    }

    pub(super) fn hit_test(&self, pos: egui::Pos2) -> Option<THandle> {
        for (h, hpos) in self.positions() {
            if egui::Rect::from_center_size(hpos, egui::vec2(HANDLE_HIT, HANDLE_HIT)).contains(pos) {
                return Some(h);
            }
        }

        let c: egui::Pos2 = self.rect.center();
        let d: egui::Vec2 = pos - c;
        let (cos_a, sin_a) = (self.angle_rad.cos(), self.angle_rad.sin());
        let local: egui::Pos2 = c + egui::vec2(d.x * cos_a + d.y * sin_a, -d.x * sin_a + d.y * cos_a);
        if self.rect.contains(local) { return Some(THandle::Move); }

        None
    }

    pub(super) fn draw(&self, painter: &egui::Painter, accent: egui::Color32) {
        let corners: [egui::Pos2; 4] = [self.rect.left_top(), self.rect.right_top(), self.rect.right_bottom(), self.rect.left_bottom()];
        let rc: Vec<egui::Pos2> = corners.iter().map(|&p| self.rot(p)).collect();

        for i in 0..4 {
            painter.line_segment([rc[i], rc[(i+1) % 4]], egui::Stroke::new(1.5, accent));
        }

        let positions: [(THandle, egui::Pos2); 9] = self.positions();
        let (_, rot_pos) = positions[8];
        let top_center: egui::Pos2 = positions[1].1;
        painter.line_segment([top_center, rot_pos], egui::Stroke::new(1.0, accent));

        for (h, hpos) in positions {
            let vis: f32 = if h == THandle::Rotate { HANDLE_VIS * 1.25 } else { HANDLE_VIS };
            let rnd: f32 = if h == THandle::Rotate { vis / 2.0 } else { 2.0 };
            painter.rect_filled(egui::Rect::from_center_size(hpos, egui::vec2(vis, vis)), rnd, accent);
        }
    }

    pub(super) fn cursor_for(h: THandle) -> egui::CursorIcon {
        match h {
            THandle::Move => egui::CursorIcon::Grab,
            THandle::N | THandle::S => egui::CursorIcon::ResizeVertical,
            THandle::E | THandle::W => egui::CursorIcon::ResizeHorizontal,
            THandle::NE | THandle::SW => egui::CursorIcon::ResizeNeSw,
            THandle::NW | THandle::SE => egui::CursorIcon::ResizeNwSe,
            THandle::Rotate => egui::CursorIcon::Alias,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct TextLayer {
    pub id: u64,
    pub content: String,
    pub img_x: f32,
    pub img_y: f32,
    pub font_size: f32,
    pub box_width: Option<f32>,
    pub box_height: Option<f32>,
    pub rotation: f32,
    pub color: egui::Color32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font_name: String,
    pub rendered_height: f32,
    pub cached_lines: Vec<String>,
}

impl TextLayer {
    pub(super) fn line_count(&self) -> usize { self.content.lines().count().max(1) }
    pub(super) fn max_line_chars(&self) -> usize {
        self.content.lines().map(|l| l.chars().count()).max().unwrap_or(1).max(1)
    }

    pub(super) fn auto_width(&self, zoom: f32) -> f32 {
        (self.max_line_chars() as f32 * self.font_size * 0.58 * zoom).max(self.font_size * zoom)
    }
    pub(super) fn auto_height(&self, zoom: f32) -> f32 {
        if self.rendered_height > 0.0 { self.rendered_height * zoom }
        else { self.line_count() as f32 * self.font_size * 1.35 * zoom }
    }

    pub(super) fn screen_rect(&self, anchor: egui::Pos2, zoom: f32) -> egui::Rect {
        let w: f32 = self.box_width.map(|bw| bw * zoom).unwrap_or_else(|| self.auto_width(zoom));
        let h: f32 = self.box_height.map(|bh| bh * zoom).unwrap_or_else(|| self.auto_height(zoom));
        egui::Rect::from_min_size(anchor, egui::vec2(w, h))
    }

    pub(super) fn font_family_name(&self) -> &'static str {
        match (self.font_name.as_str(), self.bold, self.italic) {
            ("Roboto", true, _) => "Roboto-Bold",
            ("Roboto", _, true) => "Roboto-Italic",
            ("Roboto", ..) => "Roboto",
            (_, true, _) => "Ubuntu-Bold",
            (_, _, true) => "Ubuntu-Italic",
            _ => "Ubuntu",
        }
    }
}

pub(super) struct TextDrag {
    pub handle: THandle,
    pub start: egui::Pos2,
    pub orig_img_x: f32,
    pub orig_img_y: f32,
    pub orig_font_size: f32,
    pub orig_box_width: Option<f32>,
    pub orig_box_height: Option<f32>,
    pub orig_rotation: f32,
    pub orig_rot_start_angle: f32,
}

#[derive(Debug, Clone)]
pub struct ImageLayerData {
    pub id: u64,
    pub image: DynamicImage,
    pub canvas_x: f32,
    pub canvas_y: f32,
    pub display_w: f32,
    pub display_h: f32,
    pub rotation: f32,
    pub flip_h: bool,
    pub flip_v: bool,
}

impl ImageLayerData {
    pub(super) fn orig_w(&self) -> u32 { self.image.width() }
    pub(super) fn orig_h(&self) -> u32 { self.image.height() }
    pub(super) fn native_aspect(&self) -> f32 { if self.image.height() > 0 { self.image.width() as f32 / self.image.height() as f32 } else { 1.0 } }
    pub(super) fn center_canvas(&self) -> (f32, f32) { (self.canvas_x + self.display_w / 2.0, self.canvas_y + self.display_h / 2.0) }
    pub(super) fn screen_rect(&self, editor_img_w: f32, editor_img_h: f32, canvas: egui::Rect, zoom: f32, pan: egui::Vec2) -> egui::Rect {
        let ox = canvas.center().x - editor_img_w * zoom / 2.0 + pan.x;
        let oy = canvas.center().y - editor_img_h * zoom / 2.0 + pan.y;
        egui::Rect::from_min_size(egui::pos2(ox + self.canvas_x * zoom, oy + self.canvas_y * zoom), egui::vec2(self.display_w * zoom, self.display_h * zoom))
    }
    pub(super) fn canvas_to_local_f32(&self, cx: f32, cy: f32) -> (f32, f32) {
        let (ctr_x, ctr_y) = self.center_canvas();
        let (dx, dy) = (cx - ctr_x, cy - ctr_y);
        let angle = -self.rotation.to_radians();
        let (cos_a, sin_a) = (angle.cos(), angle.sin());
        let lx = dx * cos_a - dy * sin_a + self.display_w / 2.0;
        let ly = dx * sin_a + dy * cos_a + self.display_h / 2.0;
        let mut px = (lx / self.display_w.max(1.0)) * self.orig_w() as f32;
        let mut py = (ly / self.display_h.max(1.0)) * self.orig_h() as f32;
        if self.flip_h { px = self.orig_w() as f32 - 1.0 - px; }
        if self.flip_v { py = self.orig_h() as f32 - 1.0 - py; }
        (px, py)
    }
    pub(super) fn canvas_to_local(&self, cx: f32, cy: f32) -> Option<(u32, u32)> {
        let (px, py) = self.canvas_to_local_f32(cx, cy);
        if px >= 0.0 && py >= 0.0 && (px as u32) < self.orig_w() && (py as u32) < self.orig_h() {
            Some((px as u32, py as u32))
        } else { None }
    }
    pub(super) fn pixel_scale(&self) -> f32 {
        let sx = self.orig_w() as f32 / self.display_w.max(1.0);
        let sy = self.orig_h() as f32 / self.display_h.max(1.0);
        (sx + sy) * 0.5
    }
    pub(super) fn hit_test(&self, pos: egui::Pos2, editor_img_w: f32, editor_img_h: f32, canvas: egui::Rect, zoom: f32, pan: egui::Vec2) -> bool {
        let rect = self.screen_rect(editor_img_w, editor_img_h, canvas, zoom, pan);
        let center = rect.center();
        let d = pos - center;
        let a = -self.rotation.to_radians();
        let (cos_a, sin_a) = (a.cos(), a.sin());
        let local = egui::pos2(d.x * cos_a - d.y * sin_a, d.x * sin_a + d.y * cos_a);
        egui::Rect::from_center_size(egui::pos2(0.0, 0.0), rect.size()).contains(local)
    }
}

pub(super) struct ImageDrag {
    pub handle: THandle,
    pub start: egui::Pos2,
    pub orig_x: f32,
    pub orig_y: f32,
    pub orig_w: f32,
    pub orig_h: f32,
    pub orig_rotation: f32,
    pub orig_rot_start_angle: f32,
}

#[derive(Default)]
pub(super) struct CropState { pub start: Option<(f32, f32)>, pub end: Option<(f32, f32)> }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal, Multiply, Screen, Overlay, SoftLight,
    HardLight, Darken, Lighten, Difference, Exclusion,
}

impl BlendMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Overlay => "Overlay",
            Self::SoftLight => "Soft Light",
            Self::HardLight => "Hard Light",
            Self::Darken => "Darken",
            Self::Lighten => "Lighten",
            Self::Difference => "Difference",
            Self::Exclusion => "Exclusion",
        }
    }
    pub fn all() -> &'static [BlendMode] {
        &[
            Self::Normal, Self::Multiply, Self::Screen, Self::Overlay,
            Self::SoftLight, Self::HardLight, Self::Darken, Self::Lighten,
            Self::Difference, Self::Exclusion,
        ]
    }

    pub fn blend_channel(self, bot: f32, top: f32) -> f32 {
        match self {
            Self::Normal => top,
            Self::Multiply => bot * top,
            Self::Screen => 1.0 - (1.0 - bot) * (1.0 - top),
            Self::Overlay => if bot < 0.5 { 2.0 * bot * top } else { 1.0 - 2.0 * (1.0 - bot) * (1.0 - top) },
            Self::SoftLight => {
                if top < 0.5 { bot - (1.0 - 2.0 * top) * bot * (1.0 - bot) }
                else {
                    let d = if bot < 0.25 { ((16.0 * bot - 12.0) * bot + 4.0) * bot } else { bot.sqrt() };
                    bot + (2.0 * top - 1.0) * (d - bot)
                }
            }
            Self::HardLight => if top < 0.5 { 2.0 * bot * top } else { 1.0 - 2.0 * (1.0 - bot) * (1.0 - top) },
            Self::Darken => bot.min(top),
            Self::Lighten => bot.max(top),
            Self::Difference => (bot - top).abs(),
            Self::Exclusion => bot + top - 2.0 * bot * top,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerKind { Background, Raster, Text, Image }

#[derive(Debug, Clone)]
pub struct ImageLayer {
    pub id: u64,
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    pub locked: bool,
    pub blend_mode: BlendMode,
    pub kind: LayerKind,
    pub linked_text_id: Option<u64>,
    pub linked_image_id: Option<u64>,
}

pub(super) struct LayerUndoEntry {
    pub image: Option<DynamicImage>,
    pub layer_images: std::collections::HashMap<u64, DynamicImage>,
    pub layers: Vec<ImageLayer>,
    pub text_layers: Vec<TextLayer>,
    pub active_layer_id: u64,
    pub next_layer_id: u64,
    pub next_text_id: u64,
    pub image_layer_data: std::collections::HashMap<u64, ImageLayerData>,
    pub next_image_layer_id: u64,
}

pub struct ImageEditor {
    pub(super) image: Option<DynamicImage>,
    pub(super) texture: Option<egui::TextureId>,
    pub(super) texture_dirty: bool,
    pub(super) texture_dirty_rect: Option<[u32; 4]>,
    pub(super) file_path: Option<PathBuf>,
    pub(super) dirty: bool,

    pub(super) undo_stack: VecDeque<LayerUndoEntry>,
    pub(super) redo_stack: VecDeque<LayerUndoEntry>,

    pub(super) zoom: f32,
    pub(super) pan: egui::Vec2,
    pub(super) fit_on_next_frame: bool,

    pub(super) tool: Tool,
    pub(super) brush: BrushSettings,
    pub(super) brush_favorites: BrushFavorites,
    pub(super) brush_fav_name: String,
    pub(super) brush_preview_texture: Option<egui::TextureId>,
    pub(super) brush_preview_cache_key: Option<(BrushSettings, egui::Color32, bool)>,
    pub(super) eraser_size: f32,
    pub(super) eraser_transparent: bool,
    pub(super) color: egui::Color32,

    pub(super) stroke_points: Vec<(f32, f32)>,
    pub(super) is_dragging: bool,

    pub(super) text_layers: Vec<TextLayer>,
    pub(super) selected_text: Option<u64>,
    pub(super) editing_text: bool,
    pub(super) next_text_id: u64,
    pub(super) text_font_size: f32,
    pub(super) text_bold: bool,
    pub(super) text_italic: bool,
    pub(super) text_underline: bool,
    pub(super) text_font_name: String,
    pub(super) text_drag: Option<TextDrag>,
    pub(super) text_cursor: usize,
    pub(super) text_sel_anchor: Option<usize>,

    pub(super) crop_state: CropState,
    pub(super) crop_drag: Option<THandle>,
    pub(super) crop_drag_orig: Option<(f32, f32, f32, f32)>,
    pub(super) filter_panel: FilterPanel,
    pub(super) brightness: f32,
    pub(super) contrast: f32,
    pub(super) hue: f32,
    pub(super) saturation: f32,
    pub(super) blur_radius: f32,
    pub(super) sharpen_amount: f32,
    pub(super) resize_w: u32,
    pub(super) resize_h: u32,
    pub(super) resize_locked: bool,
    pub(super) resize_stretch: bool,

    pub(super) export_format: ExportFormat,
    pub(super) export_jpeg_quality: u8,
    pub(super) export_avif_quality: u8,
    pub(super) export_avif_speed: u8,
    pub(super) export_preserve_metadata: bool,
    pub(super) export_auto_scale_ico: bool,
    pub(super) export_callback: Option<Box<dyn Fn(PathBuf) + Send + Sync>>,

    pub(super) show_color_picker: bool,
    pub(super) color_history: ColorHistory,
    pub(super) color_favorites: ColorFavorites,
    pub(super) color_fav_drag_src: Option<usize>,
    pub(super) hex_input: String,
    pub(super) canvas_rect: Option<egui::Rect>,
    pub(super) color_picker_rect: Option<egui::Rect>,
    pub(super) filter_panel_rect: Option<egui::Rect>,

    pub(super) filter_progress: Arc<Mutex<f32>>,
    pub(super) is_processing: bool,
    pub(super) processing_is_preview: bool,
    pub(super) pending_filter_result: Arc<Mutex<Option<DynamicImage>>>,

    pub(super) retouch_mode: RetouchMode,
    pub(super) retouch_size: f32,
    pub(super) retouch_strength: f32,
    pub(super) retouch_softness: f32,
    pub(super) retouch_smudge_sample: [f32; 4],
    pub(super) retouch_pixelate_block: u32,

    pub(super) filter_preview_active: bool,
    pub(super) filter_preview_snapshot: Option<LayerUndoEntry>,

    pub(super) layers: Vec<ImageLayer>,
    pub(super) active_layer_id: u64,
    pub(super) next_layer_id: u64,
    pub(super) layer_images: std::collections::HashMap<u64, DynamicImage>,
    pub(super) composite_dirty: bool,
    pub(super) composite_dirty_rect: Option<[u32; 4]>,
    pub(super) stroke_backdrop: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub(super) backdrop_cache: Arc<Mutex<Option<ImageBuffer<Rgba<u8>, Vec<u8>>>>>,
    pub(super) backdrop_cache_for: u64,
    pub(super) show_layers_panel: bool,
    pub(super) layer_panel_width: f32,
    pub(super) layer_drag_src: Option<usize>,
    pub(super) layer_rename_id: Option<u64>,
    pub(super) layer_rename_buf: String,
    pub(super) filter_target_layer_id: u64,
    pub(super) checker_texture: Option<egui::TextureId>,
    pub(super) checker_texture_dark: bool,

    pub(super) image_layer_data: std::collections::HashMap<u64, ImageLayerData>,
    pub(super) image_layer_textures: std::collections::HashMap<u64, egui::TextureId>,
    pub(super) image_layer_texture_dirty: std::collections::HashSet<u64>,
    pub(super) image_layer_stroke_rects: std::collections::HashMap<u64, [u32; 4]>,
    pub(super) selected_image_layer: Option<u64>,
    pub(super) image_drag: Option<ImageDrag>,
    pub(super) next_image_layer_id: u64,
    pub(super) image_aspect_lock: bool,

    pub(super) raster_layer_textures: std::collections::HashMap<u64, egui::TextureId>,
    pub(super) raster_layer_texture_dirty: std::collections::HashSet<u64>,
    pub(super) raster_layer_dirty_rects: std::collections::HashMap<u64, [u32; 4]>,
}

impl ImageEditor {
    pub fn new() -> Self {
        Self {
            image: None, texture: None, texture_dirty: false, texture_dirty_rect: None,
            file_path: None, dirty: false,
            undo_stack: VecDeque::new(), redo_stack: VecDeque::new(),
            zoom: 1.0, pan: egui::Vec2::ZERO, fit_on_next_frame: true,
            tool: Tool::Brush,
            brush: BrushSettings::default(), brush_favorites: BrushFavorites::load(), brush_fav_name: String::new(),
            brush_preview_texture: None, brush_preview_cache_key: None,
            eraser_size: 20.0, eraser_transparent: false,
            color: egui::Color32::BLACK,
            stroke_points: Vec::new(), is_dragging: false,
            text_layers: Vec::new(), selected_text: None, editing_text: false,
            next_text_id: 0, text_font_size: 24.0,
            text_bold: false, text_italic: false, text_underline: false,
            text_font_name: "Ubuntu".to_string(),
            text_drag: None, text_cursor: 0, text_sel_anchor: None,
            crop_state: CropState::default(), crop_drag: None, crop_drag_orig: None,
            filter_panel: FilterPanel::None,
            brightness: 0.0, contrast: 0.0, hue: 0.0, saturation: 0.0,
            blur_radius: 3.0, sharpen_amount: 1.0,
            resize_w: 0, resize_h: 0, resize_locked: true, resize_stretch: false,
            export_format: ExportFormat::Png,
            export_jpeg_quality: 90, export_avif_quality: 80, export_avif_speed: 4, export_preserve_metadata: true,
            export_auto_scale_ico: true, export_callback: None,
            show_color_picker: false, color_history: ColorHistory::load(),
            color_favorites: ColorFavorites::load(), color_fav_drag_src: None,
            hex_input: String::from("#000000FF"), canvas_rect: None,
            color_picker_rect: None,
            filter_panel_rect: None,
            filter_progress: Arc::new(Mutex::new(0.0)),
            is_processing: false,
            processing_is_preview: false,
            pending_filter_result: Arc::new(Mutex::new(None)),
            retouch_mode: RetouchMode::Blur,
            retouch_size: 40.0,
            retouch_strength: 0.5,
            retouch_softness: 0.7,
            retouch_smudge_sample: [0.0; 4],
            retouch_pixelate_block: 12,
            filter_preview_active: false,
            filter_preview_snapshot: None,
            layers: vec![ImageLayer {
                id: 0, name: "Background".to_string(),
                opacity: 1.0, visible: true, locked: false,
                blend_mode: BlendMode::Normal,
                kind: LayerKind::Background, linked_text_id: None, linked_image_id: None,
            }],
            active_layer_id: 0,
            next_layer_id: 1,
            layer_images: std::collections::HashMap::new(),
            composite_dirty: false,
            composite_dirty_rect: None,
            stroke_backdrop: None,
            backdrop_cache: Arc::new(Mutex::new(None)),
            backdrop_cache_for: u64::MAX,
            show_layers_panel: true,
            layer_panel_width: 240.0,
            layer_drag_src: None,
            layer_rename_id: None,
            layer_rename_buf: String::new(),
            filter_target_layer_id: 0,
            checker_texture: None,
            checker_texture_dark: false,
            image_layer_data: std::collections::HashMap::new(),
            image_layer_textures: std::collections::HashMap::new(),
            image_layer_texture_dirty: std::collections::HashSet::new(),
            image_layer_stroke_rects: std::collections::HashMap::new(),
            selected_image_layer: None,
            image_drag: None,
            next_image_layer_id: 0,
            image_aspect_lock: true,
            raster_layer_textures: std::collections::HashMap::new(),
            raster_layer_texture_dirty: std::collections::HashSet::new(),
            raster_layer_dirty_rects: std::collections::HashMap::new(),
        }
    }

    pub fn load(path: PathBuf) -> Self {
        let mut editor: ImageEditor = Self::new();
        let img_result = ImageReader::open(&path)
            .ok()
            .and_then(|r| r.with_guessed_format().ok())
            .and_then(|r| r.decode().ok())
            .or_else(|| image::open(&path).ok());
        if let Some(img) = img_result {
            editor.resize_w = img.width();
            editor.resize_h = img.height();
            editor.image = Some(DynamicImage::ImageRgba8(img.into_rgba8()));
            editor.texture_dirty = true;
            editor.composite_dirty = true;
            editor.file_path = Some(path);
        }
        editor
    }

    pub fn is_dirty(&self) -> bool { self.dirty }
    pub fn set_file_callback(&mut self, callback: Box<dyn Fn(PathBuf) + Send + Sync>) { self.export_callback = Some(callback);}
    pub(super) fn add_color_to_history(&mut self) { self.color_history.add_color(RgbaColor::from_egui(self.color)); }

    pub(super) fn take_undo_snapshot(&self) -> LayerUndoEntry {
        LayerUndoEntry {
            image: self.image.clone(),
            layer_images: self.layer_images.clone(),
            layers: self.layers.clone(),
            text_layers: self.text_layers.clone(),
            active_layer_id: self.active_layer_id,
            next_layer_id: self.next_layer_id,
            next_text_id: self.next_text_id,
            image_layer_data: self.image_layer_data.clone(),
            next_image_layer_id: self.next_image_layer_id,
        }
    }

    pub(super) fn restore_undo_snapshot(&mut self, entry: LayerUndoEntry) {
        self.image = entry.image;
        self.layer_images = entry.layer_images;
        self.layers = entry.layers;
        self.text_layers = entry.text_layers;
        self.active_layer_id = entry.active_layer_id;
        self.next_layer_id = entry.next_layer_id;
        self.next_text_id = entry.next_text_id;
        let old_keys: std::collections::HashSet<u64> = self.image_layer_data.keys().cloned().collect();
        let new_keys: std::collections::HashSet<u64> = entry.image_layer_data.keys().cloned().collect();
        for removed_id in old_keys.difference(&new_keys) { self.image_layer_texture_dirty.remove(removed_id); }
        for changed_id in new_keys.iter() { self.image_layer_texture_dirty.insert(*changed_id); }
        self.image_layer_data = entry.image_layer_data;
        self.next_image_layer_id = entry.next_image_layer_id;
        self.raster_layer_texture_dirty.clear();
        self.raster_layer_dirty_rects.clear();
        for l in &self.layers {
            if l.kind == LayerKind::Raster {
                self.raster_layer_texture_dirty.insert(l.id);
            }
        }
        if let Some(img) = &self.image {
            self.resize_w = img.width();
            self.resize_h = img.height();
        }
        self.texture_dirty = true;
        self.composite_dirty = true;
        self.dirty = true;
        self.backdrop_cache_for = u64::MAX;
    }

    pub(super) fn push_undo(&mut self) {
        let entry = self.take_undo_snapshot();
        self.undo_stack.push_back(entry);
        if self.undo_stack.len() > MAX_UNDO { self.undo_stack.pop_front(); }
        self.redo_stack.clear();
    }

    pub(super) fn cancel_filter_preview(&mut self) {
        if let Some(snapshot) = self.filter_preview_snapshot.take() {
            self.restore_undo_snapshot(snapshot);
        }
        self.filter_preview_active = false;
        self.processing_is_preview = false;
    }

    pub(super) fn accept_filter_preview(&mut self) {
        if let Some(snapshot) = self.filter_preview_snapshot.take() {
            self.undo_stack.push_back(snapshot);
            if self.undo_stack.len() > MAX_UNDO { self.undo_stack.pop_front(); }
            self.redo_stack.clear();
        }
        self.filter_preview_active = false;
    }

    pub(super) fn undo(&mut self) {
        if let Some(entry) = self.undo_stack.pop_back() {
            let current = self.take_undo_snapshot();
            self.redo_stack.push_back(current);
            self.restore_undo_snapshot(entry);
        }
    }

    pub(super) fn redo(&mut self) {
        if let Some(entry) = self.redo_stack.pop_back() {
            let current = self.take_undo_snapshot();
            self.undo_stack.push_back(current);
            self.restore_undo_snapshot(entry);
        }
    }

    pub(super) fn active_raster_image(&self) -> Option<&DynamicImage> {
        match self.layers.iter().find(|l| l.id == self.active_layer_id)?.kind {
            LayerKind::Background => self.image.as_ref(),
            LayerKind::Raster => self.layer_images.get(&self.active_layer_id),
            LayerKind::Text | LayerKind::Image => None,
        }
    }

    pub(super) fn composite_for_display(&self) -> Option<DynamicImage> { self.image.clone() }
    pub(super) fn composite_all_layers(&self) -> Option<DynamicImage> {
        let bg = self.image.as_ref()?;
        let (w, h) = (bg.width(), bg.height());
        let mut result: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(w, h, Rgba([0u8, 0, 0, 0]));
        let mut linked: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for layer in &self.layers {
            if !layer.visible { continue; }
            match layer.kind {
                LayerKind::Text => {
                    if let Some(tid) = layer.linked_text_id {
                        linked.insert(tid);
                        if let Some(tl) = self.text_layers.iter().find(|t| t.id == tid).cloned() {
                            let base = DynamicImage::ImageRgba8(result.clone());
                            result = self.stamp_single_text_layer(&base, &tl, layer.opacity).to_rgba8();
                        }
                    }
                }
                LayerKind::Image => {
                    if let Some(iid) = layer.linked_image_id {
                        if let Some(ild) = self.image_layer_data.get(&iid) {
                            Self::stamp_image_layer(&mut result, ild, layer.opacity, layer.blend_mode);
                        }
                    }
                }
                LayerKind::Background | LayerKind::Raster => {
                    let src = match layer.kind {
                        LayerKind::Background => Some(bg),
                        LayerKind::Raster => self.layer_images.get(&layer.id),
                        _ => unreachable!(),
                    };
                    let Some(src) = src else { continue };
                    let src_rgba = src.to_rgba8();
                    let opacity = layer.opacity.clamp(0.0, 1.0);
                    let mode = layer.blend_mode;
                    for y in 0..h {
                        for x in 0..w {
                            let out = blend_pixels_linear(result.get_pixel(x, y).0, src_rgba.get_pixel(x, y).0, opacity, mode);
                            result.put_pixel(x, y, Rgba(out));
                        }
                    }
                }
            }
        }
        for tl in self.text_layers.iter().filter(|t| !linked.contains(&t.id)) {
            let base = DynamicImage::ImageRgba8(result.clone());
            result = self.stamp_single_text_layer(&base, tl, 1.0).to_rgba8();
        }
        Some(DynamicImage::ImageRgba8(result))
    }

    pub(super) fn stamp_image_layer(composite: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, ild: &ImageLayerData, layer_opacity: f32, blend_mode: BlendMode) {
        let (cw, ch) = (composite.width(), composite.height());
        let (orig_w, orig_h) = (ild.image.width(), ild.image.height());
        if orig_w == 0 || orig_h == 0 || ild.display_w <= 0.0 || ild.display_h <= 0.0 { return; }
        let disp_w = ild.display_w.round().max(1.0) as u32;
        let disp_h = ild.display_h.round().max(1.0) as u32;
        let src = if disp_w < orig_w || disp_h < orig_h {
            ild.image.resize_exact(disp_w, disp_h, image::imageops::FilterType::Lanczos3).to_rgba8()
        } else {
            ild.image.to_rgba8()
        };
        let (src_w, src_h) = (src.width(), src.height());

        let (cx, cy) = ild.center_canvas();
        let angle_rad = ild.rotation.to_radians();
        let (cos_a, sin_a) = (angle_rad.cos(), angle_rad.sin());
        let corners = [(ild.canvas_x, ild.canvas_y), (ild.canvas_x + ild.display_w, ild.canvas_y),
                       (ild.canvas_x + ild.display_w, ild.canvas_y + ild.display_h), (ild.canvas_x, ild.canvas_y + ild.display_h)];
        let rc: Vec<(f32, f32)> = corners.iter().map(|&(px, py)| {
            let (dx, dy) = (px - cx, py - cy);
            (cx + dx * cos_a - dy * sin_a, cy + dx * sin_a + dy * cos_a)
        }).collect();
        let min_x = rc.iter().map(|p| p.0).fold(f32::MAX, f32::min).max(0.0) as u32;
        let max_x = (rc.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil() as u32).min(cw);
        let min_y = rc.iter().map(|p| p.1).fold(f32::MAX, f32::min).max(0.0) as u32;
        let max_y = (rc.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as u32).min(ch);
        let opacity = layer_opacity.clamp(0.0, 1.0);
        for py in min_y..max_y {
            for px in min_x..max_x {
                let (dx, dy) = (px as f32 - cx, py as f32 - cy);
                let lx = dx * cos_a + dy * sin_a + ild.display_w / 2.0;
                let ly = -dx * sin_a + dy * cos_a + ild.display_h / 2.0;
                if lx < 0.0 || ly < 0.0 || lx >= ild.display_w || ly >= ild.display_h { continue; }
                let mut sx = lx / ild.display_w * src_w as f32;
                let mut sy = ly / ild.display_h * src_h as f32;
                if ild.flip_h { sx = src_w as f32 - 1.0 - sx; }
                if ild.flip_v { sy = src_h as f32 - 1.0 - sy; }
                let sp = Self::bicubic_sample_rgba(&src, sx.clamp(0.0, src_w as f32 - 1.0001), sy.clamp(0.0, src_h as f32 - 1.0001), src_w, src_h);
                let dst = composite.get_pixel(px, py).0;
                composite.put_pixel(px, py, Rgba(blend_pixels_linear(dst, sp, opacity, blend_mode)));
            }
        }
    }

    #[inline]
    pub(super) fn bicubic_sample_rgba(src: &ImageBuffer<Rgba<u8>, Vec<u8>>, sx: f32, sy: f32, w: u32, h: u32) -> [u8; 4] {
        let ix = sx.floor() as i32;
        let iy = sy.floor() as i32;
        let fx = sx - ix as f32;
        let fy = sy - iy as f32;
        let wt = |t: f32| -> f32 {
            let t = t.abs();
            if t < 1.0 { 1.5*t*t*t - 2.5*t*t + 1.0 }
            else if t < 2.0 { -0.5*t*t*t + 2.5*t*t - 4.0*t + 2.0 }
            else { 0.0 }
        };

        let wx = [wt(1.0 + fx), wt(fx), wt(1.0 - fx), wt(2.0 - fx)];
        let wy = [wt(1.0 + fy), wt(fy), wt(1.0 - fy), wt(2.0 - fy)];
        let get = |xi: i32, yi: i32| -> [f32; 4] {
            let xi = xi.clamp(0, w as i32 - 1) as u32;
            let yi = yi.clamp(0, h as i32 - 1) as u32;
            let p = src.get_pixel(xi, yi).0;
            [p[0] as f32/255.0, p[1] as f32/255.0, p[2] as f32/255.0, p[3] as f32/255.0]
        };
        let mut out = [0.0f32; 4];
        for dy in 0..4i32 {
            for dx in 0..4i32 {
                let p = get(ix - 1 + dx, iy - 1 + dy);
                let w = wx[dx as usize] * wy[dy as usize];
                for c in 0..4 { out[c] += p[c] * w; }
            }
        }
        [
            (out[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            (out[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            (out[2].clamp(0.0, 1.0) * 255.0).round() as u8,
            (out[3].clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }


    fn insert_above_active(&self) -> usize {
        self.layers.iter().rposition(|l| l.id == self.active_layer_id).map(|pos| pos + 1).unwrap_or(self.layers.len())
    }

    pub(super) fn new_raster_layer(&mut self) {
        let (w, h) = match &self.image { Some(img) => (img.width(), img.height()), None => return, };
        self.push_undo();
        let id = self.next_layer_id; self.next_layer_id += 1;
        let layer = ImageLayer {
            id, name: format!("Layer {}", id),
            opacity: 1.0, visible: true, locked: false,
            blend_mode: BlendMode::Normal,
            kind: LayerKind::Raster, linked_text_id: None, linked_image_id: None,
        };
        let pos = self.insert_above_active();
        self.layers.insert(pos, layer);
        self.layer_images.insert(id, DynamicImage::ImageRgba8(ImageBuffer::from_pixel(w, h, Rgba([0u8, 0, 0, 0])),));
        self.active_layer_id = id;
        self.raster_layer_texture_dirty.insert(id);
        self.composite_dirty = true;
        self.dirty = true;
    }

    pub(super) fn duplicate_active_layer(&mut self) {
        let (src_kind, src_opacity, src_blend, src_name, src_text_id, src_image_id, src_locked) =
            match self.layers.iter().find(|l| l.id == self.active_layer_id) {
                Some(l) => (l.kind, l.opacity, l.blend_mode, l.name.clone(), l.linked_text_id, l.linked_image_id, l.locked),
                None => return,
            };
        self.push_undo();
        let new_id = self.next_layer_id; self.next_layer_id += 1;
        let src_img = match src_kind {
            LayerKind::Background => self.image.clone(),
            LayerKind::Raster => self.layer_images.get(&self.active_layer_id).cloned(),
            LayerKind::Text | LayerKind::Image => None,
        };

        let mut new_text_id = None;
        if src_kind == LayerKind::Text {
            if let Some(tid) = src_text_id {
                if let Some(tl) = self.text_layers.iter().find(|t| t.id == tid).cloned() {
                    let ntid = self.next_text_id; self.next_text_id += 1;
                    let mut new_tl = tl; new_tl.id = ntid;
                    self.text_layers.push(new_tl);
                    new_text_id = Some(ntid);
                }
            }
        }

        let mut new_image_id = None;
        if src_kind == LayerKind::Image {
            if let Some(iid) = src_image_id {
                if let Some(ild) = self.image_layer_data.get(&iid).cloned() {
                    let niid = self.next_image_layer_id; self.next_image_layer_id += 1;
                    let mut new_ild = ild; new_ild.id = niid;
                    self.image_layer_texture_dirty.insert(niid);
                    self.image_layer_data.insert(niid, new_ild);
                    new_image_id = Some(niid);
                }
            }
        }

        let new_kind = if src_kind == LayerKind::Background { LayerKind::Raster } else { src_kind };
        let new_name = if src_kind == LayerKind::Background { format!("{} (copy)", src_name) } else { format!("{} copy", src_name) };
        let new_layer = ImageLayer {
            id: new_id, name: new_name,
            opacity: src_opacity, visible: true, locked: src_locked,
            blend_mode: src_blend,
            kind: new_kind, linked_text_id: new_text_id, linked_image_id: new_image_id,
        };
        let pos = self.insert_above_active();
        self.layers.insert(pos, new_layer);
        if let Some(img) = src_img {
            self.layer_images.insert(new_id, img);
            if matches!(new_kind, LayerKind::Raster) {
                self.raster_layer_texture_dirty.insert(new_id);
            }
        }
        self.active_layer_id = new_id;
        self.composite_dirty = true;
        self.dirty = true;
    }

    pub(super) fn delete_active_layer(&mut self) {
        if self.layers.len() <= 1 { return; }
        let idx = match self.layers.iter().position(|l| l.id == self.active_layer_id) {
            Some(i) => i, None => return,
        };
        self.push_undo();
        let removed = self.layers.remove(idx);
        self.layer_images.remove(&removed.id);
        if removed.kind == LayerKind::Raster {
            self.raster_layer_textures.remove(&removed.id);
            self.raster_layer_texture_dirty.remove(&removed.id);
            self.raster_layer_dirty_rects.remove(&removed.id);
        }
        if removed.kind == LayerKind::Text {
            if let Some(tid) = removed.linked_text_id {
                self.text_layers.retain(|t| t.id != tid);
            }
        }
        if removed.kind == LayerKind::Image {
            if let Some(iid) = removed.linked_image_id {
                self.image_layer_data.remove(&iid);
                if let Some(tid) = self.image_layer_textures.remove(&iid) {
                    let _ = tid;
                }
                self.image_layer_texture_dirty.remove(&iid);
                if self.selected_image_layer == Some(iid) { self.selected_image_layer = None; }
            }
        }
        let new_idx = if idx > 0 { idx - 1 } else { 0 };
        self.active_layer_id = self.layers.get(new_idx).map(|l| l.id).unwrap_or(0);
        self.composite_dirty = true;
        self.dirty = true;
    }

    pub(super) fn merge_down(&mut self) {
        let idx = match self.layers.iter().position(|l| l.id == self.active_layer_id) {
            Some(i) if i > 0 => i,
            _ => return,
        };
        let below_kind = self.layers[idx - 1].kind;
        if matches!(below_kind, LayerKind::Text | LayerKind::Image) { return; }

        self.push_undo();

        let top_id = self.layers[idx].id;
        let below_id = self.layers[idx - 1].id;
        let top_img: Option<DynamicImage> = match self.layers[idx].kind {
            LayerKind::Background => self.image.clone(),
            LayerKind::Raster => self.layer_images.get(&top_id).cloned(),
            LayerKind::Text | LayerKind::Image => None,
        };
        let bot_img: Option<DynamicImage> = match below_kind {
            LayerKind::Background => self.image.clone(),
            LayerKind::Raster => self.layer_images.get(&below_id).cloned(),
            _ => None,
        };

        if let (Some(top), Some(bot)) = (top_img, bot_img) {
            let top_opacity = self.layers[idx].opacity;
            let top_mode = self.layers[idx].blend_mode;
            let (w, h) = (bot.width(), bot.height());
            let mut result = bot.to_rgba8();
            let top_rgba = top.to_rgba8();

            for y in 0..h {
                for x in 0..w {
                    let out = blend_pixels_u8(result.get_pixel(x, y).0, top_rgba.get_pixel(x, y).0, top_opacity, top_mode);
                    result.put_pixel(x, y, Rgba(out));
                }
            }

            let merged = DynamicImage::ImageRgba8(result);
            match below_kind {
                LayerKind::Background => { self.image = Some(merged); }
                LayerKind::Raster => { self.layer_images.insert(below_id, merged); }
                _ => {}
            }
        }
        self.layers.remove(idx);
        self.active_layer_id = below_id;
        self.raster_layer_textures.remove(&top_id);
        self.raster_layer_texture_dirty.remove(&top_id);
        self.raster_layer_dirty_rects.remove(&top_id);
        if below_kind == LayerKind::Raster {
            self.raster_layer_texture_dirty.insert(below_id);
            self.raster_layer_dirty_rects.remove(&below_id);
        }
        self.composite_dirty = true;
        self.dirty = true;
    }

    pub(super) fn flatten_all_layers(&mut self) {
        if let Some(composite) = self.composite_all_layers() {
            self.push_undo();
            self.image = Some(composite);
            self.layer_images.clear();
            self.text_layers.clear();
            self.image_layer_data.clear();
            self.image_layer_texture_dirty.clear();
            self.raster_layer_textures.clear();
            self.raster_layer_texture_dirty.clear();
            self.raster_layer_dirty_rects.clear();
            self.selected_image_layer = None;
            self.layers = vec![ImageLayer {
                id: 0, name: "Background".to_string(),
                opacity: 1.0, visible: true, locked: false,
                blend_mode: BlendMode::Normal,
                kind: LayerKind::Background, linked_text_id: None, linked_image_id: None,
            }];
            self.active_layer_id = 0;
            self.texture_dirty = true;
            self.composite_dirty = false;
            self.dirty = true;
        }
    }

    pub(super) fn ensure_layer_entry_for_text(&mut self, text_id: u64) {
        if !self.layers.iter().any(|l| l.linked_text_id == Some(text_id)) {
            let id = self.next_layer_id; self.next_layer_id += 1;
            let layer = ImageLayer {
                id, name: format!("Text {}", text_id + 1),
                opacity: 1.0, visible: true, locked: false,
                blend_mode: BlendMode::Normal,
                kind: LayerKind::Text, linked_text_id: Some(text_id), linked_image_id: None,
            };
            let pos = self.insert_above_active();
            self.layers.insert(pos, layer);
            self.active_layer_id = id;
        }
    }

    pub(super) fn insert_image_layer(&mut self, img: DynamicImage, center_on_canvas: bool) {
        let Some(bg) = &self.image else { return };
        let (cw, ch) = (bg.width() as f32, bg.height() as f32);
        let (iw, ih) = (img.width() as f32, img.height() as f32);
        let (display_w, display_h) = {
            let scale = (cw / iw).min(ch / ih).min(1.0);
            (iw * scale, ih * scale)
        };
        let (cx, cy) = if center_on_canvas {
            ((cw - display_w) / 2.0, (ch - display_h) / 2.0)
        } else { (0.0, 0.0) };
        self.push_undo();
        let iid = self.next_image_layer_id; self.next_image_layer_id += 1;
        let lid = self.next_layer_id; self.next_layer_id += 1;
        let img = DynamicImage::ImageRgba8(img.to_rgba8());
        let ild = ImageLayerData { id: iid, image: img, canvas_x: cx, canvas_y: cy, display_w, display_h, rotation: 0.0, flip_h: false, flip_v: false };
        self.image_layer_data.insert(iid, ild);
        self.image_layer_texture_dirty.insert(iid);
        let layer = ImageLayer { id: lid, name: format!("Image {}", iid + 1), opacity: 1.0, visible: true, locked: false, blend_mode: BlendMode::Normal, kind: LayerKind::Image, linked_text_id: None, linked_image_id: Some(iid) };
        let pos = self.insert_above_active();
        self.layers.insert(pos, layer);
        self.active_layer_id = lid;
        self.selected_image_layer = Some(iid);
        self.selected_text = None;
        self.editing_text = false;
        self.text_drag = None;
        self.tool = Tool::Pan;
        self.composite_dirty = true;
        self.dirty = true;
    }

    pub(super) fn image_layer_for_active(&self) -> Option<u64> {
        let layer = self.layers.iter().find(|l| l.id == self.active_layer_id)?;
        if layer.kind == LayerKind::Image { layer.linked_image_id } else { None }
    }

    pub(super) fn ensure_image_layer_textures(&mut self, ctx: &egui::Context) {
        const MAX_TEX_DIM: u32 = 4096;
        let dirty_ids: Vec<u64> = self.image_layer_texture_dirty.drain().collect();
        for iid in dirty_ids {
            let stroke_rect = self.image_layer_stroke_rects.remove(&iid);
            if let Some(ild) = self.image_layer_data.get(&iid) {
                let (orig_w, orig_h) = (ild.image.width(), ild.image.height());
                let scale = (MAX_TEX_DIM as f32 / orig_w as f32).min(MAX_TEX_DIM as f32 / orig_h as f32).min(1.0);
                let opts = egui::TextureOptions { magnification: egui::TextureFilter::Linear, minification: egui::TextureFilter::Linear, ..Default::default() };
                
                let orig_tex_w = (orig_w as f32 * scale) as usize;
                let orig_tex_h = (orig_h as f32 * scale) as usize;

                if let (Some([dr_x0, dr_y0, dr_x1, dr_y1]), Some(&existing)) = (stroke_rect, self.image_layer_textures.get(&iid)) {
                    let nx0 = dr_x0.min(orig_w); let ny0 = dr_y0.min(orig_h);
                    let nx1 = dr_x1.min(orig_w); let ny1 = dr_y1.min(orig_h);
                    if nx1 > nx0 && ny1 > ny0 {
                        let tx0 = ((nx0 as f32 * scale) as usize).min(orig_tex_w.saturating_sub(1));
                        let ty0 = ((ny0 as f32 * scale) as usize).min(orig_tex_h.saturating_sub(1));
                        let mut tw = ((nx1 - nx0) as f32 * scale).ceil() as usize + 1;
                        let mut th = ((ny1 - ny0) as f32 * scale).ceil() as usize + 1;
                        tw = tw.min(orig_tex_w - tx0).min(MAX_TEX_DIM as usize);
                        th = th.min(orig_tex_h - ty0).min(MAX_TEX_DIM as usize);

                        if tw > 0 && th > 0 {
                            let sub = ild.image.crop_imm(nx0, ny0, nx1 - nx0, ny1 - ny0);
                            let sub_rgba = if scale < 1.0 {
                                sub.resize_exact(tw as u32, th as u32, image::imageops::FilterType::Lanczos3).to_rgba8()
                            } else { sub.to_rgba8() };
                            
                            let (sw, sh) = (sub_rgba.width() as usize, sub_rgba.height() as usize);
                            if sw > 0 && sh > 0 {
                                let safe_sw = sw.min(orig_tex_w - tx0);
                                let safe_sh = sh.min(orig_tex_h - ty0);
                                let final_rgba = if safe_sw != sw || safe_sh != sh {
                                    let cropped = image::DynamicImage::ImageRgba8(sub_rgba).crop_imm(0, 0, safe_sw as u32, safe_sh as u32);
                                    cropped.to_rgba8()
                                } else {
                                    sub_rgba
                                };
                                let img = egui::ColorImage::from_rgba_unmultiplied([safe_sw, safe_sh], final_rgba.as_raw());
                                ctx.tex_manager().write().set(existing, egui::epaint::ImageDelta::partial([tx0, ty0], img, opts));
                                continue;
                            }
                        }
                    }
                }
                
                let tw = orig_tex_w as u32;
                let th = orig_tex_h as u32;
                let rgba = if scale < 1.0 {
                    ild.image.resize_exact(tw, th, image::imageops::FilterType::Triangle).to_rgba8()
                } else { ild.image.to_rgba8() };
                
                let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                let img = egui::ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());
                if let Some(&existing) = self.image_layer_textures.get(&iid) {
                    ctx.tex_manager().write().set(existing, egui::epaint::ImageDelta::full(img, opts));
                } else {
                    let tid = ctx.tex_manager().write().alloc(format!("img_layer_{}", iid), img.into(), opts);
                    self.image_layer_textures.insert(iid, tid);
                }
            }
        }
    }

    pub(super) fn ensure_raster_layer_textures(&mut self, ctx: &egui::Context) {
        let dirty_ids: Vec<u64> = self.raster_layer_texture_dirty.drain().collect();
        if dirty_ids.is_empty() { return; }
        let linear_opts = egui::TextureOptions {
            magnification: egui::TextureFilter::Linear,
            minification: egui::TextureFilter::Linear,
            ..Default::default()
        };
        for id in dirty_ids {
            let img = match self.layer_images.get(&id) { Some(i) => i, None => continue };
            let dirty_rect = self.raster_layer_dirty_rects.remove(&id);
            if let (Some(tex_id), Some([rx0, ry0, rx1, ry1])) = (self.raster_layer_textures.get(&id).copied(), dirty_rect) {
                let rgba_owned;
                let rgba: &ImageBuffer<Rgba<u8>, Vec<u8>> = match img {
                    DynamicImage::ImageRgba8(b) => b,
                    _ => { rgba_owned = img.to_rgba8(); &rgba_owned }
                };
                let (iw, ih) = (rgba.width(), rgba.height());
                let (x0, y0, x1, y1) = (rx0.min(iw), ry0.min(ih), rx1.min(iw), ry1.min(ih));
                if x0 < x1 && y0 < y1 {
                    let (pw, ph) = ((x1 - x0) as usize, (y1 - y0) as usize);
                    let pixels: Vec<egui::Color32> = (y0..y1).flat_map(|y| (x0..x1).map(move |x| {
                        let p = unsafe { rgba.unsafe_get_pixel(x, y) }.0;
                        egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])
                    })).collect();
                    ctx.tex_manager().write().set(tex_id, egui::epaint::ImageDelta::partial(
                        [x0 as usize, y0 as usize],
                        egui::ColorImage { size: [pw, ph], source_size: egui::vec2(pw as f32, ph as f32), pixels },
                        linear_opts,
                    ));
                    continue;
                }
            }
            let rgba_owned;
            let rgba: &ImageBuffer<Rgba<u8>, Vec<u8>> = match img {
                DynamicImage::ImageRgba8(b) => b,
                _ => { rgba_owned = img.to_rgba8(); &rgba_owned }
            };
            let (w, h) = (rgba.width() as usize, rgba.height() as usize);
            let pixels: Vec<egui::Color32> = rgba.pixels().map(|p| {
                egui::Color32::from_rgba_unmultiplied(p.0[0], p.0[1], p.0[2], p.0[3])
            }).collect();
            let ci = egui::ColorImage { size: [w, h], source_size: egui::vec2(w as f32, h as f32), pixels };
            if let Some(&existing) = self.raster_layer_textures.get(&id) {
                ctx.tex_manager().write().set(existing, egui::epaint::ImageDelta::full(ci, linear_opts));
            } else {
                let tid = ctx.tex_manager().write().alloc(format!("raster_layer_{id}").into(), ci.into(), linear_opts);
                self.raster_layer_textures.insert(id, tid);
            }
        }
        let live_ids: std::collections::HashSet<u64> = self.layers.iter().filter(|l| l.kind == LayerKind::Raster)
            .map(|l| l.id).collect();
        self.raster_layer_textures.retain(|id, _| live_ids.contains(id));
    }

    pub(super) fn image_layer_transform_handles(&self) -> Option<TransformHandleSet> {
        let iid = self.selected_image_layer?;
        let ild = self.image_layer_data.get(&iid)?;
        let canvas = self.canvas_rect?;
        let (img_w, img_h) = self.image.as_ref().map(|i| (i.width() as f32, i.height() as f32))?;
        let rect = ild.screen_rect(img_w, img_h, canvas, self.zoom, self.pan);
        Some(TransformHandleSet::with_rotation(rect, ild.rotation.to_radians()))
    }

    pub(super) fn flip_image_layer_h(&mut self) {
        if let Some(iid) = self.image_layer_for_active() {
            if let Some(ild) = self.image_layer_data.get_mut(&iid) {
                ild.flip_h = !ild.flip_h;
                self.composite_dirty = true; self.dirty = true;
            }
        }
    }

    pub(super) fn flip_image_layer_v(&mut self) {
        if let Some(iid) = self.image_layer_for_active() {
            if let Some(ild) = self.image_layer_data.get_mut(&iid) {
                ild.flip_v = !ild.flip_v;
                self.composite_dirty = true; self.dirty = true;
            }
        }
    }

    pub(super) fn reset_image_layer_size(&mut self) {
        if let Some(iid) = self.image_layer_for_active() {
            if let Some(ild) = self.image_layer_data.get_mut(&iid) {
                ild.display_w = ild.image.width() as f32;
                ild.display_h = ild.image.height() as f32;
                self.composite_dirty = true; self.dirty = true;
            }
        }
    }

    pub(super) fn fit_image_layer_to_canvas(&mut self) {
        let bg_size = self.image.as_ref().map(|i| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
        if let Some(iid) = self.image_layer_for_active() {
            if let Some(ild) = self.image_layer_data.get_mut(&iid) {
                let scale = (bg_size.0 / ild.orig_w() as f32).min(bg_size.1 / ild.orig_h() as f32);
                ild.display_w = ild.orig_w() as f32 * scale;
                ild.display_h = ild.orig_h() as f32 * scale;
                ild.canvas_x = (bg_size.0 - ild.display_w) / 2.0;
                ild.canvas_y = (bg_size.1 - ild.display_h) / 2.0;
                self.composite_dirty = true; self.dirty = true;
            }
        }
    }

    pub(super) fn rasterize_image_layer(&mut self) {
        let (cw, ch) = match &self.image { Some(i) => (i.width(), i.height()), None => return };
        let iid = match self.image_layer_for_active() { Some(id) => id, None => return };
        let layer_idx = match self.layers.iter().position(|l| l.id == self.active_layer_id) { Some(i) => i, None => return };
        let opacity = self.layers[layer_idx].opacity;
        let blend = self.layers[layer_idx].blend_mode;
        let ild_clone = match self.image_layer_data.get(&iid) { Some(d) => d.clone(), None => return };
        self.push_undo();
        let mut raster: image::ImageBuffer<Rgba<u8>, Vec<u8>> = image::ImageBuffer::from_pixel(cw, ch, Rgba([0, 0, 0, 0]));
        Self::stamp_image_layer(&mut raster, &ild_clone, opacity, blend);
        let new_img = DynamicImage::ImageRgba8(raster);
        let new_lid = self.next_layer_id; self.next_layer_id += 1;
        self.layer_images.insert(new_lid, new_img);
        self.image_layer_data.remove(&iid);
        self.image_layer_texture_dirty.remove(&iid);
        if let Some(old_tex) = self.image_layer_textures.remove(&iid) { let _ = old_tex; }
        if self.selected_image_layer == Some(iid) { self.selected_image_layer = None; }
        let name = self.layers[layer_idx].name.clone();
        self.layers[layer_idx] = ImageLayer { id: new_lid, name, opacity: 1.0, visible: true, locked: false, blend_mode: BlendMode::Normal, kind: LayerKind::Raster, linked_text_id: None, linked_image_id: None };
        self.active_layer_id = new_lid;
        self.composite_dirty = true; self.dirty = true;
    }

    pub(super) fn move_layer_up(&mut self) {
        if let Some(idx) = self.layers.iter().position(|l| l.id == self.active_layer_id) {
            if idx + 1 < self.layers.len() {
                self.layers.swap(idx, idx + 1);
                self.composite_dirty = true;
                self.dirty = true;
            }
        }
    }

    pub(super) fn move_layer_down(&mut self) {
        if let Some(idx) = self.layers.iter().position(|l| l.id == self.active_layer_id) {
            if idx > 1 {
                self.layers.swap(idx, idx - 1);
                self.composite_dirty = true;
                self.dirty = true;
            }
        }
    }

    pub(super) fn screen_to_image(&self, screen_pos: egui::Pos2) -> Option<(u32, u32)> {
        let canvas: egui::Rect = self.canvas_rect?;
        let img: &DynamicImage = self.image.as_ref()?;
        let (img_w, img_h) = (img.width() as f32, img.height() as f32);
        let sw: f32 = img_w * self.zoom;
        let sh: f32 = img_h * self.zoom;
        let ox: f32 = canvas.center().x - sw / 2.0 + self.pan.x;
        let oy: f32 = canvas.center().y - sh / 2.0 + self.pan.y;
        let rx: f32 = (screen_pos.x - ox) / self.zoom;
        let ry: f32 = (screen_pos.y - oy) / self.zoom;
        if rx < 0.0 || ry < 0.0 || rx >= img_w || ry >= img_h { return Option::None; }
        Some((rx as u32, ry as u32))
    }

    pub(super) fn ensure_checker_texture(&mut self, ctx: &egui::Context) -> egui::TextureId {
        let is_dark = ctx.style().visuals.dark_mode;
        if let Some(tid) = self.checker_texture {
            if self.checker_texture_dark == is_dark { return tid; }
            ctx.tex_manager().write().free(tid);
        }
        let sq: usize = 16;
        let sz = sq * 2;
        let (light, dark) = if is_dark {
            ([55u8, 55, 55, 255], [40u8, 40, 40, 255])
        } else {
            ([220u8, 220, 220, 255], [200u8, 200, 200, 255])
        };
        let mut pixels: Vec<u8> = Vec::with_capacity(sz * sz * 4);
        for row in 0..sz {
            for col in 0..sz {
                let c = if (row / sq + col / sq) % 2 == 0 { light } else { dark };
                pixels.extend_from_slice(&c);
            }
        }
        let img = egui::ColorImage::from_rgba_unmultiplied([sz, sz], &pixels);
        let opts = egui::TextureOptions { wrap_mode: egui::TextureWrapMode::Repeat, ..Default::default() };
        let tid = ctx.tex_manager().write().alloc("checker_bg".into(), img.into(), opts);
        self.checker_texture = Some(tid);
        self.checker_texture_dark = is_dark;
        tid
    }

    pub(super) fn image_to_screen(&self, ix: f32, iy: f32) -> egui::Pos2 {
        let canvas: egui::Rect = self.canvas_rect.unwrap_or(egui::Rect::NOTHING);
        let (img_w, img_h) = self.image.as_ref()
            .map(|i: &DynamicImage| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
        let ox: f32 = canvas.center().x - img_w * self.zoom / 2.0 + self.pan.x;
        let oy: f32 = canvas.center().y - img_h * self.zoom / 2.0 + self.pan.y;
        egui::pos2(ox + ix * self.zoom, oy + iy * self.zoom)
    }

    pub(super) fn fit_image(&mut self) {
        if let (Some(img), Some(canvas)) = (&self.image, self.canvas_rect) {
            let sx: f32 = canvas.width() / img.width() as f32;
            let sy: f32 = canvas.height() / img.height() as f32;
            self.zoom = sx.min(sy).min(1.0).max(0.01);
            self.pan = egui::Vec2::ZERO;
        }
    }

    pub(super) fn new_image(&mut self, w: u32, h: u32) {
        self.push_undo();
        self.image = Some(DynamicImage::ImageRgba8(ImageBuffer::from_pixel(w, h, Rgba([255, 255, 255, 255]))));
        self.resize_w = w; self.resize_h = h;
        self.texture_dirty = true;
        self.composite_dirty = true;
        self.file_path = None;
        self.dirty = true; self.fit_on_next_frame = true;
    }

    pub(super) fn ensure_texture(&mut self, ctx: &egui::Context) {
        if self.composite_dirty {
            let partial = self.composite_dirty_rect.take();
            let tex_opt = self.texture;
            if let (Some(tex_id), Some([cx0, cy0, cx1, cy1])) = (tex_opt, partial) {
                let all_rgba8 = self.image.as_ref().map_or(true, |i| matches!(i, DynamicImage::ImageRgba8(_)))
                    && self.layer_images.values().all(|i| matches!(i, DynamicImage::ImageRgba8(_)));
                if all_rgba8 && (self.is_dragging || !self.has_visible_text_in_rect(cx0, cy0, cx1, cy1)) {
                    self.upload_partial_composite(ctx, tex_id, cx0, cy0, cx1, cy1);
                    self.composite_dirty = false;
                    self.texture_dirty = false;
                    self.texture_dirty_rect = None;
                    return;
                }
            }
            self.composite_dirty_rect = None;
            if let Some(composite) = self.composite_for_display() {
                let rgba = composite.to_rgba8();
                let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                let pixels: Vec<egui::Color32> = rgba.pixels().map(|p| egui::Color32::from_rgba_unmultiplied(p.0[0], p.0[1], p.0[2], p.0[3])).collect();
                let color_image = egui::ColorImage { size: [w, h], source_size: egui::vec2(w as f32, h as f32), pixels };
                let linear_opts = egui::TextureOptions { magnification: egui::TextureFilter::Linear, minification: egui::TextureFilter::Linear, ..Default::default() };
                if let Some(tid) = self.texture {
                    ctx.tex_manager().write().set(tid, egui::epaint::ImageDelta::full(color_image, linear_opts));
                } else {
                    self.texture = Some(ctx.tex_manager().write().alloc("image_editor_img".into(), color_image.into(), linear_opts));
                }
            }
            self.composite_dirty = false;
            self.texture_dirty = false;
            self.texture_dirty_rect = None;
            return;
        }

        if !self.texture_dirty { return; }

        let img: &DynamicImage = match &self.image {
            Some(i) => i,
            None => { self.texture_dirty = false; self.texture_dirty_rect = None; return; }
        };

        if let (Some(tex_id), Some([rx0, ry0, rx1, ry1])) = (self.texture, self.texture_dirty_rect) {
            let rgba_owned;
            let rgba: &ImageBuffer<Rgba<u8>, Vec<u8>> = match img {
                DynamicImage::ImageRgba8(b) => b,
                _ => { rgba_owned = img.to_rgba8(); &rgba_owned }
            };
            let iw: u32 = rgba.width(); let ih: u32 = rgba.height();
            let x0: u32 = rx0.min(iw); let y0: u32 = ry0.min(ih);
            let x1: u32 = rx1.min(iw); let y1: u32 = ry1.min(ih);

            if x0 < x1 && y0 < y1 {
                let pw: usize = (x1 - x0) as usize;
                let ph: usize = (y1 - y0) as usize;
                let mut pixels: Vec<egui::Color32> = Vec::with_capacity(pw * ph);
                for y in y0..y1 {
                    for x in x0..x1 {
                        let p: &Rgba<u8> = rgba.get_pixel(x, y);
                        pixels.push(egui::Color32::from_rgba_unmultiplied(p.0[0], p.0[1], p.0[2], p.0[3]));
                    }
                }
                let partial: egui::ColorImage = egui::ColorImage {
                    size: [pw, ph],
                    source_size: egui::vec2(pw as f32, ph as f32),
                    pixels,
                };
                ctx.tex_manager().write().set(
                    tex_id,
                    egui::epaint::ImageDelta::partial([x0 as usize, y0 as usize], partial, egui::TextureOptions { magnification: egui::TextureFilter::Linear, minification: egui::TextureFilter::Linear, ..Default::default() }),
                );
                self.texture_dirty = false;
                self.texture_dirty_rect = None;
                return;
            }
        }
        let rgba_owned;
        let rgba: &ImageBuffer<Rgba<u8>, Vec<u8>> = match img {
            DynamicImage::ImageRgba8(b) => b,
            _ => { rgba_owned = img.to_rgba8(); &rgba_owned }
        };
        let (w, h): (usize, usize) = (rgba.width() as usize, rgba.height() as usize);
        let color_image: egui::ColorImage = egui::ColorImage {
            size: [w, h],
            source_size: egui::vec2(w as f32, h as f32),
            pixels: rgba.pixels().map(|p: &Rgba<u8>| egui::Color32::from_rgba_unmultiplied(p.0[0], p.0[1], p.0[2], p.0[3])).collect(),
        };
        let linear_opts = egui::TextureOptions { magnification: egui::TextureFilter::Linear, minification: egui::TextureFilter::Linear, ..Default::default() };
        if let Some(texture_id) = self.texture {
            ctx.tex_manager().write().set(texture_id, egui::epaint::ImageDelta::full(color_image, linear_opts));
        } else {
            self.texture = Some(ctx.tex_manager().write().alloc("image_editor_img".into(), color_image.into(), linear_opts));
        }
        self.texture_dirty = false;
        self.texture_dirty_rect = None;
    }

    fn has_visible_text_in_rect(&self, x0: u32, y0: u32, x1: u32, y1: u32) -> bool {
        self.text_layers.iter().any(|tl| {
            let bw = tl.box_width.unwrap_or_else(|| tl.auto_width(1.0)).ceil() as u32 + 1;
            let bh = tl.box_height.unwrap_or_else(|| tl.auto_height(1.0)).ceil() as u32 + 1;
            let (tx0, ty0) = (tl.img_x.max(0.0) as u32, tl.img_y.max(0.0) as u32);
            tx0 < x1 && tx0.saturating_add(bw) > x0 && ty0 < y1 && ty0.saturating_add(bh) > y0
        })
    }

    fn upload_partial_composite(&self, ctx: &egui::Context, tex_id: egui::TextureId, x0: u32, y0: u32, x1: u32, y1: u32) {
        let bg = match &self.image { Some(i) => i, None => return };
        let (iw, ih) = (bg.width(), bg.height());
        let x0 = x0.min(iw); let y0 = y0.min(ih);
        let x1 = x1.min(iw); let y1 = y1.min(ih);
        if x0 >= x1 || y0 >= y1 { return; }
        let pw = (x1 - x0) as usize; let ph = (y1 - y0) as usize;
        let mut out: Vec<[f32; 4]> = vec![[0.0f32; 4]; pw * ph];
        for layer in &self.layers {
            if !layer.visible { continue; }
            match layer.kind {
                LayerKind::Text | LayerKind::Image => continue,
                LayerKind::Background | LayerKind::Raster => {
                    let src_buf: &ImageBuffer<Rgba<u8>, Vec<u8>> = match layer.kind {
                        LayerKind::Background => match bg { DynamicImage::ImageRgba8(b) => b, _ => continue },
                        LayerKind::Raster => match self.layer_images.get(&layer.id) { Some(DynamicImage::ImageRgba8(b)) => b, _ => continue },
                        _ => unreachable!(),
                    };
                    if src_buf.width() < x1 || src_buf.height() < y1 { continue; }
                    let opacity = layer.opacity; let mode = layer.blend_mode;
                    for py in y0..y1 {
                        for px in x0..x1 {
                            let s = unsafe { src_buf.unsafe_get_pixel(px, py) }.0;
                            if (s[3] as f32 / 255.0) * opacity < 1e-6 { continue; }
                            let idx = (py - y0) as usize * pw + (px - x0) as usize;
                            let d = out[idx];
                            let d_u8 = [(d[0]*255.0) as u8, (d[1]*255.0) as u8, (d[2]*255.0) as u8, (d[3]*255.0) as u8];
                            let r = blend_pixels_u8(d_u8, s, opacity, mode);
                            out[idx] = [r[0] as f32/255.0, r[1] as f32/255.0, r[2] as f32/255.0, r[3] as f32/255.0];
                        }
                    }
                }
            }
        }
        let pixels: Vec<egui::Color32> = out.iter().map(|p| egui::Color32::from_rgba_unmultiplied(
            (p[0]*255.0).clamp(0.0,255.0) as u8, (p[1]*255.0).clamp(0.0,255.0) as u8,
            (p[2]*255.0).clamp(0.0,255.0) as u8, (p[3]*255.0).clamp(0.0,255.0) as u8,
        )).collect();
        ctx.tex_manager().write().set(tex_id, egui::epaint::ImageDelta::partial(
            [x0 as usize, y0 as usize],
            egui::ColorImage { size: [pw, ph], source_size: egui::vec2(pw as f32, ph as f32), pixels },
            egui::TextureOptions { magnification: egui::TextureFilter::Linear, minification: egui::TextureFilter::Linear, ..Default::default() },
        ));
    }

    pub(super) fn kick_backdrop_compute(&mut self, active_id: u64) {
        let is_raster = self.layers.iter().find(|l| l.id == active_id)
            .map_or(false, |l| l.kind == LayerKind::Raster);
        if !is_raster { self.backdrop_cache_for = u64::MAX; *self.backdrop_cache.lock().unwrap() = None; return; }
        if self.backdrop_cache_for == active_id { return; }
        self.backdrop_cache_for = active_id;
        *self.backdrop_cache.lock().unwrap() = None;

        let mut layers_below: Vec<(u64, LayerKind, BlendMode, f32)> = Vec::new();
        for layer in &self.layers {
            if layer.id == active_id { break; }
            if layer.visible { layers_below.push((layer.id, layer.kind, layer.blend_mode, layer.opacity)); }
        }
        let bg_buf: Option<ImageBuffer<Rgba<u8>, Vec<u8>>> = self.image.as_ref().and_then(|i| match i { DynamicImage::ImageRgba8(b) => Some(b.clone()), _ => Some(i.to_rgba8()) });
        let mut raster_bufs: std::collections::HashMap<u64, ImageBuffer<Rgba<u8>, Vec<u8>>> = std::collections::HashMap::new();
        for (id, kind, _, _) in &layers_below {
            if *kind == LayerKind::Raster {
                if let Some(DynamicImage::ImageRgba8(b)) = self.layer_images.get(id) { raster_bufs.insert(*id, b.clone()); }
            }
        }
        let sink = Arc::clone(&self.backdrop_cache);
        std::thread::spawn(move || {
            let bg = match bg_buf { Some(b) => b, None => return };
            let (w, h) = (bg.width(), bg.height());
            let mut result: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(w, h, Rgba([0u8, 0, 0, 0]));
            for (id, kind, mode, opacity) in &layers_below {
                let src: &ImageBuffer<Rgba<u8>, Vec<u8>> = match kind {
                    LayerKind::Background => &bg,
                    LayerKind::Raster => match raster_bufs.get(id) { Some(b) => b, None => continue },
                    LayerKind::Text | LayerKind::Image => continue,
                };
                for y in 0..h {
                    for x in 0..w {
                        let s = unsafe { src.unsafe_get_pixel(x, y) }.0;
                        let d = unsafe { result.unsafe_get_pixel(x, y) }.0;
                        let out = super::ie_helpers::blend_pixels_u8(d, s, *opacity, *mode);
                        unsafe { result.unsafe_put_pixel(x, y, Rgba(out)); }
                    }
                }
            }
            *sink.lock().unwrap() = Some(result);
        });
    }

    #[inline]
    pub(super) fn expand_dirty_rect(&mut self, x0: u32, y0: u32, x1: u32, y1: u32) {
        match self.texture_dirty_rect {
            None => {
                if !self.texture_dirty {
                    self.texture_dirty_rect = Some([x0, y0, x1, y1]);
                }
            }
            Some(ref mut r) => {
                r[0] = r[0].min(x0);
                r[1] = r[1].min(y0);
                r[2] = r[2].max(x1);
                r[3] = r[3].max(y1);
            }
        }
    }

    pub(super) fn check_filter_completion(&mut self) {
        if !self.is_processing { return; }

        if *self.filter_progress.lock().unwrap() >= 1.0 {
            if let Some(result) = self.pending_filter_result.lock().unwrap().take() {
                let target_id = self.filter_target_layer_id;
                let kind = self.layers.iter().find(|l| l.id == target_id)
                    .map(|l| l.kind).unwrap_or(LayerKind::Background);
                match kind {
                    LayerKind::Background => {
                        self.resize_w = result.width(); self.resize_h = result.height();
                        self.image = Some(result);
                    }
                    LayerKind::Raster => {
                        self.layer_images.insert(target_id, result);
                        self.raster_layer_texture_dirty.insert(target_id);
                        self.raster_layer_dirty_rects.remove(&target_id);
                    }
                    LayerKind::Text | LayerKind::Image => {}
                }
                self.texture_dirty = true;
                self.composite_dirty = true;
                self.dirty = true;
                self.is_processing = false;
                if self.processing_is_preview {
                    self.processing_is_preview = false;
                } else {
                    self.filter_panel = FilterPanel::None;
                    if self.resize_w != 0 { self.fit_on_next_frame = true; }
                }
            }
        }
    }

    pub(super) fn handle_keyboard(&mut self, ctx: &egui::Context) {
        self.process_text_input(ctx);

        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Z) { self.undo(); }
            if i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z) { self.redo(); }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Y) { self.redo(); }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::S) {
                if i.modifiers.shift { let _ = self.save_as_impl(); }
                else { let _ = self.save_impl(); }
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                self.commit_or_discard_active_text();
            }
            if i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::N) {
                self.new_raster_layer();
            }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::E) {
                self.merge_down();
            }
        });

        if !self.editing_text && ctx.memory(|m| m.focused().is_none()) {
            ctx.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, egui::Key::B) { self.commit_or_discard_active_text(); self.tool = Tool::Brush; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::E) { self.commit_or_discard_active_text(); self.tool = Tool::Eraser; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::F) { self.commit_or_discard_active_text(); self.tool = Tool::Fill; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::T) { self.tool = Tool::Text; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::D) { self.commit_or_discard_active_text(); self.tool = Tool::Eyedropper; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::C) { self.commit_or_discard_active_text(); self.tool = Tool::Crop; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::P) { self.commit_or_discard_active_text(); self.tool = Tool::Pan; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::R) { self.commit_or_discard_active_text(); self.tool = Tool::Retouch; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Num0) {
                    if let Some(c) = self.color_favorites.colors.get(9) { let mut col = *c; col.a = 255; self.color = col.to_egui(); self.hex_input = col.to_hex(); }
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                    if self.tool == Tool::Crop && self.crop_state.start.is_some() && self.crop_state.end.is_some() {
                        if self.image_layer_for_active().is_some() { self.apply_crop_to_image_layer(); }
                        else { self.push_undo(); self.apply_crop(); }
                    }
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Delete) || i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace) {
                    if self.selected_image_layer.is_some() && self.image_layer_for_active().is_some() {
                        self.delete_active_layer();
                    }
                }
                for (key, slot) in [
                    (egui::Key::Num1, 0usize), (egui::Key::Num2, 1), (egui::Key::Num3, 2),
                    (egui::Key::Num4, 3), (egui::Key::Num5, 4), (egui::Key::Num6, 5),
                    (egui::Key::Num7, 6), (egui::Key::Num8, 7), (egui::Key::Num9, 8),
                ] {
                    if i.consume_key(egui::Modifiers::NONE, key) {
                        if let Some(c) = self.color_favorites.colors.get(slot) { let mut col = *c; col.a = 255; self.color = col.to_egui(); self.hex_input = col.to_hex(); }
                    }
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Home) { self.fit_image(); }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Plus)  { self.zoom *= 1.25; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Minus) { self.zoom = (self.zoom / 1.25).max(0.01); }

                for (key, slot) in [
                    (egui::Key::Num1, 0usize), (egui::Key::Num2, 1), (egui::Key::Num3, 2),
                    (egui::Key::Num4, 3), (egui::Key::Num5, 4), (egui::Key::Num6, 5),
                    (egui::Key::Num7, 6), (egui::Key::Num8, 7), (egui::Key::Num9, 8),
                    (egui::Key::Num0, 9),
                ] {
                    if i.consume_key(egui::Modifiers::CTRL, key) {
                        if let Some(b) = self.brush_favorites.brushes.get(slot) {
                            self.brush = b.settings.clone();
                            self.brush_preview_cache_key = None;
                        }
                    }
                }
            });
        }
    }

    pub(super) fn save_impl(&mut self) -> Result<(), String> {
        let path: PathBuf = match &self.file_path {
            Some(p) => p.clone(),
            None => return self.save_as_impl(),
        };

        if self.image.is_some() {
            let composite = self.composite_all_layers().ok_or("No image to save")?;
            composite.save(&path).map_err(|e| e.to_string())?;
            self.dirty = false;
        }
        Ok(())
    }

    pub(super) fn save_as_impl(&mut self) -> Result<(), String> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "tiff", "gif"])
            .save_file()
        {
            if self.image.is_some() {
                let composite = self.composite_all_layers().ok_or("No image to save")?;
                composite.save(&path).map_err(|e| e.to_string())?;
                self.file_path = Some(path);
                self.dirty = false;
            }
            Ok(())
        } else {
            Err("Cancelled".to_string())
        }
    }
}

impl EditorModule for ImageEditor {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn get_title(&self) -> String {
        let name: &str = self.file_path.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled");
        if self.dirty { format!("{} *", name) } else { name.to_string() }
    }

    fn save(&mut self) -> Result<(), String> { self.save_impl() }
    fn save_as(&mut self) -> Result<(), String> { self.save_as_impl() }

    fn get_menu_contributions(&self) -> MenuContribution {
        let has_image: bool = self.image.is_some();
        let can_merge  = self.layers.iter().position(|l| l.id == self.active_layer_id).map(|i| i > 0).unwrap_or(false);
        MenuContribution {
            file_items: vec![
                (MenuItem { label: "Export...".to_string(), shortcut: None, enabled: has_image }, MenuAction::Export),
                (MenuItem { label: "Import to Canvas...".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Place Image".to_string())),
            ],
            edit_items: vec![
                (MenuItem { label: "Undo".to_string(), shortcut: Some("Ctrl+Z".to_string()), enabled: !self.undo_stack.is_empty() }, MenuAction::Undo),
                (MenuItem { label: "Redo".to_string(), shortcut: Some("Ctrl+Y".to_string()), enabled: !self.redo_stack.is_empty() }, MenuAction::Redo),
            ],
            view_items: vec![
                (MenuItem { label: "Zoom In".to_string(), shortcut: Some("+".to_string()), enabled: true }, MenuAction::Custom("Zoom In".to_string())),
                (MenuItem { label: "Zoom Out".to_string(), shortcut: Some("-".to_string()), enabled: true }, MenuAction::Custom("Zoom Out".to_string())),
                (MenuItem { label: "Fit".to_string(), shortcut: Some("0".to_string()), enabled: true }, MenuAction::Custom("Fit".to_string())),
                (MenuItem { label: "Separator".to_string(), shortcut: None, enabled: false }, MenuAction::None),
                (MenuItem { label: if self.show_layers_panel { "Hide Layers Panel".to_string() } else { "Show Layers Panel".to_string() }, shortcut: None, enabled: true }, MenuAction::Custom("Toggle Layers".to_string())),
            ],
            image_items: vec![
                (MenuItem { label: "Resize Canvas...".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Resize Canvas".to_string())),
                (MenuItem { label: "Separator".to_string(), shortcut: None, enabled: false }, MenuAction::None),
                (MenuItem { label: "Flip Horizontal".to_string(), shortcut: None, enabled: true }, MenuAction::Custom("Flip Horizontal".to_string())),
                (MenuItem { label: "Flip Vertical".to_string(), shortcut: None, enabled: true }, MenuAction::Custom("Flip Vertical".to_string())),
                (MenuItem { label: "Rotate CCW".to_string(), shortcut: None, enabled: true }, MenuAction::Custom("Rotate CCW".to_string())),
                (MenuItem { label: "Rotate CW".to_string(), shortcut: None, enabled: true }, MenuAction::Custom("Rotate CW".to_string())),
            ],
            filter_items: vec![
                (MenuItem { label: "Brightness/Contrast...".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("B/C".to_string())),
                (MenuItem { label: "Hue/Saturation...".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("H/S".to_string())),
                (MenuItem { label: "Blur...".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Blur".to_string())),
                (MenuItem { label: "Sharpen...".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Sharpen".to_string())),
                (MenuItem { label: "Separator".to_string(), shortcut: None, enabled: false }, MenuAction::None),
                (MenuItem { label: "Grayscale".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Gray".to_string())),
                (MenuItem { label: "Invert".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Invert".to_string())),
                (MenuItem { label: "Sepia".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Sepia".to_string())),
            ],
            layer_items: vec![
                (MenuItem { label: "New Layer".to_string(), shortcut: Some("Ctrl+Shift+N".to_string()), enabled: has_image }, MenuAction::Custom("Layer New".to_string())),
                (MenuItem { label: "Duplicate Layer".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Layer Duplicate".to_string())),
                (MenuItem { label: "Delete Layer".to_string(), shortcut: None, enabled: self.layers.len() > 1 }, MenuAction::Custom("Layer Delete".to_string())),
                (MenuItem { label: "Separator".to_string(), shortcut: None, enabled: false }, MenuAction::None),
                (MenuItem { label: "Merge Down".to_string(), shortcut: Some("Ctrl+E".to_string()), enabled: can_merge }, MenuAction::Custom("Layer Merge Down".to_string())),
                (MenuItem { label: "Flatten Image".to_string(), shortcut: None, enabled: self.layers.len() > 1 }, MenuAction::Custom("Layer Flatten".to_string())),
            ],
        }
    }

    fn handle_menu_action(&mut self, action: MenuAction) -> bool {
        match action {
            MenuAction::Undo => { self.undo(); true }
            MenuAction::Redo => { self.redo(); true }
            MenuAction::Export => { self.filter_panel = FilterPanel::Export; true }
            MenuAction::Custom(ref val) if val == "Zoom In" => { self.zoom *= 1.25; true }
            MenuAction::Custom(ref val) if val == "Zoom Out" => { self.zoom = (self.zoom / 1.25).max(0.01); true }
            MenuAction::Custom(ref val) if val == "Fit" => { self.fit_image(); true }
            MenuAction::Custom(ref val) if val == "Toggle Layers" => { self.show_layers_panel = !self.show_layers_panel; true }
            MenuAction::Custom(ref val) if val == "Flip Horizontal" => { self.push_undo(); self.apply_flip_h(); true }
            MenuAction::Custom(ref val) if val == "Flip Vertical" => { self.push_undo(); self.apply_flip_v(); true }
            MenuAction::Custom(ref val) if val == "Rotate CCW" => { self.push_undo(); self.apply_rotate_ccw(); true }
            MenuAction::Custom(ref val) if val == "Rotate CW" => { self.push_undo(); self.apply_rotate_cw(); true }
            MenuAction::Custom(ref val) if val == "Resize Canvas" => { self.filter_panel = FilterPanel::Resize; true }
            MenuAction::Custom(ref val) if val == "B/C" => { self.filter_panel = FilterPanel::BrightnessContrast; true }
            MenuAction::Custom(ref val) if val == "H/S" => { self.filter_panel = FilterPanel::HueSaturation; true }
            MenuAction::Custom(ref val) if val == "Blur" => { self.filter_panel = FilterPanel::Blur; true }
            MenuAction::Custom(ref val) if val == "Sharpen" => { self.filter_panel = FilterPanel::Sharpen; true }
            MenuAction::Custom(ref val) if val == "Gray" => { self.push_undo(); self.apply_grayscale(); true }
            MenuAction::Custom(ref val) if val == "Invert" => { self.push_undo(); self.apply_invert(); true }
            MenuAction::Custom(ref val) if val == "Sepia" => { self.push_undo(); self.apply_sepia(); true }
            MenuAction::Custom(ref val) if val == "Layer New" => { self.new_raster_layer(); true }
            MenuAction::Custom(ref val) if val == "Layer Duplicate" => { self.duplicate_active_layer(); true }
            MenuAction::Custom(ref val) if val == "Layer Delete" => { self.delete_active_layer(); true }
            MenuAction::Custom(ref val) if val == "Layer Merge Down" => { self.merge_down(); true }
            MenuAction::Custom(ref val) if val == "Layer Flatten" => { self.flatten_all_layers(); true }
            MenuAction::Custom(ref val) if val == "Place Image" => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "tiff", "tif", "gif"])
                    .pick_file()
                {
                    let img_opt = image::ImageReader::open(&path)
                        .ok()
                        .and_then(|r| r.with_guessed_format().ok())
                        .and_then(|r| r.decode().ok())
                        .or_else(|| image::open(&path).ok());
                    if let Some(img) = img_opt {
                        self.insert_image_layer(img, true);
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, _show_file_info: bool) {
        let theme: ThemeMode = if ui.visuals().dark_mode { ThemeMode::Dark } else { ThemeMode::Light };

        self.handle_keyboard(ctx);
        self.check_filter_completion();

        if self.is_processing { ctx.request_repaint(); }
        if self.image.is_none() && self.file_path.is_none() { self.new_image(800, 600); }

        self.render_toolbar(ui, theme);
        ui.add_space(4.0);
        self.render_options_bar(ui, theme);
        ui.add_space(4.0);

        if self.show_layers_panel {
            let panel_width = self.layer_panel_width;
            egui::SidePanel::right("layers_panel")
                .resizable(true)
                .default_width(panel_width)
                .min_width(180.0)
                .max_width(360.0)
                .frame(egui::Frame::new()
                    .fill(if matches!(theme, ThemeMode::Dark) { egui::Color32::from_rgb(28, 28, 32) } else { egui::Color32::from_rgb(245, 245, 248) })
                    .stroke(egui::Stroke::new(1.0, if matches!(theme, ThemeMode::Dark) { egui::Color32::from_rgb(55, 55, 65) } else { egui::Color32::from_rgb(210, 210, 220) }))
                )
                .show_inside(ui, |ui| {
                    self.render_layers_panel(ui, theme);
                });
        }

        if self.filter_panel != FilterPanel::None { self.render_filter_panel(ui, ctx, theme); }
        if self.show_color_picker { self.render_color_picker(ui, ctx, theme); }
        self.render_canvas(ui, ctx);
    }
}
