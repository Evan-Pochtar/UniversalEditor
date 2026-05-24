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
    pub(super) focus_find: bool,
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
    pub(super) link_input: String,
    pub(super) doc_sel: Option<[DocPos; 2]>,
    pub(super) page_settings_draft: Option<(PageLayout, usize, String, String, String)>,
    pub(super) last_edit_action: u8,
    pub(super) table_picker_hover: (usize, usize),
    pub(super) active_table: Option<(usize, usize, usize)>,
    pub(super) table_sel: Option<(usize, (usize, usize), (usize, usize))>,
    pub(super) table_multi_sel: Option<(usize, Vec<(usize, usize)>)>,
    pub(super) table_text_sel: Option<(usize, usize, usize, usize, usize)>,
    pub(super) cell_edit_buf: String,
    pub(super) image_textures: std::collections::HashMap<u64, egui::TextureId>,
    pub(super) selected_image_para: Option<usize>,
    pub(super) image_drag: Option<(usize, u8, egui::Pos2, f32, f32, f32)>,
    pub(super) next_image_uid: u64,
    pub(super) toolbar_has_focus: bool,
    pub pending_open_in_image_editor: Option<Vec<u8>>,
    pub(super) ctx_sel: Option<(usize, usize, usize)>,
    pub(super) ctx_link_show: bool,
}

impl DocumentEditor {
    pub fn new_empty() -> Self {
        let mut s = Self::make(vec![DocParagraph::new()], None, PageLayout::default());
        s.sync_texts(); s
    }

    pub fn load(path: PathBuf) -> Self {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let (paras, layout) = match ext.as_str() {
            "docx" | "doc" => load_docx(&path).unwrap_or_else(|_| (vec![DocParagraph::new()], PageLayout::default())),
            "odt" => load_odt(&path).unwrap_or_else(|_| (vec![DocParagraph::new()], PageLayout::default())),
            _ => (load_txt_as_doc(&path).unwrap_or_else(|_| vec![DocParagraph::new()]), PageLayout::default()),
        };
        let mut s = Self::make(paras, Some(path), layout);
        let mut uid = 0u64;
        for p in &mut s.paras { if let Some(ref mut img) = p.image { img.uid = uid; uid += 1; } }
        s.next_image_uid = uid;
        s.sync_texts(); s
    }

