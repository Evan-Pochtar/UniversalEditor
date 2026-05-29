use eframe::egui;
use crate::style::{ColorPalette, ThemeMode, toolbar_action_btn, toolbar_toggle_btn};
use super::de_main::{DocumentEditor, DocPos};
use crate::modules::EditorModule;
use super::de_tools::*;
use std::cell::RefCell;

thread_local! { static RICH_CLIP: RefCell<Option<(String, Vec<DocSpan>)>> = RefCell::new(None); }
const PAGE_GAP: f32 = 28.0;
const PAGE_PAD: f32 = 24.0;

fn extract_sel_spans(para: &DocParagraph, start: usize, end: usize) -> Vec<DocSpan> {
    let mut out = Vec::new(); let mut pos = 0usize;
    for span in &para.spans {
        let se = pos + span.len;
        if se > start && pos < end { let cs = start.max(pos); let ce = end.min(se); if ce > cs { out.push(DocSpan { len: ce - cs, fmt: span.fmt.clone() }); } }
        pos = se;
    }
    out
}

fn draw_squiggle(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let y = rect.max.y + 1.0;
    let x0 = rect.min.x;
    let x1 = rect.max.x;
    let w = (x1 - x0).max(0.0);
    if w < 2.0 { return; }
    let amp = 1.5_f32;
    let period = 4.0_f32;
    let steps = ((w / period) * 4.0).ceil() as usize + 1;
    let stroke = egui::Stroke::new(1.0, color);
    let mut prev: Option<egui::Pos2> = None;
    for s in 0..=steps {
        let t = x0 + (s as f32 / steps as f32) * w;
        let phase = (t - x0) / period * std::f32::consts::TAU;
        let ys = y + phase.sin() * amp;
        let cur = egui::pos2(t, ys);
        if let Some(p) = prev { painter.line_segment([p, cur], stroke); }
        prev = Some(cur);
    }
}

fn compute_drop_idx(ed: &DocumentEditor, pl: &ComputedPageLayout, omy: f32, cy: f32) -> usize {
    let n = ed.paras.len();
    let gy = |i: usize| -> f32 {
        if i < n { omy + pl.page_tops.get(pl.para_page[i]).copied().unwrap_or(0.0) + pl.para_content_y[i] * ed.zoom }
        else { let j = n.saturating_sub(1); omy + pl.page_tops.get(pl.para_page[j]).copied().unwrap_or(0.0) + (pl.para_content_y[j] + ed.para_heights.get(j).copied().unwrap_or(0.0)) * ed.zoom }
    };
    (0..=n).min_by(|&a, &b| (cy - gy(a)).abs().partial_cmp(&(cy - gy(b)).abs()).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(0)
}

fn drop_line_y(ed: &DocumentEditor, pl: &ComputedPageLayout, omy: f32, idx: usize) -> f32 {
    let n = ed.paras.len();
    if idx < n { omy + pl.page_tops.get(pl.para_page[idx]).copied().unwrap_or(0.0) + pl.para_content_y[idx] * ed.zoom }
    else { let j = n.saturating_sub(1); omy + pl.page_tops.get(pl.para_page[j]).copied().unwrap_or(0.0) + (pl.para_content_y[j] + ed.para_heights.get(j).copied().unwrap_or(0.0)) * ed.zoom }
}

enum CtxAction {
    Copy, Cut, Paste, Delete,
    CopyWithFmt, PasteWithFmt,
    Bold, Italic, Underline, Strike, ClearFmt,
    TextColor(Option<[u8; 3]>), Highlight(Option<[u8; 3]>), SetLink,
    ImgCopy(usize), ImgCut(usize), ImgDelete(usize),
    ImgOpen(usize), ImgReplace(usize),
}

fn multiline_highlight(galley: &egui::text::Galley, text: &str, start_byte: usize, end_byte: usize) -> Vec<egui::Rect> {
    let start_byte = start_byte.min(text.len()); let end_byte = end_byte.min(text.len());
    let start_char = text[..start_byte].chars().count(); let end_char = text[..end_byte].chars().count();
    let start_adjust = 4.0; let end_adjust = 4.0;
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
                row.rect().min.x + start_adjust
            } else {
                row.glyphs.get(local_start).map(|g| g.pos.x + start_adjust).unwrap_or(row.rect().max.x)
            };
            let x1 = if local_end >= glyph_count {
                row.rect().max.x + end_adjust
            } else {
                row.glyphs.get(local_end).map(|g| g.pos.x + end_adjust).unwrap_or(row.rect().max.x)
            };
            if x1 >= x0 {
                rects.push(egui::Rect::from_min_max(
                    egui::pos2(x0, row.rect().min.y),
                    egui::pos2(x1.max(x0 + 4.0), row.rect().max.y),
                ));
            }
        }
        char_pos = row_end;
        if row.ends_with_newline { char_pos += 1; }
    }

    if rects.is_empty() && start_byte == 0 && end_byte >= text.len() {
        if let Some(row) = galley.rows.first() {
            rects.push(egui::Rect::from_min_max(
                egui::pos2(row.rect().min.x + start_adjust, row.rect().min.y),
                egui::pos2(row.rect().min.x + 8.0 + end_adjust, row.rect().max.y),
            ));
        }
    }
    rects
}

fn text_color_palette(ui: &mut egui::Ui, ed: &mut DocumentEditor, is_dark: bool, popup_id: egui::Id) {
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
    if ui.add(egui::Button::new(egui::RichText::new("Auto (default)").size(11.0)).min_size(egui::vec2(120.0, 20.0))).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        ed.apply_fmt_color(None);
        egui::Popup::close_id(ui.ctx(), popup_id);
    }
    ui.add_space(4.0);
    for row in PALETTE.chunks(6) {
        ui.horizontal(|ui| {
            for &(c, name) in row {
                let col = egui::Color32::from_rgb(c[0], c[1], c[2]);
                let border = if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 };
                if ui.add(egui::Button::new("").fill(col).stroke(egui::Stroke::new(1.0, border)).min_size(egui::vec2(20.0, 20.0)).corner_radius(3.0))
                    .on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                { ed.apply_fmt_color(Some(c)); egui::Popup::close_id(ui.ctx(), popup_id); }
            }
        });
    }
}

fn highlight_color_palette(ui: &mut egui::Ui, ed: &mut DocumentEditor, is_dark: bool, popup_id: egui::Id) {
    const PALETTE: &[([u8; 3], &str)] = &[
        ([255, 235, 59], "Yellow"), ([255, 204, 128], "Peach"), ([255, 171, 145], "Salmon"),
        ([199, 210, 254], "Lavender"), ([167, 243, 208], "Mint"), ([147, 197, 253], "Sky"),
        ([253, 224, 71], "Gold"), ([250, 204, 21], "Amber"), ([253, 186, 116], "Apricot"),
        ([190, 242, 100], "Lime"), ([125, 211, 252], "Cyan"), ([196, 181, 253], "Violet"),
    ];
    let lc = if is_dark { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
    ui.label(egui::RichText::new("Highlight").size(11.0).color(lc));
    ui.add_space(4.0);
    if ui.add(egui::Button::new(egui::RichText::new("No Highlight").size(11.0)).min_size(egui::vec2(120.0, 20.0))).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        ed.apply_fmt_highlight(None);
        egui::Popup::close_id(ui.ctx(), popup_id);
    }
    ui.add_space(4.0);
    for row in PALETTE.chunks(4) {
        ui.horizontal(|ui| {
            for &(c, name) in row {
                let col = egui::Color32::from_rgb(c[0], c[1], c[2]);
                let border = if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 };
                if ui.add(egui::Button::new("").fill(col).stroke(egui::Stroke::new(1.0, border)).min_size(egui::vec2(26.0, 22.0)).corner_radius(3.0))
                    .on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                { ed.apply_fmt_highlight(Some(c)); egui::Popup::close_id(ui.ctx(), popup_id); }
            }
        });
    }
}

fn link_at_byte<'a>(para: &'a DocParagraph, byte: usize) -> Option<&'a str> {
    let mut pos = 0usize;
    for span in &para.spans {
        let end = pos + span.len;
        if byte >= pos && byte < end { return span.fmt.link.as_deref(); }
        pos = end;
    }
    None
}

fn table_col_widths(tbl: &TableData, cw: f32) -> Vec<f32> {
    let nc = tbl.rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
    if tbl.col_widths.len() == nc { tbl.col_widths.iter().map(|&f| f * cw).collect() }
    else { vec![cw / nc as f32; nc] }
}

fn table_row_h(row: &[TableCell], col_ws: &[f32], zoom: f32, ctx: &egui::Context, live_cell: Option<(usize, &str)>) -> f32 {
    let min_h = DEFAULT_BASE_SIZE as f32 * zoom * 1.6;
    row.iter().enumerate().fold(min_h, |acc, (ci, cell)| {
        let text = if let Some((lci, lt)) = live_cell { if lci == ci { lt } else { cell.text.as_str() } } else { cell.text.as_str() };
        if text.is_empty() { return acc; }
        let w = col_ws.get(ci).copied().unwrap_or(min_h) - 16.0 * zoom;
        let job = egui::text::LayoutJob::simple(text.to_owned(), egui::FontId::new(DEFAULT_BASE_SIZE as f32 * zoom * 0.9, DEFAULT_BASE_FONT.egui_family(false, false)), egui::Color32::WHITE, w.max(1.0));
        let mut gh = ctx.fonts_mut(|f| f.layout_job(job)).rect.height();
        if text.ends_with('\n') { gh += DEFAULT_BASE_SIZE as f32 * zoom * 0.9 * 1.2; }
        acc.max(gh + 10.0 * zoom)
    })
}

fn cell_in_sel(sel: Option<(usize, (usize, usize), (usize, usize))>, pi: usize, ri: usize, ci: usize) -> bool {
    let Some((sp, (ar, ac), (br, bc))) = sel else { return false };
    if sp != pi { return false }
    let (r0, r1) = (ar.min(br), ar.max(br));
    let (c0, c1) = (ac.min(bc), ac.max(bc));
    ri >= r0 && ri <= r1 && ci >= c0 && ci <= c1
}

fn cell_in_multi_sel(ms: Option<&(usize, Vec<(usize, usize)>)>, pi: usize, ri: usize, ci: usize) -> bool {
    ms.map_or(false, |(sp, cells)| *sp == pi && cells.contains(&(ri, ci)))
}

fn get_doc_image_texture(ctx: &egui::Context, ed: &mut DocumentEditor, img: &DocImage) -> egui::TextureId {
    if let Some(&tid) = ed.image_textures.get(&img.uid) { return tid; }
    let dyn_img = image::load_from_memory(&img.data).unwrap_or_else(|_| image::DynamicImage::new_rgba8(1, 1));
    let rgba = dyn_img.to_rgba8();
    let (w, h) = (rgba.width().max(1) as usize, rgba.height().max(1) as usize);
    let ci = egui::ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());
    let tid = ctx.tex_manager().write().alloc(format!("doc_img_{}", img.uid).into(), ci.into(), egui::TextureOptions::LINEAR);
    ed.image_textures.insert(img.uid, tid);
    tid
}

fn image_handle_positions(rect: egui::Rect) -> [egui::Pos2; 8] {
    let (cx, cy) = (rect.center().x, rect.center().y);
    [rect.left_top(), egui::pos2(cx, rect.top()), rect.right_top(), egui::pos2(rect.right(), cy),
     rect.right_bottom(), egui::pos2(cx, rect.bottom()), rect.left_bottom(), egui::pos2(rect.left(), cy)]
}

fn image_handle_cursor(handle: u8) -> egui::CursorIcon {
    match handle {
        0 | 4 => egui::CursorIcon::ResizeNwSe,
        2 | 6 => egui::CursorIcon::ResizeNeSw,
        1 | 5 => egui::CursorIcon::ResizeVertical,
        _ => egui::CursorIcon::ResizeHorizontal,
    }
}

fn image_drag_dims(ed: &DocumentEditor, ctx: &egui::Context, drag: (usize, u8, egui::Pos2, f32, f32, f32)) -> (f32, f32) {
    let (_, handle, drag_start, orig_w, orig_h, orig_asp) = drag;
    if handle == 255 { return (orig_w, orig_h); }
    let cur = ctx.input(|inp| inp.pointer.latest_pos()).unwrap_or(drag_start);
    let delta = (cur - drag_start) / ed.zoom;
    let shift = ctx.input(|inp| inp.modifiers.shift);
    let (mut nw, mut nh) = match handle {
        0 => (orig_w - delta.x, orig_h - delta.y), 1 => (orig_w, orig_h - delta.y),
        2 => (orig_w + delta.x, orig_h - delta.y), 3 => (orig_w + delta.x, orig_h),
        4 => (orig_w + delta.x, orig_h + delta.y), 5 => (orig_w, orig_h + delta.y),
        6 => (orig_w - delta.x, orig_h + delta.y), _ => (orig_w - delta.x, orig_h),
    };
    nw = nw.max(20.0); nh = nh.max(20.0);
    if shift { nh = nw / orig_asp.max(0.001); }
    (nw, nh)
}

fn get_sel_text(ed: &DocumentEditor) -> Option<String> {
    if let Some((from, to)) = ed.norm_sel() {
        if from.para != to.para || from.byte != to.byte {
            return Some(ed.collect_sel_text(from, to));
        }
    }
    if let Some((pi, sb, eb)) = ed.last_selection {
        if sb != eb && pi < ed.paras.len() {
            let (lo, hi) = (sb.min(eb), sb.max(eb));
            return Some(ed.paras[pi].text[lo..hi].to_string());
        }
    }
    None
}

fn do_delete_sel(ed: &mut DocumentEditor, ctx: &egui::Context) {
    if ed.has_cross_sel() {
        ed.delete_sel();
    } else if let Some((pi, sb, eb)) = ed.last_selection {
        if sb != eb && pi < ed.paras.len() {
            let (lo, hi) = (sb.min(eb), sb.max(eb));
            ed.push_undo();
            let new_text = format!("{}{}", &ed.paras[pi].text[..lo], &ed.paras[pi].text[hi..]);
            rebuild_spans(&mut ed.paras[pi], new_text, &ed.cur_fmt);
            ed.para_texts[pi] = ed.paras[pi].text.clone();
            let ci = ed.paras[pi].text[..lo].chars().count();
            let id = ed.para_ids[pi];
            let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(ci))));
            egui::TextEdit::store_state(ctx, id, state);
            ed.last_selection = Some((pi, lo, lo));
            ed.dirty = true; ed.heights_dirty = true; ed.find_stale = true;
        }
    }
}

