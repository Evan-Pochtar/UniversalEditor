use eframe::egui;
use serde_json::Value;
use crate::style::{self, ColorPalette};
use super::je_main::{JsonEditor, JsonViewMode, EditCell, AddKeyDialog};
use super::je_tools::{
    SortMode, SearchTarget, ValType, FlatNode,
    parse_cell_value, serialize_value, validate_json,
    value_at_path,
};

const ROW_H: f32 = 22.0;

fn c_bg(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(16, 16, 20) } else { egui::Color32::WHITE }
}
fn c_panel(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(22, 22, 28) } else { ColorPalette::GRAY_50 }
}
fn c_border(dark: bool) -> egui::Color32 {
    if dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_200 }
}
fn c_row_alt(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(24, 24, 30) } else { egui::Color32::from_rgb(248, 249, 251) }
}
fn c_row_sel(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(32, 52, 88) } else { ColorPalette::BLUE_100 }
}
fn c_row_match(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(60, 48, 16) } else { ColorPalette::AMBER_100 }
}
fn c_row_match_active(dark: bool) -> egui::Color32 {
    if dark { egui::Color32::from_rgb(90, 72, 16) } else { ColorPalette::AMBER_200 }
}
fn c_key(dark: bool) -> egui::Color32 {
    if dark { ColorPalette::BLUE_300 } else { ColorPalette::BLUE_700 }
}
fn c_text(dark: bool) -> egui::Color32 {
    if dark { ColorPalette::SLATE_200 } else { ColorPalette::GRAY_800 }
}
fn c_muted(dark: bool) -> egui::Color32 {
    if dark { ColorPalette::ZINC_500 } else { ColorPalette::GRAY_400 }
}
fn c_error(dark: bool) -> egui::Color32 {
    if dark { ColorPalette::RED_400 } else { ColorPalette::RED_600 }
}
fn c_string(dark: bool) -> egui::Color32 {
    if dark { ColorPalette::GREEN_400 } else { ColorPalette::GREEN_700 }
}
fn c_number(dark: bool) -> egui::Color32 {
    if dark { ColorPalette::AMBER_300 } else { ColorPalette::AMBER_700 }
}
fn c_bool_null(dark: bool) -> egui::Color32 {
    if dark { ColorPalette::PURPLE_400 } else { ColorPalette::PURPLE_600 }
}
fn c_container(dark: bool) -> egui::Color32 {
    if dark { ColorPalette::TEAL_400 } else { ColorPalette::TEAL_600 }
}

fn val_color(v: &ValType, dark: bool) -> egui::Color32 {
    match v {
        ValType::Null => c_bool_null(dark),
        ValType::Bool(_) => c_bool_null(dark),
        ValType::Number(_) => c_number(dark),
        ValType::Str(_) => c_string(dark),
        ValType::Array(_) => c_container(dark),
        ValType::Object(_) => c_container(dark),
    }
}

fn compact_button(ui: &mut egui::Ui, label: &str, dark: bool) -> egui::Response {
    let (bg, hov, txt) = if dark {
        (egui::Color32::from_rgb(36, 36, 44), egui::Color32::from_rgb(46, 46, 56), ColorPalette::SLATE_200)
    } else {
        (egui::Color32::WHITE, ColorPalette::GRAY_100, ColorPalette::GRAY_800)
    };
    ui.scope(|ui| {
        let s = ui.style_mut();
        s.visuals.widgets.inactive.bg_fill = bg;
        s.visuals.widgets.inactive.weak_bg_fill = bg;
        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, c_border(dark));
        s.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, txt);
        s.visuals.widgets.hovered.bg_fill = hov;
        s.visuals.widgets.hovered.weak_bg_fill = hov;
        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, c_border(dark));
        s.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, txt);
        s.visuals.widgets.active.bg_fill = hov;
        ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(txt))
            .min_size(egui::vec2(0.0, 24.0)))
    }).inner
}

fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.scope(|ui| {
        let s = ui.style_mut();
        let r = ColorPalette::RED_600;
        let rh = ColorPalette::RED_500;
        s.visuals.widgets.inactive.bg_fill = r;
        s.visuals.widgets.inactive.weak_bg_fill = r;
        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        s.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        s.visuals.widgets.hovered.bg_fill = rh;
        s.visuals.widgets.hovered.weak_bg_fill = rh;
        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        s.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        s.visuals.widgets.active.bg_fill = r;
        ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(egui::Color32::WHITE))
            .min_size(egui::vec2(0.0, 24.0)))
    }).inner
}

