use eframe::egui;
use crate::style::ColorPalette;
use super::style::{self, ThemeMode};
use super::modules::{EditorModule, text_edit::TextEditor, image_converter::ImageConverter, image_edit::ImageEditor, json_edit::JsonEditor, data_converter::DataConverter, archive_converter::ArchiveConverter};
use crate::modules::image_editor::ie_cache;
use crate::modules::doc_edit::DocumentEditor;
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
    fn new() -> Self { Self { files: Vec::new() } }

    fn load() -> Self {
        let p = Self::get_config_path();
        if let Ok(s) = fs::read_to_string(&p) { if let Ok(r) = serde_json::from_str(&s) { return r; } }
        Self::new()
    }

    fn save(&self) {
        let p = Self::get_config_path();
        if let Some(parent) = p.parent() { let _ = fs::create_dir_all(parent); }
        if let Ok(json) = serde_json::to_string_pretty(self) { let _ = fs::write(p, json); }
    }

    fn get_config_path() -> PathBuf {
        let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("universal_editor"); p.push("recent_files.json"); p
    }

    fn add_file(&mut self, path: PathBuf) {
        self.files.retain(|f| f.path != path);
        self.files.insert(0, RecentFile { path, timestamp: chrono::Utc::now().timestamp() });
        if self.files.len() > 20 { self.files.truncate(20); }
        self.save();
    }

    fn get_files(&self) -> &[RecentFile] { &self.files }

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
    #[serde(default = "default_font_name")] default_font: String,
    #[serde(default = "default_font_size")] default_font_size: f32,
    show_file_info_je: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme_preference: ThemePreference::System,
            show_toolbar_te: true, show_file_info_te: true,
            default_font: default_font_name(), default_font_size: default_font_size(),
            show_file_info_je: true,
        }
    }
}

impl AppSettings {
    fn load() -> Self {
        let p = Self::get_config_path();
        if let Ok(s) = fs::read_to_string(&p) { if let Ok(settings) = serde_json::from_str(&s) { return settings; } }
        Self::default()
    }

    fn save(&self) {
        let p = Self::get_config_path();
        if let Some(parent) = p.parent() { let _ = fs::create_dir_all(parent); }
        if let Ok(json) = serde_json::to_string_pretty(self) { let _ = fs::write(p, json); }
    }

    fn get_config_path() -> PathBuf {
        let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("universal_editor"); p.push("app_settings.json"); p
    }
}

enum PendingAction { OpenFile(PathBuf), NewFile, SwitchModule(Box<dyn EditorModule>), GoHome, Exit }

#[derive(PartialEq)]
enum HomeAction { NewTextFile, OpenFile, OpenScreen(&'static str), OpenConverter(&'static str), ShowSettings, ShowPatchNotes, ShowAbout }

struct PatchNote { module_tag: String, text: String }
struct PatchCategory { name: String, notes: Vec<PatchNote> }
struct PatchVersion { version: String, tag: String, categories: Vec<PatchCategory> }

#[derive(PartialEq, Clone, Copy)]
enum SettingsTab { General, TextEditor, JsonEditor, Cache }

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
    show_about: bool,
    settings_tab: SettingsTab,
    pending_action: Option<PendingAction>,
    recent_file_tx: SyncSender<PathBuf>,
    recent_file_rx: Receiver<PathBuf>,
    path_replace_tx: SyncSender<(PathBuf, PathBuf)>,
    path_replace_rx: Receiver<(PathBuf, PathBuf)>,
    patch_notes: Vec<PatchVersion>,
    patch_notes_page: usize,
    rename_target: Option<PathBuf>,
    rename_buffer: String,
    cache_entries: Option<Vec<ie_cache::CacheEntry>>,
    open_cache_path: Option<PathBuf>,
}

fn open_file_location(path: &PathBuf) {
    if let Some(_dir) = path.parent() {
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").args(["-R", &path.to_string_lossy()]).spawn();
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let _ = std::process::Command::new("xdg-open").arg(_dir).spawn();
    }
}

