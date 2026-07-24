use eframe::egui;
use std::collections::VecDeque;
use std::path::PathBuf;
use ahash::AHashMap;
use crate::modules::{EditorModule, MenuAction, MenuItem, MenuContribution};
use super::se_model::{Sheet, CellFmt};
use super::se_formula::Cache;
use super::{se_io, se_tools};

#[derive(Clone, Copy, PartialEq)]
pub enum SheetViewMode { Table, Text }

pub struct SpreadsheetEditor {
    pub(super) file_path: Option<PathBuf>,
    pub(super) dirty: bool,
    pub(super) sheets: Vec<Sheet>,
    pub(super) active_sheet: usize,
    pub(super) view_mode: SheetViewMode,
    pub(super) sel: Option<((u32,u32),(u32,u32))>,
    pub(super) editing: Option<(u32,u32,String)>,
    pub(super) cur_fmt: CellFmt,
    pub(super) text_content: String,
    pub(super) text_dirty: bool,
    pub(super) undo_stack: VecDeque<Vec<Sheet>>,
    pub(super) redo_stack: VecDeque<Vec<Sheet>>,
    pub(super) calc_cache: Cache,
    pub(super) calc_dirty: bool,
    pub(super) search_query: String,
    pub(super) search_results: Vec<(u32,u32)>,
    pub(super) search_cursor: usize,
    pub(super) rename_buf: Option<(usize, String)>,
    pub(super) scroll_to: Option<(u32,u32)>,
}

impl SpreadsheetEditor {
    pub fn new_empty() -> Self { Self::make(vec![Sheet::new("Sheet1")], None) }

    pub fn load(path: PathBuf) -> Self {
        let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
        let sheets = match ext.as_str() {
            "csv" => vec![se_io::load_delim(&path, b',')],
            "tsv" => vec![se_io::load_delim(&path, b'\t')],
            _ => se_io::load_workbook(&path),
        };
        Self::make(sheets, Some(path))
    }

    fn make(sheets: Vec<Sheet>, path: Option<PathBuf>) -> Self {
        Self {
            file_path: path, dirty: false, sheets, active_sheet: 0, view_mode: SheetViewMode::Table,
            sel: Some(((0,0),(0,0))), editing: None, cur_fmt: CellFmt::default(),
            text_content: String::new(), text_dirty: true,
            undo_stack: VecDeque::new(), redo_stack: VecDeque::new(),
            calc_cache: AHashMap::default(), calc_dirty: true,
            search_query: String::new(), search_results: Vec::new(), search_cursor: 0,
            rename_buf: None, scroll_to: None,
        }
    }

