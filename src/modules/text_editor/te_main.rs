use eframe::egui;
use ropey::Rope;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use crate::modules::{EditorModule, MenuAction, MenuItem, MenuContribution};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Plain,
    Markdown,
}

pub(super) struct LineHeightCache {
    pub version: u64,
    pub font_size: f32,
    pub wrap_width: f32,
    pub is_dark: bool,
    pub heights: Vec<Vec<f32>>,
}

pub struct TextEditor {
    pub(super) file_path: Option<PathBuf>,
    pub(super) content: String,
    pub(super) dirty: bool,
    pub(super) font_size: f32,
    pub(super) font_family: egui::FontFamily,
    pub(super) view_mode: ViewMode,
    pub(super) last_cursor_range: Option<egui::text::CCursorRange>,
    pub(super) pending_cursor_pos: Option<usize>,
    pub(super) content_version: u64,
    pub(super) cached_word_count: usize,
    pub(super) cached_char_count: usize,
    pub(super) cached_counts_version: u64,
    pub(super) line_height_cache: Option<LineHeightCache>,
}

impl TextEditor {
    pub fn new_empty() -> Self {
        Self {
            file_path: None,
            content: String::new(),
            dirty: false,
            font_size: 14.0,
            font_family: egui::FontFamily::Monospace,
            view_mode: ViewMode::Plain,
            last_cursor_range: None,
            pending_cursor_pos: None,
            content_version: 0,
            cached_word_count: 0,
            cached_char_count: 0,
            cached_counts_version: u64::MAX,
            line_height_cache: None,
        }
    }

    pub fn load(path: PathBuf) -> Self {
        let content: String = File::open(&path).ok()
            .map(BufReader::new)
            .and_then(|r: BufReader<File>| Rope::from_reader(r).ok())
            .map(|rope: Rope| rope.to_string().replace("\r\n", "\n"))
            .unwrap_or_default();

        let view_mode: ViewMode = Self::detect_view_mode(&path);

        Self {
            file_path: Some(path),
            content,
            dirty: false,
            font_size: 14.0,
            font_family: egui::FontFamily::Monospace,
            view_mode,
            last_cursor_range: None,
            pending_cursor_pos: None,
            content_version: 0,
            cached_word_count: 0,
            cached_char_count: 0,
            cached_counts_version: u64::MAX,
            line_height_cache: None,
        }
    }

    pub(super) fn detect_view_mode(path: &PathBuf) -> ViewMode {
        path.extension()
            .and_then(|e: &std::ffi::OsStr| e.to_str())
            .map(|e: &str| match e.to_lowercase().as_str() {
                "md" | "markdown" => ViewMode::Markdown,
                _ => ViewMode::Plain,
            })
            .unwrap_or(ViewMode::Plain)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(super) fn get_file_name(&self) -> String {
        self.file_path.as_ref()
            .and_then(|p: &PathBuf| p.file_name())
            .and_then(|n: &std::ffi::OsStr| n.to_str())
            .map(|s: &str| s.to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }
}

impl EditorModule for TextEditor {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn get_title(&self) -> String {
        let name = self.file_path.as_ref()
            .and_then(|p: &PathBuf| p.file_name())
            .and_then(|n: &std::ffi::OsStr| n.to_str())
            .unwrap_or("Untitled");
        if self.dirty { format!("{} *", name) } else { name.to_string() }
    }

    fn save(&mut self) -> Result<(), String> {
        if self.file_path.is_none() {
            return self.save_as();
        }
        let path: &PathBuf = self.file_path.as_ref().unwrap();
        let f: File = File::create(path).map_err(|e: std::io::Error| e.to_string())?;
        let mut writer: BufWriter<File> = BufWriter::new(f);
        let rope: Rope = Rope::from_str(&self.content);
        rope.write_to(&mut writer).map_err(|e: std::io::Error| e.to_string())?;
        self.dirty = false;
        Ok(())
    }

    fn save_as(&mut self) -> Result<(), String> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text", &["txt", "md"])
            .save_file()
        {
            self.file_path = Some(path);
            self.save()
        } else {
            Err("Cancelled".to_string())
        }
    }

    fn get_menu_contributions(&self) -> MenuContribution {
        MenuContribution {
            file_items: Vec::new(),
            edit_items: vec![
                (MenuItem { label: "Undo".to_string(), shortcut: Some("Ctrl+Z".to_string()), enabled: false }, MenuAction::Undo),
                (MenuItem { label: "Redo".to_string(), shortcut: Some("Ctrl+Y".to_string()), enabled: false }, MenuAction::Redo),
            ],
            view_items: Vec::new(),
            image_items: Vec::new(),
            filter_items: Vec::new(),
        }
    }

    fn handle_menu_action(&mut self, action: MenuAction) -> bool {
        match action {
            MenuAction::Undo | MenuAction::Redo => false,
            _ => false,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, show_toolbar: bool, show_file_info: bool) {
        self.render_editor_ui(ui, ctx, show_toolbar, show_file_info);
    }
}
