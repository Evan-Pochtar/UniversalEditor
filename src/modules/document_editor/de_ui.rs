use eframe::egui;
use crate::style::{ColorPalette, ThemeMode, toolbar_action_btn, toolbar_toggle_btn};
use super::de_main::{DocumentEditor, DocPos};
use crate::modules::EditorModule;
use super::de_tools::*;

const PAGE_GAP: f32 = 28.0;
const PAGE_PAD: f32 = 24.0;

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
        let consumed = ctx.input_mut(|i| {
            let del = i.key_pressed(egui::Key::Backspace) || i.key_pressed(egui::Key::Delete);
            if del {
                i.events.retain(|e| !matches!(e,
                    egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } |
                    egui::Event::Key { key: egui::Key::Delete, pressed: true, .. }
                ));
            }
            del
        });
        if consumed { ed.delete_sel(); return; }

        let typed: Vec<String> = ctx.input(|i| i.events.iter().filter_map(|e| {
            if let egui::Event::Text(t) = e { Some(t.clone()) } else { None }
        }).collect());
        if !typed.is_empty() { ed.delete_sel(); }
    }

    ctx.input_mut(|i| {
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
                    if act_btn(ui, egui::RichText::new("Page").size(11.0), theme, "Page layout settings") { ed.show_page_settings = true; }
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
        let mut h = ed
            .para_heights
            .get(i)
            .copied()
            .unwrap_or(ed.base_size * ed.zoom * 1.8);

        if h <= 0.0 {
            h = ed.base_size * ed.zoom * 1.2;
        }

        if cur_y > mt && cur_y + h > mt + page_content_h {
            cur_page += 1;
            page_tops.push(*page_tops.last().unwrap() + page_h + PAGE_GAP);
            cur_y = mt;
        }

        para_page[i] = cur_page;
        para_content_y[i] = cur_y;
        cur_y += h;
        if h > page_content_h {
            cur_y = mt + page_content_h;
        }
    }

    ComputedPageLayout {
        para_page,
        para_content_y,
        page_tops,
    }
}

fn approx_byte(text: &str, rel_x: f32, char_w: f32) -> usize {
    let idx = (rel_x / char_w.max(1.0)) as usize;
    text.char_indices().nth(idx).map(|(b, _)| b).unwrap_or(text.len())
}

