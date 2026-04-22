use eframe::egui;
use flate2::Compression;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use crate::style::{ColorPalette, ThemeMode};
use crate::modules::EditorModule;
use super::converter_style::{panel_colors, label_col, format_btn_colors, drop_zone_colors, error_panel_colors};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArchiveFormat { Zip, TarGz, TarBz2, Tar, SevenZ }
impl ArchiveFormat {
    pub fn as_str(self) -> &'static str {
        match self { Self::Zip => "ZIP", Self::TarGz => "TAR.GZ", Self::TarBz2 => "TAR.BZ2", Self::Tar => "TAR", Self::SevenZ => "7Z" }
    }
    pub fn extension(self) -> &'static str {
        match self { Self::Zip => "zip", Self::TarGz => "tar.gz", Self::TarBz2 => "tar.bz2", Self::Tar => "tar", Self::SevenZ => "7z" }
    }
    pub fn all() -> &'static [ArchiveFormat] { &[Self::Zip, Self::TarGz, Self::TarBz2, Self::Tar, Self::SevenZ] }
    pub fn from_path(p: &Path) -> Option<Self> {
        let name = p.file_name()?.to_str()?.to_lowercase();
        if name.ends_with(".tar.gz") || name.ends_with(".tgz") { return Some(Self::TarGz); }
        if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") { return Some(Self::TarBz2); }
        match p.extension()?.to_str()?.to_lowercase().as_str() {
            "zip" => Some(Self::Zip),
            "tar" => Some(Self::Tar),
            "7z" => Some(Self::SevenZ),
            _ => None,
        }
    }
    pub fn supports_compression_level(self) -> bool { !matches!(self, Self::Tar | Self::SevenZ) }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConvState { Idle, Converting, Done, Failed }
#[derive(Clone)]
struct Progress { state: ConvState, current: usize, total: usize, message: String }
impl Default for Progress { fn default() -> Self { Self { state: ConvState::Idle, current: 0, total: 0, message: String::new() } } }

#[derive(Clone)]
struct ArchiveFile { path: PathBuf, format: Option<ArchiveFormat>, size_kb: Option<u64> }
impl ArchiveFile {
    fn new(path: PathBuf) -> Self {
        let format = ArchiveFormat::from_path(&path);
        let size_kb = fs::metadata(&path).ok().map(|m| m.len() / 1024);
        Self { path, format, size_kb }
    }
    fn name(&self) -> String { self.path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string() }
}

pub struct ArchiveConverter {
    files: Vec<ArchiveFile>,
    target_format: ArchiveFormat,
    output_dir: Option<PathBuf>,
    compression_level: u32,
    overwrite: bool,
    progress: Arc<Mutex<Progress>>,
    errors: Arc<Mutex<Vec<String>>>,
    show_options: bool,
    drag_hover: bool,
}

