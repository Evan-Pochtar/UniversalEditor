use eframe::egui;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, Rgba};
use crate::modules::image_export::{ExportFormat, export_image};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use crate::style::{ColorPalette, ThemeMode};
use super::{EditorModule, MenuAction, MenuItem, MenuContribution};
use serde::{Deserialize, Serialize};
use std::fs;
use ab_glyph::{Font as AbFont, FontRef, PxScale, ScaleFont};

const MAX_UNDO: usize = 20;
const MAX_COLOR_HISTORY: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct RgbaColor { r: u8, g: u8, b: u8, a: u8 }

impl RgbaColor {
    fn to_egui(&self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }
    fn from_egui(c: egui::Color32) -> Self {
        Self { r: c.r(), g: c.g(), b: c.b(), a: c.a() }
    }
    fn to_hex(&self) -> String {
        if self.a == 255 { format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b) }
        else { format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a) }
    }
    fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
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
    fn to_rgb_string(&self) -> String {
        if self.a == 255 { format!("rgb({}, {}, {})", self.r, self.g, self.b) }
        else { format!("rgba({}, {}, {}, {:.2})", self.r, self.g, self.b, self.a as f32 / 255.0) }
    }
}

#[derive(Serialize, Deserialize)]
struct ColorHistory { colors: VecDeque<RgbaColor> }

impl ColorHistory {
    fn new() -> Self { Self { colors: VecDeque::new() } }
    fn load() -> Self {
        let path = Self::get_config_path();
        if let Ok(s) = fs::read_to_string(&path) {
            if let Ok(h) = serde_json::from_str(&s) { return h; }
        }
        Self::new()
    }
    fn save(&self) {
        let path = Self::get_config_path();
        if let Some(p) = path.parent() { let _ = fs::create_dir_all(p); }
        if let Ok(j) = serde_json::to_string(self) { let _ = fs::write(path, j); }
    }
    fn get_config_path() -> PathBuf {
        let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("universal_editor");
        p.push("color_history.json");
        p
    }
    fn add_color(&mut self, color: RgbaColor) {
        if let Some(pos) = self.colors.iter().position(|c| *c == color) { self.colors.remove(pos); }
        self.colors.push_front(color);
        if self.colors.len() > MAX_COLOR_HISTORY { self.colors.pop_back(); }
        self.save();
    }
    fn get_colors(&self) -> &VecDeque<RgbaColor> { &self.colors }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tool { Brush, Eraser, Fill, Text, Eyedropper, Crop, Pan }

#[derive(Debug, Clone, Copy, PartialEq)]
enum FilterPanel { None, BrightnessContrast, HueSaturation, Blur, Sharpen, Resize, Export }

#[derive(Debug, Clone, Copy, PartialEq)]
enum THandle { Move, N, S, E, W, NE, NW, SE, SW, Rotate }

const HANDLE_HIT: f32 = 18.0;
const HANDLE_VIS: f32 = 8.0;
const ROTATE_DIST: f32 = 28.0;

struct TransformHandleSet { rect: egui::Rect, angle_rad: f32 }

impl TransformHandleSet {
    fn with_rotation(rect: egui::Rect, angle_rad: f32) -> Self { Self { rect, angle_rad } }
    fn rot(&self, p: egui::Pos2) -> egui::Pos2 {
        if self.angle_rad == 0.0 { return p; }
        let c = self.rect.center();
        let d = p - c;
        let (cos_a, sin_a) = (self.angle_rad.cos(), self.angle_rad.sin());
        c + egui::vec2(d.x * cos_a - d.y * sin_a, d.x * sin_a + d.y * cos_a)
    }
    fn positions(&self) -> [(THandle, egui::Pos2); 9] {
        let r = &self.rect;
        let (cx, cy) = (r.center().x, r.center().y);
        let top_center = self.rot(egui::pos2(cx, r.min.y));
        let (cos_a, sin_a) = (self.angle_rad.cos(), self.angle_rad.sin());
        let up_dir = egui::vec2(sin_a, -cos_a);
        let rot_handle_pos = top_center + up_dir * ROTATE_DIST;
        [
            (THandle::NW, self.rot(r.left_top())),
            (THandle::N,  self.rot(egui::pos2(cx, r.min.y))),
            (THandle::NE, self.rot(r.right_top())),
            (THandle::E,  self.rot(egui::pos2(r.max.x, cy))),
            (THandle::SE, self.rot(r.right_bottom())),
            (THandle::S,  self.rot(egui::pos2(cx, r.max.y))),
            (THandle::SW, self.rot(r.left_bottom())),
            (THandle::W,  self.rot(egui::pos2(r.min.x, cy))),
            (THandle::Rotate, rot_handle_pos),
        ]
    }
    fn hit_test(&self, pos: egui::Pos2) -> Option<THandle> {
        for (h, hpos) in self.positions() {
            if egui::Rect::from_center_size(hpos, egui::vec2(HANDLE_HIT, HANDLE_HIT)).contains(pos) {
                return Some(h);
            }
        }
        let c = self.rect.center();
        let d = pos - c;
        let (cos_a, sin_a) = (self.angle_rad.cos(), self.angle_rad.sin());
        let local = c + egui::vec2(d.x * cos_a + d.y * sin_a, -d.x * sin_a + d.y * cos_a);
        if self.rect.contains(local) { return Some(THandle::Move); }
        None
    }
    fn draw(&self, painter: &egui::Painter, accent: egui::Color32) {
        let corners = [self.rect.left_top(), self.rect.right_top(),
                       self.rect.right_bottom(), self.rect.left_bottom()];
        let rc: Vec<egui::Pos2> = corners.iter().map(|&p| self.rot(p)).collect();
        for i in 0..4 {
            painter.line_segment([rc[i], rc[(i+1) % 4]], egui::Stroke::new(1.5, accent));
        }
        let positions = self.positions();
        let (_, rot_pos) = positions[8];
        let top_center = positions[1].1;
        painter.line_segment([top_center, rot_pos], egui::Stroke::new(1.0, accent));
        for (h, hpos) in positions {
            let vis = if h == THandle::Rotate { HANDLE_VIS * 1.25 } else { HANDLE_VIS };
            let rnd = if h == THandle::Rotate { vis / 2.0 } else { 2.0 };
            painter.rect_filled(egui::Rect::from_center_size(hpos, egui::vec2(vis, vis)), rnd, accent);
        }
    }
    fn cursor_for(h: THandle) -> egui::CursorIcon {
        match h {
            THandle::Move   => egui::CursorIcon::Grab,
            THandle::N | THandle::S => egui::CursorIcon::ResizeVertical,
            THandle::E | THandle::W => egui::CursorIcon::ResizeHorizontal,
            THandle::NE | THandle::SW => egui::CursorIcon::ResizeNeSw,
            THandle::NW | THandle::SE => egui::CursorIcon::ResizeNwSe,
            THandle::Rotate => egui::CursorIcon::Alias,
        }
    }
}

#[derive(Debug, Clone)]
struct TextLayer {
    id: u64,
    content: String,
    img_x: f32,
    img_y: f32,
    font_size: f32,
    box_width: Option<f32>,
    box_height: Option<f32>,
    rotation: f32,
    color: egui::Color32,
    bold: bool,
    italic: bool,
    underline: bool,
    font_name: String,
    rendered_height: f32,
}

impl TextLayer {
    fn line_count(&self) -> usize { self.content.lines().count().max(1) }
    fn max_line_chars(&self) -> usize {
        self.content.lines().map(|l| l.chars().count()).max().unwrap_or(1).max(1)
    }
    fn auto_width(&self, zoom: f32) -> f32 {
        (self.max_line_chars() as f32 * self.font_size * 0.58 * zoom).max(self.font_size * zoom)
    }
    fn auto_height(&self, zoom: f32) -> f32 {
        if self.rendered_height > 0.0 { self.rendered_height * zoom }
        else { self.line_count() as f32 * self.font_size * 1.35 * zoom }
    }
    fn screen_rect(&self, anchor: egui::Pos2, zoom: f32) -> egui::Rect {
        let w = self.box_width.map(|bw| bw * zoom).unwrap_or_else(|| self.auto_width(zoom));
        let h = self.box_height.map(|bh| bh * zoom).unwrap_or_else(|| self.auto_height(zoom));
        egui::Rect::from_min_size(anchor, egui::vec2(w, h))
    }
    fn font_family_name(&self) -> &'static str {
        match (self.font_name.as_str(), self.bold, self.italic) {
            ("Roboto", true, _) => "Roboto-Bold",
            ("Roboto", _, true) => "Roboto-Italic",
            ("Roboto", ..)     => "Roboto",
            (_, true, _)       => "Ubuntu-Bold",
            (_, _, true)       => "Ubuntu-Italic",
            _                  => "Ubuntu",
        }
    }
}

struct TextDrag {
    handle: THandle,
    start: egui::Pos2,
    orig_img_x: f32,
    orig_img_y: f32,
    orig_font_size: f32,
    orig_box_width: Option<f32>,
    orig_box_height: Option<f32>,
    orig_rotation: f32,
    orig_rot_start_angle: f32,
}

struct CropState { start: Option<(f32, f32)>, end: Option<(f32, f32)> }

impl Default for CropState {
    fn default() -> Self {
        Self { start: None, end: None }
    }
}

pub struct ImageEditor {
    image: Option<DynamicImage>,
    texture: Option<egui::TextureId>,
    texture_dirty: bool,
    file_path: Option<PathBuf>,
    dirty: bool,

    undo_stack: VecDeque<DynamicImage>,
    redo_stack: VecDeque<DynamicImage>,

    zoom: f32,
    pan: egui::Vec2,
    fit_on_next_frame: bool,

    tool: Tool,
    brush_size: f32,
    brush_opacity: f32,
    eraser_size: f32,
    color: egui::Color32,

    stroke_points: Vec<(f32, f32)>,
    is_dragging: bool,

    text_layers: Vec<TextLayer>,
    selected_text: Option<u64>,
    editing_text: bool,
    next_text_id: u64,
    text_font_size: f32,
    text_bold: bool,
    text_italic: bool,
    text_underline: bool,
    text_font_name: String,
    text_drag: Option<TextDrag>,
    text_cursor: usize,
    text_sel_anchor: Option<usize>,

    crop_state: CropState,

    filter_panel: FilterPanel,
    brightness: f32,
    contrast: f32,
    hue: f32,
    saturation: f32,
    blur_radius: f32,
    sharpen_amount: f32,
    resize_w: u32,
    resize_h: u32,
    resize_locked: bool,
    resize_stretch: bool,

    export_format: ExportFormat,
    export_jpeg_quality: u8,
    export_preserve_metadata: bool,
    export_auto_scale_ico: bool,
    export_callback: Option<Box<dyn Fn(PathBuf) + Send + Sync>>,

    show_color_picker: bool,
    color_history: ColorHistory,
    hex_input: String,
    canvas_rect: Option<egui::Rect>,

    filter_progress: Arc<Mutex<f32>>,
    is_processing: bool,
    pending_filter_result: Arc<Mutex<Option<DynamicImage>>>,
    fonts_registered: bool,
}

impl ImageEditor {
    pub fn new() -> Self {
        Self {
            image: None,
            texture: None,
            texture_dirty: false,
            file_path: None,
            dirty: false,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            fit_on_next_frame: true,
            tool: Tool::Brush,
            brush_size: 12.0,
            brush_opacity: 1.0,
            eraser_size: 20.0,
            color: egui::Color32::BLACK,
            stroke_points: Vec::new(),
            is_dragging: false,
            text_layers: Vec::new(),
            selected_text: None,
            editing_text: false,
            next_text_id: 0,
            text_font_size: 24.0,
            text_bold: false,
            text_italic: false,
            text_underline: false,
            text_font_name: "Ubuntu".to_string(),
            text_drag: None,
            text_cursor: 0,
            text_sel_anchor: None,
            crop_state: CropState::default(),
            filter_panel: FilterPanel::None,
            brightness: 0.0,
            contrast: 0.0,
            hue: 0.0,
            saturation: 0.0,
            blur_radius: 3.0,
            sharpen_amount: 1.0,
            resize_w: 0,
            resize_h: 0,
            resize_locked: true,
            resize_stretch: false,
            export_format: ExportFormat::Png,
            export_jpeg_quality: 90,
            export_preserve_metadata: true,
            export_auto_scale_ico: true,
            export_callback: None,
            show_color_picker: false,
            color_history: ColorHistory::load(),
            hex_input: String::from("#000000FF"),
            canvas_rect: None,
            filter_progress: Arc::new(Mutex::new(0.0)),
            is_processing: false,
            pending_filter_result: Arc::new(Mutex::new(None)),
            fonts_registered: false,
        }
    }

