use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use crate::style::{ColorPalette, ThemeMode};
use super::EditorModule;
use image::ImageEncoder;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
    Bmp,
    Tiff,
    Ico,
}

impl ImageFormat {
    fn as_str(&self) -> &str {
        match self {
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::Png => "PNG",
            ImageFormat::Webp => "WebP",
            ImageFormat::Bmp => "BMP",
            ImageFormat::Tiff => "TIFF",
            ImageFormat::Ico => "ICO",
        }
    }

    fn extension(&self) -> &str {
        match self {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Webp => "webp",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Tiff => "tiff",
            ImageFormat::Ico => "ico",
        }
    }

    fn all() -> Vec<ImageFormat> {
        vec![
            ImageFormat::Jpeg,
            ImageFormat::Png,
            ImageFormat::Webp,
            ImageFormat::Bmp,
            ImageFormat::Tiff,
            ImageFormat::Ico,
        ]
    }
}

#[derive(Debug, Clone)]
struct ImageFile {
    path: PathBuf,
    format: Option<String>,
    size_kb: Option<u64>,
}

impl ImageFile {
    fn new(path: PathBuf) -> Self {
        let format = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_uppercase());
        
        let size_kb = std::fs::metadata(&path)
            .ok()
            .map(|m| m.len() / 1024);

