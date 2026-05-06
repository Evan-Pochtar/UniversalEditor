use eframe::egui;
use crate::style::{ColorPalette, ThemeMode, toolbar_action_btn, toolbar_toggle_btn};
use super::de_main::{DocumentEditor, DocPos};
use crate::modules::EditorModule;
use super::de_tools::*;

const PAGE_GAP: f32 = 28.0;
const PAGE_PAD: f32 = 24.0;

fn multiline_highlight(galley: &egui::Galley, text: &str, start_byte: usize, end_byte: usize) -> Vec<egui::Rect> {
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
        if row.ends_with_newline {
            char_pos += 1;
        }
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
                if ui.add(egui::Button::new("").fill(col)
                    .stroke(egui::Stroke::new(1.0, border))
                    .min_size(egui::vec2(20.0, 20.0))
                    .corner_radius(3.0))
                    .on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                {
                    ed.apply_fmt_color(Some(c));
                    egui::Popup::close_id(ui.ctx(), popup_id);
                }
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
                if ui.add(egui::Button::new("").fill(col)
                    .stroke(egui::Stroke::new(1.0, border))
                    .min_size(egui::vec2(26.0, 22.0))
                    .corner_radius(3.0))
                    .on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                {
                    ed.apply_fmt_highlight(Some(c));
                    egui::Popup::close_id(ui.ctx(), popup_id);
                }
            }
        });
    }
}

fn link_at_byte<'a>(para: &'a DocParagraph, byte: usize) -> Option<&'a str> {
    let mut pos = 0usize;
    for span in &para.spans {
        let end = pos + span.len;
        if byte >= pos && byte < end {
            return span.fmt.link.as_deref();
        }
        pos = end;
    }
    None
}

fn table_col_widths(tbl: &TableData, cw: f32) -> Vec<f32> {
    let nc = tbl.rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
    if tbl.col_widths.len() == nc { tbl.col_widths.iter().map(|&f| f * cw).collect() }
    else { vec![cw / nc as f32; nc] }
}

