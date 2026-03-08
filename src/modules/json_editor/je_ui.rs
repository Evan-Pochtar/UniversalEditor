use eframe::egui;
use serde_json::Value;
use crate::style::{self, ColorPalette, ThemeMode, toolbar_action_btn};
use super::je_main::{JsonEditor, JsonViewMode, EditCell, AddKeyDialog};
use super::je_tools::{
    SortMode, SearchTarget, FlatNode,
    sort_label, search_target_label, parse_cell_value, parse_edit_value, serialize_value, value_at_path,
};
use super::je_style::{ 
    c_panel, c_border, c_row_alt, c_row_sel, c_row_match, c_row_match_active, c_key, c_text, c_muted, c_error,
    val_color, danger_button, ghost_btn_small, expand_triangle, accent_button, compact_button   
};

const ROW_H: f32 = 22.0;

impl JsonEditor {
    pub(super) fn render_editor_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, show_file_info: bool) {
        let dark = ui.visuals().dark_mode;
        let theme = if dark { ThemeMode::Dark } else { ThemeMode::Light };
        self.handle_keyboard(ctx);
        self.rebuild_flat_if_needed();
        if self.text_stale && matches!(self.view_mode, JsonViewMode::Text) { self.sync_text_from_root();}
        self.run_search();

        ui.vertical(|ui| {
            // Main Toolbar UI
            ui.horizontal(|ui| {
                ui.horizontal(|ui: &mut egui::Ui| {
                    self.render_view_tabs(ui, dark);
                    ui.separator();
                    if toolbar_action_btn(ui, "Expand All", theme).clicked() { self.expand_all(); }
                    if toolbar_action_btn(ui, "Collapse All", theme).clicked() { self.collapse_all(); }

                    ui.separator();
                    if toolbar_action_btn(ui, "+ Add Key", theme).clicked() {
                        self.add_dialog = Some(AddKeyDialog {
                            parent_path: self.scope_path.clone(),
                            key_buf: String::new(),
                            val_buf: String::new(),
                            error: None,
                        });
                    }

                    if toolbar_action_btn(ui, "New", theme).clicked() { self.show_new_confirm = true; }
                    ui.separator();
                });

                ui.label(egui::RichText::new("Sort:").size(12.0).color(c_muted(dark)));
                ui.vertical(|ui: &mut egui::Ui| {
                    egui::ComboBox::from_id_salt("je_sort")
                        .selected_text(egui::RichText::new(sort_label(self.sort_mode)).size(12.0))
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for m in [SortMode::None, SortMode::KeyAsc, SortMode::KeyDesc, SortMode::ValueAsc, SortMode::ValueDesc] {
                                if ui.selectable_value(&mut self.sort_mode, m, sort_label(m)).changed() { self.invalidate_flat(); }
                            }
                        });
                });
                ui.separator();
    
                ui.label(egui::RichText::new("Export Format:").size(12.0).color(c_muted(dark)));
                ui.vertical(|ui: &mut egui::Ui| {
                    let fmt_label = if self.export_pretty { "Pretty" } else { "Compact" };
                    egui::ComboBox::from_id_salt("je_fmt")
                        .selected_text(egui::RichText::new(fmt_label).size(12.0))
                        .width(80.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.export_pretty, true, "Pretty");
                            ui.selectable_value(&mut self.export_pretty, false, "Compact");
                        });
                });
                if toolbar_action_btn(ui, "Export", theme).clicked() { self.do_export(); }
            });
            ui.separator();

            // File Information UI
            if show_file_info {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(self.get_file_name()).size(12.0).color(c_text(dark)));
                    ui.separator();
                    let (status, color) = if self.dirty { ("Modified", if dark { ColorPalette::AMBER_400 } else { ColorPalette::AMBER_600 }) } 
                        else { ("Saved", if dark { ColorPalette::GREEN_400 } else { ColorPalette::GREEN_600 }) };

                    ui.label(egui::RichText::new(status).size(12.0).color(color));
                    ui.separator();
                    let node_count = self.flat.len();
                    ui.label(egui::RichText::new(format!("{} visible nodes", node_count)).size(12.0).color(c_muted(dark)));
                });
                ui.separator();
            }

            // Breadcrumb navigation UI
            if !self.scope_path.is_empty() { 
                ui.horizontal(|ui| {
                    if ui.add(egui::Label::new(egui::RichText::new("root").size(12.0).color(ColorPalette::BLUE_500)).sense(egui::Sense::click())).clicked() {
                        self.scope_path.clear();
                        self.invalidate_flat();
                        self.text_stale = true;
                        self.search_stale = true;
                        self.selected_row = None;
                    }

                    let path_snapshot = self.scope_path.clone();
                    let mut truncate_to: Option<usize> = None;
                    for (i, seg) in path_snapshot.iter().enumerate() {
                        ui.label(egui::RichText::new("/").size(12.0).color(c_border(dark)));
                        let is_last = i + 1 == path_snapshot.len();
                        let color = if is_last { c_text(dark) } else { ColorPalette::BLUE_500 };
                        if ui.add(egui::Label::new(egui::RichText::new(seg).size(12.0).color(color)).sense(egui::Sense::click())).clicked() && !is_last {
                            truncate_to = Some(i + 1);
                        }
                    }
                    if let Some(len) = truncate_to {
                        self.scope_path.truncate(len);
                        self.invalidate_flat();
                        self.text_stale = true;
                        self.search_stale = true;
                        self.selected_row = None;
                    }

                    ui.add_space(8.0);
                    if ui.add(egui::Label::new(egui::RichText::new("Back").size(12.0).color(c_muted(dark))).sense(egui::Sense::click())).clicked() {
                        self.scope_up();
                    }
                });
                ui.separator();
            }

            // Searchbar UI
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Search:").size(12.0).color(c_muted(dark)));
                let prev_query = self.search_query.clone();
                let resp = ui.add(egui::TextEdit::singleline(&mut self.search_query)
                    .desired_width(220.0)
                    .min_size(egui::vec2(0.0, 24.0))
                    .hint_text("Type to search..."),
                );
                if resp.changed() || self.search_query != prev_query {
                    self.search_stale = true;
                    self.search_cursor = 0;
                    self.run_search();
                }
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.search_next();
                }

                ui.vertical(|ui: &mut egui::Ui| {
                    egui::ComboBox::from_id_salt("je_search_target")
                        .selected_text(egui::RichText::new(search_target_label(self.search_target)).size(12.0))
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            for t in [SearchTarget::Both, SearchTarget::Keys, SearchTarget::Values] {
                                if ui.selectable_value(&mut self.search_target, t, search_target_label(t)).changed() {
                                    self.search_stale = true;
                                    self.run_search();
                                }
                            }
                        });
                });

                let has = !self.search_results.is_empty() || (!self.search_only_expanded && !self.search_all_paths.is_empty());
                if ghost_btn_small(ui, "Prev", dark, has).clicked() { self.search_prev(); }
                if ghost_btn_small(ui, "Next", dark, has).clicked() { self.search_next(); }

                if !self.search_query.is_empty() {
                    let count = self.search_result_count();
                    let cur = if count > 0 { self.search_cursor + 1 } else { 0 };
                    ui.label(egui::RichText::new(format!("{}/{}", cur, count))
                        .size(12.0)
                        .color(if count == 0 { c_error(dark) } else { c_muted(dark) }));
                }

                if matches!(self.view_mode, JsonViewMode::Tree) {
                    let prev = self.search_only_expanded;
                    ui.add(egui::Checkbox::new(&mut self.search_only_expanded,
                        egui::RichText::new("Search Only Expanded Nodes").size(11.0).color(c_muted(dark))));
                    if self.search_only_expanded != prev {
                        self.search_stale = true;
                        self.search_cursor = 0;
                        self.run_search();
                    }
                }

                if !self.search_query.is_empty() {
                    if compact_button(ui, "Clear", dark).clicked() {
                        self.search_query.clear();
                        self.search_results.clear();
                        self.search_all_paths.clear();
                        self.search_stale = false;
                    }
                }
            });
            ui.separator();

            match self.view_mode {
                JsonViewMode::Tree => self.render_table_view(ui, dark),
                JsonViewMode::Text => self.render_text_view(ui, ctx, dark),
            }
        });

        self.render_add_key_dialog(ctx, dark);
        self.render_confirm_delete_dialog(ctx, dark);
        self.render_new_confirm_dialog(ctx, dark);
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if matches!(self.view_mode, JsonViewMode::Text) { return; }
        ctx.input_mut(|i| {
            if i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z)) { self.undo(); }
            if i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Y)) { self.redo(); }
            if i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)) { self.redo(); }
        });
    }

    fn render_view_tabs(&mut self, ui: &mut egui::Ui, dark: bool) {
        for (mode, label) in [(JsonViewMode::Tree, "Tree"), (JsonViewMode::Text, "Text")] {
            let selected = self.view_mode == mode;
            let (bg, txt) = if selected { (ColorPalette::BLUE_600, egui::Color32::WHITE) }
                else { (egui::Color32::TRANSPARENT, c_muted(dark)) };

            ui.scope(|ui| {
                let s = ui.style_mut();
                s.visuals.widgets.inactive.bg_fill = bg;
                s.visuals.widgets.inactive.weak_bg_fill = bg;
                s.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, if selected { bg } else { c_border(dark) });
                s.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, txt);
                s.visuals.widgets.hovered.bg_fill = if selected { ColorPalette::BLUE_500 } else if dark { egui::Color32::from_rgb(34, 34, 42) } else { ColorPalette::GRAY_100 };
                s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, c_border(dark));
                s.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, if selected { egui::Color32::WHITE } else { c_text(dark) });
                s.visuals.widgets.active.bg_fill = bg;
                let resp = ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(txt)).min_size(egui::vec2(52.0, 24.0)));
                if resp.clicked() && !selected {
                    if self.view_mode == JsonViewMode::Text && self.text_modified { let _ = self.commit_text_to_root(); }
                    self.view_mode = mode;
                    if mode == JsonViewMode::Text { self.sync_text_from_root(); }
                }
            });
        }
    }

    fn render_table_view(&mut self, ui: &mut egui::Ui, dark: bool) {
        let flat_len: usize = self.flat.len();
        if flat_len == 0 { self.render_empty_state(ui, dark); return; }

        let result_set: std::collections::HashSet<usize> = if self.search_only_expanded {
            self.search_results.iter().cloned().collect()
        } else {
            self.search_all_paths.iter()
                .filter_map(|p| self.flat.iter().position(|n| &n.path == p))
                .collect()
        };
        let active_match: Option<usize> = if self.search_only_expanded {
            self.search_results.get(self.search_cursor).cloned()
        } else {
            self.search_all_paths.get(self.search_cursor)
                .and_then(|p| self.flat.iter().position(|n| &n.path == p))
        };

        let mut toggle_path: Option<Vec<String>> = None;
        let mut drill_path: Option<Vec<String>> = None;
        let mut delete_path: Option<Vec<String>> = None;
        let mut begin_edit: Option<(usize, bool)> = None;
        let mut commit_edit: bool = false;

        let available_w: f32 = ui.available_width();
        let col_key_w: f32 = (available_w * 0.35).max(120.0).min(300.0);
        let col_type_w: f32 = 70.0;
        let col_val_w: f32 = (available_w - col_key_w - col_type_w - 62.0).max(100.0);

        egui::Frame::new().fill(c_panel(dark)).inner_margin(0.0).show(ui, |ui| {
            let header_h = 28.0;
            let (header_rect, _) = ui.allocate_exact_size(egui::vec2(available_w, header_h), egui::Sense::hover());
            ui.painter().rect_filled(header_rect, 0.0, c_panel(dark));
            ui.painter().line_segment([header_rect.left_bottom(), header_rect.right_bottom()], egui::Stroke::new(1.0, c_border(dark)));
            let cy = header_rect.center().y;
            for (x, label, _) in &[(6.0, "Key", col_key_w), (col_key_w + 6.0, "Type", col_type_w), (col_key_w + col_type_w + 6.0, "Value", col_val_w)] {
                ui.painter().text(egui::pos2(header_rect.min.x + x, cy), egui::Align2::LEFT_CENTER, *label, egui::FontId::proportional(11.5), c_muted(dark));
            }
        });

        const MAX_EDIT_H: f32 = 120.0;
        let edit_row_idx: Option<usize> = if self.edit_cell_is_string {
            self.edit_cell.as_ref().and_then(|ec| self.flat.iter().position(|n| n.path == ec.path))
        } else { None };
        let extra_h: f32 = if edit_row_idx.is_some() {
            if let Some(ec) = &self.edit_cell {
                let font_id = egui::FontId::proportional(12.0);
                let wrap_w = (col_val_w - 12.0).max(40.0);
                let measured_h = ui.ctx().fonts_mut(|f| { f.layout(ec.buffer.clone(), font_id, egui::Color32::WHITE, wrap_w).size().y });
                let padded = (measured_h + 8.0).min(MAX_EDIT_H);
                (padded - ROW_H).max(0.0)
            } else { 0.0 }
        } else { 0.0 };

        let total_h: f32 = flat_len as f32 * ROW_H + extra_h;
        let current_edit: Option<EditCell> = self.edit_cell.clone();
        let scroll_to_offset: Option<f32> = self.pending_scroll_row.take().map(|row| {
            let base = row as f32 * ROW_H;
            if edit_row_idx.map_or(false, |ei| row > ei) { base + extra_h } else { base }
        });

        let mut scroll_area = egui::ScrollArea::vertical().id_salt("je_table_scroll").auto_shrink([false, false]);
        if let Some(offset) = scroll_to_offset { scroll_area = scroll_area.vertical_scroll_offset(offset); }
        scroll_area.show_viewport(ui, |ui, viewport| {
            let (total_rect, _) = ui.allocate_exact_size(egui::vec2(available_w, total_h), egui::Sense::hover());
            let ei_opt = edit_row_idx;
            let (first, last) = {
                let vmin = viewport.min.y;
                let vmax = viewport.max.y;
                let f = if ei_opt.map_or(true, |ei| vmin <= (ei as f32 + 1.0) * ROW_H) { (vmin / ROW_H) as usize } else { ((vmin - extra_h) / ROW_H) as usize };
                let l = if ei_opt.map_or(true, |ei| vmax <= (ei as f32 + 1.0) * ROW_H + extra_h) { (vmax / ROW_H) as usize + 3 } else { ((vmax - extra_h) / ROW_H) as usize + 3 };
                (f.saturating_sub(2), l.min(flat_len))
            };
            let always_include = ei_opt.filter(|&ei| ei >= first && ei < last).is_none();
            let rows: Vec<(usize, FlatNode)> = {
                let mut v: Vec<(usize, FlatNode)> = (first..last).filter_map(|i| self.flat.get(i).map(|n| (i, n.clone()))).collect();
                if always_include {
                    if let Some(ei) = ei_opt {
                        if let Some(n) = self.flat.get(ei) { v.push((ei, n.clone())); v.sort_by_key(|(i, _)| *i); }
                    }
                }
                v
            };

            for (i, node) in rows {
                let y_base = total_rect.min.y + i as f32 * ROW_H;
                let y_top = if ei_opt.map_or(false, |ei| i > ei) { y_base + extra_h } else { y_base };
                let row_h = if ei_opt == Some(i) { ROW_H + extra_h } else { ROW_H };
                let row_rect = egui::Rect::from_min_size(egui::pos2(total_rect.min.x, y_top), egui::vec2(available_w, row_h));
                if !ui.is_rect_visible(row_rect) { continue; }

                let is_selected = self.selected_row == Some(i);
                let is_match = result_set.contains(&i);
                let is_active = active_match == Some(i);
                let bg = if is_selected { c_row_sel(dark) } else if is_active { c_row_match_active(dark) } else if is_match { c_row_match(dark) } else if i % 2 == 1 { c_row_alt(dark) } else { egui::Color32::TRANSPARENT };
                if bg != egui::Color32::TRANSPARENT {
                    ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(row_rect.min.x, row_rect.min.y + 1.0), row_rect.max), 0.0, bg);
                }

                let cx = row_rect.min.x;
                let cy = row_rect.min.y + ROW_H / 2.0;
                let indent = node.depth as f32 * 12.0 + 6.0;
                let editing_k = current_edit.as_ref().filter(|e| e.path == node.path && e.editing_key).is_some();
                let key_x = cx + indent;
                let key_avail = col_key_w - indent - 4.0;

                if editing_k {
                    if let Some(ec) = &mut self.edit_cell {
                        let er = egui::Rect::from_min_size(egui::pos2(key_x, cy - 9.0), egui::vec2(key_avail, 18.0));
                        let r = ui.put(er, egui::TextEdit::singleline(&mut ec.buffer).font(egui::FontId::proportional(12.0)));
                        if r.lost_focus() { commit_edit = true; }
                    }
                } else {
                    if node.has_children {
                        let tri = egui::Rect::from_min_size(egui::pos2(key_x, row_rect.min.y), egui::vec2(16.0, ROW_H));
                        if expand_triangle(ui, tri, node.is_expanded, dark) { toggle_path = Some(node.path.clone()); }
                    }
                    let tx = if node.has_children { key_x + 16.0 } else { key_x + 2.0 };
                    ui.painter().text(egui::pos2(tx, cy), egui::Align2::LEFT_CENTER, &node.key, egui::FontId::proportional(12.5), c_key(dark));
                    let key_sense_x = if node.has_children { key_x + 16.0 } else { key_x };
                    let key_sense_w = (col_key_w - indent - if node.has_children { 16.0 } else { 0.0 }).max(0.0);
                    let kr = egui::Rect::from_min_size(egui::pos2(key_sense_x, row_rect.min.y), egui::vec2(key_sense_w, ROW_H));
                    let ks = ui.allocate_rect(kr, egui::Sense::click());
                    if ks.double_clicked() { begin_edit = Some((i, true)); }
                    if ks.clicked() { self.selected_row = Some(i); }
                }

                let div1_x = cx + col_key_w;
                ui.painter().line_segment([egui::pos2(div1_x, row_rect.min.y), egui::pos2(div1_x, row_rect.max.y)], egui::Stroke::new(1.0, c_border(dark)));
                ui.painter().text(egui::pos2(div1_x + 4.0, cy), egui::Align2::LEFT_CENTER, node.val_type.type_label(), egui::FontId::proportional(11.0), c_muted(dark));

                let div2_x = div1_x + col_type_w;
                ui.painter().line_segment([egui::pos2(div2_x, row_rect.min.y), egui::pos2(div2_x, row_rect.max.y)], egui::Stroke::new(1.0, c_border(dark)));

                let val_x = div2_x + 4.0;
                let editing_v = current_edit.as_ref().filter(|e| e.path == node.path && !e.editing_key).is_some();
                if editing_v {
                    if let Some(ec) = &mut self.edit_cell {
                        if self.edit_cell_is_string {
                            let edit_h = row_h - 4.0;
                            let er = egui::Rect::from_min_size(egui::pos2(val_x, row_rect.min.y + 2.0), egui::vec2(col_val_w - 4.0, edit_h));
                            let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(er).layout(*ui.layout()));
                            child_ui.set_clip_rect(er.intersect(ui.clip_rect()));
                            let scroll_resp = egui::ScrollArea::vertical().id_salt("je_str_edit_scroll").max_height(edit_h).auto_shrink([false, false])
                                .show(&mut child_ui, |ui| {
                                    ui.add(egui::TextEdit::multiline(&mut ec.buffer).desired_width(col_val_w - 8.0).desired_rows(1).font(egui::FontId::proportional(12.0)))
                                });
                            let r = scroll_resp.inner;
                            if r.lost_focus() && !ui.input(|inp| inp.key_pressed(egui::Key::Escape)) { commit_edit = true; }
                            if r.has_focus() && ui.input(|inp| inp.key_pressed(egui::Key::Escape)) { commit_edit = true; }
                        } else {
                            let er = egui::Rect::from_min_size(egui::pos2(val_x, cy - 9.0), egui::vec2(col_val_w - 4.0, 18.0));
                            let r = ui.put(er, egui::TextEdit::singleline(&mut ec.buffer).font(egui::FontId::proportional(12.0)));
                            if r.lost_focus() { commit_edit = true; }
                        }
                    }
                } else {
                    ui.painter().text(egui::pos2(val_x, cy), egui::Align2::LEFT_CENTER, node.val_type.preview_str(), egui::FontId::proportional(12.5), val_color(&node.val_type, dark));
                    let vr = egui::Rect::from_min_size(egui::pos2(val_x, row_rect.min.y), egui::vec2(col_val_w - 4.0, ROW_H));
                    let vs = ui.allocate_rect(vr, egui::Sense::click());
                    if vs.double_clicked() {
                        if node.has_children { drill_path = Some(node.path.clone()); } else { begin_edit = Some((i, false)); }
                    }
                    if vs.clicked() { self.selected_row = Some(i); }
                }

                if ui.rect_contains_pointer(row_rect) || is_selected {
                    let bx = row_rect.max.x - 56.0;
                    if node.has_children {
                        let add_r = egui::Rect::from_min_size(egui::pos2(bx, cy - 10.0), egui::vec2(24.0, 20.0));
                        let ar = ui.allocate_rect(add_r, egui::Sense::click());
                        let ac = if ar.hovered() { ColorPalette::BLUE_400 } else { c_muted(dark) };
                        ui.painter().text(ar.rect.center(), egui::Align2::CENTER_CENTER, "+", egui::FontId::proportional(14.0), ac);
                        if ar.clicked() {
                            self.add_dialog = Some(AddKeyDialog { parent_path: node.path.clone(), key_buf: String::new(), val_buf: String::new(), error: None });
                        }
                    }
                    let del_r = egui::Rect::from_min_size(egui::pos2(bx + 28.0, cy - 10.0), egui::vec2(24.0, 20.0));
                    let dr = ui.allocate_rect(del_r, egui::Sense::click());
                    let dc = if dr.hovered() { ColorPalette::RED_400 } else { c_muted(dark) };
                    ui.painter().text(dr.rect.center(), egui::Align2::CENTER_CENTER, "x", egui::FontId::proportional(12.0), dc);
                    if dr.clicked() { delete_path = Some(node.path.clone()); }
                }

                ui.painter().line_segment([row_rect.left_bottom(), row_rect.right_bottom()], egui::Stroke::new(0.5, c_border(dark)));
            }
        });

        if let Some(path) = toggle_path  { self.toggle_expand(&path); }
        if let Some(path) = drill_path   { self.drill_into(path); }
        if let Some(path) = delete_path  { self.confirm_delete_path = Some(path); }
        if commit_edit { self.commit_inline_edit(); }
        if let Some((row, ek)) = begin_edit {
            if let Some(node) = self.flat.get(row) {
                let (init, is_str) = if ek {
                    (node.key.clone(), false)
                } else {
                    match &node.val_type {
                        super::je_tools::ValType::Str(s) => (s.clone(), true),
                        _ => (node.val_type.preview_str(), false),
                    }
                };
                self.edit_cell_is_string = is_str;
                self.edit_cell = Some(EditCell { path: node.path.clone(), buffer: init, editing_key: ek });
            }
        }
    }

    fn render_text_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, dark: bool) {
        if self.text_stale { self.sync_text_from_root(); }

        let font_id = egui::FontId::new(13.0, egui::FontFamily::Monospace);
        let row_h = {
            let fh = ctx.fonts_mut(|f| f.row_height(&font_id));
            if fh > 0.0 { self.text_row_h = fh; fh }
            else if self.text_row_h > 0.0 { self.text_row_h }
            else { 18.0_f32 }
        };
        let total_lines = {
            let n = self.text_rope.len_lines();
            if n > 1 && self.text_rope.line(n - 1).len_chars() == 0 { n - 1 } else { n }.max(1)
        };

        let digits = ((total_lines as f32).log10().floor() as usize + 1).max(1);
        let gutter_w = 7.8_f32 * digits as f32 + 20.0;
        let total_h = total_lines as f32 * row_h;

        let (gutter_bg, num_color) = if dark {
            (egui::Color32::from_rgb(20, 20, 26), c_muted(dark))
        } else {
            (egui::Color32::from_rgb(246, 247, 250), c_muted(dark))
        };
        let border_col = c_border(dark);

        egui::ScrollArea::vertical()
            .id_salt("je_text_scroll")
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let avail_w = ui.max_rect().width();
                let text_w = (avail_w - gutter_w).max(80.0);

                let (area, _) = ui.allocate_exact_size(
                    egui::vec2(avail_w, total_h.max(viewport.height())),
                    egui::Sense::hover(),
                );

                let gutter_rect = egui::Rect::from_min_max(
                    egui::pos2(area.min.x, area.min.y + viewport.min.y),
                    egui::pos2(area.min.x + gutter_w - 1.0, area.min.y + viewport.max.y),
                );
                ui.painter().rect_filled(gutter_rect, 0.0, gutter_bg);
                ui.painter().line_segment([gutter_rect.right_top(), gutter_rect.right_bottom()], egui::Stroke::new(1.0, border_col));

                let first_vis = (viewport.min.y / row_h) as usize;
                let viewport_lines = (viewport.height() / row_h) as usize + 2;
                let in_window = first_vis >= self.text_win_start
                    && (first_vis + viewport_lines) <= (self.text_win_start + self.text_win_line_count);

                if !in_window {
                    if self.text_win_modified { self.sync_window_to_rope(); }
                    self.text_win_start = first_vis.saturating_sub(50).min(total_lines.saturating_sub(1));
                    self.rebuild_text_window();
                }

                let win_top = area.min.y + self.text_win_start as f32 * row_h;
                let te_rect = egui::Rect::from_min_size(
                    egui::pos2(area.min.x + gutter_w, win_top),
                    egui::vec2(text_w, self.text_win_line_count as f32 * row_h + row_h * 4.0),
                );

                let output = ui.scope_builder(
                    egui::UiBuilder::new().max_rect(te_rect),
                    |ui| {
                        egui::TextEdit::multiline(&mut self.text_win_buf)
                            .font(font_id.clone())
                            .desired_width(text_w)
                            .frame(false)
                            .lock_focus(true)
                            .show(ui)
                    },
                ).inner;

                if output.response.changed() {
                    self.text_win_modified = true;
                    self.text_modified = true;
                    self.sync_window_to_rope();
                    let full_text = self.text_rope.to_string();
                    self.text_errors = super::je_tools::validate_json(&full_text);
                    self.rebuild_text_window();
                }

                if let Some(rh) = output.galley.rows.first().map(|r| r.rect().height()) {
                    if rh > 0.0 && (rh - self.text_row_h).abs() > 0.5 {
                        self.text_row_h = rh;
                        ctx.request_repaint();
                    }
                }

                let rows = &output.galley.rows;
                let mut log_line = self.text_win_start;
                for (i, row) in rows.iter().enumerate() {
                    let ry = output.galley_pos.y + row.rect().min.y;
                    if ry > area.min.y + viewport.max.y { break; }
                    let is_first = i == 0 || rows[i - 1].ends_with_newline;
                    if is_first && ry + row.rect().height() >= area.min.y + viewport.min.y {
                        ui.painter().text(
                            egui::pos2(area.min.x + gutter_w - 8.0, ry + row.rect().height() / 2.0),
                            egui::Align2::RIGHT_CENTER,
                            &(log_line + 1).to_string(),
                            font_id.clone(),
                            num_color,
                        );
                    }
                    if row.ends_with_newline { log_line += 1; }
                }
            });

        let has_errors = !self.text_errors.is_empty();
        let anim_id = ui.id().with("text_err_anim");
        let alpha = ctx.animate_bool_with_time(anim_id, has_errors, 0.25);
        if alpha > 0.001 {
            let screen_rect = ctx.content_rect();
            let err_msg = self.text_errors.first().map(|(line, msg)| format!("Line {}: {}", line, msg)).unwrap_or_default();
            let pad = 12.0_f32;
            let max_text_w = 340.0_f32;
            let font = egui::FontId::proportional(12.0);
            let margin = 12.0_f32;
            let alpha_u8 = (alpha * 255.0) as u8;
            let text_color = egui::Color32::from_rgba_unmultiplied(255, 200, 200, alpha_u8);
            let galley = {
                let mut job = egui::text::LayoutJob::default();
                job.wrap = egui::text::TextWrapping { max_width: max_text_w, ..Default::default() };
                job.append(&err_msg, 0.0, egui::text::TextFormat { font_id: font.clone(), color: text_color, ..Default::default() });
                ctx.fonts_mut(|f| f.layout_job(job))
            };
            let text_size = galley.size();
            let box_w = text_size.x + pad * 2.0;
            let box_h = text_size.y + pad * 2.0;
            let box_rect = egui::Rect::from_min_size(
                egui::pos2(screen_rect.max.x - box_w - margin, screen_rect.max.y - box_h - margin),
                egui::vec2(box_w, box_h),
            );
            let bg_color = egui::Color32::from_rgba_unmultiplied(if dark { 120 } else { 200 }, if dark { 20 } else { 30 }, if dark { 20 } else { 30 }, alpha_u8);
            let border_color = egui::Color32::from_rgba_unmultiplied(if dark { 220 } else { 185 }, if dark { 60 } else { 50 }, if dark { 60 } else { 50 }, alpha_u8);
            let err_painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, anim_id));
            err_painter.rect(box_rect, egui::CornerRadius::same(8), bg_color, egui::Stroke::new(1.0, border_color), egui::StrokeKind::Outside);
            err_painter.galley(egui::pos2(box_rect.min.x + pad, box_rect.min.y + pad), galley, text_color);
            if alpha > 0.001 && alpha < 0.999 { ctx.request_repaint(); }
        }
    }

    fn render_empty_state(&self, ui: &mut egui::Ui, dark: bool) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(egui::RichText::new("No data").size(22.0).color(c_muted(dark)));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("The JSON is empty or all nodes are filtered. Use the toolbar to add a key or load a file.").size(13.0).color(c_muted(dark)));
            });
        });
    }

    fn commit_inline_edit(&mut self) {
        if let Some(ec) = self.edit_cell.take() {
            self.edit_cell_is_string = false;
            if ec.editing_key {
                if !ec.path.is_empty() {
                    let (parent, key_seg) = ec.path.split_at(ec.path.len() - 1);
                    let old_key = &key_seg[0];
                    if &ec.buffer != old_key && !ec.buffer.is_empty() { self.rename_node_key(parent, old_key, &ec.buffer); }
                }
            } else {
                let new_val = parse_edit_value(&ec.buffer);
                self.set_node_value(&ec.path, new_val);
            }
        }
    }

    fn render_add_key_dialog(&mut self, ctx: &egui::Context, dark: bool) {
        if self.add_dialog.is_none() { return; }
        let (bg, border, text) = if dark { (egui::Color32::from_rgb(26, 26, 32), ColorPalette::ZINC_700, ColorPalette::SLATE_100) }
            else { (egui::Color32::WHITE, ColorPalette::GRAY_300, ColorPalette::GRAY_900) };
        style::draw_modal_overlay(ctx, "je_add_overlay", 160);
        let mut close = false;
        let mut do_add = false;
        egui::Window::new("Add Key").collapsible(false).resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Tooltip)
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(24.0))
            .show(ctx, |ui| {
                if let Some(dialog) = &mut self.add_dialog {
                    ui.vertical(|ui| {
                        let path_str = if dialog.parent_path.is_empty() { "root".to_string() } else { dialog.parent_path.join(" / ") };
                        ui.label(egui::RichText::new(format!("Parent: {}", path_str)).size(12.0).color(c_muted(dark)));
                        let is_array = value_at_path(&self.root, &dialog.parent_path).map(|v| v.is_array()).unwrap_or(false);
                        if !is_array {
                            ui.label(egui::RichText::new("Key name:").size(13.0).color(text));
                            ui.add(egui::TextEdit::singleline(&mut dialog.key_buf).desired_width(280.0).hint_text("Enter key name..."));
                        }
                        ui.label(egui::RichText::new("Value (JSON):").size(13.0).color(text));
                        ui.add(egui::TextEdit::multiline(&mut dialog.val_buf).desired_width(280.0).desired_rows(3).hint_text("null  /  true  /  42  /  \"hello\"  /  {}  /  []"));
                        if let Some(err) = &dialog.error { ui.label(egui::RichText::new(err).size(12.0).color(c_error(dark))); }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if accent_button(ui, "Add").clicked() { do_add = true; }
                            if compact_button(ui, "Cancel", dark).clicked() { close = true; }
                        });
                    });
                }
            });
        if do_add {
            if let Some(dialog) = &mut self.add_dialog {
                let val = match serde_json::from_str::<Value>(dialog.val_buf.trim()) {
                    Ok(v) => v,
                    Err(_) => parse_cell_value(&dialog.val_buf),
                };
                let is_array = value_at_path(&self.root, &dialog.parent_path).map(|v| v.is_array()).unwrap_or(false);
                if !is_array && dialog.key_buf.trim().is_empty() {
                    dialog.error = Some("Key name cannot be empty.".into());
                } else {
                    let key = dialog.key_buf.trim().to_string();
                    let parent = dialog.parent_path.clone();
                    self.add_node(&parent, &key, val);
                    close = true;
                }
            }
        }
        if close { self.add_dialog = None; }
    }

    fn render_confirm_delete_dialog(&mut self, ctx: &egui::Context, dark: bool) {
        let path = match &self.confirm_delete_path { Some(p) => p.clone(), None => return };
        let (bg, border, text) = if dark { (egui::Color32::from_rgb(26, 26, 32), ColorPalette::ZINC_700, ColorPalette::SLATE_100) }
            else { (egui::Color32::WHITE, ColorPalette::GRAY_300, ColorPalette::GRAY_900) };
        style::draw_modal_overlay(ctx, "je_del_overlay", 160);
        let mut confirmed = false;
        let mut cancelled = false;
        egui::Window::new("Confirm Delete").collapsible(false).resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Tooltip)
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(24.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    let key = path.last().cloned().unwrap_or_default();
                    ui.label(egui::RichText::new(format!("Delete \"{}\"?", key)).size(16.0).color(text));
                    ui.label(egui::RichText::new("This will remove the key and all its children.").size(12.0).color(c_muted(dark)));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if danger_button(ui, "Delete").clicked() { confirmed = true; }
                        if compact_button(ui, "Cancel", dark).clicked() { cancelled = true; }
                    });
                });
            });
        if confirmed { self.delete_node(path); self.confirm_delete_path = None; }
        else if cancelled { self.confirm_delete_path = None; }
    }

    fn render_new_confirm_dialog(&mut self, ctx: &egui::Context, dark: bool) {
        if !self.show_new_confirm { return; }
        let (bg, border, text) = if dark { (egui::Color32::from_rgb(26, 26, 32), ColorPalette::ZINC_700, ColorPalette::SLATE_100) }
            else { (egui::Color32::WHITE, ColorPalette::GRAY_300, ColorPalette::GRAY_900) };
        style::draw_modal_overlay(ctx, "je_new_overlay", 160);
        let mut confirmed = false;
        let mut cancelled = false;
        egui::Window::new("New JSON").collapsible(false).resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Tooltip).title_bar(false)
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(24.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Create new empty JSON?").size(16.0).color(text));
                    if self.dirty {
                        ui.label(egui::RichText::new("You have unsaved changes that will be pushed to undo history.")
                            .size(12.0).color(if dark { ColorPalette::AMBER_400 } else { ColorPalette::AMBER_600 }));
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if accent_button(ui, "Create New").clicked() { confirmed = true; }
                        if compact_button(ui, "Cancel", dark).clicked() { cancelled = true; }
                    });
                });
            });
        if confirmed { self.reset_to_empty(); self.show_new_confirm = false; }
        if cancelled { self.show_new_confirm = false; }
    }

    fn do_export(&self) {
        let content = serialize_value(&self.root, self.export_pretty);
        if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).add_filter("All Files", &["*"]).save_file() {
            let _ = std::fs::write(path, content);
        }
    }
}