        Self {
            path,
            format,
            size_kb,
        }
    }

    fn file_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConversionState {
    Idle,
    Converting,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
struct ConversionProgress {
    state: ConversionState,
    current: usize,
    total: usize,
    message: String,
}

impl Default for ConversionProgress {
    fn default() -> Self {
        Self {
            state: ConversionState::Idle,
            current: 0,
            total: 0,
            message: String::new(),
        }
    }
}

pub struct ImageConverter {
    images: Vec<ImageFile>,
    target_format: ImageFormat,
    output_directory: Option<PathBuf>,
    jpeg_quality: u8,
    png_compression: u8,
    webp_quality: f32,
    preserve_metadata: bool,
    overwrite_existing: bool,
    add_suffix: bool,
    custom_suffix: String,
    progress: Arc<Mutex<ConversionProgress>>,
    show_advanced: bool,
    drag_hover: bool,
}

impl ImageConverter {
    pub fn new() -> Self {
        Self {
            images: Vec::new(),
            target_format: ImageFormat::Png,
            output_directory: None,
            jpeg_quality: 90,
            png_compression: 6,
            webp_quality: 80.0,
            preserve_metadata: true,
            overwrite_existing: false,
            add_suffix: false,
            custom_suffix: "_converted".to_string(),
            progress: Arc::new(Mutex::new(ConversionProgress::default())),
            show_advanced: false,
            drag_hover: false,
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
        if index < self.images.len() {
            self.images.remove(index);
        }
    }

    fn clear_images(&mut self) {
        self.images.clear();
    }

    fn start_conversion(&mut self) {
        if self.images.is_empty() {
            return;
        }

        let output_dir = self.output_directory.clone()
            .unwrap_or_else(|| self.images[0].path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf());

        let images = self.images.clone();
        let target_format = self.target_format;
        let jpeg_quality = self.jpeg_quality;
        let png_compression = self.png_compression;
        let webp_quality = self.webp_quality;
        let overwrite = self.overwrite_existing;
        let add_suffix = self.add_suffix;
        let suffix = self.custom_suffix.clone();
        let progress = Arc::clone(&self.progress);

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

                match Self::convert_image(
                    &image.path,
                    &output_dir,
                    target_format,
                    jpeg_quality,
                    png_compression,
                    webp_quality,
                    overwrite,
                    add_suffix,
                    &suffix,
                ) {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        eprintln!("Failed to convert {}: {}", image.file_name(), e);
                        fail_count += 1;
                    }
                }
            }

            {
                let mut p = progress.lock().unwrap();
                p.state = if fail_count == 0 { ConversionState::Completed } else { ConversionState::Failed };
                p.message = format!("Completed: {} succeeded, {} failed", success_count, fail_count);
            }
        });
    }

    fn convert_image(
        input_path: &PathBuf,
        output_dir: &PathBuf,
        target_format: ImageFormat,
        jpeg_quality: u8,
        _png_compression: u8,
        _webp_quality: f32,
        overwrite: bool,
        add_suffix: bool,
        suffix: &str,
    ) -> Result<(), String> {
        let img = image::open(input_path)
            .map_err(|e| format!("Failed to open image: {}", e))?;

        let stem = input_path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or("Invalid filename")?;

        let new_stem = if add_suffix {
            format!("{}{}", stem, suffix)
        } else {
            stem.to_string()
        };

        let output_filename = format!("{}.{}", new_stem, target_format.extension());
        let output_path = output_dir.join(output_filename);

        if output_path.exists() && !overwrite {
            return Err("File exists and overwrite is disabled".to_string());
        }

        match target_format {
            ImageFormat::Jpeg => {
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    std::fs::File::create(&output_path)
                        .map_err(|e| format!("Failed to create output file: {}", e))?,
                    jpeg_quality,
                );
                encoder.encode_image(&img)
                    .map_err(|e| format!("Failed to encode JPEG: {}", e))?;
            }
            ImageFormat::Png => {
                let file = std::fs::File::create(&output_path)
                    .map_err(|e| format!("Failed to create output file: {}", e))?;
                let encoder = image::codecs::png::PngEncoder::new_with_quality(
                    file,
                    image::codecs::png::CompressionType::Default,
                    image::codecs::png::FilterType::Adaptive,
                );
                encoder.write_image(
                    img.as_bytes(),
                    img.width(),
                    img.height(),
                    img.color().into(),
                ).map_err(|e| format!("Failed to encode PNG: {}", e))?;
            }
            ImageFormat::Webp => {
                img.save_with_format(&output_path, image::ImageFormat::WebP)
                    .map_err(|e| format!("Failed to save WebP: {}", e))?;
            }
            ImageFormat::Bmp => {
                img.save_with_format(&output_path, image::ImageFormat::Bmp)
                    .map_err(|e| format!("Failed to save BMP: {}", e))?;
            }
            ImageFormat::Tiff => {
                img.save_with_format(&output_path, image::ImageFormat::Tiff)
                    .map_err(|e| format!("Failed to save TIFF: {}", e))?;
            }
            ImageFormat::Ico => {
                img.save_with_format(&output_path, image::ImageFormat::Ico)
                    .map_err(|e| format!("Failed to save ICO: {}", e))?;
            }
        }

        Ok(())
    }

    fn render_header(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        ui.vertical(|ui| {
            ui.add_space(12.0);
            
            ui.horizontal(|ui| {
                let title_color = if matches!(theme, ThemeMode::Dark) {
                    ColorPalette::ZINC_100
                } else {
                    ColorPalette::ZINC_900
                };

                ui.label(
                    egui::RichText::new("Image Converter")
                        .size(24.0)
                        .color(title_color)
                );
            });

            ui.add_space(4.0);

            let subtitle_color = if matches!(theme, ThemeMode::Dark) {
                ColorPalette::ZINC_400
            } else {
                ColorPalette::ZINC_600
            };

            ui.label(
                egui::RichText::new("Convert images between formats with customizable settings")
                    .size(13.0)
                    .color(subtitle_color)
            );

            ui.add_space(12.0);
        });
    }

    fn render_format_selector(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::ZINC_700, ColorPalette::ZINC_200)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::GRAY_300, ColorPalette::GRAY_800)
        };

        egui::Frame::new()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Target Format")
                        .size(14.0)
                        .color(text_color)
                );

                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    for format in ImageFormat::all() {
                        let is_selected = self.target_format == format;
                        
                        let (bg_color, txt_color) = if is_selected {
                            if matches!(theme, ThemeMode::Dark) {
                                (ColorPalette::BLUE_600, egui::Color32::WHITE)
                            } else {
                                (ColorPalette::BLUE_600, egui::Color32::WHITE)
                            }
                        } else {
                            if matches!(theme, ThemeMode::Dark) {
                                (ColorPalette::ZINC_700, ColorPalette::ZINC_300)
                            } else {
                                (ColorPalette::GRAY_200, ColorPalette::GRAY_800)
                            }
                        };

                        let button = egui::Button::new(
                            egui::RichText::new(format.as_str())
                                .size(13.0)
                                .color(txt_color)
                        )
                        .fill(bg_color)
                        .stroke(egui::Stroke::NONE)
                        .corner_radius(6.0)
                        .min_size(egui::vec2(70.0, 32.0));

                        if ui.add(button).clicked() {
                            self.target_format = format;
                        }
                    }
                });
            });
    }

    fn render_quality_settings(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color, label_color) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::ZINC_700, ColorPalette::ZINC_200, ColorPalette::ZINC_400)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::GRAY_300, ColorPalette::GRAY_800, ColorPalette::ZINC_600)
        };

        egui::Frame::new()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Quality Settings")
                            .size(14.0)
                            .color(text_color)
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let toggle_text = if self.show_advanced { "Hide" } else { "Show" };
                        if ui.button(toggle_text).clicked() {
                            self.show_advanced = !self.show_advanced;
                        }
                    });
                });

                if self.show_advanced {
                    ui.add_space(12.0);

                    match self.target_format {
                        ImageFormat::Jpeg => {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("JPEG Quality:").color(label_color));
                                ui.add(egui::Slider::new(&mut self.jpeg_quality, 1..=100).suffix("%"));
                            });
                        }
                        ImageFormat::Png => {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("PNG Compression:").color(label_color));
                                ui.add(egui::Slider::new(&mut self.png_compression, 0..=9));
                            });
                        }
                        ImageFormat::Webp => {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("WebP Quality:").color(label_color));
                                ui.add(egui::Slider::new(&mut self.webp_quality, 0.0..=100.0).suffix("%"));
                            });
                        }
                        _ => {}
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.checkbox(&mut self.preserve_metadata, 
                        egui::RichText::new("Preserve metadata (EXIF, etc.)").color(label_color));
                    ui.checkbox(&mut self.overwrite_existing, 
                        egui::RichText::new("Overwrite existing files").color(label_color));
                    
                    ui.add_space(4.0);
                    ui.checkbox(&mut self.add_suffix, 
                        egui::RichText::new("Add suffix to filename").color(label_color));
                    
                    if self.add_suffix {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Suffix:").color(label_color));
                            ui.text_edit_singleline(&mut self.custom_suffix);
                        });
                    }
                }
            });
    }

    fn render_output_directory(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color, label_color) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::ZINC_700, ColorPalette::ZINC_200, ColorPalette::ZINC_400)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::GRAY_300, ColorPalette::GRAY_800, ColorPalette::ZINC_600)
        };

        egui::Frame::new()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Output Directory")
                        .size(14.0)
                        .color(text_color)
                );

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let dir_text = if let Some(dir) = &self.output_directory {
                        dir.to_string_lossy().to_string()
                    } else {
                        "Same as source files".to_string()
                    };

                    ui.label(egui::RichText::new(dir_text).color(label_color));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Browse").clicked() {
                            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                self.output_directory = Some(dir);
                            }
                        }

                        if self.output_directory.is_some() {
                            if ui.button("Clear").clicked() {
                                self.output_directory = None;
                            }
                        }
                    });
                });
            });
    }

    fn render_image_list(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color, weak_color) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::ZINC_700, ColorPalette::ZINC_200, ColorPalette::ZINC_500)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::GRAY_300, ColorPalette::GRAY_800, ColorPalette::ZINC_500)
        };

        egui::Frame::new()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Images ({})", self.images.len()))
                            .size(14.0)
                            .color(text_color)
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.images.is_empty() && ui.button("Clear All").clicked() {
                            self.clear_images();
                        }
                        if ui.button("Add Images").clicked() {
                            if let Some(paths) = rfd::FileDialog::new()
                                .add_filter("Images", &["jpg", "jpeg", "png", "webp", "bmp", "tiff", "tif", "ico"])
                                .pick_files()
                            {
                                self.add_images(paths);
                            }
                        }
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                if self.images.is_empty() {
                    let drop_zone_bg = if self.drag_hover {
                        if matches!(theme, ThemeMode::Dark) {
                            ColorPalette::ZINC_700
                        } else {
                            ColorPalette::GRAY_200
                        }
                    } else {
                        if matches!(theme, ThemeMode::Dark) {
                            ColorPalette::ZINC_900
                        } else {
                            egui::Color32::WHITE
                        }
                    };

                    let drop_zone_border = if self.drag_hover {
                        ColorPalette::BLUE_500
                    } else {
                        if matches!(theme, ThemeMode::Dark) {
                            ColorPalette::ZINC_600
                        } else {
                            ColorPalette::GRAY_400
                        }
                    };

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 150.0),
                        egui::Sense::click(),
                    );

                    ui.painter().rect_filled(rect, 6.0, drop_zone_bg);
                    ui.painter().rect_stroke(
                        rect,
                        6.0,
                        egui::Stroke::new(2.0, drop_zone_border),
                        egui::StrokeKind::Outside,
                    );

                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Drop images here or click to browse",
                        egui::FontId::proportional(14.0),
                        weak_color,
                    );

                    if response.clicked() {
                        if let Some(paths) = rfd::FileDialog::new()
                            .add_filter("Images", &["jpg", "jpeg", "png", "webp", "bmp", "tiff", "tif", "ico"])
                            .pick_files()
                        {
                            self.add_images(paths);
                        }
                    }
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            let mut to_remove = None;

                            for (idx, image) in self.images.iter().enumerate() {
                                let item_bg = if matches!(theme, ThemeMode::Dark) {
                                    ColorPalette::ZINC_900
                                } else {
                                    egui::Color32::WHITE
                                };

                                egui::Frame::new()
                                    .fill(item_bg)
                                    .stroke(egui::Stroke::new(1.0, border_color))
                                    .corner_radius(6.0)
                                    .inner_margin(12.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&image.file_name())
                                                        .color(text_color)
                                                        .size(13.0)
                                                );
                                                
                                                let info = format!(
                                                    "{} | {}",
                                                    image.format.as_ref().unwrap_or(&"Unknown".to_string()),
                                                    image.size_kb.map(|s| format!("{} KB", s)).unwrap_or_else(|| "Unknown size".to_string())
                                                );
                                                
                                                ui.label(
                                                    egui::RichText::new(info)
                                                        .color(weak_color)
                                                        .size(11.0)
                                                );
                                            });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button("Remove").clicked() {
                                                    to_remove = Some(idx);
                                                }
                                            });
                                        });
                                    });

                                ui.add_space(6.0);
                            }

                            if let Some(idx) = to_remove {
                                self.remove_image(idx);
                            }
                        });
                }
            });
    }

    fn render_progress(&self, ui: &mut egui::Ui, theme: ThemeMode) {
        let progress = self.progress.lock().unwrap();

        if progress.state == ConversionState::Idle {
            return;
        }

        let (panel_bg, border_color, text_color) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::ZINC_700, ColorPalette::ZINC_200)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::GRAY_300, ColorPalette::GRAY_800)
        };

        egui::Frame::new()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Conversion Progress")
                        .size(14.0)
                        .color(text_color)
                );

                ui.add_space(8.0);

                let progress_fraction = if progress.total > 0 {
                    progress.current as f32 / progress.total as f32
                } else {
                    0.0
                };

                let progress_bg = if matches!(theme, ThemeMode::Dark) {
                    ColorPalette::ZINC_700
                } else {
                    ColorPalette::GRAY_200
                };

                let progress_fill = match progress.state {
                    ConversionState::Converting => ColorPalette::BLUE_500,
                    ConversionState::Completed => ColorPalette::GREEN_500,
                    ConversionState::Failed => ColorPalette::RED_500,
                    ConversionState::Idle => ColorPalette::ZINC_500,
                };

                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 24.0),
                    egui::Sense::hover(),
                );

                ui.painter().rect_filled(rect, 4.0, progress_bg);

                let fill_rect = egui::Rect::from_min_size(
                    rect.min,
                    egui::vec2(rect.width() * progress_fraction, rect.height()),
                );
                ui.painter().rect_filled(fill_rect, 4.0, progress_fill);

                let progress_text = format!("{:.0}%", progress_fraction * 100.0);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &progress_text,
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );

                ui.add_space(8.0);

                ui.label(
                    egui::RichText::new(&progress.message)
                        .size(12.0)
                        .color(text_color)
                );
            });
    }

    fn render_action_buttons(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let progress = self.progress.lock().unwrap();
        let is_converting = progress.state == ConversionState::Converting;
        drop(progress);

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let can_convert = !self.images.is_empty() && !is_converting;

            let (button_bg, button_hover, button_text) = if can_convert {
                if matches!(theme, ThemeMode::Dark) {
                    (ColorPalette::BLUE_600, ColorPalette::BLUE_500, egui::Color32::WHITE)
                } else {
                    (ColorPalette::BLUE_600, ColorPalette::BLUE_500, egui::Color32::WHITE)
                }
            } else {
                if matches!(theme, ThemeMode::Dark) {
                    (ColorPalette::ZINC_700, ColorPalette::ZINC_700, ColorPalette::ZINC_500)
                } else {
                    (ColorPalette::GRAY_300, ColorPalette::GRAY_300, ColorPalette::GRAY_500)
                }
            };

            ui.scope(|ui| {
                let style = ui.style_mut();
                style.visuals.widgets.inactive.bg_fill = button_bg;
                style.visuals.widgets.hovered.bg_fill = button_hover;
                style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, button_text);
                style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, button_text);

                let button = egui::Button::new(
                    egui::RichText::new("Convert Images")
                        .size(15.0)
                        .color(button_text)
                )
                .min_size(egui::vec2(140.0, 40.0))
                .corner_radius(6.0);

                if ui.add_enabled(can_convert, button).clicked() {
                    self.start_conversion();
                }
            });
        });
    }
}

impl EditorModule for ImageConverter {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, _show_file_info: bool) {
        let theme = if ui.visuals().dark_mode { ThemeMode::Dark } else { ThemeMode::Light };

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
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
                self.render_progress(ui, theme);

                ui.add_space(12.0);
                self.render_action_buttons(ui, theme);

                ui.add_space(16.0);
            });

        ctx.request_repaint();
    }

    fn save(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn save_as(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn get_title(&self) -> String {
        "Image Converter".to_string()
    }
}