fn table_row_h(row: &[TableCell], col_ws: &[f32], font: FontChoice, base_size: u32, zoom: f32, ctx: &egui::Context, live_cell: Option<(usize, &str)>, is_header: bool) -> f32 {
    let min_h = base_size as f32 * zoom * 1.6;
    row.iter().enumerate().fold(min_h, |acc, (ci, cell)| {
        let text = if let Some((live_ci, live_text)) = live_cell {
            if live_ci == ci { live_text } else { cell.text.as_str() }
        } else {
            cell.text.as_str()
        };
        if text.is_empty() { return acc; }
        let w = col_ws.get(ci).copied().unwrap_or(min_h) - 16.0 * zoom;
        let job = egui::text::LayoutJob::simple(text.to_owned(), egui::FontId::new(base_size as f32 * zoom * 0.9, font.egui_family(is_header, false)), egui::Color32::WHITE, w.max(1.0));
        let mut galley_h = ctx.fonts_mut(|f| f.layout_job(job)).rect.height();
        if text.ends_with('\n') {
            galley_h += base_size as f32 * zoom * 0.9 * 1.2;
        }
        acc.max(galley_h + 10.0 * zoom)
    })
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
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Plus) || i.consume_key(egui::Modifiers::CTRL, egui::Key::Equals) {
            ed.zoom = (ed.zoom + 0.1).min(3.0);
            ed.heights_dirty = true;
        }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Minus) {
            ed.zoom = (ed.zoom - 0.1).max(0.3);
            ed.heights_dirty = true;
        }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num0) { ed.auto_zoom_done = false; }
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::A) {
            let last = ed.paras.len().saturating_sub(1);
            let end = ed.paras.last().map(|p| p.text.len()).unwrap_or(0);
            ed.doc_sel = Some([DocPos { para: 0, byte: 0 }, DocPos { para: last, byte: end }]);
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
                        .selected_text(egui::RichText::new(cur_style.label()).size(12.0))
                        .width(130.0)
                        .show_ui(ui, |ui| {
                            for s in ParaStyle::all() {
                                if ui.selectable_label(cur_style == *s, s.label()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { ed.apply_style(*s); }
                            }
                        });

                    let cur_text_font = ed.cur_fmt.font.unwrap_or(ed.base_font);
                    egui::ComboBox::from_id_salt("de_font_cb")
                        .selected_text(egui::RichText::new(cur_text_font.label()).size(12.0))
                        .width(112.0)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(ed.cur_fmt.font.is_none(), format!("Default ({})", ed.base_font.label())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                ed.apply_fmt_font(None);
                            }
                            for f in FontChoice::all() {
                                if ui.selectable_label(ed.cur_fmt.font == Some(*f), f.label()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    ed.apply_fmt_font(Some(*f));
                                }
                            }
                        });

                    ui.label(egui::RichText::new("Font Size:").size(11.0).color(lc));
                    let mut sel_sz = ed.sel_font_size_pt();
                    if ui.add(egui::DragValue::new(&mut sel_sz).range(4..=288).speed(1).suffix("pt")).changed() {
                        ed.apply_fmt_size(sel_sz);
                    }

                    ui.separator();
                    if fmt_btn(ui, egui::RichText::new("B").strong().size(13.0), ed.fmt_state_bold(), theme, "Bold (Ctrl+B)") { ed.apply_fmt_toggle_bold(); }
                    if fmt_btn(ui, egui::RichText::new("I").italics().size(13.0), ed.fmt_state_italic(), theme, "Italic (Ctrl+I)") { ed.apply_fmt_toggle_italic(); }
                    if fmt_btn(ui, egui::RichText::new("U").underline().size(13.0), ed.fmt_state_underline(), theme, "Underline (Ctrl+U)") { ed.apply_fmt_toggle_underline(); }

                    let cur_col = ed.cur_fmt.color.map(|c| egui::Color32::from_rgb(c[0], c[1], c[2]))
                        .unwrap_or(if is_dark { ColorPalette::ZINC_200 } else { egui::Color32::from_rgb(22, 22, 22) });
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
                            text_color_palette(ui, ed, is_dark, color_popup_id);
                        });
                    let _ = color_popup_id;

                    let hl_col = ed.cur_fmt.highlight.map(highlight_color32).unwrap_or(if is_dark { ColorPalette::ZINC_300 } else { ColorPalette::GRAY_500 });
                    let hl_btn = ui.scope(|ui| {
                        let s = ui.style_mut();
                        s.visuals.widgets.inactive.bg_fill = if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_200 };
                        s.visuals.widgets.hovered.bg_fill = if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 };
                        ui.add(egui::Button::new(egui::RichText::new("H").size(13.0).color(hl_col)).min_size(egui::vec2(24.0, 26.0)))
                    }).inner.on_hover_text("Highlight color");
                    let hl_popup_id = hl_btn.id;
                    { let r = hl_btn.rect; ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(r.min.x+2.0, r.max.y-4.0), egui::vec2(r.width()-4.0, 3.0)), 1.0, hl_col); }
                    egui::Popup::from_toggle_button_response(&hl_btn)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            ui.set_min_width(160.0);
                            highlight_color_palette(ui, ed, is_dark, hl_popup_id);
                        });
                    let _ = hl_popup_id;

                    let link_btn = ui.add(egui::Button::new(egui::RichText::new("Link").size(11.0)).min_size(egui::vec2(42.0, 26.0)));
                    let link_popup_id = link_btn.id;
                    if link_btn.clicked() {
                        ed.link_input = ed.cur_fmt.link.clone().unwrap_or_default();
                    }
                    egui::Popup::from_toggle_button_response(&link_btn)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            ui.set_min_width(220.0);
                            ui.label(egui::RichText::new("Link URL").size(11.0).color(lc));
                            ui.add(egui::TextEdit::singleline(&mut ed.link_input).desired_width(190.0).hint_text("https://..."));
                            ui.horizontal(|ui| {
                                if ui.button("Apply").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    let url = ed.link_input.trim().to_string();
                                    ed.apply_fmt_link(if url.is_empty() { None } else { Some(url) });
                                    egui::Popup::close_id(ui.ctx(), link_popup_id);
                                }
                                if ui.button("Remove").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    ed.apply_fmt_link(None);
                                    egui::Popup::close_id(ui.ctx(), link_popup_id);
                                }
                            });
                        });
                    let _ = link_popup_id;

                    let tbl_btn = toolbar_action_btn(ui, "Table", theme).on_hover_text("Insert Table");
                    let tbl_pid = tbl_btn.id;
                    egui::Popup::from_toggle_button_response(&tbl_btn)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            let (def, hi, bdr) = if is_dark {
                                (ColorPalette::ZINC_700, ColorPalette::BLUE_600, ColorPalette::ZINC_500)
                            } else {
                                (ColorPalette::GRAY_200, ColorPalette::BLUE_500, ColorPalette::GRAY_400)
                            };
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
                                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                            ed.insert_table(row + 1, col + 1);
                                            egui::Popup::close_id(ui.ctx(), tbl_pid);
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
                    for a in Align::all() {
                        if fmt_btn(ui, egui::RichText::new(a.label()).size(12.0), cur_align == *a, theme, a.full_label()) { ed.apply_align(*a); }
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("LH:").size(11.0).color(lc));
                    if ui.add(egui::DragValue::new(&mut ed.line_spacing_input).range(0.8..=4.0).speed(0.05).fixed_decimals(2)).changed() {
                        ed.paras[ed.focused_para].line_height = ed.line_spacing_input; ed.dirty = true; ed.heights_dirty = true;
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if act_btn(ui, "-", theme, "Zoom out (Ctrl+-)") { ed.zoom = (ed.zoom - 0.1).max(0.3); }
                        ui.label(egui::RichText::new(format!("{:.0}%", ed.zoom * 100.0)).size(11.0).color(lc));
                        if act_btn(ui, "+", theme, "Zoom in (Ctrl++)") { ed.zoom = (ed.zoom + 0.1).min(3.0); }
                    });
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
        let mut h = ed.para_heights.get(i).copied().unwrap_or(ed.base_size as f32 * ed.zoom * 1.8);
        if h <= 0.0 { h = ed.base_size as f32 * ed.zoom * 1.2; }

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

fn measure_para_total_height(ctx: &egui::Context, para: &DocParagraph, base_font: FontChoice, base_size: u32, wrap_w: f32, zoom: f32, is_dark: bool) -> f32 {
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
    left.is_split = false;
    merge_adjacent(&mut left);
    let mut right = src.clone();
    right.text = src.text[split_byte..].to_string();
    right.spans = if right_spans.is_empty() { vec![DocSpan { len: 0, fmt: SpanFmt::default() }] } else { right_spans };
    right.space_before = 0.0;
    right.indent_first = right.indent_left;
    right.is_split = true;
    merge_adjacent(&mut right);
    (left, right)
}