fn process_ctx_action(ed: &mut DocumentEditor, ctx: &egui::Context, action: CtxAction) {
    if !ed.has_cross_sel() {
        if let Some(sel) = ed.ctx_sel.filter(|(_, s, e)| s != e) { ed.last_selection = Some(sel); }
    }
    match action {
        CtxAction::Copy => { if let Some(t) = get_sel_text(ed) { ctx.copy_text(t); } }
        CtxAction::Cut => {
            if let Some(t) = get_sel_text(ed) { ctx.copy_text(t); }
            do_delete_sel(ed, ctx);
        }
        CtxAction::Paste => {
            if let Ok(clip) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                let had_sel = ed.has_cross_sel() || ed.last_selection.map(|(_, s, e)| s != e).unwrap_or(false);
                if had_sel { do_delete_sel(ed, ctx); } else { ed.push_undo(); }
                let pi = ed.focused_para.min(ed.paras.len().saturating_sub(1));
                if pi < ed.paras.len() {
                    let cur = ed.last_selection.filter(|&(lpi, sb, eb)| lpi == pi && sb == eb)
                        .map(|(_, s, _)| s.min(ed.paras[pi].text.len())).unwrap_or(0);
                    let suf = ed.paras[pi].text[cur..].to_string();
                    let pfx = ed.paras[pi].text[..cur].to_string();
                    let lines: Vec<&str> = clip.split('\n').collect();
                    let ns = if ed.paras[pi].style.is_heading() { ParaStyle::Normal } else { ed.paras[pi].style };
                    let (al, lh, il) = (ed.paras[pi].align, ed.paras[pi].line_height, ed.paras[pi].indent_left);
                    if lines.len() == 1 {
                        rebuild_spans(&mut ed.paras[pi], format!("{}{}{}", pfx, lines[0], suf), &ed.cur_fmt);
                        ed.para_texts[pi] = ed.paras[pi].text.clone();
                        let nc = pfx.len() + lines[0].len();
                        let ci = ed.paras[pi].text[..nc].chars().count();
                        let id = ed.para_ids[pi];
                        let mut st = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                        st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(ci))));
                        egui::TextEdit::store_state(ctx, id, st);
                        ed.last_selection = Some((pi, nc, nc)); ed.focused_para = pi; ed.pending_focus = Some(pi);
                    } else {
                        rebuild_spans(&mut ed.paras[pi], format!("{}{}", pfx, lines[0]), &ed.cur_fmt);
                        ed.para_texts[pi] = ed.paras[pi].text.clone();
                        let mut ins = pi + 1;
                        for &ln in &lines[1..lines.len()-1] {
                            let mut np = DocParagraph::with_style(ns);
                            np.text = ln.to_string(); np.spans = vec![DocSpan { len: ln.len(), fmt: ed.cur_fmt.clone() }];
                            np.align = al; np.line_height = lh; np.indent_left = il;
                            ed.paras.insert(ins, np); ins += 1;
                        }
                        let ll = lines.last().unwrap_or(&"");
                        let last_t = format!("{}{}", ll, suf);
                        let mut lp = DocParagraph::with_style(ns);
                        lp.text = last_t.clone(); lp.spans = vec![DocSpan { len: last_t.len(), fmt: ed.cur_fmt.clone() }];
                        lp.align = al; lp.line_height = lh; lp.indent_left = il;
                        ed.paras.insert(ins, lp);
                        ed.focused_para = ins; ed.pending_focus = Some(ins);
                        ed.sync_texts();
                        let nc = ll.len().min(last_t.len());
                        let id = ed.para_ids[ins];
                        let ci = last_t[..nc].chars().count();
                        let mut st = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                        st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(ci))));
                        egui::TextEdit::store_state(ctx, id, st);
                        ed.last_selection = Some((ins, nc, nc));
                    }
                    ed.dirty = true; ed.heights_dirty = true; ed.find_stale = true;
                }
            } else if let Ok(img_data) = arboard::Clipboard::new().and_then(|mut c| c.get_image()) {
                let (w, h) = (img_data.width as u32, img_data.height as u32);
                let scale = (ed.layout.content_width() / w.max(1) as f32).min(1.0);
                if let Some(rgba) = image::RgbaImage::from_raw(w, h, img_data.bytes.into_owned()) {
                    let mut buf = Vec::new();
                    image::DynamicImage::ImageRgba8(rgba).write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png).ok();
                    ed.insert_image(buf, w as f32 * scale, h as f32 * scale, "pasted.png".to_string());
                }
            }
        }
        CtxAction::Delete => do_delete_sel(ed, ctx),
        CtxAction::CopyWithFmt => {
            if let Some((from, to)) = ed.norm_sel() {
                if from.para != to.para || from.byte != to.byte {
                    let text = ed.collect_sel_text(from, to);
                    ctx.copy_text(text.clone());
                    let mut combined_spans: Vec<DocSpan> = Vec::new();
                    for pi in from.para..=to.para {
                        if pi >= ed.paras.len() { break; }
                        let start = if pi == from.para { from.byte } else { 0 };
                        let end = if pi == to.para { to.byte } else { ed.paras[pi].text.len() };
                        if pi > from.para { combined_spans.push(DocSpan { len: 1, fmt: SpanFmt::default() }); }
                        combined_spans.extend(extract_sel_spans(&ed.paras[pi], start, end));
                    }
                    RICH_CLIP.with(|c| *c.borrow_mut() = Some((text, combined_spans)));
                    return;
                }
            }
            let sel = ed.ctx_sel.filter(|(_, s, e)| s != e).or(ed.last_selection.filter(|(_, s, e)| s != e));
            if let Some((pi, sb, eb)) = sel {
                if pi < ed.paras.len() {
                    let (lo, hi) = (sb.min(eb), sb.max(eb).min(ed.paras[pi].text.len()));
                    let text = ed.paras[pi].text[lo..hi].to_string();
                    ctx.copy_text(text.clone());
                    let spans = extract_sel_spans(&ed.paras[pi], lo, hi);
                    RICH_CLIP.with(|c| *c.borrow_mut() = Some((text, spans)));
                }
            }
        }
        CtxAction::PasteWithFmt => {
            let rich = RICH_CLIP.with(|c| c.borrow().as_ref().cloned());
            if let Some((text, rich_spans)) = rich {
                let had_sel = ed.has_cross_sel() || ed.last_selection.map(|(_, s, e)| s != e).unwrap_or(false);
                if had_sel { do_delete_sel(ed, ctx); } else { ed.push_undo(); }
                let pi = ed.focused_para.min(ed.paras.len().saturating_sub(1));
                if pi < ed.paras.len() {
                    let cur = ed.last_selection.filter(|(lpi, sb, eb)| *lpi == pi && sb == eb)
                        .map(|(_, s, _)| s.min(ed.paras[pi].text.len())).unwrap_or(0);
                    ensure_boundary(&mut ed.paras[pi], cur);
                    let ins_len = text.len();
                    ed.paras[pi].text.insert_str(cur, &text);
                    let mut acc = 0usize;
                    let split = ed.paras[pi].spans.iter().position(|s| { if acc >= cur { true } else { acc += s.len; false } }).unwrap_or(ed.paras[pi].spans.len());
                    let tail = ed.paras[pi].spans.split_off(split);
                    for s in rich_spans { ed.paras[pi].spans.push(DocSpan { len: s.len, fmt: s.fmt }); }
                    ed.paras[pi].spans.extend(tail);
                    merge_adjacent(&mut ed.paras[pi]);
                    ed.para_texts[pi] = ed.paras[pi].text.clone();
                    let nc = ed.paras[pi].text[..cur + ins_len].chars().count();
                    let id = ed.para_ids[pi];
                    let mut st = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                    st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(nc))));
                    egui::TextEdit::store_state(ctx, id, st);
                    ed.last_selection = Some((pi, cur + ins_len, cur + ins_len));
                    ed.dirty = true; ed.heights_dirty = true; ed.find_stale = true;
                }
            }
        }
        CtxAction::Bold => ed.apply_fmt_toggle_bold(),
        CtxAction::Italic => ed.apply_fmt_toggle_italic(),
        CtxAction::Underline => ed.apply_fmt_toggle_underline(),
        CtxAction::Strike => ed.apply_fmt_toggle_strike(),
        CtxAction::ClearFmt => { ed.apply_fmt_property(|f| { f.bold=false; f.italic=false; f.underline=false; f.strike=false; f.sup=false; f.sub=false; f.color=None; f.highlight=None; f.link=None; }); }
        CtxAction::TextColor(c) => ed.apply_fmt_color(c),
        CtxAction::Highlight(c) => ed.apply_fmt_highlight(c),
        CtxAction::SetLink => {
            let cur_link = ed.last_selection.filter(|(_, s, e)| s != e)
                .and_then(|(pi, sb, _)| ed.paras.get(pi).map(|p| link_at_byte(p, sb.min(p.text.len()))))
                .flatten().map(|s| s.to_string()).unwrap_or_default();
            ed.link_input = cur_link;
            ed.ctx_link_show = true;
            return;
        }
        CtxAction::ImgCopy(pi) => {
            if let Some(data) = ed.paras.get(pi).and_then(|p| p.image.as_ref()).map(|img| img.data.clone()) {
                if let Ok(img) = image::load_from_memory(&data) {
                    let (w, h) = (img.width() as usize, img.height() as usize);
                    let bytes = img.into_rgba8().into_raw();
                    let _ = arboard::Clipboard::new().map(|mut c| c.set_image(arboard::ImageData { width: w, height: h, bytes: bytes.into() }));
                }
            }
        }
        CtxAction::ImgCut(pi) => {
            if let Some(data) = ed.paras.get(pi).and_then(|p| p.image.as_ref()).map(|img| img.data.clone()) {
                if let Ok(img) = image::load_from_memory(&data) {
                    let (w, h) = (img.width() as usize, img.height() as usize);
                    let bytes = img.into_rgba8().into_raw();
                    let _ = arboard::Clipboard::new().map(|mut c| c.set_image(arboard::ImageData { width: w, height: h, bytes: bytes.into() }));
                }
            }
            if pi < ed.paras.len() {
                ed.push_undo();
                if let Some(ref img) = ed.paras[pi].image { ed.image_textures.remove(&img.uid); }
                ed.paras.remove(pi); ed.selected_image_para = None;
                ed.focused_para = pi.min(ed.paras.len().saturating_sub(1));
                ed.sync_texts(); ed.dirty = true; ed.heights_dirty = true;
            }
        }
        CtxAction::ImgDelete(pi) => {
            if pi < ed.paras.len() {
                ed.push_undo();
                if let Some(ref img) = ed.paras[pi].image { ed.image_textures.remove(&img.uid); }
                ed.paras.remove(pi); ed.selected_image_para = None;
                ed.focused_para = pi.min(ed.paras.len().saturating_sub(1));
                ed.sync_texts(); ed.dirty = true; ed.heights_dirty = true;
            }
        }
        CtxAction::ImgOpen(pi) => {
            if let Some(data) = ed.paras.get(pi).and_then(|p| p.image.as_ref()).map(|img| img.data.clone()) {
                ed.pending_open_in_image_editor = Some(data);
            }
        }
        CtxAction::ImgReplace(pi) => {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", &["jpg","jpeg","png","webp","bmp","tiff","ico"])
                .pick_file()
            {
                if let Ok(img) = image::open(&path) {
                    let (iw, ih) = (img.width() as f32, img.height() as f32);
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
                    let fmt = match ext.to_lowercase().as_str() { "jpg"|"jpeg" => image::ImageFormat::Jpeg, "webp" => image::ImageFormat::WebP, "bmp" => image::ImageFormat::Bmp, _ => image::ImageFormat::Png };
                    let mut buf = Vec::new();
                    img.write_to(&mut std::io::Cursor::new(&mut buf), fmt).ok();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("image").to_string();
                    if let Some(p) = ed.paras.get_mut(pi) {
                        if let Some(ref mut di) = p.image {
                            ed.image_textures.remove(&di.uid);
                            di.data = buf; di.display_w = iw; di.display_h = ih; di.name = name;
                        }
                    }
                    ed.heights_dirty = true; ed.dirty = true;
                }
            }
        }
    }
    ed.ctx_sel = None;
}

fn cm_btn(ui: &mut egui::Ui, label: &str, enabled: bool) -> bool {
    ui.add_enabled(enabled, egui::Button::new(egui::RichText::new(label).size(12.5)).min_size(egui::vec2(50.0, 26.0))).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn cm_sep(ui: &mut egui::Ui) {
    let c = if ui.visuals().dark_mode { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_200 };
    ui.add_space(1.0);
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, c);
    ui.add_space(1.0);
}

fn ctx_color_palette(ui: &mut egui::Ui, is_dark: bool, on_pick: &mut dyn FnMut(Option<[u8; 3]>)) {
    if ui.add(egui::Button::new(egui::RichText::new("Auto (default)").size(12.0)).min_size(egui::vec2(140.0, 22.0))).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { on_pick(None); }
    const P: &[([u8; 3], &str)] = &[([0,0,0],"Black"),([68,68,68],"Dark Gray"),([102,102,102],"Gray"),([153,153,153],"Light Gray"),([204,204,204],"Silver"),([255,255,255],"White"),([220,38,38],"Red"),([234,88,12],"Orange"),([234,179,8],"Yellow"),([22,163,74],"Green"),([20,184,166],"Teal"),([59,130,246],"Blue"),([99,102,241],"Indigo"),([168,85,247],"Purple"),([236,72,153],"Pink"),([120,53,15],"Brown")];
    let bdr = if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 };
    for row in P.chunks(6) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            for &(c, n) in row {
                if ui.add(egui::Button::new("").fill(egui::Color32::from_rgb(c[0],c[1],c[2])).stroke(egui::Stroke::new(1.0,bdr)).min_size(egui::vec2(20.0,20.0)).corner_radius(3.0)).on_hover_text(n).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { on_pick(Some(c)); }
            }
        });
    }
}

fn ctx_highlight_palette(ui: &mut egui::Ui, is_dark: bool, on_pick: &mut dyn FnMut(Option<[u8; 3]>)) {
    if ui.add(egui::Button::new(egui::RichText::new("No Highlight").size(12.0)).min_size(egui::vec2(140.0, 22.0))).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { on_pick(None); }
    const P: &[([u8; 3], &str)] = &[([255,235,59],"Yellow"),([255,204,128],"Peach"),([255,171,145],"Salmon"),([199,210,254],"Lavender"),([167,243,208],"Mint"),([147,197,253],"Sky"),([253,224,71],"Gold"),([250,204,21],"Amber"),([253,186,116],"Apricot"),([190,242,100],"Lime"),([125,211,252],"Cyan"),([196,181,253],"Violet")];
    let bdr = if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 };
    for row in P.chunks(4) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            for &(c, n) in row {
                if ui.add(egui::Button::new("").fill(egui::Color32::from_rgb(c[0],c[1],c[2])).stroke(egui::Stroke::new(1.0,bdr)).min_size(egui::vec2(26.0,22.0)).corner_radius(3.0)).on_hover_text(n).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { on_pick(Some(c)); }
            }
        });
    }
}

fn text_cm(ui: &mut egui::Ui, has_sel: bool, _sel_has_link: bool, is_dark: bool, action: &std::cell::RefCell<Option<CtxAction>>) {
    ui.set_max_width(160.0);
    ui.spacing_mut().item_spacing.y = 2.0;
    let set = |a: CtxAction| { *action.borrow_mut() = Some(a); };
    if cm_btn(ui, "Copy", has_sel) { set(CtxAction::Copy); }
    if cm_btn(ui, "Paste", true) { set(CtxAction::Paste); }
    if cm_btn(ui, "Cut", has_sel) { set(CtxAction::Cut); }
    if cm_btn(ui, "Delete", has_sel) { set(CtxAction::Delete); }
    cm_sep(ui);
    if cm_btn(ui, "Bold", true) { set(CtxAction::Bold); }
    if cm_btn(ui, "Italic", true) { set(CtxAction::Italic); }
    if cm_btn(ui, "Underline", true) { set(CtxAction::Underline); }
    if cm_btn(ui, "Strikethrough", true) { set(CtxAction::Strike); }
    if cm_btn(ui, "Clear Formatting", has_sel) { set(CtxAction::ClearFmt); }
    cm_sep(ui);
    ui.scope(|ui| {
        ui.style_mut().spacing.interact_size.y = 26.0;
        ui.menu_button(egui::RichText::new("Text Color").size(12.5), |ui| {
            ctx_color_palette(ui, is_dark, &mut |c| { *action.borrow_mut() = Some(CtxAction::TextColor(c)); });
        });
        ui.menu_button(egui::RichText::new("Highlight").size(12.5), |ui| {
            ctx_highlight_palette(ui, is_dark, &mut |c| { *action.borrow_mut() = Some(CtxAction::Highlight(c)); });
        });
    });
    if cm_btn(ui, "Link", true) { set(CtxAction::SetLink); }
    cm_sep(ui);
    if cm_btn(ui, "Copy with Formatting", has_sel) { set(CtxAction::CopyWithFmt); }
    let has_rich = RICH_CLIP.with(|c| c.borrow().is_some());
    if cm_btn(ui, "Paste with Formatting", has_rich) { set(CtxAction::PasteWithFmt); }
}

fn img_cm(ui: &mut egui::Ui, para_idx: usize, action: &std::cell::RefCell<Option<CtxAction>>) {
    ui.set_max_width(120.0);
    ui.spacing_mut().item_spacing.y = 2.0;
    let set = |a: CtxAction| { *action.borrow_mut() = Some(a); };
    if cm_btn(ui, "Copy", true) { set(CtxAction::ImgCopy(para_idx)); }
    if cm_btn(ui, "Cut", true) { set(CtxAction::ImgCut(para_idx)); }
    cm_btn(ui, "Paste", false);
    if cm_btn(ui, "Delete", true) { set(CtxAction::ImgDelete(para_idx)); }
    cm_sep(ui);
    if cm_btn(ui, "Edit in Image Editor", true) { set(CtxAction::ImgOpen(para_idx)); }
    if cm_btn(ui, "Replace", true) { set(CtxAction::ImgReplace(para_idx)); }
    cm_btn(ui, "Crop Image", false);
}

fn run_spell_check(ed: &mut DocumentEditor) {
    if !ed.spell_dirty { return; }
    let n = ed.paras.len();
    ed.spell_errors.resize(n, Vec::new());
    for i in 0..n {
        let p = &ed.paras[i];
        if matches!(p.style, ParaStyle::Table | ParaStyle::Image | ParaStyle::HRule) {
            ed.spell_errors[i] = Vec::new();
        } else {
            ed.spell_errors[i] = crate::spell::check_para(&p.text);
        }
    }
    ed.spell_dirty = false;
    ed.spell_version = ed.spell_version.wrapping_add(1);
}

pub fn render(ed: &mut DocumentEditor, ui: &mut egui::Ui, ctx: &egui::Context) {
    let is_dark = ui.visuals().dark_mode;
    let theme = if is_dark { ThemeMode::Dark } else { ThemeMode::Light };
    handle_keyboard(ed, ctx);
    ed.run_find();
    run_spell_check(ed);
    render_toolbar(ed, ui, theme, is_dark);
    ui.separator();
    egui::SidePanel::left("de_outline_panel").resizable(true).default_width(200.0).min_width(140.0).max_width(320.0)
        .frame(egui::Frame::new().fill(if is_dark { egui::Color32::from_rgb(20,20,26) } else { ColorPalette::GRAY_50 })
            .stroke(egui::Stroke::new(1.0, if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 })))
        .show_animated_inside(ui, ed.show_outline, |ui| render_outline(ed, ui, is_dark));
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(if is_dark { egui::Color32::from_rgb(14,14,18) } else { egui::Color32::from_rgb(188,188,196) }))
        .show_inside(ui, |ui| render_canvas(ed, ui, ctx, is_dark));
    render_find_bar(ed, ctx, is_dark);
    render_stats_modal(ed, ctx, is_dark);
    render_page_settings(ed, ctx, is_dark);
    render_ctx_link_modal(ed, ctx, is_dark);
}

fn handle_keyboard(ed: &mut DocumentEditor, ctx: &egui::Context) {
    if ed.has_cross_sel() {
        let para_has_focus = ctx.memory(|m| ed.para_ids.iter().any(|&id| m.has_focus(id)));
        let del = ctx.input_mut(|i| {
            let d = para_has_focus && (i.key_pressed(egui::Key::Backspace) || i.key_pressed(egui::Key::Delete));
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
        if para_has_focus {
            let has_text = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Text(_))));
            if has_text { ed.delete_sel(); }
        }
    }

    if ed.selected_image_para.is_some() {
        let del = ctx.input_mut(|i| {
            let d = i.key_pressed(egui::Key::Backspace) || i.key_pressed(egui::Key::Delete);
            if d { i.events.retain(|e| !matches!(e, egui::Event::Key { key: egui::Key::Backspace | egui::Key::Delete, pressed: true, .. })); }
            d
        });
        if del {
            let pi = ed.selected_image_para.take().unwrap();
            if pi < ed.paras.len() {
                ed.push_undo();
                if let Some(ref img) = ed.paras[pi].image { ed.image_textures.remove(&img.uid); }
                ed.paras.remove(pi);
                ed.focused_para = pi.min(ed.paras.len().saturating_sub(1));
                ed.sync_texts(); ed.dirty = true; ed.heights_dirty = true;
            }
            return;
        }
    }

    ctx.input_mut(|i| {
        if !ed.has_cross_sel() && i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))) { ed.push_undo(); }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Z) { ed.undo(); }
        if i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z) || i.consume_key(egui::Modifiers::CTRL, egui::Key::Y) { ed.redo(); }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::S) { let _ = ed.save(); }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::F) { ed.show_find = true; ed.focus_find = true; }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Plus) || i.consume_key(egui::Modifiers::CTRL, egui::Key::Equals) { ed.zoom = (ed.zoom + 0.1).min(3.0); }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Minus) { ed.zoom = (ed.zoom - 0.1).max(0.3); }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num0) { ed.auto_zoom_done = false; }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::A) {
            if let Some((pi, _, _)) = ed.active_table {
                let new_sel = ed.paras.get(pi).and_then(|p| p.table.as_ref()).map(|tbl| {
                    let lr = tbl.rows.len().saturating_sub(1);
                    let lc = tbl.rows.iter().map(|r| r.len()).max().unwrap_or(1).saturating_sub(1);
                    (pi, (0usize, 0usize), (lr, lc))
                });
                if let Some(s) = new_sel { ed.table_sel = Some(s); ed.table_multi_sel = None; }
            } else {
                let last = ed.paras.len().saturating_sub(1);
                let end = ed.paras.last().map(|p| p.text.len()).unwrap_or(0);
                ed.doc_sel = Some([DocPos { para: 0, byte: 0 }, DocPos { para: last, byte: end }]);
                ed.table_multi_sel = None;
            }
        }
    });
}

