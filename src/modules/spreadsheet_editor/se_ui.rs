use eframe::egui;
use crate::style::{self, ColorPalette, ThemeMode, toolbar_action_btn, toolbar_toggle_btn};
use super::se_main::{SpreadsheetEditor, SheetViewMode};
use super::se_model::{HAlign, NumFmt};
use super::se_formula::{self, FVal};
use super::{se_tools, se_style};

const ROW_H: f32 = 24.0;
const HEADER_H: f32 = 24.0;
const ROWNUM_W: f32 = 46.0;
const TEXT_COLORS: &[([u8;3],&str)] = &[([0,0,0],"Black"),([220,38,38],"Red"),([234,88,12],"Orange"),([234,179,8],"Yellow"),([22,163,74],"Green"),([20,184,166],"Teal"),([59,130,246],"Blue"),([168,85,247],"Purple")];
const BG_COLORS: &[([u8;3],&str)] = &[([255,255,255],"White"),([254,242,242],"Red 50"),([255,251,235],"Amber 50"),([240,253,244],"Green 50"),([240,253,250],"Teal 50"),([239,246,255],"Blue 50"),([250,245,255],"Purple 50"),([243,244,246],"Gray")];

pub fn render(se: &mut SpreadsheetEditor, ui: &mut egui::Ui, ctx: &egui::Context, _show_toolbar: bool, show_file_info: bool) {
    let dark = ui.visuals().dark_mode;
    handle_keyboard(se, ctx);
    if se.calc_dirty { se.calc_cache.clear(); se.calc_dirty = false; }
    if se.text_dirty && matches!(se.view_mode, SheetViewMode::Text) { se.sync_text(); }
    render_toolbar(se, ui, dark);
    ui.separator();
    if show_file_info { render_file_info(se, ui, dark); ui.separator(); }
    render_search_bar(se, ui);
    ui.separator();
    egui::TopBottomPanel::bottom("se_tabs_panel").frame(egui::Frame::new()).show_inside(ui, |ui| { ui.separator(); render_sheet_tabs(se, ui, dark); });
    match se.view_mode { SheetViewMode::Table => render_grid(se, ui, ctx, dark), SheetViewMode::Text => render_text(se, ui) }
}

fn handle_keyboard(se: &mut SpreadsheetEditor, ctx: &egui::Context) {
    let (undo_k, redo_k) = ctx.input_mut(|i| (i.consume_key(egui::Modifiers::CTRL, egui::Key::Z), i.consume_key(egui::Modifiers::CTRL, egui::Key::Y)));
    if undo_k { se.undo(); }
    if redo_k { se.redo(); }
    if se.editing.is_some() { return; }
    let Some((anchor,(r,c))) = se.sel else { return };
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::A)) {
        let (mr, mc) = { let s = se.sheet(); (s.max_used_row(), s.max_used_col()) };
        se.sel = Some(((0,0),(mr,mc)));
        return;
    }
    let (del,enter,tab,up,down,left,right,shift,typed) = ctx.input_mut(|i| {
        let shift = i.modifiers.shift;
        let m = if shift { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
        let del = i.consume_key(egui::Modifiers::NONE, egui::Key::Delete) || i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace);
        let enter = i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) || i.consume_key(egui::Modifiers::NONE, egui::Key::F2);
        let tab = i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
        let up = i.consume_key(m, egui::Key::ArrowUp);
        let down = i.consume_key(m, egui::Key::ArrowDown);
        let left = i.consume_key(m, egui::Key::ArrowLeft);
        let right = i.consume_key(m, egui::Key::ArrowRight);
        let mut ch = None;
        i.events.retain(|e| { if ch.is_none() { if let egui::Event::Text(t) = e { if !t.trim().is_empty() { ch = Some(t.clone()); return false; } } } true });
        (del, enter, tab, up, down, left, right, shift, ch)
    });
    if del { se.clear_selection(); return; }
    if enter { se.begin_edit(r, c, None); return; }
    if let Some(t) = typed { se.begin_edit(r, c, Some(t)); return; }
    let mut nr = r; let mut nc = c;
    if up { nr = r.saturating_sub(1); }
    if down { nr = r + 1; }
    if left { nc = c.saturating_sub(1); }
    if right || tab { nc = c + 1; }
    if nr != r || nc != c {
        let new_anchor = if shift { anchor } else { (nr,nc) };
        se.sel = Some((new_anchor,(nr,nc))); se.scroll_to = Some((nr,nc));
    }
}

