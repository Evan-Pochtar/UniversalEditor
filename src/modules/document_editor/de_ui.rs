use eframe::egui;
use crate::style::{ColorPalette, ThemeMode, toolbar_action_btn, toolbar_toggle_btn};
use super::de_main::{DocumentEditor, DocPos};
use crate::modules::EditorModule;
use super::de_tools::*;

const PAGE_GAP: f32 = 28.0;
const PAGE_PAD: f32 = 24.0;

fn selection_rects_for_galley(galley: &egui::Galley,text: &str,start_byte: usize,end_byte: usize) -> Vec<egui::Rect> {
    let start_byte = start_byte.min(text.len());
    let end_byte = end_byte.min(text.len());
    let start_char = text[..start_byte].chars().count();
    let end_char = text[..end_byte].chars().count();
    let mut rects = Vec::new();
    let mut char_pos = 0usize;

    for row in &galley.rows {
        let row_start = char_pos;
        let glyph_count = row.glyphs.len();
        let row_end = char_pos + glyph_count;

        if start_char < row_end && end_char > row_start {
            let local_start = start_char.saturating_sub(row_start).min(glyph_count);
            let local_end = (end_char - row_start).min(glyph_count);

            let x0 = if local_start == 0 {
                row.rect().min.x
            } else {
                row.glyphs.get(local_start).map(|g| g.pos.x).unwrap_or(row.rect().max.x)
            };
            let x1 = if local_end >= glyph_count {
                row.rect().max.x
            } else {
                row.glyphs.get(local_end).map(|g| g.pos.x).unwrap_or(row.rect().max.x)
            };

            if x1 >= x0 {
                rects.push(egui::Rect::from_min_max(egui::pos2(x0, row.rect().min.y), egui::pos2(x1.max(x0 + 4.0), row.rect().max.y)));
            }
        }

        char_pos = row_end;
        if row.ends_with_newline { char_pos += 1; }
    }

    if rects.is_empty() && start_byte == 0 && end_byte >= text.len() {
        if let Some(row) = galley.rows.first() {
            rects.push(egui::Rect::from_min_max(egui::pos2(row.rect().min.x, row.rect().min.y), egui::pos2(row.rect().min.x + 8.0, row.rect().max.y)));
        }
    }

    rects
}

fn render_color_palette(ui: &mut egui::Ui, ed: &mut DocumentEditor, is_dark: bool, popup_id: egui::Id) {
    const PALETTE: &[([u8; 3], &str)] = &[
        ([0,0,0], "Black"), ([68,68,68], "Dark Gray"), ([102,102,102], "Gray"),
        ([153,153,153], "Light Gray"), ([204,204,204], "Silver"), ([255,255,255], "White"),
        ([220,38,38], "Red"), ([234,88,12], "Orange"), ([234,179,8], "Yellow"),
        ([22,163,74], "Green"), ([20,184,166], "Teal"), ([59,130,246], "Blue"),
        ([99,102,241], "Indigo"), ([168,85,247], "Purple"), ([236,72,153], "Pink"), ([120,53,15], "Brown"),
    ];
    let lc = if is_dark { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
    ui.label(egui::RichText::new("Text Color").size(11.0).color(lc));
    ui.add_space(4.0);
    if ui.add(egui::Button::new(egui::RichText::new("Auto (default)").size(11.0)).min_size(egui::vec2(120.0, 20.0))).clicked() {
        ed.apply_fmt_color(None);
        egui::Popup::close_id(ui.ctx(), popup_id);
    }
    ui.add_space(4.0);
    for row in PALETTE.chunks(6) {
        ui.horizontal(|ui| {
            for &(c, name) in row {
                let col = egui::Color32::from_rgb(c[0], c[1], c[2]);
                let border = if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 };
                if ui.add(egui::Button::new("").fill(col)
                    .stroke(egui::Stroke::new(1.0, border))
                    .min_size(egui::vec2(20.0, 20.0))
                    .corner_radius(3.0))
                    .on_hover_text(name).clicked()
                {
                    ed.apply_fmt_color(Some(c));
                    egui::Popup::close_id(ui.ctx(), popup_id);
                }
            }
        });
    }
}

pub fn render(ed: &mut DocumentEditor, ui: &mut egui::Ui, ctx: &egui::Context) {
    let is_dark = ui.visuals().dark_mode;
    let theme = if is_dark { ThemeMode::Dark } else { ThemeMode::Light };
    handle_keyboard(ed, ctx);
    ed.run_find();
    render_toolbar(ed, ui, theme, is_dark);
    ui.separator();
    egui::SidePanel::left("de_outline_panel")
        .resizable(true).default_width(200.0).min_width(140.0).max_width(320.0)
        .frame(egui::Frame::new()
            .fill(if is_dark { egui::Color32::from_rgb(20,20,26) } else { ColorPalette::GRAY_50 })
            .stroke(egui::Stroke::new(1.0, if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 })))
        .show_animated_inside(ui, ed.show_outline, |ui| render_outline(ed, ui, is_dark));
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(if is_dark { egui::Color32::from_rgb(14,14,18) } else { egui::Color32::from_rgb(188,188,196) }))
        .show_inside(ui, |ui| render_canvas(ed, ui, ctx, is_dark));
    render_find_bar(ed, ctx, is_dark);
    render_stats_modal(ed, ctx, is_dark);
    render_page_settings(ed, ctx, is_dark);
}

fn handle_keyboard(ed: &mut DocumentEditor, ctx: &egui::Context) {
    if ed.has_cross_sel() {
        let del = ctx.input_mut(|i| {
            let d = i.key_pressed(egui::Key::Backspace) || i.key_pressed(egui::Key::Delete);
            if d { i.events.retain(|e| !matches!(e, egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } | egui::Event::Key { key: egui::Key::Delete, pressed: true, .. })); }
            d
        });
        if del { ed.delete_sel(); return; }

        let (do_copy, do_cut) = ctx.input_mut(|i| {
            let c = i.events.iter().any(|e| matches!(e, egui::Event::Copy));
            let x = i.events.iter().any(|e| matches!(e, egui::Event::Cut));
            if c || x { i.events.retain(|e| !matches!(e, egui::Event::Copy | egui::Event::Cut)); }
            (c, x)
        });
        if do_copy || do_cut {
            if let Some((from, to)) = ed.norm_sel() { ctx.copy_text(ed.collect_sel_text(from, to)); }
            if do_cut { ed.delete_sel(); return; }
        }

        let has_text = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Text(_))));
        if has_text { ed.delete_sel(); }
    }

    ctx.input_mut(|i| {
        if !ed.has_cross_sel() && i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))) { ed.push_undo(); }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Z) { ed.undo(); }
        if i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z) || i.consume_key(egui::Modifiers::CTRL, egui::Key::Y) { ed.redo(); }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::S) { let _ = ed.save(); }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::F) { ed.show_find = !ed.show_find; }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Plus) || i.consume_key(egui::Modifiers::CTRL, egui::Key::Equals) { ed.zoom = (ed.zoom + 0.1).min(3.0); }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Minus) { ed.zoom = (ed.zoom - 0.1).max(0.3); }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num0) { ed.auto_zoom_done = false; }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::A) {
            let last = ed.paras.len().saturating_sub(1);
            let end = ed.paras.last().map(|p| p.text.len()).unwrap_or(0);
            ed.doc_sel = Some([DocPos { para: 0, byte: 0 }, DocPos { para: last, byte: end }]);
        }
    });
}

fn fmt_btn(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>, active: bool, theme: ThemeMode, tip: &str) -> bool {
    toolbar_toggle_btn(ui, label, active, theme).on_hover_text(tip).clicked()
}
fn act_btn(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>, theme: ThemeMode, tip: &str) -> bool {
    toolbar_action_btn(ui, label, theme).on_hover_text(tip).clicked()
}

