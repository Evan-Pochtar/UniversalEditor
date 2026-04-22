use eframe::egui;
use crate::{modules::EditorModule, style::{ColorPalette, ThemeMode, toolbar_action_btn}};
use super::te_main::{TextEditor, ViewMode};

impl TextEditor {
    pub(super) fn render_editor_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, show_toolbar: bool, show_file_info: bool) {
        if show_toolbar {
            ui.horizontal(|ui: &mut egui::Ui| {
                let dark = ui.visuals().dark_mode;
                let theme = if dark { ThemeMode::Dark } else { ThemeMode::Light };
                ui.horizontal(|ui: &mut egui::Ui| {
                    if toolbar_action_btn(ui, egui::RichText::new("B").strong().size(12.0), theme).on_hover_text("Bold (Ctrl+B)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.format_bold(); }
                    if toolbar_action_btn(ui, egui::RichText::new("I").italics().size(12.0), theme).on_hover_text("Italic (Ctrl+I)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.format_italic(); }
                    if toolbar_action_btn(ui, egui::RichText::new("U").underline().size(12.0), theme).on_hover_text("Underline (Ctrl+U)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.format_underline(); }
                    if toolbar_action_btn(ui, egui::RichText::new("S").strikethrough().size(12.0), theme).on_hover_text("Strikethrough (Ctrl+Shift+S)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.format_strikethrough(); }
                    if toolbar_action_btn(ui, egui::RichText::new("C").monospace().size(12.0), theme).on_hover_text("Code (Ctrl+E)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.format_code(); }
                    ui.separator();
                    if toolbar_action_btn(ui, "H1", theme).on_hover_text("Header 1 (Ctrl+1)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.format_heading(1); }
                    if toolbar_action_btn(ui, "H2", theme).on_hover_text("Header 2 (Ctrl+2)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.format_heading(2); }
                    if toolbar_action_btn(ui, "H3", theme).on_hover_text("Header 3 (Ctrl+3)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.format_heading(3); }
                    if toolbar_action_btn(ui, "H4", theme).on_hover_text("Header 4 (Ctrl+4)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.format_heading(4); }
                    ui.separator();
                    if toolbar_action_btn(ui, ">", theme).on_hover_text("Blockquote (Ctrl+Shift+Q)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.format_blockquote(); }
                    if toolbar_action_btn(ui, "[ ]", theme).on_hover_text("Checklist Item (Ctrl+Shift+L)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.insert_checklist_item(); }
                    ui.separator();
                    let tbl_btn = toolbar_action_btn(ui, "Table", theme).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Insert Table");
                    let tbl_popup_id = tbl_btn.id;
                    egui::Popup::from_toggle_button_response(&tbl_btn)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                        let is_dark = ui.visuals().dark_mode;
                        let (cell_def, cell_hi, border) = if is_dark {
                            (ColorPalette::ZINC_700, ColorPalette::BLUE_600, ColorPalette::ZINC_500)
                        } else {
                            (ColorPalette::GRAY_200, ColorPalette::BLUE_500, ColorPalette::GRAY_400)
                        };
                        let gap = 3.0f32;
                        let cell_sz = 20.0f32;
                        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
                        for row in 0..8usize {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
                                for col in 0..8usize {
                                    let highlighted = row <= self.table_picker_hover.0 && col <= self.table_picker_hover.1;
                                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(cell_sz, cell_sz), egui::Sense::click());
                                    ui.painter().rect_filled(rect, 2.0, if highlighted { cell_hi } else { cell_def });
                                    ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, border), egui::StrokeKind::Middle);
                                    if resp.hovered() { self.table_picker_hover = (row, col); }
                                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                        self.insert_table(row + 1, col + 1);
                                        egui::Popup::close_id(ui.ctx(), tbl_popup_id);
                                    }
                                }
                            });
                        }
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(format!("{}x{} Table", self.table_picker_hover.1 + 1, self.table_picker_hover.0 + 1)).size(12.0));
                    });
                });

                ui.separator();
                ui.label("View:");
                ui.vertical(|ui: &mut egui::Ui| {
                    egui::ComboBox::from_id_salt("view_mode")
                        .selected_text(match self.view_mode { ViewMode::Markdown => "Markdown", ViewMode::Plain => "Plain Text" })
                        .show_ui(ui, |ui: &mut egui::Ui| {
                            ui.selectable_value(&mut self.view_mode, ViewMode::Markdown, "Markdown");
                            ui.selectable_value(&mut self.view_mode, ViewMode::Plain, "Plain Text");
                        });
                });

                ui.separator();
                ui.label("Font:");
                ui.vertical(|ui: &mut egui::Ui| {
                    let font_label = match self.font_family {
                        egui::FontFamily::Name(ref n) => match n.as_ref() { "Roboto" => "Roboto", "GoogleSans" => "Google Sans", "OpenSans" => "Open Sans", _ => "Ubuntu" },
                        _ => "Ubuntu",
                    };
                    egui::ComboBox::from_id_salt("font_fam")
                        .selected_text(font_label)
                        .show_ui(ui, |ui: &mut egui::Ui| {
                            ui.selectable_value(&mut self.font_family, egui::FontFamily::Name("Ubuntu".into()), "Ubuntu");
                            ui.selectable_value(&mut self.font_family, egui::FontFamily::Name("Roboto".into()), "Roboto");
                            ui.selectable_value(&mut self.font_family, egui::FontFamily::Name("GoogleSans".into()), "Google Sans");
                            ui.selectable_value(&mut self.font_family, egui::FontFamily::Name("OpenSans".into()), "Open Sans");
                        });
                });

                ui.separator();
                ui.label("Size:");
                ui.vertical(|ui: &mut egui::Ui| {
                    ui.add(egui::DragValue::new(&mut self.font_size).speed(0.5).range(8.0..=72.0));
                });
            });
            ui.separator();
        }

        if show_file_info {
            if self.show_word_count_in_info && self.word_count_display_version != self.content_version {
                self.modal_word_count = self.count_words();
                self.word_count_display_version = self.content_version;
            }
            ui.horizontal(|ui: &mut egui::Ui| {
                let is_dark: bool = ui.visuals().dark_mode;
                let file_label_resp = ui.add(
                    egui::Label::new(format!("File: {}", self.get_file_name()))
                        .sense(egui::Sense::click()),
                );
                file_label_resp.clone().on_hover_text("Right-click for file options");
                file_label_resp.context_menu(|ui: &mut egui::Ui| {
                    if ui.button("Open File Location").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.open_file_location();
                        ui.close();
                    }
                    if ui.button("Rename File").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        let current_ext = self.file_path.as_ref()
                            .and_then(|p| p.extension())
                            .and_then(|e| e.to_str())
                            .map(|e| e.to_lowercase())
                            .unwrap_or_else(|| "txt".to_string());
                        self.rename_buffer = self.file_path.as_ref()
                            .and_then(|p| p.file_stem())
                            .and_then(|s| s.to_str())
                            .unwrap_or("untitled")
                            .to_string();
                        self.rename_ext = Some(if current_ext == "md" { "md".to_string() } else { "txt".to_string() });
                        self.rename_modal_open = true;
                        ui.close();
                    }
                    let convert_label = match self.file_path.as_ref()
                        .and_then(|p| p.extension())
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_lowercase())
                        .as_deref()
                    {
                        Some("md") | Some("markdown") => Some("Convert to .txt"),
                        Some("txt") => Some("Convert to .md"),
                        _ => None,
                    };
                    if let Some(label) = convert_label {
                        if ui.button(label).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            self.convert_file_extension();
                            ui.close();
                        }
                    }
                });
                ui.separator();
                let (status, color) = if self.dirty {
                    ("Unsaved", if is_dark { ColorPalette::AMBER_400 } else { ColorPalette::AMBER_600 })
                } else {
                    ("Saved", if is_dark { ColorPalette::GREEN_400 } else { ColorPalette::GREEN_600 })
                };
                ui.label(egui::RichText::new(status).color(color));
                if self.show_word_count_in_info {
                    ui.separator();
                    ui.label(format!("Words: {}", self.modal_word_count));
                }
            });

            if self.rename_modal_open {
                let mut open = self.rename_modal_open;
                egui::Window::new("Rename File")
                    .collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .open(&mut open)
                    .show(ui.ctx(), |ui: &mut egui::Ui| {
                        ui.label("New filename:");
                        ui.horizontal(|ui: &mut egui::Ui| {
                            ui.text_edit_singleline(&mut self.rename_buffer);
                            let ext = self.rename_ext.get_or_insert_with(|| "txt".to_string());
                            egui::ComboBox::from_id_salt("rename_ext_cb")
                                .selected_text(format!(".{}", ext))
                                .width(60.0)
                                .show_ui(ui, |ui: &mut egui::Ui| {
                                    ui.selectable_value(ext, "txt".to_string(), ".txt");
                                    ui.selectable_value(ext, "md".to_string(), ".md");
                                });
                        });
                        ui.horizontal(|ui: &mut egui::Ui| {
                            if ui.button("Rename").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                self.apply_rename();
                                self.rename_modal_open = false;
                            }
                            if ui.button("Cancel").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                self.rename_modal_open = false;
                            }
                        });
                    });
                if !open { self.rename_modal_open = false; }
            }

            ui.separator();
        }

        match self.view_mode {
            ViewMode::Markdown => self.markdown_editable(ui, ctx),
            ViewMode::Plain => {
                egui::ScrollArea::vertical().show(ui, |ui: &mut egui::Ui| {
                    let font_id: egui::FontId = egui::FontId::new(self.font_size, self.font_family.clone());
                    let text_edit: egui::TextEdit<'_> = egui::TextEdit::multiline(&mut self.content)
                        .font(font_id).lock_focus(true).frame(false);
                    let response: egui::Response = ui.add_sized(ui.available_size(), text_edit);

                    if let Some(new_pos) = self.pending_cursor_pos.take() {
                        if let Some(mut state) = egui::TextEdit::load_state(ctx, response.id) {
                            let ccursor: egui::text::CCursor = egui::text::CCursor::new(new_pos);
                            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                            state.store(ctx, response.id);
                        }
                    }
                    if let Some(state) = egui::TextEdit::load_state(ctx, response.id) {
                        if let Some(r) = state.cursor.char_range() { self.last_cursor_range = Some(r); }
                    }
                    if response.changed() { self.dirty = true; self.content_version = self.content_version.wrapping_add(1); }
                });
            }
        }

        ctx.input_mut(|i: &mut egui::InputState| {
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::S) {
                if !i.modifiers.shift { let _ = self.save(); } else { self.format_strikethrough(); }
            }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::B) { self.format_bold(); }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::I) { self.format_italic(); }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::U) { self.format_underline(); }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::E) { self.format_code(); }
            if i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::A) { let _ = self.save_as(); }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num1) { self.format_heading(1); }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num2) { self.format_heading(2); }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num3) { self.format_heading(3); }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num4) { self.format_heading(4); }
            if i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Q) { self.format_blockquote(); }
            if i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::L) { self.insert_checklist_item(); }
        });

        if self.show_word_count_modal {
            let (bg, border, text, muted) = if ui.visuals().dark_mode {
                (ColorPalette::ZINC_900, ColorPalette::ZINC_700, ColorPalette::SLATE_200, ColorPalette::ZINC_400)
            } else {
                (egui::Color32::WHITE, ColorPalette::GRAY_200, ColorPalette::GRAY_800, ColorPalette::GRAY_500)
            };
            crate::style::draw_modal_overlay(ctx, "wc_overlay", 160);
            let mut open = self.show_word_count_modal;
            let win_resp = egui::Window::new("Word Count")
                .collapsible(false).resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.0, border)).corner_radius(10.0).inner_margin(24.0))
                .open(&mut open)
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    let row = |ui: &mut egui::Ui, label: &str, value: usize| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(label).size(13.0).color(muted));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(value.to_string()).size(13.0).color(text));
                            });
                        });
                    };
                    row(ui, "Words", self.modal_word_count);
                    ui.add_space(4.0);
                    row(ui, "Characters", self.modal_char_count);
                    ui.add_space(4.0);
                    row(ui, "Characters (no spaces)", self.modal_char_no_spaces);
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.checkbox(&mut self.show_word_count_in_info,
                        egui::RichText::new("Display word count in file information").size(12.0).color(text));
                });
            if let Some(r) = win_resp {
                let clicked_outside = ctx.input(|i| {
                    i.pointer.any_pressed() && i.pointer.interact_pos().map_or(false, |p| !r.response.rect.contains(p))
                });
                if clicked_outside { open = false; }
            }
            self.show_word_count_modal = open;
        }
    }

    pub(super) fn markdown_editable(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        use egui::{pos2, vec2, Rect, Sense};

        egui::ScrollArea::vertical().show(ui, |ui: &mut egui::Ui| {
            let font_size: f32 = self.font_size;
            let font_family: egui::FontFamily = self.font_family.clone();
            let cursor_pos: Option<usize> = self.last_cursor_range.map(|r| r.primary.index);
            let is_dark_mode: bool = ui.visuals().dark_mode;
            let available_width: f32 = ui.available_width();
            let top_padding: f32 = 2.0_f32;
            let wrap_width: f32 = available_width.max(10.0);

            let mut lines: Vec<&str> = Vec::new();
            let mut code_line_flags: Vec<bool> = Vec::new();
            let mut fence_line_flags: Vec<bool> = Vec::new();
            let mut blockquote_flags: Vec<bool> = Vec::new();
            let mut hrule_flags: Vec<bool> = Vec::new();
            let mut in_code_block = false;

            for line in self.content.lines() {
                let is_fence: bool = line.trim().starts_with("```");
                if is_fence { in_code_block = !in_code_block; }
                let is_code: bool = !is_fence && in_code_block;
                let is_blockquote: bool = !is_code && !is_fence && line.starts_with("> ");
                let is_hrule: bool = !is_code && !is_fence && Self::is_horizontal_rule(line);
                lines.push(line);
                code_line_flags.push(is_code);
                fence_line_flags.push(is_fence);
                blockquote_flags.push(is_blockquote);
                hrule_flags.push(is_hrule);
            }

            let mut table_line_flags: Vec<bool> = vec![false; lines.len()];
            let mut table_sep_flags:  Vec<bool> = vec![false; lines.len()];
            for (i, line) in lines.iter().enumerate() {
                if !code_line_flags[i] && !fence_line_flags[i] && Self::is_table_row(line) {
                    table_line_flags[i] = true;
                    if Self::is_separator_row(line) { table_sep_flags[i] = true; }
                }
            }

            let cursor_line_idx: Option<usize> = cursor_pos.map(|cp| {
                let mut acc = 0usize;
                for (i, line) in lines.iter().enumerate() {
                    let end = acc + line.chars().count();
                    if cp <= end { return i; }
                    acc = end + 1;
                }
                lines.len().saturating_sub(1)
            });

            let mut table_groups: Vec<(usize, usize, usize, usize, bool)> = Vec::new();
            {
                let mut ti = 0usize;
                while ti < lines.len() {
                    if table_line_flags[ti] {
                        let start = ti;
                        while ti < lines.len() && table_line_flags[ti] { ti += 1; }
                        let end = ti - 1;
                        if let Some(sep) = (start..=end).find(|&j| table_sep_flags[j]) {
                            if sep > start {
                                let col_count = lines[start..=end].iter()
                                    .filter(|l| !Self::is_separator_row(l))
                                    .map(|l| Self::parse_table_cells(l).len())
                                    .max().unwrap_or(1);
                                let cursor_in = cursor_line_idx.map_or(false, |cl| cl >= start && cl <= end);
                                table_groups.push((start, sep, end, col_count, cursor_in));
                            } else {
                                for j in start..=end { table_line_flags[j] = false; }
                            }
                        } else {
                            for j in start..=end { table_line_flags[j] = false; }
                        }
                    } else { ti += 1; }
                }
            }
            let table_nc_flags: Vec<bool> = {
                let mut f = vec![false; lines.len()];
                for &(start, _sep, end, _cols, cursor_in) in &table_groups {
                    if !cursor_in { for j in start..=end { f[j] = true; } }
                }
                f
            };
            let line_to_col_count: Vec<usize> = {
                let mut v = vec![1usize; lines.len()];
                for &(start, _sep, end, col_count, _) in &table_groups {
                    for j in start..=end { v[j] = col_count; }
                }
                v
            };

            let cache_valid = self.line_height_cache.as_ref().map_or(false, |c| {
                c.version == self.content_version
                    && c.font_size == font_size
                    && c.font_family == font_family
                    && c.wrap_width == wrap_width
                    && c.is_dark == is_dark_mode
                    && c.heights.len() == lines.len()
            });

            if !cache_valid {
                let mut per_line_row_heights: Vec<Vec<f32>> = Vec::with_capacity(lines.len());
            ui.fonts_mut(|fonts: &mut egui::epaint::FontsView<'_>| {
                for (idx, line) in lines.iter().enumerate() {
                    if table_line_flags[idx] {
                        let row_h = if Self::is_separator_row(line) {
                            2.0f32
                        } else {
                            let cells = Self::parse_table_cells(line);
                            let ncols = line_to_col_count[idx].max(1);
                            let cw = (available_width / ncols as f32 - 12.0).max(1.0);
                            let mut max_h = (font_size * 1.25).max(16.0);
                            for cell in &cells {
                                let mut cj = egui::text::LayoutJob::default();
                                cj.wrap.max_width = cw;
                                Self::parse_inline_formatting_static(cell, &mut cj, font_size, &font_family, None, 0, is_dark_mode);
                                if cj.sections.is_empty() { cj.append(cell, 0.0, egui::TextFormat { font_id: egui::FontId::new(font_size, font_family.clone()), ..Default::default() }); }
                                let g = fonts.layout_job(cj);
                                max_h = max_h.max(g.rect.height());
                            }
                            max_h + 12.0
                        };
                        per_line_row_heights.push(vec![row_h]);
                        continue;
                    }
                    let mut job: egui::text::LayoutJob = egui::text::LayoutJob::default();
                    job.wrap.max_width = wrap_width;
                    if fence_line_flags[idx] {
                        Self::append_fence_line_job(line, &mut job, font_size, is_dark_mode, false, cursor_pos, 0);
                    } else if code_line_flags[idx] {
                        job.append(line, 0.0, Self::code_block_background_format_static(font_size, is_dark_mode, available_width));
                    } else if line.trim().is_empty() {
                        job.append(line, 0.0, Self::default_format_static(font_size, &font_family, is_dark_mode));
                    } else {
                        Self::parse_markdown_line_static(line, &mut job, font_size, &font_family, cursor_pos, 0, is_dark_mode);
                    }
                    let galley: std::sync::Arc<egui::Galley> = fonts.layout_job(job);
                    let mut row_heights: Vec<f32> = galley.rows.iter().map(|r: &egui::epaint::text::PlacedRow| r.height()).collect();
                    if row_heights.is_empty() { row_heights.push((font_size * 1.25).max(16.0)); }
                    per_line_row_heights.push(row_heights);
                }
            });
                self.line_height_cache = Some(super::te_main::LineHeightCache {
                    version: self.content_version,
                    font_size,
                    font_family: font_family.clone(),
                    wrap_width,
                    is_dark: is_dark_mode,
                    heights: per_line_row_heights,
                });
            }

            let per_line_row_heights: Vec<Vec<f32>> = self.line_height_cache.as_ref().unwrap().heights.clone();

            let total_content_height: f32 = top_padding + per_line_row_heights.iter()
                .flat_map(|v| v.iter().copied())
                .sum::<f32>();
            let desired_size = egui::vec2(
                ui.available_width(),
                total_content_height.max(ui.available_height()),
            );
            let (outer_rect, _) = ui.allocate_exact_size(desired_size, Sense::click());
            let painter: &egui::Painter = ui.painter();
            let mut y: f32 = outer_rect.min.y + top_padding;
            let full_width: f32 = outer_rect.width().max(0.0);
            let line_start_y: Vec<f32> = {
                let mut out = Vec::with_capacity(lines.len());
                let mut ry = outer_rect.min.y + top_padding;
                for heights in &per_line_row_heights {
                    out.push(ry);
                    for &h in heights { ry += h; }
                }
                out
            };
            let code_bg: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_800 } else { ColorPalette::ZINC_200 };
            let blockquote_bg: egui::Color32 = if is_dark_mode {
                egui::Color32::from_rgba_unmultiplied(59, 130, 246, 15)
            } else {
                egui::Color32::from_rgba_unmultiplied(59, 130, 246, 10)
            };
            let blockquote_bar: egui::Color32 = if is_dark_mode { ColorPalette::BLUE_500 } else { ColorPalette::BLUE_400 };
            let hrule_color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_600 } else { ColorPalette::ZINC_400 };

            for (line_idx, row_heights) in per_line_row_heights.iter().enumerate() {
                if fence_line_flags[line_idx] || code_line_flags[line_idx] {
                    for &h in row_heights {
                        painter.rect_filled(Rect::from_min_size(pos2(outer_rect.min.x, y), vec2(full_width, h)), 0.0, code_bg);
                        y += h;
                    }
                } else if blockquote_flags[line_idx] {
                    for &h in row_heights {
                        painter.rect_filled(Rect::from_min_size(pos2(outer_rect.min.x, y), vec2(full_width, h)), 0.0, blockquote_bg);
                        painter.rect_filled(Rect::from_min_size(pos2(outer_rect.min.x, y), vec2(3.0, h)), 0.0, blockquote_bar);
                        y += h;
                    }
                } else if hrule_flags[line_idx] {
                    for &h in row_heights {
                        let mid_y: f32 = y + h * 0.5;
                        painter.hline(outer_rect.min.x..=outer_rect.max.x, mid_y, egui::Stroke::new(1.0, hrule_color));
                        y += h;
                    }
                } else {
                    for &h in row_heights { y += h; }
                }
            }

            {
                let tbl_header_bg: egui::Color32 = if is_dark_mode { egui::Color32::from_rgb(28, 33, 52) } else { egui::Color32::from_rgb(239, 246, 255) };
                let tbl_alt_bg: egui::Color32 = if is_dark_mode { egui::Color32::from_rgb(22, 22, 29) } else { egui::Color32::from_rgb(249, 250, 251) };
                let tbl_border: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_300 };
                let tbl_row_sep: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_300 };
                let tbl_text: egui::Color32 = if is_dark_mode { ColorPalette::SLATE_200 } else { ColorPalette::GRAY_800 };
                let tbl_hdr_text: egui::Color32 = if is_dark_mode { ColorPalette::SLATE_100 } else { ColorPalette::GRAY_900 };
                let tbl_font: egui::FontId = egui::FontId::new(font_size, font_family.clone());
                let tbl_font_bold: egui::FontId = egui::FontId::new(font_size, Self::bold_family(&font_family));

                for &(start, sep, end, col_count, cursor_in) in &table_groups {
                    if cursor_in { continue; }

                    let x = outer_rect.min.x;
                    let cw = (full_width / col_count as f32).max(1.0);
                    let hdr_h: f32 = per_line_row_heights[start].iter().sum();
                    let sep_h: f32 = per_line_row_heights[sep].iter().sum();
                    let tsy = line_start_y[start];
                    let tey = line_start_y[end] + per_line_row_heights[end].iter().sum::<f32>();
                    let data_y = tsy + hdr_h + sep_h;
                    let cr = egui::CornerRadius::same(4);

                    painter.rect_filled(
                        Rect::from_min_max(pos2(x, tsy), pos2(x + full_width, data_y)),
                        egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 },
                        tbl_header_bg,
                    );

                    let data_lines: Vec<usize> = (start..=end).filter(|&j| j != start && j != sep).collect();
                    for (ri, &lj) in data_lines.iter().enumerate() {
                        if ri % 2 == 1 {
                            let ry2 = line_start_y[lj];
                            let rh: f32 = per_line_row_heights[lj].iter().sum();
                            painter.rect_filled(
                                Rect::from_min_max(pos2(x, ry2), pos2(x + full_width, ry2 + rh)),
                                if lj == end { egui::CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 } } else { egui::CornerRadius::same(0) },
                                tbl_alt_bg,
                            );
                        }
                    }

                    painter.hline(x..=x + full_width, data_y, egui::Stroke::new(1.0, tbl_border));
                    for col in 1..col_count {
                        painter.vline(x + cw * col as f32, tsy..=tey, egui::Stroke::new(1.0, tbl_border));
                    }
                    painter.rect_stroke(
                        Rect::from_min_max(pos2(x, tsy), pos2(x + full_width, tey)),
                        cr,
                        egui::Stroke::new(1.0, tbl_border),
                        egui::StrokeKind::Middle,
                    );

                    for (ci, cell) in Self::parse_table_cells(lines[start]).into_iter().enumerate().take(col_count) {
                        let cell_x = x + cw * ci as f32 + 6.0;
                        let cell_w = (cw - 12.0).max(1.0);
                        let clip = Rect::from_min_max(pos2(cell_x, tsy + 2.0), pos2(cell_x + cell_w, tsy + hdr_h - 2.0));
                        let bold_fam = Self::bold_family(&font_family);
                        let mut job = egui::text::LayoutJob::default();
                        job.wrap.max_width = cell_w;
                        Self::parse_inline_formatting_static(&cell, &mut job, font_size, &bold_fam, None, 0, is_dark_mode);
                        if job.sections.is_empty() { job.append(&cell, 0.0, egui::TextFormat { font_id: tbl_font_bold.clone(), color: tbl_hdr_text, ..Default::default() }); }
                        let galley = painter.ctx().fonts_mut(|f| f.layout_job(job));
                        let text_x = cell_x + ((cell_w - galley.rect.width()).max(0.0) / 2.0);
                        let text_y = tsy + ((hdr_h - galley.rect.height()).max(0.0) / 2.0);
                        painter.with_clip_rect(clip).galley(pos2(text_x, text_y), galley, tbl_hdr_text);
                    }

                    for (ri, &lj) in data_lines.iter().enumerate() {
                        let ry2 = line_start_y[lj];
                        let rh: f32 = per_line_row_heights[lj].iter().sum();
                        if ri > 0 {
                            painter.hline(x..=x + full_width, ry2, egui::Stroke::new(1.0, tbl_row_sep));
                        }
                        for (ci, cell) in Self::parse_table_cells(lines[lj]).into_iter().enumerate().take(col_count) {
                            let cell_x = x + cw * ci as f32 + 6.0;
                            let cell_w = (cw - 12.0).max(1.0);
                            let mut job = egui::text::LayoutJob::default();
                            job.wrap.max_width = cell_w;
                            Self::parse_inline_formatting_static(&cell, &mut job, font_size, &font_family, None, 0, is_dark_mode);
                            if job.sections.is_empty() { job.append(&cell, 0.0, egui::TextFormat { font_id: tbl_font.clone(), color: tbl_text, ..Default::default() }); }
                            let galley = painter.ctx().fonts_mut(|f| f.layout_job(job));
                            let text_y = ry2 + ((rh - galley.rect.height()).max(0.0) / 2.0);
                            let clip = Rect::from_min_max(pos2(cell_x, ry2 + 2.0), pos2(cell_x + cell_w, ry2 + rh - 2.0));
                            painter.with_clip_rect(clip).galley(pos2(cell_x, text_y), galley, tbl_text);
                        }
                    }
                }
            }

            let mut layouter = |ui: &egui::Ui, text_buffer: &dyn egui::TextBuffer, wrap_width_closure: f32| {
                let text: &str = text_buffer.as_str();
                let mut job: egui::text::LayoutJob = egui::text::LayoutJob::default();
                job.wrap.max_width = wrap_width_closure;
                let mut char_offset: usize = 0;
                let mut in_code_block: bool = false;
                let lines_vec: Vec<&str> = text.lines().collect();
                let ends_with_newline: bool = text.ends_with('\n');

                for (line_idx, line) in lines_vec.iter().enumerate() {
                    let is_last_line: bool = line_idx == lines_vec.len() - 1;
                    if line.trim().starts_with("```") {
                        in_code_block = !in_code_block;
                        let marker_end: usize = char_offset + line.chars().count();
                        let cursor_in_range: bool = cursor_pos.map_or(false, |p: usize| p >= char_offset && p <= marker_end);
                        if cursor_in_range {
                            job.append(line, 0.0, Self::markdown_syntax_format_static(font_size, &font_family));
                        } else {
                            Self::append_fence_line_job(line, &mut job, font_size, is_dark_mode, true, cursor_pos, char_offset);
                        }
                    } else if in_code_block {
                        job.append(line, 0.0, Self::code_block_background_format_static(font_size, is_dark_mode, available_width));
                    } else if table_nc_flags.get(line_idx).copied().unwrap_or(false) {
                        let target_h = per_line_row_heights
                            .get(line_idx)
                            .and_then(|v| v.first())
                            .copied()
                            .unwrap_or(font_size * 1.25);
                        job.append(line, 0.0, egui::TextFormat {
                            font_id: egui::FontId::new(font_size, font_family.clone()),
                            color: egui::Color32::TRANSPARENT,
                            line_height: Some(target_h),
                            ..Default::default()
                        });
                    } else {
                        Self::parse_markdown_line_static(line, &mut job, font_size, &font_family, cursor_pos, char_offset, is_dark_mode);
                    }

                    if !is_last_line || ends_with_newline {
                        job.append("\n", 0.0, Self::default_format_static(font_size, &font_family, is_dark_mode));
                    }
                    char_offset += line.chars().count() + 1;
                }
                ui.fonts_mut(|f: &mut egui::epaint::FontsView<'_>| f.layout_job(job))
            };

            let text_edit: egui::TextEdit<'_> = egui::TextEdit::multiline(&mut self.content).layouter(&mut layouter).lock_focus(true).frame(false);
            let response: egui::Response = ui.put(outer_rect, text_edit);
            if response.clicked() && ctx.input(|i: &egui::InputState| i.modifiers.ctrl || i.modifiers.command) {
                if let Some(cursor_range) = self.last_cursor_range {
                    let chars: Vec<char> = self.content.chars().collect();
                    if let Some(url) = Self::find_link_at_offset(&chars, cursor_range.primary.index) {
                        let final_url: String = if url.starts_with("http://") || url.starts_with("https://") { url } else { format!("https://{}", url) };
                        ctx.open_url(egui::OpenUrl::new_tab(&final_url));
                    }
                }
            }

            if response.clicked() && !ctx.input(|i: &egui::InputState| i.modifiers.ctrl || i.modifiers.command) {
                self.try_toggle_checkbox();
            }

            if let Some(new_pos) = self.pending_cursor_pos.take() {
                if let Some(mut state) = egui::TextEdit::load_state(ctx, response.id) {
                    let ccursor: egui::text::CCursor = egui::text::CCursor::new(new_pos);
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                    state.store(ctx, response.id);
                }
            }
            if let Some(state) = egui::TextEdit::load_state(ctx, response.id) {
                if let Some(r) = state.cursor.char_range() { self.last_cursor_range = Some(r); }
            }
            if response.changed() { self.dirty = true; self.content_version = self.content_version.wrapping_add(1); }
        });
    }

    fn is_table_row(line: &str) -> bool {
        let t = line.trim();
        t.starts_with('|') && t.len() > 1
    }

    fn is_separator_row(line: &str) -> bool {
        let t = line.trim();
        Self::is_table_row(t) && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
    }

    fn parse_table_cells(line: &str) -> Vec<String> {
    line.trim().trim_matches('|').split('|').map(|c| c.trim().to_string()).collect()
}

    fn append_fence_line_job(
        line: &str,
        job: &mut egui::text::LayoutJob,
        font_size: f32,
        is_dark_mode: bool,
        in_layouter: bool,
        _cursor_pos: Option<usize>,
        _char_offset: usize,
    ) {
        if let Some(start_idx) = line.find("```") {
            let prefix: &str = &line[..start_idx];
            let rest: &str = &line[start_idx + 3..];
            let label: &str = rest.trim_end();
            let suffix_len: usize = rest.len() - label.len();
            let has_label: bool = !label.is_empty();

            if !prefix.is_empty() {
                job.append(prefix, 0.0, Self::transparent_format_static(font_size));
            }

            let marker_fmt: egui::TextFormat = if has_label { Self::zero_width_format_static() } else { Self::transparent_format_static(font_size) };
            job.append("```", 0.0, marker_fmt);

            if has_label {
                job.append(label, 0.0, Self::code_block_label_format_static(font_size, is_dark_mode));
            }
            if suffix_len > 0 {
                let suffix: &str = &rest[label.len()..];
                if in_layouter { job.append(suffix, 0.0, Self::zero_width_format_static()); }
                else { job.append(suffix, 0.0, Self::transparent_format_static(font_size)); }
            }
        } else {
            job.append(line, 0.0, Self::transparent_format_static(font_size));
        }
    }

    pub(super) fn parse_markdown_line_static(
        line: &str,
        job: &mut egui::text::LayoutJob,
        font_size: f32,
        font_family: &egui::FontFamily,
        cursor_pos: Option<usize>,
        line_start_offset: usize,
        is_dark_mode: bool,
    ) {
        if Self::is_horizontal_rule(line) {
            job.append(line, 0.0, Self::transparent_format_static(font_size));
            return;
        }

        if let Some(rest) = line.strip_prefix("> ") {
            let cursor_in_line: bool = cursor_pos.map_or(false, |p| {
                p >= line_start_offset && p <= line_start_offset + line.chars().count()
            });
            if cursor_in_line {
                job.append("> ", 0.0, Self::markdown_syntax_format_static(font_size, font_family));
            } else {
                job.append("> ", 0.0, Self::invisible_format_static());
            }
            Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + 2, is_dark_mode);
            return;
        }

        let headings: &[(&str, f32)] = &[("#### ", 1.1), ("### ", 1.2), ("## ", 1.4), ("# ", 1.6)];
        for (prefix, scale) in headings {
            if let Some(rest) = line.strip_prefix(prefix) {
                let header_end: usize = line_start_offset + line.chars().count();
                let cursor_in: bool = cursor_pos.map_or(false, |p: usize| p >= line_start_offset && p <= header_end);
                if cursor_in {
                    job.append(prefix, 0.0, Self::markdown_syntax_format_static(font_size, font_family));
                    job.append(rest, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                } else {
                    job.append(prefix, 0.0, Self::invisible_format_static());
                    job.append(rest, 0.0, Self::heading_format_static(font_size, *scale, font_family, is_dark_mode));
                }
                return;
            }
        }

        let indent_count: usize = line.len() - line.trim_start_matches(' ').len();
        if indent_count >= 2 {
            let trimmed_line: &str = &line[indent_count..];
            let indent_visual: String = " ".repeat(indent_count);

            let checkbox_variants: &[(&str, bool)] = &[
                ("- [ ] ", false), ("- [x] ", true), ("- [X] ", true),
                ("* [ ] ", false), ("* [x] ", true), ("* [X] ", true),
                ("+ [ ] ", false), ("+ [x] ", true), ("+ [X] ", true),
            ];
            for (prefix, checked) in checkbox_variants {
                if let Some(rest) = trimmed_line.strip_prefix(prefix) {
                    job.append(&indent_visual, 0.0, Self::transparent_format_static(font_size));
                    job.append("\u{2022} ", 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                    Self::append_checkbox_marker(job, *checked, font_size, font_family, is_dark_mode);
                    let content_offset: usize = line_start_offset + indent_count + prefix.chars().count();
                    Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, content_offset, is_dark_mode);
                    return;
                }
            }
            for prefix in &["- ", "* ", "+ "] {
                if let Some(rest) = trimmed_line.strip_prefix(prefix) {
                    job.append(&indent_visual, 0.0, Self::transparent_format_static(font_size));
                    job.append("\u{2022} ", 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                    Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + indent_count + 2, is_dark_mode);
                    return;
                }
            }
        }

        let checkbox_prefixes: &[(&str, bool)] = &[
            ("- [ ] ", false), ("- [x] ", true), ("- [X] ", true),
            ("* [ ] ", false), ("* [x] ", true), ("* [X] ", true),
            ("+ [ ] ", false), ("+ [x] ", true), ("+ [X] ", true),
        ];
        for (prefix, checked) in checkbox_prefixes {
            if let Some(rest) = line.strip_prefix(prefix) {
                job.append("\u{2022} ", 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                Self::append_checkbox_marker(job, *checked, font_size, font_family, is_dark_mode);
                let content_offset: usize = line_start_offset + prefix.chars().count();
                Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, content_offset, is_dark_mode);
                return;
            }
        }

        let list_prefixes: &[&str] = &["- ", "* ", "+ "];
        for prefix in list_prefixes {
            if let Some(rest) = line.strip_prefix(prefix) {
                job.append("\u{2022} ", 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + 2, is_dark_mode);
                return;
            }
        }

        let mut k: usize = 0;
        let line_chars: Vec<char> = line.chars().collect();
        while k < line_chars.len() && line_chars[k].is_ascii_digit() { k += 1; }
        if k > 0 && line_chars.get(k) == Some(&'.') && line_chars.get(k + 1) == Some(&' ') {
            let split_at: usize = line.char_indices().nth(k + 2).map(|(i, _)| i).unwrap_or(line.len());
            let (prefix, rest) = line.split_at(split_at);
            job.append(prefix, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
            Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + prefix.chars().count(), is_dark_mode);
            return;
        }

        Self::parse_inline_formatting_static(line, job, font_size, font_family, cursor_pos, line_start_offset, is_dark_mode);
    }

    fn append_checkbox_marker(
        job: &mut egui::text::LayoutJob,
        checked: bool,
        font_size: f32,
        font_family: &egui::FontFamily,
        is_dark_mode: bool,
    ) {
        let fmt: egui::TextFormat = Self::checkbox_format_static(checked, font_size, font_family, is_dark_mode);
        let marker: &str = if checked { "[x] " } else { "[ ] " };
        job.append(marker, 0.0, fmt);
    }

    pub(super) fn parse_inline_formatting_static(
        text: &str,
        job: &mut egui::text::LayoutJob,
        font_size: f32,
        font_family: &egui::FontFamily,
        cursor_pos: Option<usize>,
        text_start_offset: usize,
        is_dark_mode: bool,
    ) {
        Self::parse_inline_with_context(text, job, font_size, font_family, cursor_pos, text_start_offset, is_dark_mode, false, false);
    }

    fn parse_inline_with_context(
        text: &str,
        job: &mut egui::text::LayoutJob,
        font_size: f32,
        font_family: &egui::FontFamily,
        cursor_pos: Option<usize>,
        text_start_offset: usize,
        is_dark_mode: bool,
        is_bold: bool,
        is_italic: bool,
    ) {
        let chars: Vec<char> = text.chars().collect();
        let mut i: usize = 0;
        let mut current_text: String = String::new();

        let ctx_format = |b: bool, it: bool| -> egui::TextFormat {
            match (b, it) {
                (true, true)  => Self::bold_italic_format_static(font_size, font_family, is_dark_mode),
                (true, false) => Self::bold_format_static(font_size, font_family, is_dark_mode),
                (false, true) => Self::italic_format_static(font_size, font_family, is_dark_mode),
                (false, false) => Self::default_format_static(font_size, font_family, is_dark_mode),
            }
        };

        let flush = |current_text: &mut String, job: &mut egui::text::LayoutJob| {
            if !current_text.is_empty() {
                job.append(current_text, 0.0, ctx_format(is_bold, is_italic));
                current_text.clear();
            }
        };

        let is_valid_start = |pos: usize, mlen: usize| -> bool {
            if pos + mlen >= chars.len() { return false; }
            (pos == 0 || chars[pos - 1].is_whitespace()) && !chars[pos + mlen].is_whitespace()
        };

        let cursor_in = |current_pos: usize, marker_end: usize| -> bool {
            cursor_pos.map_or(false, |p: usize| p >= current_pos && p < text_start_offset + marker_end)
        };

        while i < chars.len() {
            let current_pos: usize = text_start_offset + i;
            if i + 1 < chars.len() && chars[i] == '~' && chars[i + 1] == '~' && is_valid_start(i, 2) {
                if let Some(end) = Self::find_closing_marker(&chars, i + 2, "~~") {
                    if end > i + 2 {
                        let end_pos = end + 2;
                        flush(&mut current_text, job);
                        if cursor_in(current_pos, end_pos) {
                            let r: String = chars[i..end_pos].iter().collect();
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size, font_family));
                        } else {
                            job.append("~~", 0.0, Self::invisible_format_static());
                            let content: String = chars[i + 2..end].iter().collect();
                            job.append(&content, 0.0, Self::strikethrough_format_static(font_size, font_family, is_dark_mode));
                            job.append("~~", 0.0, Self::invisible_format_static());
                        }
                        i = end_pos; continue;
                    }
                }
            }

            if chars[i] == '~' && i + 1 < chars.len() && !chars[i + 1].is_whitespace() && chars[i + 1] != '~' {
                if let Some(end) = Self::find_closing_marker(&chars, i + 1, "~") {
                    let content_chars: &[char] = &chars[i + 1..end];
                    let valid: bool = end > i + 1
                        && !content_chars.is_empty()
                        && !content_chars.iter().any(|&c| c.is_whitespace())
                        && !(end + 1 < chars.len() && chars[end + 1] == '~');
                    if valid {
                        let end_pos: usize = end + 1;
                        flush(&mut current_text, job);
                        if cursor_pos.map_or(false, |p| p >= current_pos && p < text_start_offset + end_pos) {
                            let r: String = chars[i..end_pos].iter().collect();
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size, font_family));
                        } else {
                            job.append("~", 0.0, Self::invisible_format_static());
                            let content: String = content_chars.iter().collect();
                            job.append(&content, 0.0, Self::subscript_format_static(font_size, font_family, is_dark_mode));
                            job.append("~", 0.0, Self::invisible_format_static());
                        }
                        i = end_pos; continue;
                    }
                }
            }

            if !is_bold && !is_italic
                && i + 2 < chars.len()
                && chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '*'
                && is_valid_start(i, 3)
            {
                if let Some(end) = Self::find_closing_marker(&chars, i + 3, "***") {
                    if end > i + 3 {
                        let end_pos: usize = end + 3;
                        flush(&mut current_text, job);
                        if cursor_in(current_pos, end_pos) {
                            let r: String = chars[i..end_pos].iter().collect();
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size, font_family));
                        } else {
                            job.append("***", 0.0, Self::invisible_format_static());
                            let content: String = chars[i + 3..end].iter().collect();
                            Self::parse_inline_with_context(
                                &content, job, font_size, font_family, cursor_pos,
                                text_start_offset + i + 3, is_dark_mode, true, true,
                            );
                            job.append("***", 0.0, Self::invisible_format_static());
                        }
                        i = end_pos; continue;
                    }
                }
            }

            if !is_bold
                && i + 1 < chars.len()
                && chars[i] == '*' && chars[i + 1] == '*'
                && !(i + 2 < chars.len() && chars[i + 2] == '*')
                && is_valid_start(i, 2)
            {
                if let Some(end) = Self::find_closing_marker(&chars, i + 2, "**") {
                    if end > i + 2 {
                        let end_pos: usize = end + 2;
                        flush(&mut current_text, job);
                        if cursor_in(current_pos, end_pos) {
                            let r: String = chars[i..end_pos].iter().collect();
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size, font_family));
                        } else {
                            job.append("**", 0.0, Self::invisible_format_static());
                            let content: String = chars[i + 2..end].iter().collect();
                            Self::parse_inline_with_context(
                                &content, job, font_size, font_family, cursor_pos,
                                text_start_offset + i + 2, is_dark_mode, true, is_italic,
                            );
                            job.append("**", 0.0, Self::invisible_format_static());
                        }
                        i = end_pos; continue;
                    }
                }
            }

            if !is_italic
                && chars[i] == '*'
                && !(i + 1 < chars.len() && chars[i + 1] == '*')
                && is_valid_start(i, 1)
            {
                if let Some(end) = Self::find_closing_marker(&chars, i + 1, "*") {
                    if end > i + 1 {
                        let end_pos = end + 1;
                        flush(&mut current_text, job);
                        if cursor_in(current_pos, end_pos) {
                            let r: String = chars[i..end_pos].iter().collect();
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size, font_family));
                        } else {
                            job.append("*", 0.0, Self::invisible_format_static());
                            let content: String = chars[i + 1..end].iter().collect();
                            Self::parse_inline_with_context(
                                &content, job, font_size, font_family, cursor_pos,
                                text_start_offset + i + 1, is_dark_mode, is_bold, true,
                            );
                            job.append("*", 0.0, Self::invisible_format_static());
                        }
                        i = end_pos; continue;
                    }
                }
            }

            if i + 1 < chars.len() && chars[i] == '_' && chars[i + 1] == '_' && is_valid_start(i, 2) {
                if let Some(end) = Self::find_closing_marker(&chars, i + 2, "__") {
                    if end > i + 2 {
                        let end_pos: usize = end + 2;
                        flush(&mut current_text, job);
                        if cursor_in(current_pos, end_pos) {
                            let r: String = chars[i..end_pos].iter().collect();
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size, font_family));
                        } else {
                            job.append("__", 0.0, Self::invisible_format_static());
                            let content: String = chars[i + 2..end].iter().collect();
                            job.append(&content, 0.0, Self::underline_format_static(font_size, font_family, is_dark_mode));
                            job.append("__", 0.0, Self::invisible_format_static());
                        }
                        i = end_pos; continue;
                    }
                }
            }

            if chars[i] == '`' && is_valid_start(i, 1) {
                if i + 2 < chars.len() && chars[i + 1] == '`' && chars[i + 2] == '`' {
                    current_text.push(chars[i]); i += 1; continue;
                }
                if let Some(end) = Self::find_closing_marker(&chars, i + 1, "`") {
                    if end > i + 1 {
                        let end_pos: usize = end + 1;
                        flush(&mut current_text, job);
                        if cursor_in(current_pos, end_pos) {
                            let r: String = chars[i..end_pos].iter().collect();
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size, font_family));
                        } else {
                            job.append("`", 0.0, Self::invisible_format_static());
                            let content: String = chars[i + 1..end].iter().collect();
                            job.append(&content, 0.0, Self::code_format_static(font_size, is_dark_mode));
                            job.append("`", 0.0, Self::invisible_format_static());
                        }
                        i = end_pos; continue;
                    }
                }
            }

            if chars[i] == '^' && i + 1 < chars.len() && !chars[i + 1].is_whitespace() {
                if let Some(end) = Self::find_closing_marker(&chars, i + 1, "^") {
                    let content_chars: &[char] = &chars[i + 1..end];
                    let valid: bool = end > i + 1
                        && !content_chars.is_empty()
                        && !content_chars.iter().any(|&c| c.is_whitespace());
                    if valid {
                        let end_pos: usize = end + 1;
                        flush(&mut current_text, job);
                        if cursor_pos.map_or(false, |p| p >= current_pos && p < text_start_offset + end_pos) {
                            let r: String = chars[i..end_pos].iter().collect();
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size, font_family));
                        } else {
                            job.append("^", 0.0, Self::invisible_format_static());
                            let content: String = content_chars.iter().collect();
                            job.append(&content, 0.0, Self::superscript_format_static(font_size, font_family, is_dark_mode));
                            job.append("^", 0.0, Self::invisible_format_static());
                        }
                        i = end_pos; continue;
                    }
                }
            }

            if chars[i] == '[' {
                if let Some(text_end) = Self::find_closing_bracket(&chars, i + 1) {
                    if text_end + 1 < chars.len() && chars[text_end + 1] == '(' {
                        if let Some(url_end) = Self::find_closing_paren(&chars, text_end + 2) {
                            let end_pos: usize = url_end + 1;
                            flush(&mut current_text, job);
                            if cursor_pos.map_or(false, |p| p >= current_pos && p <= text_start_offset + end_pos) {
                                let r: String = chars[i..end_pos].iter().collect();
                                job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size, font_family));
                            } else {
                                job.append("[", 0.0, Self::invisible_format_static());
                                let link_text: String = chars[i + 1..text_end].iter().collect();
                                job.append(&link_text, 0.0, Self::link_format_static(font_size, font_family));
                                let tail: String = chars[text_end..end_pos].iter().collect();
                                job.append(&tail, 0.0, Self::invisible_format_static());
                            }
                            i = end_pos; continue;
                        }
                    }
                }
            }

            current_text.push(chars[i]);
            i += 1;
        }

        if !current_text.is_empty() {
            job.append(&current_text, 0.0, ctx_format(is_bold, is_italic));
        }
    }

    pub(super) fn invisible_format_static() -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(0.001, egui::FontFamily::Name("Ubuntu".into())),
            color: egui::Color32::TRANSPARENT,
            ..Default::default()
        }
    }

    pub(super) fn transparent_format_static(font_size: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Name("Ubuntu".into())),
            color: egui::Color32::TRANSPARENT,
            ..Default::default()
        }
    }

    pub(super) fn zero_width_format_static() -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(0.01, egui::FontFamily::Name("Ubuntu".into())),
            color: egui::Color32::TRANSPARENT,
            ..Default::default()
        }
    }

    pub(super) fn default_format_static(font_size: f32, font_family: &egui::FontFamily, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_300 } else { ColorPalette::ZINC_700 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, font_family.clone()),
            color,
            ..Default::default()
        }
    }

    pub(super) fn bold_format_static(font_size: f32, font_family: &egui::FontFamily, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_200 } else { ColorPalette::ZINC_800 };
        let bold_family = Self::bold_family(font_family);
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, bold_family),
            color,
            ..Default::default()
        }
    }

    pub(super) fn italic_format_static(font_size: f32, font_family: &egui::FontFamily, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_300 } else { ColorPalette::ZINC_700 };
        let italic_family = Self::italic_family(font_family);
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, italic_family),
            color,
            ..Default::default()
        }
    }

    pub(super) fn underline_format_static(font_size: f32, font_family: &egui::FontFamily, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_300 } else { ColorPalette::ZINC_700 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, font_family.clone()),
            underline: egui::Stroke::new(1.0, ColorPalette::ZINC_500),
            color,
            ..Default::default()
        }
    }

    pub(super) fn strikethrough_format_static(font_size: f32, font_family: &egui::FontFamily, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_300 } else { ColorPalette::ZINC_700 };
        let strikethrough_color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_900 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, font_family.clone()),
            strikethrough: egui::Stroke::new(font_size/8.0, strikethrough_color),
            color,
            ..Default::default()
        }
    }

    pub(super) fn code_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let (bg_color, text_color) = if is_dark_mode {
            (ColorPalette::ZINC_800, ColorPalette::AMBER_400)
        } else {
            (ColorPalette::ZINC_200, ColorPalette::AMBER_600)
        };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 0.9, egui::FontFamily::Name("Ubuntu".into())),
            background: bg_color,
            color: text_color,
            ..Default::default()
        }
    }

    pub(super) fn code_block_background_format_static(font_size: f32, is_dark_mode: bool, _available_width: f32) -> egui::TextFormat {
        let text_color: egui::Color32 = if is_dark_mode { ColorPalette::SLATE_300 } else { ColorPalette::ZINC_800 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Name("Ubuntu".into())),
            color: text_color,
            ..Default::default()
        }
    }

    pub(super) fn code_block_label_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let text_color: egui::Color32 = if is_dark_mode { ColorPalette::BLUE_400 } else { ColorPalette::BLUE_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 0.7, egui::FontFamily::Name("Ubuntu".into())),
            color: text_color,
            ..Default::default()
        }
    }

    pub(super) fn superscript_format_static(font_size: f32, font_family: &egui::FontFamily, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_300 } else { ColorPalette::ZINC_700 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 0.7, font_family.clone()),
            valign: egui::Align::TOP,
            color,
            ..Default::default()
        }
    }

    pub(super) fn subscript_format_static(font_size: f32, font_family: &egui::FontFamily, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_300 } else { ColorPalette::ZINC_700 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 0.7, font_family.clone()),
            valign: egui::Align::BOTTOM,
            color,
            ..Default::default()
        }
    }

    pub(super) fn link_format_static(font_size: f32, font_family: &egui::FontFamily) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, font_family.clone()),
            underline: egui::Stroke::new(1.0, ColorPalette::BLUE_500),
            color: ColorPalette::BLUE_500,
            ..Default::default()
        }
    }

    pub(super) fn markdown_syntax_format_static(font_size: f32, font_family: &egui::FontFamily) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, font_family.clone()),
            color: ColorPalette::ZINC_500,
            ..Default::default()
        }
    }

    pub(super) fn checkbox_format_static(checked: bool, font_size: f32, _font_family: &egui::FontFamily, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if checked {
            if is_dark_mode { ColorPalette::GREEN_400 } else { ColorPalette::GREEN_600 }
        } else {
            if is_dark_mode { ColorPalette::ZINC_500 } else { ColorPalette::ZINC_400 }
        };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Name("Ubuntu".into())),
            color,
            ..Default::default()
        }
    }

    pub(super) fn heading_format_static(font_size: f32, scale: f32, font_family: &egui::FontFamily, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_200 } else { ColorPalette::ZINC_800 };
        let bold_family = Self::bold_family(font_family);
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * scale, bold_family),
            color,
            ..Default::default()
        }
    }

    fn bold_family(font_family: &egui::FontFamily) -> egui::FontFamily {
        match font_family {
            egui::FontFamily::Name(n) if n.as_ref() == "Roboto" => egui::FontFamily::Name("Roboto-Bold".into()),
            egui::FontFamily::Name(n) if n.as_ref() == "GoogleSans" => egui::FontFamily::Name("GoogleSans-Bold".into()),
            egui::FontFamily::Name(n) if n.as_ref() == "OpenSans" => egui::FontFamily::Name("OpenSans-Bold".into()),
            _ => egui::FontFamily::Name("Ubuntu-Bold".into()),
        }
    }

    fn italic_family(font_family: &egui::FontFamily) -> egui::FontFamily {
        match font_family {
            egui::FontFamily::Name(n) if n.as_ref() == "Roboto" => egui::FontFamily::Name("Roboto-Italic".into()),
            egui::FontFamily::Name(n) if n.as_ref() == "GoogleSans" => egui::FontFamily::Name("GoogleSans-Italic".into()),
            egui::FontFamily::Name(n) if n.as_ref() == "OpenSans" => egui::FontFamily::Name("OpenSans-Italic".into()),
            _ => egui::FontFamily::Name("Ubuntu-Italic".into()),
        }
    }

    fn bold_italic_family(font_family: &egui::FontFamily) -> egui::FontFamily {
        match font_family {
            egui::FontFamily::Name(n) if n.as_ref() == "Roboto" => egui::FontFamily::Name("Roboto-BoldItalic".into()),
            egui::FontFamily::Name(n) if n.as_ref() == "GoogleSans" => egui::FontFamily::Name("GoogleSans-BoldItalic".into()),
            egui::FontFamily::Name(n) if n.as_ref() == "OpenSans" => egui::FontFamily::Name("OpenSans-BoldItalic".into()),
            _ => egui::FontFamily::Name("Ubuntu-BoldItalic".into()),
        }
    }

    pub(super) fn bold_italic_format_static(font_size: f32, font_family: &egui::FontFamily, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_200 } else { ColorPalette::ZINC_800 };
        let bi_family: egui::FontFamily = Self::bold_italic_family(font_family);
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, bi_family),
            color,
            ..Default::default()
        }
    }
}
