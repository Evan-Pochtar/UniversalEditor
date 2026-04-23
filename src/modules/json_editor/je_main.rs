use eframe::egui;
use ropey::Rope;
use serde_json::Value;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use crate::modules::{EditorModule, MenuAction, MenuItem, MenuContribution};
use super::je_tools::{
    SortMode, SearchTarget, FlatNode,
    build_flat, serialize_value, parse_text, expand_recursive, collapse_recursive,
    search_flat, search_all_nodes, path_key,
};

pub(super) const TEXT_WIN_LINES: usize = 350;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JsonViewMode { Tree, Text }

#[derive(Debug, Clone)]
pub struct EditCell { pub path: Vec<String>, pub buffer: String, pub editing_key: bool, }

#[derive(Debug, Clone)]
pub struct AddKeyDialog { pub parent_path: Vec<String>, pub key_buf: String, pub val_buf: String, pub error: Option<String>, }

pub struct JsonEditor {
    pub(super) file_path: Option<PathBuf>,
    pub(super) dirty: bool,
    pub(super) root: Value,

    pub(super) view_mode: JsonViewMode,
    pub(super) scope_path: Vec<String>,
    pub(super) flat: Vec<FlatNode>,
    pub(super) flat_stale: bool,
    pub(super) expanded: HashSet<String>,

    pub(super) sort_mode: SortMode,
    pub(super) search_query: String,
    pub(super) search_target: SearchTarget,
    pub(super) search_results: Vec<usize>,
    pub(super) search_all_paths: Vec<Vec<String>>,
    pub(super) search_only_expanded: bool,
    pub(super) search_cursor: usize,
    pub(super) search_stale: bool,

    pub(super) text_content: String,
    pub(super) text_rope: Rope,
    pub(super) text_win_buf: String,
    pub(super) text_win_start: usize,
    pub(super) text_win_line_count: usize,
    pub(super) text_win_modified: bool,
    pub(super) text_row_h: f32,
    pub(super) text_stale: bool,
    pub(super) text_modified: bool,
    pub(super) text_errors: Vec<(usize, String)>,

    pub(super) pending_scroll_row: Option<usize>,

    pub(super) undo_stack: VecDeque<Box<Value>>,
    pub(super) redo_stack: VecDeque<Box<Value>>,
    pub(super) undo_limit: usize,

    pub(super) add_dialog: Option<AddKeyDialog>,
    pub(super) edit_cell: Option<EditCell>,
    pub(super) edit_cell_is_string: bool,
    pub(super) selected_row: Option<usize>,
    pub(super) confirm_delete_path: Option<Vec<String>>,

    pub(super) export_pretty: bool,
    pub(super) show_new_confirm: bool,
    pub(super) save_error: Option<String>,
    pub(super) rename_modal_open: bool,
    pub(super) rename_buffer: String,
    pub(super) open_in_converter_path: Option<std::path::PathBuf>,
}

impl JsonEditor {
    pub fn is_dirty(&self) -> bool { self.dirty }
    pub fn is_text_modified(&self) -> bool { self.text_modified }
    pub fn new_empty() -> Self {
        let root = Value::Object(serde_json::Map::new());
        Self::from_value(root, None, None)
    }

    pub fn load(path: PathBuf) -> Self {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let root = serde_json::from_str(&content).unwrap_or(Value::Null);
        Self::from_value(root, Some(path), Some(content))
    }