fn draw_cross_sel_for_para(painter: &egui::Painter, from: DocPos, to: DocPos, pi: usize, rect: egui::Rect, text: &str, char_w: f32) {
    if pi < from.para || pi > to.para { return; }
    let byte_to_x = |byte: usize| -> f32 {
        let n = text[..byte.min(text.len())].chars().count();
        (rect.min.x + n as f32 * char_w).clamp(rect.min.x, rect.max.x)
    };
    let x0 = if pi == from.para { byte_to_x(from.byte) } else { rect.min.x };
    let x1 = if pi == to.para { byte_to_x(to.byte) } else { rect.max.x };
    if x0 < x1 {
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(x0, rect.min.y), egui::pos2(x1, rect.max.y)),
            2.0,
            egui::Color32::from_rgba_unmultiplied(66, 133, 244, 100),
        );
    }
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
        if s.len == 0 {
            continue;
        }

        let start = pos;
        let end = pos + s.len;

        if end <= split_byte {
            left.push(s.clone());
        } else if start >= split_byte {
            right.push(s.clone());
        } else {
            let left_len = split_byte - start;
            let right_len = end - split_byte;

            if left_len > 0 {
                left.push(DocSpan { len: left_len, fmt: s.fmt.clone() });
            }
            if right_len > 0 {
                right.push(DocSpan { len: right_len, fmt: s.fmt.clone() });
            }
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
    left.spans = if left_spans.is_empty() {
        vec![DocSpan { len: 0, fmt: SpanFmt::default() }]
    } else {
        left_spans
    };
    left.space_after = 0.0;
    merge_adjacent(&mut left);

    let mut right = src.clone();
    right.text = src.text[split_byte..].to_string();
    right.spans = if right_spans.is_empty() {
        vec![DocSpan { len: 0, fmt: SpanFmt::default() }]
    } else {
        right_spans
    };
    right.space_before = 0.0;
    merge_adjacent(&mut right);

    (left, right)
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

    if char_bytes.len() <= 2 {
        return para.text.len();
    }

    let mut lo = 1usize;
    let mut hi = char_bytes.len() - 1;
    let mut best = char_bytes[1];

    while lo <= hi {
        let mid = (lo + hi) / 2;
        let split = char_bytes[mid];

        if split == 0 || split >= para.text.len() {
            break;
        }

        let (left, _) = split_para_at_byte(para, split);
        let h = measure_para_total_height(ctx, &left, base_font, base_size, wrap_w, zoom, is_dark);

        if h <= max_total_h + 0.5 {
            best = split;
            lo = mid + 1;
        } else {
            if mid == 0 {
                break;
            }
            hi = mid.saturating_sub(1);
        }
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
            let abs_top = pm.y + content_y;
            let near_view = !(abs_top + total_h < vp.min.y - page_h * 0.5 || abs_top > vp.max.y + page_h * 0.5);

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
                    let byte = approx_byte(&ed.paras[i].text, pp.x - edit_rect.min.x, char_w);
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
                if i == focused {
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(pm.x + ml, text_y - 2.0),
                            egui::vec2(cw, text_h + 4.0),
                        ),
                        2.0,
                        focus_bg,
                    );
                }

                match para.style {
                    ParaStyle::BlockQuote => {
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(cw, text_h)),
                            0.0,
                            bq_bg,
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(3.0, text_h)),
                            1.0,
                            ColorPalette::BLUE_500,
                        );
                    }
                    ParaStyle::Code => {
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(cw, text_h)),
                            3.0,
                            if is_dark {
                                egui::Color32::from_rgb(24, 24, 30)
                            } else {
                                egui::Color32::from_rgb(244, 244, 248)
                            },
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(3.0, text_h)),
                            1.0,
                            code_left,
                        );
                    }
                    ParaStyle::ListBullet => {
                        let by = text_y + text_h / 2.0;
                        painter.circle_filled(
                            egui::pos2(edit_x - 8.0 * ed.zoom, by),
                            2.5 * ed.zoom,
                            bullet_col,
                        );
                    }
                    ParaStyle::ListOrdered => {
                        let num = para.list_num.unwrap_or((i + 1) as u32);
                        painter.text(
                            egui::pos2(edit_x - 4.0, text_y),
                            egui::Align2::RIGHT_TOP,
                            format!("{}.", num),
                            egui::FontId::proportional(bs * para.style.size_scale() * 0.9),
                            bullet_col,
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
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(255, 210, 0, 90),
                        );
                    }
                }

                if let Some((from, to)) = cross_sel {
                    draw_cross_sel_for_para(&painter, from, to, i, edit_rect, &ed.paras[i].text, char_w);
                }
            }

            if i == focused {
                let id = ed.para_ids[i];
                let at_start = egui::TextEdit::load_state(ctx, id)
                    .and_then(|s| s.cursor.char_range())
                    .map(|cr| cr.primary.index == 0 && cr.secondary.index == 0)
                    .unwrap_or(false);

                let should_handle = at_start && (ed.paras[i].indent_left > 0.0 || i > 0);
                if should_handle && ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)) {
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
            }

            let para_clone = ed.paras[i].clone();
            let spans_clone: Vec<DocSpan> = ed.paras[i].spans.clone();
            let base_size_snap = bs;
            let base_font_snap = font;
            let dark_snap = is_dark;
            let zoom_snap = ed.zoom;
            let mut layouter = move |lui: &egui::Ui, s: &dyn egui::TextBuffer, _ww: f32| {
                let job = build_layout_job(
                    &spans_clone,
                    s.as_str(),
                    &para_clone,
                    base_font_snap,
                    base_size_snap,
                    wrap_w,
                    dark_snap,
                    zoom_snap,
                );
                lui.fonts_mut(|f| f.layout_job(job))
            };

            let id = ed.para_ids[i];
            if ed.pending_focus == Some(i) {
                ctx.memory_mut(|m| m.request_focus(id));
            }
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

            let has_focus = output.response.has_focus();
            if has_focus && ed.focused_para != i {
                ed.focused_para = i;
                ed.line_spacing_input = ed.paras[i].line_height;
            }
            if has_focus {
                let mut b = false;
                let mut it = false;
                let mut u = false;
                let mut h: u8 = 0;
                let mut tab_delta: Option<f32> = None;

                ctx.input_mut(|inp| {
                    if inp.consume_key(egui::Modifiers::CTRL, egui::Key::B) { b = true; }
                    if inp.consume_key(egui::Modifiers::CTRL, egui::Key::I) { it = true; }
                    if inp.consume_key(egui::Modifiers::CTRL, egui::Key::U) { u = true; }

                    if inp.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab) {
                        tab_delta = Some(-18.0);
                    } else if inp.consume_key(egui::Modifiers::NONE, egui::Key::Tab) {
                        tab_delta = Some(18.0);
                    }

                    for (key, level) in [
                        (egui::Key::Num1, 1u8),
                        (egui::Key::Num2, 2),
                        (egui::Key::Num3, 3),
                        (egui::Key::Num4, 4),
                        (egui::Key::Num5, 5),
                    ] {
                        if inp.consume_key(egui::Modifiers::CTRL | egui::Modifiers::ALT, key) {
                            h = level;
                        }
                    }
                });

                if b { ed.apply_fmt_toggle_bold(); }
                if it { ed.apply_fmt_toggle_italic(); }
                if u { ed.apply_fmt_toggle_underline(); }
                if h > 0 {
                    let style = match h {
                        1 => ParaStyle::H1,
                        2 => ParaStyle::H2,
                        3 => ParaStyle::H3,
                        4 => ParaStyle::H4,
                        _ => ParaStyle::H5,
                    };
                    ed.apply_style_toggle(style);
                }

                if let Some(delta) = tab_delta {
                    ed.push_undo();
                    let saved_state = egui::TextEdit::load_state(ctx, id);
                    ed.indent_para(i, delta);
                    if let Some(state) = saved_state {
                        egui::TextEdit::store_state(ctx, id, state);
                    }
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
                            c.bold = fmt.bold;
                            c.italic = fmt.italic;
                            c.underline = fmt.underline;
                            c.strike = fmt.strike;
                            c.sub = fmt.sub;
                            c.sup = fmt.sup;
                        }
                    }
                }
            }

            if ed.pending_focus == Some(i) {
                pending_focus_next = Some(i);
            }

            if output.response.changed() {
                text_change = Some((i, ed.para_texts[i].clone()));
            }

            let new_h = output.galley.size().y;
            if (new_h - text_h).abs() > 0.5 {
                ed.heights_dirty = true;
            }
        }
    });

    if let Some(f) = pending_focus_next { ed.focused_para = f.min(ed.paras.len().saturating_sub(1)); ed.pending_focus = None; }
    if let Some(sel) = new_selection { ed.last_selection = Some(sel); }

    if let Some(mu) = merge_up {
        if mu > 0 && mu < ed.paras.len() {
            ed.push_undo();
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
            ed.dirty = true;
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
    crate::style::draw_modal_overlay(ctx, "de_page_ov", 160);
    let (bg, border, tc, muted) = if is_dark { (ColorPalette::ZINC_900, ColorPalette::ZINC_700, ColorPalette::SLATE_200, ColorPalette::ZINC_400) }
        else { (egui::Color32::WHITE, ColorPalette::GRAY_200, ColorPalette::GRAY_800, ColorPalette::GRAY_500) };
    let mut open = ed.show_page_settings;
    let win = egui::Window::new("Page Settings")
        .collapsible(false).resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .fixed_size(egui::vec2(360.0, 0.0))
        .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(24.0))
        .open(&mut open).order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.label(egui::RichText::new("Page Size").size(13.0).color(tc));
            ui.horizontal_wrapped(|ui| {
                for (idx, name) in PageLayout::presets().iter().enumerate() {
                    let sel = ed.preset_idx == idx;
                    let (bg2, fg) = if sel { (ColorPalette::BLUE_600, egui::Color32::WHITE) } else if is_dark { (ColorPalette::ZINC_700, ColorPalette::ZINC_300) } else { (ColorPalette::GRAY_200, ColorPalette::GRAY_800) };
                    if ui.add(egui::Button::new(egui::RichText::new(*name).size(12.0).color(fg)).fill(bg2).corner_radius(4.0).min_size(egui::vec2(100.0, 28.0))).clicked() {
                        ed.preset_idx = idx; ed.layout = PageLayout::from_preset(idx); ed.heights_dirty = true; ed.auto_zoom_done = false;
                    }
                }
            });
            ui.label(egui::RichText::new("Custom (points)").size(11.0).color(muted));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("W:").size(12.0).color(muted)); if ui.add(egui::DragValue::new(&mut ed.layout.width).range(200.0..=1200.0).speed(1.0)).changed() { ed.heights_dirty = true; }
                ui.label(egui::RichText::new("H:").size(12.0).color(muted)); ui.add(egui::DragValue::new(&mut ed.layout.height).range(200.0..=1700.0).speed(1.0));
            });
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Margins (points)").size(13.0).color(tc));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Top:").size(12.0).color(muted)); if ui.add(egui::DragValue::new(&mut ed.layout.margin_top).range(0.0..=200.0).speed(1.0)).changed() { ed.heights_dirty = true; }
                ui.label(egui::RichText::new("Bottom:").size(12.0).color(muted)); if ui.add(egui::DragValue::new(&mut ed.layout.margin_bot).range(0.0..=200.0).speed(1.0)).changed() { ed.heights_dirty = true; }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Left:").size(12.0).color(muted)); if ui.add(egui::DragValue::new(&mut ed.layout.margin_left).range(0.0..=200.0).speed(1.0)).changed() { ed.heights_dirty = true; }
                ui.label(egui::RichText::new("Right:").size(12.0).color(muted)); if ui.add(egui::DragValue::new(&mut ed.layout.margin_right).range(0.0..=200.0).speed(1.0)).changed() { ed.heights_dirty = true; }
            });
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Base Font Size").size(12.0).color(muted));
            if ui.add(egui::Slider::new(&mut ed.base_size, 8.0..=24.0).suffix(" pt")).changed() { ed.heights_dirty = true; }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Apply").clicked() { ed.dirty = true; ed.show_page_settings = false; }
                if ui.button("Cancel").clicked() { ed.show_page_settings = false; }
            });
        });
    if !open { ed.show_page_settings = false; }
    if let Some(win) = win {
        let clicked_outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().map_or(false, |p| !win.response.rect.contains(p)));
        if clicked_outside { ed.show_page_settings = false; }
    }
}