    fn make(paras: Vec<DocParagraph>, path: Option<PathBuf>, layout: PageLayout) -> Self {
        let n = paras.len(); let preset_idx = layout.preset_idx();
        Self {
            file_path: path, dirty: false, paras, layout, cur_fmt: SpanFmt::default(),
            focused_para: 0, last_selection: None, pending_focus: None,
            show_outline: false, show_stats: false, show_page_settings: false,
            find_text: String::new(), replace_text: String::new(), show_find: false, focus_find: false,
            find_results: Vec::new(), find_cursor: 0, find_stale: false,
            zoom: 1.0, auto_zoom_done: false, scroll_to_para: None,
            undo_stack: VecDeque::new(), redo_stack: VecDeque::new(),
            para_texts: vec![String::new(); n],
            para_ids: (0..n).map(|i| egui::Id::new(("de_para", i as u64))).collect(),
            para_heights: vec![0.0; n], heights_dirty: true,
            preset_idx, line_spacing_input: 1.15, link_input: String::new(),
            doc_sel: None, page_settings_draft: None, last_edit_action: 0,
            table_picker_hover: (0, 0), active_table: None, table_sel: None, table_multi_sel: None, table_text_sel: None, cell_edit_buf: String::new(),
            image_textures: std::collections::HashMap::new(), selected_image_para: None, image_drag: None, next_image_uid: 0,
            toolbar_has_focus: false, pending_open_in_image_editor: None, ctx_sel: None, ctx_link_show: false,
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
        self.last_edit_action = 0;
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

    pub(super) fn collect_sel_text(&self, from: DocPos, to: DocPos) -> String {
        if from.para == to.para {
            let len = self.paras[from.para].text.len();
            let s = from.byte.min(len); let e = to.byte.min(len);
            return self.paras[from.para].text[s..e].to_string();
        }
        let mut parts = Vec::new();
        parts.push(self.paras[from.para].text[from.byte.min(self.paras[from.para].text.len())..].to_string());
        for i in (from.para + 1)..to.para {
            if i < self.paras.len() { parts.push(self.paras[i].text.clone()); }
        }
        if to.para < self.paras.len() {
            parts.push(self.paras[to.para].text[..to.byte.min(self.paras[to.para].text.len())].to_string());
        }
        parts.join("\n")
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
            } else { self.paras[from.para].style = ParaStyle::Normal; }
        }

        rebuild_spans(&mut self.paras[from.para], format!("{}{}", prefix, suffix), &self.cur_fmt);
        if to.para > from.para && to.para < n { self.paras.drain(from.para + 1..=to.para.min(n - 1)); }
        self.doc_sel = None;
        self.table_sel = None; self.table_multi_sel = None;
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

    pub(super) fn sel_font_size_pt(&self) -> u32 {
        if let Some((pi, s, e)) = self.last_selection {
            if pi < self.paras.len() {
                let byte = if s == e { s } else { s };
                if let Some(hp) = para_fmt_at(&self.paras[pi], byte).size_hp { return (hp as f32 / 2.0).round() as u32; }
            }
        }
        let pi = self.focused_para.min(self.paras.len().saturating_sub(1));
        para_fmt_at(&self.paras[pi], 0).size_hp.map(|hp| (hp as f32 / 2.0).round() as u32).unwrap_or_else(|| self.paras[pi].style.default_font_size_pt())
    }

    pub(super) fn apply_fmt_property<F: Fn(&mut SpanFmt)>(&mut self, op: F) -> bool {
        if self.table_text_sel.map(|(_, _, _, s, e)| s < e).unwrap_or(false) {
            return self.apply_fmt_to_table_text_sel_fn(op);
        }
        if self.table_sel.is_some() || self.table_multi_sel.as_ref().map_or(false, |(_, v)| !v.is_empty()) {
            return self.apply_fmt_to_table_sel_fn(op);
        }
        if self.has_cross_sel() {
            if let Some((from, to)) = self.norm_sel() {
                if from.para != to.para || from.byte != to.byte {
                    self.push_undo();
                    for pi in from.para..=to.para {
                        if pi >= self.paras.len() { break; }
                        let start = if pi == from.para { from.byte } else { 0 };
                        let end = if pi == to.para { to.byte } else { self.paras[pi].text.len() };
                        if start < end {
                            apply_fmt_range(&mut self.paras[pi], start, end, &op);
                            self.para_texts[pi] = self.paras[pi].text.clone();
                        } else if self.paras[pi].text.is_empty() {
                            if let Some(span) = self.paras[pi].spans.first_mut() { op(&mut span.fmt); }
                        }
                    }
                    self.dirty = true; self.heights_dirty = true; return true;
                }
            }
        }
        
        if let Some((pi, s, e)) = self.last_selection {
            if s != e && pi < self.paras.len() {
                self.push_undo();
                apply_fmt_range(&mut self.paras[pi], s, e, &op);
                self.para_texts[pi] = self.paras[pi].text.clone();
                self.dirty = true; self.heights_dirty = true; return true;
            }
        }
        false
    }

    pub(super) fn apply_fmt_toggle<Get, Set>(&mut self, get: Get, set: Set) where Get: Fn(&SpanFmt) -> bool, Set: Fn(&mut SpanFmt, bool) {
        if self.table_text_sel.map(|(_, _, _, s, e)| s < e).unwrap_or(false) {
            let enabled = !self.table_text_all_set_in_sel(get);
            set(&mut self.cur_fmt, enabled);
            self.apply_fmt_to_table_text_sel_fn(move |f| set(f, enabled));
            return;
        }
        if self.table_sel.is_some() || self.table_multi_sel.as_ref().map_or(false, |(_, v)| !v.is_empty()) {
            let enabled = !self.table_all_set_in_sel(get);
            set(&mut self.cur_fmt, enabled);
            self.apply_fmt_to_table_sel_fn(move |f| set(f, enabled));
            return;
        }
        if self.has_cross_sel() {
            if let Some((from, to)) = self.norm_sel() {
                if from.para != to.para || from.byte != to.byte {
                    self.push_undo();
                    let mut enabled = false;
                    for pi in from.para..=to.para {
                        if pi >= self.paras.len() { break; }
                        let start = if pi == from.para { from.byte } else { 0 };
                        let end = if pi == to.para { to.byte } else { self.paras[pi].text.len() };
                        if start < end {
                            if !all_set_range(&self.paras[pi], start, end, &get) { enabled = true; break; }
                        } else if self.paras[pi].text.is_empty() {
                            if let Some(span) = self.paras[pi].spans.first() { if !get(&span.fmt) { enabled = true; break; } }
                        }
                    }
                    
                    for pi in from.para..=to.para {
                        if pi >= self.paras.len() { break; }
                        let start = if pi == from.para { from.byte } else { 0 };
                        let end = if pi == to.para { to.byte } else { self.paras[pi].text.len() };
                        if start < end {
                            apply_fmt_range(&mut self.paras[pi], start, end, |f| set(f, enabled));
                            self.para_texts[pi] = self.paras[pi].text.clone();
                        } else if self.paras[pi].text.is_empty() {
                            if let Some(span) = self.paras[pi].spans.first_mut() { set(&mut span.fmt, enabled); }
                        }
                    }
                    self.dirty = true; self.heights_dirty = true;
                    set(&mut self.cur_fmt, enabled);
                    return;
                }
            }
        }
        
        if let Some((pi, s, e)) = self.last_selection {
            if s != e && pi < self.paras.len() {
                self.push_undo();
                let enabled = !all_set_range(&self.paras[pi], s, e, &get);
                apply_fmt_range(&mut self.paras[pi], s, e, |f| set(f, enabled));
                self.para_texts[pi] = self.paras[pi].text.clone();
                self.dirty = true; self.heights_dirty = true;
                set(&mut self.cur_fmt, enabled);
                return;
            }
        }
        
        let current = get(&self.cur_fmt);
        set(&mut self.cur_fmt, !current);
        let pi = self.focused_para.min(self.paras.len().saturating_sub(1));
        if self.paras[pi].text.is_empty() {
            if let Some(span) = self.paras[pi].spans.first_mut() { set(&mut span.fmt, !current); self.heights_dirty = true; }
        }
    }

    pub(super) fn apply_fmt_size(&mut self, pt: u32) {
        let hp = pt.saturating_mul(2);
        if self.apply_fmt_property(|f| f.size_hp = Some(hp)) { self.cur_fmt.size_hp = Some(hp); return; }
        self.cur_fmt.size_hp = Some(hp);
        let pi = self.focused_para.min(self.paras.len().saturating_sub(1));
        if self.paras[pi].text.is_empty() { if let Some(span) = self.paras[pi].spans.first_mut() { span.fmt.size_hp = Some(hp); self.heights_dirty = true; } }
    }

    pub(super) fn apply_fmt_toggle_bold(&mut self) { self.apply_fmt_toggle(|f| f.bold, |f, v| f.bold = v); }
    pub(super) fn apply_fmt_toggle_italic(&mut self) { self.apply_fmt_toggle(|f| f.italic, |f, v| f.italic = v); }
    pub(super) fn apply_fmt_toggle_underline(&mut self) { self.apply_fmt_toggle(|f| f.underline, |f, v| f.underline = v); }
    pub(super) fn apply_fmt_toggle_strike(&mut self) { self.apply_fmt_toggle(|f| f.strike, |f, v| f.strike = v); }
    pub(super) fn apply_fmt_toggle_sup(&mut self) { self.apply_fmt_toggle(|f| f.sup, |f, v| { f.sup = v; if v { f.sub = false; } }); }
    pub(super) fn apply_fmt_toggle_sub(&mut self) { self.apply_fmt_toggle(|f| f.sub, |f, v| { f.sub = v; if v { f.sup = false; } }); }

    pub(super) fn apply_fmt_font(&mut self, font: Option<FontChoice>) {
        if self.apply_fmt_property(|f| f.font = font) { self.cur_fmt.font = font; return; }
        self.cur_fmt.font = font;
        let pi = self.focused_para.min(self.paras.len().saturating_sub(1));
        if self.paras[pi].text.is_empty() { if let Some(span) = self.paras[pi].spans.first_mut() { span.fmt.font = font; self.heights_dirty = true; } }
    }
    
    pub(super) fn apply_fmt_color(&mut self, color: Option<[u8; 3]>) {
        if self.apply_fmt_property(|f| f.color = color) { self.cur_fmt.color = color; return; }
        self.cur_fmt.color = color;
        let pi = self.focused_para.min(self.paras.len().saturating_sub(1));
        if self.paras[pi].text.is_empty() { if let Some(span) = self.paras[pi].spans.first_mut() { span.fmt.color = color; self.heights_dirty = true; } }
    }

    pub(super) fn apply_fmt_highlight(&mut self, highlight: Option<[u8; 3]>) {
        if self.apply_fmt_property(|f| f.highlight = highlight) { self.cur_fmt.highlight = highlight; return; }
        self.cur_fmt.highlight = highlight;
        let pi = self.focused_para.min(self.paras.len().saturating_sub(1));
        if self.paras[pi].text.is_empty() { if let Some(span) = self.paras[pi].spans.first_mut() { span.fmt.highlight = highlight; self.heights_dirty = true; } }
    }

    pub(super) fn apply_fmt_link(&mut self, link: Option<String>) {
        let lnk = link.clone();
        if self.apply_fmt_property(|f| f.link = lnk.clone()) { self.cur_fmt.link = link; return; }
        self.cur_fmt.link = link.clone();
        let pi = self.focused_para.min(self.paras.len().saturating_sub(1));
        if self.paras[pi].text.is_empty() { if let Some(span) = self.paras[pi].spans.first_mut() { span.fmt.link = link; self.heights_dirty = true; } }
    }

    pub(super) fn apply_fmt_line_height(&mut self, lh: f32) {
        let lh = lh.clamp(0.8, 4.0);
        let mut changed = false;
        if let Some((from, to)) = self.norm_sel() {
            if from.para != to.para {
                self.push_undo();
                for pi in from.para..=to.para {
                    if pi >= self.paras.len() { break; }
                    if (self.paras[pi].line_height - lh).abs() > 0.01 {
                        self.paras[pi].line_height = lh;
                        changed = true;
                    }
                }
                if changed { self.dirty = true; self.heights_dirty = true; self.line_spacing_input = lh; }
                else { self.undo_stack.pop_back(); }
                return;
            }
        }
        let pi = self.focused_para.min(self.paras.len().saturating_sub(1));
        if (self.paras[pi].line_height - lh).abs() > 0.01 {
            self.push_undo();
            self.paras[pi].line_height = lh;
            self.dirty = true; self.heights_dirty = true; self.line_spacing_input = lh;
        }
    }

    fn table_all_set_in_sel(&self, get: impl Fn(&SpanFmt) -> bool) -> bool {
        let mut any = false;
        if let Some((pi, (ar, ac), (br, bc))) = self.table_sel {
            let (r0, r1) = (ar.min(br), ar.max(br)); let (c0, c1) = (ac.min(bc), ac.max(bc));
            if let Some(tbl) = self.paras.get(pi).and_then(|p| p.table.as_ref()) {
                for r in r0..=r1 { for c in c0..=c1 {
                    any = true;
                    if !tbl.rows.get(r).and_then(|row| row.get(c)).map_or(false, |cell| cell.spans.iter().all(|s| get(&s.fmt))) { return false; }
                }}
            }
        }
        if let Some((pi, ref cells)) = self.table_multi_sel {
            if let Some(tbl) = self.paras.get(pi).and_then(|p| p.table.as_ref()) {
                for (r, c) in cells {
                    any = true;
                    if !tbl.rows.get(*r).and_then(|row| row.get(*c)).map_or(false, |cell| cell.spans.iter().all(|s| get(&s.fmt))) { return false; }
                }
            }
        }
        any
    }

    fn table_text_all_set_in_sel(&self, get: impl Fn(&SpanFmt) -> bool) -> bool {
        let Some((pi, ri, ci, start, end)) = self.table_text_sel else { return false };
        if start >= end || pi >= self.paras.len() { return false; }
        self.paras.get(pi).and_then(|p| p.table.as_ref()).and_then(|tbl| tbl.rows.get(ri)).and_then(|row| row.get(ci)).map_or(false, |cell| {
            let mut pos = 0usize;
            let mut any = false;
            for span in &cell.spans {
                let span_end = pos + span.len;
                let overlaps = pos < end && span_end > start;
                if overlaps {
                    any = true;
                    if !get(&span.fmt) { return false; }
                }
                pos = span_end;
            }
            any
        })
    }

    pub(super) fn apply_fmt_to_table_text_sel_fn(&mut self, op: impl Fn(&mut SpanFmt)) -> bool {
        let Some((pi, ri, ci, start, end)) = self.table_text_sel else { return false };
        if start >= end || pi >= self.paras.len() { return false; }
        self.push_undo();
        let mut applied = false;
        if let Some(ref mut tbl) = self.paras[pi].table {
            if let Some(row) = tbl.rows.get_mut(ri) {
                if let Some(cell) = row.get_mut(ci) {
                    let mut tmp = DocParagraph {
                        text: cell.text.clone(),
                        spans: cell.spans.clone(),
                        style: ParaStyle::Normal,
                        align: Align::Left,
                        indent_left: 0.0,
                        indent_first: 0.0,
                        space_before: 0.0,
                        space_after: 0.0,
                        line_height: 1.15,
                        list_num: None,
                        checked: false,
                        is_split: false,
                        table: None,
                        image: None,
                    };
                    apply_fmt_range(&mut tmp, start, end, op);
                    cell.spans = tmp.spans;
                    applied = true;
                }
            }
        }
        if applied { self.dirty = true; self.heights_dirty = true; true } else { self.undo_stack.pop_back(); false }
    }

    pub(super) fn apply_fmt_to_table_sel_fn(&mut self, op: impl Fn(&mut SpanFmt)) -> bool {
        let has_range = self.table_sel.is_some();
        let has_multi = self.table_multi_sel.as_ref().map_or(false, |(_, v)| !v.is_empty());
        if !has_range && !has_multi { return false; }
        self.push_undo();
        if let Some((pi, (ar, ac), (br, bc))) = self.table_sel {
            if pi < self.paras.len() {
                let (r0, r1) = (ar.min(br), ar.max(br)); let (c0, c1) = (ac.min(bc), ac.max(bc));
                if let Some(ref mut tbl) = self.paras[pi].table {
                    for r in r0..=r1 { if let Some(row) = tbl.rows.get_mut(r) { for c in c0..=c1 { if let Some(cell) = row.get_mut(c) { for span in &mut cell.spans { op(&mut span.fmt); } } } } }
                }
            }
        }
        let multi = self.table_multi_sel.clone();
        if let Some((pi, cells)) = multi {
            if pi < self.paras.len() {
                if let Some(ref mut tbl) = self.paras[pi].table {
                    for (r, c) in &cells { if let Some(row) = tbl.rows.get_mut(*r) { if let Some(cell) = row.get_mut(*c) { for span in &mut cell.spans { op(&mut span.fmt); } } } }
                }
            }
        }
        self.dirty = true; self.heights_dirty = true;
        true
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
        let q = self.find_text.to_lowercase();
        let rep = &self.replace_text;
        let mut changed = false;
        for i in 0..self.paras.len() {
            let text = &self.paras[i].text;
            let lower = text.to_lowercase();
            if lower.contains(&q) {
                let mut new_text = String::new();
                let mut last = 0;
                let mut off = 0;
                while let Some(pos) = lower[off..].find(&q) {
                    let s = off + pos;
                    new_text.push_str(&text[last..s]);
                    new_text.push_str(rep);
                    last = s + q.len();
                    off = last;
                }
                new_text.push_str(&text[last..]);
                rebuild_spans(&mut self.paras[i], new_text.clone(), &self.cur_fmt);
                self.para_texts[i] = new_text;
                changed = true;
            }
        }
        if changed { self.dirty = true; self.heights_dirty = true; self.find_stale = true; self.run_find(); }
        else { self.undo_stack.pop_back(); }
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

    pub(super) fn apply_align(&mut self, align: Align) { self.paras[self.focused_para].align = align; self.dirty = true; }

    pub(super) fn fmt_state_bold(&self) -> bool {
        if let Some((pi, s, e)) = self.last_selection { if s != e && pi < self.paras.len() { return all_set_range(&self.paras[pi], s, e, |f| f.bold); } }
        if let Some((pi, ri, ci, s, e)) = self.table_text_sel {
            if s != e && pi < self.paras.len() {
                if let Some(cell) = self.paras.get(pi).and_then(|p| p.table.as_ref()).and_then(|tbl| tbl.rows.get(ri)).and_then(|row| row.get(ci)) {
                    let tmp = DocParagraph {
                        text: cell.text.clone(),
                        spans: cell.spans.clone(),
                        style: ParaStyle::Normal,
                        align: Align::Left,
                        indent_left: 0.0,
                        indent_first: 0.0,
                        space_before: 0.0,
                        space_after: 0.0,
                        line_height: 1.15,
                        list_num: None,
                        checked: false,
                        is_split: false,
                        table: None,
                        image: None,
                    };
                    return all_set_range(&tmp, s, e, |f| f.bold);
                }
            }
        }
        self.cur_fmt.bold
    }
    pub(super) fn fmt_state_italic(&self) -> bool {
        if let Some((pi, s, e)) = self.last_selection { if s != e && pi < self.paras.len() { return all_set_range(&self.paras[pi], s, e, |f| f.italic); } }
        if let Some((pi, ri, ci, s, e)) = self.table_text_sel {
            if s != e && pi < self.paras.len() {
                if let Some(cell) = self.paras.get(pi).and_then(|p| p.table.as_ref()).and_then(|tbl| tbl.rows.get(ri)).and_then(|row| row.get(ci)) {
                    let tmp = DocParagraph {
                        text: cell.text.clone(),
                        spans: cell.spans.clone(),
                        style: ParaStyle::Normal,
                        align: Align::Left,
                        indent_left: 0.0,
                        indent_first: 0.0,
                        space_before: 0.0,
                        space_after: 0.0,
                        line_height: 1.15,
                        list_num: None,
                        checked: false,
                        is_split: false,
                        table: None,
                        image: None,
                    };
                    return all_set_range(&tmp, s, e, |f| f.italic);
                }
            }
        }
        self.cur_fmt.italic
    }
    pub(super) fn fmt_state_underline(&self) -> bool {
        if let Some((pi, s, e)) = self.last_selection { if s != e && pi < self.paras.len() { return all_set_range(&self.paras[pi], s, e, |f| f.underline); } }
        if let Some((pi, ri, ci, s, e)) = self.table_text_sel {
            if s != e && pi < self.paras.len() {
                if let Some(cell) = self.paras.get(pi).and_then(|p| p.table.as_ref()).and_then(|tbl| tbl.rows.get(ri)).and_then(|row| row.get(ci)) {
                    let tmp = DocParagraph {
                        text: cell.text.clone(),
                        spans: cell.spans.clone(),
                        style: ParaStyle::Normal,
                        align: Align::Left,
                        indent_left: 0.0,
                        indent_first: 0.0,
                        space_before: 0.0,
                        space_after: 0.0,
                        line_height: 1.15,
                        list_num: None,
                        checked: false,
                        is_split: false,
                        table: None,
                        image: None,
                    };
                    return all_set_range(&tmp, s, e, |f| f.underline);
                }
            }
        }
        self.cur_fmt.underline
    }

    pub(super) fn adjust_indent_selection(&mut self, delta: f32) {
        self.push_undo();
        let max_indent = (self.layout.content_width() - 36.0).max(0.0);
        let mut changed = false;
        let target_paras = if let Some((from, to)) = self.norm_sel() { from.para..=to.para } else { self.focused_para..=self.focused_para };
        for pi in target_paras {
            let before = self.paras[pi].indent_left;
            self.paras[pi].indent_left = (self.paras[pi].indent_left + delta).clamp(0.0, max_indent);
            changed |= (self.paras[pi].indent_left - before).abs() > f32::EPSILON;
        }
        if changed { self.dirty = true; self.heights_dirty = true; } else { self.undo_stack.pop_back(); }
    }

    pub(super) fn insert_image(&mut self, data: Vec<u8>, display_w: f32, display_h: f32, name: String) {
        self.push_undo();
        let uid = self.next_image_uid;
        self.next_image_uid += 1;
        let mut p = DocParagraph::with_style(ParaStyle::Image);
        p.image = Some(DocImage { data, display_w, display_h, name, uid });
        let idx = (self.focused_para + 1).min(self.paras.len());
        self.paras.insert(idx, p);
        self.focused_para = idx;
        self.sync_texts();
        self.dirty = true; self.heights_dirty = true;
    }

    pub(super) fn insert_table(&mut self, rows: usize, cols: usize) {
        self.push_undo();
        let make_cell = || TableCell { text: String::new(), spans: vec![DocSpan { len: 0, fmt: SpanFmt::default() }], bg_color: None };
        let mut p = DocParagraph::with_style(ParaStyle::Table);
        p.table = Some(Box::new(TableData { rows: (0..rows).map(|_| (0..cols).map(|_| make_cell()).collect()).collect(), col_widths: Vec::new(), border_color: [100, 100, 110], border_width: 1.0 }));
        let idx = (self.focused_para + 1).min(self.paras.len());
        self.paras.insert(idx, p);
        self.focused_para = idx;
        self.sync_texts();
        self.dirty = true; self.heights_dirty = true;
    }

    pub(super) fn insert_horizontal_rule_after_focus(&mut self) {
        self.push_undo();
        let idx = self.focused_para;
        self.paras.insert(idx + 1, DocParagraph::with_style(ParaStyle::HRule));
        if idx + 2 >= self.paras.len() { self.paras.push(DocParagraph::new()); }
        self.focused_para = idx + 2;
        self.pending_focus = Some(self.focused_para);
        self.sync_texts(); self.dirty = true;
    }

    pub(super) fn commit_active_table_cell(&mut self) -> bool {
        let (pi, row, col) = match self.active_table.take() { Some(pos) => pos, None => return false };
        if pi >= self.paras.len() { return false; }
        self.push_undo();
        if let Some(ref mut tbl) = self.paras[pi].table {
            if let Some(r) = tbl.rows.get_mut(row) {
                if let Some(c) = r.get_mut(col) {
                    let text = self.cell_edit_buf.clone();
                    if c.text != text {
                        c.text = text.clone();
                        if c.spans.is_empty() {
                            c.spans = if text.is_empty() { vec![DocSpan { len: 0, fmt: SpanFmt::default() }] }
                                      else { vec![DocSpan { len: text.len(), fmt: SpanFmt::default() }] };
                        } else if c.spans.iter().any(|s| s.fmt != SpanFmt::default()) {
                            let fmt = c.spans.iter().find(|s| s.len > 0).map(|s| s.fmt.clone()).unwrap_or_default();
                            c.spans = if text.is_empty() { vec![DocSpan { len: 0, fmt }] }
                                      else { vec![DocSpan { len: text.len(), fmt }] };
                        } else {
                            c.spans = if text.is_empty() { vec![DocSpan { len: 0, fmt: SpanFmt::default() }] }
                                      else { vec![DocSpan { len: text.len(), fmt: SpanFmt::default() }] };
                        }
                    }
                    self.table_text_sel = None;
                    self.dirty = true; self.heights_dirty = true;
                    return true;
                }
            }
        }
        false
    }

    fn save_impl(&mut self, path: PathBuf) -> Result<(), String> {
        let _ = self.commit_active_table_cell();
        let mut save_paras = self.paras.clone();
        let mut j = 0;
        while j < save_paras.len() {
            if save_paras[j].is_split && j > 0 {
                let orig_space_after = save_paras[j].space_after;
                merge_paragraphs(&mut save_paras, j - 1);
                save_paras[j - 1].space_after = orig_space_after;
                save_paras[j - 1].is_split = false;
            } else { j += 1; }
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "docx" | "doc" => save_docx(&path, &save_paras, &self.layout)?,
            "odt" => save_odt(&path, &save_paras, &self.layout)?,
            _ => { let t: String = save_paras.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n"); std::fs::write(&path, t).map_err(|e| e.to_string())?; }
        }
        self.file_path = Some(path); self.dirty = false; Ok(())
    }

    pub(super) fn insert_table_row(&mut self, pi: usize, row: usize, above: bool) {
        if pi >= self.paras.len() { return; }
        if let Some(ref mut tbl) = self.paras[pi].table {
            let ncols = tbl.rows.first().map(|r| r.len()).unwrap_or(0);
            let new_row: Vec<TableCell> = (0..ncols).map(|_| TableCell::default()).collect();
            let idx = if above { row } else { (row + 1).min(tbl.rows.len()) };
            tbl.rows.insert(idx, new_row);
        }
        self.dirty = true; self.heights_dirty = true;
    }

    pub(super) fn insert_table_col(&mut self, pi: usize, col: usize, left: bool) {
        if pi >= self.paras.len() { return; }
        if let Some(ref mut tbl) = self.paras[pi].table {
            let max_cols = tbl.rows.iter().map(|r| r.len()).max().unwrap_or(0);
            let idx = if left { col } else { (col + 1).min(max_cols) };
            for row in &mut tbl.rows {
                let at = idx.min(row.len());
                row.insert(at, TableCell::default());
            }
            if !tbl.col_widths.is_empty() {
                let n = tbl.col_widths.len();
                let scale = n as f32 / (n + 1) as f32;
                for w in &mut tbl.col_widths { *w *= scale; }
                let at = idx.min(tbl.col_widths.len());
                tbl.col_widths.insert(at, 1.0 / (n + 1) as f32);
            }
        }
        self.dirty = true; self.heights_dirty = true;
    }

    pub(super) fn delete_table_row(&mut self, pi: usize, row: usize) {
        if pi >= self.paras.len() { return; }
        if let Some(ref mut tbl) = self.paras[pi].table {
            if tbl.rows.len() > 1 && row < tbl.rows.len() { tbl.rows.remove(row); }
        }
        self.dirty = true; self.heights_dirty = true;
    }

    pub(super) fn delete_table_col(&mut self, pi: usize, col: usize) {
        if pi >= self.paras.len() { return; }
        if let Some(ref mut tbl) = self.paras[pi].table {
            let min_cols = tbl.rows.iter().map(|r| r.len()).min().unwrap_or(0);
            if min_cols > 1 {
                for row in &mut tbl.rows { if col < row.len() { row.remove(col); } }
                if !tbl.col_widths.is_empty() && col < tbl.col_widths.len() {
                    let removed = tbl.col_widths.remove(col);
                    let n = tbl.col_widths.len();
                    if n > 0 { for w in &mut tbl.col_widths { *w += removed / n as f32; } }
                }
            }
        }
        self.dirty = true; self.heights_dirty = true;
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
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Word Document", &["docx"])
            .add_filter("OpenDocument Text", &["odt"])
            .add_filter("Text", &["txt"])
            .save_file() { self.save_impl(path) }
        else { Err("Cancelled".to_string()) }
    }
    fn take_open_in_image_editor(&mut self) -> Option<Vec<u8>> {
        self.pending_open_in_image_editor.take()
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
            insert_items: vec![
                (MenuItem { label: "Insert Image...".into(), shortcut: None, enabled: true }, MenuAction::Custom("InsertImage".into())),
                (MenuItem { label: "Separator".into(), shortcut: None, enabled: false }, MenuAction::None),
                (MenuItem { label: "Bullet List".into(), shortcut: None, enabled: true }, MenuAction::Custom("InsertBulletList".into())),
                (MenuItem { label: "Numbered List".into(), shortcut: None, enabled: true }, MenuAction::Custom("InsertNumberedList".into())),
                (MenuItem { label: "Checklist".into(), shortcut: None, enabled: true }, MenuAction::Custom("InsertChecklist".into())),
                (MenuItem { label: "Separator".into(), shortcut: None, enabled: false }, MenuAction::None),
                (MenuItem { label: "Horizontal Line".into(), shortcut: None, enabled: true }, MenuAction::Custom("InsertHorizontalRule".into())),
            ],
            format_items: vec![
                (MenuItem { label: "Strikethrough".into(), shortcut: None, enabled: true }, MenuAction::Custom("ToggleStrike".into())),
                (MenuItem { label: "Superscript".into(), shortcut: None, enabled: true }, MenuAction::Custom("ToggleSuperscript".into())),
                (MenuItem { label: "Subscript".into(), shortcut: None, enabled: true }, MenuAction::Custom("ToggleSubscript".into())),
                (MenuItem { label: "Separator".into(), shortcut: None, enabled: false }, MenuAction::None),
                (MenuItem { label: "Increase Indent".into(), shortcut: None, enabled: true }, MenuAction::Custom("IncreaseIndent".into())),
                (MenuItem { label: "Decrease Indent".into(), shortcut: None, enabled: true }, MenuAction::Custom("DecreaseIndent".into())),
            ],
            image_items: vec![], filter_items: vec![], layer_items: vec![],
        }
    }
    fn handle_menu_action(&mut self, action: MenuAction) -> bool {
        match action {
            MenuAction::Undo => { self.undo(); true }
            MenuAction::Redo => { self.redo(); true }
            MenuAction::Custom(ref v) => match v.as_str() {
                "Find" => { self.show_find = true; self.focus_find = true; true }
                "Stats" => { self.show_stats = true; true }
                "PageSettings" => { self.page_settings_draft = None; self.show_page_settings = true; true }
                "ToggleOutline" => { self.show_outline = !self.show_outline; true }
                "ZoomIn" => { self.zoom = (self.zoom + 0.1).min(3.0); true }
                "ZoomOut" => { self.zoom = (self.zoom - 0.1).max(0.3); true }
                "ZoomReset" => { self.auto_zoom_done = false; true }
                "InsertBulletList" => { self.apply_style_toggle(ParaStyle::ListBullet); true }
                "InsertNumberedList" => { self.apply_style_toggle(ParaStyle::ListOrdered); true }
                "InsertChecklist" => { self.apply_style_toggle(ParaStyle::ListCheck); true }
                "InsertHorizontalRule" => { self.insert_horizontal_rule_after_focus(); true }
                "ToggleStrike" => { self.apply_fmt_toggle_strike(); true }
                "ToggleSuperscript" => { self.apply_fmt_toggle_sup(); true }
                "ToggleSubscript" => { self.apply_fmt_toggle_sub(); true }
                "IncreaseIndent" => { self.adjust_indent_selection(36.0); true }
                "DecreaseIndent" => { self.adjust_indent_selection(-36.0); true }
                "InsertImage" => {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Images", &["jpg", "jpeg", "png", "webp", "bmp", "tiff", "ico"]).pick_file() {
                        if let Ok(img) = image::open(&path) {
                            let iw = img.width() as f32;
                            let ih = img.height() as f32;
                            let max_w = self.layout.content_width();
                            let scale = if iw > max_w { max_w / iw } else { 1.0 };
                            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
                            let fmt = match ext.to_lowercase().as_str() { "jpg" | "jpeg" => image::ImageFormat::Jpeg, "webp" => image::ImageFormat::WebP, "bmp" => image::ImageFormat::Bmp, _ => image::ImageFormat::Png };
                            let mut buf = Vec::new();
                            img.write_to(&mut std::io::Cursor::new(&mut buf), fmt).ok();
                            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("image.png").to_string();
                            self.insert_image(buf, iw * scale, ih * scale, name);
                        }
                    }
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, _show_file_info: bool) {
        super::de_ui::render(self, ui, ctx);
    }
}
