use eframe::egui;
use crate::{modules::EditorModule, style::ColorPalette};
use super::te_main::{TextEditor, ViewMode};

impl TextEditor {
    pub(super) fn render_editor_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, show_toolbar: bool, show_file_info: bool) {
        if show_toolbar {
            ui.horizontal(|ui: &mut egui::Ui| {
                ui.horizontal(|ui: &mut egui::Ui| {
                    if ui.button(egui::RichText::new("B").strong()).on_hover_text("Bold (Ctrl+B)").clicked() { self.format_bold(); }
                    if ui.button(egui::RichText::new("I").italics()).on_hover_text("Italic (Ctrl+I)").clicked() { self.format_italic(); }
                    if ui.button(egui::RichText::new("U").underline()).on_hover_text("Underline (Ctrl+U)").clicked() { self.format_underline(); }
                    if ui.button(egui::RichText::new("S").strikethrough()).on_hover_text("Strikethrough (Ctrl+Shift+S)").clicked() { self.format_strikethrough(); }
                    if ui.button(egui::RichText::new("C").monospace()).on_hover_text("Code (Ctrl+E)").clicked() { self.format_code(); }
                    ui.separator();
                    if ui.button("H1").on_hover_text("Header 1 (Ctrl+1)").clicked() { self.format_heading(1); }
                    if ui.button("H2").on_hover_text("Header 2 (Ctrl+2)").clicked() { self.format_heading(2); }
                    if ui.button("H3").on_hover_text("Header 3 (Ctrl+3)").clicked() { self.format_heading(3); }
                    if ui.button("H4").on_hover_text("Header 4 (Ctrl+4)").clicked() { self.format_heading(4); }
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
                    egui::ComboBox::from_id_salt("font_fam")
                        .selected_text(if matches!(self.font_family, egui::FontFamily::Proportional) { "Sans" } else { "Mono" })
                        .show_ui(ui, |ui: &mut egui::Ui| {
                            ui.selectable_value(&mut self.font_family, egui::FontFamily::Monospace, "Monospace");
                            ui.selectable_value(&mut self.font_family, egui::FontFamily::Proportional, "Sans-Serif");
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
            ui.horizontal(|ui: &mut egui::Ui| {
                let is_dark: bool = ui.visuals().dark_mode;
                ui.label(format!("File: {}", self.get_file_name()));
                ui.separator();
                let (status, color) = if self.dirty {
                    ("Unsaved", if is_dark { ColorPalette::AMBER_400 } else { ColorPalette::AMBER_600 })
                } else {
                    ("Saved", if is_dark { ColorPalette::GREEN_400 } else { ColorPalette::GREEN_600 })
                };
                ui.label(egui::RichText::new(status).color(color));
                ui.separator();
                ui.label(format!("Characters: {}", self.count_visible_chars()));
                ui.separator();
                ui.label(format!("Words: {}", self.count_words()));
            });
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
                    if response.changed() { self.dirty = true; }
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
        });
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
            let mut in_code_block = false;

            for line in self.content.lines() {
                let is_fence: bool = line.trim().starts_with("```");
                if is_fence { in_code_block = !in_code_block; }
                lines.push(line);
                code_line_flags.push(!is_fence && in_code_block);
                fence_line_flags.push(is_fence);
            }

            let mut per_line_row_heights: Vec<Vec<f32>> = Vec::with_capacity(lines.len());
            ui.fonts_mut(|fonts: &mut egui::epaint::FontsView<'_>| {
                for (idx, line) in lines.iter().enumerate() {
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

            let desired_size: egui::Vec2 = ui.available_size();
            let (outer_rect, _) = ui.allocate_exact_size(desired_size, Sense::click());
            let painter: &egui::Painter = ui.painter();
            let mut y: f32 = outer_rect.min.y + top_padding;
            let full_width: f32 = outer_rect.width().max(0.0);
            let code_bg: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_800 } else { ColorPalette::ZINC_200 };

            for (line_idx, row_heights) in per_line_row_heights.iter().enumerate() {
                if fence_line_flags[line_idx] || code_line_flags[line_idx] {
                    for &h in row_heights {
                        painter.rect_filled(Rect::from_min_size(pos2(outer_rect.min.x, y), vec2(full_width, h)), 0.0, code_bg);
                        y += h;
                    }
                } else {
                    for &h in row_heights { y += h; }
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
                            job.append(line, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            Self::append_fence_line_job(line, &mut job, font_size, is_dark_mode, true, cursor_pos, char_offset);
                        }
                    } else if in_code_block {
                        job.append(line, 0.0, Self::code_block_background_format_static(font_size, is_dark_mode, available_width));
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

            let text_edit: egui::TextEdit<'_> = egui::TextEdit::multiline(&mut self.content)
                .layouter(&mut layouter).lock_focus(true).frame(false);
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
            if response.changed() { self.dirty = true; }
        });
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
        let headings: &[(&str, f32)] = &[("#### ", 1.1), ("### ", 1.2), ("## ", 1.4), ("# ", 1.6)];
        for (prefix, scale) in headings {
            if let Some(rest) = line.strip_prefix(prefix) {
                let header_end: usize = line_start_offset + line.chars().count();
                let cursor_in: bool = cursor_pos.map_or(false, |p: usize| p >= line_start_offset && p <= header_end);
                if cursor_in {
                    job.append(prefix, 0.0, Self::markdown_syntax_format_static(font_size));
                    job.append(rest, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                } else {
                    job.append(prefix, 0.0, Self::invisible_format_static());
                    job.append(rest, 0.0, Self::heading_format_static(font_size, *scale, is_dark_mode));
                }
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

    pub(super) fn parse_inline_formatting_static(
        text: &str,
        job: &mut egui::text::LayoutJob,
        font_size: f32,
        font_family: &egui::FontFamily,
        cursor_pos: Option<usize>,
        text_start_offset: usize,
        is_dark_mode: bool,
    ) {
        let chars: Vec<char> = text.chars().collect();
        let mut i: usize = 0;
        let mut current_text: String = String::new();

        let flush = |current_text: &mut String, job: &mut egui::text::LayoutJob| {
            if !current_text.is_empty() {
                job.append(current_text, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
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

            macro_rules! try_span {
                ($marker:expr, $format:expr, $min_content:expr) => {{
                    let mlen = $marker.chars().count();
                    if is_valid_start(i, mlen) {
                        if let Some(end) = Self::find_closing_marker(&chars, i + mlen, $marker) {
                            if end > i + mlen {
                                let end_pos = end + mlen;
                                flush(&mut current_text, job);
                                if cursor_in(current_pos, end_pos) {
                                    let r: String = chars[i..end_pos].iter().collect();
                                    job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size));
                                } else {
                                    let om: String = chars[i..i + mlen].iter().collect();
                                    job.append(&om, 0.0, Self::invisible_format_static());
                                    let content: String = chars[i + mlen..end].iter().collect();
                                    job.append(&content, 0.0, $format);
                                    let cm: String = chars[end..end_pos].iter().collect();
                                    job.append(&cm, 0.0, Self::invisible_format_static());
                                }
                                i = end_pos;
                                continue;
                            }
                        }
                    }
                }};
            }

            if i + 1 < chars.len() && chars[i] == '~' && chars[i + 1] == '~' && is_valid_start(i, 2) {
                try_span!("~~", Self::strikethrough_format_static(font_size, is_dark_mode), 2);
                if i < chars.len() && chars[i] == '~' {
                    current_text.push(chars[i]); i += 1; continue;
                }
            }

            if chars[i] == '~' && i + 1 < chars.len() && !chars[i + 1].is_whitespace() && chars[i + 1] != '~' {
                let mut end: usize = i + 1;
                while end < chars.len() && chars[end] != '~' {
                    if chars[end].is_whitespace() || chars[end].is_ascii_punctuation() { break; }
                    end += 1;
                }
                if end > i + 1 {
                    flush(&mut current_text, job);
                    if cursor_pos.map_or(false, |p| p >= current_pos && p <= text_start_offset + end) {
                        let r: String = chars[i..end].iter().collect();
                        job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size));
                    } else {
                        job.append("~", 0.0, Self::invisible_format_static());
                        let content: String = chars[i + 1..end].iter().collect();
                        job.append(&content, 0.0, Self::subscript_format_static(font_size, is_dark_mode));
                    }
                    i = end; continue;
                }
            }

            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' && is_valid_start(i, 2) {
                if let Some(end) = Self::find_closing_marker(&chars, i + 2, "**") {
                    if end > i + 2 {
                        let end_pos: usize = end + 2;
                        flush(&mut current_text, job);
                        if cursor_in(current_pos, end_pos) {
                            let r: String = chars[i..end_pos].iter().collect();
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            job.append("**", 0.0, Self::invisible_format_static());
                            let content: String = chars[i + 2..end].iter().collect();
                            job.append(&content, 0.0, Self::bold_format_static(font_size, is_dark_mode));
                            job.append("**", 0.0, Self::invisible_format_static());
                        }
                        i = end_pos; continue;
                    }
                }
            }

            if chars[i] == '*' && !(i + 1 < chars.len() && chars[i + 1] == '*') && is_valid_start(i, 1) {
                if let Some(end) = Self::find_closing_marker(&chars, i + 1, "*") {
                    if end > i + 1 {
                        let end_pos = end + 1;
                        flush(&mut current_text, job);
                        if cursor_in(current_pos, end_pos) {
                            let r: String = chars[i..end_pos].iter().collect();
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            job.append("*", 0.0, Self::invisible_format_static());
                            let content: String = chars[i + 1..end].iter().collect();
                            job.append(&content, 0.0, Self::italic_format_static(font_size, is_dark_mode));
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
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            job.append("__", 0.0, Self::invisible_format_static());
                            let content: String = chars[i + 2..end].iter().collect();
                            job.append(&content, 0.0, Self::underline_format_static(font_size, is_dark_mode));
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
                            job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size));
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
                let mut end: usize = i + 1;
                while end < chars.len() && chars[end] != '^' {
                    if chars[end].is_whitespace() || chars[end].is_ascii_punctuation() { break; }
                    end += 1;
                }
                if end > i + 1 {
                    flush(&mut current_text, job);
                    if cursor_pos.map_or(false, |p| p >= current_pos && p <= text_start_offset + end) {
                        let r: String = chars[i..end].iter().collect();
                        job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size));
                    } else {
                        job.append("^", 0.0, Self::invisible_format_static());
                        let content: String = chars[i + 1..end].iter().collect();
                        job.append(&content, 0.0, Self::superscript_format_static(font_size, is_dark_mode));
                    }
                    i = end; continue;
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
                                job.append(&r, 0.0, Self::markdown_syntax_format_static(font_size));
                            } else {
                                job.append("[", 0.0, Self::invisible_format_static());
                                let link_text: String = chars[i + 1..text_end].iter().collect();
                                job.append(&link_text, 0.0, Self::link_format_static(font_size));
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
            job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
        }
    }

    pub(super) fn invisible_format_static() -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(0.001, egui::FontFamily::Monospace),
            color: egui::Color32::TRANSPARENT,
            ..Default::default()
        }
    }

    pub(super) fn transparent_format_static(font_size: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Monospace),
            color: egui::Color32::TRANSPARENT,
            ..Default::default()
        }
    }

    pub(super) fn zero_width_format_static() -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(0.01, egui::FontFamily::Monospace),
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

    pub(super) fn bold_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_100 } else { ColorPalette::ZINC_900 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 1.15, egui::FontFamily::Proportional),
            extra_letter_spacing: 2.5,
            color,
            ..Default::default()
        }
    }

    pub(super) fn italic_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Proportional),
            italics: true,
            color,
            ..Default::default()
        }
    }

    pub(super) fn underline_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Proportional),
            underline: egui::Stroke::new(1.0, ColorPalette::ZINC_500),
            color,
            ..Default::default()
        }
    }

    pub(super) fn strikethrough_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Proportional),
            strikethrough: egui::Stroke::new(1.0, ColorPalette::ZINC_500),
            color,
            ..Default::default()
        }
    }

    pub(super) fn code_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let (bg_color, text_color) = if is_dark_mode {
            (ColorPalette::ZINC_800, ColorPalette::AMBER_300)
        } else {
            (ColorPalette::ZINC_200, ColorPalette::AMBER_300)
        };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 0.9, egui::FontFamily::Monospace),
            background: bg_color,
            color: text_color,
            ..Default::default()
        }
    }

    pub(super) fn code_block_background_format_static(font_size: f32, is_dark_mode: bool, _available_width: f32) -> egui::TextFormat {
        let text_color: egui::Color32 = if is_dark_mode { ColorPalette::SLATE_300 } else { ColorPalette::ZINC_800 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Monospace),
            color: text_color,
            ..Default::default()
        }
    }

    pub(super) fn code_block_label_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let text_color: egui::Color32 = if is_dark_mode { ColorPalette::BLUE_400 } else { ColorPalette::BLUE_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 0.7, egui::FontFamily::Monospace),
            color: text_color,
            ..Default::default()
        }
    }

    pub(super) fn superscript_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 0.7, egui::FontFamily::Proportional),
            valign: egui::Align::TOP,
            color,
            ..Default::default()
        }
    }

    pub(super) fn subscript_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 0.7, egui::FontFamily::Proportional),
            valign: egui::Align::BOTTOM,
            color,
            ..Default::default()
        }
    }

    pub(super) fn link_format_static(font_size: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Proportional),
            underline: egui::Stroke::new(1.0, ColorPalette::BLUE_500),
            color: ColorPalette::BLUE_500,
            ..Default::default()
        }
    }

    pub(super) fn markdown_syntax_format_static(font_size: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Monospace),
            color: ColorPalette::ZINC_500,
            ..Default::default()
        }
    }

    pub(super) fn heading_format_static(font_size: f32, scale: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color: egui::Color32 = if is_dark_mode { ColorPalette::ZINC_200 } else { ColorPalette::ZINC_800 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * scale, egui::FontFamily::Proportional),
            color,
            ..Default::default()
        }
    }
}
