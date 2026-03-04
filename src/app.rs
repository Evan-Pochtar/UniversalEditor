use eframe::egui;
use crate::style::ColorPalette;
use super::style::{self, ThemeMode};
use super::modules::{EditorModule, text_edit::TextEditor, image_converter::ImageConverter, image_edit::ImageEditor, json_edit::JsonEditor, data_converter::DataConverter};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use crate::registry::{self, CreateModule};
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
struct RecentFile { path: PathBuf, timestamp: i64 }

#[derive(Serialize, Deserialize)]
struct RecentFiles { files: Vec<RecentFile> }

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
pub enum ThemePreference { System, Light, Dark }

fn default_font_name() -> String { "Ubuntu".to_string() }
fn default_font_size() -> f32 { 14.0 }

#[derive(Serialize, Deserialize)]
struct AppSettings {
    theme_preference: ThemePreference,

    show_toolbar_te: bool,
    show_file_info_te: bool,
    #[serde(default = "default_font_name")]
    default_font: String,
    #[serde(default = "default_font_size")]
    default_font_size: f32,

    show_file_info_je: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme_preference: ThemePreference::System,
            show_toolbar_te: true,
            show_file_info_te: true,
            default_font: default_font_name(),
            default_font_size: default_font_size(),
            show_file_info_je: true,
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

enum PendingAction { OpenFile(PathBuf), NewFile, SwitchModule(Box<dyn EditorModule>), GoHome, Exit }

#[derive(PartialEq)]
enum HomeAction { NewTextFile, OpenFile, OpenScreen(&'static str), OpenConverter(&'static str), ShowSettings, ShowPatchNotes }

struct PatchNote { module_tag: String,text: String }

struct PatchCategory { name: String, notes: Vec<PatchNote> }

struct PatchVersion { version: String, tag: String, categories: Vec<PatchCategory> }

#[derive(PartialEq, Clone, Copy)]
enum SettingsTab { General, TextEditor, JsonEditor }

pub struct UniversalEditor {
    active_module: Option<Box<dyn EditorModule>>,
    sidebar_open: bool,
    theme_mode: ThemeMode,
    theme_preference: ThemePreference,
    recent_files: RecentFiles,
    screens_expanded: bool,
    converters_expanded: bool,
    recent_files_expanded: bool,
    show_toolbar_te: bool,
    show_file_info_te: bool,
    show_file_info_je: bool,
    default_font: String,
    default_font_size: f32,
    show_unsaved_dialog: bool,
    show_patch_notes: bool,
    show_settings: bool,
    settings_tab: SettingsTab,
    pending_action: Option<PendingAction>,
    recent_file_tx: SyncSender<PathBuf>,
    recent_file_rx: Receiver<PathBuf>,
    path_replace_tx: SyncSender<(PathBuf, PathBuf)>,
    path_replace_rx: Receiver<(PathBuf, PathBuf)>,
    patch_notes: Vec<PatchVersion>,
    patch_notes_page: usize,
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
        style::register_fonts(&cc.egui_ctx);

        let (tx, rx) = sync_channel(20);
        let (replace_tx, replace_rx) = sync_channel::<(PathBuf, PathBuf)>(20);

        let patch_content = include_str!("../Patchnotes.md");
        let mut patch_notes: Vec<PatchVersion> = Vec::new();
        let mut current_version: Option<PatchVersion> = None;
        let mut current_category: Option<usize> = None;

        fn parse_note(raw: &str) -> PatchNote {
            let raw = raw.trim();
            if raw.starts_with("**") {
                if let Some(end) = raw[2..].find("**") {
                    let tag = raw[2..end + 2].to_string();
                    let rest = raw[end + 4..].trim_start_matches(':').trim().to_string();
                    return PatchNote { module_tag: tag, text: rest };
                }
            }
            PatchNote { module_tag: String::new(), text: raw.to_string() }
        }