impl UniversalEditor {
    pub fn new(cc: &eframe::CreationContext<'_>, startup_file: Option<PathBuf>) -> Self {
        let settings = AppSettings::load();
        let system_theme = match cc.egui_ctx.theme() { egui::Theme::Dark => ThemeMode::Dark, egui::Theme::Light => ThemeMode::Light };
        let initial_theme = match settings.theme_preference {
            ThemePreference::System => system_theme, ThemePreference::Light => ThemeMode::Light, ThemePreference::Dark => ThemeMode::Dark,
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
                    return PatchNote { module_tag: raw[2..end + 2].to_string(), text: raw[end + 4..].trim_start_matches(':').trim().to_string() };
                }
            }
            PatchNote { module_tag: String::new(), text: raw.to_string() }
        }

        for line in patch_content.lines() {
            let line = line.trim();
            if line.starts_with("## V") {
                if let Some(v) = current_version.take() { patch_notes.push(v); }
                current_version = Some(PatchVersion { version: line.trim_start_matches("## ").trim().to_string(), tag: String::new(), categories: Vec::new() });
                current_category = None;
            } else if line.starts_with("#### ") {
                if let Some(v) = &mut current_version {
                    v.categories.push(PatchCategory { name: line.trim_start_matches("#### ").trim().to_string(), notes: Vec::new() });
                    current_category = Some(v.categories.len() - 1);
                }
            } else if line.starts_with("- ") || line.starts_with("* ") {
                if let Some(v) = &mut current_version {
                    let note = parse_note(&line[2..]);
                    if let Some(idx) = current_category {
                        v.categories[idx].notes.push(note);
                    } else {
                        if v.categories.is_empty() { v.categories.push(PatchCategory { name: String::new(), notes: Vec::new() }); }
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
            v.tag = if i == 0 { "Current" } else if i == total - 1 { "Initial Release" } else { "Update" }.to_string();
        }

        let mut recent_files = RecentFiles::load();
        let active_module = startup_file.map(|path| {
            recent_files.add_file(path.clone());
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let create = registry::screen_for_extension(ext).map(|s| s.create).unwrap_or(CreateModule::TextEditor);
            let m: Box<dyn EditorModule> = match create {
                CreateModule::TextEditor => {
                    let mut e = TextEditor::load(path);
                    e.set_default_font(egui::FontFamily::Name(settings.default_font.clone().into()), settings.default_font_size);
                    e.set_path_replace_tx(replace_tx.clone());
                    Box::new(e)
                }
                CreateModule::ImageEditor => {
                    let mut e = ImageEditor::load(path);
                    let tx = tx.clone();
                    e.set_file_callback(Box::new(move |p: PathBuf| { let _ = tx.send(p); }));
                    Box::new(e)
                }
                CreateModule::JsonEditor => Box::new(JsonEditor::load(path)),
                _ => Box::new(TextEditor::load(path)),
            };
            m
        });

        Self {
            active_module, sidebar_open: true, theme_mode: initial_theme,
            theme_preference: settings.theme_preference, recent_files,
            screens_expanded: false, converters_expanded: false, recent_files_expanded: false,
            show_toolbar_te: settings.show_toolbar_te, show_file_info_te: settings.show_file_info_te,
            show_file_info_je: settings.show_file_info_je,
            default_font: settings.default_font, default_font_size: settings.default_font_size,
            show_unsaved_dialog: false, show_patch_notes: false, show_settings: false, show_about: false,
            settings_tab: SettingsTab::General, pending_action: None,
            recent_file_tx: tx, recent_file_rx: rx,
            path_replace_tx: replace_tx, path_replace_rx: replace_rx,
            patch_notes, patch_notes_page: 0, rename_target: None, rename_buffer: String::new(),
            cache_entries: None, open_cache_path: None,
        }
    }

    fn is_in_text_editor(&self) -> bool {
        self.active_module.as_ref().map_or(false, |m| m.as_any().downcast_ref::<TextEditor>().is_some())
    }

    fn is_in_json_editor(&self) -> bool {
        self.active_module.as_ref().map_or(false, |m| m.as_any().downcast_ref::<JsonEditor>().is_some())
    }

    fn has_unsaved_changes(&self) -> bool {
        if let Some(m) = &self.active_module {
            if let Some(e) = m.as_any().downcast_ref::<TextEditor>() { return e.is_dirty(); }
            if let Some(e) = m.as_any().downcast_ref::<ImageEditor>() { return e.is_dirty(); }
            if let Some(e) = m.as_any().downcast_ref::<JsonEditor>() { return e.is_dirty() || e.is_text_modified(); }
            if let Some(e) = m.as_any().downcast_ref::<DocumentEditor>() { return e.is_dirty(); }
        }
        false
    }

    fn apply_default_font(&self, editor: &mut TextEditor) {
        editor.set_default_font(egui::FontFamily::Name(self.default_font.clone().into()), self.default_font_size);
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
                let mut e = if let Some(ref p) = path { ImageEditor::load(p.clone()) } else { ImageEditor::new() };
                if let Some(ref p) = path {
                    if let Some(cache) = ie_cache::load_cache(p) { ie_cache::apply_cache(&mut e, cache); }
                }
                let tx = self.recent_file_tx.clone();
                e.set_file_callback(Box::new(move |p: PathBuf| { let _ = tx.send(p); }));
                Box::new(e)
            }
            CreateModule::JsonEditor => Box::new(if let Some(p) = path { JsonEditor::load(p) } else { JsonEditor::new_empty() }),
            CreateModule::DocEditor => { Box::new(if let Some(p) = path { DocumentEditor::load(p) } else { DocumentEditor::new_empty() }) }
            CreateModule::ImageConverter => Box::new(ImageConverter::new()),
            CreateModule::DataConverter => Box::new(DataConverter::new()),
            CreateModule::ArchiveConverter => Box::new(ArchiveConverter::new()),
        }
    }

    fn module_from_path(&self, path: PathBuf) -> Box<dyn EditorModule> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let create = registry::screen_for_extension(ext).map(|s| s.create).unwrap_or(CreateModule::TextEditor);
        self.instantiate(create, Some(path))
    }

    fn open_file(&mut self, path: PathBuf) {
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::OpenFile(path)); self.show_unsaved_dialog = true;
        } else {
            self.save_image_cache_if_needed();
            self.recent_files.add_file(path.clone()); self.active_module = Some(self.module_from_path(path));
        }
    }

    fn new_text_file(&mut self) {
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::NewFile); self.show_unsaved_dialog = true;
        } else {
            let mut editor = TextEditor::new_empty(); self.apply_default_font(&mut editor); self.active_module = Some(Box::new(editor));
        }
    }

    fn switch_to_module(&mut self, module: Box<dyn EditorModule>) {
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::SwitchModule(module)); self.show_unsaved_dialog = true;
        } else {
            self.save_image_cache_if_needed();
            self.active_module = Some(module);
        }
    }

    fn go_home(&mut self) {
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::GoHome); self.show_unsaved_dialog = true;
        } else {
            self.save_image_cache_if_needed();
            self.active_module = None;
        }
    }

    fn execute_pending_action(&mut self) {
        if let Some(action) = self.pending_action.take() {
            self.save_image_cache_if_needed();
            match action {
                PendingAction::OpenFile(path) => { self.recent_files.add_file(path.clone()); self.active_module = Some(self.module_from_path(path)); }
                PendingAction::NewFile => { let mut e = TextEditor::new_empty(); self.apply_default_font(&mut e); self.active_module = Some(Box::new(e)); }
                PendingAction::SwitchModule(module) => { self.active_module = Some(module); }
                PendingAction::GoHome => { self.active_module = None; }
                PendingAction::Exit => {}
            }
        }
    }

    fn save_image_cache_if_needed(&self) {
        if let Some(m) = &self.active_module {
            if let Some(e) = m.as_any().downcast_ref::<ImageEditor>() {
                if e.file_path.is_some() && e.layers.len() > 1 { let _ = ie_cache::save_cache(e); }
            }
        }
    }

    fn save_settings(&self) {
        AppSettings {
            theme_preference: self.theme_preference, show_toolbar_te: self.show_toolbar_te,
            show_file_info_te: self.show_file_info_te, default_font: self.default_font.clone(),
            default_font_size: self.default_font_size, show_file_info_je: self.show_file_info_je,
        }.save();
    }

    fn render_unsaved_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_unsaved_dialog { return; }
        let is_dark = matches!(self.theme_mode, ThemeMode::Dark);
        let (bg, border, text) = if is_dark { (ColorPalette::ZINC_800, ColorPalette::ZINC_700, ColorPalette::ZINC_100) } else { (egui::Color32::WHITE, ColorPalette::GRAY_300, ColorPalette::GRAY_900) };
        let sub = if is_dark { ColorPalette::ZINC_400 } else { ColorPalette::GRAY_600 };
        style::draw_modal_overlay(ctx, "unsaved_overlay", 200);
        egui::Window::new("Unsaved Changes")
            .collapsible(false).resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Tooltip)
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(8.0).inner_margin(24.0))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Do you want to save changes?").size(16.0).color(text)); ui.add_space(8.0);
                    ui.label(egui::RichText::new("Your changes will be lost if you don't save them.").size(13.0).color(sub)); ui.add_space(24.0);
                    ui.horizontal(|ui| {
                        let save = style::primary_button(ui, "Save").on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
                        let dont = style::secondary_button(ui, "Don't Save", self.theme_mode).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
                        let cancel = style::secondary_button(ui, "Cancel", self.theme_mode).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
                        if save { if let Some(m) = &mut self.active_module { let _ = m.save(); } self.show_unsaved_dialog = false; self.execute_pending_action(); }
                        if dont { self.show_unsaved_dialog = false; self.execute_pending_action(); }
                        if cancel { self.show_unsaved_dialog = false; self.pending_action = None; }
                    });
                    ui.add_space(8.0);
                });
            });
    }

    fn menu_items_ui(&mut self, ui: &mut egui::Ui, items: &[(crate::modules::MenuItem, crate::modules::MenuAction)]) {
        for (item, action) in items {
            if item.label == "Separator" { ui.separator(); continue; }
            let label = item.shortcut.as_ref().map(|s| format!("{} ({})", item.label, s)).unwrap_or_else(|| item.label.clone());
            if ui.add_enabled(item.enabled, egui::Button::new(label)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                if let Some(m) = &mut self.active_module { m.handle_menu_action(action.clone()); }
                ui.close();
            }
        }
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        let contributions = self.active_module.as_ref().map(|m| m.get_menu_contributions()).unwrap_or_default();
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            egui::MenuBar::new().ui(ui, |ui| {
                let has_module = self.active_module.is_some();
                let mut go_home = false;
                if has_module { if ui.button("Home").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { go_home = true; } ui.separator(); }
                if go_home { self.go_home(); return; }

                ui.menu_button("File", |ui| {
                    if ui.button("Open...").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        let exts = registry::all_accepted_extensions();
                        if let Some(path) = rfd::FileDialog::new().add_filter("All Files", &exts).pick_file() { self.open_file(path); }
                        ui.close();
                    }
                    ui.separator();
                    if ui.add_enabled(has_module, egui::Button::new("Save (Ctrl+S)")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        if let Some(m) = &mut self.active_module { let _ = m.save(); } ui.close();
                    }
                    if ui.add_enabled(has_module, egui::Button::new("Save As...")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        if let Some(m) = &mut self.active_module { let _ = m.save_as(); } ui.close();
                    }
                    if !contributions.file_items.is_empty() { ui.separator(); self.menu_items_ui(ui, &contributions.file_items.clone()); }
                    ui.separator();
                    if ui.button("Exit").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        if self.has_unsaved_changes() { self.pending_action = Some(PendingAction::Exit); self.show_unsaved_dialog = true; }
                        else { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                        ui.close();
                    }
                });

                if !contributions.edit_items.is_empty() {
                    let items = contributions.edit_items.clone();
                    ui.menu_button("Edit", |ui| { self.menu_items_ui(ui, &items); });
                }

                ui.menu_button("View", |ui| {
                    if ui.button("Toggle Sidebar (Ctrl+\\)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.sidebar_open = !self.sidebar_open; ui.close(); }
                    if self.is_in_text_editor() || self.is_in_json_editor() {
                        ui.separator();
                        if self.is_in_text_editor() {
                            let a = ui.checkbox(&mut self.show_toolbar_te, "Show Toolbar").changed();
                            let b = ui.checkbox(&mut self.show_file_info_te, "Show File Info").changed();
                            if a || b { self.save_settings(); }
                        }
                        if self.is_in_json_editor() {
                            if ui.checkbox(&mut self.show_file_info_je, "Show File Info").changed() { 
                                self.save_settings(); 
                            }
                        }
                    }
                    if !contributions.view_items.is_empty() { ui.separator(); self.menu_items_ui(ui, &contributions.view_items.clone()); }

                    ui.separator(); ui.label("Theme:");
                    let sys = ui.selectable_label(matches!(self.theme_preference, ThemePreference::System), "System").on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
                    let light = ui.selectable_label(matches!(self.theme_preference, ThemePreference::Light), "Light").on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
                    let dark = ui.selectable_label(matches!(self.theme_preference, ThemePreference::Dark), "Dark").on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
                    if sys { self.theme_preference = ThemePreference::System; self.theme_mode = match ctx.theme() { egui::Theme::Dark => ThemeMode::Dark, egui::Theme::Light => ThemeMode::Light }; style::apply_theme(ctx, self.theme_mode); self.save_settings(); ui.close(); }
                    if light { self.theme_preference = ThemePreference::Light; self.theme_mode = ThemeMode::Light; style::apply_theme(ctx, self.theme_mode); self.save_settings(); ui.close(); }
                    if dark { self.theme_preference = ThemePreference::Dark; self.theme_mode = ThemeMode::Dark; style::apply_theme(ctx, self.theme_mode); self.save_settings(); ui.close(); }
                });

                if !contributions.image_items.is_empty() { let items = contributions.image_items.clone(); ui.menu_button("Image", |ui| { self.menu_items_ui(ui, &items); }); }
                if !contributions.filter_items.is_empty() { let items = contributions.filter_items.clone(); ui.menu_button("Filter", |ui| { self.menu_items_ui(ui, &items); }); }
                if !contributions.layer_items.is_empty() { let items = contributions.layer_items.clone(); ui.menu_button("Layer", |ui| { self.menu_items_ui(ui, &items); }); }
            });
            ui.add_space(4.0);
        });
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        if !self.sidebar_open { return; }
        egui::SidePanel::left("sidebar").resizable(true).default_width(240.0).min_width(200.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(8.0);
                let mut open_screen: Option<&'static str> = None;
                let mut open_converter: Option<&'static str> = None;
                let theme_mode = self.theme_mode;

                style::sidebar_section(ui, "Screens", &mut self.screens_expanded, theme_mode, |ui| {
                    for s in registry::SCREENS {
                        if style::sidebar_item(ui, s.name, s.sidebar_letter, theme_mode).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { open_screen = Some(s.id); }
                    }
                });
                if let Some(id) = open_screen {
                    if let Some(s) = registry::SCREENS.iter().find(|s| s.id == id) { let m = self.instantiate(s.create, None); self.switch_to_module(m); }
                }

                style::sidebar_section(ui, "Converters", &mut self.converters_expanded, theme_mode, |ui| {
                    for c in registry::CONVERTERS {
                        if style::sidebar_item(ui, c.name, c.sidebar_letter, theme_mode).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { open_converter = Some(c.id); }
                    }
                });
                if let Some(id) = open_converter {
                    if let Some(c) = registry::CONVERTERS.iter().find(|c| c.id == id) { let m = self.instantiate(c.create, None); self.switch_to_module(m); }
                }

                let recent_files: Vec<RecentFile> = self.recent_files.get_files().to_vec();
                let mut file_to_open: Option<PathBuf> = None;
                let mut file_to_remove: Option<PathBuf> = None;
                let mut rename_init: Option<(PathBuf, String)> = None;
                let mut location_to_open: Option<PathBuf> = None;
                let mut name_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
                for rf in &recent_files {
                    if rf.path.exists() {
                        let name = rf.path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string();
                        *name_counts.entry(name).or_insert(0) += 1;
                    }
                }
                let (normal_text, muted_text) = match theme_mode {
                    ThemeMode::Dark => (ColorPalette::SLATE_200, ColorPalette::ZINC_500),
                    ThemeMode::Light => (ColorPalette::GRAY_800, ColorPalette::GRAY_400),
                };

                style::sidebar_section(ui, "Recent Files", &mut self.recent_files_expanded, theme_mode, |ui| {
                    if recent_files.is_empty() {
                        ui.centered_and_justified(|ui| { ui.weak("No recent files"); });
                    } else {
                        for rf in &recent_files {
                            if !rf.path.exists() { continue; }
                            let file_name = rf.path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown");
                            let is_duplicate = name_counts.get(file_name).copied().unwrap_or(0) > 1;
                            let mut add_context_menu = |resp: egui::Response, path: &PathBuf, name: &str| {
                                resp.context_menu(|ui| {
                                    if ui.button("Rename").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { rename_init = Some((path.clone(), name.to_string())); ui.close(); }
                                    if ui.button("Open File Location").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { location_to_open = Some(path.clone()); ui.close(); }
                                    ui.separator();
                                    if ui.button("Remove from List").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { file_to_remove = Some(path.clone()); ui.close(); }
                                });
                            };
                            if is_duplicate {
                                let parent_dir = rf.path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("...");
                                let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width() - 8.0, 42.0), egui::Sense::click());
                                let painter = ui.painter_at(rect);
                                let hover_bg = if response.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    match theme_mode {
                                        ThemeMode::Dark => egui::Color32::from_rgb(40, 40, 48),
                                        ThemeMode::Light => ColorPalette::GRAY_200,
                                    }
                                } else { egui::Color32::TRANSPARENT };

                                painter.rect_filled(rect, 4.0, hover_bg);
                                painter.text(rect.left_center() + egui::vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, "F",
                                    egui::FontId::proportional(14.0), normal_text);
                                painter.text(egui::pos2(rect.left() + 32.0, rect.top() + 10.0),egui::Align2::LEFT_TOP, file_name,
                                    egui::FontId::proportional(12.5), normal_text);
                                painter.text(egui::pos2(rect.left() + 32.0, rect.top() + 26.0),egui::Align2::LEFT_TOP,format!("…/{}", parent_dir),
                                    egui::FontId::proportional(10.5),muted_text);

                                if response.clicked() { file_to_open = Some(rf.path.clone()); }
                                add_context_menu(response, &rf.path, file_name);
                            } else {
                                let resp = style::sidebar_item(ui, file_name, "F", theme_mode).on_hover_cursor(egui::CursorIcon::PointingHand);
                                if resp.clicked() { file_to_open = Some(rf.path.clone()); }
                                add_context_menu(resp, &rf.path, file_name);
                            }
                        }
                    }
                });

                if let Some(path) = file_to_remove { self.recent_files.remove_file(&path); }
                if let Some(path) = file_to_open { self.open_file(path); }
                if let Some(path) = location_to_open { open_file_location(&path); }
                if let Some((path, name)) = rename_init { self.rename_target = Some(path); self.rename_buffer = name; }
                ui.add_space(8.0);
            });
        });
    }

    fn rename_modal(&mut self, ctx: &egui::Context) {
        let Some(target) = self.rename_target.clone() else { return };
        let theme = self.theme_mode;
        let (bg, border, text, subtext, btn_bg, btn_hover) = match theme {
            ThemeMode::Dark => (ColorPalette::ZINC_900, ColorPalette::ZINC_700, egui::Color32::WHITE, ColorPalette::ZINC_400, ColorPalette::BLUE_700, ColorPalette::BLUE_600),
            ThemeMode::Light => (egui::Color32::WHITE, ColorPalette::GRAY_200, ColorPalette::GRAY_900, ColorPalette::GRAY_500, ColorPalette::BLUE_600, ColorPalette::BLUE_500),
        };
        style::draw_modal_overlay(ctx, "rename_overlay", 120);
        let mut open = true;
        egui::Window::new("rename_modal_win")
            .title_bar(false).resizable(false).collapsible(false)
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .fixed_size(egui::vec2(320.0, 0.0))
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(egui::Margin::same(20)))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Rename File").size(15.0).color(text));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(target.to_string_lossy().as_ref()).size(11.0).color(subtext));
                ui.add_space(12.0);
                let resp = ui.add(egui::TextEdit::singleline(&mut self.rename_buffer).desired_width(f32::INFINITY).font(egui::FontId::proportional(14.0)));
                resp.request_focus();
                ui.add_space(12.0);
                let confirmed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                let cancelled = ctx.input(|i| i.key_pressed(egui::Key::Escape));
                ui.horizontal(|ui| {
                    let btn_style = |ui: &mut egui::Ui, fill: egui::Color32, hover: egui::Color32| {
                        let s = ui.style_mut();
                        s.visuals.widgets.inactive.bg_fill = fill; s.visuals.widgets.inactive.weak_bg_fill = fill;
                        s.visuals.widgets.hovered.bg_fill = hover; s.visuals.widgets.hovered.weak_bg_fill = hover;
                        s.visuals.override_text_color = Some(egui::Color32::WHITE);
                    };
                    let confirm = ui.scope(|ui| { btn_style(ui, btn_bg, btn_hover); ui.button("Rename") }).inner.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() || confirmed;
                    ui.add_space(8.0);
                    let cancel = ui.scope(|ui| {
                        let (cb, ch) = match theme { ThemeMode::Dark => (ColorPalette::ZINC_700, ColorPalette::ZINC_600), ThemeMode::Light => (ColorPalette::GRAY_200, ColorPalette::GRAY_300) };
                        btn_style(ui, cb, ch); ui.style_mut().visuals.override_text_color = Some(text); ui.button("Cancel")
                    }).inner.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() || cancelled;
                    if confirm && !self.rename_buffer.trim().is_empty() {
                        let new_name = self.rename_buffer.trim().to_string();
                        if let Some(parent) = target.parent() {
                            let new_path = parent.join(&new_name);
                            if std::fs::rename(&target, &new_path).is_ok() { self.recent_files.remove_file(&target); self.recent_files.add_file(new_path.clone()); }
                        }
                        self.rename_target = None;
                    }
                    if cancel { self.rename_target = None; }
                });
            });
        if !open { self.rename_target = None; }
    }

    fn landing_page(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme_mode;
        let (title_col, sub_col, accent_line, ver_bg, ver_text_col) = match theme {
            ThemeMode::Dark => (egui::Color32::WHITE, ColorPalette::ZINC_400, ColorPalette::ZINC_800, egui::Color32::from_rgb(32, 32, 40), ColorPalette::ZINC_400),
            ThemeMode::Light => (ColorPalette::GRAY_900, ColorPalette::GRAY_500, ColorPalette::GRAY_200, ColorPalette::GRAY_100, ColorPalette::GRAY_500),
        };
        let mut action: Option<HomeAction> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let avail_w = ui.available_width();
            let h_pad = 48.0_f32.max((avail_w - 960.0) / 2.0);
            let margin = egui::Margin { left: h_pad as i8, right: h_pad as i8, ..Default::default() };
            ui.add_space(36.0);

            egui::Frame::new().inner_margin(margin).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Universal Editor").size(38.0).strong().color(title_col));
                            ui.add_space(10.0);
                            egui::Frame::new().fill(ver_bg).corner_radius(10.0).inner_margin(egui::Margin { left: 8, right: 8, top: 3, bottom: 3 })
                                .show(ui, |ui| { ui.label(egui::RichText::new("v".to_owned() + env!("CARGO_PKG_VERSION")).size(11.0).color(ver_text_col)); });
                        });
                        ui.label(egui::RichText::new("A modular suite for text and media").size(14.0).color(sub_col));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if style::ghost_button(ui, "About", false, theme).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { action = Some(HomeAction::ShowAbout); }
                        ui.add_space(4.0);
                        if style::ghost_button(ui, "Patch Notes", false, theme).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { action = Some(HomeAction::ShowPatchNotes); }
                        ui.add_space(4.0);
                        if style::ghost_button(ui, "Settings", false, theme).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { action = Some(HomeAction::ShowSettings); }
                    });
                });
            });

            ui.add_space(20.0);
            let (sx, sy) = (ui.cursor().min.x, ui.cursor().min.y);
            let sep_rect = egui::Rect::from_min_size(egui::pos2(sx, sy), egui::vec2(avail_w, 1.0));
            ui.allocate_rect(sep_rect, egui::Sense::hover());
            ui.painter().rect_filled(sep_rect, 0.0, accent_line);
            ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(sx + h_pad, sy), egui::vec2(100.0, 1.0)), 0.0, ColorPalette::BLUE_500);

            ui.add_space(36.0);
            egui::Frame::new().inner_margin(margin).show(ui, |ui| {
                style::home_section_header(ui, "Quick Start", theme);
                ui.add_space(12.0);
                let mut open_new = false; let open_file = false;
                ui.columns(2, |cols| {
                    if style::tool_card(&mut cols[0], "New Text File", "Start with a blank document", ColorPalette::BLUE_500, theme).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { open_new = true; }
                    if style::tool_card(&mut cols[1], "Open File", "Load an existing text or image file", ColorPalette::TEAL_500, theme).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { action = Some(HomeAction::OpenFile); }
                });
                if open_new { action = Some(HomeAction::NewTextFile); }
                if open_file { action = Some(HomeAction::OpenFile); }

                ui.add_space(32.0);
                style::home_section_header(ui, "Editors", theme);
                ui.add_space(12.0);
                {
                    let screens = registry::SCREENS;
                    let rows = (screens.len() + 2) / 3;
                    for r in 0..rows {
                        ui.columns(3, |cols| {
                            for c in 0..3usize {
                                let idx = r * 3 + c;
                                if let Some(s) = screens.get(idx) {
                                    if style::tool_card(&mut cols[c], s.name, s.description, s.color, theme).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { action = Some(HomeAction::OpenScreen(s.id)); }
                                } else {
                                    style::tool_card_placeholder(&mut cols[c], "More Coming Soon", theme);
                                }
                            }
                        });
                    }
                }
                ui.add_space(32.0);
                style::home_section_header(ui, "Converters", theme);
                ui.add_space(12.0);
                {
                    let converters = registry::CONVERTERS;
                    let rows = (converters.len() + 2) / 3;
                    for r in 0..rows {
                        ui.columns(3, |cols| {
                            for c in 0..3usize {
                                let idx = r * 3 + c;
                                if let Some(cv) = converters.get(idx) {
                                    if style::tool_card(&mut cols[c], cv.name, cv.description, cv.color, theme).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { action = Some(HomeAction::OpenConverter(cv.id)); }
                                } else {
                                    style::tool_card_placeholder(&mut cols[c], "More Coming Soon", theme);
                                }
                            }
                        });
                    }
                }
            });
        });

        match action {
            Some(HomeAction::NewTextFile) => self.new_text_file(),
            Some(HomeAction::OpenFile) => {
                let exts = registry::all_accepted_extensions();
                if let Some(path) = rfd::FileDialog::new().add_filter("All Files", &exts).pick_file() { self.open_file(path); }
            }
            Some(HomeAction::OpenScreen(id)) => {
                if let Some(s) = registry::SCREENS.iter().find(|s| s.id == id) { let m = self.instantiate(s.create, None); self.switch_to_module(m); }
            }
            Some(HomeAction::OpenConverter(id)) => {
                if let Some(c) = registry::CONVERTERS.iter().find(|c| c.id == id) { let m = self.instantiate(c.create, None); self.switch_to_module(m); }
            }
            Some(HomeAction::ShowSettings) => self.show_settings = true,
            Some(HomeAction::ShowPatchNotes) => self.show_patch_notes = true,
            Some(HomeAction::ShowAbout) => self.show_about = true,
            None => {}
        }
    }

    fn render_settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_settings { return; }
        let theme = self.theme_mode;
        let is_dark = matches!(theme, ThemeMode::Dark);
        let (muted, text) = if is_dark { (ColorPalette::ZINC_500, ColorPalette::SLATE_200) } else { (ColorPalette::GRAY_400, ColorPalette::GRAY_700) };

        if self.cache_entries.is_none() {
            let mut entries = ie_cache::list_caches();
            for e in entries.iter() {
                if !std::path::Path::new(&e.src_path).exists() { let _ = std::fs::remove_dir_all(&e.cache_dir); }
            }
            entries.retain(|e| std::path::Path::new(&e.src_path).exists());
            self.cache_entries = Some(entries);
        }

        let mut hdr_close = false;
        let mut sys_c = false; let mut light_c = false; let mut dark_c = false;
        let mut prefs_changed = false;
        let mut to_delete: Option<usize> = None;

        let outside = style::main_menu_modal(ctx, "settings_mw", theme, 440.0, |ui| {
            if style::main_menu_modal_header(ui, "Settings", "", theme) { hdr_close = true; }
            egui::Frame::new().inner_margin(egui::Margin { left: 24, right: 24, top: 10, bottom: 4 }).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (tab, label) in &[(SettingsTab::General, "General"), (SettingsTab::TextEditor, "Text Editor"), (SettingsTab::Cache, "Image Editor"), (SettingsTab::JsonEditor, "JSON Editor")] {
                        let sel = self.settings_tab == *tab;
                        let (fill, tc) = if sel { (if is_dark { egui::Color32::from_rgb(40, 40, 50) } else { ColorPalette::GRAY_100 }, text) } else { (egui::Color32::TRANSPARENT, muted) };
                        if ui.add(egui::Button::new(egui::RichText::new(*label).size(12.0).color(tc)).fill(fill).corner_radius(6.0)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.settings_tab = *tab; }
                        ui.add_space(4.0);
                    }
                });
            });
            let (div, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter().rect_filled(div, 0.0, if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_200 });
            egui::ScrollArea::vertical().max_height(450.0).auto_shrink([false, true]).show(ui, |ui| {
                egui::Frame::new().inner_margin(egui::Margin { left: 28, right: 28, top: 16, bottom: 20 }).show(ui, |ui| {
                    match self.settings_tab {
                        SettingsTab::General => {
                            ui.label(egui::RichText::new("APPEARANCE").size(11.0).color(muted));
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Theme").size(14.0).color(text));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    dark_c = ui.selectable_label(matches!(self.theme_preference, ThemePreference::Dark), "Dark").on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
                                    light_c = ui.selectable_label(matches!(self.theme_preference, ThemePreference::Light), "Light").on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
                                    sys_c = ui.selectable_label(matches!(self.theme_preference, ThemePreference::System), "System").on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
                                });
                            });
                        }
                        SettingsTab::TextEditor => {
                            ui.label(egui::RichText::new("DISPLAY").size(11.0).color(muted));
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Show Toolbar").size(14.0).color(text));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.checkbox(&mut self.show_toolbar_te, "").changed() { prefs_changed = true; } });
                            });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Show File Info").size(14.0).color(text));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.checkbox(&mut self.show_file_info_te, "").changed() { prefs_changed = true; } });
                            });
                            ui.add_space(16.0);
                            ui.label(egui::RichText::new("TYPOGRAPHY").size(11.0).color(muted));
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Default Font:").size(14.0).color(text));
                            });
                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                                for (name, label) in &[("Ubuntu", "Ubuntu"), ("Roboto", "Roboto"), ("GoogleSans", "Google Sans"), ("OpenSans", "Open Sans")] {
                                    let sel = self.default_font == *name;
                                    let (fill, tc) = if sel { (if is_dark { egui::Color32::from_rgb(28,52,100) } else { ColorPalette::BLUE_100 }, if is_dark { egui::Color32::WHITE } else { ColorPalette::BLUE_700 }) }
                                        else { (if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_100 }, text) };
                                    let stroke = egui::Stroke::new(1.0, if sel { ColorPalette::BLUE_500 } else { if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_300 } });
                                    if ui.scope(|ui| {
                                        let s = ui.style_mut();
                                        s.visuals.widgets.inactive.bg_fill = fill;
                                        s.visuals.widgets.inactive.weak_bg_fill = fill;
                                        s.visuals.widgets.inactive.bg_stroke = stroke;
                                        s.visuals.widgets.hovered.bg_fill = fill.linear_multiply(1.1);
                                        s.visuals.widgets.hovered.weak_bg_fill = fill.linear_multiply(1.1);
                                        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ColorPalette::BLUE_400);
                                        ui.add(egui::Button::new(egui::RichText::new(*label).size(13.0).color(tc)).min_size(egui::vec2(0.0, 28.0)))
                                    }).inner.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                        self.default_font = name.to_string(); prefs_changed = true;
                                    }
                                }
                            });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Default Font Size:").size(14.0).color(text));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.add(egui::DragValue::new(&mut self.default_font_size).range(8.0..=72.0).speed(0.5).suffix(" pt")).changed() { prefs_changed = true; }
                                });
                            });
                        }
                        SettingsTab::JsonEditor => {
                            ui.label(egui::RichText::new("DISPLAY").size(11.0).color(muted));
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Show File Info").size(14.0).color(text));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.checkbox(&mut self.show_file_info_je, "").changed() { prefs_changed = true; } });
                            });
                        }
                        SettingsTab::Cache => {
                            let count = self.cache_entries.as_ref().map(|v| v.len()).unwrap_or(0);
                            let total_kb: u64 = self.cache_entries.as_ref().map(|v| v.iter().map(|e| e.size_kb).sum()).unwrap_or(0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("LAYER CACHES").size(11.0).color(muted));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.add_enabled(count > 0, egui::Button::new(egui::RichText::new("Clear All").size(12.0).color(if is_dark { ColorPalette::RED_400 } else { ColorPalette::RED_600 }))).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                        ie_cache::delete_all_caches(); self.cache_entries = Some(Vec::new());
                                    }
                                });
                            });
                            ui.label(egui::RichText::new(format!("{} cached files  ·  {} KB total", count, total_kb)).size(12.0).color(muted));
                            ui.add_space(8.0);
                            if count == 0 {
                                ui.label(egui::RichText::new("No layer caches stored.").size(13.0).color(muted).italics());
                            } else {
                                egui::ScrollArea::vertical().max_height(220.0).id_salt("cache_scroll").show(ui, |ui| {
                                    if let Some(ref entries_vec) = self.cache_entries {
                                        for (i, entry) in entries_vec.iter().enumerate() {
                                            let fname = std::path::Path::new(&entry.src_path).file_name().and_then(|n| n.to_str()).unwrap_or(&entry.src_path);
                                            egui::Frame::new().fill(if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_100 }).corner_radius(4.0).inner_margin(egui::Margin { left: 10, right: 8, top: 6, bottom: 6 }).show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(egui::RichText::new(fname).size(13.0).color(text));
                                                        ui.label(egui::RichText::new(&entry.src_path).size(10.0).color(muted));
                                                    });
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        ui.label(egui::RichText::new(format!("{} KB", entry.size_kb)).size(11.0).color(muted));
                                                        if ui.button(egui::RichText::new("Delete").size(11.0)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { to_delete = Some(i); }
                                                        if ui.button(egui::RichText::new("Open").size(11.0)).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Open cache metadata in JSON Editor").clicked() {
                                                            self.open_cache_path = Some(entry.cache_dir.join("meta.json"));
                                                        }
                                                    });
                                                });
                                            });
                                            ui.add_space(4.0);
                                        }
                                    }
                                });
                            }
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Layer caches are automatically cleared if the source image is modified outside this application.").size(11.0).color(muted).italics());
                        }
                    }
                });
            });
        });

        if outside || hdr_close { self.show_settings = false; self.cache_entries = None; }
        if sys_c { self.theme_preference = ThemePreference::System; self.theme_mode = match ctx.theme() { egui::Theme::Dark => ThemeMode::Dark, egui::Theme::Light => ThemeMode::Light }; style::apply_theme(ctx, self.theme_mode); self.save_settings(); }
        if light_c { self.theme_preference = ThemePreference::Light; self.theme_mode = ThemeMode::Light; style::apply_theme(ctx, self.theme_mode); self.save_settings(); }
        if dark_c { self.theme_preference = ThemePreference::Dark; self.theme_mode = ThemeMode::Dark; style::apply_theme(ctx, self.theme_mode); self.save_settings(); }
        if prefs_changed { self.save_settings(); }
        if let Some(idx) = to_delete {
            if let Some(ref v) = self.cache_entries {
                if let Some(e) = v.get(idx) { let _ = std::fs::remove_dir_all(&e.cache_dir); }
            }
            self.cache_entries = Some(ie_cache::list_caches());
        }
    }

    fn render_patch_notes_modal(&mut self, ctx: &egui::Context) {
        if !self.show_patch_notes { return; }
        let theme = self.theme_mode;
        let is_dark = matches!(theme, ThemeMode::Dark);
        let (muted, text) = if is_dark { (ColorPalette::ZINC_500, ColorPalette::SLATE_200) } else { (ColorPalette::GRAY_900, ColorPalette::GRAY_700) };
        let tag_bg = if is_dark { ColorPalette::MODAL_TAG_BG_DARK } else { ColorPalette::BLUE_50 };
        let total_pages = self.patch_notes.len().max(1);
        if self.patch_notes_page >= total_pages { self.patch_notes_page = total_pages - 1; }
        let mut hdr_close = false;

        let outside = style::main_menu_modal(ctx, "patchnotes_mw", theme, 520.0, |ui| {
            if style::main_menu_modal_header(ui, "Patch Notes", "", theme) { hdr_close = true; }
            egui::ScrollArea::vertical().max_height(420.0).auto_shrink([false, true]).show(ui, |ui| {
                egui::Frame::new().inner_margin(egui::Margin { left: 28, right: 28, top: 16, bottom: 8 }).show(ui, |ui| {
                    if let Some(entry) = self.patch_notes.get(self.patch_notes_page) {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&entry.version).size(16.0).strong().color(text));
                            ui.add_space(6.0);
                            if !entry.tag.is_empty() {
                                egui::Frame::new().fill(tag_bg).corner_radius(4.0).inner_margin(egui::Margin { left: 6, right: 6, top: 2, bottom: 2 })
                                    .show(ui, |ui| { ui.label(egui::RichText::new(&entry.tag).size(10.0).color(muted)); });
                            }
                        });
                        ui.add_space(12.0);
                        let cat_colors = [ColorPalette::BLUE_500, ColorPalette::TEAL_500, ColorPalette::PURPLE_500];
                        for (ci, category) in entry.categories.iter().enumerate() {
                            if !category.name.is_empty() {
                                let cat_color = cat_colors[ci % cat_colors.len()];
                                ui.horizontal(|ui| {
                                    let rm = ui.cursor().min;
                                    ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(rm.x, rm.y + 2.0), egui::vec2(3.0, 14.0)), 1.5, cat_color);
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new(&category.name).size(12.0).strong().color(muted));
                                });
                                ui.add_space(6.0);
                            }
                            for note in &category.notes {
                                ui.scope(|ui| {
                                    ui.spacing_mut().item_spacing.y = 3.0;
                                    ui.horizontal_wrapped(|ui| {
                                        ui.add_space(14.0);
                                        ui.painter().circle_filled(egui::pos2(ui.cursor().min.x + 3.0, ui.cursor().min.y + 8.0), 2.0, ColorPalette::BLUE_500);
                                        ui.add_space(10.0);
                                        if !note.module_tag.is_empty() {
                                            let (chip_bg, chip_text) = if is_dark { (ColorPalette::CHIP_BG_DARK, ColorPalette::BLUE_400) } else { (ColorPalette::BLUE_50, ColorPalette::BLUE_600) };
                                            egui::Frame::new().fill(chip_bg).corner_radius(3.0).inner_margin(egui::Margin { left: 5, right: 5, top: 1, bottom: 1 })
                                                .show(ui, |ui| { ui.label(egui::RichText::new(&note.module_tag).size(11.0).color(chip_text)); });
                                            ui.add_space(4.0);
                                        }
                                        ui.label(egui::RichText::new(&note.text).size(13.0).color(text));
                                    });
                                });
                                ui.add_space(7.0);
                            }
                            if ci < entry.categories.len() - 1 { ui.add_space(10.0); }
                        }
                        ui.add_space(20.0);
                    } else {
                        ui.label("No patch notes available.");
                    }
                });
            });
            let (sep, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter().rect_filled(sep, 0.0, if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_200 });
            egui::Frame::new().inner_margin(egui::Margin { left: 24, right: 24, top: 10, bottom: 10 }).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.patch_notes_page > 0, egui::Button::new("< Prev")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.patch_notes_page -= 1; }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(self.patch_notes_page < total_pages - 1, egui::Button::new("Next >")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.patch_notes_page += 1; }
                        ui.label(egui::RichText::new(format!("Page {} of {}", self.patch_notes_page + 1, total_pages)).color(muted));
                    });
                });
            });
        });

        if outside || hdr_close { self.show_patch_notes = false; }
    }

    fn render_about_modal(&mut self, ctx: &egui::Context) {
        if !self.show_about { return; }
        let theme = self.theme_mode;
        let is_dark = matches!(theme, ThemeMode::Dark);
        let title_col = if is_dark { egui::Color32::WHITE } else { ColorPalette::GRAY_900 };
        let text_col = if is_dark { ColorPalette::SLATE_300 } else { ColorPalette::GRAY_700 };
        let muted_col = if is_dark { ColorPalette::ZINC_500 } else { ColorPalette::GRAY_400 };
        let section_col = if is_dark { ColorPalette::ZINC_400 } else { ColorPalette::GRAY_500 };
        let card_bg = if is_dark { egui::Color32::from_rgb(26, 26, 32) } else { ColorPalette::GRAY_50 };
        let card_border = if is_dark { egui::Color32::from_rgb(46, 46, 54) } else { ColorPalette::GRAY_200 };
        let mut hdr_close = false;

        let outside = style::main_menu_modal(ctx, "about_mw", theme, 640.0, |ui| {
            ui.set_min_height(600.0);
            if style::main_menu_modal_header(ui, "Universal Editor", &format!("v{}  ·  Built with Rust + egui", env!("CARGO_PKG_VERSION")), theme) { hdr_close = true; }

            egui::ScrollArea::vertical().max_height(880.0).auto_shrink([false, true]).show(ui, |ui| {
                egui::Frame::new().inner_margin(egui::Margin { left: 28, right: 28, top: 16, bottom: 4 }).show(ui, |ui| {
                    ui.label(egui::RichText::new("A lightweight, modular desktop suite for editing text, images, and structured data, all in one place.").size(13.0).color(text_col));
                    ui.add_space(16.0);

                    let section = |ui: &mut egui::Ui, label: &str| {
                        ui.add_space(4.0);
                        egui::Frame::new().fill(if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_200 })
                            .inner_margin(egui::Margin { left: 16, right: 16, top: 4, bottom: 4 })
                            .show(ui, |ui| { ui.label(egui::RichText::new(label).size(10.0).color(section_col).strong()); });
                    };

                    section(ui, "MODULES");
                    ui.columns(2, |cols| {
                        for (i, &(letter, name, desc, accent)) in [
                            ("T", "Text Editor", "Markdown & plain text editing with live preview, formatting shortcuts, heading styles, tables, checklists, and inline code rendering.", ColorPalette::BLUE_500),
                            ("I", "Image Editor", "Layer-based raster editor with brushes, eraser, fill, text layers, crop, retouch tools, blend modes, and filter adjustments.", ColorPalette::PURPLE_500),
                            ("J", "JSON Editor", "Tree and raw text views for JSON with inline editing, undo/redo, sorting, search, breadcrumb navigation, and schema-free editing.", ColorPalette::AMBER_500),
                            ("W", "Document Editor", "Write and format rich documents with paragraph styles, heading hierarchy, inline formatting, alignment, indentation, and export.", ColorPalette::GREEN_500),
                            ("C", "Image Converter", "Batch-convert images between JPEG, PNG, WebP, BMP, TIFF, ICO, and AVIF with per-format quality controls and custom output paths.", ColorPalette::TEAL_500),
                            ("D", "Data Converter", "Convert structured data between JSON, YAML, TOML, XML, and CSV formats with pretty-print options and overwrite controls.", ColorPalette::GREEN_600),
                            ("A", "Archive Converter", "Convert structured data between ZIP, TAR, TAR.GZ, TAR.BZ2, and 7z archive formats with compression level settings.", ColorPalette::AMBER_600),
                        ].iter().enumerate() {
                            let col = &mut cols[i % 2];
                            egui::Frame::new().fill(card_bg).stroke(egui::Stroke::new(1.0, card_border)).corner_radius(8.0).inner_margin(14.0).show(col, |ui| {
                                ui.horizontal(|ui| {
                                    let (br, _) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::hover());
                                    ui.painter().rect_filled(br, 6.0, accent.linear_multiply(if is_dark { 0.25 } else { 0.12 }));
                                    ui.painter().text(br.center(), egui::Align2::CENTER_CENTER, letter, egui::FontId::proportional(13.0).into(), accent);
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new(name).size(13.0).strong().color(title_col));
                                });
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new(desc).size(11.5).color(text_col));
                            });
                            col.add_space(8.0);
                        }
                    });

                    section(ui, "FEATURES AT A GLANCE");
                    for &(feat, detail) in &[
                        ("Dark & Light themes", "Full system theme detection plus manual override."),
                        ("Recent files", "Persistent history with quick-open and rename support."),
                        ("Keyboard shortcuts", "Comprehensive shortcuts across all modules."),
                        ("Undo / Redo", "Multi-level history in the Image and JSON editors."),
                        ("Drag-and-drop", "Drop files onto converters or the image canvas."),
                        ("Custom fonts", "Ubuntu, Roboto, and Google/Open Sans shipped; selectable in Settings."),
                    ] {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.painter().circle_filled(egui::pos2(ui.cursor().min.x + 5.0, ui.cursor().min.y + 8.0), 2.5, ColorPalette::BLUE_500);
                            ui.add_space(14.0);
                            ui.label(egui::RichText::new(format!("{}:", feat)).size(12.5).strong().color(title_col));
                            ui.label(egui::RichText::new(format!(" {}", detail)).size(12.0).color(text_col));
                        }); ui.add_space(4.0);
                    }

                    section(ui, "ARCHITECTURE");
                    for &(heading, body) in &[
                        ("Kernel & module model", "A persistent shell handles windowing, GPU-accelerated rendering, global theming, and the sidebar registry. Individual editors are mounted on demand, switching modules does not spawn a new process or reload application state."),
                        ("Rope-backed text", "The text editor stores content as a balanced tree of chunks rather than a contiguous string, giving constant-time insertions and deletions anywhere in a file regardless of its size."),
                        ("Image handling", "Image I/O is handled through the image crate, compiled with only the format features actually used to keep the binary lean. No operation blocks the main thread, and memory usage stays flat regardless of file size."),
                        ("Design system", "Styling is driven by a central ColorPalette and ThemeMode enum, similar to a CSS design token file. Every button, modal, sidebar, and overlay pulls from the same palette. Ubuntu and Roboto fonts are compiled directly into the binary, no system fonts required."),
                    ] {
                        egui::Frame::new().fill(card_bg).stroke(egui::Stroke::new(1.0, card_border)).corner_radius(6.0)
                            .inner_margin(egui::Margin { left: 12, right: 12, top: 10, bottom: 10 })
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(heading).size(12.5).strong().color(title_col));
                                ui.add_space(3.0);
                                ui.label(egui::RichText::new(body).size(12.0).color(text_col));
                            });
                        ui.add_space(6.0);
                    }

                    section(ui, "PROJECT GOALS");
                    for &(goal, desc, accent) in &[
                        ("Modularity", "Adding a new editor requires defining a single struct and registering it, no changes to the shell. Helpers, UI components, and styling primitives are shared across modules rather than duplicated.", ColorPalette::BLUE_500),
                        ("Performance", "The Rope structure, lazy image decoding, and GPU-direct rendering keep the application fast. No operation blocks the main thread, and memory usage stays flat regardless of open file size.", ColorPalette::TEAL_500),
                        ("Modern design", "Custom typography, a Tailwind-like color system, consistent spacing, and smooth interactions on par with web-based tools, without the overhead of a browser engine.", ColorPalette::PURPLE_500),
                    ] {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.painter().circle_filled(egui::pos2(ui.cursor().min.x + 5.0, ui.cursor().min.y + 8.0), 2.5, accent);
                            ui.add_space(14.0);
                            ui.label(egui::RichText::new(format!("{}:", goal)).size(12.5).strong().color(title_col));
                            ui.label(egui::RichText::new(format!(" {}", desc)).size(12.0).color(text_col));
                        }); ui.add_space(5.0);
                    }

                    let sep = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 1.0));
                    ui.allocate_rect(sep, egui::Sense::hover());
                    ui.painter().rect_filled(sep, 0.0, card_border);
                    ui.add_space(14.0);

                    section(ui, "FONTS & LICENSES");
                    egui::Frame::new().fill(card_bg).stroke(egui::Stroke::new(1.0, card_border)).corner_radius(6.0)
                        .inner_margin(egui::Margin { left: 12, right: 12, top: 10, bottom: 10 })
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("All bundled typefaces are licensed under the SIL Open Font License, Version 1.1.").size(12.0).color(text_col));
                            ui.add_space(8.0);
                            for &(name, copyright) in &[
                                ("Ubuntu", "Copyright 2010-2011 Canonical Ltd. Ubuntu is a registered trademark of Canonical Ltd."),
                                ("Roboto", "Copyright 2011 Google LLC."),
                                ("Google Sans", "Copyright 2016 Google LLC."),
                                ("Open Sans", "Copyright 2020 The Open Sans Project Authors."),
                            ] {
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    ui.label(egui::RichText::new(name).size(12.0).strong().color(title_col));
                                    ui.label(egui::RichText::new(copyright).size(11.5).color(muted_col));
                                });
                                ui.add_space(3.0);
                            }
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("The SIL OFL allows the fonts to be used, studied, modified, and redistributed freely, provided that derivative works are not sold by themselves and that this license and copyright notice are retained. Full license text: https://openfontlicense.org").size(11.0).color(muted_col).italics());
                        });
                    ui.add_space(6.0);

                    let sep2 = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 1.0));
                    ui.allocate_rect(sep2, egui::Sense::hover());
                    ui.painter().rect_filled(sep2, 0.0, card_border);
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Universal Editor  v{}", env!("CARGO_PKG_VERSION"))).size(11.0).color(muted_col));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("Built with Rust, egui & eframe").size(11.0).color(muted_col));
                        });
                    });
                    ui.add_space(4.0);
                });
            });
        });

        if outside || hdr_close { self.show_about = false; }
    }
}

