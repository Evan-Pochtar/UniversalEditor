use eframe::egui;
use crate::style::ColorPalette;
use super::style::{self, ThemeMode};
use super::modules::{EditorModule, text_editor::TextEditor, image_converter::ImageConverter, image_editor::ImageEditor};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
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

    fn remove_file(&mut self, path: &PathBuf) {
        self.files.retain(|f| &f.path != path);
        self.save();
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize)]
struct AppSettings {
    theme_preference: ThemePreference,
    show_toolbar: bool,
    show_file_info: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme_preference: ThemePreference::System,
            show_toolbar: true,
            show_file_info: true,
        }
    }
}

impl AppSettings {
    fn load() -> Self {
        let config_path = Self::get_config_path();
        if let Ok(contents) = fs::read_to_string(&config_path) {
            if let Ok(settings) = serde_json::from_str(&contents) {
                return settings;
            }
        }
        Self::default()
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
        path.push("app_settings.json");
        path
    }
}

enum PendingAction {
    OpenFile(PathBuf),
    NewFile,
    SwitchModule(Box<dyn EditorModule>),
    Exit,
}

pub struct UniversalEditor {
    active_module: Option<Box<dyn EditorModule>>,
    sidebar_open: bool,
    theme_mode: ThemeMode,
    theme_preference: ThemePreference,
    recent_files: RecentFiles,
    screens_expanded: bool,
    converters_expanded: bool,
    recent_files_expanded: bool,
    show_toolbar: bool,
    show_file_info: bool,
    show_unsaved_dialog: bool,
    pending_action: Option<PendingAction>,
    recent_file_tx: SyncSender<PathBuf>,
    recent_file_rx: Receiver<PathBuf>,
}

impl UniversalEditor {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = AppSettings::load();
        
        let system_theme = match cc.egui_ctx.theme() {
            egui::Theme::Dark => ThemeMode::Dark,
            egui::Theme::Light => ThemeMode::Light,
        };
        
        let initial_theme = match settings.theme_preference {
            ThemePreference::System => system_theme,
            ThemePreference::Light => ThemeMode::Light,
            ThemePreference::Dark => ThemeMode::Dark,
        };
        
        style::apply_theme(&cc.egui_ctx, initial_theme);
        
        let (tx, rx) = sync_channel(20);
        