fn fmt_btn(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>, active: bool, theme: ThemeMode, tip: &str) -> bool {
    toolbar_toggle_btn(ui, label, active, theme).on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}
fn act_btn(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>, theme: ThemeMode, tip: &str) -> bool {
    toolbar_action_btn(ui, label, theme).on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn render_toolbar(ed: &mut DocumentEditor, ui: &mut egui::Ui, theme: ThemeMode, is_dark: bool) {
    let lc = if is_dark { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
    egui::Frame::new().fill(if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_50 })
        .stroke(egui::Stroke::new(1.0, if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 }))
        .corner_radius(6.0).inner_margin(egui::Margin { left: 6, right: 6, top: 3, bottom: 3 })
        .show(ui, |ui| {
            egui::ScrollArea::horizontal().auto_shrink([false, true]).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.interact_size.y = 26.0;
                    let cur_style = ed.paras.get(ed.focused_para).map(|p| p.style).unwrap_or_default();
                    egui::ComboBox::from_id_salt("de_style_cb").selected_text(egui::RichText::new(cur_style.label()).size(12.0)).width(130.0)
                        .show_ui(ui, |ui| { for s in ParaStyle::all() { if ui.selectable_label(cur_style == *s, s.label()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { ed.apply_style(*s); } } });

                    let cur_text_font = ed.cur_fmt.font.unwrap_or(DEFAULT_BASE_FONT);
                    egui::ComboBox::from_id_salt("de_font_cb").selected_text(egui::RichText::new(cur_text_font.label()).size(12.0)).width(112.0)
                        .show_ui(ui, |ui| { for f in FontChoice::all() { if ui.selectable_label(cur_text_font == *f, f.label()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { ed.apply_fmt_font(Some(*f)); } } });

                    ui.label(egui::RichText::new("Font Size:").size(11.0).color(lc));
                    let mut sel_sz = ed.sel_font_size_pt();
                    let fs_resp = ui.add(egui::DragValue::new(&mut sel_sz).range(4..=288).speed(0.5).suffix("pt"));
                    if fs_resp.changed() { ed.apply_fmt_size(sel_sz); }
                    ui.separator();
                    if fmt_btn(ui, egui::RichText::new("B").strong().size(13.0), ed.fmt_state_bold(), theme, "Bold (Ctrl+B)") { ed.apply_fmt_toggle_bold(); }
                    if fmt_btn(ui, egui::RichText::new("I").italics().size(13.0), ed.fmt_state_italic(), theme, "Italic (Ctrl+I)") { ed.apply_fmt_toggle_italic(); }
                    if fmt_btn(ui, egui::RichText::new("U").underline().size(13.0), ed.fmt_state_underline(), theme, "Underline (Ctrl+U)") { ed.apply_fmt_toggle_underline(); }

                    let cur_col = ed.cur_fmt.color.map(|c| egui::Color32::from_rgb(c[0], c[1], c[2])).unwrap_or(if is_dark { ColorPalette::ZINC_200 } else { egui::Color32::from_rgb(22, 22, 22) });
                    let color_btn = ui.scope(|ui| {
                        let s = ui.style_mut(); s.visuals.widgets.inactive.bg_fill = if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_200 };
                        s.visuals.widgets.hovered.bg_fill = if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 };
                        ui.add(egui::Button::new(egui::RichText::new("A").size(13.0).color(cur_col)).min_size(egui::vec2(24.0, 26.0)))
                    }).inner.on_hover_text("Text color");
                    let color_popup_id = color_btn.id;
                    { let r = color_btn.rect; ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(r.min.x+2.0, r.max.y-4.0), egui::vec2(r.width()-4.0, 3.0)), 1.0, cur_col); }
                    egui::Popup::from_toggle_button_response(&color_btn).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| { ui.set_min_width(140.0); text_color_palette(ui, ed, is_dark, color_popup_id); });
                    let _ = color_popup_id;

                    let hl_col = ed.cur_fmt.highlight.map(highlight_color32).unwrap_or(if is_dark { ColorPalette::ZINC_300 } else { ColorPalette::GRAY_500 });
                    let hl_btn = ui.scope(|ui| {
                        let s = ui.style_mut(); s.visuals.widgets.inactive.bg_fill = if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_200 };
                        s.visuals.widgets.hovered.bg_fill = if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 };
                        ui.add(egui::Button::new(egui::RichText::new("H").size(13.0).color(hl_col)).min_size(egui::vec2(24.0, 26.0)))
                    }).inner.on_hover_text("Highlight color");
                    let hl_popup_id = hl_btn.id;
                    { let r = hl_btn.rect; ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(r.min.x+2.0, r.max.y-4.0), egui::vec2(r.width()-4.0, 3.0)), 1.0, hl_col); }
                    egui::Popup::from_toggle_button_response(&hl_btn).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| { ui.set_min_width(160.0); highlight_color_palette(ui, ed, is_dark, hl_popup_id); });
                    let _ = hl_popup_id;

                    let link_btn = ui.add(egui::Button::new(egui::RichText::new("Link").size(11.0)).min_size(egui::vec2(42.0, 26.0)));
                    let link_popup_id = link_btn.id;
                    if link_btn.clicked() { ed.link_input = ed.cur_fmt.link.clone().unwrap_or_default(); }
                    egui::Popup::from_toggle_button_response(&link_btn).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            ui.set_min_width(220.0);
                            ui.label(egui::RichText::new("Link URL").size(11.0).color(lc));
                            ui.add(egui::TextEdit::singleline(&mut ed.link_input).desired_width(190.0).hint_text("https://..."));
                            ui.horizontal(|ui| {
                                if ui.button("Apply").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    let url = ed.link_input.trim().to_string(); ed.apply_fmt_link(if url.is_empty() { None } else { Some(url) });
                                    egui::Popup::close_id(ui.ctx(), link_popup_id);
                                }
                                if ui.button("Remove").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    ed.apply_fmt_link(None); egui::Popup::close_id(ui.ctx(), link_popup_id);
                                }
                            });
                        });
                    let _ = link_popup_id;

                    let tbl_btn = toolbar_action_btn(ui, "Table", theme).on_hover_text("Insert Table");
                    let tbl_pid = tbl_btn.id;
                    egui::Popup::from_toggle_button_response(&tbl_btn).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            let (def, hi, bdr) = if is_dark { (ColorPalette::ZINC_700, ColorPalette::BLUE_600, ColorPalette::ZINC_500) } else { (ColorPalette::GRAY_200, ColorPalette::BLUE_500, ColorPalette::GRAY_400) };
                            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                            for row in 0..8usize {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                                    for col in 0..8usize {
                                        let lit = row <= ed.table_picker_hover.0 && col <= ed.table_picker_hover.1;
                                        let (r, resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
                                        ui.painter().rect_filled(r, 2.0, if lit { hi } else { def });
                                        ui.painter().rect_stroke(r, 2.0, egui::Stroke::new(1.0, bdr), egui::StrokeKind::Middle);
                                        if resp.hovered() { ed.table_picker_hover = (row, col); }
                                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                            ed.insert_table(row + 1, col + 1); egui::Popup::close_id(ui.ctx(), tbl_pid);
                                        }
                                    }
                                });
                            }
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(format!("{}x{}", ed.table_picker_hover.1 + 1, ed.table_picker_hover.0 + 1)).size(11.0));
                        });
                    let _ = tbl_pid;
                    ui.separator();
                    let cur_align = ed.paras.get(ed.focused_para).map(|p| p.align).unwrap_or_default();
                    for a in Align::all() { if fmt_btn(ui, egui::RichText::new(a.label()).size(12.0), cur_align == *a, theme, a.full_label()) { ed.apply_align(*a); } }
                    ui.separator();
                    ui.label(egui::RichText::new("LH:").size(11.0).color(lc));
                    let lh_resp = ui.add(egui::DragValue::new(&mut ed.line_spacing_input).range(0.8..=4.0).speed(0.05).fixed_decimals(2));
                    if lh_resp.changed() { ed.apply_fmt_line_height(ed.line_spacing_input); }
                    ed.toolbar_has_focus = fs_resp.has_focus() || lh_resp.has_focus() || fs_resp.dragged() || lh_resp.dragged();
                    if ed.toolbar_has_focus { ui.ctx().input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Text(_)))); }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if act_btn(ui, "-", theme, "Zoom out (Ctrl+-)") { ed.zoom = (ed.zoom - 0.1).max(0.3); }
                        ui.label(egui::RichText::new(format!("{:.0}%", ed.zoom * 100.0)).size(11.0).color(lc));
                        if act_btn(ui, "+", theme, "Zoom in (Ctrl++)") { ed.zoom = (ed.zoom + 0.1).min(3.0); }
                    });
                    ui.separator();
                    let (status_text, status_col) = if ed.dirty { ("Unsaved", if is_dark { ColorPalette::AMBER_400 } else { ColorPalette::AMBER_600 }) }
                    else { ("Saved", if is_dark { ColorPalette::GREEN_400 } else { ColorPalette::GREEN_600 }) };
                    ui.label(egui::RichText::new(status_text).size(11.0).color(status_col));
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

struct ComputedPageLayout {para_page: Vec<usize>, para_content_y: Vec<f32>, page_tops: Vec<f32>}
fn compute_page_layout(ed: &DocumentEditor) -> ComputedPageLayout {
    let page_content_h = ed.layout.content_height();
    let mt = ed.layout.margin_top;
    let n = ed.paras.len();
    let mut para_page = vec![0usize; n];
    let mut para_content_y = vec![0.0f32; n];
    let mut page_tops = vec![PAGE_PAD];
    let mut cur_y = mt;
    let mut cur_page = 0;
    for i in 0..n {
        let mut h = ed.para_heights.get(i).copied().unwrap_or(DEFAULT_BASE_SIZE as f32 * 1.8);
        if h <= 0.0 { h = DEFAULT_BASE_SIZE as f32 * 1.2; }
        if cur_y > mt && cur_y + h > mt + page_content_h { cur_page += 1; page_tops.push(*page_tops.last().unwrap() + ed.layout.height * ed.zoom + PAGE_GAP); cur_y = mt; }
        para_page[i] = cur_page; para_content_y[i] = cur_y; cur_y += h;
        if h > page_content_h { cur_y = mt + page_content_h; }
    }
    ComputedPageLayout { para_page, para_content_y, page_tops }
}

fn measure_para_total_height(ctx: &egui::Context, para: &DocParagraph, wrap_w: f32, is_dark: bool) -> f32 {
    let job = build_layout_job(&para.spans, &para.text, para, wrap_w, is_dark, 1.0);
    let galley = ctx.fonts_mut(|f| f.layout_job(job));
    para.space_before + galley.rect.height() + para.space_after
}

fn split_spans_at_byte(spans: &[DocSpan], split_byte: usize) -> (Vec<DocSpan>, Vec<DocSpan>) {
    let mut left = Vec::new(); let mut right = Vec::new(); let mut pos = 0usize;
    for s in spans {
        if s.len == 0 { continue; }
        let (start, end) = (pos, pos + s.len);
        if end <= split_byte { left.push(s.clone()); }
        else if start >= split_byte { right.push(s.clone()); }
        else {
            let left_len = split_byte - start; let right_len = end - split_byte;
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
    left.space_after = 0.0; left.is_split = false; merge_adjacent(&mut left);
    let mut right = src.clone();
    right.text = src.text[split_byte..].to_string();
    right.spans = if right_spans.is_empty() { vec![DocSpan { len: 0, fmt: SpanFmt::default() }] } else { right_spans };
    right.space_before = 0.0; right.indent_first = 0.0; right.is_split = true; merge_adjacent(&mut right);
    (left, right)
}

fn find_split_byte_fit(ctx: &egui::Context, para: &DocParagraph, wrap_w: f32, max_total_h: f32, is_dark: bool) -> usize {
    let job = build_layout_job(&para.spans, &para.text, para, wrap_w, is_dark, 1.0);
    let galley = ctx.fonts_mut(|f| f.layout_job(job));
    let mut split_char = para.text.chars().count();
    let mut char_pos = 0usize;
    for row in &galley.rows {
        let row_bottom = para.space_before + row.rect().max.y;
        if row_bottom > max_total_h { split_char = char_pos; break; }
        char_pos += row.glyphs.len();
        if row.ends_with_newline { char_pos += 1; }
    }
    if split_char == 0 { return 0; }
    para.text.char_indices().nth(split_char).map(|(b, _)| b).unwrap_or(para.text.len())
}

fn reflow_overflow_paragraphs(ed: &mut DocumentEditor, ctx: &egui::Context, is_dark: bool) {
    if !ed.heights_dirty { return; }
    let mut focus_p = ed.focused_para; let mut focus_b = 0; let mut structure_changed = false;
    if focus_p < ed.paras.len() && focus_p < ed.para_ids.len() {
        if let Some(state) = egui::TextEdit::load_state(ctx, ed.para_ids[focus_p]) {
            if let Some(cr) = state.cursor.char_range() {
                let text = &ed.paras[focus_p].text;
                let char_idx = cr.primary.index;
                focus_b = text.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(text.len());
            }
        }
    }
    let page_content_h = ed.layout.content_height(); let cw = ed.layout.content_width();
    let bs = DEFAULT_BASE_SIZE as f32; let mt = ed.layout.margin_top; let min_fill = bs * 2.0;
    let mut j = 0;
    while j < ed.paras.len() {
        if ed.paras[j].is_split && j > 0 {
            let prev_len = ed.paras[j - 1].text.len();
            let orig_space_after = ed.paras[j].space_after;
            if focus_p == j { focus_p = j - 1; focus_b += prev_len; } else if focus_p > j { focus_p -= 1; }
            merge_paragraphs(&mut ed.paras, j - 1);
            ed.paras[j - 1].space_after = orig_space_after; ed.paras[j - 1].is_split = false; structure_changed = true;
        } else { j += 1; }
    }
    let mut cur_y = mt; let mut i = 0usize;
    while i < ed.paras.len() {
        let para = ed.paras[i].clone();
        if para.style == ParaStyle::HRule {
            let h = para.space_before + 12.0 + para.space_after; cur_y += h; if cur_y >= mt + page_content_h { cur_y = mt; } i += 1; continue;
        }
        if para.style == ParaStyle::Table {
            let h = ed.para_heights.get(i).copied().unwrap_or(bs * 1.6);
            if cur_y > mt && cur_y + h > mt + page_content_h { cur_y = mt; }
            cur_y += h.min(page_content_h); i += 1; continue;
        }
        if para.style == ParaStyle::Image {
            let h = para.image.as_ref().map(|img| img.display_h + 8.0).unwrap_or(20.0);
            if cur_y > mt && cur_y + h > mt + page_content_h { cur_y = mt; }
            cur_y += h.min(page_content_h); i += 1; continue;
        }
        let wrap_w = (cw - para.indent_left).max(40.0);
        let h = measure_para_total_height(ctx, &para, wrap_w, is_dark);
        let remaining = mt + page_content_h - cur_y;
        if h <= remaining + 0.5 { cur_y += h; if cur_y >= mt + page_content_h { cur_y = mt; } i += 1; continue; }
        if !para.text.is_empty() && cur_y > mt && remaining > min_fill {
            let split = find_split_byte_fit(ctx, &para, wrap_w, remaining, is_dark);
            if split > 0 && split < para.text.len() {
                let (left, right) = split_para_at_byte(&para, split);
                ed.paras[i] = left; ed.paras.insert(i + 1, right);
                if focus_p == i { if focus_b >= split { focus_p = i + 1; focus_b -= split; } } else if focus_p > i { focus_p += 1; }
                structure_changed = true; continue;
            }
        }
        cur_y = mt;
        if !para.text.is_empty() && h > page_content_h + 0.5 {
            let split = find_split_byte_fit(ctx, &para, wrap_w, page_content_h, is_dark);
            let split = split.max(para.text.char_indices().nth(1).map(|(b, _)| b).unwrap_or(para.text.len()));
            if split < para.text.len() {
                let (left, right) = split_para_at_byte(&para, split);
                ed.paras[i] = left; ed.paras.insert(i + 1, right);
                if focus_p == i { if focus_b >= split { focus_p = i + 1; focus_b -= split; } } else if focus_p > i { focus_p += 1; }
                structure_changed = true; continue;
            }
        }
        cur_y = (cur_y + h).min(mt + page_content_h);
        if cur_y >= mt + page_content_h { cur_y = mt; }
        i += 1;
    }
    let n = ed.paras.len();
    ed.para_texts.resize(n, String::new()); ed.para_ids.resize_with(n, || egui::Id::new(egui::Id::NULL)); ed.para_heights.resize(n, 0.0);
    for k in 0..n { ed.para_texts[k] = ed.paras[k].text.clone(); ed.para_ids[k] = egui::Id::new(("de_para", k as u64)); }
    if structure_changed {
        ed.doc_sel = None;
        if focus_p < n && ed.paras[focus_p].style != ParaStyle::Table && ed.paras[focus_p].style != ParaStyle::HRule {
            ed.focused_para = focus_p;
            if !ed.toolbar_has_focus {
                ed.pending_focus = Some(focus_p);
                let text = &ed.para_texts[focus_p];
                let safe_b = focus_b.min(text.len());
                let char_idx = text[..safe_b].chars().count();
                let id = ed.para_ids[focus_p];
                let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(char_idx))));
                egui::TextEdit::store_state(ctx, id, state);
            }
        }
    }
    ed.find_stale = true;
}

