use eframe::egui;
use image::{DynamicImage, ImageBuffer, Rgba};
use crate::modules::helpers::image_export::ExportFormat;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::style::{ThemeMode};
use crate::modules::{EditorModule, MenuAction, MenuItem, MenuContribution};
use serde::{Deserialize, Serialize};
use std::fs;

pub(super) const MAX_UNDO: usize = 20;
pub(super) const MAX_COLOR_HISTORY: usize = 20;
pub(super) const HANDLE_HIT: f32 = 18.0;
pub(super) const HANDLE_VIS: f32 = 8.0;
pub(super) const ROTATE_DIST: f32 = 28.0;

pub(super) static FONT_UB_REG: &[u8] = include_bytes!("../../../assets/Ubuntu/Ubuntu-Regular.ttf");
pub(super) static FONT_UB_BLD: &[u8] = include_bytes!("../../../assets/Ubuntu/Ubuntu-Bold.ttf");
pub(super) static FONT_UB_ITL: &[u8] = include_bytes!("../../../assets/Ubuntu/Ubuntu-Italic.ttf");
pub(super) static FONT_RB_REG: &[u8] = include_bytes!("../../../assets/Roboto/Roboto-Regular.ttf");
pub(super) static FONT_RB_BLD: &[u8] = include_bytes!("../../../assets/Roboto/Roboto-Bold.ttf");
pub(super) static FONT_RB_ITL: &[u8] = include_bytes!("../../../assets/Roboto/Roboto-Italic.ttf");

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

#[derive(Serialize, Deserialize)]
pub(super) struct ColorHistory { pub colors: VecDeque<RgbaColor> }

impl ColorHistory {
    pub(super) fn new() -> Self { Self { colors: VecDeque::new() } }
    pub(super) fn load() -> Self {
        if let Ok(s) = fs::read_to_string(Self::get_config_path()) {
            if let Ok(h) = serde_json::from_str(&s) { return h; }
        }
        Self::new()
    }
    pub(super) fn save(&self) {
        let path: PathBuf = Self::get_config_path();
        if let Some(p) = path.parent() { let _ = fs::create_dir_all(p); }
        if let Ok(j) = serde_json::to_string(self) { let _ = fs::write(path, j); }
    }
    fn get_config_path() -> PathBuf {
        let mut p: PathBuf = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("universal_editor");
        p.push("color_history.json");
        p
    }
    
    pub(super) fn add_color(&mut self, color: RgbaColor) {
        if let Some(pos) = self.colors.iter().position(|c: &RgbaColor| *c == color) { self.colors.remove(pos); }
        self.colors.push_front(color);
        if self.colors.len() > MAX_COLOR_HISTORY { self.colors.pop_back(); }
        self.save();
    }
    pub(super) fn get_colors(&self) -> &VecDeque<RgbaColor> { &self.colors }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tool { Brush, Eraser, Fill, Text, Eyedropper, Crop, Pan }

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum FilterPanel { None, BrightnessContrast, HueSaturation, Blur, Sharpen, Resize, Export }

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

pub(super) struct CropState { pub start: Option<(f32, f32)>, pub end: Option<(f32, f32)> }

impl Default for CropState {
    fn default() -> Self { Self { start: None, end: None } }
}

pub struct ImageEditor {
    pub(super) image: Option<DynamicImage>,
    pub(super) texture: Option<egui::TextureId>,
    pub(super) texture_dirty: bool,
    pub(super) file_path: Option<PathBuf>,
    pub(super) dirty: bool,

    pub(super) undo_stack: VecDeque<DynamicImage>,
    pub(super) redo_stack: VecDeque<DynamicImage>,

    pub(super) zoom: f32,
    pub(super) pan: egui::Vec2,
    pub(super) fit_on_next_frame: bool,

    pub(super) tool: Tool,
    pub(super) brush_size: f32,
    pub(super) brush_opacity: f32,
    pub(super) eraser_size: f32,
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
    pub(super) export_preserve_metadata: bool,
    pub(super) export_auto_scale_ico: bool,
    pub(super) export_callback: Option<Box<dyn Fn(PathBuf) + Send + Sync>>,

    pub(super) show_color_picker: bool,
    pub(super) color_history: ColorHistory,
    pub(super) hex_input: String,
    pub(super) canvas_rect: Option<egui::Rect>,

