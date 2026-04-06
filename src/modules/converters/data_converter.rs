use eframe::egui;
use std::{
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};
use crate::style::{ColorPalette, ThemeMode};
use crate::modules::EditorModule;
use super::converter_style::{panel_colors, label_col, format_btn_colors, drop_zone_colors, error_panel_colors};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataFormat { Json, Yaml, Toml, Xml, Csv }

impl DataFormat {
    pub fn all() -> &'static [DataFormat] {
        &[Self::Json, Self::Yaml, Self::Toml, Self::Xml, Self::Csv]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "JSON", Self::Yaml => "YAML", Self::Toml => "TOML",
            Self::Xml => "XML", Self::Csv => "CSV",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Json => "json", Self::Yaml => "yaml", Self::Toml => "toml",
            Self::Xml => "xml", Self::Csv => "csv",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "toml" => Some(Self::Toml),
            "xml" => Some(Self::Xml),
            "csv" => Some(Self::Csv),
            _ => None,
        }
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

#[derive(Debug, Clone)]
struct DataFile {
    path: PathBuf,
    format: Option<DataFormat>,
    size_kb: Option<u64>,
}

impl DataFile {
    fn new(path: PathBuf) -> Self {
        let format = path.extension().and_then(|e| e.to_str()).and_then(DataFormat::from_extension);
        let size_kb = std::fs::metadata(&path).ok().map(|m| m.len() / 1024);
        Self { path, format, size_kb }
    }

    fn file_name(&self) -> String {
        self.path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string()
    }
}

pub struct DataConverter {
    files: Vec<DataFile>,
    target_format: DataFormat,
    output_directory: Option<PathBuf>,
    pretty_output: bool,
    overwrite_existing: bool,
    progress: Arc<Mutex<ConversionProgress>>,
    conversion_errors: Arc<Mutex<Vec<String>>>,
    show_options: bool,
    drag_hover: bool,
}