        Self {
            active_module: None,
            sidebar_open: true,
            theme_mode: initial_theme,
            theme_preference: settings.theme_preference,
            recent_files: RecentFiles::load(),
            screens_expanded: false,
            converters_expanded: false,
            recent_files_expanded: false,
            show_toolbar: settings.show_toolbar,
            show_file_info: settings.show_file_info,
            show_unsaved_dialog: false,
            pending_action: None,
            recent_file_tx: tx,
            recent_file_rx: rx,
        }
    }

    fn is_in_text_editor(&self) -> bool {
        if let Some(module) = &self.active_module {
            module.as_any().downcast_ref::<TextEditor>().is_some()
        } else {
            false
        }
    }

    fn has_unsaved_changes(&self) -> bool {
        if let Some(module) = &self.active_module {
            if let Some(text_editor) = module.as_any().downcast_ref::<TextEditor>() {
                return text_editor.is_dirty();
            }
            if let Some(image_editor) = module.as_any().downcast_ref::<ImageEditor>() {
                return image_editor.is_dirty();
            }
        }
        false
    }

    fn open_file(&mut self, path: PathBuf) {
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::OpenFile(path));
            self.show_unsaved_dialog = true;
        } else {
            self.recent_files.add_file(path.clone());
            
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase());
            
            let module: Box<dyn EditorModule> = match ext.as_deref() {
                Some("jpg") | Some("jpeg") | Some("png") | Some("webp") | 
                Some("bmp") | Some("tiff") | Some("tif") | Some("gif") | Some("ico") => {
                    let mut editor = ImageEditor::load(path);
                    let tx = self.recent_file_tx.clone();
                    editor.set_file_callback(Box::new(move |p: PathBuf| {
                        let _ = tx.send(p);
                    }));
                    Box::new(editor)
                }
                _ => Box::new(TextEditor::load(path)),
            };
            
            self.active_module = Some(module);
        }
    }

    fn new_text_file(&mut self) {
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::NewFile);
            self.show_unsaved_dialog = true;
        } else {
            self.active_module = Some(Box::new(TextEditor::new_empty()));
        }
    }

    fn switch_to_module(&mut self, module: Box<dyn EditorModule>) {
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::SwitchModule(module));
            self.show_unsaved_dialog = true;
        } else {
            self.active_module = Some(module);
        }
    }

    fn execute_pending_action(&mut self) {
        if let Some(action) = self.pending_action.take() {
            match action {
                PendingAction::OpenFile(path) => {
                    self.recent_files.add_file(path.clone());
                    
                    let ext = path.extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_lowercase());
                    
                    let module: Box<dyn EditorModule> = match ext.as_deref() {
                        Some("jpg") | Some("jpeg") | Some("png") | Some("webp") | 
                        Some("bmp") | Some("tiff") | Some("tif") | Some("gif") | Some("ico") => {
                            let mut editor = ImageEditor::load(path);
                            editor.set_file_callback(Box::new(move |p: PathBuf| {
                                let mut recent = RecentFiles::load();
                                recent.add_file(p);
                            }));
                            Box::new(editor)
                        }
                        _ => Box::new(TextEditor::load(path)),
                    };
                    
                    self.active_module = Some(module);
                }
                PendingAction::NewFile => {
                    self.active_module = Some(Box::new(TextEditor::new_empty()));
                }
                PendingAction::SwitchModule(module) => {
                    self.active_module = Some(module);
                }
                PendingAction::Exit => {}
            }
        }
    }

    fn save_settings(&self) {
        let settings = AppSettings {
            theme_preference: self.theme_preference,
            show_toolbar: self.show_toolbar,
            show_file_info: self.show_file_info,
        };
        settings.save();
    }

    fn create_image_editor_with_callback(&self) -> Box<dyn EditorModule> {
        let mut editor = ImageEditor::new();
        let tx = self.recent_file_tx.clone();
        
        editor.set_file_callback(Box::new(move |path: PathBuf| {
            let _ = tx.send(path);
        }));
        Box::new(editor)
    }

    fn render_unsaved_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_unsaved_dialog {
            return;
        }

        let (bg_color, border_color, text_color, overlay_color) = if matches!(self.theme_mode, ThemeMode::Dark) {
            (
                ColorPalette::ZINC_800,
                ColorPalette::ZINC_700,
                ColorPalette::ZINC_100,
                egui::Color32::from_rgba_premultiplied(0, 0, 0, 200),
            )
        } else {
            (
                egui::Color32::WHITE,
                ColorPalette::GRAY_300,
                ColorPalette::GRAY_900,
                egui::Color32::from_rgba_premultiplied(0, 0, 0, 150),
            )
        };

        egui::Area::new(egui::Id::new("overlay"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                let screen_rect = ctx.content_rect();
                let painter = ui.painter();
                painter.rect_filled(screen_rect, 0.0, overlay_color);
            });

        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Tooltip)
            .frame(egui::Frame::new()
                .fill(bg_color)
                .stroke(egui::Stroke::new(1.0, border_color))
                .corner_radius(8.0)
                .inner_margin(24.0))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    
                    ui.label(
                        egui::RichText::new("Do you want to save changes?")
                            .size(16.0)
                            .color(text_color)
                    );
                    
                    ui.add_space(8.0);
                    
                    ui.label(
                        egui::RichText::new("Your changes will be lost if you don't save them.")
                            .size(13.0)
                            .color(if matches!(self.theme_mode, ThemeMode::Dark) {
                                ColorPalette::ZINC_400
                            } else {
                                ColorPalette::GRAY_600
                            })
                    );
                    
                    ui.add_space(24.0);
                    
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        
                        let save_clicked = style::primary_button(ui, "Save", self.theme_mode).clicked();
                        let dont_save_clicked = style::secondary_button(ui, "Don't Save", self.theme_mode).clicked();
                        let cancel_clicked = style::secondary_button(ui, "Cancel", self.theme_mode).clicked();
                        
                        if save_clicked {
                            if let Some(module) = &mut self.active_module {
                                let _ = module.save();
                            }
                            self.show_unsaved_dialog = false;
                            self.execute_pending_action();
                        }
                        
                        if dont_save_clicked {
                            self.show_unsaved_dialog = false;
                            self.execute_pending_action();
                        }
                        
                        if cancel_clicked {
                            self.show_unsaved_dialog = false;
                            self.pending_action = None;
                        }
                    });
                    
                    ui.add_space(8.0);
                });
            });
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        let contributions = if let Some(module) = &self.active_module {
            module.get_menu_contributions()
        } else {
            Default::default()
        };

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Text File").clicked() {
                        self.new_text_file();
                        ui.close();
                    }
                    if ui.button("Open...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("All Files", &["txt", "md", "jpg", "jpeg", "png", "webp", "bmp", "tiff", "tif", "gif", "ico"])
                            .pick_file() 
                        {
                            self.open_file(path);
                        }
                        ui.close();
                    }
                    ui.separator();
                    
                    let has_module = self.active_module.is_some();
                    if ui.add_enabled(has_module, egui::Button::new("Save (Ctrl+S)")).clicked() {
                        if let Some(module) = &mut self.active_module {
                            let _ = module.save();
                        }
                        ui.close();
                    }
                    if ui.add_enabled(has_module, egui::Button::new("Save As...")).clicked() {
                        if let Some(module) = &mut self.active_module {
                            let _ = module.save_as();
                        }
                        ui.close();
                    }
                    
                    if !contributions.file_items.is_empty() {
                        ui.separator();
                        for (item, action) in &contributions.file_items {
                            let label = if let Some(ref shortcut) = item.shortcut {
                                format!("{} ({})", item.label, shortcut)
                            } else {
                                item.label.clone()
                            };
                            
                            if ui.add_enabled(item.enabled, egui::Button::new(label)).clicked() {
                                if let Some(module) = &mut self.active_module {
                                    module.handle_menu_action(action.clone());
                                }
                                ui.close();
                            }
                        }
                    }
                    
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        if self.has_unsaved_changes() {
                            self.pending_action = Some(PendingAction::Exit);
                            self.show_unsaved_dialog = true;
                        } else {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.close();
                    }
                });

                if !contributions.edit_items.is_empty() {
                    ui.menu_button("Edit", |ui| {
                        for (item, action) in &contributions.edit_items {
                            let label = if let Some(ref shortcut) = item.shortcut {
                                format!("{} ({})", item.label, shortcut)
                            } else {
                                item.label.clone()
                            };
                            
                            if ui.add_enabled(item.enabled, egui::Button::new(label)).clicked() {
                                if let Some(module) = &mut self.active_module {
                                    module.handle_menu_action(action.clone());
                                }
                                ui.close();
                            }
                        }
                    });
                }
                
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.sidebar_open, "Show Sidebar");
                   
                    if self.is_in_text_editor() {
                        let toolbar_changed = ui.checkbox(&mut self.show_toolbar, "Show Toolbar").changed();
                        let file_info_changed = ui.checkbox(&mut self.show_file_info, "Show File Info").changed();
                       
                        if toolbar_changed || file_info_changed {
                            self.save_settings();
                        }
                    }

                    if !contributions.view_items.is_empty() {
                        ui.separator();
                        for (item, action) in &contributions.view_items {
                            let label = if let Some(ref shortcut) = item.shortcut {
                                format!("{} ({})", item.label, shortcut)
                            } else {
                                item.label.clone()
                            };
                            
                            if ui.add_enabled(item.enabled, egui::Button::new(label)).clicked() {
                                if let Some(module) = &mut self.active_module {
                                    module.handle_menu_action(action.clone());
                                }
                                ui.close();
                            }
                        }
                    }

                    ui.separator();
                   
                    ui.label("Theme:");
                   
                    let system_clicked = ui.selectable_label(
                        matches!(self.theme_preference, ThemePreference::System), 
                        "System"
                    ).clicked();
                   
                    let light_clicked = ui.selectable_label(
                        matches!(self.theme_preference, ThemePreference::Light), 
                        "Light"
                    ).clicked();
                   
                    let dark_clicked = ui.selectable_label(
                        matches!(self.theme_preference, ThemePreference::Dark), 
                        "Dark"
                    ).clicked();
                   
                    if system_clicked {
                        self.theme_preference = ThemePreference::System;
                        let system_theme = match ctx.theme() {
                            egui::Theme::Dark => ThemeMode::Dark,
                            egui::Theme::Light => ThemeMode::Light,
                        };
                        self.theme_mode = system_theme;
                        style::apply_theme(ctx, self.theme_mode);
                        self.save_settings();
                        ui.close();
                    }
                   
                    if light_clicked {
                        self.theme_preference = ThemePreference::Light;
                        self.theme_mode = ThemeMode::Light;
                        style::apply_theme(ctx, self.theme_mode);
                        self.save_settings();
                        ui.close();
                    }
                   
                    if dark_clicked {
                        self.theme_preference = ThemePreference::Dark;
                        self.theme_mode = ThemeMode::Dark;
                        style::apply_theme(ctx, self.theme_mode);
                        self.save_settings();
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
                        
                        let mut new_text_file_clicked = false;
                        let mut image_editor_clicked = false;
                        let mut image_converter_clicked = false;
                        
                        let theme_mode = self.theme_mode;
                        
                        style::sidebar_section(ui, "Screens", &mut self.screens_expanded, theme_mode, |ui| {
                            if style::sidebar_item(ui, "Text Editor", "T", theme_mode).clicked() {
                                new_text_file_clicked = true;
                            }
                            if style::sidebar_item(ui, "Image Editor", "I", theme_mode).clicked() {
                                image_editor_clicked = true;
                            }
                        });
                        
                        if new_text_file_clicked {
                            self.new_text_file();
                        }
                        if image_editor_clicked {
                            self.switch_to_module(self.create_image_editor_with_callback());
                        }
                                                
                        style::sidebar_section(ui, "Converters", &mut self.converters_expanded, theme_mode, |ui| {
                            if style::sidebar_item(ui, "Image Converter", "C", theme_mode).clicked() {
                                image_converter_clicked = true;
                            }
                        });
                        
                        if image_converter_clicked {
                            let converter = Box::new(ImageConverter::new());
                            self.switch_to_module(converter);
                        }
                                                
                        let recent_files: Vec<RecentFile> = self.recent_files.get_files().to_vec();
                        let mut file_to_open: Option<PathBuf> = None;
                        let mut file_to_remove: Option<PathBuf> = None;
                        
                        style::sidebar_section(ui, "Recent Files", &mut self.recent_files_expanded, theme_mode, |ui| {                 
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
                                        
                                        ui.horizontal(|ui| {
                                            if style::sidebar_item(ui, file_name, "F", theme_mode).clicked() {
                                                file_to_open = Some(recent_file.path.clone());
                                            }
                                            
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let delete_color = if matches!(theme_mode, ThemeMode::Dark) {
                                                    ColorPalette::SLATE_100
                                                } else {
                                                    ColorPalette::GRAY_600
                                                };
                                                
                                                if ui.button(egui::RichText::new("🗑").color(delete_color).size(14.0)).clicked() {
                                                    file_to_remove = Some(recent_file.path.clone());
                                                }
                                            });
                                        });
                                    }
                                }
                            }
                        });
                        
                        if let Some(path) = file_to_remove {
                            self.recent_files.remove_file(&path);
                        }
                        
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
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("All Files", &["txt", "md", "jpg", "jpeg", "png", "webp", "bmp", "tiff", "tif", "gif", "ico"])
                        .pick_file() 
                    {
                        self.open_file(path);
                    }
                }
                ui.add_space(12.0);
                if style::secondary_button(ui, "Image Editor", self.theme_mode).clicked() {
                    self.active_module = Some(self.create_image_editor_with_callback());
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
        if matches!(self.theme_preference, ThemePreference::System) {
            let system_theme = match ctx.theme() {
                egui::Theme::Dark => ThemeMode::Dark,
                egui::Theme::Light => ThemeMode::Light,
            };
            
            if self.theme_mode != system_theme {
                self.theme_mode = system_theme;
                style::apply_theme(ctx, self.theme_mode);
            }
        }

        while let Ok(path) = self.recent_file_rx.try_recv() {
            self.recent_files.add_file(path);
        }

        if let Some(PendingAction::Exit) = &self.pending_action {
            if !self.show_unsaved_dialog {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        self.render_unsaved_dialog(ctx);
        self.top_bar(ctx);
        self.sidebar(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(module) = &mut self.active_module {
                module.ui(ui, ctx, self.show_toolbar, self.show_file_info);
            } else {
                self.landing_page(ui);
            }
        });

        self.render_unsaved_dialog(ctx);
    }
}