    pub fn load(path: PathBuf) -> Self {
        let mut editor = Self::new();
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

    pub fn set_file_callback(&mut self, callback: Box<dyn Fn(PathBuf) + Send + Sync>) {
        self.export_callback = Some(callback);
    }

    fn add_color_to_history(&mut self) {
        self.color_history.add_color(RgbaColor::from_egui(self.color));
    }

    fn push_undo(&mut self) {
        if let Some(img) = &self.image {
            self.undo_stack.push_back(img.clone());
            if self.undo_stack.len() > MAX_UNDO { self.undo_stack.pop_front(); }
            self.redo_stack.clear();
        }
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop_back() {
            if let Some(cur) = self.image.take() { self.redo_stack.push_back(cur); }
            self.resize_w = prev.width();
            self.resize_h = prev.height();
            self.image = Some(prev);
            self.texture_dirty = true;
            self.dirty = true;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop_back() {
            if let Some(cur) = self.image.take() { self.undo_stack.push_back(cur); }
            self.resize_w = next.width();
            self.resize_h = next.height();
            self.image = Some(next);
            self.texture_dirty = true;
            self.dirty = true;
        }
    }

    fn screen_to_image(&self, screen_pos: egui::Pos2) -> Option<(u32, u32)> {
        let canvas = self.canvas_rect?;
        let img = self.image.as_ref()?;
        let (img_w, img_h) = (img.width() as f32, img.height() as f32);
        let sw = img_w * self.zoom;
        let sh = img_h * self.zoom;
        let ox = canvas.center().x - sw / 2.0 + self.pan.x;
        let oy = canvas.center().y - sh / 2.0 + self.pan.y;
        let rx = (screen_pos.x - ox) / self.zoom;
        let ry = (screen_pos.y - oy) / self.zoom;
        if rx < 0.0 || ry < 0.0 || rx >= img_w || ry >= img_h { return Option::None; }
        Some((rx as u32, ry as u32))
    }

    fn image_to_screen(&self, ix: f32, iy: f32) -> egui::Pos2 {
        let canvas = self.canvas_rect.unwrap_or(egui::Rect::NOTHING);
        let (img_w, img_h) = self.image.as_ref()
            .map(|i| (i.width() as f32, i.height() as f32))
            .unwrap_or((1.0, 1.0));
        let ox = canvas.center().x - img_w * self.zoom / 2.0 + self.pan.x;
        let oy = canvas.center().y - img_h * self.zoom / 2.0 + self.pan.y;
        egui::pos2(ox + ix * self.zoom, oy + iy * self.zoom)
    }

    fn fit_image(&mut self) {
        if let (Some(img), Some(canvas)) = (&self.image, self.canvas_rect) {
            let sx = canvas.width() / img.width() as f32;
            let sy = canvas.height() / img.height() as f32;
            self.zoom = sx.min(sy).min(1.0).max(0.01);
            self.pan = egui::Vec2::ZERO;
        }
    }

    fn apply_brush_stroke(&mut self) {
        if let Some(img) = self.image.as_mut() {
            if !matches!(img, DynamicImage::ImageRgba8(_)) {
                *img = DynamicImage::ImageRgba8(img.to_rgba8());
            }
        }
        
        let buf = match self.image.as_mut() {
            Some(DynamicImage::ImageRgba8(buf)) => buf,
            _ => return,
        };
        
        if self.stroke_points.len() < 2 {
            return;
        }
        
        let width = buf.width();
        let height = buf.height();
        let (r, g, b, base_a) = if self.tool == Tool::Eraser {
            (0u8, 0u8, 0u8, 0u8)
        } else {
            (self.color.r(), self.color.g(), self.color.b(), self.color.a())
        };
        let radius = if self.tool == Tool::Eraser { self.eraser_size / 2.0 } else { self.brush_size / 2.0 };
        let opacity = if self.tool == Tool::Eraser { 1.0 } else { self.brush_opacity };
        let radius_sq = radius * radius;

        for i in 0..self.stroke_points.len().saturating_sub(1) {
            let (x0, y0) = self.stroke_points[i];
            let (x1, y1) = self.stroke_points[i + 1];
            let dx = x1 - x0;
            let dy = y1 - y0;
            let dist = (dx * dx + dy * dy).sqrt();
            let steps = (dist / (radius * 0.4).max(1.0)).ceil() as usize;
            
            for s in 0..=steps {
                let t = if steps == 0 { 0.0 } else { s as f32 / steps as f32 };
                let cx = x0 + dx * t;
                let cy = y0 + dy * t;
                let min_x = (cx - radius).max(0.0) as u32;
                let max_x = ((cx + radius).ceil() as u32).min(width);
                let min_y = (cy - radius).max(0.0) as u32;
                let max_y = ((cy + radius).ceil() as u32).min(height);
                
                for py in min_y..max_y {
                    let dy_sq = (py as f32 - cy).powi(2);
                    
                    for px in min_x..max_x {
                        let dx_sq = (px as f32 - cx).powi(2);
                        let dist_sq = dx_sq + dy_sq;
                        
                        if dist_sq <= radius_sq {
                            let falloff = 1.0 - (dist_sq / radius_sq).sqrt();
                            let alpha = (falloff * opacity * 255.0) as u8;
                            
                            unsafe {
                                let pixel = buf.unsafe_get_pixel(px, py);
                                let [er, eg, eb, ea] = pixel.0;
                                
                                let new_pixel = if self.tool == Tool::Eraser {
                                    Rgba([er, eg, eb, ea.saturating_sub(alpha)])
                                } else {
                                    let fa = alpha as u16;
                                    let base_factor = (base_a as u16 * fa) / 255;
                                    let fb = 255 - base_factor;
                                    
                                    let nr = ((r as u16 * base_factor + er as u16 * fb) / 255) as u8;
                                    let ng = ((g as u16 * base_factor + eg as u16 * fb) / 255) as u8;
                                    let nb = ((b as u16 * base_factor + eb as u16 * fb) / 255) as u8;
                                    let na = ((base_factor + ea as u16 * fb / 255).min(255)) as u8;
                                    
                                    Rgba([nr, ng, nb, na])
                                };
                                
                                buf.unsafe_put_pixel(px, py, new_pixel);
                            }
                        }
                    }
                }
            }
        }
        
        self.dirty = true;
        self.texture_dirty = true;
    }

    fn flood_fill(&mut self, start_x: u32, start_y: u32) {
        let img = match self.image.as_mut() {
            Some(i) => i,
            None => return,
        };
        let mut buf = img.to_rgba8();
        let width = buf.width();
        let height = buf.height();
        let target = buf.get_pixel(start_x, start_y).0;
        let fill = [self.color.r(), self.color.g(), self.color.b(), self.color.a()];
        if target == fill { return; }
        
        let mut visited = vec![false; (width * height) as usize];
        let mut stack = Vec::with_capacity(1024);
        stack.push((start_x, start_y));
        let tolerance = 30i32;
        
        while let Some((x, y)) = stack.pop() {
            let idx = (y * width + x) as usize;
            if visited[idx] {
                continue;
            }
            visited[idx] = true;
            
            let cur = buf.get_pixel(x, y).0;
            let diff: i32 = (0..4).map(|i| (cur[i] as i32 - target[i] as i32).abs()).sum();
            if diff > tolerance { continue; }
            
            buf.put_pixel(x, y, Rgba(fill));
            if x > 0 { stack.push((x - 1, y)); }
            if x + 1 < width { stack.push((x + 1, y)); }
            if y > 0 { stack.push((x, y - 1)); }
            if y + 1 < height { stack.push((x, y + 1)); }
        }
        self.image = Some(DynamicImage::ImageRgba8(buf));
        self.texture_dirty = true;
        self.dirty = true;
    }

    fn sample_color(&mut self, x: u32, y: u32) {
        if let Some(img) = &self.image {
            let p = img.get_pixel(x, y).0;
            self.color = egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]);
            self.add_color_to_history();
            self.hex_input = RgbaColor::from_egui(self.color).to_hex();
        }
    }

    fn stamp_all_text_layers(&self, base: &DynamicImage) -> DynamicImage {
        if self.text_layers.is_empty() { return base.clone(); }

        static UB_REG: &[u8] = include_bytes!("../../assets/Ubuntu-Regular.ttf");
        static UB_BLD: &[u8] = include_bytes!("../../assets/Ubuntu-Bold.ttf");
        static UB_ITL: &[u8] = include_bytes!("../../assets/Ubuntu-Italic.ttf");
        static RB_REG: &[u8] = include_bytes!("../../assets/Roboto-Regular.ttf");
        static RB_BLD: &[u8] = include_bytes!("../../assets/Roboto-Bold.ttf");
        static RB_ITL: &[u8] = include_bytes!("../../assets/Roboto-Italic.ttf");

        let ub_reg = FontRef::try_from_slice(UB_REG).expect("Ubuntu-Regular");
        let ub_bld = FontRef::try_from_slice(UB_BLD).expect("Ubuntu-Bold");
        let ub_itl = FontRef::try_from_slice(UB_ITL).expect("Ubuntu-Italic");
        let rb_reg = FontRef::try_from_slice(RB_REG).expect("Roboto-Regular");
        let rb_bld = FontRef::try_from_slice(RB_BLD).expect("Roboto-Bold");
        let rb_itl = FontRef::try_from_slice(RB_ITL).expect("Roboto-Italic");

        let mut buf = base.to_rgba8();
        let (iw, ih) = (buf.width(), buf.height());

        for layer in &self.text_layers {
            let font: &FontRef = match (layer.font_name.as_str(), layer.bold, layer.italic) {
                ("Roboto", true, _) => &rb_bld,
                ("Roboto", _, true) => &rb_itl,
                ("Roboto", _, _) => &rb_reg,
                (_, true, _) => &ub_bld,
                (_, _, true) => &ub_itl,
                _ => &ub_reg,
            };

            let scale = PxScale::from(layer.font_size);
            let scaled = font.as_scaled(scale);
            let line_h = layer.font_size * 1.35;

            let wrap_w = layer.box_width.unwrap_or(f32::MAX);
            let mut visual_lines: Vec<String> = Vec::new();
            for paragraph in layer.content.split('\n') {
                if paragraph.is_empty() { visual_lines.push(String::new()); continue; }
                let mut cur_line = String::new();
                let mut cur_w = 0.0f32;
                for word in paragraph.split_inclusive(' ') {
                    let w: f32 = word.chars().map(|c| scaled.h_advance(font.glyph_id(c))).sum();
                    if cur_w + w > wrap_w && !cur_line.is_empty() {
                        visual_lines.push(cur_line.trim_end().to_string());
                        cur_line = word.to_string();
                        cur_w = w;
                    } else {
                        cur_line.push_str(word);
                        cur_w += w;
                    }
                }
                visual_lines.push(cur_line);
            }

            let actual_h = if layer.rendered_height > 0.0 {
                layer.rendered_height + 4.0
            } else {
                visual_lines.len() as f32 * line_h + 4.0
            };
            let bw = layer.box_width.unwrap_or_else(|| layer.auto_width(1.0)) + 4.0;
            let bh = layer.box_height.unwrap_or(actual_h);
            let ibw = bw.ceil() as usize;
            let ibh = bh.ceil() as usize;

            let mut tbuf: Vec<[f32; 4]> = vec![[0.0; 4]; ibw * ibh];
            let (cr, cg, cb, ca) = (
                layer.color.r() as f32 / 255.0,
                layer.color.g() as f32 / 255.0,
                layer.color.b() as f32 / 255.0,
                layer.color.a() as f32 / 255.0,
            );

            let put = |tbuf: &mut Vec<[f32; 4]>, tx: i32, ty: i32, cov: f32| {
                if tx < 0 || ty < 0 || tx >= ibw as i32 || ty >= ibh as i32 { return; }
                let idx = ty as usize * ibw + tx as usize;
                let src_a = (cov * ca).min(1.0);
                let dst = &mut tbuf[idx];
                let out_a = src_a + dst[3] * (1.0 - src_a);
                if out_a < 1e-5 { return; }
                dst[0] = (cr * src_a + dst[0] * dst[3] * (1.0 - src_a)) / out_a;
                dst[1] = (cg * src_a + dst[1] * dst[3] * (1.0 - src_a)) / out_a;
                dst[2] = (cb * src_a + dst[2] * dst[3] * (1.0 - src_a)) / out_a;
                dst[3] = out_a;
            };

            for (line_idx, line) in visual_lines.iter().enumerate() {
                let base_y = line_idx as f32 * line_h + scaled.ascent();
                let mut cursor_x = 0.0f32;
                for ch in line.chars() {
                    let gid = font.glyph_id(ch);
                    let adv = scaled.h_advance(gid);
                    let glyph = gid.with_scale_and_position(scale, ab_glyph::point(cursor_x, 0.0));
                    if let Some(outlined) = font.outline_glyph(glyph) {
                        let bounds = outlined.px_bounds();
                        outlined.draw(|gx, gy, cov| {
                            let tx = (bounds.min.x + gx as f32) as i32;
                            let ty = (base_y + bounds.min.y + gy as f32) as i32;
                            put(&mut tbuf, tx, ty, cov);
                        });
                    }
                    if layer.underline {
                        let uly = (base_y + scaled.descent() + 2.0) as i32;
                        for ux in cursor_x as i32..(cursor_x + adv) as i32 {
                            put(&mut tbuf, ux, uly, 1.0);
                        }
                    }
                    cursor_x += adv;
                }
            }

            let rcx = layer.img_x + bw / 2.0;
            let rcy = layer.img_y + bh / 2.0;
            let angle_rad = layer.rotation.to_radians();
            let (cos_a, sin_a) = (angle_rad.cos(), angle_rad.sin());
            let half_w = bw / 2.0;
            let half_h = bh / 2.0;

            let corners = [
                (rcx - half_w * cos_a + half_h * sin_a, rcy - half_w * sin_a - half_h * cos_a),
                (rcx + half_w * cos_a + half_h * sin_a, rcy + half_w * sin_a - half_h * cos_a),
                (rcx + half_w * cos_a - half_h * sin_a, rcy + half_w * sin_a + half_h * cos_a),
                (rcx - half_w * cos_a - half_h * sin_a, rcy - half_w * sin_a + half_h * cos_a),
            ];
            let min_x = corners.iter().map(|c| c.0).fold(f32::MAX, f32::min).max(0.0) as i32;
            let max_x = (corners.iter().map(|c| c.0).fold(f32::MIN, f32::max).ceil() as i32).min(iw as i32);
            let min_y = corners.iter().map(|c| c.1).fold(f32::MAX, f32::min).max(0.0) as i32;
            let max_y = (corners.iter().map(|c| c.1).fold(f32::MIN, f32::max).ceil() as i32).min(ih as i32);

            for py in min_y..max_y {
                for px in min_x..max_x {
                    let dx = px as f32 - rcx;
                    let dy = py as f32 - rcy;
                    let ux = dx * cos_a + dy * sin_a;
                    let uy = -dx * sin_a + dy * cos_a;
                    let tx = (ux + half_w) as i32;
                    let ty = (uy + half_h) as i32;
                    if tx < 0 || ty < 0 || tx >= ibw as i32 || ty >= ibh as i32 { continue; }
                    let src = tbuf[ty as usize * ibw + tx as usize];
                    let src_a = src[3];
                    if src_a < 1e-5 { continue; }
                    let dst = buf.get_pixel(px as u32, py as u32).0;
                    let dst_a = dst[3] as f32 / 255.0;
                    let out_a = (src_a + dst_a * (1.0 - src_a)).min(1.0);
                    if out_a < 1e-5 { continue; }
                    let blend = |s: f32, d: u8| -> u8 {
                        ((s * src_a + d as f32 / 255.0 * dst_a * (1.0 - src_a)) / out_a * 255.0).min(255.0) as u8
                    };
                    buf.put_pixel(px as u32, py as u32, Rgba([
                        blend(src[0], dst[0]), blend(src[1], dst[1]), blend(src[2], dst[2]),
                        (out_a * 255.0).min(255.0) as u8,
                    ]));
                }
            }
        }
        DynamicImage::ImageRgba8(buf)
    }