impl ArchiveConverter {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            target_format: ArchiveFormat::Zip,
            output_dir: None,
            compression_level: 9,
            overwrite: false,
            progress: Arc::new(Mutex::new(Progress::default())),
            errors: Arc::new(Mutex::new(Vec::new())),
            show_options: false,
            drag_hover: false,
        }
    }

    fn add_files(&mut self, paths: Vec<PathBuf>) {
        for p in paths {
            if p.is_file() && ArchiveFormat::from_path(&p).is_some() && !self.files.iter().any(|f| f.path == p) {
                self.files.push(ArchiveFile::new(p));
            }
        }
    }

    fn start_conversion(&mut self) {
        if self.files.is_empty() { return; }
        self.errors.lock().unwrap().clear();
        let out_dir = self.output_dir.clone().unwrap_or_else(|| {
            self.files[0].path.parent().unwrap_or(Path::new(".")).to_path_buf()
        });
        let files = self.files.clone();
        let target = self.target_format;
        let level = self.compression_level.clamp(1, 9);
        let overwrite = self.overwrite;
        let progress = Arc::clone(&self.progress);
        let errors = Arc::clone(&self.errors);
        thread::spawn(move || {
            { let mut p = progress.lock().unwrap(); p.state = ConvState::Converting; p.current = 0; p.total = files.len(); p.message = "Starting conversion...".into(); }
            let (mut ok, mut fail) = (0usize, 0usize);
            for (i, file) in files.iter().enumerate() {
                { let mut p = progress.lock().unwrap(); p.current = i + 1; p.message = format!("Converting {} ({}/{})", file.name(), i + 1, files.len()); }
                match file.format {
                    None => { errors.lock().unwrap().push(format!("{}: Unknown format", file.name())); fail += 1; }
                    Some(src_fmt) => match Self::convert_file(&file.path, &out_dir, src_fmt, target, overwrite, level) {
                        Ok(_) => ok += 1,
                        Err(e) => { errors.lock().unwrap().push(format!("{}: {}", file.name(), e)); fail += 1; }
                    }
                }
            }
            let mut p = progress.lock().unwrap();
            p.state = if fail == 0 { ConvState::Done } else { ConvState::Failed };
            p.message = format!("Completed: {} succeeded, {} failed", ok, fail);
        });
    }

    fn convert_file(input: &Path, out_dir: &Path, from: ArchiveFormat, to: ArchiveFormat, overwrite: bool, level: u32) -> Result<(), String> {
        let name = input.file_name().and_then(|n| n.to_str()).unwrap_or("archive");
        let stem = ["tar.gz", "tar.bz2", "tgz", "tbz2", "tar", "zip", "7z"]
            .iter().fold(name, |s, ext| s.strip_suffix(&format!(".{}", ext)).unwrap_or(s));
        let out = out_dir.join(format!("{}.{}", stem, to.extension()));
        if out.exists() && !overwrite { return Err("File exists and overwrite is disabled".into()); }
        let tmp = std::env::temp_dir().join(format!(
            "ue_arch_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
        ));
        fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        let result = Self::extract(input, from, &tmp).and_then(|_| Self::archive(&tmp, &out, to, level));
        let _ = fs::remove_dir_all(&tmp);
        result
    }

    fn extract(src: &Path, fmt: ArchiveFormat, dest: &Path) -> Result<(), String> {
        match fmt {
            ArchiveFormat::Zip => {
                let mut a = zip::ZipArchive::new(File::open(src).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                for i in 0..a.len() {
                    let mut entry = a.by_index(i).map_err(|e| e.to_string())?;
                    let safe: PathBuf = entry.name().replace('\\', "/").split('/').filter(|s| !s.is_empty() && *s != "..").collect();
                    if safe.as_os_str().is_empty() { continue; }
                    let out = dest.join(&safe);
                    if entry.is_dir() {
                        fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                    } else {
                        if let Some(p) = out.parent() { fs::create_dir_all(p).map_err(|e| e.to_string())?; }
                        io::copy(&mut entry, &mut File::create(&out).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                    }
                }
                Ok(())
            }
            ArchiveFormat::Tar => tar::Archive::new(File::open(src).map_err(|e| e.to_string())?).unpack(dest).map_err(|e| e.to_string()),
            ArchiveFormat::TarGz => tar::Archive::new(flate2::read::GzDecoder::new(File::open(src).map_err(|e| e.to_string())?)).unpack(dest).map_err(|e| e.to_string()),
            ArchiveFormat::TarBz2 => tar::Archive::new(bzip2::read::BzDecoder::new(File::open(src).map_err(|e| e.to_string())?)).unpack(dest).map_err(|e| e.to_string()),
            ArchiveFormat::SevenZ => sevenz_rust::decompress_file(src, dest).map_err(|e| e.to_string()),
        }
    }

    fn archive(src: &Path, dest: &Path, fmt: ArchiveFormat, level: u32) -> Result<(), String> {
        match fmt {
            ArchiveFormat::Zip => {
                let mut zw = zip::ZipWriter::new(File::create(dest).map_err(|e| e.to_string())?);
                let opts = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated).compression_level(Some(level as i64));
                Self::zip_dir(&mut zw, src, src, opts)?;
                zw.finish().map(|_| ()).map_err(|e| e.to_string())
            }
            ArchiveFormat::Tar => {
                let mut b = tar::Builder::new(File::create(dest).map_err(|e| e.to_string())?);
                b.append_dir_all(".", src).map_err(|e| e.to_string())?;
                b.finish().map_err(|e| e.to_string())
            }
            ArchiveFormat::TarGz => {
                let enc = flate2::write::GzEncoder::new(File::create(dest).map_err(|e| e.to_string())?, Compression::new(level));
                let mut b = tar::Builder::new(enc);
                b.append_dir_all(".", src).map_err(|e| e.to_string())?;
                b.into_inner().map_err(|e| e.to_string())?.finish().map(|_| ()).map_err(|e| e.to_string())
            }
            ArchiveFormat::TarBz2 => {
                let enc = bzip2::write::BzEncoder::new(File::create(dest).map_err(|e| e.to_string())?, bzip2::Compression::new(level));
                let mut b = tar::Builder::new(enc);
                b.append_dir_all(".", src).map_err(|e| e.to_string())?;
                b.into_inner().map_err(|e| e.to_string())?.finish().map_err(|e| e.to_string()).map(|_| ())
            }
            ArchiveFormat::SevenZ => sevenz_rust::compress_to_path(src, dest).map_err(|e| e.to_string()),
        }
    }

    fn zip_dir(zw: &mut zip::ZipWriter<File>, base: &Path, dir: &Path, opts: zip::write::SimpleFileOptions) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let rel = path.strip_prefix(base).map_err(|e| e.to_string())?;
            let name = rel.to_string_lossy().replace('\\', "/");
            if path.is_dir() {
                zw.add_directory(format!("{}/", name), opts).map_err(|e| e.to_string())?;
                Self::zip_dir(zw, base, &path, opts)?;
            } else {
                zw.start_file(&name, opts).map_err(|e| e.to_string())?;
                io::copy(&mut File::open(&path).map_err(|e| e.to_string())?, zw).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn render_header(ui: &mut egui::Ui, theme: ThemeMode) {
        let (tc, sc) = if matches!(theme, ThemeMode::Dark) { (ColorPalette::ZINC_100, ColorPalette::ZINC_400) } else { (ColorPalette::ZINC_900, ColorPalette::ZINC_600) };
        ui.add_space(12.0);
        ui.label(egui::RichText::new("Archive Converter").size(24.0).color(tc));
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Convert between ZIP, TAR.GZ, TAR.BZ2, TAR, and 7Z formats").size(13.0).color(sc));
        ui.add_space(12.0);
    }

    fn render_format_selector(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border, text) = panel_colors(theme);
        let lc = label_col(theme);
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.label(egui::RichText::new("Target Format").size(14.0).color(text));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for &fmt in ArchiveFormat::all() {
                    let (bg, fg) = format_btn_colors(self.target_format == fmt, ColorPalette::AMBER_600, theme);
                    if ui.add(egui::Button::new(egui::RichText::new(fmt.as_str()).size(13.0).color(fg)).fill(bg).stroke(egui::Stroke::NONE).corner_radius(6.0).min_size(egui::vec2(76.0, 32.0))).clicked() {
                        self.target_format = fmt;
                    }
                }
            });
            match self.target_format {
                ArchiveFormat::Tar => { ui.add_space(6.0); ui.label(egui::RichText::new("TAR stores files without compression. Use TAR.GZ or TAR.BZ2 for smaller output.").size(11.0).color(lc).italics()); }
                ArchiveFormat::SevenZ => { ui.add_space(6.0); ui.label(egui::RichText::new("7Z uses LZMA compression for excellent ratios. Widely supported on all platforms.").size(11.0).color(lc).italics()); }
                _ => {}
            }
        });
    }

    fn render_options(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border, text) = panel_colors(theme);
        let lc = label_col(theme);
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Options").size(14.0).color(text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if self.show_options { "Hide" } else { "Show" }).clicked() { self.show_options = !self.show_options; }
                });
            });
            if self.show_options {
                ui.add_space(12.0);
                ui.scope(|ui| {
                    if !self.target_format.supports_compression_level() { ui.disable(); }
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Compression Level:").color(lc));
                        ui.add(egui::Slider::new(&mut self.compression_level, 1..=9));
                        let desc = match self.compression_level { 1..=3 => "Fast, larger file", 4..=6 => "Balanced", _ => "Slow, smallest file" };
                        ui.label(egui::RichText::new(desc).size(11.0).color(lc).italics());
                    });
                });
                ui.add_space(4.0);
                ui.checkbox(&mut self.overwrite, egui::RichText::new("Overwrite existing files").color(lc));
            }
        });
    }

    fn render_output_dir(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border, text) = panel_colors(theme);
        let lc = label_col(theme);
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.label(egui::RichText::new("Output Directory").size(14.0).color(text));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(self.output_dir.as_ref().map(|d| d.to_string_lossy().to_string()).unwrap_or_else(|| "Same as source files".to_string())).color(lc));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Browse").clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() { self.output_dir = Some(d); }
                    }
                    if self.output_dir.is_some() && ui.button("Clear").clicked() { self.output_dir = None; }
                });
            });
        });
    }

    fn render_file_list(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border, text) = panel_colors(theme);
        let weak = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_500 } else { ColorPalette::ZINC_500 };
        let item_bg = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_900 } else { egui::Color32::WHITE };
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("Archives ({})", self.files.len())).size(14.0).color(text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.files.is_empty() && ui.button("Clear All").clicked() { self.files.clear(); }
                    if ui.button("Add Files").clicked() {
                        if let Some(paths) = rfd::FileDialog::new()
                            .add_filter("Archives", &["zip", "tar", "gz", "tgz", "bz2", "tbz2", "7z"])
                            .pick_files()
                        { self.add_files(paths); }
                    }
                });
            });
            ui.add_space(8.0); ui.separator(); ui.add_space(8.0);
            if self.files.is_empty() {
                let (dz_bg, dz_border) = drop_zone_colors(self.drag_hover, theme);
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 150.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 6.0, dz_bg);
                ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(2.0, dz_border), egui::StrokeKind::Outside);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Drop archives here or click to browse", egui::FontId::proportional(14.0), weak);
                if resp.clicked() {
                    if let Some(paths) = rfd::FileDialog::new()
                        .add_filter("Archives", &["zip", "tar", "gz", "tgz", "bz2", "tbz2", "7z"])
                        .pick_files()
                    { self.add_files(paths); }
                }
            } else {
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    let mut to_remove = None;
                    for (i, f) in self.files.iter().enumerate() {
                        egui::Frame::new().fill(item_bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(f.name()).color(text).size(13.0));
                                    ui.label(egui::RichText::new(format!(
                                        "{} | {}",
                                        f.format.map(|fmt| fmt.as_str()).unwrap_or("Unknown"),
                                        f.size_kb.map(|s| format!("{} KB", s)).unwrap_or_else(|| "Unknown size".to_string())
                                    )).color(weak).size(11.0));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Remove").clicked() { to_remove = Some(i); }
                                });
                            });
                        });
                        ui.add_space(6.0);
                    }
                    if let Some(i) = to_remove { self.files.remove(i); }
                });
            }
        });
    }

    fn render_errors(&self, ui: &mut egui::Ui, theme: ThemeMode) {
        let errs = { let e = self.errors.lock().unwrap(); if e.is_empty() { return; } e.clone() };
        let (bg, border, text) = error_panel_colors(theme);
        egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("Errors ({})", errs.len())).size(14.0).color(text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear").clicked() { self.errors.lock().unwrap().clear(); }
                });
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical().id_salt("ac_err_scroll").max_height(150.0).show(ui, |ui| {
                for e in &errs { ui.label(egui::RichText::new(e).size(12.0).color(text)); ui.add_space(4.0); }
            });
        });
    }

    fn render_progress(&self, ui: &mut egui::Ui, theme: ThemeMode) {
        let p = self.progress.lock().unwrap();
        if p.state == ConvState::Idle { return; }
        let (panel_bg, border, text) = panel_colors(theme);
        let prog_bg = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_200 };
        let fill = match p.state {
            ConvState::Converting => ColorPalette::AMBER_500,
            ConvState::Done => ColorPalette::GREEN_500,
            ConvState::Failed => ColorPalette::RED_500,
            ConvState::Idle => ColorPalette::ZINC_500,
        };
        let frac = if p.total > 0 { p.current as f32 / p.total as f32 } else { 0.0 };
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.label(egui::RichText::new("Conversion Progress").size(14.0).color(text));
            ui.add_space(8.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 4.0, prog_bg);
            ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * frac, rect.height())), 4.0, fill);
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("{:.0}%", frac * 100.0), egui::FontId::proportional(12.0), egui::Color32::WHITE);
            ui.add_space(8.0);
            ui.label(egui::RichText::new(&p.message).size(12.0).color(text));
        });
    }

    fn render_action(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let converting = self.progress.lock().unwrap().state == ConvState::Converting;
        let can = !self.files.is_empty() && !converting;
        let (bg, hover_bg, btn_text) = if can {
            (ColorPalette::AMBER_600, ColorPalette::AMBER_500, egui::Color32::WHITE)
        } else if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_700, ColorPalette::ZINC_700, ColorPalette::ZINC_500)
        } else {
            (ColorPalette::GRAY_300, ColorPalette::GRAY_300, ColorPalette::GRAY_500)
        };
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.scope(|ui| {
                let s = ui.style_mut();
                s.visuals.widgets.inactive.bg_fill = bg;
                s.visuals.widgets.inactive.weak_bg_fill = bg;
                s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                s.visuals.widgets.hovered.bg_fill = hover_bg;
                s.visuals.widgets.hovered.weak_bg_fill = hover_bg;
                s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                if ui.add_enabled(can, egui::Button::new(egui::RichText::new("Convert Archives").size(15.0).color(btn_text)).min_size(egui::vec2(160.0, 40.0)).corner_radius(6.0)).clicked() {
                    self.start_conversion();
                }
            });
        });
    }
}

impl EditorModule for ArchiveConverter {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn save(&mut self) -> Result<(), String> { Ok(()) }
    fn save_as(&mut self) -> Result<(), String> { Ok(()) }
    fn get_title(&self) -> String { "Archive Converter".to_string() }
    
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, _show_file_info: bool) {
        let theme = if ui.visuals().dark_mode { ThemeMode::Dark } else { ThemeMode::Light };
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths: Vec<PathBuf> = i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect();
                self.add_files(paths);
            }
            self.drag_hover = !i.raw.hovered_files.is_empty();
        });
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(8.0);
            Self::render_header(ui, theme);
            ui.add_space(8.0);
            self.render_format_selector(ui, theme);
            ui.add_space(12.0);
            self.render_options(ui, theme);
            ui.add_space(12.0);
            self.render_output_dir(ui, theme);
            ui.add_space(12.0);
            self.render_file_list(ui, theme);
            ui.add_space(12.0);
            self.render_errors(ui, theme);
            ui.add_space(12.0);
            self.render_progress(ui, theme);
            ui.add_space(12.0);
            self.render_action(ui, theme);
            ui.add_space(16.0);
        });
        ctx.request_repaint();
    }
}