fn render_toolbar(ed: &mut DocumentEditor, ui: &mut egui::Ui, theme: ThemeMode, is_dark: bool) {
    let lc = if is_dark { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
    egui::Frame::new()
        .fill(if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_50 })
        .stroke(egui::Stroke::new(1.0, if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 }))
        .corner_radius(6.0).inner_margin(egui::Margin { left: 6, right: 6, top: 3, bottom: 3 })
        .show(ui, |ui| {
            egui::ScrollArea::horizontal().auto_shrink([false, true]).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.interact_size.y = 26.0;
                    let cur_style = ed.paras.get(ed.focused_para).map(|p| p.style).unwrap_or_default();
                    egui::ComboBox::from_id_salt("de_style_cb")
                        .selected_text(egui::RichText::new(cur_style.label()).size(12.0)).width(130.0)
                        .show_ui(ui, |ui| { for s in ParaStyle::all() { if ui.selectable_label(cur_style == *s, s.label()).clicked() { ed.apply_style(*s); } } });
                    egui::ComboBox::from_id_salt("de_font_cb")
                        .selected_text(egui::RichText::new(ed.base_font.label()).size(12.0)).width(80.0)
                        .show_ui(ui, |ui| { for f in FontChoice::all() { ui.selectable_value(&mut ed.base_font, *f, f.label()); } });
                    ui.label(egui::RichText::new("Font Size:").size(11.0).color(lc));
                    let mut sel_sz = ed.sel_font_size_pt();
                    if ui.add(egui::DragValue::new(&mut sel_sz).range(4.0..=288.0).speed(0.5).suffix("pt")).changed() { ed.apply_fmt_size(sel_sz); }
                    ui.separator();
                    if fmt_btn(ui, egui::RichText::new("B").strong().size(13.0), ed.fmt_state_bold(), theme, "Bold (Ctrl+B)") { ed.apply_fmt_toggle_bold(); }
                    if fmt_btn(ui, egui::RichText::new("I").italics().size(13.0), ed.fmt_state_italic(), theme, "Italic (Ctrl+I)") { ed.apply_fmt_toggle_italic(); }
                    if fmt_btn(ui, egui::RichText::new("U").underline().size(13.0), ed.fmt_state_underline(), theme, "Underline (Ctrl+U)") { ed.apply_fmt_toggle_underline(); }
                    if fmt_btn(ui, egui::RichText::new("S").strikethrough().size(13.0), ed.fmt_state_strike(), theme, "Strikethrough") { ed.apply_fmt_toggle_strike(); }
                    if fmt_btn(ui, egui::RichText::new("x\u{00B2}").size(11.0), ed.fmt_state_sup(), theme, "Superscript") { ed.apply_fmt_toggle_sup(); }
                    if fmt_btn(ui, egui::RichText::new("x\u{2082}").size(11.0), ed.fmt_state_sub(), theme, "Subscript") { ed.apply_fmt_toggle_sub(); }
                    ui.separator();
                    let is_bullet = ed.paras.get(ed.focused_para).map(|p| p.style == ParaStyle::ListBullet).unwrap_or(false);
                    let is_num = ed.paras.get(ed.focused_para).map(|p| p.style == ParaStyle::ListOrdered).unwrap_or(false);
                    if fmt_btn(ui, "\u{2022}", is_bullet, theme, "Bullet List") { ed.apply_style_toggle(ParaStyle::ListBullet); }
                    if fmt_btn(ui, "1.", is_num, theme, "Numbered List") { ed.apply_style_toggle(ParaStyle::ListOrdered); }
                    if act_btn(ui, "\u{2014}", theme, "Insert Horizontal Rule") {
                        ed.push_undo();
                        let idx = ed.focused_para;
                        ed.paras.insert(idx + 1, DocParagraph::with_style(ParaStyle::HRule));
                        if idx + 2 >= ed.paras.len() { ed.paras.push(DocParagraph::new()); }
                        ed.focused_para = idx + 2;
                        ed.pending_focus = Some(ed.focused_para);
                        ed.sync_texts(); ed.dirty = true;
                    }
                    ui.separator();
                    let cur_col = ed.cur_fmt.color.map(|c| egui::Color32::from_rgb(c[0],c[1],c[2]))
                        .unwrap_or(if is_dark { ColorPalette::ZINC_200 } else { egui::Color32::from_rgb(22,22,22) });
                    let color_btn = ui.scope(|ui| {
                        let s = ui.style_mut();
                        s.visuals.widgets.inactive.bg_fill = if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_200 };
                        s.visuals.widgets.hovered.bg_fill = if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 };
                        ui.add(egui::Button::new(egui::RichText::new("A").size(13.0).color(cur_col)).min_size(egui::vec2(24.0, 26.0)))
                    }).inner.on_hover_text("Text color");
                    let color_popup_id = color_btn.id;
                    { let r = color_btn.rect; ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(r.min.x+2.0, r.max.y-4.0), egui::vec2(r.width()-4.0, 3.0)), 1.0, cur_col); }
                    egui::Popup::from_toggle_button_response(&color_btn)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            ui.set_min_width(140.0);
                            render_color_palette(ui, ed, is_dark, color_popup_id);
                        });
                    let _ = color_popup_id;
                    ui.separator();
                    let cur_align = ed.paras.get(ed.focused_para).map(|p| p.align).unwrap_or_default();
                    for a in Align::all() {
                        if fmt_btn(ui, egui::RichText::new(a.label()).size(12.0), cur_align == *a, theme, a.full_label()) { ed.apply_align(*a); }
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("LH:").size(11.0).color(lc));
                    if ui.add(egui::DragValue::new(&mut ed.line_spacing_input).range(0.8..=4.0).speed(0.05).fixed_decimals(2)).changed() {
                        ed.paras[ed.focused_para].line_height = ed.line_spacing_input; ed.dirty = true; ed.heights_dirty = true;
                    }
                    ui.separator();
                    if fmt_btn(ui, egui::RichText::new("Outline").size(11.0), ed.show_outline, theme, "Toggle document outline") { ed.show_outline = !ed.show_outline; }
                    if act_btn(ui, egui::RichText::new("Page").size(11.0), theme, "Page layout settings") { ed.page_settings_draft = None; ed.show_page_settings = true; }
                    ui.separator();
                    ui.label(egui::RichText::new("Zoom:").size(11.0).color(lc));
                    if act_btn(ui, "-", theme, "Zoom out (Ctrl+-)") { ed.zoom = (ed.zoom - 0.1).max(0.3); }
                    ui.label(egui::RichText::new(format!("{:.0}%", ed.zoom * 100.0)).size(11.0).color(lc));
                    if act_btn(ui, "+", theme, "Zoom in (Ctrl++)") { ed.zoom = (ed.zoom + 0.1).min(3.0); }
                });
            });
        });
}

fn render_outline(ed: &mut DocumentEditor, ui: &mut egui::Ui, is_dark: bool) {
    let tc = if is_dark { ColorPalette::ZINC_300 } else { ColorPalette::ZINC_800 };
    let muted = if is_dark { ColorPalette::ZINC_500 } else { ColorPalette::ZINC_500 };
    ui.add_space(8.0);
    ui.horizontal(|ui| { ui.add_space(6.0); ui.label(egui::RichText::new("Outline").size(12.0).color(muted).strong()); });
    ui.add_space(4.0); ui.separator(); ui.add_space(4.0);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let entries: Vec<(usize, u8, String)> = ed.paras.iter().enumerate()
            .filter_map(|(i, p)| p.style.outline_depth().map(|d| (i, d, p.text.clone())))
            .collect();
        for (i, depth, text) in entries {
            let sz = (14.0 - depth as f32 * 0.5).max(11.0);
            let display = if text.trim().is_empty() { "(empty)" } else { text.trim() };
            let col = if i == ed.focused_para { ColorPalette::BLUE_400 } else { tc };
            ui.horizontal(|ui| {
                ui.add_space(depth as f32 * 10.0 + 6.0);
                let r = ui.add(egui::Label::new(egui::RichText::new(display).size(sz).color(col)).truncate().sense(egui::Sense::click()));
                if r.clicked() { ed.focused_para = i; ed.scroll_to_para = Some(i); }
                if r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
            });
            ui.add_space(2.0);
        }
    });
}

