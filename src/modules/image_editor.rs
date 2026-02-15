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

struct TextState { content: String, font_size: u32, placing: bool, pos: Option<(u32, u32)> }

impl Default for TextState {
    fn default() -> Self {
        Self { content: String::new(), font_size: 24, placing: false, pos: None }
    }
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

    text_state: TextState,
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
    text_focused: bool,

    filter_progress: Arc<Mutex<f32>>,
    is_processing: bool,
    pending_filter_result: Arc<Mutex<Option<DynamicImage>>>,
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
            text_state: TextState::default(),
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
            text_focused: false,
            filter_progress: Arc::new(Mutex::new(0.0)),
            is_processing: false,
            pending_filter_result: Arc::new(Mutex::new(None)),
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

    fn stamp_text(&mut self) {
        let img = match self.image.as_mut() {
            Some(i) => i,
            None => return,
        };
        let (tx, ty) = match self.text_state.pos {
            Some(p) => p,
            None => return,
        };
        if self.text_state.content.is_empty() { return; }
        let mut buf = img.to_rgba8();
        let w = buf.width();
        let h = buf.height();
        let (r, g, b, a) = (self.color.r(), self.color.g(), self.color.b(), self.color.a());
        let font_size = self.text_state.font_size.max(4) as f32;
        let char_w = (font_size * 0.6) as u32;
        let char_h = font_size as u32;
        for (i, _ch) in self.text_state.content.chars().enumerate() {
            let cx = tx + (i as u32) * char_w;
            if cx + char_w > w { break; }
            let x_start = cx;
            let x_end = (cx + char_w).min(w);
            let y_start = ty;
            let y_end = (ty + char_h).min(h);
            for py in y_start..y_end {
                for px in x_start..x_end {
                    let existing = buf.get_pixel(px, py).0;
                    let fa = a as f32 / 255.0;
                    let fb = 1.0 - fa;
                    let nr = (r as f32 * fa + existing[0] as f32 * fb).min(255.0) as u8;
                    let ng = (g as f32 * fa + existing[1] as f32 * fb).min(255.0) as u8;
                    let nb = (b as f32 * fa + existing[2] as f32 * fb).min(255.0) as u8;
                    let na = ((a as f32 + existing[3] as f32 * fb).min(255.0)) as u8;
                    buf.put_pixel(px, py, Rgba([nr, ng, nb, na]));
                }
            }
        }
        self.image = Some(DynamicImage::ImageRgba8(buf));
        self.texture_dirty = true;
        self.dirty = true;
        self.text_state = TextState::default();
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

    fn apply_flip_h(&mut self) {
        if let Some(img) = &self.image {
            self.image = Some(img.fliph());
            self.texture_dirty = true;
            self.dirty = true;
        }
    }

    fn apply_flip_v(&mut self) {
        if let Some(img) = &self.image {
            self.image = Some(img.flipv());
            self.texture_dirty = true;
            self.dirty = true;
        }
    }

    fn apply_rotate_cw(&mut self) {
        if let Some(img) = &self.image {
            self.image = Some(img.rotate90());
            self.resize_w = self.image.as_ref().unwrap().width();
            self.resize_h = self.image.as_ref().unwrap().height();
            self.texture_dirty = true;
            self.dirty = true;
            self.fit_on_next_frame = true;
        }
    }

    fn apply_rotate_ccw(&mut self) {
        if let Some(img) = &self.image {
            self.image = Some(img.rotate270());
            self.resize_w = self.image.as_ref().unwrap().width();
            self.resize_h = self.image.as_ref().unwrap().height();
            self.texture_dirty = true;
            self.dirty = true;
            self.fit_on_next_frame = true;
        }
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
            img, 
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

                            ui.separator();
                            ui.label(egui::RichText::new("Transform").size(12.0).color(ColorPalette::ZINC_500));
                            if self.toolbar_btn(ui, "Flip H", None, theme).clicked() { self.push_undo(); self.apply_flip_h(); }
                            if self.toolbar_btn(ui, "Flip V", None, theme).clicked() { self.push_undo(); self.apply_flip_v(); }
                            if self.toolbar_btn(ui, "Rot CW", None, theme).clicked() { self.push_undo(); self.apply_rotate_cw(); }
                            if self.toolbar_btn(ui, "Rot CCW", None, theme).clicked() { self.push_undo(); self.apply_rotate_ccw(); }

                            ui.separator();
                            ui.label(egui::RichText::new("Filters").size(12.0).color(ColorPalette::ZINC_500));
                            if self.toolbar_btn(ui, "B/C", None, theme).clicked() { self.filter_panel = FilterPanel::BrightnessContrast; }
                            if self.toolbar_btn(ui, "H/S", None, theme).clicked() { self.filter_panel = FilterPanel::HueSaturation; }
                            if self.toolbar_btn(ui, "Blur", None, theme).clicked() { self.filter_panel = FilterPanel::Blur; }
                            if self.toolbar_btn(ui, "Sharpen", None, theme).clicked() { self.filter_panel = FilterPanel::Sharpen; }
                            if self.toolbar_btn(ui, "Gray", None, theme).clicked() { self.push_undo(); self.apply_grayscale(); }
                            if self.toolbar_btn(ui, "Invert", None, theme).clicked() { self.push_undo(); self.apply_invert(); }
                            if self.toolbar_btn(ui, "Sepia", None, theme).clicked() { self.push_undo(); self.apply_sepia(); }

                            ui.separator();
                            if self.toolbar_btn(ui, "Resize", None, theme).clicked() { self.filter_panel = FilterPanel::Resize; }
                        });
                    });
            });
    }

    fn toolbar_btn(&self, ui: &mut egui::Ui, label: &str, shortcut: Option<&str>, theme: ThemeMode) -> egui::Response {
        let (bg, hover, txt) = if matches!(theme, ThemeMode::Dark) {
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
            ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(txt)).min_size(egui::vec2(0.0, 24.0)))
        }).inner;
        
        if let Some(sc) = shortcut {
            response.on_hover_text(sc)
        } else {
            response
        }
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
        if response.clicked() { self.tool = tool; }
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
                            ui.label(egui::RichText::new("Font size:").size(12.0).color(label_col));
                            ui.add(egui::DragValue::new(&mut self.text_state.font_size).range(4..=200));
                            ui.label(egui::RichText::new("Text:").size(12.0).color(label_col));
                            let te_resp = ui.text_edit_singleline(&mut self.text_state.content);
                            self.text_focused = te_resp.has_focus();
                            if self.text_state.placing {
                                if ui.button("Cancel").clicked() {
                                    self.text_state.placing = false;
                                    self.text_state.pos = None;
                                }
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

        if self.text_state.placing {
            if let Some((tx, ty)) = self.text_state.pos {
                let screen_pos = self.image_to_screen(tx as f32, ty as f32);
                painter.text(screen_pos, egui::Align2::LEFT_TOP, &self.text_state.content, egui::FontId::proportional(self.text_state.font_size as f32 * self.zoom), self.color);
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

        if response.drag_stopped_by(egui::PointerButton::Primary) {
            match self.tool {
                Tool::Brush | Tool::Eraser => {
                    self.texture_dirty = true;
                    self.stroke_points.clear();
                    self.is_dragging = false;
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
                    if let Some((ix, iy)) = self.screen_to_image(pos) {
                        if self.text_state.placing && self.text_state.pos.is_some() {
                            self.push_undo();
                            self.stamp_text();
                            self.add_color_to_history();
                        }
                        self.text_state.placing = true;
                        self.text_state.pos = Some((ix, iy));
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
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Z) { self.undo(); }
            if i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z) { self.redo(); }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Y) { self.redo(); }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::S) {
                if i.modifiers.shift { let _ = self.save_as_impl(); }
                else { let _ = self.save_impl(); }
            }
        });
        if !self.text_focused {
            ctx.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, egui::Key::B) { self.tool = Tool::Brush; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::E) { self.tool = Tool::Eraser; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::F) { self.tool = Tool::Fill; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::T) { self.tool = Tool::Text; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::D) { self.tool = Tool::Eyedropper; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::C) { self.tool = Tool::Crop; }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::P) { self.tool = Tool::Pan; }
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
            img.save(&path).map_err(|e| e.to_string())?;
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
                img.save(&path).map_err(|e| e.to_string())?;
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
                (MenuItem { label: "Undo".to_string(), 
                shortcut: Some("Ctrl+Z".to_string()), enabled: has_undo }, MenuAction::Undo), 
                (MenuItem { label: "Redo".to_string(), shortcut: Some("Ctrl+Y".to_string()), enabled: has_redo }, 
                MenuAction::Redo) 
            ],
            view_items: vec![
                (MenuItem { label: "Zoom In".to_string(), 
                shortcut: Some("+".to_string()), enabled: true }, 
                MenuAction::Custom("Zoom In".to_string())), 
                (MenuItem { label: "Zoom Out".to_string(), 
                shortcut: Some("-".to_string()), enabled: true }, 
                MenuAction::Custom("Zoom Out".to_string())), 
                (MenuItem { label: "Fit".to_string(), shortcut: Some("0".to_string()), enabled: true }, 
                MenuAction::Custom("Fit".to_string())) 
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
            _ => false,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, _show_file_info: bool) {
        let theme = if ui.visuals().dark_mode { ThemeMode::Dark } else { ThemeMode::Light };

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