impl DataConverter {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            target_format: DataFormat::Json,
            output_directory: None,
            pretty_output: true,
            overwrite_existing: false,
            progress: Arc::new(Mutex::new(ConversionProgress::default())),
            conversion_errors: Arc::new(Mutex::new(Vec::new())),
            show_options: false,
            drag_hover: false,
        }
    }

    fn add_files(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if DataFormat::from_extension(ext).is_some() && !self.files.iter().any(|f| f.path == path) {
                        self.files.push(DataFile::new(path));
                    }
                }
            }
        }
    }

    fn start_conversion(&mut self) {
        if self.files.is_empty() { return; }
        self.conversion_errors.lock().unwrap().clear();

        let output_dir = self.output_directory.clone().unwrap_or_else(|| {
            self.files[0].path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf()
        });

        let files = self.files.clone();
        let target = self.target_format;
        let pretty = self.pretty_output;
        let overwrite = self.overwrite_existing;
        let progress = Arc::clone(&self.progress);
        let errors = Arc::clone(&self.conversion_errors);

        thread::spawn(move || {
            {
                let mut p = progress.lock().unwrap();
                p.state = ConversionState::Converting;
                p.current = 0;
                p.total = files.len();
                p.message = "Starting conversion...".to_string();
            }

            let (mut ok, mut fail) = (0usize, 0usize);

            for (idx, file) in files.iter().enumerate() {
                {
                    let mut p = progress.lock().unwrap();
                    p.current = idx + 1;
                    p.message = format!("Converting {} ({}/{})", file.file_name(), idx + 1, files.len());
                }

                let Some(src_fmt) = file.format else {
                    errors.lock().unwrap().push(format!("{}: Unknown source format", file.file_name()));
                    fail += 1;
                    continue;
                };

                match Self::convert_file(&file.path, &output_dir, src_fmt, target, pretty, overwrite) {
                    Ok(_) => ok += 1,
                    Err(e) => {
                        errors.lock().unwrap().push(format!("{}: {}", file.file_name(), e));
                        fail += 1;
                    }
                }
            }

            let mut p = progress.lock().unwrap();
            p.state = if fail == 0 { ConversionState::Completed } else { ConversionState::Failed };
            p.message = format!("Completed: {} succeeded, {} failed", ok, fail);
        });
    }

    fn convert_file(input: &PathBuf, output_dir: &PathBuf, from: DataFormat, to: DataFormat, pretty: bool, overwrite: bool) -> Result<(), String> {
        let value = Self::read_as_value(input, from)?;
        let stem = input.file_stem().and_then(|s| s.to_str()).ok_or("Invalid filename")?;
        let out_path = output_dir.join(format!("{}.{}", stem, to.extension()));
        if out_path.exists() && !overwrite {
            return Err("File exists and overwrite is disabled".to_string());
        }
        let out_file = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(out_file);
        Self::write_value(&mut writer, &value, to, pretty)
    }

    fn read_as_value(path: &PathBuf, fmt: DataFormat) -> Result<serde_json::Value, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        match fmt {
            DataFormat::Json => serde_json::from_reader(reader).map_err(|e| e.to_string()),
            DataFormat::Yaml => serde_yaml::from_reader(reader).map_err(|e| e.to_string()),
            DataFormat::Toml => {
                let mut s = String::new();
                BufReader::new(reader).read_to_string(&mut s).map_err(|e| e.to_string())?;
                toml::from_str(&s).map_err(|e| e.to_string())
            }
            DataFormat::Xml => {
                let mut s = String::new();
                BufReader::new(reader).read_to_string(&mut s).map_err(|e| e.to_string())?;
                xml_to_value(&s)
            }
            DataFormat::Csv => {
                let mut rdr = csv::Reader::from_reader(reader);
                let headers: Vec<String> = rdr.headers().map_err(|e| e.to_string())?.iter().map(String::from).collect();
                let rows: Vec<serde_json::Value> = rdr.records()
                    .map(|rec| {
                        let rec = rec.map_err(|e| e.to_string())?;
                        Ok(serde_json::Value::Object(
                            headers.iter().zip(rec.iter())
                                .map(|(h, v)| (h.clone(), serde_json::Value::String(v.to_string())))
                                .collect()
                        ))
                    })
                    .collect::<Result<_, String>>()?;
                Ok(serde_json::Value::Array(rows))
            }
        }
    }

    fn write_value<W: Write>(writer: &mut W, value: &serde_json::Value, fmt: DataFormat, pretty: bool) -> Result<(), String> {
        match fmt {
            DataFormat::Json => {
                if pretty { serde_json::to_writer_pretty(writer, value).map_err(|e| e.to_string()) }
                else { serde_json::to_writer(writer, value).map_err(|e| e.to_string()) }
            }
            DataFormat::Yaml => serde_yaml::to_writer(writer, value).map_err(|e| e.to_string()),
            DataFormat::Toml => {
                let tv = json_to_toml(value)?;
                let s = if pretty { toml::to_string_pretty(&tv) } else { toml::to_string(&tv) }.map_err(|e| e.to_string())?;
                writer.write_all(s.as_bytes()).map_err(|e| e.to_string())
            }
            DataFormat::Xml => {
                let s = value_to_xml(value, pretty)?;
                writer.write_all(s.as_bytes()).map_err(|e| e.to_string())
            }
            DataFormat::Csv => {
                let arr = value.as_array().ok_or("CSV output requires a top-level array of objects")?;
                if arr.is_empty() { return Ok(()); }
                let headers: Vec<String> = arr[0].as_object().ok_or("CSV requires an array of objects")?.keys().cloned().collect();
                let mut wtr = csv::Writer::from_writer(writer);
                wtr.write_record(&headers).map_err(|e| e.to_string())?;
                for row in arr {
                    let obj = row.as_object().ok_or("CSV requires an array of objects")?;
                    let record: Vec<String> = headers.iter()
                        .map(|h| obj.get(h).map(|v| match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        }).unwrap_or_default())
                        .collect();
                    wtr.write_record(&record).map_err(|e| e.to_string())?;
                }
                wtr.flush().map_err(|e| e.to_string())
            }
        }
    }

    fn render_header(&self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (tc, sc) = if matches!(theme, ThemeMode::Dark) { (ColorPalette::ZINC_100, ColorPalette::ZINC_400) } else { (ColorPalette::ZINC_900, ColorPalette::ZINC_600) };
        ui.add_space(12.0);
        ui.label(egui::RichText::new("Data Format Converter").size(24.0).color(tc));
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Convert between JSON, YAML, TOML, XML, and CSV formats").size(13.0).color(sc));
        ui.add_space(12.0);
    }

    fn render_format_selector(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color) = panel_colors(theme);
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
                ui.label(egui::RichText::new("Target Format").size(14.0).color(text_color));
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    for &fmt in DataFormat::all() {
                        let (bg, fg) = format_btn_colors(self.target_format == fmt, ColorPalette::GREEN_600, theme);
                        let btn = egui::Button::new(egui::RichText::new(fmt.as_str()).size(13.0).color(fg))
                            .fill(bg).stroke(egui::Stroke::NONE).corner_radius(6.0)
                            .min_size(egui::vec2(72.0, 32.0));
                        if ui.add(btn).clicked() { self.target_format = fmt; }
                    }
                });
            });
    }

    fn render_options(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color) = panel_colors(theme);
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Options").size(14.0).color(text_color));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(if self.show_options { "Hide" } else { "Show" }).clicked() {
                            self.show_options = !self.show_options;
                        }
                    });
                });
                if self.show_options {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.pretty_output, egui::RichText::new("Pretty output").color(text_color));
                        ui.add_space(16.0);
                        ui.checkbox(&mut self.overwrite_existing, egui::RichText::new("Overwrite existing files").color(text_color));
                    });
                }
            });
    }

    fn render_output_directory(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color) = panel_colors(theme);
        let lc = label_col(theme);
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
                ui.label(egui::RichText::new("Output Directory").size(14.0).color(text_color));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let dir_text = self.output_directory.as_ref()
                        .map(|d| d.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Same as source files".to_string());
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

    fn render_file_list(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (panel_bg, border_color, text_color) = panel_colors(theme);
        let weak_color = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_500 } else { ColorPalette::ZINC_500 };
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Files ({})", self.files.len())).size(14.0).color(text_color));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.files.is_empty() && ui.button("Clear All").clicked() { self.files.clear(); }
                        if ui.button("Add Files").clicked() {
                            if let Some(paths) = rfd::FileDialog::new()
                                .add_filter("Data Files", &["json", "yaml", "yml", "toml", "xml", "csv"])
                                .pick_files()
                            { self.add_files(paths); }
                        }
                    });
                });

                ui.add_space(8.0); ui.separator(); ui.add_space(8.0);

                if self.files.is_empty() {
                    let (drop_bg, drop_border) = drop_zone_colors(self.drag_hover, theme);
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 150.0), egui::Sense::click());
                    ui.painter().rect_filled(rect, 6.0, drop_bg);
                    ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(2.0, drop_border), egui::StrokeKind::Outside);
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Drop files here or click to browse", egui::FontId::proportional(14.0), weak_color);
                    if response.clicked() {
                        if let Some(paths) = rfd::FileDialog::new()
                            .add_filter("Data Files", &["json", "yaml", "yml", "toml", "xml", "csv"])
                            .pick_files()
                        { self.add_files(paths); }
                    }
                } else {
                    let item_bg = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_900 } else { egui::Color32::WHITE };
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        let mut to_remove = None;
                        for (idx, file) in self.files.iter().enumerate() {
                            egui::Frame::new().fill(item_bg).stroke(egui::Stroke::new(1.0, border_color))
                                .corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(egui::RichText::new(&file.file_name()).color(text_color).size(13.0));
                                            let info = format!(
                                                "{} | {}",
                                                file.format.map(|f| f.as_str()).unwrap_or("Unknown"),
                                                file.size_kb.map(|s| format!("{} KB", s)).unwrap_or_else(|| "Unknown size".to_string())
                                            );
                                            ui.label(egui::RichText::new(info).color(weak_color).size(11.0));
                                        });
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("Remove").clicked() { to_remove = Some(idx); }
                                        });
                                    });
                                });
                            ui.add_space(6.0);
                        }
                        if let Some(idx) = to_remove { self.files.remove(idx); }
                    });
                }
            });
    }

    fn render_progress(&self, ui: &mut egui::Ui, theme: ThemeMode) {
        let progress = self.progress.lock().unwrap();
        if progress.state == ConversionState::Idle { return; }
        let (panel_bg, border_color, text_color) = panel_colors(theme);
        let fill = match progress.state {
            ConversionState::Converting => ColorPalette::GREEN_500,
            ConversionState::Completed => ColorPalette::GREEN_600,
            ConversionState::Failed => ColorPalette::RED_500,
            ConversionState::Idle => ColorPalette::ZINC_500,
        };
        let progress_bg = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_200 };
        let fraction = if progress.total > 0 { progress.current as f32 / progress.total as f32 } else { 0.0 };
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
                ui.label(egui::RichText::new("Conversion Progress").size(14.0).color(text_color));
                ui.add_space(8.0);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 4.0, progress_bg);
                let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * fraction, rect.height()));
                ui.painter().rect_filled(fill_rect, 4.0, fill);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("{:.0}%", fraction * 100.0), egui::FontId::proportional(12.0), egui::Color32::WHITE);
                ui.add_space(8.0);
                ui.label(egui::RichText::new(&progress.message).size(12.0).color(text_color));
            });
    }

    fn render_errors(&self, ui: &mut egui::Ui, theme: ThemeMode) {
        let errors_vec = { let e = self.conversion_errors.lock().unwrap(); if e.is_empty() { return; } e.clone() };
        let (panel_bg, border_color, text_color) = error_panel_colors(theme);
        egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Errors ({})", errors_vec.len())).size(14.0).color(text_color));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Clear").clicked() { self.conversion_errors.lock().unwrap().clear(); }
                    });
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical().id_salt("dc_error_scroll").max_height(150.0).show(ui, |ui| {
                    for error in &errors_vec {
                        ui.label(egui::RichText::new(error).size(12.0).color(text_color));
                        ui.add_space(4.0);
                    }
                });
            });
    }

    fn render_action_buttons(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let is_converting = self.progress.lock().unwrap().state == ConversionState::Converting;
        let can_convert = !self.files.is_empty() && !is_converting;
        let (button_bg, button_text) = if can_convert {
            (ColorPalette::GREEN_600, egui::Color32::WHITE)
        } else if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_700, ColorPalette::ZINC_500)
        } else {
            (ColorPalette::GRAY_300, ColorPalette::GRAY_500)
        };
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.scope(|ui| {
                let style = ui.style_mut();
                style.visuals.widgets.inactive.bg_fill = button_bg;
                style.visuals.widgets.inactive.weak_bg_fill = button_bg;
                style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                style.visuals.widgets.hovered.bg_fill = ColorPalette::GREEN_500;
                style.visuals.widgets.hovered.weak_bg_fill = ColorPalette::GREEN_500;
                style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                if ui.add_enabled(can_convert, egui::Button::new(egui::RichText::new("Convert Files").size(15.0).color(button_text)).min_size(egui::vec2(140.0, 40.0)).corner_radius(6.0)).clicked() {
                    self.start_conversion();
                }
            });
        });
    }
}