    fn from_value(root: Value, path: Option<PathBuf>, raw_content: Option<String>) -> Self {
        let scope_path: Vec<String> = Vec::new();
        let mut expanded = HashSet::new();
        expanded.insert(path_key(&scope_path));

        let flat = build_flat(&root, &scope_path, &expanded, SortMode::None);
        let text_content = raw_content.unwrap_or_else(|| serialize_value(&root, true));
        let text_rope = Rope::from_str(&text_content);
        let init_win = TEXT_WIN_LINES.min(text_rope.len_lines());
        let text_win_buf = text_rope.slice(..text_rope.line_to_char(init_win)).to_string();

        Self {
            file_path: path,
            dirty: false,
            root,
            view_mode: JsonViewMode::Tree,
            scope_path,
            flat,
            flat_stale: false,
            expanded,
            sort_mode: SortMode::None,
            search_query: String::new(),
            search_target: SearchTarget::Both,
            search_results: Vec::new(),
            search_all_paths: Vec::new(),
            search_only_expanded: true,
            search_cursor: 0,
            search_stale: false,
            text_content,
            text_rope,
            text_win_buf,
            text_win_start: 0,
            text_win_line_count: init_win,
            text_win_modified: false,
            text_row_h: 0.0,
            text_stale: false,
            text_modified: false,
            text_errors: Vec::new(),
            pending_scroll_row: None,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            undo_limit: 20,
            add_dialog: None,
            edit_cell: None,
            edit_cell_is_string: false,
            selected_row: None,
            confirm_delete_path: None,
            export_pretty: true,
            show_new_confirm: false,
            save_error: None,
            rename_modal_open: false,
            rename_buffer: String::new(),
            open_in_converter_path: None,
        }
    }

    pub(super) fn get_file_name(&self) -> String {
        self.file_path.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Untitled.json".to_string())
    }

    pub(super) fn rebuild_flat_if_needed(&mut self) {
        if self.flat_stale {
            self.flat = build_flat(&self.root, &self.scope_path, &self.expanded, self.sort_mode);
            self.flat_stale = false;
            if self.search_only_expanded { self.search_stale = true; }
        }
    }
    pub(super) fn invalidate_flat(&mut self) {
        self.flat_stale = true;
    }


    pub(super) fn sync_text_from_root(&mut self) {
        self.text_content = serialize_value(
            super::je_tools::value_at_path(&self.root, &self.scope_path)
                .unwrap_or(&self.root),
            true,
        );
        self.text_rope = Rope::from_str(&self.text_content);
        self.text_win_start = 0;
        self.text_win_modified = false;
        self.rebuild_text_window();
        self.text_errors.clear();
        self.text_stale = false;
        self.text_modified = false;
    }

    pub(super) fn commit_text_to_root(&mut self) -> bool {
        self.sync_window_to_rope();
        let full_text = self.text_rope.to_string();
        match parse_text(&full_text) {
            Ok(v) => {
                self.push_undo();
                if self.scope_path.is_empty() {
                    self.root = v;
                } else {
                    super::je_tools::set_at_path(&mut self.root, &self.scope_path, v);
                }
                self.text_content = full_text;
                self.text_errors.clear();
                self.dirty = true;
                self.invalidate_flat();
                true
            }
            Err((msg, line)) => {
                self.text_errors = vec![(line, msg)];
                false
            }
        }
    }

    pub(super) fn rebuild_text_window(&mut self) {
        let total = self.text_rope.len_lines();
        let start = self.text_win_start.min(total.saturating_sub(1));
        let end = (start + TEXT_WIN_LINES).min(total);
        self.text_win_start = start;
        self.text_win_line_count = end - start;
        let cs = self.text_rope.line_to_char(start);
        let ce = if end >= total { self.text_rope.len_chars() } else { self.text_rope.line_to_char(end) };
        self.text_win_buf = self.text_rope.slice(cs..ce).to_string();
        self.text_win_modified = false;
    }

    pub(super) fn sync_window_to_rope(&mut self) {
        if !self.text_win_modified { return; }
        let total = self.text_rope.len_lines();
        let start = self.text_win_start.min(total.saturating_sub(1));
        let end = (start + self.text_win_line_count).min(total);
        let cs = self.text_rope.line_to_char(start);
        let ce = if end >= total { self.text_rope.len_chars() } else { self.text_rope.line_to_char(end) };
        self.text_rope.remove(cs..ce);
        self.text_rope.insert(cs, &self.text_win_buf);
        let new_total = self.text_rope.len_lines();
        self.text_win_line_count = (start + TEXT_WIN_LINES).min(new_total) - start;
        self.text_win_modified = false;
    }

