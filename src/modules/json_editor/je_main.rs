use eframe::egui;
use serde_json::Value;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use crate::modules::{EditorModule, MenuAction, MenuItem, MenuContribution};
use super::je_tools::{
    SortMode, SearchTarget, FlatNode,
    build_flat, serialize_value, parse_text, expand_recursive, collapse_recursive,
    search_flat, path_key,
};

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
    pub(super) search_cursor: usize,
    pub(super) search_stale: bool,

    pub(super) text_content: String,
    pub(super) text_stale: bool,
    pub(super) text_errors: Vec<(usize, String)>,

    pub(super) undo_stack: VecDeque<Box<Value>>,
    pub(super) redo_stack: VecDeque<Box<Value>>,
    pub(super) undo_limit: usize,

    pub(super) add_dialog: Option<AddKeyDialog>,
    pub(super) edit_cell: Option<EditCell>,
    pub(super) selected_row: Option<usize>,
    pub(super) confirm_delete_path: Option<Vec<String>>,

    pub(super) export_pretty: bool,
    pub(super) show_new_confirm: bool,
}

impl JsonEditor {
    pub fn new_empty() -> Self {
        let root = Value::Object(serde_json::Map::new());
        Self::from_value(root, None)
    }

    pub fn load(path: PathBuf) -> Self {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let root = serde_json::from_str(&content).unwrap_or(Value::Null);
        Self::from_value(root, Some(path))
    }

    fn from_value(root: Value, path: Option<PathBuf>) -> Self {
        let scope_path: Vec<String> = Vec::new();
        let mut expanded = HashSet::new();
        expanded.insert(path_key(&scope_path));

        let flat = build_flat(&root, &scope_path, &expanded, SortMode::None);
        let text_content = serialize_value(&root, true);

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
            search_cursor: 0,
            search_stale: false,
            text_content,
            text_stale: false,
            text_errors: Vec::new(),
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            undo_limit: 20,
            add_dialog: None,
            edit_cell: None,
            selected_row: None,
            confirm_delete_path: None,
            export_pretty: true,
            show_new_confirm: false,
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
            self.search_stale = true;
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
        self.text_errors.clear();
        self.text_stale = false;
    }

    pub(super) fn commit_text_to_root(&mut self) -> bool {
        match parse_text(&self.text_content) {
            Ok(v) => {
                self.push_undo();
                if self.scope_path.is_empty() {
                    self.root = v;
                } else {
                    super::je_tools::set_at_path(&mut self.root, &self.scope_path, v);
                }
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

    pub(super) fn run_search(&mut self) {
        if self.search_stale || self.search_results.is_empty() && !self.search_query.is_empty() {
            self.search_results = search_flat(&self.flat, &self.search_query, self.search_target);
            if self.search_cursor >= self.search_results.len() {
                self.search_cursor = 0;
            }
            self.search_stale = false;
        }
    }

    pub(super) fn search_next(&mut self) {
        if self.search_results.is_empty() { return; }
        self.search_cursor = (self.search_cursor + 1) % self.search_results.len();
    }

    pub(super) fn search_prev(&mut self) {
        if self.search_results.is_empty() { return; }
        if self.search_cursor == 0 {
            self.search_cursor = self.search_results.len() - 1;
        } else {
            self.search_cursor -= 1;
        }
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
        self.edit_cell = None;
        self.add_dialog = None;
    }
}

impl EditorModule for JsonEditor {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn get_title(&self) -> String {
        let name = self.get_file_name();
        if self.dirty { format!("{} *", name) } else { name }
    }

    fn save(&mut self) -> Result<(), String> {
        if self.file_path.is_none() {
            return self.save_as();
        }
        let content = serialize_value(&self.root, self.export_pretty);
        std::fs::write(self.file_path.as_ref().unwrap(), content)
            .map_err(|e| e.to_string())?;
        self.dirty = false;
        Ok(())
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
            view_items: Vec::new(),
            image_items: Vec::new(),
            filter_items: Vec::new(),
        }
    }

    fn handle_menu_action(&mut self, action: MenuAction) -> bool {
        match action {
            MenuAction::Undo => { self.undo(); true }
            MenuAction::Redo => { self.redo(); true }
            _ => false,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, _show_file_info: bool) {
        self.render_editor_ui(ui, ctx);
    }
}