fn json_to_toml(v: &serde_json::Value) -> Result<toml::Value, String> {
    match v {
        serde_json::Value::Null => Err("TOML does not support null values".to_string()),
        serde_json::Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Ok(toml::Value::Integer(i)) }
            else { Ok(toml::Value::Float(n.as_f64().unwrap_or(0.0))) }
        }
        serde_json::Value::String(s) => Ok(toml::Value::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let items = arr.iter().map(json_to_toml).collect::<Result<_, _>>()?;
            Ok(toml::Value::Array(items))
        }
        serde_json::Value::Object(map) => {
            let table = map.iter()
                .map(|(k, v)| json_to_toml(v).map(|tv| (k.clone(), tv)))
                .collect::<Result<_, _>>()?;
            Ok(toml::Value::Table(table))
        }
    }
}

fn xml_to_value(content: &str) -> Result<serde_json::Value, String> {
    use quick_xml::{Reader, events::Event};
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                let inner = read_xml_children(&mut reader)?;
                let mut root = serde_json::Map::new();
                root.insert(tag, inner);
                return Ok(serde_json::Value::Object(root));
            }
            Event::Eof => return Err("Empty XML document".to_string()),
            _ => {}
        }
    }
}

fn read_xml_children(reader: &mut quick_xml::Reader<&[u8]>) -> Result<serde_json::Value, String> {
    use quick_xml::events::Event;
    let mut map: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    let mut text = String::new();
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                let child = read_xml_children(reader)?;
                match map.entry(tag) {
                    serde_json::map::Entry::Vacant(v) => { v.insert(child); }
                    serde_json::map::Entry::Occupied(mut o) => {
                        if let serde_json::Value::Array(arr) = o.get_mut() { arr.push(child); }
                        else { let prev = o.get().clone(); o.insert(serde_json::Value::Array(vec![prev, child])); }
                    }
                }
            }
            Event::Text(e) => {
                let raw = std::str::from_utf8(e.as_ref()).map_err(|e| e.to_string())?;
                text.push_str(&quick_xml::escape::unescape(raw).map_err(|e| e.to_string())?);
            }
            Event::End(_) | Event::Eof => break,
            _ => {}
        }
    }
    Ok(if map.is_empty() { serde_json::Value::String(text) } else { serde_json::Value::Object(map) })
}