struct ComputedPageLayout {
    para_page: Vec<usize>,
    para_content_y: Vec<f32>,
    page_tops: Vec<f32>,
}

fn compute_page_layout(ed: &DocumentEditor) -> ComputedPageLayout {
    let page_content_h = ed.layout.content_height() * ed.zoom;
    let page_h = ed.layout.height * ed.zoom;
    let mt = ed.layout.margin_top * ed.zoom;
    let n = ed.paras.len();
    let mut para_page = vec![0usize; n];
    let mut para_content_y = vec![0.0f32; n];
    let mut page_tops = vec![PAGE_PAD];
    let mut cur_y = mt;
    let mut cur_page = 0;

    for i in 0..n {
        let mut h = ed.para_heights.get(i).copied().unwrap_or(ed.base_size * ed.zoom * 1.8);
        if h <= 0.0 { h = ed.base_size * ed.zoom * 1.2; }

        if cur_y > mt && cur_y + h > mt + page_content_h {
            cur_page += 1;
            page_tops.push(*page_tops.last().unwrap() + page_h + PAGE_GAP);
            cur_y = mt;
        }

        para_page[i] = cur_page;
        para_content_y[i] = cur_y;
        cur_y += h;
        if h > page_content_h { cur_y = mt + page_content_h; }
    }

    ComputedPageLayout { para_page, para_content_y, page_tops }
}

fn measure_para_total_height(
    ctx: &egui::Context,
    para: &DocParagraph,
    base_font: FontChoice,
    base_size: f32,
    wrap_w: f32,
    zoom: f32,
    is_dark: bool,
) -> f32 {
    let job = build_layout_job(&para.spans, &para.text, para, base_font, base_size, wrap_w, is_dark, zoom);
    let galley = ctx.fonts_mut(|f| f.layout_job(job));
    para.space_before * zoom + galley.rect.height() + para.space_after * zoom
}

fn split_spans_at_byte(spans: &[DocSpan], split_byte: usize) -> (Vec<DocSpan>, Vec<DocSpan>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut pos = 0usize;

    for s in spans {
        if s.len == 0 { continue; }
        let (start, end) = (pos, pos + s.len);
        if end <= split_byte {
            left.push(s.clone());
        } else if start >= split_byte {
            right.push(s.clone());
        } else {
            let left_len = split_byte - start;
            let right_len = end - split_byte;
            if left_len > 0 { left.push(DocSpan { len: left_len, fmt: s.fmt.clone() }); }
            if right_len > 0 { right.push(DocSpan { len: right_len, fmt: s.fmt.clone() }); }
        }
        pos = end;
    }

    (left, right)
}

fn split_para_at_byte(src: &DocParagraph, split_byte: usize) -> (DocParagraph, DocParagraph) {
    let split_byte = split_byte.min(src.text.len());
    let (left_spans, right_spans) = split_spans_at_byte(&src.spans, split_byte);

    let mut left = src.clone();
    left.text = src.text[..split_byte].to_string();
    left.spans = if left_spans.is_empty() { vec![DocSpan { len: 0, fmt: SpanFmt::default() }] } else { left_spans };
    left.space_after = 0.0;
    merge_adjacent(&mut left);

    let mut right = src.clone();
    right.text = src.text[split_byte..].to_string();
    right.spans = if right_spans.is_empty() { vec![DocSpan { len: 0, fmt: SpanFmt::default() }] } else { right_spans };
    right.space_before = 0.0;
    merge_adjacent(&mut right);

    (left, right)
}

fn adjust_paragraph_indent(ed: &mut DocumentEditor, idx: usize, delta: f32) {
    if idx >= ed.paras.len() { return; }
    ed.paras[idx].indent_left = (ed.paras[idx].indent_left + delta).max(0.0);
}

fn handle_tab_edit(ed: &mut DocumentEditor, ctx: &egui::Context, idx: usize, id: egui::Id, shift: bool, cur_fmt: &SpanFmt) -> bool {
    if idx >= ed.paras.len() { return false; }
    let Some(mut state) = egui::TextEdit::load_state(ctx, id) else { return false; };
    let Some(cr) = state.cursor.char_range() else { return false; };

    let old_text = ed.para_texts.get(idx).cloned().unwrap_or_default();
    let si = cr.primary.index.min(cr.secondary.index);
    let ei = cr.primary.index.max(cr.secondary.index);
    let mut new_text = old_text.clone();
    let mut new_cursor = si;

    if shift {
        if si != ei {
            let s = char_to_byte(&old_text, si);
            let e = char_to_byte(&old_text, ei);
            if old_text[s..e].starts_with('\t') {
                new_text.replace_range(s..s + '\t'.len_utf8(), "");
            } else if old_text[s..e].ends_with('\t') {
                let tab_start = e.saturating_sub('\t'.len_utf8());
                new_text.replace_range(tab_start..e, "");
                new_cursor = si;
            } else {
                return false;
            }
        } else if si > 0 {
            let prev = char_to_byte(&old_text, si - 1);
            let cur = char_to_byte(&old_text, si);
            if &old_text[prev..cur] == "\t" {
                new_text.replace_range(prev..cur, "");
                new_cursor = si - 1;
            } else {
                return false;
            }
        } else {
            return false;
        }
    } else {
        let s = char_to_byte(&old_text, si);
        let e = char_to_byte(&old_text, ei);
        new_text.replace_range(s..e, "\t");
        new_cursor = si + 1;
    }

    ed.push_undo();
    rebuild_spans(&mut ed.paras[idx], new_text.clone(), cur_fmt);
    ed.para_texts[idx] = new_text;
    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(new_cursor))));
    egui::TextEdit::store_state(ctx, id, state);
    ed.dirty = true;
    ed.heights_dirty = true;
    true
}

fn find_split_byte_fit(
    ctx: &egui::Context,
    para: &DocParagraph,
    base_font: FontChoice,
    base_size: f32,
    wrap_w: f32,
    max_total_h: f32,
    zoom: f32,
    is_dark: bool,
) -> usize {
    let mut char_bytes: Vec<usize> = para.text.char_indices().map(|(b, _)| b).collect();
    char_bytes.push(para.text.len());
    if char_bytes.len() <= 2 { return para.text.len(); }

    let (mut lo, mut hi, mut best) = (1usize, char_bytes.len() - 1, char_bytes[1]);
    while lo <= hi {
        let mid = (lo + hi) / 2;
        let split = char_bytes[mid];
        if split == 0 || split >= para.text.len() { break; }
        let (left, _) = split_para_at_byte(para, split);
        let h = measure_para_total_height(ctx, &left, base_font, base_size, wrap_w, zoom, is_dark);
        if h <= max_total_h + 0.5 { best = split; lo = mid + 1; }
        else { if mid == 0 { break; } hi = mid.saturating_sub(1); }
    }
    best
}