        for line in patch_content.lines() {
            let line = line.trim();
            if line.starts_with("## V") {
                if let Some(v) = current_version.take() { patch_notes.push(v); }
                let ver_str = line.trim_start_matches("## ").trim().to_string();
                current_version = Some(PatchVersion { version: ver_str, tag: String::new(), categories: Vec::new() });
                current_category = None;
            } else if line.starts_with("#### ") {
                let cat_name = line.trim_start_matches("#### ").trim().to_string();
                if let Some(v) = &mut current_version {
                    v.categories.push(PatchCategory { name: cat_name, notes: Vec::new() });
                    current_category = Some(v.categories.len() - 1);
                }
            } else if line.starts_with("- ") || line.starts_with("* ") {
                if let Some(v) = &mut current_version {
                    let note = parse_note(&line[2..]);
                    if let Some(idx) = current_category {
                        v.categories[idx].notes.push(note);
                    } else {
                        if v.categories.is_empty() {
                            v.categories.push(PatchCategory { name: String::new(), notes: Vec::new() });
                        }
                        let last = v.categories.len() - 1;
                        v.categories[last].notes.push(note);
                    }
                }
            }
        }
        if let Some(v) = current_version { patch_notes.push(v); }
        patch_notes.reverse();
        let total = patch_notes.len();
        for (i, v) in patch_notes.iter_mut().enumerate() {
            v.tag = if i == 0 { "Current".to_string() }
                    else if i == total - 1 { "Initial Release".to_string() }
                    else { "Update".to_string() };
        }
        
        Self {
            active_module: None,
            sidebar_open: true,
            theme_mode: initial_theme,
            theme_preference: settings.theme_preference,
            recent_files: RecentFiles::load(),
            screens_expanded: false,
            converters_expanded: false,
            recent_files_expanded: false,
            show_toolbar_te: settings.show_toolbar_te,
            show_file_info_te: settings.show_file_info_te,
            show_file_info_je: settings.show_file_info_je,
            default_font: settings.default_font,
            default_font_size: settings.default_font_size,
            show_unsaved_dialog: false,
            show_patch_notes: false,
            show_settings: false,
            settings_tab: SettingsTab::General,
            pending_action: None,
            recent_file_tx: tx,
            recent_file_rx: rx,
            path_replace_tx: replace_tx,
            path_replace_rx: replace_rx,
            patch_notes,
            patch_notes_page: 0,
        }
    }

    fn is_in_text_editor(&self) -> bool {
        if let Some(module) = &self.active_module {
            module.as_any().downcast_ref::<TextEditor>().is_some()
        } else {
            false
        }
    }

    fn is_in_json_editor(&self) -> bool {
        if let Some(module) = &self.active_module {
            module.as_any().downcast_ref::<JsonEditor>().is_some()
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
            if let Some(json_editor) = module.as_any().downcast_ref::<JsonEditor>() {
                return json_editor.is_dirty();
            }
        }
        false
    }

    fn apply_default_font(&self, editor: &mut TextEditor) {
        editor.set_default_font(
            egui::FontFamily::Name(self.default_font.clone().into()),
            self.default_font_size,
        );
    }

    fn instantiate(&self, create: CreateModule, path: Option<PathBuf>) -> Box<dyn EditorModule> {
        match create {
            CreateModule::TextEditor => {
                let mut e = if let Some(p) = path { TextEditor::load(p) } else { TextEditor::new_empty() };
                self.apply_default_font(&mut e);
                e.set_path_replace_tx(self.path_replace_tx.clone());
                Box::new(e)
            }
            CreateModule::ImageEditor => {
                let mut e = if let Some(p) = path { ImageEditor::load(p) } else { ImageEditor::new() };
                let tx = self.recent_file_tx.clone();
                e.set_file_callback(Box::new(move |p: PathBuf| { let _ = tx.send(p); }));
                Box::new(e)
            }
            CreateModule::JsonEditor => Box::new(if let Some(p) = path { JsonEditor::load(p) } else { JsonEditor::new_empty() }),
            CreateModule::ImageConverter => Box::new(ImageConverter::new()),
            CreateModule::DataConverter => Box::new(DataConverter::new()),
        }
    }