    fn hit_text_layer(&self, pos: egui::Pos2) -> Option<u64> {
        for layer in self.text_layers.iter().rev() {
            let anchor = self.image_to_screen(layer.img_x, layer.img_y);
            if layer.screen_rect(anchor, self.zoom).contains(pos) {
                return Some(layer.id);
            }
        }
        None
    }

    fn text_transform_handles(&self) -> Option<TransformHandleSet> {
        let id = self.selected_text?;
        let layer = self.text_layers.iter().find(|l| l.id == id)?;
        let anchor = self.image_to_screen(layer.img_x, layer.img_y);
        Some(TransformHandleSet::with_rotation(layer.screen_rect(anchor, self.zoom), layer.rotation.to_radians()))
    }

    fn commit_or_discard_active_text(&mut self) {
        if let Some(id) = self.selected_text {
            let empty = self.text_layers.iter().find(|l| l.id == id).map(|l| l.content.is_empty()).unwrap_or(true);
            if empty { self.text_layers.retain(|l| l.id != id); }
        }
        self.selected_text = None;
        self.editing_text = false;
        self.text_drag = None;
        self.text_cursor = 0;
        self.text_sel_anchor = None;
    }

    fn process_text_input(&mut self, ctx: &egui::Context) {
        if !self.editing_text || self.selected_text.is_none() { return; }
        let id = self.selected_text.unwrap();

        let (events, shift, ctrl) = ctx.input(|i| {
            (i.events.clone(), i.modifiers.shift, i.modifiers.ctrl || i.modifiers.mac_cmd)
        });

        for event in &events {
            let cursor = self.text_cursor;
            let sel = self.text_sel_anchor;
            match event {
                egui::Event::Text(t) => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi);
                            self.text_cursor = lo;
                            self.text_sel_anchor = None;
                        }
                        let c = self.text_cursor;
                        layer.content.insert_str(c, t);
                        self.text_cursor += t.len();
                    }
                }
                egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi);
                            self.text_cursor = lo;
                            self.text_sel_anchor = None;
                        }
                        let c = self.text_cursor;
                        layer.content.insert(c, '\n');
                        self.text_cursor += 1;
                    }
                }
                egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi);
                            self.text_cursor = lo;
                            self.text_sel_anchor = None;
                        } else if cursor > 0 {
                            let prev = layer.content[..cursor]
                                .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                            layer.content.drain(prev..cursor);
                            self.text_cursor = prev;
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::Delete, pressed: true, .. } => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi);
                            self.text_cursor = lo;
                            self.text_sel_anchor = None;
                        } else if cursor < layer.content.len() {
                            let next = layer.content[cursor..]
                                .char_indices().nth(1).map(|(i, _)| cursor + i)
                                .unwrap_or(layer.content.len());
                            layer.content.drain(cursor..next);
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::ArrowLeft, pressed: true, .. } => {
                    if let Some(layer) = self.text_layers.iter().find(|l| l.id == id) {
                        if !shift && sel.is_some() {
                            let anchor = sel.unwrap();
                            self.text_cursor = cursor.min(anchor);
                            self.text_sel_anchor = None;
                        } else {
                            if shift && self.text_sel_anchor.is_none() { self.text_sel_anchor = Some(cursor); }
                            if cursor > 0 {
                                let prev = layer.content[..cursor]
                                    .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                                self.text_cursor = prev;
                            }
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::ArrowRight, pressed: true, .. } => {
                    if let Some(layer) = self.text_layers.iter().find(|l| l.id == id) {
                        if !shift && sel.is_some() {
                            let anchor = sel.unwrap();
                            self.text_cursor = cursor.max(anchor);
                            self.text_sel_anchor = None;
                        } else {
                            if shift && self.text_sel_anchor.is_none() { self.text_sel_anchor = Some(cursor); }
                            if cursor < layer.content.len() {
                                let next = layer.content[cursor..]
                                    .char_indices().nth(1).map(|(i, _)| cursor + i)
                                    .unwrap_or(layer.content.len());
                                self.text_cursor = next;
                            }
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::Home, pressed: true, .. } => {
                    if shift && self.text_sel_anchor.is_none() { self.text_sel_anchor = Some(cursor); }
                    else if !shift { self.text_sel_anchor = None; }
                    self.text_cursor = 0;
                }
                egui::Event::Key { key: egui::Key::End, pressed: true, .. } => {
                    let len = self.text_layers.iter().find(|l| l.id == id).map(|l| l.content.len()).unwrap_or(0);
                    if shift && self.text_sel_anchor.is_none() { self.text_sel_anchor = Some(cursor); }
                    else if !shift { self.text_sel_anchor = None; }
                    self.text_cursor = len;
                }
                egui::Event::Key { key: egui::Key::A, pressed: true, modifiers, .. }
                    if modifiers.ctrl || modifiers.mac_cmd =>
                {
                    let len = self.text_layers.iter().find(|l| l.id == id).map(|l| l.content.len()).unwrap_or(0);
                    self.text_sel_anchor = Some(0);
                    self.text_cursor = len;
                }
                egui::Event::Copy => {
                    if let Some(anchor) = sel {
                        if let Some(layer) = self.text_layers.iter().find(|l| l.id == id) {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            if lo < hi && hi <= layer.content.len() {
                                ctx.copy_text(layer.content[lo..hi].to_string());
                            }
                        }
                    }
                }
                egui::Event::Cut => {
                    if let Some(anchor) = sel {
                        if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            if lo < hi && hi <= layer.content.len() {
                                let cut_text = layer.content[lo..hi].to_string();
                                ctx.copy_text(cut_text);
                                layer.content.drain(lo..hi);
                                self.text_cursor = lo;
                                self.text_sel_anchor = None;
                            }
                        }
                    }
                }
                egui::Event::Paste(text) => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi);
                            self.text_cursor = lo;
                            self.text_sel_anchor = None;
                        }
                        let c = self.text_cursor;
                        layer.content.insert_str(c, text);
                        self.text_cursor += text.len();
                    }
                }
                _ => {}
            }
        }

        if let Some(layer) = self.text_layers.iter().find(|l| l.id == id) {
            let clamp = |c: usize| -> usize {
                let c = c.min(layer.content.len());
                if layer.content.is_char_boundary(c) { c }
                else { (0..c).rev().find(|&i| layer.content.is_char_boundary(i)).unwrap_or(0) }
            };
            self.text_cursor = clamp(self.text_cursor);
            if let Some(a) = self.text_sel_anchor { self.text_sel_anchor = Some(clamp(a)); }
        }
        let _ = ctrl;
    }

    fn apply_crop(&mut self) {
        let img = match &self.image {
            Some(i) => i,
            None => return,
        };
        let (s, e) = match (self.crop_state.start, self.crop_state.end) {
            (Some(s), Some(e)) => (s, e),
            _ => return,
        };
        let x0 = s.0.min(e.0).max(0.0) as u32;
        let y0 = s.1.min(e.1).max(0.0) as u32;
        let x1 = (s.0.max(e.0) as u32).min(img.width());
        let y1 = (s.1.max(e.1) as u32).min(img.height());
        if x1 <= x0 || y1 <= y0 { return; }
        let cropped = img.crop_imm(x0, y0, x1 - x0, y1 - y0);
        self.resize_w = cropped.width();
        self.resize_h = cropped.height();
        self.image = Some(cropped);
        self.texture_dirty = true;
        self.dirty = true;
        self.crop_state = CropState::default();
        self.fit_on_next_frame = true;
    }

    fn apply_brightness_contrast(&mut self) {
        let img = match self.image.clone() { Some(i) => i, None => return };
        let b = self.brightness;
        let c = 1.0 + self.contrast / 100.0;
        let progress = Arc::clone(&self.filter_progress);
        let result = Arc::clone(&self.pending_filter_result);
        self.is_processing = true;
        *progress.lock().unwrap() = 0.0;
        thread::spawn(move || {
            let mut buf = img.to_rgba8();
            let total = (buf.width() * buf.height()) as usize;
            let mut processed = 0;
            for pixel in buf.pixels_mut() {
                for i in 0..3 {
                    let val = pixel[i] as f32;
                    pixel[i] = ((val - 128.0) * c + 128.0 + b).clamp(0.0, 255.0) as u8;
                }
                processed += 1;
                if processed % 5000 == 0 { *progress.lock().unwrap() = processed as f32 / total as f32; }
            }
            *result.lock().unwrap() = Some(DynamicImage::ImageRgba8(buf));
            *progress.lock().unwrap() = 1.0;
        });
    }

    fn apply_hue_saturation(&mut self) {
         let img = match self.image.clone() { Some(i) => i, None => return };
        let sat_factor = 1.0 + self.saturation / 100.0;
        let hue_shift = self.hue;
        let progress = Arc::clone(&self.filter_progress);
        let result = Arc::clone(&self.pending_filter_result);
        self.is_processing = true;
        *progress.lock().unwrap() = 0.0;
        
        thread::spawn(move || {
            let mut buf = img.to_rgba8();
            for y in 0..buf.height() {
                for x in 0..buf.width() {
                    let p = buf.get_pixel(x, y).0;
                    let (h, s, v) = rgb_to_hsv(p[0], p[1], p[2]);
                    let nh = (h + hue_shift).rem_euclid(360.0);
                    let ns = (s * sat_factor).clamp(0.0, 1.0);
                    let (nr, ng, nb) = hsv_to_rgb(nh, ns, v);
                    buf.put_pixel(x, y, Rgba([nr, ng, nb, p[3]]));
                }
                if y % 10 == 0 { *progress.lock().unwrap() = y as f32 / buf.height() as f32; }
            }
            *result.lock().unwrap() = Some(DynamicImage::ImageRgba8(buf));
            *progress.lock().unwrap() = 1.0;
        });
    }
    
    fn apply_blur(&mut self) { 
        let img = match self.image.clone() { Some(i) => i, None => return };
        let radius = self.blur_radius;
        let result = Arc::clone(&self.pending_filter_result);
        let progress = Arc::clone(&self.filter_progress);
        self.is_processing = true;
        thread::spawn(move || {
            *progress.lock().unwrap() = 0.5;
            *result.lock().unwrap() = Some(img.blur(radius));
            *progress.lock().unwrap() = 1.0;
        });
    }
    fn apply_sharpen(&mut self) { 
         let img = match self.image.clone() { Some(i) => i, None => return };
        let amount = self.sharpen_amount;
        let result = Arc::clone(&self.pending_filter_result);
        let progress = Arc::clone(&self.filter_progress);
        self.is_processing = true;
        thread::spawn(move || {
            *progress.lock().unwrap() = 0.5;
            *result.lock().unwrap() = Some(img.unsharpen(amount, 0));
            *progress.lock().unwrap() = 1.0;
        });
    }

    fn apply_grayscale(&mut self) {
        if let Some(img) = &self.image {
            let gray = img.grayscale();
            self.image = Some(DynamicImage::ImageRgba8(gray.to_rgba8()));
            self.texture_dirty = true;
            self.dirty = true;
        }
    }

    fn apply_invert(&mut self) {
        let img = match self.image.as_mut() {
            Some(i) => i,
            None => return,
        };
        let mut buf = img.to_rgba8();
        let mut pixels = buf.as_flat_samples_mut();
        let samples = pixels.as_mut_slice();
        
        for chunk in samples.chunks_exact_mut(4) {
            chunk[0] = 255 - chunk[0];
            chunk[1] = 255 - chunk[1];
            chunk[2] = 255 - chunk[2];
        }
        
        self.image = Some(DynamicImage::ImageRgba8(buf));
        self.texture_dirty = true;
        self.dirty = true;
    }

    fn apply_sepia(&mut self) {
        let img = match self.image.as_mut() {
            Some(i) => i,
            None => return,
        };
        let mut buf = img.to_rgba8();
        
        for pixel in buf.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            let (rf, gf, bf) = (r as f32, g as f32, b as f32);
            
            pixel.0 = [
                (rf * 0.393 + gf * 0.769 + bf * 0.189).min(255.0) as u8,
                (rf * 0.349 + gf * 0.686 + bf * 0.168).min(255.0) as u8,
                (rf * 0.272 + gf * 0.534 + bf * 0.131).min(255.0) as u8,
                a,
            ];
        }
        
        self.image = Some(DynamicImage::ImageRgba8(buf));
        self.texture_dirty = true;
        self.dirty = true;
    }

    fn transform_text_rotate_cw(&mut self, _old_w: u32, old_h: u32) {
        for layer in &mut self.text_layers {
            let bw = layer.box_width.unwrap_or_else(|| layer.auto_width(1.0));
            let bh = layer.box_height.unwrap_or_else(|| layer.auto_height(1.0));
            let cx = layer.img_x + bw / 2.0;
            let cy = layer.img_y + bh / 2.0;
            let new_cx = old_h as f32 - cy;
            let new_cy = cx;
            layer.img_x = new_cx - bh / 2.0;
            layer.img_y = new_cy - bw / 2.0;
            std::mem::swap(&mut layer.box_width, &mut layer.box_height);
            layer.rotation = (layer.rotation + 90.0).rem_euclid(360.0);
        }
    }

    fn transform_text_rotate_ccw(&mut self, old_w: u32, _old_h: u32) {
        for layer in &mut self.text_layers {
            let bw = layer.box_width.unwrap_or_else(|| layer.auto_width(1.0));
            let bh = layer.box_height.unwrap_or_else(|| layer.auto_height(1.0));
            let cx = layer.img_x + bw / 2.0;
            let cy = layer.img_y + bh / 2.0;
            let new_cx = cy;
            let new_cy = old_w as f32 - cx;
            layer.img_x = new_cx - bh / 2.0;
            layer.img_y = new_cy - bw / 2.0;
            std::mem::swap(&mut layer.box_width, &mut layer.box_height);
            layer.rotation = (layer.rotation - 90.0).rem_euclid(360.0);
        }
    }

    fn transform_text_flip_h(&mut self, old_w: u32) {
        for layer in &mut self.text_layers {
            let bw = layer.box_width.unwrap_or_else(|| layer.auto_width(1.0));
            let cx = layer.img_x + bw / 2.0;
            layer.img_x = old_w as f32 - cx - bw / 2.0;
            layer.rotation = -(layer.rotation).rem_euclid(360.0);
        }
    }

    fn transform_text_flip_v(&mut self, old_h: u32) {
        for layer in &mut self.text_layers {
            let bh = layer.box_height.unwrap_or_else(|| layer.auto_height(1.0));
            let cy = layer.img_y + bh / 2.0;
            layer.img_y = old_h as f32 - cy - bh / 2.0;
            layer.rotation = -(layer.rotation).rem_euclid(360.0);
        }
    }

    fn apply_flip_h(&mut self) {
        let (old_w, flipped) = match &self.image {
            Some(img) => (img.width(), img.fliph()),
            None => return,
        };
        self.transform_text_flip_h(old_w);
        self.image = Some(flipped);
        self.texture_dirty = true;
        self.dirty = true;
    }

    fn apply_flip_v(&mut self) {
        let (old_h, flipped) = match &self.image {
            Some(img) => (img.height(), img.flipv()),
            None => return,
        };
        self.transform_text_flip_v(old_h);
        self.image = Some(flipped);
        self.texture_dirty = true;
        self.dirty = true;
    }

    fn apply_rotate_cw(&mut self) {
        let (old_w, old_h, rotated) = match &self.image {
            Some(img) => (img.width(), img.height(), img.rotate90()),
            None => return,
        };
        self.transform_text_rotate_cw(old_w, old_h);
        self.image = Some(rotated);
        self.resize_w = self.image.as_ref().unwrap().width();
        self.resize_h = self.image.as_ref().unwrap().height();
        self.texture_dirty = true;
        self.dirty = true;
        self.fit_on_next_frame = true;
    }

    fn apply_rotate_ccw(&mut self) {
        let (old_w, old_h, rotated) = match &self.image {
            Some(img) => (img.width(), img.height(), img.rotate270()),
            None => return,
        };
        self.transform_text_rotate_ccw(old_w, old_h);
        self.image = Some(rotated);
        self.resize_w = self.image.as_ref().unwrap().width();
        self.resize_h = self.image.as_ref().unwrap().height();
        self.texture_dirty = true;
        self.dirty = true;
        self.fit_on_next_frame = true;
    }

    fn apply_resize(&mut self) { 
        let img = match self.image.clone() { Some(i) => i, None => return };
        if self.resize_w == 0 || self.resize_h == 0 { return; }
        let (w, h, stretch) = (self.resize_w, self.resize_h, self.resize_stretch);
        let result = Arc::clone(&self.pending_filter_result);
        let progress = Arc::clone(&self.filter_progress);
        self.is_processing = true;
        thread::spawn(move || {
            *progress.lock().unwrap() = 0.5;
            let final_img = if stretch { img.resize_exact(w, h, image::imageops::FilterType::Lanczos3) } else {
                let mut new_buf = ImageBuffer::from_pixel(w, h, Rgba([255, 255, 255, 255]));
                image::imageops::overlay(&mut new_buf, &img, 0, 0);
                DynamicImage::ImageRgba8(new_buf)
            };
            *result.lock().unwrap() = Some(final_img);
            *progress.lock().unwrap() = 1.0;
        });
    }

    fn export_image(&mut self) -> Result<PathBuf, String> {
        let img = match &self.image {
            Some(i) => i,
            None => return Err("No image to export".to_string()),
        };

        let composite = self.stamp_all_text_layers(img);

        let default_name = self.file_path.as_ref().and_then(|p| p.file_stem()).and_then(|s| s.to_str()).unwrap_or("export");
        let filename = format!("{}.{}", default_name, self.export_format.extension());
        
        let path = match rfd::FileDialog::new()
            .set_file_name(&filename)
            .add_filter(self.export_format.as_str(), &[self.export_format.extension()])
            .save_file()
        {
            Some(p) => p,
            None => return Err("Export cancelled".to_string()),
        };

        export_image(
            &composite, 
            &path, 
            self.export_format, 
            self.export_jpeg_quality,
            6,
            100.0,
            self.export_auto_scale_ico
        )?;

        self.filter_panel = FilterPanel::None;
        Ok(path)
    }

    fn ensure_fonts_registered(&mut self, ctx: &egui::Context) {
        if self.fonts_registered { return; }
        self.fonts_registered = true;

        static UB_REG: &[u8] = include_bytes!("../../assets/Ubuntu-Regular.ttf");
        static UB_BLD: &[u8] = include_bytes!("../../assets/Ubuntu-Bold.ttf");
        static UB_ITL: &[u8] = include_bytes!("../../assets/Ubuntu-Italic.ttf");
        static RB_REG: &[u8] = include_bytes!("../../assets/Roboto-Regular.ttf");
        static RB_BLD: &[u8] = include_bytes!("../../assets/Roboto-Bold.ttf");
        static RB_ITL: &[u8] = include_bytes!("../../assets/Roboto-Italic.ttf");

        let mut fonts = egui::FontDefinitions::default();
        let entries: &[(&str, &'static [u8])] = &[
            ("Ubuntu", UB_REG),
            ("Ubuntu-Bold", UB_BLD),
            ("Ubuntu-Italic", UB_ITL),
            ("Roboto", RB_REG),
            ("Roboto-Bold", RB_BLD),
            ("Roboto-Italic", RB_ITL),
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

    fn ensure_texture(&mut self, ctx: &egui::Context) {
        if !self.texture_dirty { return; }
        let img = match &self.image { Some(i) => i, None => { self.texture_dirty = false; return; } };
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width() as usize, rgba.height() as usize);
        let color_image = egui::ColorImage {
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

    fn new_image(&mut self, w: u32, h: u32) {
        self.push_undo();
        self.image = Some(DynamicImage::ImageRgba8(ImageBuffer::from_pixel(w, h, Rgba([255, 255, 255, 255]))));
        self.resize_w = w; self.resize_h = h;
        self.texture_dirty = true;
        self.file_path = None;
        self.dirty = true;
        self.fit_on_next_frame = true;
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (bg, border, _text_col) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::ZINC_700, ColorPalette::ZINC_200)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::GRAY_300, ColorPalette::GRAY_800)
        };

        egui::Frame::new()
            .fill(bg)
            .stroke(egui::Stroke::new(1.0, border))
            .corner_radius(6.0)
            .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 4 })
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, true])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                    .min_scrolled_height(32.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            self.tool_btn(ui, "Brush", Tool::Brush, Some("B"), theme);
                            self.tool_btn(ui, "Eraser", Tool::Eraser, Some("E"), theme);
                            self.tool_btn(ui, "Fill", Tool::Fill, Some("F"), theme);
                            self.tool_btn(ui, "Text", Tool::Text, Some("T"), theme);
                            self.tool_btn(ui, "Eyedrop", Tool::Eyedropper, Some("D"), theme);
                            self.tool_btn(ui, "Crop", Tool::Crop, Some("C"), theme);
                            self.tool_btn(ui, "Pan", Tool::Pan, Some("P"), theme);
                        });
                    });
            });
    }

    fn tool_btn(&mut self, ui: &mut egui::Ui, label: &str, tool: Tool, shortcut: Option<&str>, theme: ThemeMode) {
        let active = self.tool == tool;
        let (bg, hover, txt) = if active {
            (ColorPalette::BLUE_600, ColorPalette::BLUE_500, egui::Color32::WHITE)
        } else if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_700, ColorPalette::ZINC_600, ColorPalette::ZINC_200)
        } else {
            (ColorPalette::GRAY_200, ColorPalette::GRAY_300, ColorPalette::GRAY_800)
        };
        let response = ui.scope(|ui| {
            let s = ui.style_mut();
            s.visuals.widgets.inactive.bg_fill = bg;
            s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            s.visuals.widgets.hovered.bg_fill = hover;
            s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            s.visuals.widgets.active.bg_fill = hover;
            let btn = ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(txt)).min_size(egui::vec2(0.0, 24.0)));
            if let Some(sc) = shortcut {
                btn.on_hover_text(sc)
            } else {
                btn
            }
        }).inner;
        if response.clicked() { 
            if tool != Tool::Text { self.commit_or_discard_active_text(); }
            self.tool = tool; 
        }
    }

    fn render_options_bar(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        ui.spacing_mut().slider_width = 100.0;
        let (bg, border, label_col) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::ZINC_700, ColorPalette::ZINC_400)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::GRAY_300, ColorPalette::ZINC_600)
        };

        egui::Frame::new()
            .fill(bg)
            .stroke(egui::Stroke::new(1.0, border))
            .corner_radius(6.0)
            .inner_margin(egui::Margin { left: 8, right: 8, top: 3, bottom: 3 })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    match self.tool {
                        Tool::Brush => {
                            ui.label(egui::RichText::new("Size:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.brush_size, 1.0..=200.0));
                            ui.label(egui::RichText::new("Opacity:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.brush_opacity, 0.0..=1.0));
                        }
                        Tool::Eraser => {
                            ui.label(egui::RichText::new("Size:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.eraser_size, 1.0..=200.0));
                        }
                        Tool::Text => {
                            ui.label(egui::RichText::new("Font:").size(12.0).color(label_col));
                            let avail_fonts = ["Ubuntu", "Roboto"];
                            let cur_font = self.text_font_name.clone();
                            egui::ComboBox::from_id_salt("text_font_pick")
                                .selected_text(cur_font.as_str())
                                .width(90.0)
                                .show_ui(ui, |ui| {
                                    for f in &avail_fonts {
                                        if ui.selectable_label(self.text_font_name == *f, *f).clicked() {
                                            self.text_font_name = f.to_string();
                                            if let Some(id) = self.selected_text {
                                                if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                                                    layer.font_name = f.to_string();
                                                }
                                            }
                                        }
                                    }
                                });

                            ui.separator();
                            ui.label(egui::RichText::new("Size:").size(12.0).color(label_col));
                            let mut fs = self.text_font_size;
                            if ui.add(egui::DragValue::new(&mut fs).range(6.0..=400.0).speed(1.0)).changed() {
                                self.text_font_size = fs;
                                if let Some(id) = self.selected_text {
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                                        layer.font_size = fs;
                                    }
                                }
                            }

                            ui.separator();
                            let style_btn = |ui: &mut egui::Ui, label: egui::RichText, active: bool, theme: ThemeMode| -> bool {
                                let (bg, txt) = if active {
                                    (ColorPalette::BLUE_600, egui::Color32::WHITE)
                                } else if matches!(theme, ThemeMode::Dark) {
                                    (ColorPalette::ZINC_700, ColorPalette::ZINC_200)
                                } else {
                                    (ColorPalette::GRAY_200, ColorPalette::GRAY_800)
                                };
                                ui.scope(|ui| {
                                    let s = ui.style_mut();
                                    s.visuals.widgets.inactive.bg_fill = bg;
                                    s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                    s.visuals.widgets.hovered.bg_fill = bg;
                                    ui.add(egui::Button::new(label.color(txt)).min_size(egui::vec2(24.0, 24.0)))
                                }).inner.clicked()
                            };

                            if style_btn(ui, egui::RichText::new("B").strong().size(13.0), self.text_bold, theme) {
                                self.text_bold = !self.text_bold;
                                if let Some(id) = self.selected_text {
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) { layer.bold = self.text_bold; }
                                }
                            }
                            if style_btn(ui, egui::RichText::new("I").italics().size(13.0), self.text_italic, theme) {
                                self.text_italic = !self.text_italic;
                                if let Some(id) = self.selected_text {
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) { layer.italic = self.text_italic; }
                                }
                            }
                            if style_btn(ui, egui::RichText::new("U").underline().size(13.0), self.text_underline, theme) {
                                self.text_underline = !self.text_underline;
                                if let Some(id) = self.selected_text {
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) { layer.underline = self.text_underline; }
                                }
                            }

                            if let Some(id) = self.selected_text {
                                ui.separator();
                                if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                                    layer.color = self.color;
                                }

                                if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                                    ui.separator();
                                    ui.label(egui::RichText::new("Rot:").size(12.0).color(label_col));
                                    ui.add(egui::DragValue::new(&mut layer.rotation).speed(1.0).range(-360.0..=360.0).suffix("°")).on_hover_text("Rotation in degrees");
                                }

                                if ui.button("Deselect").clicked() {
                                    self.commit_or_discard_active_text();
                                }
                                if ui.button("Delete").clicked() {
                                    let del_id = id;
                                    self.text_layers.retain(|l| l.id != del_id);
                                    self.selected_text = None;
                                    self.editing_text = false;
                                }
                            }
                            if !self.text_layers.is_empty() {
                                ui.separator();
                                ui.label(egui::RichText::new(format!("{} layer(s)", self.text_layers.len())).size(11.0).color(label_col));
                            }
                        }
                        Tool::Crop => {
                            if self.crop_state.start.is_some() && self.crop_state.end.is_some() {
                                if ui.button("Apply Crop").clicked() {
                                    self.push_undo();
                                    self.apply_crop();
                                }
                                if ui.button("Cancel").clicked() {
                                    self.crop_state = CropState::default();
                                }
                            }
                        }
                        _ => {}
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let color_btn = egui::Button::new("")
                            .fill(self.color)
                            .min_size(egui::vec2(28.0, 28.0));
                        if ui.add(color_btn).clicked() {
                            self.show_color_picker = !self.show_color_picker;
                        }
                        ui.label(egui::RichText::new("Color:").size(12.0).color(label_col));

                        if let Some(img) = &self.image {
                            let info = format!("{}x{}", img.width(), img.height());
                            ui.label(egui::RichText::new(info).size(12.0).color(label_col));
                            ui.label(egui::RichText::new(format!("{:.0}%", self.zoom * 100.0)).size(12.0).color(label_col));
                            ui.label(egui::RichText::new("Zoom:").size(12.0).color(label_col));
                        }
                    });
                });
            });
    }

    fn render_filter_panel(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        ui.spacing_mut().slider_width = 120.0;
        if self.filter_panel == FilterPanel::None { return; }
        let (bg, border, text_col, label_col) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::BLUE_600, ColorPalette::ZINC_100, ColorPalette::ZINC_400)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::BLUE_600, ColorPalette::GRAY_900, ColorPalette::ZINC_600)
        };

        egui::Frame::new()
            .fill(bg)
            .stroke(egui::Stroke::new(1.5, border))
            .corner_radius(6.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                if self.is_processing {
                    let progress_val = *self.filter_progress.lock().unwrap();
                    
                    ui.label(egui::RichText::new("Processing Filter...").size(13.0).color(text_col));
                    ui.add_space(8.0);
                    
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width().min(300.0), 28.0),
                        egui::Sense::hover(),
                    );
                    
                    let progress_bg = if matches!(theme, ThemeMode::Dark) {
                        ColorPalette::ZINC_700
                    } else {
                        ColorPalette::GRAY_200
                    };
                    
                    ui.painter().rect_filled(rect, 4.0, progress_bg);
                    
                    let fill_rect = egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(rect.width() * progress_val, rect.height()),
                    );
                    ui.painter().rect_filled(fill_rect, 4.0, ColorPalette::BLUE_500);
                    
                    let progress_text = format!("{:.0}%", progress_val * 100.0);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &progress_text,
                        egui::FontId::proportional(13.0),
                        egui::Color32::WHITE,
                    );
                    
                    return;
                }
                
                match self.filter_panel {
                    FilterPanel::BrightnessContrast => {
                        ui.label(egui::RichText::new("Brightness / Contrast").size(13.0).color(text_col));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Brightness:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.brightness, -100.0..=100.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Contrast:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.contrast, -100.0..=100.0));
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked() { self.push_undo(); self.apply_brightness_contrast(); }
                            if ui.button("Cancel").clicked() { self.brightness = 0.0; self.contrast = 0.0; self.filter_panel = FilterPanel::None; }
                        });
                    }
                    FilterPanel::HueSaturation => {
                        ui.label(egui::RichText::new("Hue / Saturation").size(13.0).color(text_col));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Hue:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.hue, -180.0..=180.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Saturation:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.saturation, -100.0..=100.0));
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked() { self.push_undo(); self.apply_hue_saturation(); }
                            if ui.button("Cancel").clicked() { self.hue = 0.0; self.saturation = 0.0; self.filter_panel = FilterPanel::None; }
                        });
                    }
                    FilterPanel::Blur => {
                        ui.label(egui::RichText::new("Gaussian Blur").size(13.0).color(text_col));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Radius:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.blur_radius, 0.5..=20.0));
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked() { self.push_undo(); self.apply_blur(); }
                            if ui.button("Cancel").clicked() { self.blur_radius = 3.0; self.filter_panel = FilterPanel::None; }
                        });
                    }
                    FilterPanel::Sharpen => {
                        ui.label(egui::RichText::new("Sharpen").size(13.0).color(text_col));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Amount:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.sharpen_amount, 0.1..=5.0));
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked() { self.push_undo(); self.apply_sharpen(); }
                            if ui.button("Cancel").clicked() { self.sharpen_amount = 1.0; self.filter_panel = FilterPanel::None; }
                        });
                    }
                    FilterPanel::Resize => {
                        ui.label(egui::RichText::new("Resize").size(16.0).color(text_col));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Width:").size(12.0).color(label_col));
                            let old_w = self.resize_w;
                            ui.add(egui::DragValue::new(&mut self.resize_w).range(1..=8192));
                            if self.resize_locked && self.resize_w != old_w && old_w > 0 {
                                let ratio = self.resize_w as f64 / old_w as f64;
                                self.resize_h = (self.resize_h as f64 * ratio).max(1.0) as u32;
                            }
                            ui.label(egui::RichText::new("Height:").size(12.0).color(label_col));
                            let old_h = self.resize_h;
                            ui.add(egui::DragValue::new(&mut self.resize_h).range(1..=8192));
                            if self.resize_locked && self.resize_h != old_h && old_h > 0 {
                                let ratio = self.resize_h as f64 / old_h as f64;
                                self.resize_w = (self.resize_w as f64 * ratio).max(1.0) as u32;
                            }
                        });
                        if ui.checkbox(&mut self.resize_locked, "Lock Aspect Ratio").changed() {}
                        ui.checkbox(&mut self.resize_stretch, "Stretch Image")
                            .on_hover_text("If unchecked, resizes canvas and pads with white/crops");

                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked() { self.push_undo(); self.apply_resize(); }
                            if ui.button("Cancel").clicked() {
                                if let Some(img) = &self.image { self.resize_w = img.width(); self.resize_h = img.height(); }
                                self.filter_panel = FilterPanel::None;
                            }
                        });
                    }
                    FilterPanel::Export => {
                        ui.label(egui::RichText::new("Export Image").size(13.0).color(text_col));
                        
                        ui.label(egui::RichText::new("Format:").size(12.0).color(label_col));
                        ui.horizontal_wrapped(|ui| {
                            for format in ExportFormat::all() {
                                let is_selected = self.export_format == format;
                                let (bg_color, txt_color) = if is_selected {
                                    (ColorPalette::BLUE_600, egui::Color32::WHITE)
                                } else if matches!(theme, ThemeMode::Dark) {
                                    (ColorPalette::ZINC_700, ColorPalette::ZINC_300)
                                } else {
                                    (ColorPalette::GRAY_200, ColorPalette::GRAY_800)
                                };

                                let button = egui::Button::new(
                                    egui::RichText::new(format.as_str())
                                        .size(11.0)
                                        .color(txt_color)
                                )
                                .fill(bg_color)
                                .stroke(egui::Stroke::NONE)
                                .corner_radius(4.0)
                                .min_size(egui::vec2(50.0, 24.0));

                                if ui.add(button).clicked() {
                                    self.export_format = format;
                                }
                            }
                        });

                        ui.add_space(8.0);

                        match self.export_format {
                            ExportFormat::Jpeg => {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Quality:").size(12.0).color(label_col));
                                    ui.add(egui::Slider::new(&mut self.export_jpeg_quality, 1..=100).suffix("%"));
                                });
                            }
                            ExportFormat::Ico => {
                                ui.checkbox(&mut self.export_auto_scale_ico, 
                                    egui::RichText::new("Auto-scale to 256px").size(12.0).color(label_col));
                            }
                            _ => {}
                        }

                        ui.checkbox(&mut self.export_preserve_metadata, 
                            egui::RichText::new("Preserve metadata").size(12.0).color(label_col));

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.button("Export").clicked() {
                                match self.export_image() {
                                    Ok(path) => {
                                        if let Some(cb) = &self.export_callback {
                                            cb(path);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Export error: {}", e);
                                    }
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                self.filter_panel = FilterPanel::None;
                            }
                        });
                    }
                    FilterPanel::None => {}
                }
            });
    }

    fn render_color_picker(&mut self, _ui: &mut egui::Ui, ctx: &egui::Context, theme: ThemeMode) {
        if !self.show_color_picker { return; }

        let (bg, border, text_col, weak_col) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::BLUE_600, ColorPalette::ZINC_100, ColorPalette::ZINC_400)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::BLUE_600, ColorPalette::GRAY_900, ColorPalette::ZINC_600)
        };

        egui::Window::new("Color Picker")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 60.0))
            .fixed_size(egui::vec2(340.0, 0.0))
            .frame(egui::Frame::new()
                .fill(bg)
                .stroke(egui::Stroke::new(1.5, border))
                .corner_radius(8.0)
                .inner_margin(16.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                let mut rgb = [self.color.r() as f32 / 255.0, self.color.g() as f32 / 255.0, self.color.b() as f32 / 255.0];
                let (h_current, s, v) = rgb_to_hsv_f32(rgb[0], rgb[1], rgb[2]);
                let hue_id = ui.make_persistent_id("picker_hue_state");
                let mut h = ui.data(|d| d.get_temp(hue_id)).unwrap_or(h_current);

                if s > 0.005 && v > 0.005 {
                    h = h_current;
                    ui.data_mut(|d| d.insert_temp(hue_id, h));
                }
                
                let mut color_changed = false;      
                let picker_size = egui::vec2(280.0, 280.0);
                let (rect, response) = ui.allocate_exact_size(picker_size, egui::Sense::click_and_drag());
                
                if ui.is_rect_visible(rect) {
                    let painter = ui.painter_at(rect);
                    let steps = 40;
                    let cell_w = rect.width() / steps as f32;
                    let cell_h = rect.height() / steps as f32;
                    
                    for y in 0..steps {
                        for x in 0..steps {
                            let s_cell = x as f32 / (steps - 1) as f32;
                            let v_cell = 1.0 - (y as f32 / (steps - 1) as f32);
                            let (r, g, b) = hsv_to_rgb_f32(h, s_cell, v_cell);
                            let color: egui::Color32 = egui::Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
                            let cell_rect = egui::Rect::from_min_size(
                                egui::pos2(rect.min.x + x as f32 * cell_w, rect.min.y + y as f32 * cell_h),
                                egui::vec2(cell_w.ceil(), cell_h.ceil()),
                            );
                            painter.rect_filled(cell_rect, 0.0, color);
                        }
                    }
                    
                    let cursor_x = rect.min.x + s * rect.width();
                    let cursor_y = rect.min.y + (1.0 - v) * rect.height();
                    let cursor_pos = egui::pos2(cursor_x, cursor_y);
                    
                    painter.circle_stroke(cursor_pos, 6.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
                    painter.circle_stroke(cursor_pos, 6.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
                }
                
                if response.clicked() || response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let x = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                        let y = ((pos.y - rect.min.y) / rect.height()).clamp(0.0, 1.0);
                        let s_new = x;
                        let v_new = 1.0 - y;
                        let (r, g, b) = hsv_to_rgb_f32(h, s_new, v_new);
                        rgb = [r, g, b];
                        color_changed = true;
                    }
                }
                
                ui.add_space(4.0);
                
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Hue:").size(12.0).color(weak_col));
                    let hue_size = egui::vec2(ui.available_width(), 24.0);
                    let (hue_rect, hue_response) = ui.allocate_exact_size(hue_size, egui::Sense::click_and_drag());
                    
                    if ui.is_rect_visible(hue_rect) {
                        let painter = ui.painter_at(hue_rect);
                        let steps = 60;
                        let step_w = hue_rect.width() / steps as f32;
                        
                        for i in 0..steps {
                            let h_step = (i as f32 / steps as f32) * 360.0;
                            let (r, g, b) = hsv_to_rgb_f32(h_step, 1.0, 1.0);
                            let color = egui::Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
                            let cell_rect = egui::Rect::from_min_size(
                                egui::pos2(hue_rect.min.x + i as f32 * step_w, hue_rect.min.y),
                                egui::vec2(step_w.ceil(), hue_rect.height()),
                            );
                            painter.rect_filled(cell_rect, 0.0, color);
                        }
                        
                        painter.rect_stroke(hue_rect, 2.0, egui::Stroke::new(1.0, if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 }), egui::StrokeKind::Outside);
                        
                        let hue_cursor_x = hue_rect.min.x + (h / 360.0) * hue_rect.width();
                        let hue_cursor_rect = egui::Rect::from_center_size(
                            egui::pos2(hue_cursor_x, hue_rect.center().y),
                            egui::vec2(4.0, hue_rect.height() + 2.0)
                        );
                        painter.rect_filled(hue_cursor_rect, 2.0, egui::Color32::WHITE);
                        painter.rect_stroke(hue_cursor_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Outside);
                    }
                    
                    if hue_response.clicked() || hue_response.dragged() {
                        if let Some(pos) = hue_response.interact_pointer_pos() {
                            let x = ((pos.x - hue_rect.min.x) / hue_rect.width()).clamp(0.0, 1.0);
                            let h_new = x * 360.0;
                            
                            h = h_new;
                            ui.data_mut(|d| d.insert_temp(hue_id, h));
                            
                            let (_, s_curr, v_curr) = rgb_to_hsv_f32(rgb[0], rgb[1], rgb[2]);
                            let (r, g, b) = hsv_to_rgb_f32(h_new, s_curr, v_curr);
                            rgb = [r, g, b];
                            color_changed = true;
                        }
                    }
                });
                
                self.color = egui::Color32::from_rgb(
                    (rgb[0] * 255.0) as u8,
                    (rgb[1] * 255.0) as u8,
                    (rgb[2] * 255.0) as u8,
                );
                
                if color_changed {
                    self.hex_input = RgbaColor::from_egui(self.color).to_hex();
                }
                
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                
                ui.label(egui::RichText::new("Color Values").size(13.0).color(text_col));
                
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("RGB:").size(12.0).color(weak_col));
                    let rgb_str = RgbaColor::from_egui(self.color).to_rgb_string();
                    ui.label(egui::RichText::new(&rgb_str).size(12.0).color(text_col).monospace());
                    if ui.small_button("Copy").clicked() {
                        ctx.copy_text(rgb_str);
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Hex:").size(12.0).color(weak_col));
                    let response = ui.text_edit_singleline(&mut self.hex_input);
                    if response.changed() {
                        if let Some(mut color) = RgbaColor::from_hex(&self.hex_input) {
                            color.a = 255;
                            self.color = color.to_egui();
                        }
                    }
                    if response.lost_focus() {
                        self.hex_input = RgbaColor::from_egui(self.color).to_hex();
                    }
                    if ui.small_button("Copy").clicked() {
                        ctx.copy_text(self.hex_input.clone());
                    }
                });
                
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
                
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Recent").size(13.0).color(text_col));
                    if ui.small_button("Clear").clicked() {
                        self.color_history = ColorHistory::new();
                    }
                });
                
                ui.horizontal_wrapped(|ui| {
                    let history = self.color_history.get_colors().clone();
                    for color in history.iter() {
                        let btn = egui::Button::new("")
                            .fill(color.to_egui())
                            .min_size(egui::vec2(28.0, 28.0));
                        if ui.add(btn).clicked() {
                            let mut c = *color;
                            c.a = 255;
                            self.color = c.to_egui();
                            self.hex_input = c.to_hex();
                        }
                    }
                });
                
                ui.add_space(8.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        self.add_color_to_history();
                        self.show_color_picker = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_color_picker = false;
                    }
                });
            });
    }


    fn render_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let canvas_rect = ui.available_rect_before_wrap();
        self.canvas_rect = Some(canvas_rect);

        if self.fit_on_next_frame {
            self.fit_image();
            self.fit_on_next_frame = false;
        }

        self.ensure_texture(ctx);

        let (rect, response) = ui.allocate_exact_size(canvas_rect.size(), egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        let checker_size = 16.0;
        let (c1, c2) = if ui.visuals().dark_mode {
            (egui::Color32::from_rgb(40, 40, 40), egui::Color32::from_rgb(55, 55, 55))
        } else {
            (egui::Color32::from_rgb(200, 200, 200), egui::Color32::from_rgb(220, 220, 220))
        };
        let mut cy = rect.min.y;
        while cy < rect.max.y {
            let mut cx = rect.min.x;
            let row = ((cy - rect.min.y) / checker_size) as i32;
            while cx < rect.max.x {
                let col = ((cx - rect.min.x) / checker_size) as i32;
                let color = if (row + col) % 2 == 0 { c1 } else { c2 };
                let tile = egui::Rect::from_min_size(
                    egui::pos2(cx, cy),
                    egui::vec2(checker_size, checker_size),
                );
                painter.rect_filled(tile, 0.0, color);
                cx += checker_size;
            }
            cy += checker_size;
        }

        if let (Some(tex), Some(img)) = (&self.texture, &self.image) {
            let img_w = img.width() as f32;
            let img_h = img.height() as f32;
            let scaled_w = img_w * self.zoom;
            let scaled_h = img_h * self.zoom;
            let center = canvas_rect.center();
            let img_rect = egui::Rect::from_center_size(
                egui::pos2(center.x + self.pan.x, center.y + self.pan.y),
                egui::vec2(scaled_w, scaled_h),
            );
            painter.image(*tex, img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);

            painter.rect_stroke(img_rect, 0.0, egui::Stroke::new(1.0, ColorPalette::ZINC_500), egui::StrokeKind::Outside);
        }

        if self.tool == Tool::Crop {
            if let (Some(s), Some(e)) = (self.crop_state.start, self.crop_state.end) {
                let p0 = self.image_to_screen(s.0, s.1);
                let p1 = self.image_to_screen(e.0, e.1);
                let crop_rect = egui::Rect::from_two_pos(p0, p1);
                painter.rect_stroke(crop_rect, 0.0, egui::Stroke::new(2.0, ColorPalette::BLUE_400), egui::StrokeKind::Outside);
                let overlay = egui::Color32::from_rgba_premultiplied(0, 0, 0, 60);
                if crop_rect.min.y > canvas_rect.min.y {
                    painter.rect_filled(egui::Rect::from_min_max(canvas_rect.min, egui::pos2(canvas_rect.max.x, crop_rect.min.y)), 0.0, overlay);
                }
                if crop_rect.max.y < canvas_rect.max.y {
                    painter.rect_filled(egui::Rect::from_min_max(egui::pos2(canvas_rect.min.x, crop_rect.max.y), canvas_rect.max), 0.0, overlay);
                }
                if crop_rect.min.x > canvas_rect.min.x {
                    painter.rect_filled(egui::Rect::from_min_max(egui::pos2(canvas_rect.min.x, crop_rect.min.y), egui::pos2(crop_rect.min.x, crop_rect.max.y)), 0.0, overlay);
                }
                if crop_rect.max.x < canvas_rect.max.x {
                    painter.rect_filled(egui::Rect::from_min_max(egui::pos2(crop_rect.max.x, crop_rect.min.y), egui::pos2(canvas_rect.max.x, crop_rect.max.y)), 0.0, overlay);
                }
            }
        }

        for layer in &self.text_layers {
            let anchor = self.image_to_screen(layer.img_x, layer.img_y);
            let font_size_screen = layer.font_size * self.zoom;
            let angle_rad = layer.rotation.to_radians();
            let (cos_a, sin_a) = (angle_rad.cos(), angle_rad.sin());

            let font_family = egui::FontFamily::Name(layer.font_family_name().into());
            let font_id = egui::FontId::new(font_size_screen, font_family);
            let box_w_screen = layer.box_width.map(|w| w * self.zoom).unwrap_or(f32::INFINITY);

            let make_job = |text: &str| {
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = box_w_screen;
                job.append(text, 0.0, egui::TextFormat {
                    font_id: font_id.clone(),
                    color: layer.color,
                    italics: false,
                    underline: if layer.underline {
                        egui::Stroke::new((font_size_screen * 0.06).max(1.0), layer.color)
                    } else {
                        egui::Stroke::NONE
                    },
                    ..Default::default()
                });
                job
            };

            let sel_rect = layer.screen_rect(anchor, self.zoom);
            let center = sel_rect.center();
            let d = anchor - center;
            let text_pos = center + egui::vec2(d.x * cos_a - d.y * sin_a, d.x * sin_a + d.y * cos_a);
            let galley = ui.painter().layout_job(make_job(&layer.content));
            let mut text_shape = egui::epaint::TextShape::new(text_pos, galley.clone(), layer.color);
            text_shape.angle = angle_rad;

            if self.editing_text && self.selected_text == Some(layer.id) {
                let cursor_byte = self.text_cursor;
                let sel_anchor = self.text_sel_anchor;
                let content = &layer.content;
                let glyph_pos_for = |byte_off: usize| -> egui::Pos2 {
                    let char_idx = content[..byte_off.min(content.len())].chars().count();
                    let mut ci = 0usize;
                    for row in &galley.rows {
                        for g in &row.glyphs {
                            if ci == char_idx {
                                return egui::pos2(g.pos.x, row.rect().min.y);
                            }
                            ci += 1;
                        }
                        if ci == char_idx {
                            return egui::pos2(row.rect().max.x, row.rect().min.y);
                        }
                    }
                    if let Some(last_row) = galley.rows.last() {
                        egui::pos2(last_row.rect().max.x, last_row.rect().min.y)
                    } else {
                        egui::pos2(0.0, 0.0)
                    }
                };

                let galley_to_canvas = |lp: egui::Pos2| -> egui::Pos2 {
                    text_pos + egui::vec2(lp.x * cos_a - lp.y * sin_a, lp.x * sin_a + lp.y * cos_a)
                };

                if let Some(anchor) = sel_anchor {
                    let (lo, hi) = (anchor.min(cursor_byte), anchor.max(cursor_byte));
                    let char_lo = content[..lo.min(content.len())].chars().count();
                    let char_hi = content[..hi.min(content.len())].chars().count();
                    let mut ci = 0usize;
                    for row in &galley.rows {
                        let row_start = ci;
                        let row_end = ci + row.glyphs.len();
                        let sel_start_in_row = char_lo.max(row_start);
                        let sel_end_in_row   = char_hi.min(row_end);
                        if sel_start_in_row < sel_end_in_row || (char_lo <= row_start && char_hi >= row_end) {
                            let x0 = if sel_start_in_row <= row_start {
                                row.rect().min.x
                            } else {
                                row.glyphs.get(sel_start_in_row - row_start).map(|g| g.pos.x).unwrap_or(row.rect().min.x)
                            };
                            let x1 = if sel_end_in_row >= row_end {
                                row.rect().max.x
                            } else {
                                row.glyphs.get(sel_end_in_row - row_start).map(|g| g.pos.x).unwrap_or(row.rect().max.x)
                            };
                            let y0 = row.rect().min.y;
                            let y1 = row.rect().max.y;
                            let corners = [
                                galley_to_canvas(egui::pos2(x0, y0)),
                                galley_to_canvas(egui::pos2(x1, y0)),
                                galley_to_canvas(egui::pos2(x1, y1)),
                                galley_to_canvas(egui::pos2(x0, y1)),
                            ];
                            painter.add(egui::Shape::convex_polygon(
                                corners.to_vec(),
                                egui::Color32::from_rgba_unmultiplied(100, 140, 255, 80),
                                egui::Stroke::NONE,
                            ));
                        }
                        ci = row_end;
                    }
                }

                let blink = (ctx.input(|i| i.time) * 2.0) as u32 % 2 == 0;
                if blink {
                    let lp = glyph_pos_for(cursor_byte);
                    let row_h = galley.rows.iter().find(|r| r.rect().min.y <= lp.y && lp.y <= r.rect().max.y)
                        .map(|r| r.rect().height()).unwrap_or(font_size_screen);
                    let top = galley_to_canvas(lp);
                    let bot = galley_to_canvas(egui::pos2(lp.x, lp.y + row_h));
                    painter.line_segment([top, bot], egui::Stroke::new(2.0, layer.color));
                }
                ctx.request_repaint();
            }

            painter.add(egui::Shape::Text(text_shape));

            if self.selected_text == Some(layer.id) {
                let handles = TransformHandleSet::with_rotation(sel_rect, angle_rad);
                handles.draw(&painter, ColorPalette::BLUE_400);
            }
        }

        let zoom = self.zoom;
        let height_updates: Vec<(u64, f32)> = self.text_layers.iter().map(|layer| {
            let font_size_screen = layer.font_size * zoom;
            let font_family = egui::FontFamily::Name(layer.font_family_name().into());
            let font_id = egui::FontId::new(font_size_screen, font_family);
            let box_w_screen = layer.box_width.map(|w| w * zoom).unwrap_or(f32::INFINITY);
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = box_w_screen;
            job.append(&layer.content, 0.0, egui::TextFormat {
                font_id, color: layer.color, ..Default::default()
            });
            let galley = ui.painter().layout_job(job);
            (layer.id, galley.rect.height() / zoom)
        }).collect();
        for (id, h) in height_updates {
            if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                layer.rendered_height = h.max(layer.font_size);
            }
        }

        let mouse_pos = ui.input(|i| i.pointer.latest_pos());
        if let Some(mp) = mouse_pos {
            if canvas_rect.contains(mp) {
                match self.tool {
                    Tool::Brush => {
                        let r = self.brush_size / 2.0 * self.zoom;
                        painter.circle_stroke(mp, r, egui::Stroke::new(1.5, self.color));
                    }
                    Tool::Eraser => {
                        let r = self.eraser_size / 2.0 * self.zoom;
                        painter.circle_stroke(mp, r, egui::Stroke::new(1.5, ColorPalette::RED_400));
                    }
                    Tool::Text => {
                        if let Some(handles) = self.text_transform_handles() {
                            if let Some(h) = handles.hit_test(mp) {
                                ctx.set_cursor_icon(TransformHandleSet::cursor_for(h));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if response.dragged_by(egui::PointerButton::Primary) {
            let drag_delta = response.drag_delta();
            let pos = response.interact_pointer_pos().unwrap_or(canvas_rect.center());

            match self.tool {
                Tool::Pan => {
                    self.pan += drag_delta;
                }
                Tool::Brush | Tool::Eraser => {
                    if !self.is_dragging {
                        self.push_undo();
                        self.is_dragging = true;
                        self.stroke_points.clear();
                    }
                    if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.stroke_points.push((ix as f32, iy as f32));
                        if self.stroke_points.len() >= 2 {
                            self.apply_brush_stroke();
                            let last = *self.stroke_points.last().unwrap();
                            self.stroke_points.clear();
                            self.stroke_points.push(last);
                        }
                    }
                }
                Tool::Crop => {
                    if let Some((ix, iy)) = self.screen_to_image(pos) {
                        if self.crop_state.start.is_none() {
                            self.crop_state.start = Some((ix as f32, iy as f32));
                        }
                        self.crop_state.end = Some((ix as f32, iy as f32));
                    }
                }
                Tool::Text => {
                    let drag_data = self.text_drag.as_ref().map(|d| (
                        d.handle, d.start, d.orig_img_x, d.orig_img_y,
                        d.orig_font_size, d.orig_box_width, d.orig_box_height,
                        d.orig_rotation, d.orig_rot_start_angle,
                    ));
                    if let (Some(id), Some((handle, drag_start, orig_ix, orig_iy, orig_fs, orig_bw, orig_bh, orig_rot, orig_rot_start))) =
                        (self.selected_text, drag_data)
                    {
                        let zoom = self.zoom;
                        let anchor_screen = self.image_to_screen(orig_ix, orig_iy);
                        let canvas = self.canvas_rect.unwrap_or(egui::Rect::NOTHING);
                        let (img_w, img_h) = self.image.as_ref()
                            .map(|i| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
                        let ox = canvas.center().x - img_w * zoom / 2.0 + self.pan.x;
                        let oy = canvas.center().y - img_h * zoom / 2.0 + self.pan.y;

                        let orig_w_screen = orig_bw.map(|bw| bw * zoom).unwrap_or_else(|| {
                            let max_chars = self.text_layers.iter().find(|l| l.id == id)
                                .map(|l| l.max_line_chars()).unwrap_or(1);
                            (max_chars as f32 * orig_fs * 0.58 * zoom).max(orig_fs * zoom)
                        });
                        let orig_h_screen = orig_bh.map(|bh| bh * zoom).unwrap_or_else(|| {
                            let lines = self.text_layers.iter().find(|l| l.id == id)
                                .map(|l| l.line_count()).unwrap_or(1);
                            lines as f32 * orig_fs * 1.35 * zoom
                        });
                        let rot_center = anchor_screen + egui::vec2(orig_w_screen / 2.0, orig_h_screen / 2.0);

                        if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                            let min_sz = orig_fs * 0.5 * zoom;
                            match handle {
                                THandle::Move => {
                                    let delta = pos - drag_start;
                                    layer.img_x = orig_ix + delta.x / zoom;
                                    layer.img_y = orig_iy + delta.y / zoom;
                                }
                                THandle::E => {
                                    let new_w = (pos.x - anchor_screen.x).max(min_sz);
                                    layer.box_width = Some((new_w / zoom).max(1.0));
                                }
                                THandle::W => {
                                    let orig_right = anchor_screen.x + orig_w_screen;
                                    let new_w = (orig_right - pos.x).max(min_sz);
                                    layer.box_width = Some((new_w / zoom).max(1.0));
                                    layer.img_x = (pos.x - ox) / zoom;
                                }
                                THandle::S => {
                                    let new_h = (pos.y - anchor_screen.y).max(min_sz);
                                    layer.box_height = Some((new_h / zoom).max(1.0));
                                }
                                THandle::N => {
                                    let orig_bottom = anchor_screen.y + orig_h_screen;
                                    let new_h = (orig_bottom - pos.y).max(min_sz);
                                    layer.box_height = Some((new_h / zoom).max(1.0));
                                    layer.img_y = ((orig_bottom - new_h) - oy) / zoom;
                                }
                                THandle::SE => {
                                    layer.box_width  = Some(((pos.x - anchor_screen.x).max(min_sz) / zoom).max(1.0));
                                    layer.box_height = Some(((pos.y - anchor_screen.y).max(min_sz) / zoom).max(1.0));
                                }
                                THandle::NE => {
                                    let orig_bottom = anchor_screen.y + orig_h_screen;
                                    let new_h = (orig_bottom - pos.y).max(min_sz);
                                    layer.box_width  = Some(((pos.x - anchor_screen.x).max(min_sz) / zoom).max(1.0));
                                    layer.box_height = Some((new_h / zoom).max(1.0));
                                    layer.img_y = ((orig_bottom - new_h) - oy) / zoom;
                                }
                                THandle::NW => {
                                    let orig_right  = anchor_screen.x + orig_w_screen;
                                    let orig_bottom = anchor_screen.y + orig_h_screen;
                                    let new_w = (orig_right  - pos.x).max(min_sz);
                                    let new_h = (orig_bottom - pos.y).max(min_sz);
                                    layer.box_width  = Some((new_w / zoom).max(1.0));
                                    layer.box_height = Some((new_h / zoom).max(1.0));
                                    layer.img_x = (pos.x - ox) / zoom;
                                    layer.img_y = ((orig_bottom - new_h) - oy) / zoom;
                                }
                                THandle::SW => {
                                    let orig_right = anchor_screen.x + orig_w_screen;
                                    let new_w = (orig_right - pos.x).max(min_sz);
                                    layer.box_width  = Some((new_w / zoom).max(1.0));
                                    layer.box_height = Some(((pos.y - anchor_screen.y).max(min_sz) / zoom).max(1.0));
                                    layer.img_x = (pos.x - ox) / zoom;
                                }
                                THandle::Rotate => {
                                    let cur_angle = (pos - rot_center).angle();
                                    let delta = cur_angle - orig_rot_start;
                                    layer.rotation = orig_rot + delta.to_degrees();
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary) && self.tool == Tool::Crop {
            self.crop_state = CropState::default();
            let pos = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            if let Some((ix, iy)) = self.screen_to_image(pos) {
                self.crop_state.start = Some((ix as f32, iy as f32));
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary) && self.tool == Tool::Text {
            let pos = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            self.text_drag = None;

            if let Some(id) = self.selected_text {
                if let Some(handles) = self.text_transform_handles() {
                    if let Some(h) = handles.hit_test(pos) {
                        if let Some(layer) = self.text_layers.iter().find(|l| l.id == id) {
                            let anchor = self.image_to_screen(layer.img_x, layer.img_y);
                            let rect = layer.screen_rect(anchor, self.zoom);
                            let rot_start = (pos - rect.center()).angle();
                            self.text_drag = Some(TextDrag {
                                handle: h,
                                start: pos,
                                orig_img_x: layer.img_x,
                                orig_img_y: layer.img_y,
                                orig_font_size: layer.font_size,
                                orig_box_width: layer.box_width,
                                orig_box_height: layer.box_height,
                                orig_rotation: layer.rotation,
                                orig_rot_start_angle: rot_start,
                            });
                        }
                    }
                }
            }
        }

        if response.drag_stopped_by(egui::PointerButton::Primary) {
            match self.tool {
                Tool::Brush | Tool::Eraser => {
                    self.texture_dirty = true;
                    self.stroke_points.clear();
                    self.is_dragging = false;
                }
                Tool::Text => {
                    self.text_drag = None;
                }
                _ => {}
            }
        }

        if response.clicked_by(egui::PointerButton::Primary) {
            let pos = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            match self.tool {
                Tool::Brush | Tool::Eraser => {
                    if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.push_undo();
                        self.stroke_points.clear();
                        self.stroke_points.push((ix as f32, iy as f32));
                        self.stroke_points.push((ix as f32 + 0.1, iy as f32 + 0.1));
                        self.apply_brush_stroke();
                        self.stroke_points.clear();
                        if self.tool == Tool::Brush {
                            self.add_color_to_history();
                        }
                    }
                }
                Tool::Fill => {
                    if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.push_undo();
                        self.flood_fill(ix, iy);
                        self.add_color_to_history();
                    }
                }
                Tool::Eyedropper => {
                    if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.sample_color(ix, iy);
                    }
                }
                Tool::Text => {
                    if let Some(hit) = self.hit_text_layer(pos) {
                        self.selected_text = Some(hit);
                        self.editing_text = true;
                        self.text_sel_anchor = None;
                        if let Some(layer) = self.text_layers.iter().find(|l| l.id == hit) {
                            self.text_font_size = layer.font_size;
                            self.text_bold = layer.bold;
                            self.text_italic = layer.italic;
                            self.text_underline = layer.underline;
                            self.text_font_name = layer.font_name.clone();
                            self.text_cursor = layer.content.len();
                        }
                    } else {
                        self.commit_or_discard_active_text();
                        if let Some((ix, iy)) = self.screen_to_image(pos) {
                            let id = self.next_text_id;
                            self.next_text_id += 1;
                            self.text_layers.push(TextLayer {
                                id,
                                content: String::new(),
                                img_x: ix as f32,
                                img_y: iy as f32,
                                font_size: self.text_font_size,
                                box_width: Some(300.0),
                                box_height: None,
                                rotation: 0.0,
                                color: self.color,
                                bold: self.text_bold,
                                italic: self.text_italic,
                                underline: self.text_underline,
                                font_name: self.text_font_name.clone(),
                                rendered_height: 0.0,
                            });
                            self.selected_text = Some(id);
                            self.editing_text = true;
                            self.text_cursor = 0;
                            self.text_sel_anchor = None;
                        }
                    }
                }
                _ => {}
            }
        }

        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 && canvas_rect.contains(mouse_pos.unwrap_or(canvas_rect.center())) {
            let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
            self.zoom = (self.zoom * factor).clamp(0.01, 50.0);
        }

        if response.dragged_by(egui::PointerButton::Middle) {
            self.pan += response.drag_delta();
        }
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
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
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Plus) { self.zoom *= 1.25; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Minus) { self.zoom = (self.zoom / 1.25).max(0.01); }
            });
        }
    }

    fn save_impl(&mut self) -> Result<(), String> {
        let path = match &self.file_path {
            Some(p) => p.clone(),
            None => return self.save_as_impl(),
        };
        if let Some(img) = &self.image {
            let composite = self.stamp_all_text_layers(img);
            composite.save(&path).map_err(|e| e.to_string())?;
            self.dirty = false;
        }
        Ok(())
    }

    fn save_as_impl(&mut self) -> Result<(), String> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "tiff", "gif"])
            .save_file()
        {
            if let Some(img) = &self.image {
                let composite = self.stamp_all_text_layers(img);
                composite.save(&path).map_err(|e| e.to_string())?;
                self.file_path = Some(path);
                self.dirty = false;
            }
            Ok(())
        } else {
            Err("Cancelled".to_string())
        }
    }

    fn check_filter_completion(&mut self) {
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
}

fn rgb_to_hsv_f32(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (if h < 0.0 { h + 360.0 } else { h }, s, v)
}

fn hsv_to_rgb_f32(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
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

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let h = if delta == 0.0 { 0.0 } else if max == r { 60.0 * (((g - b) / delta) % 6.0) } else if max == g { 60.0 * ((b - r) / delta + 2.0) } else { 60.0 * ((r - g) / delta + 4.0) };
    (if h < 0.0 { h + 360.0 } else { h }, s, v)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0), 60..=119 => (x, c, 0.0), 120..=179 => (0.0, c, x), 180..=239 => (0.0, x, c), 240..=299 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}