fn reflow_overflow_paragraphs(ed: &mut DocumentEditor, ctx: &egui::Context, is_dark: bool) {
    let page_content_h = ed.layout.content_height() * ed.zoom;
    let cw = ed.layout.content_width() * ed.zoom;
    let bs = ed.base_size * ed.zoom;
    let font = ed.base_font;
    let mt = ed.layout.margin_top * ed.zoom;
    let min_fill = bs * 2.0;
    let mut changed = false;
    let mut cur_y = mt;
    let mut i = 0usize;

    while i < ed.paras.len() {
        let para = ed.paras[i].clone();
        if para.style == ParaStyle::HRule {
            let h = para.space_before * ed.zoom + 12.0 + para.space_after * ed.zoom;
            cur_y += h;
            if cur_y >= mt + page_content_h { cur_y = mt; }
            i += 1;
            continue;
        }
        let wrap_w = (cw - para.indent_left * ed.zoom).max(40.0);
        let h = measure_para_total_height(ctx, &para, font, bs, wrap_w, ed.zoom, is_dark);
        let remaining = mt + page_content_h - cur_y;

        if h <= remaining + 0.5 {
            cur_y += h;
            if cur_y >= mt + page_content_h { cur_y = mt; }
            i += 1;
            continue;
        }

        if !para.text.is_empty() && cur_y > mt && remaining > min_fill {
            let split = find_split_byte_fit(ctx, &para, font, bs, wrap_w, remaining, ed.zoom, is_dark);
            if split > 0 && split < para.text.len() {
                let (left, right) = split_para_at_byte(&para, split);
                ed.paras[i] = left;
                ed.paras.insert(i + 1, right);
                changed = true;
                continue;
            }
        }

        cur_y = mt;

        if !para.text.is_empty() && h > page_content_h + 0.5 {
            let split = find_split_byte_fit(ctx, &para, font, bs, wrap_w, page_content_h, ed.zoom, is_dark);
            let split = split.max(para.text.char_indices().nth(1).map(|(b, _)| b).unwrap_or(para.text.len()));
            if split < para.text.len() {
                let (left, right) = split_para_at_byte(&para, split);
                ed.paras[i] = left;
                ed.paras.insert(i + 1, right);
                changed = true;
                continue;
            }
        }

        cur_y = (cur_y + h).min(mt + page_content_h);
        if cur_y >= mt + page_content_h { cur_y = mt; }
        i += 1;
    }

    if changed { ed.sync_texts(); ed.heights_dirty = true; ed.find_stale = true; }
}