fn render_toolbar(se: &mut SpreadsheetEditor, ui: &mut egui::Ui, dark: bool) {
    let theme = if dark { ThemeMode::Dark } else { ThemeMode::Light };
    ui.horizontal(|ui| {
        for (mode, label) in [(SheetViewMode::Table,"Table"), (SheetViewMode::Text,"Text")] {
            if toolbar_toggle_btn(ui, label, se.view_mode == mode, theme).clicked() {
                if se.view_mode == SheetViewMode::Text && mode == SheetViewMode::Table { se.commit_text(); }
                se.view_mode = mode;
                if mode == SheetViewMode::Text { se.sync_text(); }
            }
        }
        ui.separator();
        if toolbar_action_btn(ui, "Undo", theme).clicked() { se.undo(); }
        if toolbar_action_btn(ui, "Redo", theme).clicked() { se.redo(); }
        ui.separator();
        if toolbar_toggle_btn(ui, egui::RichText::new("B").strong(), se.cur_fmt.bold, theme).clicked() { let v = !se.cur_fmt.bold; se.cur_fmt.bold = v; se.apply_fmt(move |f| f.bold = v); }
        if toolbar_toggle_btn(ui, egui::RichText::new("I").italics(), se.cur_fmt.italic, theme).clicked() { let v = !se.cur_fmt.italic; se.cur_fmt.italic = v; se.apply_fmt(move |f| f.italic = v); }
        if toolbar_toggle_btn(ui, egui::RichText::new("U").underline(), se.cur_fmt.underline, theme).clicked() { let v = !se.cur_fmt.underline; se.cur_fmt.underline = v; se.apply_fmt(move |f| f.underline = v); }
        ui.separator();
        for (a,label) in [(HAlign::Left,"L"),(HAlign::Center,"C"),(HAlign::Right,"R")] {
            if toolbar_toggle_btn(ui, label, se.cur_fmt.align == a, theme).clicked() { se.cur_fmt.align = a; se.apply_fmt(move |f| f.align = a); }
        }
        ui.separator();
        let txt_col = se.cur_fmt.color.map(|c| egui::Color32::from_rgb(c[0],c[1],c[2])).unwrap_or(if dark {ColorPalette::ZINC_200} else {egui::Color32::from_rgb(30,30,30)});
        let tbtn = ui.add(egui::Button::new(egui::RichText::new("A").color(txt_col)).min_size(egui::vec2(26.0,24.0)));
        let tpicked: std::cell::Cell<Option<Option<[u8;3]>>> = std::cell::Cell::new(None);
        egui::Popup::from_toggle_button_response(&tbtn).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            style::color_palette_grid(ui, TEXT_COLORS, 6, Some("Automatic"), dark, |c| tpicked.set(Some(c)));
        });
        if let Some(c) = tpicked.get() { se.cur_fmt.color = c; se.apply_fmt(move |f| f.color = c); }

        let bg_col = se.cur_fmt.bg.map(|c| egui::Color32::from_rgb(c[0],c[1],c[2])).unwrap_or(if dark {ColorPalette::ZINC_700} else {ColorPalette::GRAY_200});
        let bbtn = ui.add(egui::Button::new(egui::RichText::new("Fill")).fill(bg_col).min_size(egui::vec2(36.0,24.0)));
        let bpicked: std::cell::Cell<Option<Option<[u8;3]>>> = std::cell::Cell::new(None);
        egui::Popup::from_toggle_button_response(&bbtn).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            style::color_palette_grid(ui, BG_COLORS, 6, Some("No Fill"), dark, |c| bpicked.set(Some(c)));
        });
        if let Some(c) = bpicked.get() { se.cur_fmt.bg = c; se.apply_fmt(move |f| f.bg = c); }

        ui.separator();
        egui::ComboBox::from_id_salt("se_numfmt").selected_text(se.cur_fmt.numfmt.label()).width(90.0).show_ui(ui, |ui| {
            for nf in NumFmt::all() { if ui.selectable_label(se.cur_fmt.numfmt == *nf, nf.label()).clicked() { se.cur_fmt.numfmt = *nf; let v = *nf; se.apply_fmt(move |f| f.numfmt = v); } }
        });
        ui.separator();
        let (sr, sc) = se.sel.map(|(a,_)| a).unwrap_or((0,0));
        if toolbar_action_btn(ui, "+Row", theme).on_hover_text("Insert Row Above").clicked() { se.insert_row(sr); }
        if toolbar_action_btn(ui, "-Row", theme).on_hover_text("Delete Row").clicked() { se.delete_row(sr); }
        if toolbar_action_btn(ui, "+Col", theme).on_hover_text("Insert Column Left").clicked() { se.insert_col(sc); }
        if toolbar_action_btn(ui, "-Col", theme).on_hover_text("Delete Column").clicked() { se.delete_col(sc); }
        if toolbar_action_btn(ui, "Sort A-Z", theme).clicked() { se.sort_col(sc, true); }
        if toolbar_action_btn(ui, "Sort Z-A", theme).clicked() { se.sort_col(sc, false); }
    });
    render_formula_bar(se, ui);
}