    pub(super) filter_progress: Arc<Mutex<f32>>,
    pub(super) is_processing: bool,
    pub(super) pending_filter_result: Arc<Mutex<Option<DynamicImage>>>,
    pub(super) fonts_registered: bool,
}

impl ImageEditor {
    pub fn new() -> Self {
        Self {
            image: None, texture: None, texture_dirty: false,
            file_path: None, dirty: false,
            undo_stack: VecDeque::new(), redo_stack: VecDeque::new(),
            zoom: 1.0, pan: egui::Vec2::ZERO, fit_on_next_frame: true,
            tool: Tool::Brush,
            brush_size: 12.0, brush_opacity: 1.0, eraser_size: 20.0,
            color: egui::Color32::BLACK,
            stroke_points: Vec::new(), is_dragging: false,
            text_layers: Vec::new(), selected_text: None, editing_text: false,
            next_text_id: 0, text_font_size: 24.0,
            text_bold: false, text_italic: false, text_underline: false,
            text_font_name: "Ubuntu".to_string(),
            text_drag: None, text_cursor: 0, text_sel_anchor: None,
            crop_state: CropState::default(),
            filter_panel: FilterPanel::None,
            brightness: 0.0, contrast: 0.0, hue: 0.0, saturation: 0.0,
            blur_radius: 3.0, sharpen_amount: 1.0,
            resize_w: 0, resize_h: 0, resize_locked: true, resize_stretch: false,
            export_format: ExportFormat::Png,
            export_jpeg_quality: 90, export_preserve_metadata: true,
            export_auto_scale_ico: true, export_callback: None,
            show_color_picker: false, color_history: ColorHistory::load(),
            hex_input: String::from("#000000FF"), canvas_rect: None,
            filter_progress: Arc::new(Mutex::new(0.0)),
            is_processing: false,
            pending_filter_result: Arc::new(Mutex::new(None)),
            fonts_registered: false,
        }
    }

    pub fn load(path: PathBuf) -> Self {
        let mut editor: ImageEditor = Self::new();
        if let Ok(img) = image::open(&path) {
            editor.resize_w = img.width();
            editor.resize_h = img.height();
            editor.image = Some(img);
            editor.texture_dirty = true;
            editor.file_path = Some(path);
        }
        editor
    }

    pub fn is_dirty(&self) -> bool { self.dirty }
    pub fn set_file_callback(&mut self, callback: Box<dyn Fn(PathBuf) + Send + Sync>) { self.export_callback = Some(callback);}
    pub(super) fn add_color_to_history(&mut self) { self.color_history.add_color(RgbaColor::from_egui(self.color)); }

    pub(super) fn push_undo(&mut self) {
        if let Some(img) = &self.image {
            self.undo_stack.push_back(img.clone());
            if self.undo_stack.len() > MAX_UNDO { self.undo_stack.pop_front(); }
            self.redo_stack.clear();
        }
    }
    pub(super) fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop_back() {
            if let Some(cur) = self.image.take() { self.redo_stack.push_back(cur); }
            self.resize_w = prev.width(); self.resize_h = prev.height();
            self.image = Some(prev); self.texture_dirty = true; self.dirty = true;
        }
    }
    pub(super) fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop_back() {
            if let Some(cur) = self.image.take() { self.undo_stack.push_back(cur); }
            self.resize_w = next.width(); self.resize_h = next.height();
            self.image = Some(next); self.texture_dirty = true; self.dirty = true;
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
        self.texture_dirty = true; self.file_path = None;
        self.dirty = true; self.fit_on_next_frame = true;
    }