fn render_canvas(ed: &mut DocumentEditor, ui: &mut egui::Ui, ctx: &egui::Context, is_dark: bool) {
    let avail_w = ui.available_width();

    if !ed.auto_zoom_done && avail_w > 50.0 {
        ed.zoom = (avail_w * 0.60 / ed.layout.width).clamp(0.3, 2.5);
        ed.auto_zoom_done = true;
        ed.heights_dirty = true;
    }

    let page_w = ed.layout.width * ed.zoom;
    let page_h = ed.layout.height * ed.zoom;
    let ml = ed.layout.margin_left * ed.zoom;
    let mt = ed.layout.margin_top * ed.zoom;
    let mb = ed.layout.margin_bot * ed.zoom;
    let cw = ed.layout.content_width() * ed.zoom;
    let bs = ed.base_size * ed.zoom;
    let font = ed.base_font;
    let n = ed.paras.len();
    reflow_overflow_paragraphs(ed, ctx, is_dark);

    if ed.heights_dirty || ed.para_heights.len() != n {
        ed.para_heights.resize(n, 0.0);
        for i in 0..n {
            let p = &ed.paras[i];
            if p.style == ParaStyle::HRule { ed.para_heights[i] = p.space_before * ed.zoom + 12.0 + p.space_after * ed.zoom; continue; }
            let indent = p.indent_left * ed.zoom;
            let wrap_w = (cw - indent).max(40.0);
            let job = build_layout_job(&p.spans, &p.text, p, font, bs, wrap_w, is_dark, ed.zoom);
            let galley = ctx.fonts_mut(|f| f.layout_job(job));
            ed.para_heights[i] = p.space_before * ed.zoom + galley.rect.height() + p.space_after * ed.zoom;
        }
        ed.heights_dirty = false;
    }

    let pl = compute_page_layout(ed);
    let total_scroll_h = pl.page_tops.last().copied().unwrap_or(PAGE_PAD) + page_h + PAGE_PAD;

    let scroll_target_y = ed.scroll_to_para.take().and_then(|t| {
        pl.para_page.get(t).and_then(|&pg| pl.page_tops.get(pg)).map(|&pt| {
            pt + pl.para_content_y[t] - 80.0
        })
    });

    let cross_sel = ed.norm_sel().filter(|(f, t)| f.para != t.para);
    let ptr = ctx.pointer_hover_pos();
    let btn_pressed = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
    let btn_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
    let shift = ctx.input(|i| i.modifiers.shift);
    let char_w = (bs * 0.55).max(1.0);
    let mut text_change: Option<(usize, String)> = None;
    let mut merge_up: Option<usize> = None;
    let mut merge_down: Option<usize> = None;
    let mut new_selection: Option<(usize, usize, usize)> = None;
    let mut pending_focus_next: Option<usize> = None;

    let page_bg = if is_dark { egui::Color32::from_rgb(38,38,44) } else { egui::Color32::WHITE };
    let page_border = if is_dark { egui::Color32::from_rgb(55,55,66) } else { egui::Color32::from_rgb(190,190,202) };
    let shadow = egui::Color32::from_rgba_unmultiplied(0,0,0,55);
    let margin_line = if is_dark { egui::Color32::from_rgba_unmultiplied(80,110,180,28) } else { egui::Color32::from_rgba_unmultiplied(0,70,180,16) };
    let bq_bg = if is_dark { egui::Color32::from_rgba_unmultiplied(59,130,246,38) } else { egui::Color32::from_rgba_unmultiplied(59,130,246,14) };
    let focus_bg = if is_dark { egui::Color32::from_rgba_unmultiplied(59,130,246,10) } else { egui::Color32::from_rgba_unmultiplied(59,130,246,5) };
    let bullet_col = if is_dark { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
    let code_left = if is_dark { ColorPalette::AMBER_500 } else { ColorPalette::AMBER_600 };
    let sel_color = ctx.style().visuals.selection.bg_fill;

    let focused = ed.focused_para;
    let find_hl: Option<(usize, usize, usize)> = if ed.find_cursor < ed.find_results.len() { Some(ed.find_results[ed.find_cursor]) } else { None };
    let cur_fmt = ed.cur_fmt.clone();

    let mut scroll_area = egui::ScrollArea::vertical().id_salt("de_canvas_scroll").auto_shrink([false, false]);
    if let Some(off) = scroll_target_y { scroll_area = scroll_area.vertical_scroll_offset(off.max(0.0)); }

    scroll_area.show_viewport(ui, |ui, vp| {
        let page_x = ((avail_w - page_w) / 2.0).max(16.0);
        let (outer, _) = ui.allocate_exact_size(egui::vec2(avail_w, total_scroll_h), egui::Sense::hover());
        let painter = ui.painter_at(outer);

        for (pg_idx, &pt) in pl.page_tops.iter().enumerate() {
            if pt + page_h < vp.min.y - 50.0 || pt > vp.max.y + 50.0 { continue; }
            let pm = egui::pos2(outer.min.x + page_x, outer.min.y + pt);
            let pr = egui::Rect::from_min_size(pm, egui::vec2(page_w, page_h));
            painter.rect_filled(pr.translate(egui::vec2(4.0, 4.0)), 3.0, shadow);
            painter.rect_filled(pr, 3.0, page_bg);
            painter.rect_stroke(pr, 3.0, egui::Stroke::new(1.0, page_border), egui::StrokeKind::Outside);
            let mlx = pm.x + ml; let mrx = pm.x + page_w - ed.layout.margin_right * ed.zoom;
            let mty = pm.y + mt; let mby = pm.y + page_h - mb;
            painter.line_segment([egui::pos2(mlx, mty), egui::pos2(mlx, mby)], egui::Stroke::new(0.5, margin_line));
            painter.line_segment([egui::pos2(mrx, mty), egui::pos2(mrx, mby)], egui::Stroke::new(0.5, margin_line));
            if pg_idx > 0 {
                let num_col = if is_dark { ColorPalette::ZINC_500 } else { ColorPalette::ZINC_400 };
                painter.text(egui::pos2(pm.x + page_w / 2.0, pm.y + page_h - mb / 2.0), egui::Align2::CENTER_CENTER,
                    format!("{}", pg_idx + 1), egui::FontId::proportional(9.0 * ed.zoom), num_col);
            }
        }

        for i in 0..n {
            let pg = pl.para_page[i];
            let pt = pl.page_tops[pg];
            let pm = egui::pos2(outer.min.x + page_x, outer.min.y + pt);
            let para = &ed.paras[i];
            let indent = para.indent_left * ed.zoom;
            let wrap_w = (cw - indent).max(40.0);
            let content_y = pl.para_content_y[i];
            let total_h = ed.para_heights.get(i).copied().unwrap_or(bs * 1.8);
            let space_b = para.space_before * ed.zoom;
            let text_y = pm.y + content_y + space_b;
            let text_h = total_h - space_b - para.space_after * ed.zoom;
            let scroll_local_top = pt + content_y;
            let near_view = !(scroll_local_top + total_h < vp.min.y - page_h * 0.5 || scroll_local_top > vp.max.y + page_h * 0.5);

            let (edit_x, edit_w) = if matches!(para.align, Align::Left) {
                (pm.x + ml + indent, wrap_w)
            } else {
                (pm.x + ml, cw)
            };
            let edit_rect = egui::Rect::from_min_size(
                egui::pos2(edit_x, text_y),
                egui::vec2(edit_w, text_h.max(bs * 1.2)),
            );

           if let Some(pp) = ptr {
            if edit_rect.expand(2.0).contains(pp) {
                let job = build_layout_job(&ed.paras[i].spans, &ed.paras[i].text, &ed.paras[i], font, bs, wrap_w, is_dark, ed.zoom);
                let galley = ctx.fonts_mut(|f| f.layout_job(job));
                let rel = pp - egui::pos2(edit_x, text_y);
                let cursor = galley.cursor_from_pos(rel);
                let byte = char_to_byte(&ed.paras[i].text, cursor.index);
                    if btn_pressed {
                        let pos = DocPos { para: i, byte };
                        ed.doc_sel = if shift {
                            ed.doc_sel.map(|[a, _]| [a, pos]).or(Some([pos, pos]))
                        } else {
                            Some([pos, pos])
                        };
                    } else if btn_down {
                        if let Some(ref mut sel) = ed.doc_sel {
                            if sel[0].para != i || (sel[0].para == i && sel[0].byte != byte) {
                                sel[1] = DocPos { para: i, byte };
                            }
                        }
                    }
                }
            }

            if near_view {
                if para.style == ParaStyle::HRule {
                    let rule_col = if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 };
                    let mid_y = text_y + text_h / 2.0;
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(pm.x + ml, mid_y - 1.0), egui::vec2(cw, 2.0)),
                        1.0, rule_col,
                    );
                    if let Some((from, to)) = cross_sel {
                        if i >= from.para && i <= to.para {
                            painter.rect_filled(
                                egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(cw, text_h.max(4.0))),
                                0.0, sel_color,
                            );
                        }
                    }
                    continue;
                }

                if i == focused {
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y - 2.0), egui::vec2(cw, text_h + 4.0)),
                        2.0, focus_bg,
                    );
                }

                match para.style {
                    ParaStyle::BlockQuote => {
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(cw, text_h)),
                            0.0, bq_bg,
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(3.0, text_h)),
                            1.0, ColorPalette::BLUE_500,
                        );
                    }
                    ParaStyle::Code => {
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(cw, text_h)),
                            3.0,
                            if is_dark { egui::Color32::from_rgb(24, 24, 30) } else { egui::Color32::from_rgb(244, 244, 248) },
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(3.0, text_h)),
                            1.0, code_left,
                        );
                    }
                    ParaStyle::ListBullet => {
                        painter.circle_filled(egui::pos2(edit_x - 8.0 * ed.zoom, text_y + text_h / 2.0), 2.5 * ed.zoom, bullet_col);
                    }
                    ParaStyle::ListOrdered => {
                        let num = para.list_num.unwrap_or_else(|| {
                            ed.paras[..i].iter().rev().take_while(|p| p.style == ParaStyle::ListOrdered).count() as u32 + 1
                        });
                        painter.text(
                            egui::pos2(edit_x - 4.0, text_y), egui::Align2::RIGHT_TOP,
                            format!("{}.", num), egui::FontId::proportional(bs * para.style.size_scale() * 0.9), bullet_col,
                        );
                    }
                    _ => {}
                }

                if let Some((fi, fs, fe)) = find_hl {
                    if fi == i {
                        let hx = edit_x + fs as f32 * char_w;
                        let hw = ((fe - fs) as f32 * char_w).max(4.0);
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(hx, text_y), egui::vec2(hw, text_h)),
                            2.0, egui::Color32::from_rgba_unmultiplied(255, 210, 0, 90),
                        );
                    }
                }

                if let Some((from, to)) = cross_sel {
                    if i >= from.para && i <= to.para {
                        let start_byte = if i == from.para { from.byte } else { 0 };
                        let end_byte = if i == to.para { to.byte } else { para.text.len() };
                        let job = build_layout_job(&para.spans, &para.text, para, font, bs, wrap_w, is_dark, ed.zoom);
                        let galley = ctx.fonts_mut(|f| f.layout_job(job));
                        let align_offset = match para.align {
                            Align::Center => ((edit_w - galley.rect.width()) / 2.0).max(0.0),
                            Align::Right  => (edit_w - galley.rect.width()).max(0.0),
                            _ => 0.0,
                        };
                        let base_x = edit_x + align_offset;

                        for rect in selection_rects_for_galley(&galley, &para.text, start_byte, end_byte) {
                            painter.rect_filled(
                                rect.translate(egui::vec2(base_x, text_y)),
                                0.0, sel_color,
                            );
                        }
                    }
                }
            }

            if i == focused {
                let id = ed.para_ids[i];
                let state = egui::TextEdit::load_state(ctx, id);
                let cr = state.as_ref().and_then(|s| s.cursor.char_range());
                let at_start = cr.map(|cr| cr.primary.index == 0 && cr.secondary.index == 0).unwrap_or(false);
                let at_end = cr.map(|cr| {
                    let len = ed.para_texts[i].chars().count();
                    cr.primary.index == len && cr.secondary.index == len
                }).unwrap_or(false);

                let should_handle_bksp = at_start && (ed.paras[i].indent_left > 0.0 || i > 0);
                if should_handle_bksp && ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)) {
                    if ed.paras[i].indent_left > 0.0 {
                        ed.push_undo();
                        ed.paras[i].indent_left = (ed.paras[i].indent_left - 18.0).max(0.0);
                        ed.dirty = true;
                        ed.heights_dirty = true;
                    } else if i > 0 {
                        merge_up = Some(i);
                    }
                    continue;
                }

                if at_end && i + 1 < ed.paras.len() && ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) {
                    merge_down = Some(i);
                    continue;
                }
            }

            if ed.paras[i].style == ParaStyle::HRule { continue; }

            let para_clone = ed.paras[i].clone();
            let spans_clone: Vec<DocSpan> = ed.paras[i].spans.clone();
            let para_clone_layouter = para_clone.clone();
            let spans_clone_layouter = spans_clone.clone();
            let base_size_snap = bs;
            let base_font_snap = font;
            let dark_snap = is_dark;
            let zoom_snap = ed.zoom;
            let mut layouter = move |lui: &egui::Ui, s: &dyn egui::TextBuffer, _ww: f32| {
                let job = build_layout_job(&spans_clone_layouter, s.as_str(), &para_clone_layouter, base_font_snap, base_size_snap, wrap_w, dark_snap, zoom_snap);
                lui.fonts_mut(|f| f.layout_job(job))
            };

            let id = ed.para_ids[i];
            if i == focused && !shift {
                let no_modifiers = ctx.input(|inp| inp.modifiers.is_none());
                let up_pressed = ctx.input(|inp| inp.key_pressed(egui::Key::ArrowUp)) && no_modifiers;
                let down_pressed = ctx.input(|inp| inp.key_pressed(egui::Key::ArrowDown)) && no_modifiers;
                if (up_pressed && i > 0) || (down_pressed && i + 1 < ed.paras.len()) {
                    if let Some(state) = egui::TextEdit::load_state(ctx, id) {
                        if let Some(cr) = state.cursor.char_range() {
                            let job = build_layout_job(&spans_clone, &para_clone.text, &para_clone, base_font_snap, base_size_snap, wrap_w, dark_snap, zoom_snap);
                            let galley = ctx.fonts_mut(|f| f.layout_job(job));
                            let mut cur_x_local = 0.0;
                            let mut is_top = false; let mut is_bottom = false;
                            let mut char_pos = 0usize;
                            for (row_idx, row) in galley.rows.iter().enumerate() {
                                let row_start = char_pos;
                                let glyph_count = row.glyphs.len();
                                let row_end = char_pos + glyph_count;
                                if cr.primary.index >= row_start && cr.primary.index < row_end {
                                    let local_index = cr.primary.index - row_start;
                                    cur_x_local = if local_index == 0 {
                                        row.rect().min.x
                                    } else {
                                        row.glyphs.get(local_index).map(|g| g.pos.x).unwrap_or(row.rect().max.x)
                                    };
                                    is_top = row_idx == 0; is_bottom = row_idx == galley.rows.len() - 1;
                                    break;
                                }
                                char_pos = row_end;
                                if row.ends_with_newline { char_pos += 1; }
                            }
                            
                            let mut nav_to = None;
                            if up_pressed && is_top {
                                ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                                let mut t = i.saturating_sub(1);
                                while t > 0 && ed.paras[t].style == ParaStyle::HRule { t -= 1; }
                                if ed.paras[t].style != ParaStyle::HRule { nav_to = Some((t, true)); }
                            } else if down_pressed && is_bottom {
                                ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
                                let mut t = i + 1;
                                while t + 1 < ed.paras.len() && ed.paras[t].style == ParaStyle::HRule { t += 1; }
                                if t < ed.paras.len() && ed.paras[t].style != ParaStyle::HRule { nav_to = Some((t, false)); }
                            }
                            
                            if let Some((target_i, to_bottom)) = nav_to {
                                let cur_x_abs = edit_x + cur_x_local;
                                let target_para = &ed.paras[target_i];
                                let target_wrap_w = (cw - target_para.indent_left * ed.zoom).max(40.0);
                                let target_job = build_layout_job(&target_para.spans, &target_para.text, target_para, font, bs, target_wrap_w, is_dark, ed.zoom);
                                let target_galley = ctx.fonts_mut(|f| f.layout_job(target_job));
                                let target_edit_x = if matches!(target_para.align, Align::Left) { pm.x + ml + target_para.indent_left * ed.zoom } else { pm.x + ml };
                                let target_x_local = cur_x_abs - target_edit_x;
                                let target_row_idx = if to_bottom { target_galley.rows.len().saturating_sub(1) } else { 0 };
                                if let Some(row) = target_galley.rows.get(target_row_idx) {
                                    let pos = egui::pos2(target_x_local, row.rect().center().y);
                                    let new_gcursor = target_galley.cursor_from_pos(pos.to_vec2());
                                    let target_id = ed.para_ids[target_i];
                                    let mut target_state = egui::TextEdit::load_state(ctx, target_id).unwrap_or_default();
                                    target_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(new_gcursor.index))));
                                    egui::TextEdit::store_state(ctx, target_id, target_state);
                                    ed.pending_focus = Some(target_i); pending_focus_next = Some(target_i);
                                } else {
                                    let target_id = ed.para_ids[target_i];
                                    let mut target_state = egui::TextEdit::load_state(ctx, target_id).unwrap_or_default();
                                    target_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(0))));
                                    egui::TextEdit::store_state(ctx, target_id, target_state);
                                    ed.pending_focus = Some(target_i); pending_focus_next = Some(target_i);
                                }
                            }
                        }
                    }
                }
            }
            if ed.pending_focus == Some(i) { ctx.memory_mut(|m| m.request_focus(id)); }
            let tab_keys = if i == focused {
                ctx.input_mut(|inp| (
                    inp.consume_key(egui::Modifiers::NONE, egui::Key::Tab),
                    inp.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab),
                    inp.consume_key(egui::Modifiers::CTRL, egui::Key::M),
                    inp.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::M),
                ))
            } else { (false, false, false, false) };
            let text_ref = &mut ed.para_texts[i];
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit_rect));
            let output = egui::TextEdit::multiline(text_ref)
                .id(id)
                .desired_width(edit_w)
                .frame(false)
                .lock_focus(true)
                .horizontal_align(para.align.egui_align())
                .layouter(&mut layouter)
                .show(&mut child);

            if ed.has_cross_sel() {
                if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                    if let Some(cr) = state.cursor.char_range() {
                        if cr.primary != cr.secondary {
                            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(cr.primary)));
                            egui::TextEdit::store_state(ctx, id, state);
                        }
                    }
                }
            }

            let has_focus = output.response.has_focus();
            if has_focus && ed.focused_para != i {
                ed.focused_para = i;
                ed.line_spacing_input = ed.paras[i].line_height;
            }
            if has_focus {
                let mut b = false; let mut it = false; let mut u = false; let mut h: u8 = 0;

                ctx.input_mut(|inp| {
                    if inp.consume_key(egui::Modifiers::CTRL, egui::Key::B) { b = true; }
                    if inp.consume_key(egui::Modifiers::CTRL, egui::Key::I) { it = true; }
                    if inp.consume_key(egui::Modifiers::CTRL, egui::Key::U) { u = true; }
                    for (key, level) in [(egui::Key::Num1,1u8),(egui::Key::Num2,2),(egui::Key::Num3,3),(egui::Key::Num4,4),(egui::Key::Num5,5)] {
                        if inp.consume_key(egui::Modifiers::CTRL | egui::Modifiers::ALT, key) { h = level; }
                    }
                });

                if b { ed.apply_fmt_toggle_bold(); }
                if it { ed.apply_fmt_toggle_italic(); }
                if u { ed.apply_fmt_toggle_underline(); }
                if h > 0 {
                    let style = match h { 1 => ParaStyle::H1, 2 => ParaStyle::H2, 3 => ParaStyle::H3, 4 => ParaStyle::H4, _ => ParaStyle::H5 };
                    ed.apply_style_toggle(style);
                }
                if tab_keys.2 || tab_keys.3 {
                    let delta = if tab_keys.2 { 18.0f32 } else { -18.0 };
                    ed.push_undo();
                    if ed.has_cross_sel() {
                        if let Some((from, to)) = ed.norm_sel() {
                            for pi in from.para..=to.para.min(ed.paras.len().saturating_sub(1)) {
                                ed.paras[pi].indent_left = (ed.paras[pi].indent_left + delta).max(0.0);
                            }
                        }
                    } else {
                        adjust_paragraph_indent(ed, i, delta);
                    }
                    ed.dirty = true;
                    ed.heights_dirty = true;
                }
                if (tab_keys.0 || tab_keys.1) && !ed.has_cross_sel() {
                    let _ = handle_tab_edit(ed, ctx, i, id, tab_keys.1, &cur_fmt);
                }

                if let Some(state) = egui::TextEdit::load_state(ctx, id) {
                    if let Some(cr) = state.cursor.char_range() {
                        let si = cr.primary.index.min(cr.secondary.index);
                        let ei = cr.primary.index.max(cr.secondary.index);
                        let sb = char_to_byte(&ed.para_texts[i], si);
                        let eb = char_to_byte(&ed.para_texts[i], ei);
                        new_selection = Some((i, sb, eb));
                        if si == ei {
                            let fmt = para_fmt_at(&ed.paras[i], sb);
                            let c = &mut ed.cur_fmt;
                            c.bold = fmt.bold; c.italic = fmt.italic; c.underline = fmt.underline;
                            c.strike = fmt.strike; c.sub = fmt.sub; c.sup = fmt.sup;
                        }
                    }
                }
            }

            if ed.pending_focus == Some(i) { pending_focus_next = Some(i); }
            if output.response.changed() { text_change = Some((i, ed.para_texts[i].clone())); }
            let new_h = output.galley.size().y;
            if (new_h - text_h).abs() > 0.5 { ed.heights_dirty = true; }
        }
    });

    if let Some(f) = pending_focus_next { ed.focused_para = f.min(ed.paras.len().saturating_sub(1)); ed.pending_focus = None; }
    if let Some(sel) = new_selection { ed.last_selection = Some(sel); }

    if let Some(mu) = merge_up {
        if mu > 0 && mu < ed.paras.len() {
            ed.push_undo();
            if ed.paras[mu - 1].style == ParaStyle::HRule {
                ed.paras.remove(mu - 1);
                ed.focused_para = mu - 1;
                ed.pending_focus = Some(mu - 1);
                ed.sync_texts(); ed.dirty = true; ed.heights_dirty = true;
            } else {
                let prev_len = ed.paras[mu - 1].text.len();
                merge_paragraphs(&mut ed.paras, mu - 1);
                ed.focused_para = mu - 1;
                ed.pending_focus = Some(mu - 1);
                ed.sync_texts();
                let id = ed.para_ids[mu - 1];
                let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                let cc = egui::text::CCursor::new(prev_len);
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(cc)));
                egui::TextEdit::store_state(ctx, id, state);
                ed.dirty = true; ed.heights_dirty = true;
            }
        }
    }

    if let Some(md) = merge_down {
        if md + 1 < ed.paras.len() {
            ed.push_undo();
            if ed.paras[md + 1].style == ParaStyle::HRule {
                ed.paras.remove(md + 1);
                ed.sync_texts(); ed.dirty = true; ed.heights_dirty = true;
            } else {
                let prev_len = ed.paras[md].text.len();
                merge_paragraphs(&mut ed.paras, md);
                ed.sync_texts();
                let id = ed.para_ids[md];
                let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                let cc = egui::text::CCursor::new(prev_len);
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(cc)));
                egui::TextEdit::store_state(ctx, id, state);
                ed.dirty = true; ed.heights_dirty = true;
            }
        }
    }

    if let Some((i, new_text)) = text_change {
        if i < ed.paras.len() {
            if let Some(nl_pos) = new_text.find('\n') {
                ed.push_undo();
                let first = new_text[..nl_pos].to_string();
                let rest = new_text[nl_pos + 1..].to_string();
                rebuild_spans(&mut ed.paras[i], first, &cur_fmt);
                let ns = if ed.paras[i].style.is_heading() { ParaStyle::Normal } else { ed.paras[i].style };
                let mut np = DocParagraph::with_style(ns);
                np.text = rest.clone();
                np.spans = vec![DocSpan { len: rest.len(), fmt: cur_fmt.clone() }];
                np.align = ed.paras[i].align; np.line_height = ed.paras[i].line_height; np.indent_left = ed.paras[i].indent_left;
                ed.paras.insert(i + 1, np);
                ed.focused_para = i + 1;
                ed.pending_focus = Some(i + 1);
                ed.sync_texts();
                let new_id = ed.para_ids[i + 1];
                let mut state = egui::TextEdit::load_state(ctx, new_id).unwrap_or_default();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(0))));
                egui::TextEdit::store_state(ctx, new_id, state);
            } else {
                rebuild_spans(&mut ed.paras[i], new_text, &cur_fmt);
                ed.para_texts[i] = ed.paras[i].text.clone();
            }
            ed.dirty = true; ed.heights_dirty = true; ed.find_stale = true;
        }
    }
}