fn value_to_xml(value: &serde_json::Value, pretty: bool) -> Result<String, String> {
    use quick_xml::{Writer, events::*};
    use std::io::Cursor;
    let cursor = Cursor::new(Vec::<u8>::new());
    let mut writer = if pretty { Writer::new_with_indent(cursor, b' ', 2) } else { Writer::new(cursor) };
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None))).map_err(|e| e.to_string())?;
    write_xml_node(&mut writer, "root", value)?;
    let bytes = writer.into_inner().into_inner();
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn write_xml_node<W: Write>(writer: &mut quick_xml::Writer<W>, tag: &str, value: &serde_json::Value) -> Result<(), String> {
    use quick_xml::events::*;
    match value {
        serde_json::Value::Object(map) => {
            writer.write_event(Event::Start(BytesStart::new(tag))).map_err(|e| e.to_string())?;
            for (k, v) in map { write_xml_node(writer, k, v)?; }
            writer.write_event(Event::End(BytesEnd::new(tag))).map_err(|e| e.to_string())?;
        }
        serde_json::Value::Array(arr) => { for item in arr { write_xml_node(writer, tag, item)?; } }
        serde_json::Value::Null => { writer.write_event(Event::Empty(BytesStart::new(tag))).map_err(|e| e.to_string())?; }
        other => {
            let s = match other { serde_json::Value::String(s) => s.clone(), v => v.to_string() };
            writer.write_event(Event::Start(BytesStart::new(tag))).map_err(|e| e.to_string())?;
            writer.write_event(Event::Text(BytesText::new(&s))).map_err(|e| e.to_string())?;
            writer.write_event(Event::End(BytesEnd::new(tag))).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

impl EditorModule for DataConverter {
    fn as_any(&self) -> &dyn std::any::Any { self }

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
            self.render_header(ui, theme);
            ui.add_space(8.0);
            self.render_format_selector(ui, theme);
            ui.add_space(12.0);
            self.render_options(ui, theme);
            ui.add_space(12.0);
            self.render_output_directory(ui, theme);
            ui.add_space(12.0);
            self.render_file_list(ui, theme);
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
    fn get_title(&self) -> String { "Data Format Converter".to_string() }
}