impl eframe::App for UniversalEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if matches!(self.theme_preference, ThemePreference::System) {
            let system_theme = match ctx.theme() { egui::Theme::Dark => ThemeMode::Dark, egui::Theme::Light => ThemeMode::Light };
            if self.theme_mode != system_theme { self.theme_mode = system_theme; style::apply_theme(ctx, self.theme_mode); }
        }

        while let Ok(path) = self.recent_file_rx.try_recv() { self.recent_files.add_file(path); }
        while let Ok((old, new)) = self.path_replace_rx.try_recv() { self.recent_files.remove_file(&old); self.recent_files.add_file(new); }

        if let Some(path) = self.open_cache_path.take() {
            self.show_settings = false;
            self.cache_entries = None;
            self.save_image_cache_if_needed();
            self.active_module = Some(Box::new(JsonEditor::load(path)));
        }

        if let Some(PendingAction::Exit) = &self.pending_action {
            if !self.show_unsaved_dialog { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        }

        if !self.show_unsaved_dialog && !self.show_settings && !self.show_patch_notes && !self.show_about {
            ctx.input_mut(|i| { if i.consume_key(egui::Modifiers::CTRL, egui::Key::Backslash) { self.sidebar_open = !self.sidebar_open; } });
        }

        self.render_unsaved_dialog(ctx);
        self.render_settings_modal(ctx);
        self.render_patch_notes_modal(ctx);
        self.render_about_modal(ctx);
        self.rename_modal(ctx);
        self.top_bar(ctx);
        self.sidebar(ctx);

        let show_fi = if self.is_in_json_editor() { self.show_file_info_je } else { self.show_file_info_te };
        let show_toolbar = self.show_toolbar_te;
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(module) = &mut self.active_module { module.ui(ui, ctx, show_toolbar, show_fi); }
            else { self.landing_page(ui); }
        });

        let converter_path = self.active_module.as_mut().and_then(|m| m.take_converter_path());
        if let Some(path) = converter_path {
            let mut converter = crate::modules::data_converter::DataConverter::new();
            converter.add_files_pub(vec![path]);
            self.switch_to_module(Box::new(converter));
        }

        if self.show_unsaved_dialog { ctx.set_cursor_icon(egui::CursorIcon::Default); }
    }
}