fn render_find_bar(ed: &mut DocumentEditor, ctx: &egui::Context, is_dark: bool) {
    if !ed.show_find { return; }
    let (bg, border, muted) = if is_dark { (ColorPalette::ZINC_800, ColorPalette::BLUE_600, ColorPalette::ZINC_400) }
        else { (ColorPalette::GRAY_50, ColorPalette::BLUE_600, ColorPalette::ZINC_600) };
    let win = egui::Window::new("Find & Replace")
        .collapsible(false).resizable(false)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 50.0))
        .fixed_size(egui::vec2(340.0, 0.0))
        .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.5, border)).corner_radius(8.0).inner_margin(14.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Find:").size(12.0).color(muted));
                let r = ui.add(egui::TextEdit::singleline(&mut ed.find_text).desired_width(200.0).hint_text("Search..."));
                if r.changed() { ed.find_stale = true; ed.run_find(); }
                if ui.small_button("x").clicked() { ed.find_text.clear(); ed.find_results.clear(); }
            });
            if !ed.find_text.is_empty() {
                let (c, t) = (if ed.find_results.is_empty() { 0 } else { ed.find_cursor + 1 }, ed.find_results.len());
                ui.label(egui::RichText::new(format!("{}/{}", c, t)).size(11.0).color(if t == 0 { ColorPalette::RED_400 } else { muted }));
            }
            ui.horizontal(|ui| { if ui.button("Prev").clicked() { ed.find_prev(); } if ui.button("Next").clicked() { ed.find_next(); } });
            ui.add_space(4.0); ui.separator(); ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Replace:").size(12.0).color(muted));
                ui.add(egui::TextEdit::singleline(&mut ed.replace_text).desired_width(200.0).hint_text("Replacement..."));
            });
            ui.horizontal(|ui| { if ui.button("Replace").clicked() { ed.replace_current(); } if ui.button("Replace All").clicked() { ed.replace_all(); } });
            ui.add_space(4.0);
            if ui.button("Close").clicked() { ed.show_find = false; }
        });

    if let Some(win) = win {
        let clicked_outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().map_or(false, |p| !win.response.rect.contains(p)));
        if clicked_outside { ed.show_find = false; }
    }
}

