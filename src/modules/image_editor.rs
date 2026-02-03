use eframe::egui;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, ImageEncoder};
use std::collections::VecDeque;
use std::path::PathBuf;
use crate::style::{ColorPalette, ThemeMode};
use super::EditorModule;

const MAX_UNDO: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tool {
    Brush,
    Eraser,
    Fill,
    Text,
    Eyedropper,
    Crop,
    Pan,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FilterPanel {
    None,
    BrightnessContrast,
    HueSaturation,
    Blur,
    Sharpen,
    Resize,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Jpeg,
    Png,
    Webp,
    Bmp,
    Tiff,
    Ico,
}

impl ExportFormat {
    fn as_str(&self) -> &str {
        match self {
            ExportFormat::Jpeg => "JPEG",
            ExportFormat::Png => "PNG",
            ExportFormat::Webp => "WebP",
            ExportFormat::Bmp => "BMP",
            ExportFormat::Tiff => "TIFF",
            ExportFormat::Ico => "ICO",
        }
    }

    fn extension(&self) -> &str {
        match self {
            ExportFormat::Jpeg => "jpg",
            ExportFormat::Png => "png",
            ExportFormat::Webp => "webp",
            ExportFormat::Bmp => "bmp",
            ExportFormat::Tiff => "tiff",
            ExportFormat::Ico => "ico",
        }
    }

    fn all() -> Vec<ExportFormat> {
        vec![
            ExportFormat::Jpeg,
            ExportFormat::Png,
            ExportFormat::Webp,
            ExportFormat::Bmp,
            ExportFormat::Tiff,
            ExportFormat::Ico,
        ]
    }
}

struct TextState {
    content: String,
    font_size: u32,
    placing: bool,
    pos: Option<(u32, u32)>,
}

impl Default for TextState {
    fn default() -> Self {
        Self { content: String::new(), font_size: 24, placing: false, pos: None }
    }
}

struct CropState {
    start: Option<(f32, f32)>,
    end: Option<(f32, f32)>,
}

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

    export_format: ExportFormat,
    export_jpeg_quality: u8,
    export_preserve_metadata: bool,
    export_auto_scale_ico: bool,
    export_callback: Option<Box<dyn Fn(PathBuf) + Send + Sync>>,

    show_color_picker: bool,
    canvas_rect: Option<egui::Rect>,
    text_focused: bool,
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
            export_format: ExportFormat::Png,
            export_jpeg_quality: 90,
            export_preserve_metadata: true,
            export_auto_scale_ico: true,
            export_callback: None,
            show_color_picker: false,
            canvas_rect: None,
            text_focused: false,
        }
    }

    #[allow(dead_code)]
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

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_file_callback(&mut self, callback: Box<dyn Fn(PathBuf) + Send + Sync>) {
        self.export_callback = Some(callback);
    }

    fn push_undo(&mut self) {
        if let Some(img) = &self.image {
            self.undo_stack.push_back(img.clone());
            if self.undo_stack.len() > MAX_UNDO {
                self.undo_stack.pop_front();
            }
            self.redo_stack.clear();
        }
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop_back() {
            if let Some(current) = self.image.take() {
                self.redo_stack.push_back(current);
            }
            self.resize_w = prev.width();
            self.resize_h = prev.height();
            self.image = Some(prev);
            self.texture_dirty = true;
            self.dirty = true;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop_back() {
            if let Some(current) = self.image.take() {
                self.undo_stack.push_back(current);
            }
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
        let img_w = img.width() as f32;
        let img_h = img.height() as f32;
        let scaled_w = img_w * self.zoom;
        let scaled_h = img_h * self.zoom;
        let offset_x = canvas.center().x - scaled_w / 2.0 + self.pan.x;
        let offset_y = canvas.center().y - scaled_h / 2.0 + self.pan.y;
        let rel_x = (screen_pos.x - offset_x) / self.zoom;
        let rel_y = (screen_pos.y - offset_y) / self.zoom;
        if rel_x < 0.0 || rel_y < 0.0 || rel_x >= img_w || rel_y >= img_h {
            return None;
        }
        Some((rel_x as u32, rel_y as u32))
    }

    fn image_to_screen(&self, ix: f32, iy: f32) -> egui::Pos2 {
        let canvas = self.canvas_rect.unwrap_or(egui::Rect::NOTHING);
        let img = self.image.as_ref();
        let (img_w, img_h) = img.map(|i| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
        let scaled_w = img_w * self.zoom;
        let scaled_h = img_h * self.zoom;
        let offset_x = canvas.center().x - scaled_w / 2.0 + self.pan.x;
        let offset_y = canvas.center().y - scaled_h / 2.0 + self.pan.y;
        egui::pos2(offset_x + ix * self.zoom, offset_y + iy * self.zoom)
    }

    fn fit_image(&mut self) {
        if let Some(img) = &self.image {
            if let Some(canvas) = self.canvas_rect {
                let scale_x = canvas.width() / img.width() as f32;
                let scale_y = canvas.height() / img.height() as f32;
                self.zoom = scale_x.min(scale_y).min(1.0).max(0.01);
                self.pan = egui::Vec2::ZERO;
            }
        }
    }

    fn apply_brush_stroke(&mut self) {
        let img = match self.image.as_mut() {
            Some(i) => i,
            None => return,
        };
        let mut buf = img.to_rgba8();
        let width = buf.width();
        let height = buf.height();
        let (r, g, b, base_a) = if self.tool == Tool::Eraser {
            (0u8, 0u8, 0u8, 0u8)
        } else {
            (self.color.r(), self.color.g(), self.color.b(), self.color.a())
        };
        let radius = if self.tool == Tool::Eraser { self.eraser_size / 2.0 } else { self.brush_size / 2.0 };
        let opacity = if self.tool == Tool::Eraser { 1.0 } else { self.brush_opacity };

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
                    for px in min_x..max_x {
                        let dist_sq = (px as f32 - cx).powi(2) + (py as f32 - cy).powi(2);
                        if dist_sq <= radius * radius {
                            let falloff = 1.0 - (dist_sq / (radius * radius)).sqrt();
                            let alpha = (falloff * opacity * 255.0) as u8;
                            let pixel = buf.get_pixel(px, py);
                            let [er, eg, eb, ea] = pixel.0;
                            if self.tool == Tool::Eraser {
                                let new_a = ea.saturating_sub(alpha);
                                buf.put_pixel(px, py, Rgba([er, eg, eb, new_a]));
                            } else {
                                let fa = alpha as f32 / 255.0;
                                let fb = 1.0 - fa * (base_a as f32 / 255.0);
                                let nr = (r as f32 * fa * (base_a as f32 / 255.0) + er as f32 * fb).min(255.0) as u8;
                                let ng = (g as f32 * fa * (base_a as f32 / 255.0) + eg as f32 * fb).min(255.0) as u8;
                                let nb = (b as f32 * fa * (base_a as f32 / 255.0) + eb as f32 * fb).min(255.0) as u8;
                                let na = ((base_a as f32 * fa + ea as f32 * fb).min(255.0)) as u8;
                                buf.put_pixel(px, py, Rgba([nr, ng, nb, na]));
                            }
                        }
                    }
                }
            }
        }
        self.image = Some(DynamicImage::ImageRgba8(buf));
        self.texture_dirty = true;
        self.dirty = true;
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
        let mut stack = vec![(start_x, start_y)];
        let tolerance = 30i32;
        while let Some((x, y)) = stack.pop() {
            let cur = buf.get_pixel(x, y).0;
            let diff: i32 = [0,1,2,3].iter().map(|&i| (cur[i] as i32 - target[i] as i32).abs()).sum();
            if diff > tolerance { continue; }
            if cur == fill { continue; }
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
            let p = img.get_pixel(x, y);
            let rgba = p.0;
            self.color = egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
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
        let img = match self.image.as_mut() {
            Some(i) => i,
            None => return,
        };
        let b = self.brightness;
        let c = self.contrast;
        let mut buf = img.to_rgba8();
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                let p = buf.get_pixel(x, y).0;
                let mut channels = [p[0] as f32, p[1] as f32, p[2] as f32];
                for ch in channels.iter_mut() {
                    *ch = (*ch - 128.0) * (1.0 + c / 100.0) + 128.0 + b;
                    *ch = ch.clamp(0.0, 255.0);
                }
                buf.put_pixel(x, y, Rgba([channels[0] as u8, channels[1] as u8, channels[2] as u8, p[3]]));
            }
        }
        self.image = Some(DynamicImage::ImageRgba8(buf));
        self.texture_dirty = true;
        self.dirty = true;
        self.brightness = 0.0;
        self.contrast = 0.0;
        self.filter_panel = FilterPanel::None;
    }

    fn apply_hue_saturation(&mut self) {
        let img = match self.image.as_mut() {
            Some(i) => i,
            None => return,
        };
        let sat_factor = 1.0 + self.saturation / 100.0;
        let hue_shift = self.hue;
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
        }
        self.image = Some(DynamicImage::ImageRgba8(buf));
        self.texture_dirty = true;
        self.dirty = true;
        self.hue = 0.0;
        self.saturation = 0.0;
        self.filter_panel = FilterPanel::None;
    }

    fn apply_blur(&mut self) {
        let img = match &self.image {
            Some(i) => i,
            None => return,
        };
        let blurred = img.blur(self.blur_radius);
        self.image = Some(blurred);
        self.texture_dirty = true;
        self.dirty = true;
        self.blur_radius = 3.0;
        self.filter_panel = FilterPanel::None;
    }

    fn apply_sharpen(&mut self) {
        let amount = self.sharpen_amount;
        let img = match &self.image {
            Some(i) => i,
            None => return,
        };
        let sharpened = img.unsharpen(amount, 0);
        self.image = Some(sharpened);
        self.texture_dirty = true;
        self.dirty = true;
        self.sharpen_amount = 1.0;
        self.filter_panel = FilterPanel::None;
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
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                let p = buf.get_pixel(x, y).0;
                buf.put_pixel(x, y, Rgba([255 - p[0], 255 - p[1], 255 - p[2], p[3]]));
            }
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
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                let p = buf.get_pixel(x, y).0;
                let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
                let nr = (r * 0.393 + g * 0.769 + b * 0.189).min(255.0) as u8;
                let ng = (r * 0.349 + g * 0.686 + b * 0.168).min(255.0) as u8;
                let nb = (r * 0.272 + g * 0.534 + b * 0.131).min(255.0) as u8;
                buf.put_pixel(x, y, Rgba([nr, ng, nb, p[3]]));
            }
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
        if let Some(img) = &self.image {
            if self.resize_w == 0 || self.resize_h == 0 { return; }
            let resized = img.resize(self.resize_w, self.resize_h, image::imageops::FilterType::Lanczos3);
            self.image = Some(resized);
            self.texture_dirty = true;
            self.dirty = true;
            self.fit_on_next_frame = true;
            self.filter_panel = FilterPanel::None;
        }
    }

    fn export_image(&mut self) -> Result<PathBuf, String> {
        let img = match &self.image {
            Some(i) => i,
            None => return Err("No image to export".to_string()),
        };

        let default_name = self.file_path.as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("export");
        
        let filename = format!("{}.{}", default_name, self.export_format.extension());
        
        let path = match rfd::FileDialog::new()
            .set_file_name(&filename)
            .add_filter(self.export_format.as_str(), &[self.export_format.extension()])
            .save_file()
        {
            Some(p) => p,
            None => return Err("Export cancelled".to_string()),
        };

        let mut export_img = img.clone();
        
        if self.export_format == ExportFormat::Ico && self.export_auto_scale_ico {
            if export_img.width() > 256 || export_img.height() > 256 {
                let scale = 256.0 / export_img.width().max(export_img.height()) as f32;
                let new_width = (export_img.width() as f32 * scale) as u32;
                let new_height = (export_img.height() as f32 * scale) as u32;
                export_img = export_img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
            }
        }

        match self.export_format {
            ExportFormat::Jpeg => {
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    std::fs::File::create(&path).map_err(|e| e.to_string())?,
                    self.export_jpeg_quality,
                );
                encoder.encode_image(&export_img).map_err(|e| e.to_string())?;
            }
            ExportFormat::Png => {
                let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
                let encoder = image::codecs::png::PngEncoder::new_with_quality(
                    file,
                    image::codecs::png::CompressionType::Default,
                    image::codecs::png::FilterType::Adaptive,
                );
                encoder.write_image(
                    export_img.as_bytes(),
                    export_img.width(),
                    export_img.height(),
                    export_img.color().into(),
                ).map_err(|e| e.to_string())?;
            }
            ExportFormat::Webp => {
                export_img.save_with_format(&path, image::ImageFormat::WebP)
                    .map_err(|e| e.to_string())?;
            }
            ExportFormat::Bmp => {
                export_img.save_with_format(&path, image::ImageFormat::Bmp)
                    .map_err(|e| e.to_string())?;
            }
            ExportFormat::Tiff => {
                export_img.save_with_format(&path, image::ImageFormat::Tiff)
                    .map_err(|e| e.to_string())?;
            }
            ExportFormat::Ico => {
                if export_img.width() > 256 || export_img.height() > 256 {
                    return Err(format!(
                        "ICO format requires dimensions ≤256px. Image is {}x{}. Enable auto-scaling in export settings.",
                        export_img.width(), export_img.height()
                    ));
                }
                export_img.save_with_format(&path, image::ImageFormat::Ico)
                    .map_err(|e| e.to_string())?;
            }
        }

        self.filter_panel = FilterPanel::None;
        Ok(path)
    }

    fn ensure_texture(&mut self, ctx: &egui::Context) {
        if !self.texture_dirty { return; }
        let img = match &self.image {
            Some(i) => i,
            None => { self.texture_dirty = false; return; }
        };
        let rgba = img.to_rgba8();
        let w = rgba.width() as usize;
        let h = rgba.height() as usize;
        let color_image = egui::ColorImage {
            size: [w, h],
            source_size: egui::vec2(w as f32, h as f32),
            pixels: rgba.pixels().map(|p| {
                let [r, g, b, a] = p.0;
                egui::Color32::from_rgba_unmultiplied(r, g, b, a)
            }).collect(),
        };
        self.texture = Some(ctx.tex_manager().write().alloc(
            "image_editor_img".into(),
            color_image.into(),
            egui::TextureOptions::default(),
        ));
        self.texture_dirty = false;
    }

    fn open_image(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["jpg", "jpeg", "png", "webp", "bmp", "tiff", "tif", "gif"])
            .pick_file()
        {
            if let Ok(img) = image::open(&path) {
                self.push_undo();
                self.resize_w = img.width();
                self.resize_h = img.height();
                self.image = Some(img);
                self.texture_dirty = true;
                self.file_path = Some(path.clone());
                self.dirty = false;
                self.fit_on_next_frame = true;
                self.crop_state = CropState::default();
                self.text_state = TextState::default();
                
                if let Some(cb) = &self.export_callback {
                    cb(path);
                }
            }
        }
    }

    fn new_image(&mut self, w: u32, h: u32) {
        self.push_undo();
        let buf = ImageBuffer::from_pixel(w, h, Rgba([255, 255, 255, 255]));
        self.image = Some(DynamicImage::ImageRgba8(buf));
        self.resize_w = w;
        self.resize_h = h;
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
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.label(egui::RichText::new("File").size(12.0).color(ColorPalette::ZINC_500));
                            if self.toolbar_btn(ui, "New", theme).clicked() { self.new_image(800, 600); }
                            if self.toolbar_btn(ui, "Open", theme).clicked() { self.open_image(); }
                            if self.toolbar_btn(ui, "Save", theme).clicked() { let _ = self.save_impl(); }
                            if self.toolbar_btn(ui, "Export", theme).clicked() { self.filter_panel = FilterPanel::Export; }

                            ui.separator();
                            ui.label(egui::RichText::new("Edit").size(12.0).color(ColorPalette::ZINC_500));
                            if self.toolbar_btn(ui, "Undo", theme).clicked() { self.undo(); }
                            if self.toolbar_btn(ui, "Redo", theme).clicked() { self.redo(); }

                            ui.separator();
                            ui.label(egui::RichText::new("Tools").size(12.0).color(ColorPalette::ZINC_500));
                            self.tool_btn(ui, "Brush", Tool::Brush, theme);
                            self.tool_btn(ui, "Eraser", Tool::Eraser, theme);
                            self.tool_btn(ui, "Fill", Tool::Fill, theme);
                            self.tool_btn(ui, "Text", Tool::Text, theme);
                            self.tool_btn(ui, "Eyedrop", Tool::Eyedropper, theme);
                            self.tool_btn(ui, "Crop", Tool::Crop, theme);
                            self.tool_btn(ui, "Pan", Tool::Pan, theme);

                            ui.separator();
                            ui.label(egui::RichText::new("Transform").size(12.0).color(ColorPalette::ZINC_500));
                            if self.toolbar_btn(ui, "Flip H", theme).clicked() { self.push_undo(); self.apply_flip_h(); }
                            if self.toolbar_btn(ui, "Flip V", theme).clicked() { self.push_undo(); self.apply_flip_v(); }
                            if self.toolbar_btn(ui, "Rot CW", theme).clicked() { self.push_undo(); self.apply_rotate_cw(); }
                            if self.toolbar_btn(ui, "Rot CCW", theme).clicked() { self.push_undo(); self.apply_rotate_ccw(); }

                            ui.separator();
                            ui.label(egui::RichText::new("Filters").size(12.0).color(ColorPalette::ZINC_500));
                            if self.toolbar_btn(ui, "B/C", theme).clicked() { self.filter_panel = FilterPanel::BrightnessContrast; }
                            if self.toolbar_btn(ui, "H/S", theme).clicked() { self.filter_panel = FilterPanel::HueSaturation; }
                            if self.toolbar_btn(ui, "Blur", theme).clicked() { self.filter_panel = FilterPanel::Blur; }
                            if self.toolbar_btn(ui, "Sharpen", theme).clicked() { self.filter_panel = FilterPanel::Sharpen; }
                            if self.toolbar_btn(ui, "Gray", theme).clicked() { self.push_undo(); self.apply_grayscale(); }
                            if self.toolbar_btn(ui, "Invert", theme).clicked() { self.push_undo(); self.apply_invert(); }
                            if self.toolbar_btn(ui, "Sepia", theme).clicked() { self.push_undo(); self.apply_sepia(); }

                            ui.separator();
                            ui.label(egui::RichText::new("View").size(12.0).color(ColorPalette::ZINC_500));
                            if self.toolbar_btn(ui, "Fit", theme).clicked() { self.fit_image(); }
                            if self.toolbar_btn(ui, "+", theme).clicked() { self.zoom *= 1.25; }
                            if self.toolbar_btn(ui, "-", theme).clicked() { self.zoom = (self.zoom / 1.25).max(0.01); }
                            if self.toolbar_btn(ui, "1:1", theme).clicked() { self.zoom = 1.0; self.pan = egui::Vec2::ZERO; }

                            ui.separator();
                            if self.toolbar_btn(ui, "Resize", theme).clicked() { self.filter_panel = FilterPanel::Resize; }
                        });
                    });
            });
    }

    fn toolbar_btn(&self, ui: &mut egui::Ui, label: &str, theme: ThemeMode) -> egui::Response {
        let (bg, hover, txt) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_700, ColorPalette::ZINC_600, ColorPalette::ZINC_200)
        } else {
            (ColorPalette::GRAY_200, ColorPalette::GRAY_300, ColorPalette::GRAY_800)
        };
        ui.scope(|ui| {
            let s = ui.style_mut();
            s.visuals.widgets.inactive.bg_fill = bg;
            s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            s.visuals.widgets.hovered.bg_fill = hover;
            s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            s.visuals.widgets.active.bg_fill = hover;
            ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(txt)).min_size(egui::vec2(0.0, 24.0)))
        }).inner
    }

    fn tool_btn(&mut self, ui: &mut egui::Ui, label: &str, tool: Tool, theme: ThemeMode) {
        let active = self.tool == tool;
        let (bg, hover, txt) = if active {
            (ColorPalette::BLUE_600, ColorPalette::BLUE_500, egui::Color32::WHITE)
        } else if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_700, ColorPalette::ZINC_600, ColorPalette::ZINC_200)
        } else {
            (ColorPalette::GRAY_200, ColorPalette::GRAY_300, ColorPalette::GRAY_800)
        };
        let clicked = ui.scope(|ui| {
            let s = ui.style_mut();
            s.visuals.widgets.inactive.bg_fill = bg;
            s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            s.visuals.widgets.hovered.bg_fill = hover;
            s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            s.visuals.widgets.active.bg_fill = hover;
            ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(txt)).min_size(egui::vec2(0.0, 24.0))).clicked()
        }).inner;
        if clicked { self.tool = tool; }
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
                        ui.label(egui::RichText::new("Resize").size(13.0).color(text_col));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("W:").size(12.0).color(label_col));
                            let old_w = self.resize_w;
                            ui.add(egui::DragValue::new(&mut self.resize_w).range(1..=8192));
                            if self.resize_locked && self.resize_w != old_w && old_w > 0 {
                                let ratio = self.resize_w as f64 / old_w as f64;
                                self.resize_h = (self.resize_h as f64 * ratio).max(1.0) as u32;
                            }
                            ui.label(egui::RichText::new("H:").size(12.0).color(label_col));
                            let old_h = self.resize_h;
                            ui.add(egui::DragValue::new(&mut self.resize_h).range(1..=8192));
                            if self.resize_locked && self.resize_h != old_h && old_h > 0 {
                                let ratio = self.resize_h as f64 / old_h as f64;
                                self.resize_w = (self.resize_w as f64 * ratio).max(1.0) as u32;
                            }
                            if ui.checkbox(&mut self.resize_locked, "Lock").changed() {}
                        });
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
                    self.stroke_points.clear();
                    self.is_dragging = false;
                }
                _ => {}
            }
        }

        if response.clicked_by(egui::PointerButton::Primary) {
            let pos = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            match self.tool {
                Tool::Fill => {
                    if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.push_undo();
                        self.flood_fill(ix, iy);
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
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
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

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
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

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, _show_file_info: bool) {
        let theme = if ui.visuals().dark_mode { ThemeMode::Dark } else { ThemeMode::Light };

        self.handle_keyboard(ctx);

        if self.image.is_none() && self.file_path.is_none() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    let title_color = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_100 } else { ColorPalette::ZINC_900 };
                    ui.label(egui::RichText::new("Image Editor").size(28.0).color(title_color));
                    ui.add_space(8.0);
                    let sub_color = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
                    ui.label(egui::RichText::new("Draw, edit, filter, and transform images").size(14.0).color(sub_color));
                    ui.add_space(32.0);

                    let (bg, hover, txt) = (ColorPalette::BLUE_600, ColorPalette::BLUE_500, egui::Color32::WHITE);
                    let open_clicked = ui.scope(|ui| {
                        let s = ui.style_mut();
                        s.visuals.widgets.inactive.bg_fill = bg;
                        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                        s.visuals.widgets.hovered.bg_fill = hover;
                        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                        ui.add(egui::Button::new(egui::RichText::new("Open Image").size(15.0).color(txt)).min_size(egui::vec2(180.0, 42.0))).clicked()
                    }).inner;
                    if open_clicked { self.open_image(); }

                    ui.add_space(12.0);

                    let (bg2, hover2, txt2) = if matches!(theme, ThemeMode::Dark) {
                        (ColorPalette::ZINC_700, ColorPalette::ZINC_600, ColorPalette::ZINC_200)
                    } else {
                        (ColorPalette::GRAY_200, ColorPalette::GRAY_300, ColorPalette::GRAY_800)
                    };
                    let new_clicked = ui.scope(|ui| {
                        let s = ui.style_mut();
                        s.visuals.widgets.inactive.bg_fill = bg2;
                        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                        s.visuals.widgets.hovered.bg_fill = hover2;
                        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                        ui.add(egui::Button::new(egui::RichText::new("New Canvas").size(15.0).color(txt2)).min_size(egui::vec2(180.0, 42.0))).clicked()
                    }).inner;
                    if new_clicked { self.new_image(800, 600); }
                });
            });
            return;
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
            egui::Frame::new()
                .fill(if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_50 })
                .stroke(egui::Stroke::new(1.0, if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 }))
                .corner_radius(6.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    let mut color = self.color;
                    if egui::color_picker::color_picker_color32(ui, &mut color, egui::color_picker::Alpha::Opaque) {
                        self.color = color;
                    }
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Presets:").size(12.0));
                        let presets = [
                            egui::Color32::BLACK, egui::Color32::WHITE,
                            egui::Color32::from_rgb(255, 0, 0), egui::Color32::from_rgb(0, 128, 0),
                            egui::Color32::from_rgb(0, 0, 255), egui::Color32::from_rgb(255, 255, 0),
                            egui::Color32::from_rgb(255, 0, 255), egui::Color32::from_rgb(0, 255, 255),
                            ColorPalette::BLUE_500, ColorPalette::AMBER_500,
                        ];
                        for &preset in &presets {
                            let btn = egui::Button::new("").fill(preset).min_size(egui::vec2(22.0, 22.0));
                            if ui.add(btn).clicked() { self.color = preset; }
                        }
                    });
                    if ui.button("Close").clicked() { self.show_color_picker = false; }
                });
            ui.add_space(4.0);
        }

        self.render_canvas(ui, ctx);
    }
}