fn render_canvas(ed: &mut DocumentEditor, ui: &mut egui::Ui, ctx: &egui::Context, is_dark: bool) {
    let avail_w = ui.available_width();
    if !ed.auto_zoom_done && avail_w > 50.0 { ed.zoom = (avail_w * 0.40 / ed.layout.width).clamp(1.0, 2.5); ed.auto_zoom_done = true; }

    let page_w = ed.layout.width * ed.zoom; let page_h = ed.layout.height * ed.zoom;
    let ml = ed.layout.margin_left * ed.zoom; let mt = ed.layout.margin_top * ed.zoom; let mb = ed.layout.margin_bot * ed.zoom;
    let cw = ed.layout.content_width() * ed.zoom;
    let font = DEFAULT_BASE_FONT; let bs = DEFAULT_BASE_SIZE as f32 * ed.zoom;
    reflow_overflow_paragraphs(ed, ctx, is_dark);
    let n = ed.paras.len();

    if ed.heights_dirty || ed.para_heights.len() != n {
        ed.para_heights.resize(n, 0.0);
        for i in 0..n {
            let p = &ed.paras[i];
            if p.style == ParaStyle::HRule { ed.para_heights[i] = p.space_before + 12.0 + p.space_after; continue; }
            if p.style == ParaStyle::Table {
                if let Some(tbl) = &p.table {
                    let col_ws = table_col_widths(tbl, ed.layout.content_width());
                    let rows_h: f32 = tbl.rows.iter().enumerate().map(|(ri, row)| {
                        let live_cell = match ed.active_table { Some((ti, tr, tc)) if ti == i && tr == ri => Some((tc, ed.cell_edit_buf.as_str())), _ => None };
                        table_row_h(row, &col_ws, 1.0, ctx, live_cell)
                    }).sum();
                    ed.para_heights[i] = p.space_before + 6.0 + rows_h + p.space_after;
                } else { ed.para_heights[i] = 0.0; }
                continue;
            }
            if p.style == ParaStyle::Image { ed.para_heights[i] = p.image.as_ref().map(|img| img.display_h + 8.0).unwrap_or(20.0); continue; }
            let wrap_w = (ed.layout.content_width() - p.indent_left).max(40.0);
            let job = build_layout_job(&p.spans, &p.text, p, wrap_w, is_dark, 1.0);
            let galley = ctx.fonts_mut(|f| f.layout_job(job));
            ed.para_heights[i] = p.space_before + galley.rect.height() + p.space_after;
        }
        ed.heights_dirty = false;
    }

    let pl = compute_page_layout(ed);
    let total_scroll_h = pl.page_tops.last().copied().unwrap_or(PAGE_PAD) + page_h + PAGE_PAD;
    let scroll_target_y = ed.scroll_to_para.take().and_then(|t| pl.para_page.get(t).and_then(|&pg| pl.page_tops.get(pg)).map(|&pt| pt + pl.para_content_y[t] * ed.zoom - 80.0));

    let mut active_sel = ed.norm_sel().filter(|(f, t)| f.para != t.para);
    if active_sel.is_none() {
        if let Some((pi, sb, eb)) = ed.ctx_sel.filter(|(_, s, e)| s != e) {
            active_sel = Some((DocPos { para: pi, byte: sb.min(eb) }, DocPos { para: pi, byte: sb.max(eb) }));
        } else if let Some((pi, sb, eb)) = ed.last_selection {
            if sb != eb && !ctx.memory(|m| m.has_focus(ed.para_ids[pi])) {
                active_sel = Some((DocPos { para: pi, byte: sb.min(eb) }, DocPos { para: pi, byte: sb.max(eb) }));
            }
        }
    }
    let has_text_sel = ed.ctx_sel.map(|(_, s, e)| s != e).unwrap_or(false) || get_sel_text(ed).is_some();
    let sel_for_link = ed.ctx_sel.filter(|(_, s, e)| s != e).or(ed.last_selection.filter(|(_, s, e)| s != e));
    let sel_has_link = sel_for_link.map(|(pi, sb, eb)| {
        if pi >= ed.paras.len() { return false; }
        let (lo, hi) = (sb.min(eb), sb.max(eb));
        let mut pos = 0usize;
        for span in &ed.paras[pi].spans {
            let end = pos + span.len;
            if pos < hi && end > lo && span.fmt.link.is_some() { return true; }
            pos = end;
        }
        false
    }).unwrap_or(false);
    let ctx_action: std::cell::RefCell<Option<CtxAction>> = Default::default();
    let ca = &ctx_action;
    let cm_para: std::cell::Cell<Option<usize>> = std::cell::Cell::new(None);
    let cm_img_para: std::cell::Cell<Option<usize>> = std::cell::Cell::new(None);
    let cp = &cm_para;
    let cip = &cm_img_para;
    let ptr = ctx.pointer_hover_pos();
    let canvas_top = ui.available_rect_before_wrap().min.y;
    let press_origin = ctx.input(|i| i.pointer.press_origin());
    let drag_in_canvas = press_origin.map_or(true, |p| p.y >= canvas_top);
    let on_popup = press_origin.map_or(false, |p| ctx.layer_id_at(p).map_or(false, |l| l.order > egui::Order::Middle));
    let btn_pressed = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) && drag_in_canvas && !on_popup;
    let secondary_pressed = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) && drag_in_canvas && !on_popup;
    let btn_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary)) && drag_in_canvas && !on_popup;
    let shift = ctx.input(|i| i.modifiers.shift);
    if secondary_pressed {
        ed.ctx_sel = if ed.has_cross_sel() { None } else { ed.last_selection.filter(|(_, s, e)| s != e) };
    }
    if btn_pressed { ed.ctx_sel = None; }
    let ctrl = ctx.input(|i| i.modifiers.ctrl);
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
    let has_scroll_target = scroll_target_y.is_some();
    if let Some(off) = scroll_target_y { ed.doc_scroll_y = off.max(0.0); }
    let is_drag_active = ed.image_drag.is_some() || btn_down;
    if is_drag_active {
        let sw = ctx.input(|i| i.smooth_scroll_delta.y);
        if sw != 0.0 { ed.doc_scroll_y = (ed.doc_scroll_y - sw).clamp(0.0, (total_scroll_h - ui.available_height()).max(0.0)); ctx.request_repaint(); }
    }
    let mut scroll_area = egui::ScrollArea::vertical().id_salt("de_canvas_scroll").auto_shrink([false, false]).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);
    if is_drag_active || has_scroll_target { scroll_area = scroll_area.vertical_scroll_offset(ed.doc_scroll_y); }
    let mut table_cell_change: Option<(usize, usize, usize, String)> = None;
    let mut table_col_resize: Option<(usize, usize, f32)> = None;
    let tbl_struct_op = std::cell::Cell::new(None::<(u8, usize, usize, usize)>);
    let tbl_cell_bg_op = std::cell::Cell::new(None::<(usize, usize, usize, Option<[u8; 3]>)>);
    let tbl_border_op = std::cell::Cell::new(None::<(usize, [u8; 3], f32)>);
    let mut img_size_change: Option<(usize, f32, f32)> = None;
    let drop_idx_cell: std::cell::Cell<Option<usize>> = std::cell::Cell::new(None);

    let scroll_out = scroll_area.show_viewport(ui, |ui, vp| {
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
                painter.text(egui::pos2(pm.x + page_w / 2.0, pm.y + page_h - mb / 2.0), egui::Align2::CENTER_CENTER, format!("{}", pg_idx + 1), egui::FontId::proportional(9.0 * ed.zoom), num_col);
            }
        }

        for i in 0..n {
            let pg = pl.para_page[i]; let pt = pl.page_tops[pg];
            let pm = egui::pos2(outer.min.x + page_x, outer.min.y + pt);
            let para = ed.paras[i].clone();
            let indent = para.indent_left * ed.zoom;
            let wrap_w = (cw - indent).max(40.0);
            let content_y = pl.para_content_y[i];
            let total_h = ed.para_heights.get(i).copied().unwrap_or(DEFAULT_BASE_SIZE as f32 * 1.8);
            let space_b = para.space_before;
            let text_y = pm.y + content_y * ed.zoom + space_b * ed.zoom;
            let text_h = (total_h - space_b - para.space_after) * ed.zoom;
            let scroll_local_top = pt + content_y * ed.zoom;
            let near_view = !(scroll_local_top + total_h * ed.zoom < vp.min.y - page_h * 0.5 || scroll_local_top > vp.max.y + page_h * 0.5);
            let (edit_x, edit_w) = if matches!(para.align, Align::Left) { (pm.x + ml + indent, wrap_w) } else { (pm.x + ml, cw) };
            let edit_rect = egui::Rect::from_min_size(egui::pos2(edit_x, text_y), egui::vec2(edit_w, text_h));
            let checkbox_rect = if para.style == ParaStyle::ListCheck {
                let box_sz = (8.0 * ed.zoom).max(4.0);
                Some(egui::Rect::from_center_size(egui::pos2(edit_x - 8.0 * ed.zoom, text_y + text_h / 2.0), egui::vec2(box_sz, box_sz)))
            } else { None };

            if let Some(pp) = ptr.filter(|p| p.y >= canvas_top) {
                if let Some(cb) = checkbox_rect {
                    let cb_hit = cb.expand(4.0).contains(pp);
                    if cb_hit { ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                    if btn_pressed && cb_hit { ed.doc_sel = None; ed.push_undo(); ed.paras[i].checked = !ed.paras[i].checked; ed.dirty = true; }
                }

                if edit_rect.contains(pp) && ed.image_drag.is_none() {
                    if para.style == ParaStyle::Table {
                        if false && btn_pressed {
                            ed.doc_sel = None; ed.selected_image_para = None;
                            if let Some(ref tbl) = ed.paras[i].table {
                                let col_ws = table_col_widths(tbl, cw);
                                let mut ry = text_y + 6.0 * ed.zoom;
                                'tbl_click: for (ri, row) in tbl.rows.iter().enumerate() {
                                    let rh = table_row_h(row, &col_ws, ed.zoom, ctx, match ed.active_table { Some((ti, tr, tc)) if ti == i && tr == ri => Some((tc, ed.cell_edit_buf.as_str())), _ => None });
                                    if pp.y >= ry && pp.y < ry + rh {
                                        let mut cx_acc = pm.x + ml;
                                        let cc = col_ws.iter().enumerate().find_map(|(ci, &w)| { if pp.x < cx_acc + w { Some(ci) } else { cx_acc += w; None } }).unwrap_or(col_ws.len().saturating_sub(1)).min(row.len().saturating_sub(1));
                                        if shift {
                                            let anchor = ed.table_sel.filter(|(sp,_,_)| *sp == i).map(|(_,a,_)| a).unwrap_or((ri, cc));
                                            ed.table_sel = Some((i, anchor, (ri, cc)));
                                        } else if ed.active_table != Some((i, ri, cc)) {
                                            if let Some((old_i, old_r, old_c)) = ed.active_table { table_cell_change = Some((old_i, old_r, old_c, ed.cell_edit_buf.clone())); }
                                            ed.active_table = Some((i, ri, cc));
                                            ed.table_sel = Some((i, (ri, cc), (ri, cc)));
                                            ed.focused_para = i; ed.pending_focus = None;
                                            ed.cell_edit_buf = row.get(cc).map(|c| c.text.clone()).unwrap_or_default();
                                            let cell_id = ui.id().with(("table_cell", i, ri, cc));
                                            ctx.memory_mut(|m| m.request_focus(cell_id));
                                        }
                                        break 'tbl_click;
                                    }
                                    ry += rh;
                                }
                            }
                        }
                    } else if para.style != ParaStyle::Image {
                        let job = build_layout_job(&ed.paras[i].spans, &ed.paras[i].text, &ed.paras[i], wrap_w, is_dark, ed.zoom);
                        let galley = ctx.fonts_mut(|f| f.layout_job(job));
                        let rel = pp - egui::pos2(edit_x, text_y);
                        let cursor = galley.cursor_from_pos(rel);
                        let byte = char_to_byte(&ed.paras[i].text, cursor.index);
                        if btn_pressed && ctrl {
                            if let Some(url) = link_at_byte(&ed.paras[i], byte) {
                                let final_url: String = if url.starts_with("http://") || url.starts_with("https://") { url.to_string() } else { format!("https://{}", url) };
                                ctx.open_url(egui::OpenUrl::new_tab(&final_url));
                            } else {
                                let pos = DocPos { para: i, byte };
                                ed.doc_sel = if shift { ed.doc_sel.map(|[a, _]| [a, pos]).or(Some([pos, pos])) } else { Some([pos, pos]) };
                            }
                        } else if btn_pressed {
                            if let Some((pi, ri, ci)) = ed.active_table.take() { table_cell_change = Some((pi, ri, ci, ed.cell_edit_buf.clone())); }
                            ed.table_sel = None; ed.table_multi_sel = None; ed.selected_image_para = None;
                            let pos = DocPos { para: i, byte };
                            ed.doc_sel = if shift { ed.doc_sel.map(|[a, _]| [a, pos]).or(Some([pos, pos])) } else { Some([pos, pos]) };
                        } else if btn_down {
                            if ed.doc_sel.is_none() {
                                if let Some((pi, sb, _)) = ed.last_selection {
                                    let anchor = DocPos { para: pi, byte: sb }; let pos = DocPos { para: i, byte };
                                    if anchor.para != i || anchor.byte != byte { ed.doc_sel = Some([anchor, pos]); }
                                }
                            } else if let Some(ref mut sel) = ed.doc_sel {
                                if sel[0].para != i || (sel[0].para == i && sel[0].byte != byte) { sel[1] = DocPos { para: i, byte }; }
                            }
                        }
                    }
                }
            }

            if near_view {
                if para.style == ParaStyle::Image {
                    if let Some(img) = para.image.as_ref().cloned() {
                        let draw_w = img.display_w * ed.zoom; let draw_h = img.display_h * ed.zoom;
                        let mut preview_w = draw_w; let mut preview_h = draw_h;
                        if let Some(drag) = ed.image_drag { if drag.0 == i { let (nw, nh) = image_drag_dims(ed, ctx, drag); preview_w = nw * ed.zoom; preview_h = nh * ed.zoom; } }
                        let draw_x = match para.align { Align::Center => pm.x + ml + (cw - draw_w) / 2.0, Align::Right => pm.x + ml + cw - draw_w, _ => pm.x + ml };
                        let preview_x = match para.align { Align::Center => pm.x + ml + (cw - preview_w) / 2.0, Align::Right => pm.x + ml + cw - preview_w, _ => pm.x + ml };
                        let img_y = text_y + 4.0;
                        let img_rect = egui::Rect::from_min_size(egui::pos2(draw_x, img_y), egui::vec2(draw_w, draw_h));
                        let preview_rect = egui::Rect::from_min_size(egui::pos2(preview_x, img_y), egui::vec2(preview_w, preview_h));
                        let page_rect = egui::Rect::from_min_size(pm, egui::vec2(page_w, page_h));
                        let tid = get_doc_image_texture(ctx, ed, &img);
                        let over_image = ptr.map(|p| p.y >= canvas_top && img_rect.expand(8.0).contains(p)).unwrap_or(false);
                        if over_image { ctx.set_cursor_icon(egui::CursorIcon::PointingHand); }
                        if over_image && btn_pressed {
                            ed.selected_image_para = Some(i); ed.focused_para = i; ed.doc_sel = None;
                            if let Some((pi, ri, ci)) = ed.active_table.take() { table_cell_change = Some((pi, ri, ci, ed.cell_edit_buf.clone())); }
                            ed.table_multi_sel = None;
                        }
                        let img_tint = if ed.image_drag.as_ref().map(|(di, h, _, _, _, _)| *di == i && *h == 255).unwrap_or(false) { egui::Color32::from_rgba_unmultiplied(255, 255, 255, 100) } else { egui::Color32::WHITE };
                        painter.with_clip_rect(page_rect).image(tid, img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), img_tint);
                        let show_handles = ed.selected_image_para == Some(i) || ed.image_drag.as_ref().map(|(di, _, _, _, _, _)| *di == i).unwrap_or(false);
                        if show_handles {
                            painter.rect_stroke(preview_rect, 0.0, egui::Stroke::new(2.0, ColorPalette::BLUE_500), egui::StrokeKind::Outside);
                            for (hi, hp) in image_handle_positions(preview_rect).iter().enumerate() {
                                let hr = egui::Rect::from_center_size(*hp, egui::vec2(10.0, 10.0));
                                painter.rect_filled(hr, 1.0, egui::Color32::WHITE); painter.rect_stroke(hr, 1.0, egui::Stroke::new(1.5, ColorPalette::BLUE_500), egui::StrokeKind::Outside);
                                let h_resp = ui.interact(hr.expand(4.0), ui.id().with(("img_h", i, hi)), egui::Sense::click_and_drag());
                                if h_resp.hovered() || h_resp.dragged() { ctx.set_cursor_icon(image_handle_cursor(hi as u8)); }
                                if h_resp.drag_started() {
                                ed.selected_image_para = Some(i); ed.focused_para = i; ed.doc_sel = None;
                                if let Some((pi, ri, ci)) = ed.active_table.take() { table_cell_change = Some((pi, ri, ci, ed.cell_edit_buf.clone())); }
                                ed.image_drag = Some((i, hi as u8, h_resp.interact_pointer_pos().unwrap_or(*hp), img.display_w, img.display_h, img.display_w / img.display_h.max(0.001)));
                            }
                        }
                        let any_handle_hov = ptr.map_or(false, |p| image_handle_positions(preview_rect).iter().any(|hp| egui::Rect::from_center_size(*hp, egui::vec2(14.0, 14.0)).contains(p)));
                        let no_resize_drag = ed.image_drag.as_ref().map_or(true, |(di, h, _, _, _, _)| *di != i || *h == 255);
                        if !any_handle_hov && no_resize_drag {
                            let body = ui.interact(img_rect, ui.id().with(("img_move", i)), egui::Sense::drag());
                            if body.hovered() { ctx.set_cursor_icon(egui::CursorIcon::Grab); }
                            if body.dragged() && ed.image_drag.as_ref().map(|(di, h, _, _, _, _)| *di == i && *h == 255).unwrap_or(false) { ctx.set_cursor_icon(egui::CursorIcon::Grabbing); }
                            if body.drag_started() {
                                ed.image_drag = Some((i, 255u8, body.interact_pointer_pos().unwrap_or(img_rect.center()), img.display_w, img.display_h, img.display_w / img.display_h.max(0.001)));
                            }
                        }
                    }
                    let img_ctx = ui.interact(img_rect, ui.id().with(("doc_img_cm", i)), egui::Sense::click());
                    img_ctx.context_menu(|ui| { cip.set(Some(i)); img_cm(ui, i, ca); });
                    }
                    continue;
                }
                
                if para.style == ParaStyle::HRule {
                    let rule_col = if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 };
                    let mid_y = text_y + text_h / 2.0;
                    painter.rect_filled(egui::Rect::from_min_size(egui::pos2(pm.x + ml, mid_y - 1.0), egui::vec2(cw, 2.0)), 1.0, rule_col);
                    if let Some((from, to)) = active_sel {
                        if i >= from.para && i <= to.para {
                            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(cw, text_h.max(4.0))), 0.0, sel_color);
                        }
                    }
                    continue;
                }

                if para.style == ParaStyle::Table {
                    if let Some(ref tbl) = para.table {
                        let col_ws = table_col_widths(tbl, cw);
                        let nc = tbl.rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
                        let table_top_pad = 6.0 * ed.zoom; let table_y = text_y + table_top_pad;
                        let row_hs: Vec<f32> = tbl.rows.iter().enumerate().map(|(ri, row)| {
                            let live_cell = match ed.active_table { Some((ti, tr, tc)) if ti == i && tr == ri => Some((tc, ed.cell_edit_buf.as_str())), _ => None };
                            table_row_h(row, &col_ws, ed.zoom, ctx, live_cell)
                        }).collect();
                        let total_h: f32 = row_hs.iter().sum();
                        let tbl_bg = if is_dark { egui::Color32::from_rgb(28, 28, 36) } else { egui::Color32::WHITE };
                        let alt_bg = if is_dark { egui::Color32::from_rgb(24, 24, 30) } else { ColorPalette::GRAY_50 };
                        let bdr = egui::Color32::from_rgb(tbl.border_color[0], tbl.border_color[1], tbl.border_color[2]);
                        let bdr_stroke = egui::Stroke::new(tbl.border_width, bdr);
                        let tc = if is_dark { ColorPalette::ZINC_200 } else { ColorPalette::GRAY_800 };
                        let border_color_snap = tbl.border_color; let border_width_snap = tbl.border_width;
                        let mut ry = table_y;
                        for (ri, row) in tbl.rows.iter().enumerate() {
                            let rh = row_hs[ri]; let mut cx = pm.x + ml;
                            for (ci, cell) in row.iter().enumerate() {
                                let cw_cell = col_ws.get(ci).copied().unwrap_or(cw / nc as f32);
                                let cell_bg = cell.bg_color.map(|c| egui::Color32::from_rgb(c[0], c[1], c[2])).unwrap_or(if ri % 2 == 0 { tbl_bg } else { alt_bg });
                                let cell_rect = egui::Rect::from_min_size(egui::pos2(cx, ry), egui::vec2(cw_cell, rh));
                                painter.rect_filled(cell_rect, 0.0, cell_bg);
                                if cell_in_sel(ed.table_sel, i, ri, ci) || cell_in_multi_sel(ed.table_multi_sel.as_ref(), i, ri, ci) { painter.rect_filled(cell_rect, 0.0, sel_color); }
                                if btn_pressed && ctrl && !shift && ptr.map_or(false, |p| cell_rect.contains(p) && p.y >= canvas_top) && ed.active_table == Some((i, ri, ci)) {
                                    table_cell_change = Some((i, ri, ci, ed.cell_edit_buf.clone()));
                                    ed.active_table = None; ed.table_text_sel = None; ed.last_selection = None; ed.doc_sel = None;
                                }
                                let is_active = ed.active_table == Some((i, ri, ci));
                                let cell_ctx_id = ui.id().with(("tc_ctx", i, ri, ci));
                                let cell_resp = ui.interact(cell_rect, cell_ctx_id, egui::Sense::click());
                                if cell_resp.hovered() { ctx.set_cursor_icon(egui::CursorIcon::Text); }
                                if cell_resp.clicked() {
                                    ed.last_selection = None;
                                    ed.doc_sel = None;
                                    if ctrl && !shift {
                                        if let Some((old_i, old_r, old_c)) = ed.active_table.take() { table_cell_change = Some((old_i, old_r, old_c, ed.cell_edit_buf.clone())); }
                                        ed.active_table = None; ed.table_text_sel = None;
                                        if ed.table_multi_sel.as_ref().map_or(false, |(sp, _)| *sp == i) {
                                            let ms = ed.table_multi_sel.as_mut().unwrap();
                                            if let Some(p) = ms.1.iter().position(|&(r, c)| r == ri && c == ci) { ms.1.remove(p); }
                                            else { ms.1.push((ri, ci)); }
                                            if ms.1.is_empty() { ed.table_multi_sel = None; }
                                        } else {
                                            ed.table_sel = None;
                                            ed.table_multi_sel = Some((i, vec![(ri, ci)]));
                                        }
                                        ed.focused_para = i;
                                    } else if shift {
                                        if let Some((old_i, old_r, old_c)) = ed.active_table.take() { table_cell_change = Some((old_i, old_r, old_c, ed.cell_edit_buf.clone())); }
                                        ed.table_multi_sel = None;
                                        let anchor = ed.table_sel.filter(|(sp, _, _)| *sp == i).map(|(_, a, _)| a).unwrap_or((ri, ci));
                                        ed.table_sel = Some((i, anchor, (ri, ci)));
                                        ed.table_text_sel = None; ed.active_table = None;
                                    } else if ed.active_table != Some((i, ri, ci)) {
                                        if let Some((old_i, old_r, old_c)) = ed.active_table { table_cell_change = Some((old_i, old_r, old_c, ed.cell_edit_buf.clone())); }
                                        ed.table_multi_sel = None;
                                        ed.active_table = Some((i, ri, ci));
                                        ed.table_sel = Some((i, (ri, ci), (ri, ci)));
                                        ed.table_text_sel = None;
                                        ed.focused_para = i; ed.pending_focus = None;
                                        ed.cell_edit_buf = cell.text.clone();
                                        let cell_id = ui.id().with(("table_cell", i, ri, ci));
                                        ctx.memory_mut(|m| m.request_focus(cell_id));
                                    } else {
                                        let cell_id = ui.id().with(("table_cell", i, ri, ci));
                                        ctx.memory_mut(|m| m.request_focus(cell_id));
                                    }
                                }
                                if is_active {
                                    painter.rect_filled(cell_rect, 0.0, egui::Color32::from_rgba_unmultiplied(59, 130, 246, 50));
                                    let cr = egui::Rect::from_min_size(egui::pos2(cx + 4.0, ry + 2.0), egui::vec2(cw_cell - 8.0, rh - 4.0));
                                    let cell_id = ui.id().with(("table_cell", i, ri, ci));
                                    let pre_state = egui::TextEdit::load_state(ctx, cell_id);
                                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(cr));
                                    let te = egui::TextEdit::multiline(&mut ed.cell_edit_buf).id(cell_id)
                                        .desired_width(cw_cell - 8.0).min_size(egui::vec2(cw_cell - 8.0, rh - 4.0))
                                        .desired_rows(1).frame(false).lock_focus(true)
                                        .font(egui::FontId::new(bs * 0.9, font.egui_family(false, false))).show(&mut child);
                                    if te.response.changed() { ed.heights_dirty = true; ed.dirty = true; }
                                    if let Some(state) = egui::TextEdit::load_state(ctx, cell_id) {
                                        if let Some(cr) = state.cursor.char_range() {
                                            let si = cr.primary.index.min(cr.secondary.index);
                                            let ei = cr.primary.index.max(cr.secondary.index);
                                            if si != ei {
                                                let sb = char_to_byte(&ed.cell_edit_buf, si);
                                                let eb = char_to_byte(&ed.cell_edit_buf, ei);
                                                ed.table_text_sel = Some((i, ri, ci, sb, eb));
                                                if let Some(cell) = tbl.rows.get(ri).and_then(|row| row.get(ci)) {
                                                    let tmp = DocParagraph { text: ed.cell_edit_buf.clone(), spans: cell.spans.clone(), style: ParaStyle::Normal, align: Align::Left, indent_left: 0.0, indent_first: 0.0, space_before: 0.0, space_after: 0.0, line_height: 1.15, list_num: None, checked: false, is_split: false, table: None, image: None };
                                                    ed.cur_fmt.bold = all_set_range(&tmp, sb, eb, |f| f.bold);
                                                    ed.cur_fmt.italic = all_set_range(&tmp, sb, eb, |f| f.italic);
                                                    ed.cur_fmt.underline = all_set_range(&tmp, sb, eb, |f| f.underline);
                                                    ed.cur_fmt.strike = all_set_range(&tmp, sb, eb, |f| f.strike);
                                                    ed.cur_fmt.sub = all_set_range(&tmp, sb, eb, |f| f.sub);
                                                    ed.cur_fmt.sup = all_set_range(&tmp, sb, eb, |f| f.sup);
                                                    ed.cur_fmt.size_hp = para_fmt_at(&tmp, sb).size_hp;
                                                    ed.cur_fmt.font = para_fmt_at(&tmp, sb).font;
                                                    ed.cur_fmt.color = para_fmt_at(&tmp, sb).color;
                                                    ed.cur_fmt.highlight = para_fmt_at(&tmp, sb).highlight;
                                                    ed.cur_fmt.link = para_fmt_at(&tmp, sb).link;
                                                }
                                            } else {
                                                ed.table_text_sel = None;
                                                if let Some(cell) = tbl.rows.get(ri).and_then(|row| row.get(ci)) {
                                                    let tmp = DocParagraph { text: ed.cell_edit_buf.clone(), spans: cell.spans.clone(), style: ParaStyle::Normal, align: Align::Left, indent_left: 0.0, indent_first: 0.0, space_before: 0.0, space_after: 0.0, line_height: 1.15, list_num: None, checked: false, is_split: false, table: None, image: None };
                                                    let sb = char_to_byte(&ed.cell_edit_buf, si);
                                                    let fmt = para_fmt_at(&tmp, sb);
                                                    let c = &mut ed.cur_fmt;
                                                    c.bold = fmt.bold; c.italic = fmt.italic; c.underline = fmt.underline;
                                                    c.strike = fmt.strike; c.sub = fmt.sub; c.sup = fmt.sup;
                                                    c.size_hp = fmt.size_hp; c.font = fmt.font; c.color = fmt.color;
                                                    c.highlight = fmt.highlight; c.link = fmt.link;
                                                }
                                            }
                                        }
                                    }
                                    if te.response.lost_focus() || ctx.input(|inp| inp.key_pressed(egui::Key::Escape)) {
                                        table_cell_change = Some((i, ri, ci, ed.cell_edit_buf.clone()));
                                        ed.table_text_sel = None;
                                        ed.active_table = None;
                                    }
                                    if te.response.has_focus() {
                                        let no_modifiers = ctx.input(|inp| inp.modifiers.is_none());
                                        let up_pressed = ctx.input(|inp| inp.key_pressed(egui::Key::ArrowUp)) && no_modifiers;
                                        let down_pressed = ctx.input(|inp| inp.key_pressed(egui::Key::ArrowDown)) && no_modifiers;
                                        let left_pressed = ctx.input(|inp| inp.key_pressed(egui::Key::ArrowLeft)) && no_modifiers;
                                        let right_pressed = ctx.input(|inp| inp.key_pressed(egui::Key::ArrowRight)) && no_modifiers;
                                        if up_pressed || down_pressed || left_pressed || right_pressed {
                                            if let Some(state) = pre_state.or_else(|| egui::TextEdit::load_state(ctx, cell_id)) {
                                                if let Some(cr) = state.cursor.char_range() {
                                                    let galley = te.galley.clone();
                                                    let mut is_top = false; let mut is_bottom = false; let mut char_pos = 0usize;
                                                    for (row_idx, row_layout) in galley.rows.iter().enumerate() {
                                                        let row_start = char_pos; let glyph_count = row_layout.glyphs.len(); let row_end = char_pos + glyph_count;
                                                        let is_last_row = row_idx == galley.rows.len() - 1;
                                                        let next_char_pos = row_end + if row_layout.ends_with_newline { 1 } else { 0 };
                                                        if cr.primary.index >= row_start && (cr.primary.index < next_char_pos || is_last_row) {
                                                            is_top = row_idx == 0; is_bottom = is_last_row; break;
                                                        }
                                                        char_pos = next_char_pos;
                                                    }
                                                    let cell_len = ed.cell_edit_buf.chars().count();
                                                    let at_start = cr.primary.index == 0 && cr.secondary.index == 0;
                                                    let at_end = cr.primary.index == cell_len && cr.secondary.index == cell_len;
                                                    let mut nav_to_cell = None; let mut nav_to_para = None;
                                                    if up_pressed && is_top {
                                                        ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                                                        if ri > 0 { nav_to_cell = Some((ri - 1, ci, true)); } else {
                                                            let mut t = i.saturating_sub(1); while t > 0 && ed.paras[t].style == ParaStyle::HRule { t -= 1; }
                                                            if ed.paras[t].style != ParaStyle::HRule { nav_to_para = Some((t, true)); }
                                                        }
                                                    } else if down_pressed && is_bottom {
                                                        ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
                                                        if ri + 1 < tbl.rows.len() { nav_to_cell = Some((ri + 1, ci, false)); } else {
                                                            let mut t = i + 1; while t + 1 < ed.paras.len() && ed.paras[t].style == ParaStyle::HRule { t += 1; }
                                                            if t < ed.paras.len() && ed.paras[t].style != ParaStyle::HRule { nav_to_para = Some((t, false)); }
                                                        }
                                                    } else if left_pressed && at_start {
                                                        ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft));
                                                        if ci > 0 { nav_to_cell = Some((ri, ci - 1, true)); }
                                                        else if ri > 0 { nav_to_cell = Some((ri - 1, tbl.rows[ri - 1].len().saturating_sub(1), true)); }
                                                        else {
                                                            let mut t = i.saturating_sub(1); while t > 0 && ed.paras[t].style == ParaStyle::HRule { t -= 1; }
                                                            if ed.paras[t].style != ParaStyle::HRule { nav_to_para = Some((t, true)); }
                                                        }
                                                    } else if right_pressed && at_end {
                                                        ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight));
                                                        if ci + 1 < tbl.rows[ri].len() { nav_to_cell = Some((ri, ci + 1, false)); }
                                                        else if ri + 1 < tbl.rows.len() { nav_to_cell = Some((ri + 1, 0, false)); }
                                                        else {
                                                            let mut t = i + 1; while t + 1 < ed.paras.len() && ed.paras[t].style == ParaStyle::HRule { t += 1; }
                                                            if t < ed.paras.len() && ed.paras[t].style != ParaStyle::HRule { nav_to_para = Some((t, false)); }
                                                        }
                                                    }
                                                    if let Some((target_r, target_c, to_bottom)) = nav_to_cell {
                                                        table_cell_change = Some((i, ri, ci, ed.cell_edit_buf.clone()));
                                                        ed.table_text_sel = None;
                                                        ed.active_table = Some((i, target_r, target_c));
                                                        ed.table_sel = Some((i, (target_r, target_c), (target_r, target_c)));
                                                        ed.cell_edit_buf = tbl.rows[target_r].get(target_c).map(|c| c.text.clone()).unwrap_or_default();
                                                        let next_id = ui.id().with(("table_cell", i, target_r, target_c));
                                                        ctx.memory_mut(|m| m.request_focus(next_id));
                                                        let mut target_state = egui::TextEdit::load_state(ctx, next_id).unwrap_or_default();
                                                        let new_idx = if to_bottom { ed.cell_edit_buf.chars().count() } else { 0 };
                                                        target_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(new_idx))));
                                                        egui::TextEdit::store_state(ctx, next_id, target_state);
                                                    } else if let Some((target_i, to_bottom)) = nav_to_para {
                                                        table_cell_change = Some((i, ri, ci, ed.cell_edit_buf.clone()));
                                                        ed.active_table = None;
                                                        if ed.paras[target_i].style == ParaStyle::Table {
                                                            if let Some(ref target_tbl) = ed.paras[target_i].table {
                                                                let next_r = if to_bottom { target_tbl.rows.len().saturating_sub(1) } else { 0 }; let next_c = 0;
                                                                ed.table_text_sel = None;
                                                                ed.active_table = Some((target_i, next_r, next_c));
                                                                ed.table_sel = Some((target_i, (next_r, next_c), (next_r, next_c)));
                                                                ed.cell_edit_buf = target_tbl.rows[next_r].get(next_c).map(|c| c.text.clone()).unwrap_or_default();
                                                                let next_id = ui.id().with(("table_cell", target_i, next_r, next_c));
                                                                ctx.memory_mut(|m| m.request_focus(next_id));
                                                                let mut target_state = egui::TextEdit::load_state(ctx, next_id).unwrap_or_default();
                                                                let new_idx = if to_bottom { ed.cell_edit_buf.chars().count() } else { 0 };
                                                                target_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(new_idx))));
                                                                egui::TextEdit::store_state(ctx, next_id, target_state);
                                                                ed.focused_para = target_i; pending_focus_next = Some(target_i);
                                                            }
                                                        } else {
                                                            let target_id = ed.para_ids[target_i];
                                                            let mut target_state = egui::TextEdit::load_state(ctx, target_id).unwrap_or_default();
                                                            let new_idx = if to_bottom { ed.para_texts[target_i].chars().count() } else { 0 };
                                                            target_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(new_idx))));
                                                            egui::TextEdit::store_state(ctx, target_id, target_state);
                                                            ed.pending_focus = Some(target_i); pending_focus_next = Some(target_i);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        let (tab_fwd, tab_bwd) = ctx.input_mut(|inp| (inp.consume_key(egui::Modifiers::NONE, egui::Key::Tab), inp.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab)));
                                        if tab_fwd || tab_bwd {
                                            table_cell_change = Some((i, ri, ci, ed.cell_edit_buf.clone()));
                                            let nr = tbl.rows.len(); let nc = tbl.rows.get(ri).map(|r| r.len()).unwrap_or(1);
                                            let (next_r, next_c) = if tab_fwd { if ci + 1 < nc { (ri, ci + 1) } else if ri + 1 < nr { (ri + 1, 0) } else { (0, 0) } }
                                            else if ci > 0 { (ri, ci - 1) } else if ri > 0 { (ri - 1, tbl.rows[ri - 1].len().saturating_sub(1)) }
                                            else { (nr.saturating_sub(1), tbl.rows.last().map(|r| r.len()).unwrap_or(1).saturating_sub(1)) };
                                            let new_buf = tbl.rows.get(next_r).and_then(|r| r.get(next_c)).map(|c| c.text.clone()).unwrap_or_default();
                                            ed.table_text_sel = None; ed.active_table = Some((i, next_r, next_c)); ed.table_sel = Some((i, (next_r, next_c), (next_r, next_c))); ed.cell_edit_buf = new_buf;
                                            let nid = ui.id().with(("table_cell", i, next_r, next_c));
                                            ctx.memory_mut(|m| m.request_focus(nid));
                                            let mut ns = egui::TextEdit::load_state(ctx, nid).unwrap_or_default();
                                            ns.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(0))));
                                            egui::TextEdit::store_state(ctx, nid, ns);
                                        }
                                    }
                                } else {
                                    if ptr.map(|p| cell_rect.contains(p)).unwrap_or(false) { ctx.set_cursor_icon(egui::CursorIcon::Text); }
                                    let cell_para = DocParagraph { text: cell.text.clone(), spans: cell.spans.clone(), style: ParaStyle::Normal, align: Align::Left, indent_left: 0.0, indent_first: 0.0, space_before: 0.0, space_after: 0.0, line_height: 1.15, list_num: None, checked: false, is_split: false, table: None, image: None };
                                    let job = build_layout_job(&cell_para.spans, &cell_para.text, &cell_para, (cw_cell - 16.0 * ed.zoom).max(1.0), is_dark, ed.zoom * 0.9);
                                    let galley = ctx.fonts_mut(|f| f.layout_job(job));
                                    let ty = ry + (rh - galley.rect.height()).max(0.0) / 2.0;
                                    let clip = egui::Rect::from_min_size(egui::pos2(cx + 4.0, ry), egui::vec2(cw_cell - 8.0, rh));
                                    painter.with_clip_rect(clip).galley(egui::pos2(cx + 8.0 * ed.zoom, ty), galley, tc);
                                }
                                if ctx.input(|inp| inp.pointer.button_pressed(egui::PointerButton::Secondary)) && ptr.map_or(false, |p| p.y >= canvas_top && cell_rect.contains(p)) {
                                    egui::Popup::open_id(ui.ctx(), cell_ctx_id.with("__context_menu"));
                                }
                                cell_resp.context_menu(|ui| {
                                    ui.set_min_width(200.0);
                                    let lc = if is_dark { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
                                    ui.label(egui::RichText::new("ROW").size(10.0).color(lc));
                                    ui.horizontal(|ui| {
                                        if ui.button("+ Above").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { tbl_struct_op.set(Some((0, i, ri, ci))); ui.close(); }
                                        if ui.button("+ Below").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { tbl_struct_op.set(Some((1, i, ri, ci))); ui.close(); }
                                        if ui.button("Delete").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { tbl_struct_op.set(Some((4, i, ri, ci))); ui.close(); }
                                    });
                                    ui.label(egui::RichText::new("COLUMN").size(10.0).color(lc));
                                    ui.horizontal(|ui| {
                                        if ui.button("+ Left").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { tbl_struct_op.set(Some((2, i, ri, ci))); ui.close(); }
                                        if ui.button("+ Right").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { tbl_struct_op.set(Some((3, i, ri, ci))); ui.close(); }
                                        if ui.button("Delete").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { tbl_struct_op.set(Some((5, i, ri, ci))); ui.close(); }
                                    });
                                    ui.separator();
                                    ui.label(egui::RichText::new("CELL BACKGROUND").size(10.0).color(lc));
                                    if ui.button("Clear").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { tbl_cell_bg_op.set(Some((i, ri, ci, None))); ui.close(); }
                                    ui.add_space(2.0);
                                    const CP: &[([u8; 3], &str)] = &[
                                        ([255,255,255],"White"),([229,231,235],"Gray"),([239,246,255],"Blue 50"),([219,234,254],"Blue 100"),([240,253,244],"Green 50"),([204,251,241],"Teal 50"),([255,251,235],"Amber 50"),
                                        ([254,242,242],"Red 50"),([255,228,230],"Rose 50"),([250,245,255],"Purple 50"),([167,243,208],"Mint"),([147,197,253],"Sky"),([253,230,138],"Yellow"),([196,181,253],"Lavender"),
                                    ];
                                    for chunk in CP.chunks(7) {
                                        ui.horizontal(|ui| {
                                            for &(c, name) in chunk {
                                                let bd = if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 };
                                                if ui.add(egui::Button::new("").fill(egui::Color32::from_rgb(c[0],c[1],c[2])).stroke(egui::Stroke::new(1.0,bd)).min_size(egui::vec2(20.0,20.0)).corner_radius(2.0)).on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                                    tbl_cell_bg_op.set(Some((i, ri, ci, Some(c)))); ui.close();
                                                }
                                            }
                                        });
                                    }
                                    ui.separator();
                                    ui.label(egui::RichText::new("TABLE BORDER COLOR").size(10.0).color(lc));
                                    const BP: &[([u8; 3], &str)] = &[
                                        ([0,0,0],"Black"),([55,65,81],"Dark Gray"),([100,100,110],"Gray"),([156,163,175],"Silver"),([209,213,219],"Light Gray"),([255,255,255],"White"),([220,38,38],"Red"),
                                        ([234,88,12],"Orange"),([234,179,8],"Yellow"),([22,163,74],"Green"),([20,184,166],"Teal"),([59,130,246],"Blue"),([168,85,247],"Purple"),([236,72,153],"Pink"),
                                    ];
                                    for chunk in BP.chunks(7) {
                                        ui.horizontal(|ui| {
                                            for &(c, name) in chunk {
                                                let bd = if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 }; let active = border_color_snap == c;
                                                let stroke = if active { egui::Stroke::new(2.0, ColorPalette::BLUE_400) } else { egui::Stroke::new(1.0, bd) };
                                                if ui.add(egui::Button::new("").fill(egui::Color32::from_rgb(c[0],c[1],c[2])).stroke(stroke).min_size(egui::vec2(20.0,20.0)).corner_radius(2.0)).on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                                    tbl_border_op.set(Some((i, c, border_width_snap))); ui.close();
                                                }
                                            }
                                        });
                                    }
                                    ui.add_space(2.0);
                                    ui.label(egui::RichText::new("BORDER WIDTH").size(10.0).color(lc));
                                    ui.horizontal(|ui| {
                                        for w in [0.5f32, 1.0, 1.5, 2.0, 3.0] {
                                            let active = (border_width_snap - w).abs() < 0.1;
                                            let (fill, fc) = if active { (ColorPalette::BLUE_600, egui::Color32::WHITE) } else if is_dark { (ColorPalette::ZINC_700, ColorPalette::ZINC_300) } else { (ColorPalette::GRAY_200, ColorPalette::GRAY_700) };
                                            if ui.add(egui::Button::new(egui::RichText::new(format!("{}", w)).size(11.0).color(fc)).fill(fill).stroke(egui::Stroke::NONE).min_size(egui::vec2(32.0, 22.0))).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                                tbl_border_op.set(Some((i, border_color_snap, w))); ui.close();
                                            }
                                        }
                                    });
                                    ui.separator();
                                    if ui.button(egui::RichText::new("Delete Table").color(ColorPalette::RED_400)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { tbl_struct_op.set(Some((6, i, ri, ci))); ui.close(); }
                                });
                                if ci > 0 { painter.vline(cx, ry..=(ry + rh), bdr_stroke); }
                                cx += cw_cell;
                            }
                            if ri > 0 { painter.hline((pm.x + ml)..=(pm.x + ml + cw), ry, bdr_stroke); }
                            ry += rh;
                        }
                        painter.rect_stroke(egui::Rect::from_min_size(egui::pos2(pm.x + ml, table_y), egui::vec2(cw, total_h)), 0.0, bdr_stroke, egui::StrokeKind::Outside);
                        let mut cx_acc = pm.x + ml;
                        for col in 1..nc {
                            cx_acc += col_ws[col - 1];
                            let hr = egui::Rect::from_min_size(egui::pos2(cx_acc - 3.0, table_y), egui::vec2(6.0, total_h));
                            let hr_resp = ui.interact(hr, ui.id().with(("tcr", i, col)), egui::Sense::drag());
                            if hr_resp.hovered() || hr_resp.dragged() { ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal); }
                            if hr_resp.dragged() && table_col_resize.is_none() { table_col_resize = Some((i, col, hr_resp.drag_delta().x)); }
                        }
                    }
                    continue;
                }

                if i == focused { painter.rect_filled(egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y - 2.0), egui::vec2(cw, text_h + 4.0)), 0.0, focus_bg); }

                match para.style {
                    ParaStyle::BlockQuote => {
                        painter.rect_filled(egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(cw, text_h)), 0.0, bq_bg);
                        painter.rect_filled(egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(3.0, text_h)), 1.0, ColorPalette::BLUE_500);
                    }
                    ParaStyle::Code => {
                        painter.rect_filled(egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(cw, text_h)), 3.0, if is_dark { egui::Color32::from_rgb(24, 24, 30) } else { egui::Color32::from_rgb(244, 244, 248) });
                        painter.rect_filled(egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(3.0, text_h)), 1.0, code_left);
                    }
                    ParaStyle::ListBullet => { painter.circle_filled(egui::pos2(edit_x - 8.0 * ed.zoom, text_y + text_h / 2.0), 2.5 * ed.zoom, bullet_col); }
                    ParaStyle::ListOrdered => {
                        let num = para.list_num.unwrap_or_else(|| ed.paras[..i].iter().rev().take_while(|p| p.style == ParaStyle::ListOrdered).count() as u32 + 1);
                        painter.text(egui::pos2(edit_x - 4.0, text_y), egui::Align2::RIGHT_TOP, format!("{}.", num), egui::FontId::proportional(para.style.default_font_size_pt() as f32 * ed.zoom * 0.9), bullet_col);
                    }
                    ParaStyle::ListCheck => {
                        let box_sz = (8.0 * ed.zoom).max(4.0);
                        let box_rect = egui::Rect::from_center_size(egui::pos2(edit_x - 8.0 * ed.zoom, text_y + text_h / 2.0), egui::vec2(box_sz, box_sz));
                        let rnd = (2.0 * ed.zoom).max(1.0); let sw = (1.4 * ed.zoom).max(0.8);
                        painter.rect_stroke(box_rect, rnd, egui::Stroke::new(sw, bullet_col), egui::StrokeKind::Outside);
                        if para.checked {
                            let tw = (1.8 * ed.zoom).max(0.8);
                            painter.line_segment([box_rect.left_top() + egui::vec2(box_sz * 0.18, box_sz * 0.55), box_rect.center() + egui::vec2(-box_sz * 0.08, box_sz * 0.18)], egui::Stroke::new(tw, bullet_col));
                            painter.line_segment([box_rect.center() + egui::vec2(-box_sz * 0.10, box_sz * 0.15), box_rect.right_top() + egui::vec2(-box_sz * 0.16, box_sz * 0.28)], egui::Stroke::new(tw, bullet_col));
                        }
                    }
                    _ => {}
                }

                let mut galley_cache: Option<std::sync::Arc<egui::text::Galley>> = None;
                let mut get_galley = || -> std::sync::Arc<egui::text::Galley> {
                    if let Some(ref g) = galley_cache { return g.clone(); }
                    let job = build_layout_job(&para.spans, &para.text, &para, wrap_w, is_dark, ed.zoom);
                    let g = ctx.fonts_mut(|f| f.layout_job(job));
                    galley_cache = Some(g.clone()); g
                };

                if let Some(errors) = ed.spell_errors.get(i) {
                    if !errors.is_empty() && near_view {
                        let galley = get_galley();
                        let align_offset = match para.align {
                            Align::Center => ((edit_w - galley.rect.width() - 8.0) / 2.0).max(0.0),
                            Align::Right => (edit_w - galley.rect.width() - 8.0).max(0.0),
                            _ => 0.0,
                        };
                        let squig_col = egui::Color32::from_rgb(220, 38, 38);
                        for &(sb, se) in errors {
                            for rect in multiline_highlight(&galley, &para.text, sb, se) {
                                let tr = rect.translate(egui::vec2(edit_x + align_offset, text_y));
                                draw_squiggle(&painter, tr, squig_col);
                            }
                        }
                    }
                }

                if let Some((fi, fs, fe)) = find_hl {
                    if fi == i {
                        let galley = get_galley();
                        let align_offset = match para.align { Align::Center => ((edit_w - galley.rect.width() - 8.0) / 2.0).max(0.0), Align::Right => (edit_w - galley.rect.width() - 8.0).max(0.0), _ => 0.0 };
                        for rect in multiline_highlight(&galley, &para.text, fs, fe) {
                            painter.rect_filled(rect.translate(egui::vec2(edit_x + align_offset, text_y)), 2.0, egui::Color32::from_rgba_unmultiplied(255, 210, 0, 90));
                        }
                    }
                }

                if let Some((from, to)) = active_sel {
                    if i >= from.para && i <= to.para {
                        let start_byte = if i == from.para { from.byte } else { 0 };
                        let end_byte = if i == to.para { to.byte } else { para.text.len() };
                        let galley = get_galley();
                        let align_offset = match para.align { Align::Center => ((edit_w - galley.rect.width() - 8.0) / 2.0).max(0.0), Align::Right => (edit_w - galley.rect.width() - 8.0).max(0.0), _ => 0.0 };
                        for rect in multiline_highlight(&galley, &para.text, start_byte, end_byte) {
                            painter.rect_filled(rect.translate(egui::vec2(edit_x + align_offset, text_y)), 0.0, sel_color);
                        }
                    }
                }
            }

            if i == focused {
                let id = ed.para_ids[i];
                let state = egui::TextEdit::load_state(ctx, id);
                let cr = state.as_ref().and_then(|s| s.cursor.char_range());
                let at_start = cr.map(|cr| cr.primary.index == 0 && cr.secondary.index == 0).unwrap_or(false);
                let at_end = cr.map(|cr| { let len = ed.para_texts[i].chars().count(); cr.primary.index == len && cr.secondary.index == len }).unwrap_or(false);
                let should_handle_bksp = at_start && (ed.paras[i].indent_first > 0.0 || ed.paras[i].indent_left > 0.0 || ed.paras[i].style != ParaStyle::Normal || i > 0);
                if should_handle_bksp && ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)) {
                    if ed.paras[i].indent_first > 0.0 { ed.push_undo(); ed.paras[i].indent_first = (ed.paras[i].indent_first - 36.0).max(0.0); ed.dirty = true; ed.heights_dirty = true; }
                    else if ed.paras[i].indent_left > 0.0 { ed.push_undo(); ed.paras[i].indent_left = (ed.paras[i].indent_left - 36.0).max(0.0); ed.dirty = true; ed.heights_dirty = true; }
                    else if ed.paras[i].style != ParaStyle::Normal { ed.push_undo(); ed.paras[i].style = ParaStyle::Normal; ed.dirty = true; ed.heights_dirty = true; }
                    else if i > 0 { merge_up = Some(i); }
                    continue;
                }
                if at_end && i + 1 < ed.paras.len() && ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) { merge_down = Some(i); continue; }
            }

            if ed.paras[i].style == ParaStyle::HRule { continue; }

            let para_clone = ed.paras[i].clone(); let spans_clone: Vec<DocSpan> = ed.paras[i].spans.clone();
            let para_clone_layouter = para_clone.clone(); let spans_clone_layouter = spans_clone.clone();
            let dark_snap = is_dark; let zoom_snap = ed.zoom;
            let mut layouter = move |lui: &egui::Ui, s: &dyn egui::TextBuffer, _ww: f32| {
                let job = build_layout_job(&spans_clone_layouter, s.as_str(), &para_clone_layouter, wrap_w, dark_snap, zoom_snap);
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
                            let job = build_layout_job(&spans_clone, &para_clone.text, &para_clone, wrap_w, dark_snap, zoom_snap);
                            let galley = ctx.fonts_mut(|f| f.layout_job(job));
                            let mut cur_x_local = 0.0; let mut is_top = false; let mut is_bottom = false; let mut char_pos = 0usize;
                            for (row_idx, row) in galley.rows.iter().enumerate() {
                                let row_start = char_pos; let glyph_count = row.glyphs.len(); let row_end = char_pos + glyph_count;
                                let is_last_row = row_idx == galley.rows.len() - 1;
                                let next_char_pos = row_end + if row.ends_with_newline { 1 } else { 0 };
                                if cr.primary.index >= row_start && (cr.primary.index < next_char_pos || is_last_row) {
                                    let local_index = cr.primary.index.saturating_sub(row_start);
                                    cur_x_local = if local_index == 0 { row.rect().min.x }
                                    else if local_index >= glyph_count { row.rect().max.x }
                                    else { row.glyphs.get(local_index).map(|g| g.pos.x).unwrap_or(row.rect().max.x) };
                                    is_top = row_idx == 0; is_bottom = is_last_row; break;
                                }
                                char_pos = next_char_pos;
                            }
                            let mut nav_to = None;
                            if up_pressed && is_top {
                                ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                                let mut t = i.saturating_sub(1); while t > 0 && ed.paras[t].style == ParaStyle::HRule { t -= 1; }
                                if ed.paras[t].style != ParaStyle::HRule { nav_to = Some((t, true)); }
                            } else if down_pressed && is_bottom {
                                ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
                                let mut t = i + 1; while t + 1 < ed.paras.len() && ed.paras[t].style == ParaStyle::HRule { t += 1; }
                                if t < ed.paras.len() && ed.paras[t].style != ParaStyle::HRule { nav_to = Some((t, false)); }
                            }
                            if let Some((target_i, to_bottom)) = nav_to {
                                let cur_x_abs = edit_x + cur_x_local; let target_para = &ed.paras[target_i];
                                if target_para.style == ParaStyle::Table {
                                    if let Some(ref tbl) = target_para.table {
                                        let next_r = if to_bottom { tbl.rows.len().saturating_sub(1) } else { 0 }; let next_c = 0;
                                        ed.table_text_sel = None;
                                        ed.active_table = Some((target_i, next_r, next_c));
                                        ed.cell_edit_buf = tbl.rows[next_r].get(next_c).map(|c| c.text.clone()).unwrap_or_default();
                                        let next_id = ui.id().with(("table_cell", target_i, next_r, next_c));
                                        ctx.memory_mut(|m| m.request_focus(next_id));
                                        let mut target_state = egui::TextEdit::load_state(ctx, next_id).unwrap_or_default();
                                        let new_idx = if to_bottom { ed.cell_edit_buf.chars().count() } else { 0 };
                                        target_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(new_idx))));
                                        egui::TextEdit::store_state(ctx, next_id, target_state);
                                        ed.focused_para = target_i; pending_focus_next = Some(target_i);
                                    }
                                } else {
                                    let target_wrap_w = (cw - target_para.indent_left * ed.zoom).max(40.0);
                                    let target_job = build_layout_job(&target_para.spans, &target_para.text, target_para, target_wrap_w, is_dark, ed.zoom);
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
            }
            if ed.pending_focus == Some(i) && ed.active_table.is_none() && !ed.toolbar_has_focus { ctx.memory_mut(|m| m.request_focus(id)); }
            let tab_keys = if i == focused {
                ctx.input_mut(|inp| {
                    let shift_tab = inp.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
                    let ctrl_shft_m = inp.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::M);
                    let plain_tab = inp.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
                    let ctrl_m = inp.consume_key(egui::Modifiers::CTRL, egui::Key::M);
                    (plain_tab, shift_tab, ctrl_m, ctrl_shft_m)
                })
            } else { (false, false, false, false) };
            let text_ref = &mut ed.para_texts[i];
            let effective_rect = if i + 1 < n && ed.paras[i + 1].style == ParaStyle::Table {
                let tpg = pl.para_page[i + 1];
                let table_top = outer.min.y + pl.page_tops[tpg] + pl.para_content_y[i + 1] * ed.zoom;
                egui::Rect::from_min_max(edit_rect.min, egui::pos2(edit_rect.max.x, table_top.min(edit_rect.max.y)))
            } else { edit_rect };
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(effective_rect));
            let output = egui::TextEdit::multiline(text_ref).id(id).desired_width(edit_w).desired_rows(1).frame(false).lock_focus(true).horizontal_align(para.align.egui_align()).layouter(&mut layouter).show(&mut child);
            output.response.context_menu(|ui| { cp.set(Some(i)); text_cm(ui, has_text_sel, sel_has_link, is_dark, ca); });
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
            if has_focus && ed.focused_para != i && ed.active_table.is_none() { ed.focused_para = i; ed.line_spacing_input = ed.paras[i].line_height; ed.last_edit_action = 0; }
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
                if h > 0 { let style = match h { 1 => ParaStyle::H1, 2 => ParaStyle::H2, 3 => ParaStyle::H3, 4 => ParaStyle::H4, _ => ParaStyle::H5 }; ed.apply_style_toggle(style); }
                if tab_keys.0 || tab_keys.1 || tab_keys.2 || tab_keys.3 {
                    let shift_tab = tab_keys.1 || tab_keys.3;
                    let delta = if shift_tab { -36.0f32 } else { 36.0f32 };
                    let max_indent = (ed.layout.content_width() - 36.0).max(0.0);
                    let state = egui::TextEdit::load_state(ctx, id);
                    let cr = state.as_ref().and_then(|s| s.cursor.char_range());
                    let has_selection = cr.map(|cr| cr.primary.index != cr.secondary.index).unwrap_or(false);
                    let caret_at_start = cr.map(|cr| cr.primary.index == 0 && cr.secondary.index == 0).unwrap_or(false);
                    let changed: bool;
                    ed.push_undo();

                    if has_selection {
                        let before = ed.paras[i].indent_left;
                        ed.paras[i].indent_left = (ed.paras[i].indent_left + delta).clamp(0.0, max_indent);
                        changed = (ed.paras[i].indent_left - before).abs() > f32::EPSILON;
                    } else if caret_at_start && !ed.paras[i].text.is_empty() {
                        if shift_tab {
                            if ed.paras[i].indent_first > 0.0 {
                                let before = ed.paras[i].indent_first; ed.paras[i].indent_first = (ed.paras[i].indent_first + delta).max(0.0);
                                changed = (ed.paras[i].indent_first - before).abs() > f32::EPSILON;
                            } else {
                                let before = ed.paras[i].indent_left; ed.paras[i].indent_left = (ed.paras[i].indent_left + delta).max(0.0);
                                changed = (ed.paras[i].indent_left - before).abs() > f32::EPSILON;
                            }
                        } else {
                            let before = ed.paras[i].indent_first; ed.paras[i].indent_first = (ed.paras[i].indent_first + delta).clamp(0.0, max_indent);
                            changed = (ed.paras[i].indent_first - before).abs() > f32::EPSILON;
                        }
                    } else {
                        let before = ed.paras[i].indent_left; ed.paras[i].indent_left = (ed.paras[i].indent_left + delta).clamp(0.0, max_indent);
                        changed = (ed.paras[i].indent_left - before).abs() > f32::EPSILON;
                    }

                    if !changed { ed.undo_stack.pop_back(); }
                    else { ed.dirty = true; ed.heights_dirty = true; ed.find_stale = true; ed.spell_dirty = true; }
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
                            c.size_hp = fmt.size_hp; c.font = fmt.font; c.color = fmt.color;
                            c.highlight = fmt.highlight; c.link = fmt.link;
                            if !ed.has_cross_sel() { ed.doc_sel = None; }
                        }
                    }
                }
            }

            if ed.pending_focus == Some(i) { pending_focus_next = Some(i); ed.pending_focus = None; }
            if output.response.changed() { text_change = Some((i, ed.para_texts[i].clone())); }
            let new_h = output.galley.size().y;
            if (new_h - text_h).abs() > 0.5 { ed.heights_dirty = true; }
        }

        if let Some((_, 255, _, _, _, _)) = ed.image_drag {
            if let Some(pp) = ptr.filter(|p| p.y >= canvas_top) {
                let di = compute_drop_idx(ed, &pl, outer.min.y, pp.y);
                drop_idx_cell.set(Some(di));
                let gy = drop_line_y(ed, &pl, outer.min.y, di);
                let lx = outer.min.x + page_x;
                painter.hline(lx..=(lx + page_w), gy, egui::Stroke::new(2.5, ColorPalette::BLUE_400));
                painter.circle_filled(egui::pos2(lx, gy), 5.0, ColorPalette::BLUE_400);
                painter.circle_filled(egui::pos2(lx + page_w, gy), 5.0, ColorPalette::BLUE_400);
            }
        }
    });
    if let Some(fp) = cm_para.get() { ed.focused_para = fp; }
    if let Some(pi) = cm_img_para.get() {
        ed.selected_image_para = Some(pi);
        ed.focused_para = pi;
        ed.doc_sel = None;
    }
    if let Some(action) = ctx_action.borrow_mut().take() { process_ctx_action(ed, ctx, action); }
    if !ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary)) {
        if let Some(drag) = ed.image_drag.take() {
            if drag.1 == 255 {
                if let Some(di) = drop_idx_cell.get() {
                    let from = drag.0;
                    let insert_at = (if di <= from { di } else { di.saturating_sub(1) }).min(ed.paras.len());
                    if from != insert_at {
                        ed.push_undo();
                        let para = ed.paras.remove(from);
                        ed.paras.insert(insert_at.min(ed.paras.len()), para);
                        let new_idx = insert_at.min(ed.paras.len().saturating_sub(1));
                        ed.selected_image_para = Some(new_idx); ed.focused_para = new_idx;
                        ed.sync_texts(); ed.dirty = true; ed.heights_dirty = true;
                    }
                }
            } else {
                let (nw, nh) = image_drag_dims(ed, ctx, drag);
                img_size_change = Some((drag.0, nw, nh));
            }
        }
    }

    if !is_drag_active { ed.doc_scroll_y = scroll_out.state.offset.y; }
    if let Some(f) = pending_focus_next { ed.focused_para = f.min(ed.paras.len().saturating_sub(1)); }
    if let Some(sel) = new_selection { ed.last_selection = Some(sel); }
    if let Some((pi, nw, nh)) = img_size_change { if let Some(ref mut img) = ed.paras[pi].image { img.display_w = nw; img.display_h = nh; } ed.heights_dirty = true; ed.dirty = true; }
    if let Some((pi, row, col, text)) = table_cell_change {
        if pi < ed.paras.len() {
            ed.push_undo();
            if let Some(ref mut tbl) = ed.paras[pi].table {
                if let Some(r) = tbl.rows.get_mut(row) {
                    if let Some(c) = r.get_mut(col) {
                        if c.text != text {
                            c.text = text.clone();
                            if c.spans.is_empty() {
                                c.spans = if text.is_empty() { vec![DocSpan { len: 0, fmt: SpanFmt::default() }] } else { vec![DocSpan { len: text.len(), fmt: SpanFmt::default() }] };
                            } else if c.spans.iter().any(|s| s.fmt != SpanFmt::default()) {
                                let fmt = c.spans.iter().find(|s| s.len > 0).map(|s| s.fmt.clone()).unwrap_or_default();
                                c.spans = if text.is_empty() { vec![DocSpan { len: 0, fmt }] } else { vec![DocSpan { len: text.len(), fmt }] };
                            } else {
                                c.spans = if text.is_empty() { vec![DocSpan { len: 0, fmt: SpanFmt::default() }] } else { vec![DocSpan { len: text.len(), fmt: SpanFmt::default() }] };
                            }
                        }
                    }
                }
            }
            ed.dirty = true; ed.heights_dirty = true;
        }
    }
    if let Some((pi, col, delta)) = table_col_resize {
        if pi < ed.paras.len() {
            if let Some(ref mut tbl) = ed.paras[pi].table {
                let nc = tbl.rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
                if tbl.col_widths.len() != nc { tbl.col_widths = vec![1.0 / nc as f32; nc]; }
                if col < nc {
                    let df = (delta / (ed.layout.content_width() * ed.zoom)).clamp(0.05 - tbl.col_widths[col - 1], tbl.col_widths[col] - 0.05);
                    tbl.col_widths[col - 1] += df; tbl.col_widths[col] -= df;
                    ed.heights_dirty = true; ed.dirty = true;
                }
            }
        }
    }

    if let Some((op, pi, row, col)) = tbl_struct_op.get() {
        ed.push_undo();
        match op {
            0 => ed.insert_table_row(pi, row, true), 1 => ed.insert_table_row(pi, row, false),
            2 => ed.insert_table_col(pi, col, true), 3 => ed.insert_table_col(pi, col, false),
            4 => ed.delete_table_row(pi, row),
            5 => ed.delete_table_col(pi, col),
            6 => { if pi < ed.paras.len() { ed.paras.remove(pi); ed.focused_para = pi.min(ed.paras.len().saturating_sub(1)); ed.active_table = None; ed.table_sel = None; } }
            _ => {}
        }
        ed.table_multi_sel = None; ed.sync_texts(); ed.find_stale = true;
    }
    if let Some((pi, row, col, color)) = tbl_cell_bg_op.get() {
        if pi < ed.paras.len() {
            ed.push_undo();
            let mut targets: Vec<(usize, usize)> = match ed.table_sel {
                Some((sp, (ar, ac), (br, bc))) if sp == pi => {
                    let (r0, r1) = (ar.min(br), ar.max(br)); let (c0, c1) = (ac.min(bc), ac.max(bc));
                    (r0..=r1).flat_map(|r| (c0..=c1).map(move |c| (r, c))).collect()
                }
                _ => Vec::new(),
            };
            if let Some((sp, ref cells)) = ed.table_multi_sel { if sp == pi { targets.extend_from_slice(cells); } }
            if targets.is_empty() { targets.push((row, col)); }
            if let Some(ref mut tbl) = ed.paras[pi].table {
                for (r, c) in targets { if let Some(rw) = tbl.rows.get_mut(r) { if let Some(cl) = rw.get_mut(c) { cl.bg_color = color; } } }
            }
            ed.dirty = true; ed.heights_dirty = true;
        }
    }
    if let Some((pi, color, width)) = tbl_border_op.get() {
        if pi < ed.paras.len() { ed.push_undo(); if let Some(ref mut tbl) = ed.paras[pi].table { tbl.border_color = color; tbl.border_width = width; } ed.dirty = true; ed.heights_dirty = true; }
    }

    if let Some(mu) = merge_up {
        if mu > 0 && mu < ed.paras.len() {
            ed.push_undo();
            if ed.paras[mu - 1].style == ParaStyle::HRule {
                ed.paras.remove(mu - 1); ed.focused_para = mu - 1; ed.pending_focus = Some(mu - 1);
                ed.sync_texts(); ed.dirty = true; ed.heights_dirty = true;
            } else if ed.paras[mu - 1].style == ParaStyle::Table || ed.paras[mu].style == ParaStyle::Table {
                ed.undo_stack.pop_back();
            } else {
                let prev_len = ed.paras[mu - 1].text.len();
                merge_paragraphs(&mut ed.paras, mu - 1); ed.focused_para = mu - 1; ed.pending_focus = Some(mu - 1); ed.sync_texts();
                let id = ed.para_ids[mu - 1]; let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(prev_len)))); egui::TextEdit::store_state(ctx, id, state);
                ed.dirty = true; ed.heights_dirty = true;
            }
        }
    }
    if let Some(md) = merge_down {
        if md + 1 < ed.paras.len() {
            ed.push_undo();
            if ed.paras[md + 1].style == ParaStyle::HRule {
                ed.paras.remove(md + 1); ed.sync_texts(); ed.dirty = true; ed.heights_dirty = true;
            } else if ed.paras[md + 1].style == ParaStyle::Table || ed.paras[md].style == ParaStyle::Table {
                ed.undo_stack.pop_back();
            } else {
                let prev_len = ed.paras[md].text.len();
                merge_paragraphs(&mut ed.paras, md); ed.sync_texts();
                let id = ed.para_ids[md]; let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(prev_len)))); egui::TextEdit::store_state(ctx, id, state);
                ed.dirty = true; ed.heights_dirty = true;
            }
        }
    }

    if let Some((i, new_text)) = text_change {
        if i < ed.paras.len() {
            let lns: Vec<&str> = new_text.split('\n').collect();
            if lns.len() > 1 {
                ed.push_undo();
                let ns = if ed.paras[i].style.is_heading() { ParaStyle::Normal } else { ed.paras[i].style };
                let (al, lh, il) = (ed.paras[i].align, ed.paras[i].line_height, ed.paras[i].indent_left);
                rebuild_spans(&mut ed.paras[i], lns[0].to_string(), &cur_fmt);
                let mut ins = i + 1;
                for &ln in &lns[1..] {
                    let mut np = DocParagraph::with_style(ns);
                    np.text = ln.to_string(); np.spans = vec![DocSpan { len: ln.len(), fmt: cur_fmt.clone() }];
                    np.align = al; np.line_height = lh; np.indent_left = il;
                    ed.paras.insert(ins, np); ins += 1;
                }
                ed.focused_para = ins - 1; ed.pending_focus = Some(ins - 1);
                ed.sync_texts();
                let last_len = lns.last().unwrap_or(&"").chars().count();
                let new_id = ed.para_ids[ins - 1]; let mut state = egui::TextEdit::load_state(ctx, new_id).unwrap_or_default();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(last_len)))); egui::TextEdit::store_state(ctx, new_id, state);
            } else {
                let old_text = &ed.paras[i].text; let diff = new_text.len() as isize - old_text.len() as isize;
                let mut should_push = false; let new_action: u8;
                if diff > 1 { should_push = true; new_action = 0; }
                else if diff < 0 { if ed.last_edit_action != 2 { should_push = true; } new_action = 2; }
                else if diff == 1 {
                    let is_space = new_text.ends_with(|c: char| c.is_whitespace() || c.is_ascii_punctuation());
                    let was_space = old_text.ends_with(|c: char| c.is_whitespace() || c.is_ascii_punctuation());
                    if (was_space && !is_space) || ed.last_edit_action != 1 { should_push = true; }
                    new_action = 1;
                } else { should_push = true; new_action = 0; }
                if should_push { ed.push_undo(); }
                ed.last_edit_action = new_action;
                rebuild_spans(&mut ed.paras[i], new_text, &cur_fmt);
                ed.para_texts[i] = ed.paras[i].text.clone();
            }
            ed.dirty = true; ed.heights_dirty = true; ed.find_stale = true; ed.spell_dirty = true;
        }
    }
}