fn render_stats_modal(ed: &mut DocumentEditor, ctx: &egui::Context, is_dark: bool) {
    if !ed.show_stats { return; }
    crate::style::draw_modal_overlay(ctx, "de_stats_ov", 160);
    let (bg, border, tc, muted) = if is_dark { (ColorPalette::ZINC_900, ColorPalette::ZINC_700, ColorPalette::SLATE_200, ColorPalette::ZINC_400) }
        else { (egui::Color32::WHITE, ColorPalette::GRAY_200, ColorPalette::GRAY_800, ColorPalette::GRAY_500) };
    let mut open = ed.show_stats;
    let win = egui::Window::new("Document Statistics")
        .collapsible(false).resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(24.0))
        .open(&mut open).order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let row = |ui: &mut egui::Ui, lbl: &str, val: String| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(lbl).size(13.0).color(muted));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(egui::RichText::new(val).size(13.0).color(tc)); });
                });
            };
            row(ui, "Words", word_count(&ed.paras).to_string());
            row(ui, "Characters", char_count(&ed.paras).to_string());
            row(ui, "Paragraphs", ed.paras.len().to_string());
            row(ui, "Headings", ed.paras.iter().filter(|p| p.style.is_heading()).count().to_string());
            row(ui, "Page size", format!("{:.0} x {:.0} pt", ed.layout.width, ed.layout.height));
            ui.add_space(8.0); if ui.button("Close").clicked() { ed.show_stats = false; }
        });
    if !open { ed.show_stats = false; }
    if let Some(win) = win {
        let clicked_outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().map_or(false, |p| !win.response.rect.contains(p)));
        if clicked_outside { ed.show_stats = false; }
    }
}