impl EditorModule for ImageEditor {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn get_title(&self) -> String {
        let name = self.file_path.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled");
        if self.dirty { format!("{} *", name) } else { name.to_string() }
    }

    fn save(&mut self) -> Result<(), String> { self.save_impl() }
    fn save_as(&mut self) -> Result<(), String> { self.save_as_impl() }

    fn get_menu_contributions(&self) -> MenuContribution {
        let has_image = self.image.is_some();
        let has_undo = !self.undo_stack.is_empty();
        let has_redo = !self.redo_stack.is_empty();
        MenuContribution {
            file_items: vec![ (MenuItem { label: "Export...".to_string(), shortcut: None, enabled: has_image }, MenuAction::Export) ],
            edit_items: vec![ 
                (MenuItem { label: "Undo".to_string(), shortcut: Some("Ctrl+Z".to_string()), enabled: has_undo }, MenuAction::Undo), 
                (MenuItem { label: "Redo".to_string(), shortcut: Some("Ctrl+Y".to_string()), enabled: has_redo }, MenuAction::Redo) 
            ],
            view_items: vec![
                (MenuItem { label: "Zoom In".to_string(), shortcut: Some("+".to_string()), enabled: true }, MenuAction::Custom("Zoom In".to_string())), 
                (MenuItem { label: "Zoom Out".to_string(), shortcut: Some("-".to_string()), enabled: true }, MenuAction::Custom("Zoom Out".to_string())), 
                (MenuItem { label: "Fit".to_string(), shortcut: Some("0".to_string()), enabled: true }, MenuAction::Custom("Fit".to_string())) 
            ],
            image_items: vec![
                (MenuItem { label: "Resize Canvas...".to_string(), shortcut: None, enabled: self.image.is_some() }, MenuAction::Custom("Resize Canvas".to_string())),
                (MenuItem { label: "Seperator".to_string(), shortcut: None, enabled: false}, MenuAction::None),
                (MenuItem { label: "Flip Horizontal".to_string(), shortcut: None, enabled: true }, MenuAction::Custom("Flip Horizontal".to_string())), 
                (MenuItem { label: "Flip Vertical".to_string(), shortcut: None, enabled: true }, MenuAction::Custom("Flip Vertical".to_string())), 
                (MenuItem { label: "Rotate CCW".to_string(), shortcut: None, enabled: true }, MenuAction::Custom("Rotate CCW".to_string())), 
                (MenuItem { label: "Rotate CW".to_string(), shortcut: None, enabled: true },MenuAction::Custom("Rotate CW".to_string()))
            ],
            filter_items: vec![
                (MenuItem { label: "Brightness/Contrast...".to_string(), shortcut: None, enabled: self.image.is_some() }, MenuAction::Custom("B/C".to_string())),
                (MenuItem { label: "Hue/Saturation...".to_string(), shortcut: None, enabled: self.image.is_some() }, MenuAction::Custom("H/S".to_string())),
                (MenuItem { label: "Blur...".to_string(), shortcut: None, enabled: self.image.is_some() }, MenuAction::Custom("Blur".to_string())),
                (MenuItem { label: "Sharpen...".to_string(), shortcut: None, enabled: self.image.is_some() }, MenuAction::Custom("Sharpen".to_string())),
                (MenuItem { label: "Grayscale".to_string(), shortcut: None, enabled: self.image.is_some() }, MenuAction::Custom("Gray".to_string())),
                (MenuItem { label: "Invert".to_string(), shortcut: None, enabled: self.image.is_some() }, MenuAction::Custom("Invert".to_string())),
                (MenuItem { label: "Sepia".to_string(), shortcut: None, enabled: self.image.is_some() }, MenuAction::Custom("Sepia".to_string()))
            ],
        }
    }