fn render_find_bar(ed: &mut DocumentEditor, ctx: &egui::Context, is_dark: bool) {
    if !ed.show_find { return; }
    let (panel_bg, border, text_col, muted) = if is_dark { (egui::Color32::from_rgb(26, 26, 34), ColorPalette::ZINC_600, ColorPalette::ZINC_100, ColorPalette::ZINC_500) }
    else { (egui::Color32::WHITE, ColorPalette::GRAY_200, ColorPalette::GRAY_900, ColorPalette::GRAY_500) };
    let field_bg = if is_dark { egui::Color32::from_rgb(36, 36, 46) } else { ColorPalette::GRAY_50 };
    let field_bdr = if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_300 };
    let hover = if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_100 };
    let act_bg = if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_100 };
    let n = ed.find_results.len(); let no_match = !ed.find_text.is_empty() && n == 0;
    let has = n > 0; let can_replace = has && !ed.find_text.is_empty();
    let mut action: u8 = 0;

    let ghost = |ui: &mut egui::Ui, label: &str, col: egui::Color32, enabled: bool| -> bool {
        ui.scope(|ui| {
            let s = ui.style_mut(); s.visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT; s.visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
            s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE; s.visuals.widgets.hovered.bg_fill = hover; s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            s.visuals.widgets.active.bg_fill = hover;
            ui.add_enabled(enabled, egui::Button::new(egui::RichText::new(label).size(11.5).color(col)).min_size(egui::vec2(26.0, 28.0)).corner_radius(4.0))
        }).inner.clicked()
    };

    let win = egui::Window::new("##de_find")
        .title_bar(false).collapsible(false).resizable(false).anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 44.0)).fixed_size(egui::vec2(368.0, 0.0))
        .frame(egui::Frame::new().fill(panel_bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(8.0).inner_margin(egui::Margin { left: 12, right: 10, top: 10, bottom: 12 }))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 6.0);
            ui.horizontal(|ui| {
                let find_r = egui::Frame::new().fill(field_bg).stroke(egui::Stroke::new(1.0, if no_match { ColorPalette::RED_500 } else { field_bdr })).corner_radius(4.0).inner_margin(egui::Margin { left: 8, right: 6, top: 3, bottom: 3 })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let r = ui.add(egui::TextEdit::singleline(&mut ed.find_text).id_salt("find_input").desired_width(168.0).frame(false).hint_text("Find in document").font(egui::FontId::proportional(12.5)));
                            if ed.focus_find { r.request_focus(); ed.focus_find = false; }
                            if !ed.find_text.is_empty() {
                                let col = if no_match { ColorPalette::RED_400 } else { muted };
                                let s = if has { format!("{}/{}", ed.find_cursor + 1, n) } else { "0/0".to_string() };
                                ui.label(egui::RichText::new(s).size(10.5).color(col));
                            }
                            r
                        }).inner
                    }).inner;
                if find_r.changed() { ed.find_stale = true; ed.run_find(); }
                if find_r.has_focus() { ctx.input_mut(|i| { if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) { action = 2; } if i.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter) { action = 1; } }); }
                let nav = if has { text_col } else { muted };
                if ghost(ui, "^", nav, has) { action = 1; }
                if ghost(ui, "v", nav, has) { action = 2; }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ghost(ui, "X", muted, true) { action = 5; } });
            });

            let (sr, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter().rect_filled(sr, 0.0, border);

            ui.horizontal(|ui| {
                egui::Frame::new().fill(field_bg).stroke(egui::Stroke::new(1.0, field_bdr)).corner_radius(4.0).inner_margin(egui::Margin { left: 8, right: 6, top: 3, bottom: 3 })
                    .show(ui, |ui| { ui.add(egui::TextEdit::singleline(&mut ed.replace_text).desired_width(168.0).frame(false).hint_text("Replace with").font(egui::FontId::proportional(12.5))); });
                let tc = if can_replace { text_col } else { muted }; let bg = if can_replace { act_bg } else { egui::Color32::TRANSPARENT };
                for (label, id) in [("Replace", 3u8), ("All", 4u8)] {
                    let clicked = ui.scope(|ui| {
                        let s = ui.style_mut(); s.visuals.widgets.inactive.bg_fill = bg; s.visuals.widgets.inactive.weak_bg_fill = bg;
                        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE; s.visuals.widgets.hovered.bg_fill = hover; s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE; s.visuals.widgets.active.bg_fill = hover;
                        ui.add_enabled(can_replace, egui::Button::new(egui::RichText::new(label).size(11.5).color(tc)).min_size(egui::vec2(0.0, 28.0)).corner_radius(4.0))
                    }).inner.clicked();
                    if clicked { action = id; }
                }
            });
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) { action = 5; }
        });

    match action { 1 => ed.find_prev(), 2 => ed.find_next(), 3 => ed.replace_current(), 4 => ed.replace_all(), 5 => ed.show_find = false, _ => {} }
    if let Some(w) = win { if ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().map_or(false, |p| !w.response.rect.contains(p))) { ed.show_find = false; } }
}