    pub(super) fn push_undo(&mut self) {
        self.redo_stack.clear();
        self.undo_stack.push_back(Box::new(self.root.clone()));
        if self.undo_stack.len() > self.undo_limit {
            self.undo_stack.pop_front();
        }
    }

    pub(super) fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(Box::new(self.root.clone()));
            self.root = *prev;
            self.dirty = true;
            self.invalidate_flat();
            self.text_stale = true;
            self.search_stale = true;
        }
    }

    pub(super) fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(Box::new(self.root.clone()));
            self.root = *next;
            self.dirty = true;
            self.invalidate_flat();
            self.text_stale = true;
            self.search_stale = true;
        }
    }

    pub(super) fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }
    pub(super) fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }

    pub(super) fn search_result_count(&self) -> usize {
        if self.search_only_expanded { self.search_results.len() } else { self.search_all_paths.len() }
    }

    pub(super) fn run_search(&mut self) {
        if !self.search_stale { return; }
        if self.search_only_expanded {
            self.search_results = search_flat(&self.flat, &self.search_query, self.search_target);
            self.search_all_paths.clear();
            if self.search_cursor >= self.search_results.len() { self.search_cursor = 0; }
            self.pending_scroll_row = self.search_results.get(self.search_cursor).cloned();
        } else {
            self.search_all_paths = search_all_nodes(&self.root, &self.scope_path, &self.search_query, self.search_target);
            self.search_results.clear();
            if self.search_cursor >= self.search_all_paths.len() { self.search_cursor = 0; }
            self.apply_all_nodes_cursor();
        }
        self.search_stale = false;
    }

    fn expand_to_path(&mut self, path: &[String]) {
        for i in 0..path.len() { self.expanded.insert(path_key(&path[..i])); }
        self.flat = build_flat(&self.root, &self.scope_path, &self.expanded, self.sort_mode);
        self.flat_stale = false;
    }

    fn apply_all_nodes_cursor(&mut self) {
        if let Some(path) = self.search_all_paths.get(self.search_cursor).cloned() {
            self.expand_to_path(&path);
            self.pending_scroll_row = self.flat.iter().position(|n| n.path == path);
        }
    }

    pub(super) fn search_next(&mut self) {
        let count = self.search_result_count();
        if count == 0 { return; }
        self.search_cursor = (self.search_cursor + 1) % count;
        if self.search_only_expanded {
            self.pending_scroll_row = self.search_results.get(self.search_cursor).cloned();
        } else { self.apply_all_nodes_cursor(); }
    }

    pub(super) fn search_prev(&mut self) {
        let count = self.search_result_count();
        if count == 0 { return; }
        self.search_cursor = if self.search_cursor == 0 { count - 1 } else { self.search_cursor - 1 };
        if self.search_only_expanded {
            self.pending_scroll_row = self.search_results.get(self.search_cursor).cloned();
        } else { self.apply_all_nodes_cursor(); }
    }

    pub(super) fn toggle_expand(&mut self, path: &[String]) {
        let key = path_key(path);
        if self.expanded.contains(&key) {
            collapse_recursive(path, &mut self.expanded);
        } else {
            self.expanded.insert(key);
        }
        self.invalidate_flat();
    }

    pub(super) fn expand_all(&mut self) {
        expand_recursive(&self.root, &self.scope_path, &self.scope_path, 64, &mut self.expanded);
        self.invalidate_flat();
    }

    pub(super) fn collapse_all(&mut self) {
        self.expanded.clear();
        self.expanded.insert(path_key(&self.scope_path));
        self.invalidate_flat();
    }

    pub(super) fn drill_into(&mut self, path: Vec<String>) {
        self.scope_path = path;
        self.expanded.insert(path_key(&self.scope_path));
        self.invalidate_flat();
        self.text_stale = true;
        self.search_stale = true;
        self.selected_row = None;
    }

    pub(super) fn scope_up(&mut self) {
        if !self.scope_path.is_empty() {
            self.scope_path.pop();
            self.expanded.insert(path_key(&self.scope_path));
            self.invalidate_flat();
            self.text_stale = true;
            self.search_stale = true;
            self.selected_row = None;
        }
    }

    pub(super) fn delete_node(&mut self, path: Vec<String>) {
        self.push_undo();
        super::je_tools::delete_at_path(&mut self.root, &path);
        self.dirty = true;
        self.invalidate_flat();
        self.text_stale = true;
        self.search_stale = true;
    }

    pub(super) fn set_node_value(&mut self, path: &[String], val: Value) {
        self.push_undo();
        super::je_tools::set_at_path(&mut self.root, path, val);
        self.dirty = true;
        self.invalidate_flat();
        self.text_stale = true;
        self.search_stale = true;
    }

    pub(super) fn rename_node_key(&mut self, parent_path: &[String], old_key: &str, new_key: &str) {
        self.push_undo();
        super::je_tools::rename_key_at_path(&mut self.root, parent_path, old_key, new_key);
        self.dirty = true;
        self.invalidate_flat();
        self.text_stale = true;
        self.search_stale = true;
    }

    pub(super) fn add_node(
        &mut self,
        parent_path: &[String],
        key: &str,
        val: Value,
    ) {
        self.push_undo();
        super::je_tools::add_key_at_path(&mut self.root, parent_path, key, val);
        self.dirty = true;
        self.invalidate_flat();
        self.text_stale = true;
        self.search_stale = true;
    }
    
    pub(super) fn reset_to_empty(&mut self) {
        self.push_undo();
        self.root = Value::Object(serde_json::Map::new());
        self.scope_path.clear();
        self.expanded.clear();
        self.expanded.insert(path_key(&self.scope_path));
        self.dirty = true;
        self.invalidate_flat();
        self.text_stale = true;
        self.search_stale = true;
        self.search_query.clear();
        self.search_results.clear();
        self.search_all_paths.clear();
        self.edit_cell = None;
        self.edit_cell_is_string = false;
        self.add_dialog = None;
    }
}

