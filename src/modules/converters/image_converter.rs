use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use crate::style::{ColorPalette, ThemeMode};
use crate::modules::image_export::{ExportFormat, export_image};
use crate::modules::EditorModule;
use super::converter_style::{panel_colors, label_col, format_btn_colors, drop_zone_colors, error_panel_colors};

#[derive(Debug, Clone)]
struct ImageFile {
    path: PathBuf,
    format: Option<String>,
    size_kb: Option<u64>,
}

impl ImageFile {
    fn new(path: PathBuf) -> Self {
        let format = path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_uppercase());
        let size_kb = std::fs::metadata(&path).ok().map(|m| m.len() / 1024);
        Self { path, format, size_kb }
    }

    fn file_name(&self) -> String {
        self.path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConversionState { Idle, Converting, Completed, Failed }

#[derive(Debug, Clone)]
struct ConversionProgress {
    state: ConversionState,
    current: usize,
    total: usize,
    message: String,
}

impl Default for ConversionProgress {
    fn default() -> Self { Self { state: ConversionState::Idle, current: 0, total: 0, message: String::new() } }
}

pub struct ImageConverter {
    images: Vec<ImageFile>,
    target_format: ExportFormat,
    output_directory: Option<PathBuf>,
    jpeg_quality: u8,
    png_compression: u8,
    webp_quality: f32,
    avif_quality: u8,
    avif_speed: u8,
    preserve_metadata: bool,
    overwrite_existing: bool,
    add_suffix: bool,
    custom_suffix: String,
    progress: Arc<Mutex<ConversionProgress>>,
    show_advanced: bool,
    drag_hover: bool,
    auto_scale_ico: bool,
    conversion_errors: Arc<Mutex<Vec<String>>>,
}

impl ImageConverter {
    pub fn new() -> Self {
        Self {
            images: Vec::new(),
            target_format: ExportFormat::Png,
            output_directory: None,
            jpeg_quality: 90,
            png_compression: 6,
            webp_quality: 80.0,
            avif_quality: 80,
            avif_speed: 4,
            preserve_metadata: true,
            overwrite_existing: false,
            add_suffix: false,
            custom_suffix: "_converted".to_string(),
            progress: Arc::new(Mutex::new(ConversionProgress::default())),
            show_advanced: false,
            drag_hover: false,
            auto_scale_ico: true,
            conversion_errors: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn add_images(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if matches!(ext_lower.as_str(), "jpg" | "jpeg" | "png" | "webp" | "bmp" | "tiff" | "tif" | "ico") {
                        if !self.images.iter().any(|img| img.path == path) {
                            self.images.push(ImageFile::new(path));
                        }
                    }
                }
            }
        }
    }

    fn remove_image(&mut self, index: usize) {
        if index < self.images.len() { self.images.remove(index); }
    }

    fn clear_images(&mut self) { self.images.clear(); }

    fn start_conversion(&mut self) {
        if self.images.is_empty() { return; }
        self.conversion_errors.lock().unwrap().clear();

        let output_dir = self.output_directory.clone()
            .unwrap_or_else(|| self.images[0].path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf());

        let images = self.images.clone();
        let target_format = self.target_format;
        let jpeg_quality = self.jpeg_quality;
        let png_compression = self.png_compression;
        let webp_quality = self.webp_quality;
        let avif_quality = self.avif_quality;
        let avif_speed = self.avif_speed;
        let overwrite = self.overwrite_existing;
        let add_suffix = self.add_suffix;
        let suffix = self.custom_suffix.clone();
        let progress = Arc::clone(&self.progress);
        let errors = Arc::clone(&self.conversion_errors);
        let auto_scale_ico = self.auto_scale_ico;

        thread::spawn(move || {
            {
                let mut p = progress.lock().unwrap();
                p.state = ConversionState::Converting;
                p.current = 0;
                p.total = images.len();
                p.message = "Starting conversion...".to_string();
            }

            let mut success_count = 0;
            let mut fail_count = 0;

            for (idx, image) in images.iter().enumerate() {
                {
                    let mut p = progress.lock().unwrap();
                    p.current = idx + 1;
                    p.message = format!("Converting {} ({}/{})", image.file_name(), idx + 1, images.len());
                }
                match Self::convert_image(&image.path, &output_dir, target_format, jpeg_quality, png_compression, webp_quality, overwrite, add_suffix, &suffix, auto_scale_ico, avif_quality, avif_speed) {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        errors.lock().unwrap().push(format!("{}: {}", image.file_name(), e));
                        fail_count += 1;
                    }
                }
            }

            let mut p = progress.lock().unwrap();
            p.state = if fail_count == 0 { ConversionState::Completed } else { ConversionState::Failed };
            p.message = format!("Completed: {} succeeded, {} failed", success_count, fail_count);
        });
    }

    fn convert_image(
        input_path: &PathBuf, output_dir: &PathBuf, target_format: ExportFormat,
        jpeg_quality: u8, png_compression: u8, webp_quality: f32,
        overwrite: bool, add_suffix: bool, suffix: &str,
        auto_scale_ico: bool, avif_quality: u8, avif_speed: u8,
    ) -> Result<(), String> {
        let img = image::open(input_path).map_err(|e| format!("Failed to open image: {}", e))?;
        let stem = input_path.file_stem().and_then(|s| s.to_str()).ok_or("Invalid filename")?;
        let new_stem = if add_suffix { format!("{}{}", stem, suffix) } else { stem.to_string() };
        let output_path = output_dir.join(format!("{}.{}", new_stem, target_format.extension()));
        if output_path.exists() && !overwrite { return Err("File exists and overwrite is disabled".to_string()); }
        export_image(&img, &output_path, target_format, jpeg_quality, png_compression, webp_quality, auto_scale_ico, avif_quality, avif_speed)
    }

    fn render_header(&self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (tc, sc) = if matches!(theme, ThemeMode::Dark) { (ColorPalette::ZINC_100, ColorPalette::ZINC_400) } else { (ColorPalette::ZINC_900, ColorPalette::ZINC_600) };
        ui.add_space(12.0);
        ui.label(egui::RichText::new("Image Converter").size(24.0).color(tc));
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Convert images between formats with customizable settings").size(13.0).color(sc));
        ui.add_space(12.0);
    }

    fn render_format_selector(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color) = panel_colors(theme);
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.label(egui::RichText::new("Target Format").size(14.0).color(text_color));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for format in ExportFormat::all() {
                    let (bg_color, txt_color) = format_btn_colors(self.target_format == format, ColorPalette::BLUE_600, theme);
                    if ui.add(egui::Button::new(egui::RichText::new(format.as_str()).size(13.0).color(txt_color))
                        .fill(bg_color).stroke(egui::Stroke::NONE).corner_radius(6.0).min_size(egui::vec2(70.0, 32.0))).clicked()
                    {
                        self.target_format = format;
                    }
                }
            });
            if self.target_format == ExportFormat::Avif {
                ui.add_space(6.0);
                ui.label(egui::RichText::new("AVIF is output-only. AVIF files cannot be used as input sources.").size(11.0).color(label_col(theme)).italics());
            }
        });
    }

    fn render_quality_settings(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color) = panel_colors(theme);
        let lc = label_col(theme);
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Quality Settings").size(14.0).color(text_color));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if self.show_advanced { "Hide" } else { "Show" }).clicked() { self.show_advanced = !self.show_advanced; }
                });
            });

            if self.show_advanced {
                ui.add_space(12.0);
                match self.target_format {
                    ExportFormat::Jpeg => {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("JPEG Quality:").color(lc));
                            ui.add(egui::Slider::new(&mut self.jpeg_quality, 1..=100).suffix("%"));
                        });
                    }
                    ExportFormat::Png => {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("PNG Compression:").color(lc));
                            ui.add(egui::Slider::new(&mut self.png_compression, 0..=9));
                        });
                    }
                    ExportFormat::Webp => {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("WebP Quality:").color(lc));
                            ui.add(egui::Slider::new(&mut self.webp_quality, 0.0..=100.0).suffix("%"));
                        });
                    }
                    ExportFormat::Ico => {
                        ui.checkbox(&mut self.auto_scale_ico, egui::RichText::new("Auto-scale to 256px (maintains aspect ratio, only if width > 256px)").color(lc));
                    }
                    ExportFormat::Avif => {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("AVIF Quality:").color(lc));
                            ui.add(egui::Slider::new(&mut self.avif_quality, 1..=100).suffix("%"));
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Encode Speed:").color(lc));
                            ui.add(egui::Slider::new(&mut self.avif_speed, 0..=10));
                        });
                        let speed_desc = match self.avif_speed {
                            0..=2 => "Slowest encode, smallest file size",
                            3..=5 => "Balanced encode time and file size",
                            6..=8 => "Faster encode, larger file size",
                            _ => "Fastest encode, largest file size",
                        };
                        ui.label(egui::RichText::new(speed_desc).size(11.0).color(lc).italics());
                    }
                    _ => {}
                }
                ui.add_space(8.0); ui.separator(); ui.add_space(8.0);
                ui.checkbox(&mut self.preserve_metadata, egui::RichText::new("Preserve metadata (EXIF, etc.)").color(lc));
                ui.checkbox(&mut self.overwrite_existing, egui::RichText::new("Overwrite existing files").color(lc));
                ui.add_space(4.0);
                ui.checkbox(&mut self.add_suffix, egui::RichText::new("Add suffix to filename").color(lc));
                if self.add_suffix {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Suffix:").color(lc));
                        ui.text_edit_singleline(&mut self.custom_suffix);
                    });
                }
            }
        });
    }

    fn render_output_directory(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color) = panel_colors(theme);
        let lc = label_col(theme);
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.label(egui::RichText::new("Output Directory").size(14.0).color(text_color));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let dir_text = if let Some(dir) = &self.output_directory { dir.to_string_lossy().to_string() } else { "Same as source files".to_string() };
                ui.label(egui::RichText::new(dir_text).color(lc));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Browse").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() { self.output_directory = Some(dir); }
                    }
                    if self.output_directory.is_some() && ui.button("Clear").clicked() { self.output_directory = None; }
                });
            });
        });
    }

    fn render_image_list(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color) = panel_colors(theme);
        let weak_color = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_500 } else { ColorPalette::ZINC_500 };
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("Images ({})", self.images.len())).size(14.0).color(text_color));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.images.is_empty() && ui.button("Clear All").clicked() { self.clear_images(); }
                    if ui.button("Add Images").clicked() {
                        if let Some(paths) = rfd::FileDialog::new()
                            .add_filter("Images", &["jpg", "jpeg", "png", "webp", "bmp", "tiff", "tif", "ico"])
                            .pick_files()
                        { self.add_images(paths); }
                    }
                });
            });

            ui.add_space(8.0); ui.separator(); ui.add_space(8.0);

            if self.images.is_empty() {
                let (drop_zone_bg, drop_zone_border) = drop_zone_colors(self.drag_hover, theme);
                let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 150.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 6.0, drop_zone_bg);
                ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(2.0, drop_zone_border), egui::StrokeKind::Outside);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Drop images here or click to browse", egui::FontId::proportional(14.0), weak_color);
                if response.clicked() {
                    if let Some(paths) = rfd::FileDialog::new()
                        .add_filter("Images", &["jpg", "jpeg", "png", "webp", "bmp", "tiff", "tif", "ico"])
                        .pick_files()
                    { self.add_images(paths); }
                }
            } else {
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    let item_bg = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_900 } else { egui::Color32::WHITE };
                    let mut to_remove = None;
                    for (idx, image) in self.images.iter().enumerate() {
                        egui::Frame::new().fill(item_bg).stroke(egui::Stroke::new(1.0, border_color)).corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(&image.file_name()).color(text_color).size(13.0));
                                    ui.label(egui::RichText::new(format!(
                                        "{} | {}",
                                        image.format.as_ref().unwrap_or(&"Unknown".to_string()),
                                        image.size_kb.map(|s| format!("{} KB", s)).unwrap_or_else(|| "Unknown size".to_string())
                                    )).color(weak_color).size(11.0));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Remove").clicked() { to_remove = Some(idx); }
                                });
                            });
                        });
                        ui.add_space(6.0);
                    }
                    if let Some(idx) = to_remove { self.remove_image(idx); }
                });
            }
        });
    }

    fn render_errors(&self, ui: &mut egui::Ui, theme: ThemeMode) {
        let errors_vec = { let e = self.conversion_errors.lock().unwrap(); if e.is_empty() { return; } e.clone() };
        let (panel_bg, border_color, text_color) = error_panel_colors(theme);
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("Errors ({})", errors_vec.len())).size(14.0).color(text_color));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear").clicked() { self.conversion_errors.lock().unwrap().clear(); }
                });
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical().id_salt("error_scroll").max_height(150.0).show(ui, |ui| {
                for error in errors_vec.iter() {
                    ui.label(egui::RichText::new(error).size(12.0).color(text_color));
                    ui.add_space(4.0);
                }
            });
        });
    }

    fn render_progress(&self, ui: &mut egui::Ui, theme: ThemeMode) {
        let progress = self.progress.lock().unwrap();
        if progress.state == ConversionState::Idle { return; }
        let (panel_bg, border_color, text_color) = panel_colors(theme);
        let progress_bg = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_200 };
        let progress_fill = match progress.state {
            ConversionState::Converting => ColorPalette::BLUE_500,
            ConversionState::Completed => ColorPalette::GREEN_500,
            ConversionState::Failed => ColorPalette::RED_500,
            ConversionState::Idle => ColorPalette::ZINC_500,
        };
        let fraction = if progress.total > 0 { progress.current as f32 / progress.total as f32 } else { 0.0 };
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.label(egui::RichText::new("Conversion Progress").size(14.0).color(text_color));
            ui.add_space(8.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 4.0, progress_bg);
            ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * fraction, rect.height())), 4.0, progress_fill);
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("{:.0}%", fraction * 100.0), egui::FontId::proportional(12.0), egui::Color32::WHITE);
            ui.add_space(8.0);
            ui.label(egui::RichText::new(&progress.message).size(12.0).color(text_color));
        });
    }

    fn render_action_buttons(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let is_converting = self.progress.lock().unwrap().state == ConversionState::Converting;
        let can_convert = !self.images.is_empty() && !is_converting;
        let (button_bg, button_hover, button_text) = if can_convert {
            (ColorPalette::BLUE_600, ColorPalette::BLUE_500, egui::Color32::WHITE)
        } else if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_700, ColorPalette::ZINC_700, ColorPalette::ZINC_500)
        } else {
            (ColorPalette::GRAY_300, ColorPalette::GRAY_300, ColorPalette::GRAY_500)
        };
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.scope(|ui| {
                let style = ui.style_mut();
                style.visuals.widgets.inactive.bg_fill = button_bg;
                style.visuals.widgets.inactive.weak_bg_fill = button_bg;
                style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                style.visuals.widgets.hovered.bg_fill = button_hover;
                style.visuals.widgets.hovered.weak_bg_fill = button_hover;
                style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                if ui.add_enabled(can_convert, egui::Button::new(egui::RichText::new("Convert Images").size(15.0).color(button_text)).min_size(egui::vec2(140.0, 40.0)).corner_radius(6.0)).clicked() {
                    self.start_conversion();
                }
            });
        });
    }
}

impl EditorModule for ImageConverter {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, _show_file_info: bool) {
        let theme = if ui.visuals().dark_mode { ThemeMode::Dark } else { ThemeMode::Light };
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths: Vec<PathBuf> = i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect();
                self.add_images(paths);
            }
            self.drag_hover = !i.raw.hovered_files.is_empty();
        });
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(8.0);
            self.render_header(ui, theme);
            ui.add_space(8.0);
            self.render_format_selector(ui, theme);
            ui.add_space(12.0);
            self.render_quality_settings(ui, theme);
            ui.add_space(12.0);
            self.render_output_directory(ui, theme);
            ui.add_space(12.0);
            self.render_image_list(ui, theme);
            ui.add_space(12.0);
            self.render_errors(ui, theme);
            ui.add_space(12.0);
            self.render_progress(ui, theme);
            ui.add_space(12.0);
            self.render_action_buttons(ui, theme);
            ui.add_space(16.0);
        });
        ctx.request_repaint();
    }

    fn save(&mut self) -> Result<(), String> { Ok(()) }
    fn save_as(&mut self) -> Result<(), String> { Ok(()) }
    fn get_title(&self) -> String { "Image Converter".to_string() }
}