fn render_stats_modal(ed: &mut DocumentEditor, ctx: &egui::Context, is_dark: bool) {
    if !ed.show_stats { return; }
    crate::style::draw_modal_overlay(ctx, "de_stats_ov", 160);
    let (bg, border, tc, muted) = if is_dark { (ColorPalette::ZINC_900, ColorPalette::ZINC_700, ColorPalette::SLATE_200, ColorPalette::ZINC_400) } else { (egui::Color32::WHITE, ColorPalette::GRAY_200, ColorPalette::GRAY_800, ColorPalette::GRAY_500) };
    let mut open = ed.show_stats;
    let win = egui::Window::new("Document Statistics")
        .collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(24.0)).open(&mut open).order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let row = |ui: &mut egui::Ui, lbl: &str, val: String| { ui.horizontal(|ui| { ui.label(egui::RichText::new(lbl).size(13.0).color(muted)); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(egui::RichText::new(val).size(13.0).color(tc)); }); }); };
            row(ui, "Words", word_count(&ed.paras).to_string()); row(ui, "Characters", char_count(&ed.paras).to_string()); row(ui, "Paragraphs", ed.paras.len().to_string());
            row(ui, "Headings", ed.paras.iter().filter(|p| p.style.is_heading()).count().to_string()); row(ui, "Page size", format!("{:.0} x {:.0} pt", ed.layout.width, ed.layout.height));
            ui.add_space(8.0); if ui.button("Close").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { ed.show_stats = false; }
        });
    if !open { ed.show_stats = false; }
    if let Some(win) = win { let clicked_outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().map_or(false, |p| !win.response.rect.contains(p))); if clicked_outside { ed.show_stats = false; } }
}