fn find_split_byte_fit(ctx: &egui::Context, para: &DocParagraph, base_font: FontChoice, base_size: u32, wrap_w: f32, max_total_h: f32, zoom: f32, is_dark: bool) -> usize {
    let job = build_layout_job(&para.spans, &para.text, para, base_font, base_size, wrap_w, is_dark, zoom);
    let galley = ctx.fonts_mut(|f| f.layout_job(job));
    let mut split_char = para.text.chars().count();
    let mut char_pos = 0usize;
    for row in &galley.rows {
        let row_bottom = para.space_before * zoom + row.rect().max.y;
        if row_bottom > max_total_h {
            split_char = char_pos;
            break;
        }
        char_pos += row.glyphs.len();
        if row.ends_with_newline {
            char_pos += 1;
        }
    }
    if split_char == 0 {
        return 0;
    }
    para.text.char_indices().nth(split_char).map(|(b, _)| b).unwrap_or(para.text.len())
}

fn reflow_overflow_paragraphs(ed: &mut DocumentEditor, ctx: &egui::Context, is_dark: bool) {
    if !ed.heights_dirty { return; }
    let mut focus_p = ed.focused_para;
    let mut focus_b = 0;
    if focus_p < ed.paras.len() && focus_p < ed.para_ids.len() {
        if let Some(state) = egui::TextEdit::load_state(ctx, ed.para_ids[focus_p]) {
            if let Some(cr) = state.cursor.char_range() {
                let text = &ed.paras[focus_p].text;
                let char_idx = cr.primary.index;
                focus_b = text.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(text.len());
            }
        }
    }

    let page_content_h = ed.layout.content_height() * ed.zoom;
    let cw = ed.layout.content_width() * ed.zoom;
    let bs = ed.base_size as f32 * ed.zoom;
    let font = ed.base_font;
    let mt = ed.layout.margin_top * ed.zoom;
    let min_fill = bs * 2.0;

    let mut j = 0;
    while j < ed.paras.len() {
        if ed.paras[j].is_split && j > 0 {
            let prev_len = ed.paras[j - 1].text.len();
            let orig_space_after = ed.paras[j].space_after;
            if focus_p == j {
                focus_p = j - 1;
                focus_b += prev_len;
            } else if focus_p > j {
                focus_p -= 1;
            }
            merge_paragraphs(&mut ed.paras, j - 1);
            ed.paras[j - 1].space_after = orig_space_after;
            ed.paras[j - 1].is_split = false;
        } else {
            j += 1;
        }
    }

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
        if para.style == ParaStyle::Table {
            let h = ed.para_heights.get(i).copied().unwrap_or(bs * 1.6);
            if cur_y > mt && cur_y + h > mt + page_content_h { cur_y = mt; }
            cur_y += h.min(page_content_h);
            i += 1;
            continue;
        }
        let wrap_w = (cw - para.indent_left * ed.zoom).max(40.0);
        let h = measure_para_total_height(ctx, &para, font, ed.base_size, wrap_w, ed.zoom, is_dark);
        let remaining = mt + page_content_h - cur_y;

        if h <= remaining + 0.5 {
            cur_y += h;
            if cur_y >= mt + page_content_h { cur_y = mt; }
            i += 1;
            continue;
        }
        
        if !para.text.is_empty() && cur_y > mt && remaining > min_fill {
            let split = find_split_byte_fit(ctx, &para, font, ed.base_size, wrap_w, remaining, ed.zoom, is_dark);
            if split > 0 && split < para.text.len() {
                let (left, right) = split_para_at_byte(&para, split);
                ed.paras[i] = left;
                ed.paras.insert(i + 1, right);
                if focus_p == i {
                    if focus_b >= split {
                        focus_p = i + 1;
                        focus_b -= split;
                    }
                } else if focus_p > i {
                    focus_p += 1;
                }
                continue;
            }
        }
        cur_y = mt;
        
        if !para.text.is_empty() && h > page_content_h + 0.5 {
            let split = find_split_byte_fit(ctx, &para, font, ed.base_size, wrap_w, page_content_h, ed.zoom, is_dark);
            let split = split.max(para.text.char_indices().nth(1).map(|(b, _)| b).unwrap_or(para.text.len()));
            if split < para.text.len() {
                let (left, right) = split_para_at_byte(&para, split);
                ed.paras[i] = left;
                ed.paras.insert(i + 1, right);
                if focus_p == i {
                    if focus_b >= split {
                        focus_p = i + 1;
                        focus_b -= split;
                    }
                } else if focus_p > i {
                    focus_p += 1;
                }
                continue;
            }
        }
        
        cur_y = (cur_y + h).min(mt + page_content_h);
        if cur_y >= mt + page_content_h { cur_y = mt; }
        i += 1;
    }
    
    let n = ed.paras.len();
    ed.para_texts.resize(n, String::new());
    ed.para_ids.resize_with(n, || egui::Id::new(egui::Id::NULL));
    ed.para_heights.resize(n, 0.0);
    for k in 0..n {
        ed.para_texts[k] = ed.paras[k].text.clone();
        ed.para_ids[k] = egui::Id::new(("de_para", k as u64));
    }

    if focus_p < n && ed.paras[focus_p].style != ParaStyle::Table && ed.paras[focus_p].style != ParaStyle::HRule {
        ed.focused_para = focus_p;
        ed.pending_focus = Some(focus_p);
        let text = &ed.para_texts[focus_p];
        let safe_b = focus_b.min(text.len());
        let char_idx = text[..safe_b].chars().count();
        
        let id = ed.para_ids[focus_p];
        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(char_idx))));
        egui::TextEdit::store_state(ctx, id, state);
    }
    ed.find_stale = true;
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
    let bs = ed.base_size as f32 * ed.zoom;
    let font = ed.base_font;
    reflow_overflow_paragraphs(ed, ctx, is_dark);
    let n = ed.paras.len();

    if ed.heights_dirty || ed.para_heights.len() != n {
        ed.para_heights.resize(n, 0.0);
        for i in 0..n {
            let p = &ed.paras[i];
            if p.style == ParaStyle::HRule { ed.para_heights[i] = p.space_before * ed.zoom + 12.0 + p.space_after * ed.zoom; continue; }
            if p.style == ParaStyle::Table {
                if let Some(tbl) = &p.table {
                    let col_ws = table_col_widths(tbl, ed.layout.content_width() * ed.zoom);
                    let rows_h: f32 = tbl.rows.iter().enumerate().map(|(ri, row)| {
                        let live_cell = match ed.active_table {
                            Some((ti, tr, tc)) if ti == i && tr == ri => Some((tc, ed.cell_edit_buf.as_str())),
                            _ => None,
                        };
                        table_row_h(row, &col_ws, font, ed.base_size, ed.zoom, ctx, live_cell, ri == 0)
                    }).sum();
                    ed.para_heights[i] = p.space_before * ed.zoom + 6.0 * ed.zoom + rows_h + p.space_after * ed.zoom;
                } else { ed.para_heights[i] = 0.0; }
                continue;
            }
            let indent = p.indent_left * ed.zoom;
            let wrap_w = (cw - indent).max(40.0);
            let job = build_layout_job(&p.spans, &p.text, p, font, ed.base_size, wrap_w, is_dark, ed.zoom);
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

    let mut active_sel = ed.norm_sel().filter(|(f, t)| f.para != t.para);
    if active_sel.is_none() {
        if let Some((pi, sb, eb)) = ed.last_selection {
            if sb != eb && !ctx.memory(|m| m.has_focus(ed.para_ids[pi])) {
                let start = sb.min(eb);
                let end = sb.max(eb);
                active_sel = Some((DocPos { para: pi, byte: start }, DocPos { para: pi, byte: end }));
            }
        }
    }
    let ptr = ctx.pointer_hover_pos();
    let btn_pressed = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
    let btn_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
    let shift = ctx.input(|i| i.modifiers.shift);
    let ctrl = ctx.input(|i| i.modifiers.ctrl);
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
    let mut table_cell_change: Option<(usize, usize, usize, String)> = None;
    let mut table_col_resize: Option<(usize, usize, f32)> = None;

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
            let checkbox_rect = if para.style == ParaStyle::ListCheck {
                Some(egui::Rect::from_center_size(
                    egui::pos2(edit_x - 8.0 * ed.zoom, text_y + text_h / 2.0),
                    egui::vec2((12.0 * ed.zoom).max(10.0), (12.0 * ed.zoom).max(10.0)),
                ))
            } else { None };

           if let Some(pp) = ptr {
            if let Some(cb) = checkbox_rect {
                if btn_pressed && cb.expand(4.0).contains(pp) {
                    ed.push_undo();
                    ed.paras[i].checked = !ed.paras[i].checked;
                    ed.sync_texts();
                    ed.dirty = true;
                    continue;
                }
            }

            if edit_rect.contains(pp) {
                if para.style == ParaStyle::Table {
                    if btn_pressed {
                        if let Some(ref tbl) = ed.paras[i].table {
                            let col_ws = table_col_widths(tbl, cw);
                            let mut ry = text_y + 6.0 * ed.zoom;
                            'tbl_click: for (ri, row) in tbl.rows.iter().enumerate() {
                                let rh = table_row_h(row, &col_ws, font, ed.base_size, ed.zoom, ctx,
                                    match ed.active_table {
                                        Some((ti, tr, tc)) if ti == i && tr == ri => Some((tc, ed.cell_edit_buf.as_str())),
                                        _ => None,
                                    },
                                    ri == 0
                                );
                                if pp.y >= ry && pp.y < ry + rh {
                                    let mut cx_acc = pm.x + ml;
                                    let cc = col_ws.iter().enumerate().find_map(|(ci, &w)| {
                                        if pp.x < cx_acc + w { Some(ci) } else { cx_acc += w; None }
                                    }).unwrap_or(col_ws.len().saturating_sub(1)).min(row.len().saturating_sub(1));
                                    if let Some((old_i, old_r, old_c)) = ed.active_table {
                                        table_cell_change = Some((old_i, old_r, old_c, ed.cell_edit_buf.clone()));
                                    }
                                    ed.active_table = Some((i, ri, cc));
                                    ed.focused_para = i;
                                    ed.pending_focus = None;
                                    ed.cell_edit_buf = row.get(cc).map(|c| c.text.clone()).unwrap_or_default();
                                    let cell_id = ui.id().with(("table_cell", i, ri, cc));
                                    ctx.memory_mut(|m| m.request_focus(cell_id));
                                    break 'tbl_click;
                                }
                                ry += rh;
                            }
                        }
                    }
                } else {
                    let job = build_layout_job(&ed.paras[i].spans, &ed.paras[i].text, &ed.paras[i], font, ed.base_size, wrap_w, is_dark, ed.zoom);
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
                            ed.doc_sel = if shift {
                                ed.doc_sel.map(|[a, _]| [a, pos]).or(Some([pos, pos]))
                            } else {
                                Some([pos, pos])
                            };
                        }
                    } else if btn_pressed {
                        if let Some((pi, ri, ci)) = ed.active_table.take() {
                            table_cell_change = Some((pi, ri, ci, ed.cell_edit_buf.clone()));
                        }
                        let pos = DocPos { para: i, byte };
                        ed.doc_sel = if shift {
                            ed.doc_sel.map(|[a, _]| [a, pos]).or(Some([pos, pos]))
                        } else {
                            Some([pos, pos])
                        };
                    } else if btn_down {
                        if ed.doc_sel.is_none() {
                            if let Some((pi, sb, _)) = ed.last_selection {
                                let anchor = DocPos { para: pi, byte: sb };
                                let pos = DocPos { para: i, byte };
                                if anchor.para != i || anchor.byte != byte {
                                    ed.doc_sel = Some([anchor, pos]);
                                }
                            }
                        } else if let Some(ref mut sel) = ed.doc_sel {
                            if sel[0].para != i || (sel[0].para == i && sel[0].byte != byte) {
                                sel[1] = DocPos { para: i, byte };
                            }
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
                    if let Some((from, to)) = active_sel {
                        if i >= from.para && i <= to.para {
                            painter.rect_filled(
                                egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y), egui::vec2(cw, text_h.max(4.0))),
                                0.0, sel_color,
                            );
                        }
                    }
                    continue;
                }

                if para.style == ParaStyle::Table {
                    if let Some(ref tbl) = para.table {
                        let col_ws = table_col_widths(tbl, cw);
                        let nc = tbl.rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
                        let table_top_pad = 6.0 * ed.zoom;
                        let table_y = text_y + table_top_pad;
                        let row_hs: Vec<f32> = tbl.rows.iter().enumerate().map(|(ri, row)| {
                            let live_cell = match ed.active_table {
                                Some((ti, tr, tc)) if ti == i && tr == ri => Some((tc, ed.cell_edit_buf.as_str())),
                                _ => None,
                            };
                            table_row_h(row, &col_ws, font, ed.base_size, ed.zoom, ctx, live_cell, ri == 0)
                        }).collect();
                        let total_h: f32 = row_hs.iter().sum();
                        let tbl_bg = if is_dark { egui::Color32::from_rgb(28, 28, 36) } else { egui::Color32::WHITE };
                        let hdr_bg = if is_dark { egui::Color32::from_rgb(30, 42, 72) } else { ColorPalette::BLUE_50 };
                        let alt_bg = if is_dark { egui::Color32::from_rgb(24, 24, 30) } else { ColorPalette::GRAY_50 };
                        let bdr = if is_dark { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_300 };
                        let tc = if is_dark { ColorPalette::ZINC_200 } else { ColorPalette::GRAY_800 };
                        let hdr_tc = if is_dark { egui::Color32::WHITE } else { ColorPalette::GRAY_900 };
                        let mut ry = table_y;
                        for (ri, row) in tbl.rows.iter().enumerate() {
                            let rh = row_hs[ri];
                            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(pm.x + ml, ry), egui::vec2(cw, rh)), 0.0, if ri == 0 { hdr_bg } else if ri % 2 == 0 { alt_bg } else { tbl_bg });
                            let mut cx = pm.x + ml;
                            for (ci, cell) in row.iter().enumerate() {
                                let cw_cell = col_ws.get(ci).copied().unwrap_or(cw / nc as f32);
                                let is_active = ed.active_table == Some((i, ri, ci));
                                if is_active {
                                    painter.rect_filled(egui::Rect::from_min_size(egui::pos2(cx, ry), egui::vec2(cw_cell, rh)), 0.0, egui::Color32::from_rgba_unmultiplied(59, 130, 246, 50));
                                    let cr = egui::Rect::from_min_size(egui::pos2(cx + 4.0, ry + 2.0), egui::vec2(cw_cell - 8.0, rh - 4.0));
                                    let cell_id = ui.id().with(("table_cell", i, ri, ci));
                                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(cr));
                                    let te = egui::TextEdit::multiline(&mut ed.cell_edit_buf).id(cell_id)
                                        .desired_width(cw_cell - 8.0)
                                        .min_size(egui::vec2(cw_cell - 8.0, rh - 4.0)) // <-- Forces TextEdit to fill the cell area
                                        .desired_rows(1).frame(false)
                                        .font(egui::FontId::new(bs * 0.9, font.egui_family(ri == 0, false))).show(&mut child);
                                    if te.response.changed() {
                                        ed.heights_dirty = true;
                                        ed.dirty = true;
                                    }
                                    if te.response.lost_focus() || ctx.input(|inp| inp.key_pressed(egui::Key::Escape)) {
                                        table_cell_change = Some((i, ri, ci, ed.cell_edit_buf.clone()));
                                        ed.active_table = None;
                                    }
                                } else {
                                    let job = egui::text::LayoutJob::simple(cell.text.clone(), egui::FontId::new(bs * 0.9, font.egui_family(ri == 0, false)), if ri == 0 { hdr_tc } else { tc }, (cw_cell - 16.0 * ed.zoom).max(1.0));
                                    let galley = ctx.fonts_mut(|f| f.layout_job(job));
                                    let ty = ry + (rh - galley.rect.height()).max(0.0) / 2.0;
                                    let clip = egui::Rect::from_min_size(egui::pos2(cx + 4.0, ry), egui::vec2(cw_cell - 8.0, rh));
                                    painter.with_clip_rect(clip).galley(egui::pos2(cx + 8.0 * ed.zoom, ty), galley, if ri == 0 { hdr_tc } else { tc });
                                }
                                if ci > 0 { painter.vline(cx, ry..=(ry + rh), egui::Stroke::new(1.0, bdr)); }
                                cx += cw_cell;
                            }
                            if ri > 0 { painter.hline((pm.x + ml)..=(pm.x + ml + cw), ry, egui::Stroke::new(1.0, bdr)); }
                            ry += rh;
                        }
                        painter.rect_stroke(egui::Rect::from_min_size(egui::pos2(pm.x + ml, table_y), egui::vec2(cw, total_h)), 0.0, egui::Stroke::new(1.0, bdr), egui::StrokeKind::Outside);
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

                if i == focused {
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(pm.x + ml, text_y - 2.0), egui::vec2(cw, text_h + 4.0)),
                        0.0, focus_bg,
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
                            format!("{}.", num), egui::FontId::proportional(para.style.default_font_size_pt(ed.base_size) as f32 * ed.zoom * 0.9), bullet_col,
                        );
                    }
                    ParaStyle::ListCheck => {
                        let box_sz = (11.0 * ed.zoom).max(9.0);
                        let box_rect = egui::Rect::from_center_size(
                            egui::pos2(edit_x - 8.0 * ed.zoom, text_y + text_h / 2.0),
                            egui::vec2(box_sz, box_sz),
                        );
                        painter.rect_stroke(box_rect, 2.0, egui::Stroke::new(1.4, bullet_col), egui::StrokeKind::Outside);
                        if para.checked {
                            painter.line_segment([
                                box_rect.left_top() + egui::vec2(box_sz * 0.18, box_sz * 0.55),
                                box_rect.center() + egui::vec2(-box_sz * 0.08, box_sz * 0.18),
                            ], egui::Stroke::new(1.8, bullet_col));
                            painter.line_segment([
                                box_rect.center() + egui::vec2(-box_sz * 0.10, box_sz * 0.15),
                                box_rect.right_top() + egui::vec2(-box_sz * 0.16, box_sz * 0.28),
                            ], egui::Stroke::new(1.8, bullet_col));
                        }
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

                if let Some((from, to)) = active_sel {
                    if i >= from.para && i <= to.para {
                        let start_byte = if i == from.para { from.byte } else { 0 };
                        let end_byte = if i == to.para { to.byte } else { para.text.len() };
                        let job = build_layout_job(&para.spans, &para.text, para, font, ed.base_size, wrap_w, is_dark, ed.zoom);
                        let galley = ctx.fonts_mut(|f| f.layout_job(job));
                        let align_offset = match para.align {
                            Align::Center => ((edit_w - galley.rect.width() - 8.0) / 2.0).max(0.0),
                            Align::Right => (edit_w - galley.rect.width() - 8.0).max(0.0),
                            _ => 0.0,
                        };
                        let base_x = edit_x + align_offset;

                        for rect in multiline_highlight(&galley, &para.text, start_byte, end_byte) {
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
                let should_handle_bksp = at_start && (ed.paras[i].indent_first > 0.0 || ed.paras[i].indent_left > 0.0 || i > 0);
                if should_handle_bksp && ctx.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)) {
                    if ed.paras[i].indent_first > 0.0 {
                        ed.push_undo();
                        ed.paras[i].indent_first = (ed.paras[i].indent_first - 36.0).max(0.0);
                        ed.dirty = true;
                        ed.heights_dirty = true;
                    } else if ed.paras[i].indent_left > 0.0 {
                        ed.push_undo();
                        ed.paras[i].indent_left = (ed.paras[i].indent_left - 36.0).max(0.0);
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
            let base_size_snap = ed.base_size;
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
                                let is_last_row = row_idx == galley.rows.len() - 1;
                                let next_char_pos = row_end + if row.ends_with_newline { 1 } else { 0 };
                                if cr.primary.index >= row_start && (cr.primary.index < next_char_pos || is_last_row) {
                                    let local_index = cr.primary.index.saturating_sub(row_start);
                                    cur_x_local = if local_index == 0 {
                                        row.rect().min.x
                                    } else if local_index >= glyph_count {
                                        row.rect().max.x
                                    } else {
                                        row.glyphs.get(local_index).map(|g| g.pos.x).unwrap_or(row.rect().max.x)
                                    };
                                    is_top = row_idx == 0; 
                                    is_bottom = is_last_row;
                                    break;
                                }
                                char_pos = next_char_pos;
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
                                let target_job = build_layout_job(&target_para.spans, &target_para.text, target_para, font, ed.base_size, target_wrap_w, is_dark, ed.zoom);
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
            if ed.pending_focus == Some(i) && ed.active_table.is_none() { ctx.memory_mut(|m| m.request_focus(id)); }
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
                let table_top = outer.min.y + pl.page_tops[tpg] + pl.para_content_y[i + 1];
                egui::Rect::from_min_max(edit_rect.min, egui::pos2(edit_rect.max.x, table_top.min(edit_rect.max.y)))
            } else { edit_rect };
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(effective_rect));
            let output = egui::TextEdit::multiline(text_ref)
                .id(id)
                .desired_width(edit_w)
                .desired_rows(1)
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
            if has_focus && ed.focused_para != i && ed.active_table.is_none() {
                ed.focused_para = i;
                ed.line_spacing_input = ed.paras[i].line_height;
                ed.last_edit_action = 0;
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
                if tab_keys.0 || tab_keys.1 || tab_keys.2 || tab_keys.3 {
                    let shift_tab = tab_keys.1 || tab_keys.3;
                    let delta = if shift_tab { -36.0f32 } else { 36.0f32 };
                    let max_indent = (ed.layout.content_width() - 36.0).max(0.0);
                    let state = egui::TextEdit::load_state(ctx, id);
                    let cr = state.as_ref().and_then(|s| s.cursor.char_range());
                    let has_selection = cr.map(|cr| cr.primary.index != cr.secondary.index).unwrap_or(false);
                    let caret_index = cr.map(|cr| cr.primary.index.min(cr.secondary.index)).unwrap_or(0);
                    let caret_at_start = cr.map(|cr| cr.primary.index == 0 && cr.secondary.index == 0).unwrap_or(false);
                    let caret_byte = char_to_byte(&ed.para_texts[i], caret_index);
                    let mut changed = false;
                    ed.push_undo();

                    if has_selection {
                        let before = ed.paras[i].indent_left;
                        ed.paras[i].indent_left = (ed.paras[i].indent_left + delta).clamp(0.0, max_indent);
                        changed = (ed.paras[i].indent_left - before).abs() > f32::EPSILON;
                    } else if caret_at_start {
                        if shift_tab {
                            if ed.paras[i].indent_first > 0.0 {
                                let before = ed.paras[i].indent_first;
                                ed.paras[i].indent_first = (ed.paras[i].indent_first + delta).max(0.0);
                                changed = (ed.paras[i].indent_first - before).abs() > f32::EPSILON;
                            } else {
                                let before = ed.paras[i].indent_left;
                                ed.paras[i].indent_left = (ed.paras[i].indent_left + delta).max(0.0);
                                changed = (ed.paras[i].indent_left - before).abs() > f32::EPSILON;
                            }
                        } else {
                            let before = ed.paras[i].indent_first;
                            ed.paras[i].indent_first = (ed.paras[i].indent_first + delta).clamp(0.0, max_indent);
                            changed = (ed.paras[i].indent_first - before).abs() > f32::EPSILON;
                        }
                    } else if shift_tab {
                        if caret_byte > 0 && ed.paras[i].text.as_bytes().get(caret_byte - 1) == Some(&b'\t') {
                            let mut new_text = ed.paras[i].text.clone();
                            new_text.remove(caret_byte - 1);
                            let fmt = para_fmt_at(&ed.paras[i], caret_byte.saturating_sub(1));
                            rebuild_spans(&mut ed.paras[i], new_text, &fmt);
                            ed.para_texts[i] = ed.paras[i].text.clone();
                            if let Some(mut st) = state {
                                let cc = egui::text::CCursor::new(caret_index.saturating_sub(1));
                                st.cursor.set_char_range(Some(egui::text::CCursorRange::one(cc)));
                                egui::TextEdit::store_state(ctx, id, st);
                            }
                            changed = true;
                        }
                    } else {
                        let mut new_text = ed.paras[i].text.clone();
                        new_text.insert(caret_byte, '\t');
                        let fmt = para_fmt_at(&ed.paras[i], caret_byte);
                        rebuild_spans(&mut ed.paras[i], new_text, &fmt);
                        ed.para_texts[i] = ed.paras[i].text.clone();
                        if let Some(mut st) = state {
                            let cc = egui::text::CCursor::new(caret_index + 1);
                            st.cursor.set_char_range(Some(egui::text::CCursorRange::one(cc)));
                            egui::TextEdit::store_state(ctx, id, st);
                        }
                        changed = true;
                    }

                    if !changed {
                        ed.undo_stack.pop_back();
                    } else {
                        ed.dirty = true;
                        ed.heights_dirty = true;
                        ed.find_stale = true;
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
                            c.bold = fmt.bold; c.italic = fmt.italic; c.underline = fmt.underline;
                            c.strike = fmt.strike; c.sub = fmt.sub; c.sup = fmt.sup;
                            c.size_hp = fmt.size_hp; c.font = fmt.font; c.color = fmt.color;
                            c.highlight = fmt.highlight; c.link = fmt.link;
                            if !ed.has_cross_sel() {
                                ed.doc_sel = None;
                            }
                        }
                    }
                }
            }

            if ed.pending_focus == Some(i) { pending_focus_next = Some(i); ed.pending_focus = None; }
            if output.response.changed() { text_change = Some((i, ed.para_texts[i].clone())); }
            let new_h = output.galley.size().y;
            if (new_h - text_h).abs() > 0.5 { ed.heights_dirty = true; }
        }
    });

    if let Some(f) = pending_focus_next { ed.focused_para = f.min(ed.paras.len().saturating_sub(1)); }
    if let Some(sel) = new_selection { ed.last_selection = Some(sel); }

    if let Some((pi, row, col, text)) = table_cell_change {
        if pi < ed.paras.len() {
            ed.push_undo();
            if let Some(ref mut tbl) = ed.paras[pi].table {
                if let Some(r) = tbl.rows.get_mut(row) {
                    if let Some(c) = r.get_mut(col) {
                        c.text = text.clone();
                        c.spans = if text.is_empty() { vec![DocSpan { len: 0, fmt: SpanFmt::default() }] }
                            else { vec![DocSpan { len: text.len(), fmt: SpanFmt::default() }] };
                    }
                }
            }
            ed.dirty = true;
            ed.heights_dirty = true;
        }
    }
    if let Some((pi, col, delta)) = table_col_resize {
        if pi < ed.paras.len() {
            if let Some(ref mut tbl) = ed.paras[pi].table {
                let nc = tbl.rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
                if tbl.col_widths.len() != nc { tbl.col_widths = vec![1.0 / nc as f32; nc]; }
                if col < nc {
                    let df = (delta / (ed.layout.content_width() * ed.zoom)).clamp(0.05 - tbl.col_widths[col - 1], tbl.col_widths[col] - 0.05);
                    tbl.col_widths[col - 1] += df;
                    tbl.col_widths[col] -= df;
                    ed.heights_dirty = true;
                    ed.dirty = true;
                }
            }
        }
    }

    if let Some(mu) = merge_up {
        if mu > 0 && mu < ed.paras.len() {
            ed.push_undo();
            if ed.paras[mu - 1].style == ParaStyle::HRule {
                ed.paras.remove(mu - 1);
                ed.focused_para = mu - 1;
                ed.pending_focus = Some(mu - 1);
                ed.sync_texts(); ed.dirty = true; ed.heights_dirty = true;
            } else if ed.paras[mu - 1].style == ParaStyle::Table || ed.paras[mu].style == ParaStyle::Table {
                ed.undo_stack.pop_back();
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
            } else if ed.paras[md + 1].style == ParaStyle::Table || ed.paras[md].style == ParaStyle::Table {
                ed.undo_stack.pop_back();
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
                let old_text = &ed.paras[i].text;
                let diff = new_text.len() as isize - old_text.len() as isize;
                let mut should_push = false;
                let new_action: u8;

                if diff > 1 {
                    should_push = true;
                    new_action = 0;
                } else if diff < 0 {
                    if ed.last_edit_action != 2 { should_push = true; }
                    new_action = 2;
                } else if diff == 1 {
                    let is_space = new_text.ends_with(|c: char| c.is_whitespace() || c.is_ascii_punctuation());
                    let was_space = old_text.ends_with(|c: char| c.is_whitespace() || c.is_ascii_punctuation());
                    if (was_space && !is_space) || ed.last_edit_action != 1 {
                        should_push = true;
                    }
                    new_action = 1;
                } else {
                    should_push = true;
                    new_action = 0;
                }

                if should_push { ed.push_undo(); }
                ed.last_edit_action = new_action;

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
                if ui.small_button("x").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { ed.find_text.clear(); ed.find_results.clear(); }
            });
            if !ed.find_text.is_empty() {
                let (c, t) = (if ed.find_results.is_empty() { 0 } else { ed.find_cursor + 1 }, ed.find_results.len());
                ui.label(egui::RichText::new(format!("{}/{}", c, t)).size(11.0).color(if t == 0 { ColorPalette::RED_400 } else { muted }));
            }
            ui.horizontal(|ui| { 
                if ui.button("Prev").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { ed.find_prev(); } 
                if ui.button("Next").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { ed.find_next(); } 
            });
            ui.add_space(4.0); ui.separator(); ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Replace:").size(12.0).color(muted));
                ui.add(egui::TextEdit::singleline(&mut ed.replace_text).desired_width(200.0).hint_text("Replacement..."));
            });
            ui.horizontal(|ui| {
                if ui.button("Replace").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { ed.replace_current(); }
                if ui.button("Replace All").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { ed.replace_all(); } 
            });
            ui.add_space(4.0);
            if ui.button("Close").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { ed.show_find = false; }
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
            ui.add_space(8.0); if ui.button("Close").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { ed.show_stats = false; }
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

    let page_win = egui::Window::new("Page Setup")
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
            let sizes = ["8.5 x 11 in", "8.5 x 11 in", "210 x 297 mm", "8.5 x 14 in", "297 x 420 mm", "148 x 210 mm", "7.25 x 10.5 in", "11 x 17 in", "176 x 250 mm"];
            for row in 0..rows {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for col in 0..cols {
                        let idx = row * cols + col;
                        let Some((name, _)) = presets.get(idx) else { continue; };
                        let sel = draft.1 == idx;
                        let size_str = sizes.get(idx).copied().unwrap_or("");
                        let (fill, stroke, label_col) = if sel {
                            (if is_dark { egui::Color32::from_rgb(28,52,100) } else { egui::Color32::from_rgb(225,238,255) },
                            egui::Stroke::new(1.5, if is_dark { ColorPalette::BLUE_500 } else { ColorPalette::BLUE_400 }),
                            if is_dark { egui::Color32::WHITE } else { ColorPalette::BLUE_700 })
                        } else {
                            (if is_dark { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_100 },
                            egui::Stroke::new(1.0, if is_dark { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 }),
                            tc)
                        };
                        let size_col = if sel {
                            egui::Color32::from_rgba_unmultiplied(label_col.r(), label_col.g(), label_col.b(), 160)
                        } else {
                            muted
                        };
                        let (tag_bg, tag_col, badge) = match idx {
                            0 => (tag_bg_gd, tag_col_gd, Some("Docs")),
                            1 => (tag_bg_wd, tag_col_wd, Some("Word")),
                            _ => (egui::Color32::TRANSPARENT, egui::Color32::TRANSPARENT, None),
                        };
                        let (cell, _) = ui.allocate_exact_size(egui::vec2(btn_w, 56.0), egui::Sense::hover());
                        if ui.is_rect_visible(cell) {
                            let painter = ui.painter_at(cell);
                            painter.rect(cell, 6.0, fill, stroke, egui::StrokeKind::Inside);
                            let name_y = if badge.is_some() { cell.min.y + 15.0 } else { cell.center().y - 8.0 };
                            painter.text(egui::pos2(cell.center().x, name_y), egui::Align2::CENTER_CENTER, *name, egui::FontId::proportional(12.0), label_col);
                            painter.text(egui::pos2(cell.center().x, name_y + 14.0), egui::Align2::CENTER_CENTER, size_str, egui::FontId::proportional(9.5), size_col);
                            if let Some(b) = badge {
                                let bw = 34.0_f32;
                                let br = egui::Rect::from_min_size(egui::pos2(cell.center().x - bw/2.0, cell.max.y - 13.0), egui::vec2(bw, 11.0));
                                painter.rect_filled(br, 3.0, tag_bg);
                                painter.text(br.center(), egui::Align2::CENTER_CENTER, b, egui::FontId::proportional(8.0), tag_col);
                            }
                        }
                        let resp = ui.interact(cell, ui.id().with(("ps", idx)), egui::Sense::click());
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { draft.1 = idx; draft.0 = PageLayout::from_preset(idx); }
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
                ui.add(egui::Slider::new(&mut draft.2, 8..=24).suffix(" pt"));
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Apply").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { apply = true; }
                if ui.button("Cancel").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { cancel = true; }
            });
        });

    if let Some(r) = &page_win {
        let clicked_outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().map_or(false, |p| !r.response.rect.contains(p)));
        if clicked_outside { cancel = true; }
    }

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