fn accent_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.scope(|ui| {
        let s = ui.style_mut();
        s.visuals.widgets.inactive.bg_fill = ColorPalette::BLUE_600;
        s.visuals.widgets.inactive.weak_bg_fill = ColorPalette::BLUE_600;
        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        s.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        s.visuals.widgets.hovered.bg_fill = ColorPalette::BLUE_500;
        s.visuals.widgets.hovered.weak_bg_fill = ColorPalette::BLUE_500;
        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        s.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        s.visuals.widgets.active.bg_fill = ColorPalette::BLUE_700;
        ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(egui::Color32::WHITE))
            .min_size(egui::vec2(0.0, 24.0)))
    }).inner
}

fn ghost_btn_small(ui: &mut egui::Ui, label: &str, dark: bool, enabled: bool) -> egui::Response {
    let txt = if enabled { c_text(dark) } else { c_muted(dark) };
    ui.scope(|ui| {
        if !enabled { ui.disable(); }
        let s = ui.style_mut();
        s.visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
        s.visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, if enabled { c_border(dark) } else { c_muted(dark) });
        s.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, txt);
        s.visuals.widgets.hovered.bg_fill = if dark { egui::Color32::from_rgb(34, 34, 42) } else { ColorPalette::GRAY_100 };
        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, c_border(dark));
        s.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, txt);
        s.visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
        ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(txt))
            .min_size(egui::vec2(0.0, 24.0)))
    }).inner
}

fn expand_triangle(ui: &mut egui::Ui, rect: egui::Rect, expanded: bool, dark: bool) -> bool {
    let c: egui::Pos2 = rect.center();
    let s: f32 = 4.5_f32;
    let color: egui::Color32 = c_muted(dark);

    let resp = ui.allocate_rect(rect, egui::Sense::click());
    
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let pts: Vec<egui::Pos2> = if expanded {vec![c + egui::vec2(-s, -s * 0.5), c + egui::vec2(s, -s * 0.5), c + egui::vec2(0.0, s * 0.8)] }
    else { vec![c + egui::vec2(-s * 0.5, -s), c + egui::vec2(-s * 0.5, s), c + egui::vec2(s * 0.8, 0.0)] };

    let tri_color = if resp.hovered() { c_text(dark) } else { color };
    ui.painter().add(egui::Shape::convex_polygon(pts, tri_color, egui::Stroke::NONE));
    
    resp.clicked()
}

fn hline(ui: &mut egui::Ui, dark: bool) {
    let x0 = ui.min_rect().left();
    let x1 = ui.min_rect().right();
    let y  = ui.cursor().top();
    ui.painter().line_segment(
        [egui::pos2(x0, y), egui::pos2(x1, y)],
        egui::Stroke::new(1.0, c_border(dark)),
    );
    ui.add_space(1.0);
}

impl JsonEditor {
    pub(super) fn render_editor_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let dark = ui.visuals().dark_mode;
        self.handle_keyboard(ctx);
        self.rebuild_flat_if_needed();