fn render_ctx_link_modal(ed: &mut DocumentEditor, ctx: &egui::Context, is_dark: bool) {
    if !ed.ctx_link_show { return; }
    crate::style::draw_modal_overlay(ctx, "ctx_link_ov", 120);
    let (bg, border, tc) = if is_dark { (ColorPalette::ZINC_900, ColorPalette::ZINC_700, ColorPalette::SLATE_200) } else { (egui::Color32::WHITE, ColorPalette::GRAY_200, ColorPalette::GRAY_800) };
    let muted = if is_dark { ColorPalette::ZINC_400 } else { ColorPalette::GRAY_500 };
    let (mut apply, remove, mut cancel) = (false, false, false);
    egui::Window::new("##ctx_link_win").title_bar(false).collapsible(false).resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .fixed_size(egui::vec2(280.0, 0.0))
        .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(8.0).inner_margin(16.0))
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("Link URL").size(13.0).color(tc));
            ui.add_space(4.0);
            let r = ui.add(egui::TextEdit::singleline(&mut ed.link_input).desired_width(248.0).hint_text("https://..."));
            r.request_focus();
            if r.has_focus() { ctx.input_mut(|i| { if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) { apply = true; } if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) { cancel = true; } }); }
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Leave empty to remove the link.").size(11.0).color(muted));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Apply").clicked() { apply = true; }
                if ui.button("Cancel").clicked() { cancel = true; }
            });
        });
    if apply {
        let url = ed.link_input.trim().to_string();
        ed.apply_fmt_link(if url.is_empty() { None } else { Some(url) });
        ed.ctx_link_show = false; ed.ctx_sel = None;
    } else if remove || cancel {
        if remove { ed.apply_fmt_link(None); }
        ed.ctx_link_show = false; ed.ctx_sel = None;
    }
}