fn render_formula_bar(se: &mut SpreadsheetEditor, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let (r,c) = se.sel.map(|(a,_)| a).unwrap_or((0,0));
        ui.label(egui::RichText::new(se_tools::cell_ref(r,c)).monospace().strong());
        ui.separator();
        let mut buf = se.sheet_raw(r,c);
        let resp = ui.add(egui::TextEdit::singleline(&mut buf).desired_width(ui.available_width()-4.0).font(egui::FontId::monospace(13.0)));
        if resp.lost_focus() { se.set_cell(r,c,buf); }
    });
}

fn render_file_info(se: &SpreadsheetEditor, ui: &mut egui::Ui, dark: bool) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(se.file_name()).size(12.0));
        ui.separator();
        let (status,color) = if se.dirty { ("Modified", if dark {ColorPalette::AMBER_400} else {ColorPalette::AMBER_600}) } else { ("Saved", if dark {ColorPalette::GREEN_400} else {ColorPalette::GREEN_600}) };
        ui.label(egui::RichText::new(status).size(12.0).color(color));
        ui.separator();
        ui.label(egui::RichText::new(format!("{} cells", se.sheet_cell_count())).size(12.0));
    });
}

fn render_search_bar(se: &mut SpreadsheetEditor, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Find:");
        let prev = se.search_query.clone();
        ui.add(egui::TextEdit::singleline(&mut se.search_query).desired_width(160.0));
        if se.search_query != prev { se.run_search(); }
        if !se.search_query.is_empty() { ui.label(format!("{}/{}", if se.search_results.is_empty() {0} else {se.search_cursor+1}, se.search_results.len())); }
        if ui.button("Prev").clicked() { se.search_prev(); }
        if ui.button("Next").clicked() { se.search_next(); }
    });
}

fn render_sheet_tabs(se: &mut SpreadsheetEditor, ui: &mut egui::Ui, dark: bool) {
    ui.horizontal(|ui| {
        let mut switch_to = None; let mut delete_idx = None;
        let names: Vec<String> = se.sheets.iter().map(|s| s.name.clone()).collect();
        for (i, name) in names.iter().enumerate() {
            let active = i == se.active_sheet;
            let (bg, txt) = if active { (ColorPalette::BLUE_600, egui::Color32::WHITE) } else if dark { (ColorPalette::ZINC_800, ColorPalette::ZINC_300) } else { (ColorPalette::GRAY_200, ColorPalette::GRAY_800) };
            let resp = ui.add(egui::Button::new(egui::RichText::new(name).size(12.0).color(txt)).fill(bg).stroke(egui::Stroke::NONE).min_size(egui::vec2(0.0,24.0)));
            if resp.clicked() { switch_to = Some(i); }
            resp.context_menu(|ui| {
                if ui.button("Rename").clicked() { se.rename_buf = Some((i, name.clone())); ui.close(); }
                if ui.add_enabled(se.sheets.len()>1, egui::Button::new("Delete")).clicked() { delete_idx = Some(i); ui.close(); }
            });
        }
        if ui.button("+").on_hover_text("Add sheet").clicked() { se.add_sheet(); }
        if let Some(i) = switch_to { se.switch_sheet(i); }
        if let Some(i) = delete_idx { se.delete_sheet(i); }
    });
    if let Some((idx, mut name)) = se.rename_buf.clone() {
        style::draw_modal_overlay(ui.ctx(), "se_rename_ov", 160);
        let mut close = false; let mut save = false;
        egui::Window::new("Rename Sheet").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0,0.0)).order(egui::Order::Tooltip).show(ui.ctx(), |ui| {
            ui.text_edit_singleline(&mut name);
            ui.horizontal(|ui| { if ui.button("Save").clicked() { save = true; } if ui.button("Cancel").clicked() { close = true; } });
        });
        if save && !name.trim().is_empty() { se.sheets[idx].name = name.trim().to_string(); se.dirty = true; close = true; }
        se.rename_buf = if close { None } else { Some((idx, name)) };
    }
}