fn render_page_settings(ed: &mut DocumentEditor, ctx: &egui::Context, is_dark: bool) {
    if !ed.show_page_settings { return; }
    if ed.page_settings_draft.is_none() {
        ed.page_settings_draft = Some((ed.layout.clone(), ed.preset_idx, ed.base_size));
    }

    crate::style::draw_modal_overlay(ctx, "de_page_ov", 160);
    let (bg, border, tc, muted) = if is_dark {
        (ColorPalette::ZINC_900, ColorPalette::ZINC_700, ColorPalette::SLATE_200, ColorPalette::ZINC_400)
    } else {
        (egui::Color32::WHITE, ColorPalette::GRAY_200, ColorPalette::GRAY_800, ColorPalette::GRAY_500)
    };
    let sep_col = if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_200 };
    let tag_bg_gd = if is_dark { egui::Color32::from_rgb(24,44,80) } else { ColorPalette::BLUE_50 };
    let tag_bg_wd = if is_dark { egui::Color32::from_rgb(30,40,26) } else { ColorPalette::GREEN_50 };
    let tag_col_gd = if is_dark { ColorPalette::BLUE_300 } else { ColorPalette::BLUE_600 };
    let tag_col_wd = if is_dark { ColorPalette::GREEN_400 } else { ColorPalette::GREEN_700 };
    let draft = ed.page_settings_draft.as_mut().unwrap();
    let mut apply = false;
    let mut cancel = false;
    let mut open = true;

    egui::Window::new("Page Setup")
        .collapsible(false).resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .fixed_size(egui::vec2(400.0, 0.0))
        .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(24.0))
        .open(&mut open).order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.label(egui::RichText::new("Paper Size").size(13.0).strong().color(tc));
            ui.add_space(4.0);

            let presets = PageLayout::presets();
            let cols = 3usize;
            let rows = (presets.len() + cols - 1) / cols;
            let btn_w = (ui.available_width() - (cols as f32 - 1.0) * 6.0) / cols as f32;
            for row in 0..rows {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for col in 0..cols {
                        let idx = row * cols + col;
                        let Some((name, _)) = presets.get(idx) else { continue; };
                        let sel = draft.1 == idx;
                        let (fill, stroke, label_col) = if sel {
                            (if is_dark { egui::Color32::from_rgb(28,52,100) } else { egui::Color32::from_rgb(225,238,255) },
                             egui::Stroke::new(1.5, if is_dark { ColorPalette::BLUE_500 } else { ColorPalette::BLUE_400 }),
                             if is_dark { egui::Color32::WHITE } else { ColorPalette::BLUE_700 })
                        } else {
                            (if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_100 },
                             egui::Stroke::new(1.0, if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 }),
                             tc)
                        };
                        let (tag_bg, tag_col, badge) = match idx {
                            0 => (tag_bg_gd, tag_col_gd, Some("Docs")),
                            1 => (tag_bg_wd, tag_col_wd, Some("Word")),
                            _ => (egui::Color32::TRANSPARENT, egui::Color32::TRANSPARENT, None),
                        };
                        let (cell, _) = ui.allocate_exact_size(egui::vec2(btn_w, 44.0), egui::Sense::hover());
                        if ui.is_rect_visible(cell) {
                            let painter = ui.painter_at(cell);
                            painter.rect(cell, 6.0, fill, stroke, egui::StrokeKind::Inside);
                            let ty = if badge.is_some() { cell.min.y + 13.0 } else { cell.center().y };
                            painter.text(egui::pos2(cell.center().x, ty), egui::Align2::CENTER_CENTER, *name, egui::FontId::proportional(12.0), label_col);
                            if let Some(b) = badge {
                                let bw = 34.0_f32;
                                let br = egui::Rect::from_min_size(egui::pos2(cell.center().x - bw/2.0, cell.max.y - 14.0), egui::vec2(bw, 11.0));
                                painter.rect_filled(br, 3.0, tag_bg);
                                painter.text(br.center(), egui::Align2::CENTER_CENTER, b, egui::FontId::proportional(8.0), tag_col);
                            }
                        }
                        let resp = ui.interact(cell, ui.id().with(("ps", idx)), egui::Sense::click());
                        if resp.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
                        if resp.clicked() { draft.1 = idx; draft.0 = PageLayout::from_preset(idx); }
                    }
                });
            }

            ui.add_space(6.0);
            let sep = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 1.0));
            ui.allocate_rect(sep, egui::Sense::hover());
            ui.painter().rect_filled(sep, 0.0, sep_col);
            ui.add_space(8.0);

            ui.label(egui::RichText::new("Margins (inches)").size(13.0).strong().color(tc));
            ui.add_space(2.0);
            let p = PageLayout::PTS_PER_INCH;
            ui.columns(2, |cols| {
                for (label, pts) in [("Top", &mut draft.0.margin_top), ("Bottom", &mut draft.0.margin_bot)] {
                    cols[0].horizontal(|ui| {
                        ui.add_sized(egui::vec2(52.0, 18.0), egui::Label::new(egui::RichText::new(label).size(12.0).color(muted)));
                        let mut inches = *pts / p;
                        if ui.add(egui::DragValue::new(&mut inches).range(0.0..=4.0).speed(0.01).fixed_decimals(2).suffix("\"")).changed() { *pts = inches * p; }
                    });
                }
                for (label, pts) in [("Left", &mut draft.0.margin_left), ("Right", &mut draft.0.margin_right)] {
                    cols[1].horizontal(|ui| {
                        ui.add_sized(egui::vec2(52.0, 18.0), egui::Label::new(egui::RichText::new(label).size(12.0).color(muted)));
                        let mut inches = *pts / p;
                        if ui.add(egui::DragValue::new(&mut inches).range(0.0..=4.0).speed(0.01).fixed_decimals(2).suffix("\"")).changed() { *pts = inches * p; }
                    });
                }
            });

            ui.add_space(6.0);
            let sep2 = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 1.0));
            ui.allocate_rect(sep2, egui::Sense::hover());
            ui.painter().rect_filled(sep2, 0.0, sep_col);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Base Font Size:").size(12.0).color(muted));
                ui.add(egui::Slider::new(&mut draft.2, 8.0..=24.0).suffix(" pt"));
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Apply").clicked() { apply = true; }
                if ui.button("Cancel").clicked() { cancel = true; }
            });
        });

    if apply {
        let (layout, preset, base_size) = ed.page_settings_draft.take().unwrap();
        ed.layout = layout;
        ed.preset_idx = preset;
        ed.base_size = base_size;
        ed.heights_dirty = true;
        ed.auto_zoom_done = false;
        ed.dirty = true;
        ed.show_page_settings = false;
    } else if cancel || !open {
        ed.page_settings_draft = None;
        ed.show_page_settings = false;
    }
}