    fn handle_menu_action(&mut self, action: MenuAction) -> bool {
        match action {
            MenuAction::Undo => { self.undo(); true }
            MenuAction::Redo => { self.redo(); true }
            MenuAction::Export => { self.filter_panel = FilterPanel::Export; true }
            MenuAction::Custom(val) if val == "Zoom In" => { self.zoom *= 1.25; true }
            MenuAction::Custom(val) if val == "Zoom Out" => { self.zoom = (self.zoom / 1.25).max(0.01); true }
            MenuAction::Custom(val) if val == "Fit" => { self.fit_image(); true }
            MenuAction::Custom(val) if val == "Flip Horizontal" => { self.push_undo(); self.apply_flip_h(); true }
            MenuAction::Custom(val) if val == "Flip Vertical" => { self.push_undo(); self.apply_flip_v(); true }
            MenuAction::Custom(val) if val == "Rotate CCW" => { self.push_undo(); self.apply_rotate_ccw(); true }
            MenuAction::Custom(val) if val == "Rotate CW" => { self.push_undo(); self.apply_rotate_cw(); true }
            MenuAction::Custom(val) if val == "Resize Canvas" => { self.filter_panel = FilterPanel::Resize; true }
            MenuAction::Custom(val) if val == "B/C" => { self.filter_panel = FilterPanel::BrightnessContrast; true }
            MenuAction::Custom(val) if val == "H/S" => { self.filter_panel = FilterPanel::HueSaturation; true }
            MenuAction::Custom(val) if val == "Blur" => { self.filter_panel = FilterPanel::Blur; true }
            MenuAction::Custom(val) if val == "Sharpen" => { self.filter_panel = FilterPanel::Sharpen; true }
            MenuAction::Custom(val) if val == "Gray" => { self.push_undo(); self.apply_grayscale(); true }
            MenuAction::Custom(val) if val == "Invert" => { self.push_undo(); self.apply_invert(); true }
            MenuAction::Custom(val) if val == "Sepia" => { self.push_undo(); self.apply_sepia(); true }
            _ => false,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, _show_file_info: bool) {
        let theme = if ui.visuals().dark_mode { ThemeMode::Dark } else { ThemeMode::Light };

        self.ensure_fonts_registered(ctx);
        self.handle_keyboard(ctx);
        self.check_filter_completion();

        if self.is_processing {
            ctx.request_repaint();
        }

        if self.image.is_none() && self.file_path.is_none() {
            self.new_image(800, 600);
        }

        self.render_toolbar(ui, theme);
        ui.add_space(4.0);
        self.render_options_bar(ui, theme);
        ui.add_space(4.0);

        if self.filter_panel != FilterPanel::None {
            self.render_filter_panel(ui, theme);
            ui.add_space(4.0);
        }

        if self.show_color_picker {
            self.render_color_picker(ui, ctx, theme);
        }

        self.render_canvas(ui, ctx);
    }
}