    pub fn is_dirty(&self) -> bool { self.dirty }
    pub(super) fn sheet(&self) -> &Sheet { &self.sheets[self.active_sheet] }
    pub(super) fn sheet_mut(&mut self) -> &mut Sheet { &mut self.sheets[self.active_sheet] }
    pub(super) fn sheet_raw(&self, r: u32, c: u32) -> String { self.sheet().raw(r,c).to_string() }
    pub(super) fn sheet_cell_count(&self) -> usize { self.sheet().cells.len() }
    pub(super) fn file_name(&self) -> String { self.file_path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).map(|s| s.to_string()).unwrap_or_else(|| "Untitled.csv".to_string()) }

    pub(super) fn push_undo(&mut self) { self.redo_stack.clear(); self.undo_stack.push_back(self.sheets.clone()); if self.undo_stack.len() > 20 { self.undo_stack.pop_front(); } }
    pub(super) fn undo(&mut self) { if let Some(p) = self.undo_stack.pop_back() { self.redo_stack.push_back(self.sheets.clone()); self.sheets = p; self.after_mutate(); } }
    pub(super) fn redo(&mut self) { if let Some(n) = self.redo_stack.pop_back() { self.undo_stack.push_back(self.sheets.clone()); self.sheets = n; self.after_mutate(); } }
    fn after_mutate(&mut self) { self.dirty = true; self.calc_dirty = true; self.text_dirty = true; self.active_sheet = self.active_sheet.min(self.sheets.len()-1); }

    pub(super) fn begin_edit(&mut self, r: u32, c: u32, initial: Option<String>) {
        self.push_undo();
        let raw = initial.unwrap_or_else(|| self.sheet().raw(r,c).to_string());
        self.editing = Some((r, c, raw));
    }
    pub(super) fn commit_edit(&mut self) { if let Some((r,c,val)) = self.editing.take() { self.sheet_mut().set_raw(r,c,val); self.dirty = true; self.calc_dirty = true; self.text_dirty = true; } }
    pub(super) fn cancel_edit(&mut self) { if self.editing.take().is_some() { self.undo_stack.pop_back(); } }
    pub(super) fn set_cell(&mut self, r: u32, c: u32, val: String) { self.push_undo(); self.sheet_mut().set_raw(r, c, val); self.dirty = true; self.calc_dirty = true; self.text_dirty = true; }

    pub(super) fn clear_selection(&mut self) {
        let Some(((r0,c0),(r1,c1))) = self.sel else { return };
        let (rr0,rr1) = (r0.min(r1), r0.max(r1)); let (cc0,cc1) = (c0.min(c1), c0.max(c1));
        self.push_undo();
        for r in rr0..=rr1 { for c in cc0..=cc1 { self.sheet_mut().set_raw(r, c, String::new()); } }
        self.dirty = true; self.calc_dirty = true; self.text_dirty = true;
    }

    pub(super) fn apply_fmt(&mut self, f: impl Fn(&mut CellFmt)) {
        let Some(((r0,c0),(r1,c1))) = self.sel else { return };
        let (rr0,rr1) = (r0.min(r1), r0.max(r1)); let (cc0,cc1) = (c0.min(c1), c0.max(c1));
        self.push_undo();
        let sheet = self.sheet_mut();
        for r in rr0..=rr1 { for c in cc0..=cc1 { f(&mut sheet.cells.entry((r,c)).or_default().fmt); } }
        self.dirty = true; self.calc_dirty = true;
    }

    pub(super) fn insert_row(&mut self, at: u32) { self.push_undo(); se_tools::insert_row(self.sheet_mut(), at); self.dirty = true; self.calc_dirty = true; self.text_dirty = true; }
    pub(super) fn delete_row(&mut self, at: u32) { self.push_undo(); se_tools::delete_row(self.sheet_mut(), at); self.dirty = true; self.calc_dirty = true; self.text_dirty = true; }
    pub(super) fn insert_col(&mut self, at: u32) { self.push_undo(); se_tools::insert_col(self.sheet_mut(), at); self.dirty = true; self.calc_dirty = true; self.text_dirty = true; }
    pub(super) fn delete_col(&mut self, at: u32) { self.push_undo(); se_tools::delete_col(self.sheet_mut(), at); self.dirty = true; self.calc_dirty = true; self.text_dirty = true; }
    pub(super) fn sort_col(&mut self, col: u32, asc: bool) { self.push_undo(); se_tools::sort_by_column(self.sheet_mut(), col, asc, true); self.dirty = true; self.calc_dirty = true; self.text_dirty = true; }

    pub(super) fn copy_selection(&self, ctx: &egui::Context) {
        if let Some(((r0,c0),(r1,c1))) = self.sel {
            let (rr0,rr1) = (r0.min(r1), r0.max(r1)); let (cc0,cc1) = (c0.min(c1), c0.max(c1));
            ctx.copy_text(se_tools::range_to_tsv(self.sheet(), rr0, cc0, rr1, cc1));
        }
    }
    pub(super) fn paste_text(&mut self, text: &str) {
        let Some(((r0,c0),_)) = self.sel else { return };
        self.push_undo();
        se_tools::paste_tsv(self.sheet_mut(), r0, c0, text);
        self.dirty = true; self.calc_dirty = true; self.text_dirty = true;
    }

    pub(super) fn add_sheet(&mut self) { self.push_undo(); let n = self.sheets.len()+1; self.sheets.push(Sheet::new(format!("Sheet{}", n))); self.active_sheet = self.sheets.len()-1; self.calc_dirty = true; self.text_dirty = true; self.dirty = true; }
    pub(super) fn delete_sheet(&mut self, idx: usize) { if self.sheets.len()<=1 { return; } self.push_undo(); self.sheets.remove(idx); self.active_sheet = self.active_sheet.min(self.sheets.len()-1); self.calc_dirty = true; self.text_dirty = true; self.dirty = true; }
    pub(super) fn switch_sheet(&mut self, idx: usize) { if idx < self.sheets.len() { self.active_sheet = idx; self.calc_dirty = true; self.text_dirty = true; } }

    pub(super) fn sync_text(&mut self) {
        let sheet = self.sheet();
        let mut out = String::new();
        for r in 0..=sheet.max_used_row() {
            for c in 0..=sheet.max_used_col() {
                if c > 0 { out.push(','); }
                let raw = sheet.raw(r,c);
                if raw.contains(',') || raw.contains('"') || raw.contains('\n') { out.push('"'); out.push_str(&raw.replace('"',"\"\"")); out.push('"'); } else { out.push_str(raw); }
            }
            out.push('\n');
        }
        self.text_content = out; self.text_dirty = false;
    }

    pub(super) fn commit_text(&mut self) {
        self.push_undo();
        let old_fmts: AHashMap<(u32,u32), CellFmt> = self.sheet().cells.iter().map(|(&k,v)| (k, v.fmt.clone())).collect();
        let mut sheet = Sheet::new(self.sheet().name.clone());
        let mut rdr = csv::ReaderBuilder::new().has_headers(false).flexible(true).from_reader(self.text_content.as_bytes());
        for (r, rec) in rdr.records().enumerate() { if let Ok(rec) = rec { for (c, field) in rec.iter().enumerate() { if !field.is_empty() { sheet.set_raw(r as u32, c as u32, field.to_string()); } } } }
        for (k, fmt) in old_fmts { sheet.cells.entry(k).or_default().fmt = fmt; }
        self.sheets[self.active_sheet] = sheet;
        self.dirty = true; self.calc_dirty = true;
    }