impl EditorModule for JsonEditor {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn take_converter_path(&mut self) -> Option<std::path::PathBuf> {
        self.open_in_converter_path.take()
    }

    fn get_title(&self) -> String {
        let name = self.get_file_name();
        if self.dirty { format!("{} *", name) } else { name }
    }

    fn save(&mut self) -> Result<(), String> {
        if self.file_path.is_none() {
            return self.save_as();
        }
        if self.text_modified {
            if !self.commit_text_to_root() {
                let msg = "Cannot save: the JSON has syntax errors. Fix them in Text view first.".to_string();
                self.save_error = Some(msg.clone());
                return Err(msg);
            }
        }
        let content = serialize_value(&self.root, self.export_pretty);
        match std::fs::write(self.file_path.as_ref().unwrap(), &content) {
            Ok(_) => {
                self.dirty = false;
                self.text_modified = false;
                self.save_error = None;
                Ok(())
            }
            Err(e) => {
                let msg = format!("Save failed: {}", e);
                self.save_error = Some(msg.clone());
                Err(msg)
            }
        }
    }

    fn save_as(&mut self) -> Result<(), String> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .add_filter("All Files", &["*"])
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
                (MenuItem {
                    label: "Undo".to_string(),
                    shortcut: Some("Ctrl+Z".to_string()),
                    enabled: self.can_undo(),
                }, MenuAction::Undo),
                (MenuItem {
                    label: "Redo".to_string(),
                    shortcut: Some("Ctrl+Y".to_string()),
                    enabled: self.can_redo(),
                }, MenuAction::Redo),
            ],
            view_items: Vec::new(), image_items: Vec::new(), filter_items: Vec::new(), layer_items: Vec::new(), insert_items: Vec::new(),
        }
    }

    fn handle_menu_action(&mut self, action: MenuAction) -> bool {
        match action {
            MenuAction::Undo => { self.undo(); true }
            MenuAction::Redo => { self.redo(); true }
            _ => false,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, show_toolbar: bool, show_file_info: bool) {
        self.render_editor_ui(ui, ctx, show_toolbar, show_file_info);
    }
}
