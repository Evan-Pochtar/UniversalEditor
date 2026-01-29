use eframe::egui;
use super::style::{self, ThemeMode};
use super::modules::{EditorModule, text_editor::TextEditor, image_converter::ImageConverter};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
struct RecentFile {
    path: PathBuf,
    timestamp: i64,
}

#[derive(Serialize, Deserialize)]
struct RecentFiles {
    files: Vec<RecentFile>,
}

impl RecentFiles {
    fn new() -> Self {
        Self { files: Vec::new() }
    }

    fn load() -> Self {
        let config_path = Self::get_config_path();
        if let Ok(contents) = fs::read_to_string(&config_path) {
            if let Ok(recent) = serde_json::from_str(&contents) {
                return recent;
            }
        }
        Self::new()
    }

    fn save(&self) {
        let config_path = Self::get_config_path();
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(config_path, json);
        }
    }

    fn get_config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("universal_editor");
        path.push("recent_files.json");
        path
    }

    fn add_file(&mut self, path: PathBuf) {
        self.files.retain(|f| f.path != path);
        
        let timestamp = chrono::Utc::now().timestamp();
        self.files.insert(0, RecentFile { path, timestamp });
        
        if self.files.len() > 20 {
            self.files.truncate(20);
        }
        
        self.save();
    }

    fn get_files(&self) -> &[RecentFile] {
        &self.files
    }
}

pub struct UniversalEditor {
    active_module: Option<Box<dyn EditorModule>>,
    sidebar_open: bool,
    theme_mode: ThemeMode,
    recent_files: RecentFiles,
    screens_expanded: bool,
    converters_expanded: bool,
    recent_files_expanded: bool,
    show_toolbar: bool,
    show_file_info: bool,
}

impl UniversalEditor {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let initial_theme = match cc.egui_ctx.theme() {
            egui::Theme::Dark => ThemeMode::Dark,
            egui::Theme::Light => ThemeMode::Light,
        };
        
        style::apply_theme(&cc.egui_ctx, initial_theme);
        
        Self {
            active_module: None,
            sidebar_open: true,
            theme_mode: initial_theme,
            recent_files: RecentFiles::load(),
            screens_expanded: false,
            converters_expanded: true,
            recent_files_expanded: true,
            show_toolbar: true,
            show_file_info: true,
        }
    }

    fn open_file(&mut self, path: PathBuf) {
        self.active_module = Some(Box::new(TextEditor::load(path.clone())));
        self.recent_files.add_file(path);
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Text File").clicked() {
                        self.active_module = Some(Box::new(TextEditor::new_empty()));
                        ui.close();
                    }
                    if ui.button("Open...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("Text Files", &["txt", "md"]).pick_file() {
                            self.open_file(path);
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Save (Ctrl+S)").clicked() {
                        if let Some(module) = &mut self.active_module {
                            let _ = module.save();
                        }
                        ui.close();
                    }
                    if ui.button("Save As...").clicked() {
                        if let Some(module) = &mut self.active_module {
                            let _ = module.save_as();
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                   ui.checkbox(&mut self.sidebar_open, "Show Sidebar");
                   ui.checkbox(&mut self.show_toolbar, "Show Toolbar");
                   ui.checkbox(&mut self.show_file_info, "Show File Info");
                   
                   ui.separator();
                   
                   ui.label("Theme:");
                   if ui.selectable_label(matches!(self.theme_mode, ThemeMode::Light), "Light").clicked() {
                       self.theme_mode = ThemeMode::Light;
                       style::apply_theme(ctx, self.theme_mode);
                       ui.close();
                   }
                   if ui.selectable_label(matches!(self.theme_mode, ThemeMode::Dark), "Dark").clicked() {
                       self.theme_mode = ThemeMode::Dark;
                       style::apply_theme(ctx, self.theme_mode);
                       ui.close();
                   }
                });
            });
            ui.add_space(4.0);
        });
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        if !self.sidebar_open { return; }
        
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(240.0)
            .min_width(200.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        
                        style::sidebar_section(ui, "Screens", &mut self.screens_expanded, self.theme_mode, |ui| {
                            if style::sidebar_item(ui, "Text Editor", "T", self.theme_mode).clicked() {
                                self.active_module = Some(Box::new(TextEditor::new_empty()));
                            }
                        });
                        
                        ui.add_space(4.0);
                        
                        style::sidebar_section(ui, "Converters", &mut self.converters_expanded, self.theme_mode, |ui| {
                            if style::sidebar_item(ui, "Image Converter", "I", self.theme_mode).clicked() {
                                self.active_module = Some(Box::new(ImageConverter::new()));
                            }
                        });
                        
                        ui.add_space(4.0);
                        
                        let recent_files: Vec<RecentFile> = self.recent_files.get_files().to_vec();
                        let mut file_to_open: Option<PathBuf> = None;
                        
                        style::sidebar_section(ui, "Recent Files", &mut self.recent_files_expanded, self.theme_mode, |ui| {
                            if recent_files.is_empty() {
                                ui.centered_and_justified(|ui| {
                                    ui.weak("No recent files");
                                });
                            } else {
                                for recent_file in &recent_files {
                                    if recent_file.path.exists() {
                                        let file_name = recent_file.path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Unknown");
                                        
                                        if style::sidebar_item(ui, file_name, "F", self.theme_mode).clicked() {
                                            file_to_open = Some(recent_file.path.clone());
                                        }
                                    }
                                }
                            }
                        });
                        
                        if let Some(path) = file_to_open {
                            self.open_file(path);
                        }
                        
                        ui.add_space(8.0);
                    });

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.weak("FPS:");
                        ui.label(format!("{:.0}", 1.0 / ctx.input(|i| i.unstable_dt)));
                    });
                    ui.add_space(4.0);
                });
            });
    }

    fn landing_page(&mut self, ui: &mut egui::Ui) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                
                ui.heading(egui::RichText::new("UNIVERSAL EDITOR").size(32.0));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("A modern, modular editor").size(14.0));
                
                ui.add_space(40.0);

                if style::primary_button(ui, "New Text File", self.theme_mode).clicked() {
                    self.active_module = Some(Box::new(TextEditor::new_empty()));
                }
                ui.add_space(12.0);
                if style::secondary_button(ui, "Open File", self.theme_mode).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Text Files", &["txt", "md"]).pick_file() {
                        self.open_file(path);
                    }
                }
                ui.add_space(12.0);
                if style::secondary_button(ui, "Image Converter", self.theme_mode).clicked() {
                    self.active_module = Some(Box::new(ImageConverter::new()));
                }
            });
        });
    }
}

impl eframe::App for UniversalEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.top_bar(ctx);
        self.sidebar(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(module) = &mut self.active_module {
                module.ui(ui, ctx, self.show_toolbar, self.show_file_info);
            } else {
                self.landing_page(ui);
            }
        });
    }
}