fn render_page_settings(ed: &mut DocumentEditor, ctx: &egui::Context, is_dark: bool) {
    if !ed.show_page_settings { return; }
    if ed.page_settings_draft.is_none() { let l = ed.layout.clone(); ed.page_settings_draft = Some((l.clone(), l.preset_idx(), format!("{:.2}", l.width / PageLayout::PTS_PER_INCH), format!("{:.2}", l.height / PageLayout::PTS_PER_INCH), String::new())); }
    crate::style::draw_modal_overlay(ctx, "de_page_ov", 160);
    let (bg, border, tc, muted) = if is_dark { (ColorPalette::ZINC_900, ColorPalette::ZINC_700, ColorPalette::SLATE_200, ColorPalette::ZINC_400) } else { (egui::Color32::WHITE, ColorPalette::GRAY_200, ColorPalette::GRAY_800, ColorPalette::GRAY_500) };
    let sep_col = if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_200 };
    let tag_bg_gd = if is_dark { egui::Color32::from_rgb(24,44,80) } else { ColorPalette::BLUE_50 };
    let tag_bg_wd = if is_dark { egui::Color32::from_rgb(30,40,26) } else { ColorPalette::GREEN_50 };
    let tag_col_gd = if is_dark { ColorPalette::BLUE_300 } else { ColorPalette::BLUE_600 };
    let tag_col_wd = if is_dark { ColorPalette::GREEN_400 } else { ColorPalette::GREEN_700 };
    let mut apply = false; let mut cancel = false; let mut open = true;

    let page_win = {
        let draft = ed.page_settings_draft.as_mut().unwrap();
        egui::Window::new("Page Setup").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).fixed_size(egui::vec2(420.0, 0.0))
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(24.0)).open(&mut open).order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0; ui.label(egui::RichText::new("Paper Size").size(13.0).strong().color(tc)); ui.add_space(4.0);
                let mut presets: &[(&str, &str)] = PageLayout::presets(); presets = &presets[..presets.len() - 1];
                let sizes = ["8.5 x 11 in", "8.5 x 11 in", "210 x 297 mm", "8.5 x 14 in", "297 x 420 mm", "148 x 210 mm", "7.25 x 10.5 in", "11 x 17 in", "176 x 250 mm"];
                let cols = 3usize; let btn_w = (ui.available_width() - (cols as f32 - 1.0) * 6.0) / cols as f32;
                for row in 0..((presets.len() + cols - 1) / cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for col in 0..cols {
                            let idx = row * cols + col; let Some((name, _)) = presets.get(idx) else { continue; }; let sel = draft.1 == idx; let size_str = sizes.get(idx).copied().unwrap_or("");
                            let (fill, stroke, label_col) = if sel { (if is_dark { egui::Color32::from_rgb(28,52,100) } else { egui::Color32::from_rgb(225,238,255) }, egui::Stroke::new(1.5, if is_dark { ColorPalette::BLUE_500 } else { ColorPalette::BLUE_400 }), if is_dark { egui::Color32::WHITE } else { ColorPalette::BLUE_700 }) } else { (if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_100 }, egui::Stroke::new(1.0, if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 }), tc) };
                            let size_col = if sel { egui::Color32::from_rgba_unmultiplied(label_col.r(), label_col.g(), label_col.b(), 160) } else { muted };
                            let (tag_bg, tag_col, badge) = match idx { 0 => (tag_bg_gd, tag_col_gd, Some("Docs")), 1 => (tag_bg_wd, tag_col_wd, Some("Word")), _ => (egui::Color32::TRANSPARENT, egui::Color32::TRANSPARENT, None) };
                            let (cell, _) = ui.allocate_exact_size(egui::vec2(btn_w, 56.0), egui::Sense::hover());
                            if ui.is_rect_visible(cell) {
                                let painter = ui.painter_at(cell); painter.rect(cell, 6.0, fill, stroke, egui::StrokeKind::Inside);
                                let name_y = if badge.is_some() { cell.min.y + 15.0 } else { cell.center().y - 8.0 };
                                painter.text(egui::pos2(cell.center().x, name_y), egui::Align2::CENTER_CENTER, *name, egui::FontId::proportional(12.0), label_col);
                                painter.text(egui::pos2(cell.center().x, name_y + 14.0), egui::Align2::CENTER_CENTER, size_str, egui::FontId::proportional(9.5), size_col);
                                if let Some(b) = badge {
                                    let bw = 34.0_f32; let br = egui::Rect::from_min_size(egui::pos2(cell.center().x - bw / 2.0, cell.max.y - 13.0), egui::vec2(bw, 11.0));
                                    painter.rect_filled(br, 3.0, tag_bg); painter.text(br.center(), egui::Align2::CENTER_CENTER, b, egui::FontId::proportional(8.0), tag_col);
                                }
                            }
                            let resp = ui.interact(cell, ui.id().with(("ps", idx)), egui::Sense::click());
                            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { draft.1 = idx; draft.0 = PageLayout::from_preset(idx); draft.4.clear(); }
                        }
                    });
                }
                let custom_sel = draft.1 == PageLayout::CUSTOM;
                let (fill, stroke, label_col) = if custom_sel { (if is_dark { egui::Color32::from_rgb(28,52,100) } else { egui::Color32::from_rgb(225,238,255) }, egui::Stroke::new(1.5, if is_dark { ColorPalette::BLUE_500 } else { ColorPalette::BLUE_400 }), if is_dark { egui::Color32::WHITE } else { ColorPalette::BLUE_700 }) } else { (if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_100 }, egui::Stroke::new(1.0, if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 }), tc) };
                let size_col = if custom_sel { egui::Color32::from_rgba_unmultiplied(label_col.r(), label_col.g(), label_col.b(), 160) } else { muted };
                let (cell, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 68.0), egui::Sense::hover());
                if ui.is_rect_visible(cell) {
                    let painter = ui.painter_at(cell); painter.rect(cell, 8.0, fill, stroke, egui::StrokeKind::Inside);
                    painter.text(egui::pos2(cell.center().x, cell.min.y + 20.0), egui::Align2::CENTER_CENTER, "Custom", egui::FontId::proportional(13.0), label_col);
                    painter.text(egui::pos2(cell.center().x, cell.min.y + 36.0), egui::Align2::CENTER_CENTER, "Set width, height, and margins", egui::FontId::proportional(9.5), size_col);
                    painter.text(egui::pos2(cell.center().x, cell.min.y + 50.0), egui::Align2::CENTER_CENTER, "For any paper size", egui::FontId::proportional(9.5), size_col);
                }
                let resp = ui.interact(cell, ui.id().with("ps_custom"), egui::Sense::click());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    draft.1 = PageLayout::CUSTOM;
                    if draft.2.is_empty() || draft.3.is_empty() { let p = PageLayout::PTS_PER_INCH; draft.2 = format!("{:.2}", draft.0.width / p); draft.3 = format!("{:.2}", draft.0.height / p); }
                    draft.4.clear();
                }

                if draft.1 == PageLayout::CUSTOM {
                    ui.add_space(6.0); ui.separator(); ui.add_space(8.0); ui.label(egui::RichText::new("Custom Size (inches)").size(13.0).strong().color(tc));
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Width").size(12.0).color(muted)); ui.add(egui::TextEdit::singleline(&mut draft.2).desired_width(70.0).hint_text("8.5"));
                        ui.label(egui::RichText::new("Height").size(12.0).color(muted)); ui.add(egui::TextEdit::singleline(&mut draft.3).desired_width(70.0).hint_text("11.0"));
                    });
                    if !draft.4.is_empty() { ui.label(egui::RichText::new(&draft.4).size(11.0).color(ColorPalette::RED_400)); }
                }
                ui.add_space(6.0); let sep = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 1.0)); ui.allocate_rect(sep, egui::Sense::hover()); ui.painter().rect_filled(sep, 0.0, sep_col); ui.add_space(8.0);
                ui.label(egui::RichText::new("Margins (inches)").size(13.0).strong().color(tc)); ui.add_space(2.0); let p = PageLayout::PTS_PER_INCH;
                ui.columns(2, |cols| {
                    for (label, pts) in [("Top", &mut draft.0.margin_top), ("Bottom", &mut draft.0.margin_bot)] {
                        cols[0].horizontal(|ui| {
                            ui.add_sized(egui::vec2(52.0, 18.0), egui::Label::new(egui::RichText::new(label).size(12.0).color(muted)));
                            let mut inches = *pts / p; if ui.add(egui::DragValue::new(&mut inches).range(0.0..=4.0).speed(0.01).fixed_decimals(2).suffix(" in")).changed() { *pts = inches * p; }
                        });
                    }
                    for (label, pts) in [("Left", &mut draft.0.margin_left), ("Right", &mut draft.0.margin_right)] {
                        cols[1].horizontal(|ui| {
                            ui.add_sized(egui::vec2(52.0, 18.0), egui::Label::new(egui::RichText::new(label).size(12.0).color(muted)));
                            let mut inches = *pts / p; if ui.add(egui::DragValue::new(&mut inches).range(0.0..=4.0).speed(0.01).fixed_decimals(2).suffix(" in")).changed() { *pts = inches * p; }
                        });
                    }
                });
                ui.add_space(6.0); let sep2 = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 1.0)); ui.allocate_rect(sep2, egui::Sense::hover()); ui.painter().rect_filled(sep2, 0.0, sep_col); ui.add_space(8.0);
                ui.horizontal(|ui| { if ui.button("Apply").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { apply = true; } if ui.button("Cancel").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { cancel = true; } });
            })
    };

    if let Some(r) = &page_win { let clicked_outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().map_or(false, |p| !r.response.rect.contains(p))); if clicked_outside { cancel = true; } }

    if apply {
        let (layout, preset, w, h, _) = ed.page_settings_draft.as_ref().unwrap().clone();
        let p = PageLayout::PTS_PER_INCH;
        let layout = if preset == PageLayout::CUSTOM {
            let w = match w.trim().parse::<f32>() { Ok(v) => v, Err(_) => { ed.page_settings_draft.as_mut().unwrap().4 = "Width must be a number.".into(); return; } };
            let h = match h.trim().parse::<f32>() { Ok(v) => v, Err(_) => { ed.page_settings_draft.as_mut().unwrap().4 = "Height must be a number.".into(); return; } };
            match PageLayout::custom_in(w, h, layout.margin_top / p, layout.margin_bot / p, layout.margin_left / p, layout.margin_right / p) { Ok(v) => v, Err(e) => { ed.page_settings_draft.as_mut().unwrap().4 = e.into(); return; } }
        } else { match PageLayout::custom_in(layout.width / p, layout.height / p, layout.margin_top / p, layout.margin_bot / p, layout.margin_left / p, layout.margin_right / p) { Ok(v) => v, Err(e) => { ed.page_settings_draft.as_mut().unwrap().4 = e.into(); return; } } };
        ed.layout = layout; ed.preset_idx = preset; ed.heights_dirty = true; ed.auto_zoom_done = false; ed.dirty = true;
        ed.page_settings_draft = None; ed.show_page_settings = false;
    } else if cancel || !open { ed.page_settings_draft = None; ed.show_page_settings = false; }
}
