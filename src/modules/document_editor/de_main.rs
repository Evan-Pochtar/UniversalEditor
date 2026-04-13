use eframe::egui;
use std::collections::VecDeque;
use std::path::PathBuf;
use crate::modules::{EditorModule, MenuAction, MenuItem, MenuContribution};
use super::de_tools::*;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(super) struct DocPos { pub para: usize, pub byte: usize }

pub struct DocumentEditor {
    pub(super) file_path: Option<PathBuf>,
    pub(super) dirty: bool,
    pub(super) paras: Vec<DocParagraph>,
    pub(super) layout: PageLayout,
    pub(super) base_font: FontChoice,
    pub(super) base_size: f32,
    pub(super) cur_fmt: SpanFmt,
    pub(super) focused_para: usize,
    pub(super) last_selection: Option<(usize, usize, usize)>,
    pub(super) pending_focus: Option<usize>,
    pub(super) show_outline: bool,
    pub(super) show_stats: bool,
    pub(super) show_page_settings: bool,
    pub(super) find_text: String,
    pub(super) replace_text: String,
    pub(super) show_find: bool,
    pub(super) find_results: Vec<(usize, usize, usize)>,
    pub(super) find_cursor: usize,
    pub(super) find_stale: bool,
    pub(super) zoom: f32,
    pub(super) auto_zoom_done: bool,
    pub(super) scroll_to_para: Option<usize>,
    pub(super) undo_stack: VecDeque<Vec<DocParagraph>>,
    pub(super) redo_stack: VecDeque<Vec<DocParagraph>>,
    pub(super) para_texts: Vec<String>,
    pub(super) para_ids: Vec<egui::Id>,
    pub(super) para_heights: Vec<f32>,
    pub(super) heights_dirty: bool,
    pub(super) preset_idx: usize,
    pub(super) line_spacing_input: f32,
    pub(super) doc_sel: Option<[DocPos; 2]>,
    pub(super) page_settings_draft: Option<(PageLayout, usize, f32)>,
}

impl DocumentEditor {
    pub fn new_empty() -> Self {
        let mut s = Self::make(vec![DocParagraph::new()], None, PageLayout::default());
        s.sync_texts(); s
    }

    pub fn load(path: PathBuf) -> Self {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let (paras, layout) = if ext == "docx" {
            load_docx(&path).unwrap_or_else(|_| (vec![DocParagraph::new()], PageLayout::default()))
        } else {
            (load_txt_as_doc(&path).unwrap_or_else(|_| vec![DocParagraph::new()]), PageLayout::default())
        };
        let mut s = Self::make(paras, Some(path), layout);
        s.sync_texts(); s
    }

    fn make(paras: Vec<DocParagraph>, path: Option<PathBuf>, layout: PageLayout) -> Self {
        let n = paras.len();
        Self {
            file_path: path, dirty: false, paras, layout,
            base_font: FontChoice::Ubuntu, base_size: 12.0, cur_fmt: SpanFmt::default(),
            focused_para: 0, last_selection: None, pending_focus: None,
            show_outline: false, show_stats: false, show_page_settings: false,
            find_text: String::new(), replace_text: String::new(), show_find: false,
            find_results: Vec::new(), find_cursor: 0, find_stale: false,
            zoom: 1.0, auto_zoom_done: false, scroll_to_para: None,
            undo_stack: VecDeque::new(), redo_stack: VecDeque::new(),
            para_texts: vec![String::new(); n],
            para_ids: (0..n).map(|i| egui::Id::new(("de_para", i as u64))).collect(),
            para_heights: vec![0.0; n], heights_dirty: true,
            preset_idx: 0, line_spacing_input: 1.15,
            doc_sel: None, page_settings_draft: None,
        }
    }

    pub(super) fn sync_texts(&mut self) {
        let n = self.paras.len();
        self.para_texts.resize(n, String::new());
        self.para_ids.resize_with(n, || egui::Id::new(egui::Id::NULL));
        self.para_heights.resize(n, 0.0);
        for i in 0..n {
            self.para_texts[i] = self.paras[i].text.clone();
            self.para_ids[i] = egui::Id::new(("de_para", i as u64));
        }
        self.heights_dirty = true;
    }