    fn module_from_path(&self, path: PathBuf) -> Box<dyn EditorModule> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let create = registry::screen_for_extension(ext)
            .map(|s| s.create)
            .unwrap_or(CreateModule::TextEditor);
        self.instantiate(create, Some(path))
    }

    fn open_file(&mut self, path: PathBuf) {
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::OpenFile(path));
            self.show_unsaved_dialog = true;
        } else {
            self.recent_files.add_file(path.clone());
            self.active_module = Some(self.module_from_path(path));
        }
    }

    fn new_text_file(&mut self) {
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::NewFile);
            self.show_unsaved_dialog = true;
        } else {
            let mut editor = TextEditor::new_empty();
            self.apply_default_font(&mut editor);
            self.active_module = Some(Box::new(editor));
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

    fn go_home(&mut self) {
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::GoHome);
            self.show_unsaved_dialog = true;
        } else {
            self.active_module = None;
        }
    }

    fn execute_pending_action(&mut self) {
        if let Some(action) = self.pending_action.take() {
            match action {
                PendingAction::OpenFile(path) => {
                    self.recent_files.add_file(path.clone());
                    self.active_module = Some(self.module_from_path(path));
                }
                PendingAction::NewFile => {
                    let mut editor = TextEditor::new_empty();
                    self.apply_default_font(&mut editor);
                    self.active_module = Some(Box::new(editor));
                }
                PendingAction::SwitchModule(module) => {
                    self.active_module = Some(module);
                }
                PendingAction::GoHome => {
                    self.active_module = None;
                }
                PendingAction::Exit => {}
            }
        }
    }

    fn save_settings(&self) {
        let settings = AppSettings {
            theme_preference: self.theme_preference,
            show_toolbar_te: self.show_toolbar_te,
            show_file_info_te: self.show_file_info_te,
            default_font: self.default_font.clone(),
            default_font_size: self.default_font_size,
            show_file_info_je: self.show_file_info_je,
        };
        settings.save();
    }

    fn render_unsaved_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_unsaved_dialog {
            return;
        }

        let (bg_color, border_color, text_color) = if matches!(self.theme_mode, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::ZINC_700, ColorPalette::ZINC_100)
        } else {
            (egui::Color32::WHITE, ColorPalette::GRAY_300, ColorPalette::GRAY_900)
        };

        style::draw_modal_overlay(ctx, "unsaved_overlay", 200);

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
                        let save_clicked = style::primary_button(ui, "Save").clicked();
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
                let has_module = self.active_module.is_some();
                let mut go_home = false;

                if has_module {
                    if ui.button("Home").clicked() { go_home = true; }
                    ui.separator();
                }
                if go_home { self.go_home(); return; }

                ui.menu_button("File", |ui| {
                    if ui.button("Open...").clicked() {
                        let exts = registry::all_accepted_extensions();
                        if let Some(path) = rfd::FileDialog::new().add_filter("All Files", &exts).pick_file() {
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
                        let toolbar_changed = ui.checkbox(&mut self.show_toolbar_te, "Show Toolbar").changed();
                        let file_info_changed = ui.checkbox(&mut self.show_file_info_te, "Show File Info").changed();
                       
                        if toolbar_changed || file_info_changed {
                            self.save_settings();
                        }
                    }

                    if self.is_in_json_editor() {
                        let file_info_changed = ui.checkbox(&mut self.show_file_info_je, "Show File Info").changed();                       
                        if file_info_changed {
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
                    let system_clicked = ui.selectable_label(matches!(self.theme_preference, ThemePreference::System), "System").clicked();
                    let light_clicked = ui.selectable_label(matches!(self.theme_preference, ThemePreference::Light), "Light").clicked();
                    let dark_clicked = ui.selectable_label(matches!(self.theme_preference, ThemePreference::Dark), "Dark").clicked();
                   
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

                if !contributions.image_items.is_empty() {
                    ui.menu_button("Image", |ui| {
                        for (item, action) in &contributions.image_items {
                            if item.label == "Separator" {
                                ui.separator();
                                continue;
                            }
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

                if !contributions.filter_items.is_empty() {
                    ui.menu_button("Filter", |ui| {
                        for (item, action) in &contributions.filter_items {
                            if item.label == "Separator" {
                                ui.separator();
                                continue;
                            }
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
                        let mut open_screen: Option<&'static str> = None;
                        let mut open_converter: Option<&'static str> = None;
                        let theme_mode = self.theme_mode;

                        style::sidebar_section(ui, "Screens", &mut self.screens_expanded, theme_mode, |ui| {
                            for s in registry::SCREENS {
                                if style::sidebar_item(ui, s.name, s.sidebar_letter, theme_mode).clicked() {
                                    open_screen = Some(s.id);
                                }
                            }
                        });

                        if let Some(id) = open_screen {
                            if let Some(s) = registry::SCREENS.iter().find(|s| s.id == id) {
                                let m = self.instantiate(s.create, None);
                                self.switch_to_module(m);
                            }
                        }

                        style::sidebar_section(ui, "Converters", &mut self.converters_expanded, theme_mode, |ui| {
                            for c in registry::CONVERTERS {
                                if style::sidebar_item(ui, c.name, c.sidebar_letter, theme_mode).clicked() {
                                    open_converter = Some(c.id);
                                }
                            }
                        });

                        if let Some(id) = open_converter {
                            if let Some(c) = registry::CONVERTERS.iter().find(|c| c.id == id) {
                                let m = self.instantiate(c.create, None);
                                self.switch_to_module(m);
                            }
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
        let theme = self.theme_mode;
        let (title_col, sub_col, accent_line, ver_bg, ver_text_col) = match theme {
            ThemeMode::Dark => (
                egui::Color32::WHITE,
                ColorPalette::ZINC_400,
                ColorPalette::ZINC_800,
                egui::Color32::from_rgb(32, 32, 40),
                ColorPalette::ZINC_400,
            ),
            ThemeMode::Light => (
                ColorPalette::GRAY_900,
                ColorPalette::GRAY_500,
                ColorPalette::GRAY_200,
                ColorPalette::GRAY_100,
                ColorPalette::GRAY_500,
            ),
        };

        let mut action: Option<HomeAction> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let avail_w = ui.available_width();
                let h_pad   = 48.0_f32.max((avail_w - 960.0) / 2.0);
                let margin  = egui::Margin { left: h_pad as i8, right: h_pad as i8, ..Default::default() };

                ui.add_space(36.0);

                egui::Frame::new().inner_margin(margin).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Universal Editor").size(38.0).strong().color(title_col));
                                ui.add_space(10.0);
                                egui::Frame::new().fill(ver_bg).corner_radius(10.0).inner_margin(egui::Margin { left: 8, right: 8, top: 3, bottom: 3 })
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("v".to_owned() + env!("CARGO_PKG_VERSION")).size(11.0).color(ver_text_col));
                                    });
                            });
                            ui.label(egui::RichText::new("A modular suite for text and media").size(14.0).color(sub_col));
                        });

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if style::ghost_button(ui, "About", true, theme).clicked() {}
                                ui.add_space(4.0);
                                if style::ghost_button(ui, "Patch Notes", false, theme).clicked() {
                                    action = Some(HomeAction::ShowPatchNotes);
                                }
                                ui.add_space(4.0);
                                if style::ghost_button(ui, "Settings", false, theme).clicked() {
                                    action = Some(HomeAction::ShowSettings);
                                }
                            },
                        );
                    });
                });

                ui.add_space(20.0);

                let start_x = ui.cursor().min.x;
                let sep_y = ui.cursor().min.y;
                let sep_rect = egui::Rect::from_min_size(
                    egui::pos2(start_x, sep_y),
                    egui::vec2(avail_w, 1.0),
                );
                ui.allocate_rect(sep_rect, egui::Sense::hover());
                ui.painter().rect_filled(sep_rect, 0.0, accent_line);
                
                let accent_rect = egui::Rect::from_min_size(
                    egui::pos2(start_x + h_pad, sep_y),
                    egui::vec2(100.0, 1.0),
                );
                ui.painter().rect_filled(accent_rect, 0.0, ColorPalette::BLUE_500);

                ui.add_space(36.0);
                egui::Frame::new().inner_margin(margin).show(ui, |ui| {
                    style::home_section_header(ui, "Quick Start", theme);
                    ui.add_space(12.0);

                    let mut open_new  = false;
                    let mut open_file = false;
                    ui.columns(2, |cols| {
                        if style::tool_card(
                            &mut cols[0],
                            "New Text File",
                            "Start with a blank document",
                            ColorPalette::BLUE_500,
                            theme,
                        ).clicked() { open_new = true; }

                        if style::tool_card(
                            &mut cols[1],
                            "Open File",
                            "Load an existing text or image file",
                            ColorPalette::TEAL_500,
                            theme,
                        ).clicked() { open_file = true; }
                    });
                    if open_new { action = Some(HomeAction::NewTextFile); }
                    if open_file { action = Some(HomeAction::OpenFile); }

                    ui.add_space(32.0);

                    style::home_section_header(ui, "Editors", theme);
                    ui.add_space(12.0);
                    ui.columns(3, |cols| {
                        for (i, s) in registry::SCREENS.iter().enumerate() {
                            if i < 3 && style::tool_card(&mut cols[i], s.name, s.description, s.color, theme).clicked() {
                                action = Some(HomeAction::OpenScreen(s.id));
                            }
                        }
                    });

                    ui.add_space(32.0);
                    style::home_section_header(ui, "Converters", theme);
                    ui.add_space(12.0);
                    ui.columns(3, |cols| {
                        for (i, c) in registry::CONVERTERS.iter().enumerate() {
                            if i < 3 && style::tool_card(&mut cols[i], c.name, c.description, c.color, theme).clicked() {
                                action = Some(HomeAction::OpenConverter(c.id));
                            }
                        }
                        for i in registry::CONVERTERS.len()..3 {
                            style::tool_card_placeholder(&mut cols[i], "More Coming Soon", theme);
                        }
                    });
                });
            });

        match action {
            Some(HomeAction::NewTextFile) => self.new_text_file(),
            Some(HomeAction::OpenFile) => {
                let exts = registry::all_accepted_extensions();
                if let Some(path) = rfd::FileDialog::new().add_filter("All Files", &exts).pick_file() {
                    self.open_file(path);
                }
            }
            Some(HomeAction::OpenScreen(id)) => {
                if let Some(s) = registry::SCREENS.iter().find(|s| s.id == id) {
                    let m = self.instantiate(s.create, None);
                    self.switch_to_module(m);
                }
            }
            Some(HomeAction::OpenConverter(id)) => {
                if let Some(c) = registry::CONVERTERS.iter().find(|c| c.id == id) {
                    let m = self.instantiate(c.create, None);
                    self.switch_to_module(m);
                }
            }
            Some(HomeAction::ShowSettings) => self.show_settings = true,
            Some(HomeAction::ShowPatchNotes) => self.show_patch_notes = true,
            None => {}
        }
    }

    fn render_settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_settings { return; }

        style::draw_modal_overlay(ctx, "settings_overlay", 160);

        let (bg, border, muted, text) = if matches!(self.theme_mode, ThemeMode::Dark) {
            (ColorPalette::ZINC_900, ColorPalette::ZINC_700, ColorPalette::ZINC_500, ColorPalette::SLATE_200)
        } else {
            (egui::Color32::WHITE, ColorPalette::GRAY_200, ColorPalette::GRAY_400, ColorPalette::GRAY_700)
        };

        let mut sys_clicked = false;
        let mut light_clicked = false;
        let mut dark_clicked = false;
        let mut prefs_changed = false;
        let mut open = self.show_settings;

        let response = egui::Window::new("Settings")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .min_width(400.0)
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(28.0))
            .open(&mut open)
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let tabs = [(SettingsTab::General, "General"), (SettingsTab::TextEditor, "Text Editor"), (SettingsTab::JsonEditor, "Json Editor")];
                    for (tab, label) in &tabs {
                        let selected = self.settings_tab == *tab;
                        let (fill, text_col) = if selected {
                            (if matches!(self.theme_mode, ThemeMode::Dark) { egui::Color32::from_rgb(40, 40, 50) } else { ColorPalette::GRAY_100 }, text)
                        } else {
                            (egui::Color32::TRANSPARENT, muted)
                        };
                        if ui.add(egui::Button::new(egui::RichText::new(*label).size(13.0).color(text_col)).fill(fill).corner_radius(6.0)).clicked() {
                            self.settings_tab = *tab;
                        }
                        ui.add_space(4.0);
                    }
                });
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);

                match self.settings_tab {
                    SettingsTab::General => {
                        ui.label(egui::RichText::new("APPEARANCE").size(11.0).color(muted));
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Theme").size(14.0).color(text));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                dark_clicked = ui.selectable_label(matches!(self.theme_preference, ThemePreference::Dark), "Dark").clicked();
                                light_clicked = ui.selectable_label(matches!(self.theme_preference, ThemePreference::Light), "Light").clicked();
                                sys_clicked = ui.selectable_label(matches!(self.theme_preference, ThemePreference::System), "System").clicked();
                            });
                        });
                    }
                    SettingsTab::TextEditor => {
                        ui.label(egui::RichText::new("DISPLAY").size(11.0).color(muted));
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Show Toolbar").size(14.0).color(text));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.checkbox(&mut self.show_toolbar_te, "").changed() { prefs_changed = true; }
                            });
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Show File Info").size(14.0).color(text));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.checkbox(&mut self.show_file_info_te, "").changed() { prefs_changed = true; }
                            });
                        });

                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("TYPOGRAPHY").size(11.0).color(muted));
                        ui.add_space(10.0);
                        let font_changed = false;
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Default Font").size(14.0).color(text));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.selectable_label(self.default_font == "Roboto", "Roboto").clicked() {
                                    self.default_font = "Roboto".to_string();
                                    prefs_changed = true;
                                }
                                if ui.selectable_label(self.default_font == "Ubuntu", "Ubuntu").clicked() {
                                    self.default_font = "Ubuntu".to_string();
                                    prefs_changed = true;
                                }
                            });
                        });
                        ui.add_space(6.0);
                        let mut size_changed = false;
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Default Font Size").size(14.0).color(text));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(
                                    egui::DragValue::new(&mut self.default_font_size)
                                        .range(8.0..=72.0)
                                        .speed(0.5)
                                        .suffix(" pt"),
                                ).changed() {
                                    size_changed = true;
                                }
                            });
                        });

                        if font_changed || size_changed {
                            prefs_changed = true;
                        }
                    }
                    SettingsTab::JsonEditor => {
                        ui.label(egui::RichText::new("DISPLAY").size(11.0).color(muted));
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Show File Info").size(14.0).color(text));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.checkbox(&mut self.show_file_info_je, "").changed() { prefs_changed = true; }
                            });
                        });
                    }
                }
            });

        if let Some(r) = response {
            let clicked_outside = ctx.input(|i| {
                i.pointer.any_click()
                    && i.pointer.interact_pos().map_or(false, |p| !r.response.rect.contains(p))
            });
            if clicked_outside { open = false; }
        }

        self.show_settings = open;
        if sys_clicked {
            self.theme_preference = ThemePreference::System;
            self.theme_mode = match ctx.theme() { egui::Theme::Dark => ThemeMode::Dark, egui::Theme::Light => ThemeMode::Light };
            style::apply_theme(ctx, self.theme_mode);
            self.save_settings();
        }
        if light_clicked {
            self.theme_preference = ThemePreference::Light;
            self.theme_mode = ThemeMode::Light;
            style::apply_theme(ctx, self.theme_mode);
            self.save_settings();
        }
        if dark_clicked {
            self.theme_preference = ThemePreference::Dark;
            self.theme_mode = ThemeMode::Dark;
            style::apply_theme(ctx, self.theme_mode);
            self.save_settings();
        }
        if prefs_changed { self.save_settings(); }
    }

    fn render_patch_notes_modal(&mut self, ctx: &egui::Context) {
        if !self.show_patch_notes { return; }

        style::draw_modal_overlay(ctx, "patchnotes_overlay", 160);

        let (bg, border, muted, text, tag_bg) = if matches!(self.theme_mode, ThemeMode::Dark) {
            (ColorPalette::ZINC_900, ColorPalette::ZINC_700, ColorPalette::ZINC_500, ColorPalette::SLATE_200, ColorPalette::MODAL_TAG_BG_DARK)
        } else {
            (egui::Color32::WHITE, ColorPalette::GRAY_200, ColorPalette::GRAY_900, ColorPalette::GRAY_700, ColorPalette::BLUE_50)
        };

        let mut open = self.show_patch_notes;
        let total_pages = self.patch_notes.len().max(1);
        if self.patch_notes_page >= total_pages { self.patch_notes_page = total_pages - 1; }

        let response = egui::Window::new("Patch Notes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .min_width(480.0)
            .max_width(560.0)
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(28.0))
            .open(&mut open)
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {

                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        if let Some(entry) = self.patch_notes.get(self.patch_notes_page) {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&entry.version).size(16.0).strong().color(text));
                                ui.add_space(6.0);
                                if !entry.tag.is_empty() {
                                    egui::Frame::new().fill(tag_bg).corner_radius(4.0)
                                        .inner_margin(egui::Margin { left: 6, right: 6, top: 2, bottom: 2 })
                                        .show(ui, |ui| { ui.label(egui::RichText::new(&entry.tag).size(10.0).color(muted)); });
                                }
                            });
                            ui.add_space(12.0);

                            let cat_colors = [ColorPalette::BLUE_500, ColorPalette::TEAL_500, ColorPalette::PURPLE_500];
                            for (ci, category) in entry.categories.iter().enumerate() {
                                if !category.name.is_empty() {
                                    let cat_color = cat_colors[ci % cat_colors.len()];
                                    ui.horizontal(|ui| {
                                        let rect_min = ui.cursor().min;
                                        ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(rect_min.x, rect_min.y + 2.0), egui::vec2(3.0, 14.0)), 1.5, cat_color);
                                        ui.add_space(8.0);
                                        ui.label(egui::RichText::new(&category.name).size(12.0).strong().color(muted));
                                    });
                                    ui.add_space(6.0);
                                }
                                for note in &category.notes {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.add_space(14.0);
                                        ui.painter().circle_filled(egui::pos2(ui.cursor().min.x + 3.0, ui.cursor().min.y + 8.0), 2.0, ColorPalette::BLUE_500);
                                        ui.add_space(10.0);
                                        if !note.module_tag.is_empty() {
                                            let (chip_bg, chip_text) = if matches!(self.theme_mode, ThemeMode::Dark) {
                                                (ColorPalette::CHIP_BG_DARK, ColorPalette::BLUE_400)
                                            } else {
                                                (ColorPalette::BLUE_50, ColorPalette::BLUE_600)
                                            };
                                            egui::Frame::new().fill(chip_bg).corner_radius(3.0)
                                                .inner_margin(egui::Margin { left: 5, right: 5, top: 1, bottom: 1 })
                                                .show(ui, |ui| { ui.label(egui::RichText::new(&note.module_tag).size(11.0).color(chip_text)); });
                                            ui.add_space(4.0);
                                        }
                                        ui.label(egui::RichText::new(&note.text).size(13.0).color(text));
                                    });
                                    ui.add_space(4.0);
                                }
                                if ci < entry.categories.len() - 1 { ui.add_space(10.0); }
                            }
                            ui.add_space(20.0);
                        } else {
                            ui.label("No patch notes available.");
                        }
                    });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.add_enabled(self.patch_notes_page > 0, egui::Button::new("< Prev")).clicked() {
                        self.patch_notes_page -= 1;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(self.patch_notes_page < total_pages - 1, egui::Button::new("Next >")).clicked() {
                            self.patch_notes_page += 1;
                        }
                        ui.label(egui::RichText::new(format!("Page {} of {}", self.patch_notes_page + 1, total_pages)).color(muted));
                    });
                });
            });

        if let Some(r) = response {
            let clicked_outside = ctx.input(|i| {
                i.pointer.any_click()
                    && i.pointer.interact_pos().map_or(false, |p| !r.response.rect.contains(p))
            });
            if clicked_outside { open = false; }
        }

        self.show_patch_notes = open;
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

        while let Ok((old_path, new_path)) = self.path_replace_rx.try_recv() {
            self.recent_files.remove_file(&old_path);
            self.recent_files.add_file(new_path);
        }

        if let Some(PendingAction::Exit) = &self.pending_action {
            if !self.show_unsaved_dialog {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        self.render_unsaved_dialog(ctx);
        self.render_settings_modal(ctx);
        self.render_patch_notes_modal(ctx);
        self.top_bar(ctx);
        self.sidebar(ctx);

        let show_fi = if self.is_in_json_editor() { self.show_file_info_je } else { self.show_file_info_te };
        let show_toolbar = self.show_toolbar_te;
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(module) = &mut self.active_module {
                module.ui(ui, ctx, show_toolbar, show_fi);
            } else {
                self.landing_page(ui);
            }
        });

        if self.show_unsaved_dialog {
            ctx.set_cursor_icon(egui::CursorIcon::Default);
        }
    }
}