        if self.text_stale && matches!(self.view_mode, JsonViewMode::Text) { self.sync_text_from_root();}
        self.run_search();
        egui::Frame::new()
            .fill(c_bg(dark))
            .inner_margin(0.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    self.render_toolbar(ui, ctx, dark);
                    hline(ui, dark);
                    self.render_file_strip(ui, dark);
                    hline(ui, dark);

                    if !self.scope_path.is_empty() { self.render_breadcrumbs(ui, dark); hline(ui, dark);}
                    self.render_search_bar(ui, dark);
                    hline(ui, dark);

                    match self.view_mode {
                        JsonViewMode::Tree => self.render_table_view(ui, dark),
                        JsonViewMode::Text => self.render_text_view(ui, ctx, dark),
                    }
                });
            });

        self.render_add_key_dialog(ctx, dark);
        self.render_confirm_delete_dialog(ctx, dark);
        self.render_new_confirm_dialog(ctx, dark);
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input_mut(|i| {
            if i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z)) {
                self.undo();
            }
            if i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Y)) {
                self.redo();
            }
            if i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z))
            {
                self.redo();
            }
        });
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, dark: bool) {
        let panel_bg = c_panel(dark);
        let available_w = ui.available_width();
        egui::Frame::new()
            .fill(panel_bg)
            .inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 6 })
            .show(ui, |ui| {
                ui.set_min_width(available_w - 16.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    self.render_view_tabs(ui, dark);
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    if ghost_btn_small(ui, "Undo", dark, self.can_undo()).clicked() { self.undo(); }
                    if ghost_btn_small(ui, "Redo", dark, self.can_redo()).clicked() { self.redo(); }

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.label(egui::RichText::new("Sort:").size(12.0).color(c_muted(dark)));
                    egui::ComboBox::from_id_salt("jp_sort")
                        .selected_text(egui::RichText::new(sort_label(self.sort_mode)).size(12.0))
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for m in [SortMode::None, SortMode::KeyAsc, SortMode::KeyDesc,
                                      SortMode::ValueAsc, SortMode::ValueDesc]
                            {
                                if ui.selectable_value(&mut self.sort_mode, m, sort_label(m))
                                    .changed()
                                {
                                    self.invalidate_flat();
                                }
                            }
                        });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    if compact_button(ui, "Expand All", dark).clicked() { self.expand_all(); }
                    if compact_button(ui, "Collapse All", dark).clicked() { self.collapse_all(); }

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    if accent_button(ui, "+ Add Key").clicked() {
                        self.add_dialog = Some(AddKeyDialog {
                            parent_path: self.scope_path.clone(),
                            key_buf: String::new(),
                            val_buf: String::new(),
                            error: None,
                        });
                    }

                    if compact_button(ui, "New", dark).clicked() { self.show_new_confirm = true; }

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.label(egui::RichText::new("Format:").size(12.0).color(c_muted(dark)));
                    let fmt_label = if self.export_pretty { "Pretty" } else { "Compact" };
                    egui::ComboBox::from_id_salt("jp_fmt")
                        .selected_text(egui::RichText::new(fmt_label).size(12.0))
                        .width(80.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.export_pretty, true, "Pretty");
                            ui.selectable_value(&mut self.export_pretty, false, "Compact");
                        });

                    if compact_button(ui, "Export", dark).clicked() { self.do_export(); }
                });
            });
    }

    fn render_view_tabs(&mut self, ui: &mut egui::Ui, dark: bool) {
        for (mode, label) in [
            (JsonViewMode::Tree, "Tree"),
            (JsonViewMode::Text, "Text"),
        ] {
            let selected = self.view_mode == mode;
            let (bg, txt) = if selected { (ColorPalette::BLUE_600, egui::Color32::WHITE) } 
            else { (egui::Color32::TRANSPARENT, c_muted(dark)) };

            ui.scope(|ui| {
                let s = ui.style_mut();
                s.visuals.widgets.inactive.bg_fill = bg;
                s.visuals.widgets.inactive.weak_bg_fill = bg;
                s.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, if selected { bg } else { c_border(dark) });
                s.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, txt);
                s.visuals.widgets.hovered.bg_fill = if selected { ColorPalette::BLUE_500 } else { if dark { egui::Color32::from_rgb(34, 34, 42) } else { ColorPalette::GRAY_100 } };
                s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, c_border(dark));
                s.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, if selected { egui::Color32::WHITE } else { c_text(dark) });
                s.visuals.widgets.active.bg_fill = bg;

                let resp = ui.add(
                    egui::Button::new(egui::RichText::new(label).size(12.0).color(txt)).min_size(egui::vec2(52.0, 24.0))
                );
                if resp.clicked() && !selected {
                    if self.view_mode == JsonViewMode::Text { let _ = self.commit_text_to_root(); }
                    self.view_mode = mode;
                    if mode == JsonViewMode::Text { self.sync_text_from_root(); }
                }
            });
        }
    }

    fn render_file_strip(&self, ui: &mut egui::Ui, dark: bool) {
        egui::Frame::new()
            .fill(c_panel(dark))
            .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 4 })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(egui::RichText::new(self.get_file_name()).size(12.0).color(c_text(dark)));
                    ui.separator();

                    let (status, color) = if self.dirty { ("Modified", if dark { ColorPalette::AMBER_400 } else { ColorPalette::AMBER_600 }) } 
                    else {("Saved", if dark { ColorPalette::GREEN_400 } else { ColorPalette::GREEN_600 }) };

                    ui.label(egui::RichText::new(status).size(12.0).color(color));
                    ui.separator();
                    let node_count = self.flat.len();
                    ui.label(egui::RichText::new(format!("{} visible nodes", node_count)).size(12.0).color(c_muted(dark)));
                    if !self.undo_stack.is_empty() {
                        ui.separator();
                        ui.label(egui::RichText::new(format!("{} undo steps", self.undo_stack.len())).size(12.0).color(c_muted(dark)));
                    }
                });
            });
    }

    fn render_breadcrumbs(&mut self, ui: &mut egui::Ui, dark: bool) {
        let accent = ColorPalette::BLUE_500;
        let muted = c_muted(dark);
        let sep_clr = c_border(dark);

        egui::Frame::new()
            .fill(c_panel(dark))
            .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 4 })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    if ui.add(egui::Label::new(
                        egui::RichText::new("root").size(12.0).color(accent)
                    ).sense(egui::Sense::click())).clicked()
                    {
                        self.scope_path.clear();
                        self.invalidate_flat();
                        self.text_stale = true;
                    }

                    for i in 0..self.scope_path.len() {
                        ui.label(egui::RichText::new("/").size(12.0).color(sep_clr));
                        let seg = self.scope_path[i].clone();
                        let is_last = i + 1 == self.scope_path.len();
                        let color = if is_last { c_text(dark) } else { accent };
                        if ui.add(egui::Label::new(
                            egui::RichText::new(&seg).size(12.0).color(color)
                        ).sense(egui::Sense::click())).clicked() && !is_last
                        {
                            self.scope_path.truncate(i + 1);
                            self.invalidate_flat();
                            self.text_stale = true;
                        }
                    }

                    ui.add_space(8.0);
                    if ui.add(egui::Label::new(
                        egui::RichText::new("Back").size(12.0).color(muted)
                    ).sense(egui::Sense::click())).clicked()
                    {
                        self.scope_up();
                    }
                });
            });
    }

    fn render_search_bar(&mut self, ui: &mut egui::Ui, dark: bool) {
        let panel_bg = c_panel(dark);
        egui::Frame::new()
            .fill(panel_bg)
            .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 4 })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(egui::RichText::new("Search:").size(12.0).color(c_muted(dark)));
                    let prev_query = self.search_query.clone();
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
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

                    egui::ComboBox::from_id_salt("jp_search_target")
                        .selected_text(egui::RichText::new(search_target_label(self.search_target)).size(12.0))
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            for t in [SearchTarget::Both, SearchTarget::Keys, SearchTarget::Values] {
                                if ui.selectable_value(&mut self.search_target, t, search_target_label(t))
                                    .changed()
                                {
                                    self.search_stale = true;
                                    self.run_search();
                                }
                            }
                        });

                    let has = !self.search_results.is_empty();
                    if ghost_btn_small(ui, "Prev", dark, has).clicked() { self.search_prev(); }
                    if ghost_btn_small(ui, "Next", dark, has).clicked() { self.search_next(); }

                    if !self.search_query.is_empty() {
                        let count = self.search_results.len();
                        let cur   = if count > 0 { self.search_cursor + 1 } else { 0 };
                        ui.label(egui::RichText::new(format!("{}/{}", cur, count))
                            .size(12.0)
                            .color(if count == 0 { c_error(dark) } else { c_muted(dark) }));
                    }

                    if !self.search_query.is_empty() {
                        if compact_button(ui, "Clear", dark).clicked() {
                            self.search_query.clear();
                            self.search_results.clear();
                            self.search_stale = false;
                        }
                    }
                });
            });
    }

    fn render_table_view(&mut self, ui: &mut egui::Ui, dark: bool) {
        let flat_len: usize = self.flat.len();
        if flat_len == 0 { self.render_empty_state(ui, dark); return; }

        let result_set: std::collections::HashSet<usize> =
            self.search_results.iter().cloned().collect();
        let active_match = self.search_results
            .get(self.search_cursor)
            .cloned();

        let mut toggle_path: Option<Vec<String>> = None;
        let mut drill_path: Option<Vec<String>> = None;
        let mut delete_path: Option<Vec<String>> = None;
        let mut begin_edit: Option<(usize, bool)> = None;
        let mut commit_edit: bool = false;

        let available_w: f32 = ui.available_width();
        let col_key_w: f32 = (available_w * 0.35).max(120.0).min(300.0);
        let col_type_w: f32 = 70.0;
        let col_val_w: f32 = (available_w - col_key_w - col_type_w - 62.0).max(100.0);

        egui::Frame::new()
            .fill(c_panel(dark))
            .inner_margin(0.0)
            .show(ui, |ui| {
                let header_h = 28.0;
                let (header_rect, _) = ui.allocate_exact_size(
                    egui::vec2(available_w, header_h), egui::Sense::hover());
                ui.painter().rect_filled(header_rect, 0.0, c_panel(dark));
                ui.painter().line_segment(
                    [header_rect.left_bottom(), header_rect.right_bottom()],
                    egui::Stroke::new(1.0, c_border(dark)),
                );
                let cy = header_rect.center().y;
                let cols = [
                    (6.0, "Key", col_key_w),
                    (col_key_w + 6.0, "Type", col_type_w),
                    (col_key_w + col_type_w + 6.0, "Value", col_val_w),
                ];
                for (x, label, _) in &cols {
                    ui.painter().text(
                        egui::pos2(header_rect.min.x + x, cy),
                        egui::Align2::LEFT_CENTER,
                        *label,
                        egui::FontId::proportional(11.5),
                        c_muted(dark),
                    );
                }
            });

        let total_h: f32 = flat_len as f32 * ROW_H;
        let current_edit: Option<EditCell> = self.edit_cell.clone();
        egui::ScrollArea::vertical()
            .id_salt("jp_table_scroll")
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let (total_rect, _) = ui.allocate_exact_size(egui::vec2(available_w, total_h), egui::Sense::hover());
                let first: usize = ((viewport.min.y / ROW_H) as usize).saturating_sub(2);
                let last: usize  = (((viewport.max.y / ROW_H) as usize) + 3).min(flat_len);
                let rows: Vec<(usize, FlatNode)> = (first..last)
                    .filter_map(|i| self.flat.get(i).map(|n| (i, n.clone())))
                    .collect();

                for (i, node) in rows {
                    let y_top = total_rect.min.y + i as f32 * ROW_H;
                    let row_rect = egui::Rect::from_min_size(
                        egui::pos2(total_rect.min.x, y_top),
                        egui::vec2(available_w, ROW_H),
                    );
                    if !ui.is_rect_visible(row_rect) { continue; }

                    let is_selected: bool = self.selected_row == Some(i);
                    let is_match: bool = result_set.contains(&i);
                    let is_active: bool = active_match == Some(i);
                    let bg = if is_selected { c_row_sel(dark) }
                        else if is_active { c_row_match_active(dark) }
                        else if is_match { c_row_match(dark) }
                        else if i % 2 == 1 { c_row_alt(dark) }
                        else { egui::Color32::TRANSPARENT };

                    if bg != egui::Color32::TRANSPARENT { ui.painter().rect_filled(row_rect, 0.0, bg); }

                    let cx: f32 = row_rect.min.x;
                    let cy: f32 = row_rect.center().y;
                    let indent: f32 = node.depth as f32 * 12.0 + 6.0;
                    let editing_k = current_edit.as_ref()
                        .filter(|e| e.path == node.path && e.editing_key)
                        .is_some();
                    let key_x = cx + indent;
                    let key_avail = col_key_w - indent - 4.0;

                    if editing_k {
                        if let Some(ec) = &mut self.edit_cell {
                            let er = egui::Rect::from_min_size(
                                egui::pos2(key_x, cy - 9.0), egui::vec2(key_avail, 18.0));
                            let r = ui.put(er, egui::TextEdit::singleline(&mut ec.buffer)
                                .font(egui::FontId::proportional(12.0)));
                            if r.lost_focus() { commit_edit = true; }
                        }
                    } else {
                        if node.has_children {
                            let tri = egui::Rect::from_min_size(
                                egui::pos2(key_x, row_rect.min.y), egui::vec2(16.0, ROW_H));
                            if expand_triangle(ui, tri, node.is_expanded, dark) {
                                toggle_path = Some(node.path.clone());
                            }
                        }
                        let tx = if node.has_children { key_x + 16.0 } else { key_x + 2.0 };
                        ui.painter().text(egui::pos2(tx, cy), egui::Align2::LEFT_CENTER,
                            &node.key, egui::FontId::proportional(12.5), c_key(dark));
                        let key_sense_x = if node.has_children { key_x + 16.0 } else { key_x };
                        let key_sense_w = (col_key_w - indent - if node.has_children { 16.0 } else { 0.0 }).max(0.0);
                        let kr = egui::Rect::from_min_size(
                            egui::pos2(key_sense_x, row_rect.min.y), egui::vec2(key_sense_w, ROW_H));
                        let ks = ui.allocate_rect(kr, egui::Sense::click());
                        if ks.double_clicked() { begin_edit = Some((i, true)); }
                        if ks.clicked() { self.selected_row = Some(i); }
                    }

                    let div1_x = cx + col_key_w;
                    ui.painter().line_segment(
                        [egui::pos2(div1_x, row_rect.min.y), egui::pos2(div1_x, row_rect.max.y)],
                        egui::Stroke::new(1.0, c_border(dark)),
                    );

                    let type_x = div1_x + 4.0;
                    ui.painter().text(egui::pos2(type_x, cy), egui::Align2::LEFT_CENTER,
                        node.val_type.type_label(),
                        egui::FontId::proportional(11.0), c_muted(dark));

                    let div2_x = div1_x + col_type_w;
                    ui.painter().line_segment(
                        [egui::pos2(div2_x, row_rect.min.y), egui::pos2(div2_x, row_rect.max.y)],
                        egui::Stroke::new(1.0, c_border(dark)),
                    );

                    let val_x = div2_x + 4.0;
                    let editing_v = current_edit.as_ref()
                        .filter(|e| e.path == node.path && !e.editing_key)
                        .is_some();

                    if editing_v {
                        if let Some(ec) = &mut self.edit_cell {
                            let er = egui::Rect::from_min_size(
                                egui::pos2(val_x, cy - 9.0), egui::vec2(col_val_w - 4.0, 18.0));
                            let r = ui.put(er, egui::TextEdit::singleline(&mut ec.buffer)
                                .font(egui::FontId::proportional(12.0)));
                            if r.lost_focus() { commit_edit = true; }
                        }
                    } else {
                        let col = val_color(&node.val_type, dark);
                        ui.painter().text(egui::pos2(val_x, cy), egui::Align2::LEFT_CENTER,
                            node.val_type.preview_str(),
                            egui::FontId::proportional(12.5), col);

                        let vr = egui::Rect::from_min_size(
                            egui::pos2(val_x, row_rect.min.y), egui::vec2(col_val_w - 4.0, ROW_H));
                        let vs = ui.allocate_rect(vr, egui::Sense::click());
                        if vs.double_clicked() {
                            if node.has_children { drill_path = Some(node.path.clone()); }
                            else { begin_edit = Some((i, false)); }
                        }
                        if vs.clicked() { self.selected_row = Some(i); }
                    }

                    let hov = ui.rect_contains_pointer(row_rect);
                    if hov || is_selected {
                        let bx = row_rect.max.x - 56.0;

                        if node.has_children {
                            let add_r = egui::Rect::from_min_size(
                                egui::pos2(bx, cy - 10.0), egui::vec2(24.0, 20.0));
                            let ar = ui.allocate_rect(add_r, egui::Sense::click());
                            let ac = if ar.hovered() { ColorPalette::BLUE_400 } else { c_muted(dark) };
                            ui.painter().text(ar.rect.center(), egui::Align2::CENTER_CENTER,
                                "+", egui::FontId::proportional(14.0), ac);
                            if ar.clicked() {
                                self.add_dialog = Some(AddKeyDialog {
                                    parent_path: node.path.clone(),
                                    key_buf: String::new(),
                                    val_buf: String::new(),
                                    error: None,
                                });
                            }
                        }

                        let del_r = egui::Rect::from_min_size(
                            egui::pos2(bx + 28.0, cy - 10.0), egui::vec2(24.0, 20.0));
                        let dr = ui.allocate_rect(del_r, egui::Sense::click());
                        let dc = if dr.hovered() { ColorPalette::RED_400 } else { c_muted(dark) };
                        ui.painter().text(dr.rect.center(), egui::Align2::CENTER_CENTER,
                            "x", egui::FontId::proportional(12.0), dc);
                        if dr.clicked() { delete_path = Some(node.path.clone()); }
                    }

                    ui.painter().line_segment(
                        [row_rect.left_bottom(), row_rect.right_bottom()],
                        egui::Stroke::new(0.5, c_border(dark)),
                    );
                }
            });

        if let Some(path) = toggle_path  { self.toggle_expand(&path); }
        if let Some(path) = drill_path   { self.drill_into(path); }
        if let Some(path) = delete_path  { self.confirm_delete_path = Some(path); }
        if commit_edit { self.commit_inline_edit(); }
        if let Some((row, ek)) = begin_edit {
            if let Some(node) = self.flat.get(row) {
                let init = if ek { node.key.clone() } else { node.val_type.preview_str() };
                self.edit_cell = Some(EditCell {
                    path: node.path.clone(),
                    buffer: init,
                    editing_key: ek,
                });
            }
        }
    }

    fn render_text_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, dark: bool) {
        if self.text_stale { self.sync_text_from_root(); }
        if !self.text_content.is_empty() {
            let errs = validate_json(&self.text_content);
            if errs != self.text_errors {
                self.text_errors = errs;
            }
        } else {
            self.text_errors.clear();
        }

        let monospace = egui::FontId::new(13.0, egui::FontFamily::Monospace);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .id_salt("jp_text_scroll")
            .show(ui, |ui| {
                let resp = ui.add_sized(
                    ui.available_size(),
                    egui::TextEdit::multiline(&mut self.text_content)
                        .font(monospace)
                        .frame(false)
                        .lock_focus(true),
                );
                if resp.changed() {
                    let errs = validate_json(&self.text_content);
                    self.text_errors = errs;
                }
            });

        let has_errors = !self.text_errors.is_empty();
        let anim_id = ui.id().with("text_err_anim");
        let alpha = ctx.animate_bool_with_time(anim_id, has_errors, 0.25);

        if alpha > 0.001 {
            let screen_rect = ctx.content_rect();
            let err_msg = self.text_errors.first()
                .map(|(line, msg)| format!("Line {}: {}", line, msg))
                .unwrap_or_default();

            let pad: f32 = 14.0;
            let max_w: f32 = 380.0;
            let font: egui::FontId = egui::FontId::proportional(12.0);
            let approx_w: f32 = (err_msg.len() as f32 * 6.5).min(max_w).max(120.0);
            let box_w: f32 = approx_w + pad * 2.0;
            let box_h: f32 = 34.0;
            let margin: f32 = 12.0;

            let box_rect = egui::Rect::from_min_size(
                egui::pos2(screen_rect.min.x + margin, screen_rect.max.y - box_h - margin),
                egui::vec2(box_w, box_h),
            );

            let alpha_u8 = (alpha * 255.0) as u8;
            let bg_color = egui::Color32::from_rgba_unmultiplied(
                if dark { 120 } else { 200 },
                if dark { 20 } else { 30 },
                if dark { 20 } else { 30 },
                alpha_u8,
            );
            let border_color = egui::Color32::from_rgba_unmultiplied(
                if dark { 220 } else { 185 },
                if dark { 60 } else { 50 },
                if dark { 60 } else { 50 },
                alpha_u8,
            );
            let text_color = egui::Color32::from_rgba_unmultiplied(255, 200, 200, alpha_u8);

            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, anim_id));
            painter.rect(box_rect, egui::CornerRadius::same(8), bg_color, egui::Stroke::new(1.0, border_color), egui::StrokeKind::Outside);
            painter.text(
                egui::pos2(box_rect.min.x + pad, box_rect.center().y),
                egui::Align2::LEFT_CENTER,
                &err_msg,
                font,
                text_color,
            );

            if alpha > 0.001 && alpha < 0.999 {
                ctx.request_repaint();
            }
        }
    }

    fn render_empty_state(&self, ui: &mut egui::Ui, dark: bool) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(egui::RichText::new("No data")
                    .size(22.0).color(c_muted(dark)));
                ui.add_space(8.0);
                ui.label(egui::RichText::new(
                    "The JSON is empty or all nodes are filtered. Use the toolbar to add a key or load a file.")
                    .size(13.0).color(c_muted(dark)));
            });
        });
    }

    fn commit_inline_edit(&mut self) {
        if let Some(ec) = self.edit_cell.take() {
            if ec.editing_key {
                if !ec.path.is_empty() {
                    let (parent, key_seg) = ec.path.split_at(ec.path.len() - 1);
                    let old_key = &key_seg[0];
                    if &ec.buffer != old_key && !ec.buffer.is_empty() {
                        self.rename_node_key(parent, old_key, &ec.buffer);
                    }
                }
            } else {
                let new_val = parse_cell_value(&ec.buffer);
                self.set_node_value(&ec.path, new_val);
            }
        }
    }

    fn render_add_key_dialog(&mut self, ctx: &egui::Context, dark: bool) {
        let open = self.add_dialog.is_some();
        if !open { return; }

        let (bg, border, text) = if dark {
            (egui::Color32::from_rgb(26, 26, 32), ColorPalette::ZINC_700, ColorPalette::SLATE_100)
        } else {
            (egui::Color32::WHITE, ColorPalette::GRAY_300, ColorPalette::GRAY_900)
        };

        style::draw_modal_overlay(ctx, "jp_add_overlay", 160);

        let mut close: bool = false;
        let mut do_add: bool = false;
        egui::Window::new("Add Key")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Tooltip)
            .frame(egui::Frame::new()
                .fill(bg)
                .stroke(egui::Stroke::new(1.0, border))
                .corner_radius(10.0)
                .inner_margin(24.0))
            .show(ctx, |ui| {
                if let Some(dialog) = &mut self.add_dialog {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 10.0;
                        let path_str = if dialog.parent_path.is_empty() { "root".to_string() } 
                            else { dialog.parent_path.join(" / ") };

                        ui.label(egui::RichText::new(format!("Parent: {}", path_str))
                            .size(12.0).color(c_muted(dark)));

                        let is_array = value_at_path(&self.root, &dialog.parent_path)
                            .map(|v| v.is_array())
                            .unwrap_or(false);
                        if !is_array {
                            ui.label(egui::RichText::new("Key name:").size(13.0).color(text));
                            ui.add(egui::TextEdit::singleline(&mut dialog.key_buf)
                                .desired_width(280.0)
                                .hint_text("Enter key name..."));
                        }

                        ui.label(egui::RichText::new("Value (JSON):").size(13.0).color(text));
                        ui.add(egui::TextEdit::multiline(&mut dialog.val_buf)
                            .desired_width(280.0)
                            .desired_rows(3)
                            .hint_text("null  /  true  /  42  /  \"hello\"  /  {}  /  []"));

                        if let Some(err) = &dialog.error {
                            ui.label(egui::RichText::new(err).size(12.0).color(c_error(dark)));
                        }

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
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

                let is_array = value_at_path(&self.root, &dialog.parent_path)
                    .map(|v| v.is_array())
                    .unwrap_or(false);

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
        let path = match &self.confirm_delete_path {
            Some(p) => p.clone(),
            None => return,
        };

        let (bg, border, text) = if dark {
            (egui::Color32::from_rgb(26, 26, 32), ColorPalette::ZINC_700, ColorPalette::SLATE_100)
        } else {
            (egui::Color32::WHITE, ColorPalette::GRAY_300, ColorPalette::GRAY_900)
        };

        style::draw_modal_overlay(ctx, "jp_del_overlay", 160);

        let mut confirmed: bool = false;
        let mut cancelled: bool = false;
        egui::Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Tooltip)
            .frame(egui::Frame::new()
                .fill(bg)
                .stroke(egui::Stroke::new(1.0, border))
                .corner_radius(10.0)
                .inner_margin(24.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 10.0;
                    let key = path.last().cloned().unwrap_or_default();
                    ui.label(egui::RichText::new(format!("Delete \"{}\"?", key))
                        .size(16.0).color(text));
                    ui.label(egui::RichText::new("This will remove the key and all its children.")
                        .size(12.0).color(c_muted(dark)));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if danger_button(ui, "Delete").clicked() { confirmed = true; }
                        if compact_button(ui, "Cancel", dark).clicked() { cancelled = true; }
                    });
                });
            });

        if confirmed {
            self.delete_node(path);
            self.confirm_delete_path = None;
        } else if cancelled {
            self.confirm_delete_path = None;
        }
    }

    fn render_new_confirm_dialog(&mut self, ctx: &egui::Context, dark: bool) {
        if !self.show_new_confirm { return; }

        let (bg, border, text) = if dark {
            (egui::Color32::from_rgb(26, 26, 32), ColorPalette::ZINC_700, ColorPalette::SLATE_100)
        } else {
            (egui::Color32::WHITE, ColorPalette::GRAY_300, ColorPalette::GRAY_900)
        };

        style::draw_modal_overlay(ctx, "jp_new_overlay", 160);

        let mut confirmed: bool = false;
        let mut cancelled: bool = false;
        egui::Window::new("New JSON")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Tooltip)
            .frame(egui::Frame::new()
                .fill(bg)
                .stroke(egui::Stroke::new(1.0, border))
                .corner_radius(10.0)
                .inner_margin(24.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 10.0;
                    ui.label(egui::RichText::new("Create new empty JSON?")
                        .size(16.0).color(text));
                    if self.dirty {
                        ui.label(egui::RichText::new(
                            "You have unsaved changes that will be pushed to undo history.")
                            .size(12.0).color(if dark { ColorPalette::AMBER_400 } else { ColorPalette::AMBER_600 }));
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
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
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .add_filter("All Files", &["*"])
            .save_file()
        {
            let _ = std::fs::write(path, content);
        }
    }
}

fn sort_label(m: SortMode) -> &'static str {
    match m {
        SortMode::None => "None",
        SortMode::KeyAsc => "Key A-Z",
        SortMode::KeyDesc => "Key Z-A",
        SortMode::ValueAsc => "Value A-Z",
        SortMode::ValueDesc => "Value Z-A",
    }
}

fn search_target_label(t: SearchTarget) -> &'static str {
    match t {
        SearchTarget::Both => "Both",
        SearchTarget::Keys => "Keys",
        SearchTarget::Values => "Values",
    }
}