    pub(super) fn push_undo(&mut self) {
        self.redo_stack.clear();
        self.undo_stack.push_back(self.paras.clone());
        if self.undo_stack.len() > 50 { self.undo_stack.pop_front(); }
    }

    pub(super) fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(self.paras.clone());
            self.paras = prev; self.dirty = true; self.sync_texts(); self.find_stale = true;
        }
    }

    pub(super) fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(self.paras.clone());
            self.paras = next; self.dirty = true; self.sync_texts(); self.find_stale = true;
        }
    }

    pub fn is_dirty(&self) -> bool { self.dirty }

    pub(super) fn norm_sel(&self) -> Option<(DocPos, DocPos)> {
        let [a, b] = self.doc_sel?;
        if a.para < b.para || (a.para == b.para && a.byte <= b.byte) { Some((a, b)) } else { Some((b, a)) }
    }

    pub(super) fn has_cross_sel(&self) -> bool {
        self.doc_sel.map(|[a, b]| a.para != b.para).unwrap_or(false)
    }

    pub(super) fn delete_sel(&mut self) {
        let (from, to) = match self.norm_sel() { Some(r) => r, None => return };
        self.push_undo();
        let n = self.paras.len();
        let prefix = self.paras[from.para].text[..from.byte.min(self.paras[from.para].text.len())].to_string();
        let suffix = if to.para < n { self.paras[to.para].text[to.byte.min(self.paras[to.para].text.len())..].to_string() } else { String::new() };
        
        if self.paras[from.para].style == ParaStyle::HRule {
            if to.para < n && self.paras[to.para].style != ParaStyle::HRule {
                self.paras[from.para].style = self.paras[to.para].style;
                self.paras[from.para].align = self.paras[to.para].align;
                self.paras[from.para].indent_left = self.paras[to.para].indent_left;
            } else {
                self.paras[from.para].style = ParaStyle::Normal;
            }
        }

        rebuild_spans(&mut self.paras[from.para], format!("{}{}", prefix, suffix), &self.cur_fmt);
        if to.para > from.para && to.para < n {
            self.paras.drain(from.para + 1..=to.para.min(n - 1));
        }
        self.doc_sel = None;
        self.focused_para = from.para.min(self.paras.len().saturating_sub(1));
        self.pending_focus = Some(self.focused_para);
        self.sync_texts();
        self.dirty = true; self.heights_dirty = true; self.find_stale = true;
    }

    pub(super) fn run_find(&mut self) {
        if !self.find_stale { return; }
        self.find_results.clear();
        if !self.find_text.is_empty() {
            let q = self.find_text.to_lowercase();
            for (pi, para) in self.paras.iter().enumerate() {
                let hay = para.text.to_lowercase();
                let mut off = 0;
                while let Some(pos) = hay[off..].find(&q) {
                    let s = off + pos; let e = s + self.find_text.len();
                    self.find_results.push((pi, s, e)); off = e;
                }
            }
        }
        if self.find_cursor >= self.find_results.len() { self.find_cursor = 0; }
        self.find_stale = false;
    }

    pub(super) fn find_next(&mut self) {
        let n = self.find_results.len(); if n == 0 { return; }
        self.find_cursor = (self.find_cursor + 1) % n;
        self.scroll_to_para = Some(self.find_results[self.find_cursor].0);
    }

    pub(super) fn find_prev(&mut self) {
        let n = self.find_results.len(); if n == 0 { return; }
        self.find_cursor = if self.find_cursor == 0 { n - 1 } else { self.find_cursor - 1 };
        self.scroll_to_para = Some(self.find_results[self.find_cursor].0);
    }

    pub(super) fn sel_font_size_pt(&self) -> f32 {
        if let Some((pi, s, e)) = self.last_selection {
            if pi < self.paras.len() {
                let byte = if s == e { s } else { s };
                if let Some(hp) = para_fmt_at(&self.paras[pi], byte).size_hp { return hp as f32 / 2.0; }
            }
        }
        let pi = self.focused_para.min(self.paras.len().saturating_sub(1));
        para_fmt_at(&self.paras[pi], 0).size_hp.map(|hp| hp as f32 / 2.0).unwrap_or(self.base_size)
    }

    pub(super) fn apply_fmt_size(&mut self, pt: f32) {
        let hp = (pt * 2.0).round() as u32;
        if let Some((pi, s, e)) = self.last_selection {
            if s != e && pi < self.paras.len() {
                self.push_undo();
                apply_fmt_range(&mut self.paras[pi], s, e, |f| f.size_hp = Some(hp));
                self.para_texts[pi] = self.paras[pi].text.clone();
                self.dirty = true; self.heights_dirty = true; return;
            }
        }
        let pi = self.focused_para.min(self.paras.len().saturating_sub(1));
        self.push_undo();
        let len = self.paras[pi].text.len();
        if len > 0 {
            apply_fmt_range(&mut self.paras[pi], 0, len, |f| f.size_hp = Some(hp));
            self.para_texts[pi] = self.paras[pi].text.clone();
        }
        self.cur_fmt.size_hp = Some(hp);
        self.dirty = true; self.heights_dirty = true;
    }

    pub(super) fn replace_current(&mut self) {
        if self.find_results.is_empty() { return; }
        let (pi, s, e) = self.find_results[self.find_cursor];
        if pi >= self.paras.len() { return; }
        let rep = self.replace_text.clone();
        let new_text = format!("{}{}{}", &self.paras[pi].text[..s], rep, &self.paras[pi].text[e..]);
        rebuild_spans(&mut self.paras[pi], new_text.clone(), &self.cur_fmt);
        self.para_texts[pi] = new_text;
        self.dirty = true; self.heights_dirty = true; self.find_stale = true; self.run_find();
    }

    pub(super) fn replace_all(&mut self) {
        if self.find_text.is_empty() { return; }
        self.push_undo();
        let q = self.find_text.clone(); let rep = self.replace_text.clone();
        for i in 0..self.paras.len() {
            if self.paras[i].text.contains(&q) {
                let new_text = self.paras[i].text.replace(&q, &rep);
                rebuild_spans(&mut self.paras[i], new_text.clone(), &self.cur_fmt);
                self.para_texts[i] = new_text;
            }
        }
        self.dirty = true; self.heights_dirty = true; self.find_stale = true; self.run_find();
    }

    pub(super) fn apply_style(&mut self, style: ParaStyle) {
        self.push_undo();
        let p = &mut self.paras[self.focused_para];
        p.style = style; p.space_before = style.space_before(); p.space_after = style.space_after(); p.indent_left = style.default_indent();
        self.dirty = true; self.heights_dirty = true;
    }

    pub(super) fn apply_style_toggle(&mut self, style: ParaStyle) {
        let cur = self.paras[self.focused_para].style;
        self.apply_style(if cur == style { ParaStyle::Normal } else { style });
    }

    pub(super) fn apply_align(&mut self, align: Align) {
        self.paras[self.focused_para].align = align; self.dirty = true;
    }

    pub(super) fn indent_para(&mut self, idx: usize, delta: f32) {
        self.paras[idx].indent_left = (self.paras[idx].indent_left + delta).max(0.0).min(200.0);
        self.dirty = true; self.heights_dirty = true;
    }

    pub(super) fn apply_fmt_toggle_bold(&mut self) {
        if let Some((pi, s, e)) = self.last_selection {
            if s != e && pi < self.paras.len() {
                self.push_undo();
                toggle_fmt(&mut self.paras[pi], s, e, |f| f.bold, |f, v| f.bold = v);
                self.para_texts[pi] = self.paras[pi].text.clone();
                self.dirty = true; self.heights_dirty = true; return;
            }
        }
        self.cur_fmt.bold = !self.cur_fmt.bold;
    }

    pub(super) fn apply_fmt_toggle_italic(&mut self) {
        if let Some((pi, s, e)) = self.last_selection {
            if s != e && pi < self.paras.len() {
                self.push_undo();
                toggle_fmt(&mut self.paras[pi], s, e, |f| f.italic, |f, v| f.italic = v);
                self.para_texts[pi] = self.paras[pi].text.clone();
                self.dirty = true; self.heights_dirty = true; return;
            }
        }
        self.cur_fmt.italic = !self.cur_fmt.italic;
    }

    pub(super) fn apply_fmt_toggle_underline(&mut self) {
        if let Some((pi, s, e)) = self.last_selection {
            if s != e && pi < self.paras.len() {
                self.push_undo();
                toggle_fmt(&mut self.paras[pi], s, e, |f| f.underline, |f, v| f.underline = v);
                self.para_texts[pi] = self.paras[pi].text.clone();
                self.dirty = true; self.heights_dirty = true; return;
            }
        }
        self.cur_fmt.underline = !self.cur_fmt.underline;
    }

    pub(super) fn apply_fmt_toggle_strike(&mut self) {
        if let Some((pi, s, e)) = self.last_selection {
            if s != e && pi < self.paras.len() {
                self.push_undo();
                toggle_fmt(&mut self.paras[pi], s, e, |f| f.strike, |f, v| f.strike = v);
                self.para_texts[pi] = self.paras[pi].text.clone();
                self.dirty = true; self.heights_dirty = true; return;
            }
        }
        self.cur_fmt.strike = !self.cur_fmt.strike;
    }

    pub(super) fn apply_fmt_toggle_sup(&mut self) {
        if let Some((pi, s, e)) = self.last_selection {
            if s != e && pi < self.paras.len() {
                self.push_undo();
                let v = !all_set_range(&self.paras[pi], s, e, |f| f.sup);
                apply_fmt_range(&mut self.paras[pi], s, e, |f| { f.sup = v; if v { f.sub = false; } });
                self.para_texts[pi] = self.paras[pi].text.clone();
                self.dirty = true; self.heights_dirty = true; return;
            }
        }
        if self.cur_fmt.sup { self.cur_fmt.sup = false; } else { self.cur_fmt.sup = true; self.cur_fmt.sub = false; }
    }

    pub(super) fn apply_fmt_toggle_sub(&mut self) {
        if let Some((pi, s, e)) = self.last_selection {
            if s != e && pi < self.paras.len() {
                self.push_undo();
                let v = !all_set_range(&self.paras[pi], s, e, |f| f.sub);
                apply_fmt_range(&mut self.paras[pi], s, e, |f| { f.sub = v; if v { f.sup = false; } });
                self.para_texts[pi] = self.paras[pi].text.clone();
                self.dirty = true; self.heights_dirty = true; return;
            }
        }
        if self.cur_fmt.sub { self.cur_fmt.sub = false; } else { self.cur_fmt.sub = true; self.cur_fmt.sup = false; }
    }

    pub(super) fn fmt_state_bold(&self) -> bool {
        if let Some((pi, s, e)) = self.last_selection { if s != e && pi < self.paras.len() { return all_set_range(&self.paras[pi], s, e, |f| f.bold); } }
        self.cur_fmt.bold
    }
    pub(super) fn fmt_state_italic(&self) -> bool {
        if let Some((pi, s, e)) = self.last_selection { if s != e && pi < self.paras.len() { return all_set_range(&self.paras[pi], s, e, |f| f.italic); } }
        self.cur_fmt.italic
    }
    pub(super) fn fmt_state_underline(&self) -> bool {
        if let Some((pi, s, e)) = self.last_selection { if s != e && pi < self.paras.len() { return all_set_range(&self.paras[pi], s, e, |f| f.underline); } }
        self.cur_fmt.underline
    }
    pub(super) fn fmt_state_strike(&self) -> bool {
        if let Some((pi, s, e)) = self.last_selection { if s != e && pi < self.paras.len() { return all_set_range(&self.paras[pi], s, e, |f| f.strike); } }
        self.cur_fmt.strike
    }
    pub(super) fn fmt_state_sup(&self) -> bool {
        if let Some((pi, s, e)) = self.last_selection { if s != e && pi < self.paras.len() { return all_set_range(&self.paras[pi], s, e, |f| f.sup); } }
        self.cur_fmt.sup
    }
    pub(super) fn fmt_state_sub(&self) -> bool {
        if let Some((pi, s, e)) = self.last_selection { if s != e && pi < self.paras.len() { return all_set_range(&self.paras[pi], s, e, |f| f.sub); } }
        self.cur_fmt.sub
    }

    pub(super) fn apply_fmt_color(&mut self, color: Option<[u8; 3]>) {
        if let Some((pi, s, e)) = self.last_selection {
            if s != e && pi < self.paras.len() {
                self.push_undo();
                apply_fmt_range(&mut self.paras[pi], s, e, |f| f.color = color);
                self.para_texts[pi] = self.paras[pi].text.clone();
                self.dirty = true; self.heights_dirty = true; return;
            }
        }
        self.cur_fmt.color = color;
    }

    fn save_impl(&mut self, path: PathBuf) -> Result<(), String> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext == "docx" { save_docx(&path, &self.paras, &self.layout)?; }
        else { let t: String = self.paras.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n"); std::fs::write(&path, t).map_err(|e| e.to_string())?; }
        self.file_path = Some(path); self.dirty = false; Ok(())
    }
}