pub(super) fn run_search(&mut self) {
        self.search_results.clear(); self.search_cursor = 0;
        if self.search_query.is_empty() { return; }
        let q = self.search_query.to_lowercase();
        let mut results: Vec<(u32,u32)> = self.sheet().cells.iter().filter(|(_,c)| c.raw.to_lowercase().contains(&q)).map(|(&k,_)| k).collect();
        results.sort();
        self.search_results = results;
        if let Some(&(r,c)) = self.search_results.first() { self.sel = Some(((r,c),(r,c))); self.scroll_to = Some((r,c)); }
    }
    pub(super) fn search_next(&mut self) {
        if self.search_results.is_empty() { return; }
        self.search_cursor = (self.search_cursor+1) % self.search_results.len();
        let (r,c) = self.search_results[self.search_cursor]; self.sel = Some(((r,c),(r,c))); self.scroll_to = Some((r,c));
    }
    pub(super) fn search_prev(&mut self) {
        if self.search_results.is_empty() { return; }
        self.search_cursor = if self.search_cursor==0 {self.search_results.len()-1} else {self.search_cursor-1};
        let (r,c) = self.search_results[self.search_cursor]; self.sel = Some(((r,c),(r,c))); self.scroll_to = Some((r,c));
    }

    fn save_impl(&mut self, path: PathBuf) -> Result<(), String> {
        if matches!(self.view_mode, SheetViewMode::Text) { self.commit_text(); }
        let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
        match ext.as_str() {
            "tsv" => se_io::save_delim(&self.sheets, &path, b'\t'),
            "ods" => se_io::save_ods(&self.sheets, &path),
            "xlsx" => se_io::save_xlsx(&self.sheets, &path),
            _ => se_io::save_delim(&self.sheets, &path, b','),
        }?;
        self.file_path = Some(path); self.dirty = false;
        Ok(())
    }
}

impl EditorModule for SpreadsheetEditor {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_title(&self) -> String { let n = self.file_name(); if self.dirty { format!("{} *", n) } else { n } }

    fn save(&mut self) -> Result<(), String> {
        match self.file_path.clone() {
            Some(p) if !p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("xls")).unwrap_or(false) => self.save_impl(p),
            _ => self.save_as(),
        }
    }
    fn save_as(&mut self) -> Result<(), String> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"]).add_filter("TSV", &["tsv"])
            .add_filter("Excel Workbook", &["xlsx"]).add_filter("OpenDocument Spreadsheet", &["ods"])
            .save_file()
        { self.save_impl(path) } else { Err("Cancelled".to_string()) }
    }

    fn get_menu_contributions(&self) -> MenuContribution {
        MenuContribution {
            edit_items: vec![
                (MenuItem { label: "Undo".into(), shortcut: Some("Ctrl+Z".into()), enabled: !self.undo_stack.is_empty() }, MenuAction::Undo),
                (MenuItem { label: "Redo".into(), shortcut: Some("Ctrl+Y".into()), enabled: !self.redo_stack.is_empty() }, MenuAction::Redo),
            ],
            ..Default::default()
        }
    }
    fn handle_menu_action(&mut self, action: MenuAction) -> bool { match action { MenuAction::Undo => { self.undo(); true } MenuAction::Redo => { self.redo(); true } _ => false } }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, show_toolbar: bool, show_file_info: bool) {
        super::se_ui::render(self, ui, ctx, show_toolbar, show_file_info);
    }
}