fn render_text(se: &mut SpreadsheetEditor, ui: &mut egui::Ui) {
    egui::ScrollArea::both().auto_shrink([false,false]).show(ui, |ui| {
        let resp = ui.add(egui::TextEdit::multiline(&mut se.text_content).font(egui::FontId::monospace(13.0)).desired_width(f32::INFINITY));
        if resp.changed() { se.dirty = true; }
        if resp.lost_focus() { se.commit_text(); }
    });
}

fn cell_at_pos(pos: egui::Pos2, origin: egui::Pos2, col_x: &[f32], n_cols: u32, n_rows: u32) -> Option<(u32,u32)> {
    let rx = pos.x - origin.x - ROWNUM_W;
    let ry = pos.y - origin.y - HEADER_H;
    if rx < 0.0 || ry < 0.0 { return None; }
    let r = (ry / ROW_H) as u32;
    if r >= n_rows { return None; }
    let c = col_x.partition_point(|&x| x <= rx).saturating_sub(1) as u32;
    if c >= n_cols { return None; }
    Some((r, c))
}

fn render_grid(se: &mut SpreadsheetEditor, ui: &mut egui::Ui, ctx: &egui::Context, dark: bool) {
    let idx = se.active_sheet;
    let n_rows = se.sheets[idx].rows.max(se.sheets[idx].max_used_row()+40);
    let n_cols = se.sheets[idx].cols.max(se.sheets[idx].max_used_col()+12);
    let col_w: Vec<f32> = (0..n_cols).map(|c| se.sheets[idx].col_width(c)).collect();
    let mut col_x = vec![0.0f32; n_cols as usize + 1];
    for c in 0..n_cols as usize { col_x[c+1] = col_x[c] + col_w[c]; }
    let total_w = col_x[n_cols as usize]; let total_h = n_rows as f32 * ROW_H;
    let shift = ctx.input(|i| i.modifiers.shift);

    let mut scroll_area = egui::ScrollArea::both().id_salt("se_grid").auto_shrink([false,false]);
    if let Some((r,c)) = se.scroll_to.take() {
        scroll_area = scroll_area.vertical_scroll_offset((r as f32*ROW_H-ROW_H*4.0).max(0.0)).horizontal_scroll_offset((col_x[c as usize]-100.0).max(0.0));
    }

    scroll_area.show_viewport(ui, |ui, viewport| {
        let (outer, _) = ui.allocate_exact_size(egui::vec2(total_w+ROWNUM_W, total_h+HEADER_H), egui::Sense::hover());
        let painter = ui.painter_at(outer);

        let pointer_pos = ctx.input(|i| i.pointer.latest_pos());
        let primary_pressed = ctx.input(|i| i.pointer.primary_pressed());
        let primary_down = ctx.input(|i| i.pointer.primary_down());
        if let Some(cell) = pointer_pos.filter(|p| outer.contains(*p)).and_then(|p| cell_at_pos(p, outer.min, &col_x, n_cols, n_rows)) {
            if primary_pressed {
                let anchor = if shift { se.sel.map(|(a,_)| a).unwrap_or(cell) } else { cell };
                se.sel = Some((anchor, cell));
            } else if primary_down {
                if let Some((anchor,_)) = se.sel { se.sel = Some((anchor, cell)); }
            }
        }

        let r0 = ((viewport.min.y-HEADER_H).max(0.0)/ROW_H) as u32;
        let r1 = (((viewport.max.y-HEADER_H).max(0.0)/ROW_H) as u32 + 2).min(n_rows);
        let c0 = col_x.partition_point(|&x| x < viewport.min.x-ROWNUM_W).saturating_sub(1) as u32;
        let c1 = (col_x.partition_point(|&x| x < viewport.max.x-ROWNUM_W) as u32 + 1).min(n_cols);

        let sel = se.sel;
        let editing_pos = se.editing.as_ref().map(|(r,c,_)| (*r,*c));
        let mut begin_at = None; let mut commit_edit = false; let mut cancel_edit = false;
        let mut commit_move: Option<(u32,u32,i32,i32)> = None;

        for r in r0..r1 {
            let ry = outer.min.y + HEADER_H + r as f32*ROW_H;
            let alt = r % 2 == 1;
            for c in c0..c1 {
                let rx = outer.min.x + ROWNUM_W + col_x[c as usize];
                let rect = egui::Rect::from_min_size(egui::pos2(rx,ry), egui::vec2(col_w[c as usize], ROW_H));
                if !ui.is_rect_visible(rect) { continue; }
                let is_editing = editing_pos == Some((r,c));
                let fmt = se.sheets[idx].fmt(r,c);
                let in_sel = sel.map_or(false, |((r0_,c0_),(r1_,c1_))| { let (a,b)=(r0_.min(r1_),r0_.max(r1_)); let (x,y)=(c0_.min(c1_),c0_.max(c1_)); r>=a&&r<=b&&c>=x&&c<=y });
                painter.rect_filled(rect, 0.0, se_style::cell_bg(dark, fmt.bg, alt));
                if in_sel { painter.rect_filled(rect, 0.0, se_style::sel_fill(dark)); }
                painter.rect_stroke(rect, 0.0, egui::Stroke::new(0.5_f32, se_style::grid_line(dark)), egui::StrokeKind::Outside);

                if is_editing {
                    if let Some((_,_,val)) = se.editing.as_mut() {
                        let er = ui.put(rect.shrink(1.0), egui::TextEdit::singleline(val).font(egui::FontId::monospace(12.5)));
                        if se.editing_focus_pending { er.request_focus(); se.editing_focus_pending = false; }
                        if er.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) { cancel_edit = true; }
                        else if er.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { commit_edit = true; commit_move = Some((r,c,1,0)); }
                        else if er.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Tab)) { commit_edit = true; commit_move = Some((r,c,0,1)); }
                        else if er.lost_focus() { commit_edit = true; }
                    }
                } else {
                    let mut stack = Vec::new();
                    let val = se_formula::eval_cell(&se.sheets[idx], r, c, &mut se.calc_cache, &mut stack);
                    let disp = se_formula::format_display(&val, fmt.numfmt);
                    if !disp.is_empty() {
                        let numeric = matches!(val, FVal::Num(_));
                        let (align, pos) = match fmt.align {
                            HAlign::Center => (egui::Align2::CENTER_CENTER, rect.center()),
                            HAlign::Right => (egui::Align2::RIGHT_CENTER, egui::pos2(rect.max.x-4.0, rect.center().y)),
                            HAlign::Left => (egui::Align2::LEFT_CENTER, egui::pos2(rect.min.x+4.0, rect.center().y)),
                            HAlign::General => if numeric { (egui::Align2::RIGHT_CENTER, egui::pos2(rect.max.x-4.0, rect.center().y)) } else { (egui::Align2::LEFT_CENTER, egui::pos2(rect.min.x+4.0, rect.center().y)) },
                        };
                        let color = se_style::cell_text(dark, fmt.color);
                        let name = match (fmt.bold, fmt.italic) { (true,true)=>"Ubuntu-BoldItalic", (true,false)=>"Ubuntu-Bold", (false,true)=>"Ubuntu-Italic", _=>"Ubuntu" };
                        let font = egui::FontId::new(12.5, egui::FontFamily::Name(name.into()));
                        let text_rect = painter.text(pos, align, &disp, font, color);
                        if fmt.underline { painter.line_segment([text_rect.left_bottom(), text_rect.right_bottom()], egui::Stroke::new(1.0_f32, color)); }
                    }
                    let resp = ui.interact(rect, ui.id().with(("se_cell", r, c)), egui::Sense::click());
                    if resp.double_clicked() { begin_at = Some((r,c)); }
                }
            }
        }

        for c in c0..c1 {
            let x = outer.min.x + ROWNUM_W + col_x[c as usize];
            let rect = egui::Rect::from_min_size(egui::pos2(x, outer.min.y+viewport.min.y.max(0.0)), egui::vec2(col_w[c as usize], HEADER_H));
            painter.rect_filled(rect, 0.0, se_style::header_bg(dark));
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(0.5_f32, se_style::grid_line(dark)), egui::StrokeKind::Outside);
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, se_tools::col_label(c), egui::FontId::proportional(12.0), se_style::header_text(dark));
            let resize_rect = egui::Rect::from_min_size(egui::pos2(rect.max.x-3.0, rect.min.y), egui::vec2(6.0, HEADER_H));
            let hresp = ui.interact(resize_rect, ui.id().with(("se_colrz", c)), egui::Sense::drag());
            if hresp.hovered() || hresp.dragged() { ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal); }
            if hresp.dragged() { let cur = se.sheets[idx].col_width(c); let nw = (cur+hresp.drag_delta().x).max(30.0); se.sheets[idx].col_widths.insert(c, nw); se.dirty = true; }
            let hdr_resp = ui.interact(rect, ui.id().with(("se_colhdr", c)), egui::Sense::click());
            if hdr_resp.clicked() { se.sel = Some(((0,c),(n_rows.saturating_sub(1),c))); }
            hdr_resp.context_menu(|ui| {
                if ui.button("Sort A-Z").clicked() { se.sort_col(c, true); ui.close(); }
                if ui.button("Sort Z-A").clicked() { se.sort_col(c, false); ui.close(); }
                if ui.button("Insert Column Left").clicked() { se.insert_col(c); ui.close(); }
                if ui.button("Delete Column").clicked() { se.delete_col(c); ui.close(); }
            });
        }

        for r in r0..r1 {
            let y = outer.min.y + HEADER_H + r as f32*ROW_H;
            let rect = egui::Rect::from_min_size(egui::pos2(outer.min.x+viewport.min.x.max(0.0), y), egui::vec2(ROWNUM_W, ROW_H));
            painter.rect_filled(rect, 0.0, se_style::header_bg(dark));
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(0.5_f32, se_style::grid_line(dark)), egui::StrokeKind::Outside);
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, format!("{}", r+1), egui::FontId::proportional(11.0), se_style::header_text(dark));
            let rn_resp = ui.interact(rect, ui.id().with(("se_rowhdr", r)), egui::Sense::click());
            if rn_resp.clicked() { se.sel = Some(((r,0),(r,n_cols.saturating_sub(1)))); }
            rn_resp.context_menu(|ui| {
                if ui.button("Insert Row Above").clicked() { se.insert_row(r); ui.close(); }
                if ui.button("Delete Row").clicked() { se.delete_row(r); ui.close(); }
            });
        }

        let corner = egui::Rect::from_min_size(egui::pos2(outer.min.x+viewport.min.x.max(0.0), outer.min.y+viewport.min.y.max(0.0)), egui::vec2(ROWNUM_W, HEADER_H));
        painter.rect_filled(corner, 0.0, se_style::header_bg(dark));

        if let Some((r,c)) = begin_at { se.begin_edit(r,c,None); }
        if commit_edit { se.commit_edit(); }
        if cancel_edit { se.cancel_edit(); }
        if let Some((er,ec,dr,dc)) = commit_move {
            let nr = (er as i32 + dr).max(0) as u32;
            let nc = (ec as i32 + dc).max(0) as u32;
            se.sel = Some(((nr,nc),(nr,nc))); se.scroll_to = Some((nr,nc));
        }
    });

    let (do_copy, do_cut, paste) = ctx.input(|i| {
        let c = i.events.iter().any(|e| matches!(e, egui::Event::Copy));
        let x = i.events.iter().any(|e| matches!(e, egui::Event::Cut));
        let p = i.events.iter().find_map(|e| if let egui::Event::Paste(t) = e { Some(t.clone()) } else { None });
        (c, x, p)
    });
    if do_copy || do_cut { se.copy_selection(ctx); if do_cut { se.clear_selection(); } }
    if let Some(text) = paste { se.paste_text(&text); }
}