    pub(super) fn ensure_fonts_registered(&mut self, ctx: &egui::Context) {
        if self.fonts_registered { return; }
        self.fonts_registered = true;
        let mut fonts: egui::FontDefinitions = egui::FontDefinitions::default();
        let entries: &[(&str, &'static [u8])] = &[
            ("Ubuntu", FONT_UB_REG),
            ("Ubuntu-Bold", FONT_UB_BLD),
            ("Ubuntu-Italic", FONT_UB_ITL),
            ("Roboto", FONT_RB_REG),
            ("Roboto-Bold", FONT_RB_BLD),
            ("Roboto-Italic", FONT_RB_ITL),
        ];
        for (name, bytes) in entries {
            fonts.font_data.insert(name.to_string(), egui::FontData::from_static(bytes).into());
            fonts.families.insert(
                egui::FontFamily::Name((*name).into()),
                vec![name.to_string()],
            );
        }
        ctx.set_fonts(fonts);
    }

    pub(super) fn ensure_texture(&mut self, ctx: &egui::Context) {
        if !self.texture_dirty { return; }
        let img: &DynamicImage = match &self.image { Some(i) => i, None => { self.texture_dirty = false; return; } };
        let rgba: ImageBuffer<Rgba<u8>, Vec<u8>> = img.to_rgba8();
        let (w, h) = (rgba.width() as usize, rgba.height() as usize);
        let color_image: egui::ColorImage = egui::ColorImage {
            size: [w, h],
            source_size: egui::vec2(w as f32, h as f32),
            pixels: rgba.pixels().map(|p| egui::Color32::from_rgba_unmultiplied(p.0[0], p.0[1], p.0[2], p.0[3])).collect(),
        };

        if let Some(texture_id) = self.texture {
            ctx.tex_manager().write().set(texture_id, egui::epaint::ImageDelta::full(color_image, egui::TextureOptions::default()));
        } else {
            self.texture = Some(ctx.tex_manager().write().alloc("image_editor_img".into(), color_image.into(), egui::TextureOptions::default()));
        }

        self.texture_dirty = false;
    }

    pub(super) fn check_filter_completion(&mut self) {
        if !self.is_processing { return; }

        if *self.filter_progress.lock().unwrap() >= 1.0 {
            if let Some(result) = self.pending_filter_result.lock().unwrap().take() {
                self.resize_w = result.width(); self.resize_h = result.height();
                self.image = Some(result);
                self.texture_dirty = true; self.dirty = true; self.is_processing = false;
                self.filter_panel = FilterPanel::None;
                if self.resize_w != 0 { self.fit_on_next_frame = true; }
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
        });

        if !self.editing_text {
            ctx.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, egui::Key::B) { self.commit_or_discard_active_text(); self.tool = Tool::Brush; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::E) { self.commit_or_discard_active_text(); self.tool = Tool::Eraser; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::F) { self.commit_or_discard_active_text(); self.tool = Tool::Fill; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::T) { self.tool = Tool::Text; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::D) { self.commit_or_discard_active_text(); self.tool = Tool::Eyedropper; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::C) { self.commit_or_discard_active_text(); self.tool = Tool::Crop; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::P) { self.commit_or_discard_active_text(); self.tool = Tool::Pan; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Num0) { self.fit_image(); }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Plus)  { self.zoom *= 1.25; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Minus) { self.zoom = (self.zoom / 1.25).max(0.01); }
            });
        }
    }

    pub(super) fn save_impl(&mut self) -> Result<(), String> {
        let path: PathBuf = match &self.file_path {
            Some(p) => p.clone(),
            None => return self.save_as_impl(),
        };

        if let Some(img) = &self.image {
            let composite: DynamicImage = self.stamp_all_text_layers(img);
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
            if let Some(img) = &self.image {
                let composite: DynamicImage = self.stamp_all_text_layers(img);
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
        MenuContribution {
            file_items: vec![
                (MenuItem { label: "Export...".to_string(), shortcut: None, enabled: has_image }, MenuAction::Export),
            ],
            edit_items: vec![
                (MenuItem { label: "Undo".to_string(), shortcut: Some("Ctrl+Z".to_string()), enabled: !self.undo_stack.is_empty() }, MenuAction::Undo),
                (MenuItem { label: "Redo".to_string(), shortcut: Some("Ctrl+Y".to_string()), enabled: !self.redo_stack.is_empty() }, MenuAction::Redo),
            ],
            view_items: vec![
                (MenuItem { label: "Zoom In".to_string(), shortcut: Some("+".to_string()), enabled: true }, MenuAction::Custom("Zoom In".to_string())),
                (MenuItem { label: "Zoom Out".to_string(), shortcut: Some("-".to_string()), enabled: true }, MenuAction::Custom("Zoom Out".to_string())),
                (MenuItem { label: "Fit".to_string(), shortcut: Some("0".to_string()), enabled: true }, MenuAction::Custom("Fit".to_string())),
            ],
            image_items: vec![
                (MenuItem { label: "Resize Canvas...".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Resize Canvas".to_string())),
                (MenuItem { label: "Seperator".to_string(), shortcut: None, enabled: false }, MenuAction::None),
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
                (MenuItem { label: "Grayscale".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Gray".to_string())),
                (MenuItem { label: "Invert".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Invert".to_string())),
                (MenuItem { label: "Sepia".to_string(), shortcut: None, enabled: has_image }, MenuAction::Custom("Sepia".to_string())),
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
            _ => false,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, _show_file_info: bool) {
        let theme: ThemeMode = if ui.visuals().dark_mode { ThemeMode::Dark } else { ThemeMode::Light };

        self.ensure_fonts_registered(ctx);
        self.handle_keyboard(ctx);
        self.check_filter_completion();

        if self.is_processing { ctx.request_repaint(); }
        if self.image.is_none() && self.file_path.is_none() { self.new_image(800, 600); }

        self.render_toolbar(ui, theme);
        ui.add_space(4.0);
        self.render_options_bar(ui, theme);
        ui.add_space(4.0);

        if self.filter_panel != FilterPanel::None {
            self.render_filter_panel(ui, theme);
            ui.add_space(4.0);
        }

        if self.show_color_picker { self.render_color_picker(ui, ctx, theme); }
        self.render_canvas(ui, ctx);
    }
}