impl EditorModule for DocumentEditor {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_title(&self) -> String {
        let name = self.file_path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("Untitled").to_string();
        if self.dirty { format!("{} *", name) } else { name }
    }
    fn save(&mut self) -> Result<(), String> {
        if let Some(p) = self.file_path.clone() { self.save_impl(p) } else { self.save_as() }
    }
    fn save_as(&mut self) -> Result<(), String> {
        if let Some(path) = rfd::FileDialog::new().add_filter("Word Document", &["docx"]).add_filter("Text", &["txt"]).save_file() { self.save_impl(path) }
        else { Err("Cancelled".to_string()) }
    }
    fn get_menu_contributions(&self) -> MenuContribution {
        MenuContribution {
            file_items: vec![
                (MenuItem { label: "Find & Replace...".into(), shortcut: Some("Ctrl+F".into()), enabled: true }, MenuAction::Custom("Find".into())),
                (MenuItem { label: "Document Statistics".into(), shortcut: None, enabled: true }, MenuAction::Custom("Stats".into())),
                (MenuItem { label: "Page Settings...".into(), shortcut: None, enabled: true }, MenuAction::Custom("PageSettings".into())),
            ],
            edit_items: vec![
                (MenuItem { label: "Undo".into(), shortcut: Some("Ctrl+Z".into()), enabled: !self.undo_stack.is_empty() }, MenuAction::Undo),
                (MenuItem { label: "Redo".into(), shortcut: Some("Ctrl+Y".into()), enabled: !self.redo_stack.is_empty() }, MenuAction::Redo),
            ],
            view_items: vec![
                (MenuItem { label: if self.show_outline { "Hide Outline".into() } else { "Show Outline".into() }, shortcut: None, enabled: true }, MenuAction::Custom("ToggleOutline".into())),
                (MenuItem { label: "Zoom In".into(), shortcut: Some("Ctrl++".into()), enabled: true }, MenuAction::Custom("ZoomIn".into())),
                (MenuItem { label: "Zoom Out".into(), shortcut: Some("Ctrl+-".into()), enabled: true }, MenuAction::Custom("ZoomOut".into())),
                (MenuItem { label: "Reset Zoom".into(), shortcut: Some("Ctrl+0".into()), enabled: true }, MenuAction::Custom("ZoomReset".into())),
            ],
            image_items: vec![], filter_items: vec![], layer_items: vec![],
        }
    }
    fn handle_menu_action(&mut self, action: MenuAction) -> bool {
        match action {
            MenuAction::Undo => { self.undo(); true }
            MenuAction::Redo => { self.redo(); true }
            MenuAction::Custom(ref v) => match v.as_str() {
                "Find" => { self.show_find = true; true }
                "Stats" => { self.show_stats = true; true }
                "PageSettings" => { self.page_settings_draft = None; self.show_page_settings = true; true }
                "ToggleOutline" => { self.show_outline = !self.show_outline; true }
                "ZoomIn" => { self.zoom = (self.zoom + 0.1).min(3.0); true }
                "ZoomOut" => { self.zoom = (self.zoom - 0.1).max(0.3); true }
                "ZoomReset" => { self.auto_zoom_done = false; true }
                _ => false,
            },
            _ => false,
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, _show_file_info: bool) {
        super::de_ui::render(self, ui, ctx);
    }
}
