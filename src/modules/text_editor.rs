use eframe::egui;
use ropey::Rope;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use crate::style::ColorPalette;

use super::EditorModule;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Plain,
    Markdown,
}

pub struct TextEditor {
    file_path: Option<PathBuf>,
    content: String,
    dirty: bool,
    font_size: f32,
    font_family: egui::FontFamily,
    view_mode: ViewMode,
    last_cursor_range: Option<egui::text::CCursorRange>,
    pending_cursor_pos: Option<usize>,
}

impl TextEditor {
    pub fn new_empty() -> Self {
        Self {
            file_path: None,
            content: String::new(),
            dirty: false,
            font_size: 14.0,
            font_family: egui::FontFamily::Monospace,
            view_mode: ViewMode::Plain,
            last_cursor_range: None,
            pending_cursor_pos: None,
        }
    }

    pub fn load(path: PathBuf) -> Self {
        let file = File::open(&path).ok().map(BufReader::new);
        
        let content = if let Some(reader) = file {
            let rope = Rope::from_reader(reader).unwrap_or_default();
            rope.to_string().replace("\r\n", "\n")
        } else {
            String::new()
        };

        let view_mode = Self::detect_view_mode(&path);

        Self {
            file_path: Some(path),
            content,
            dirty: false,
            font_size: 14.0,
            font_family: egui::FontFamily::Monospace,
            view_mode,
            last_cursor_range: None,
            pending_cursor_pos: None,
        }
    }

    fn detect_view_mode(path: &PathBuf) -> ViewMode {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_lowercase().as_str() {
                "md" | "markdown" => ViewMode::Markdown,
                _ => ViewMode::Plain,
            })
            .unwrap_or(ViewMode::Plain)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn char_index_to_byte_index(&self, char_index: usize) -> usize {
        self.content.char_indices()
            .nth(char_index)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or(self.content.len())
    }

    fn insert_wrapper_at_cursor(&mut self, wrapper: &str) {
        if let Some(range) = self.last_cursor_range {
            let cursor_pos = self.char_index_to_byte_index(range.primary.index);
            let wrapped = format!("{}{}", wrapper, wrapper);
            self.content.insert_str(cursor_pos, &wrapped);
            self.dirty = true;
            
            let wrapper_char_count = wrapper.chars().count();
            self.pending_cursor_pos = Some(range.primary.index + wrapper_char_count);
        }
    }

    fn wrap_selection(&mut self, wrapper: &str) {
        if let Some(range) = self.last_cursor_range {
            let start_char = range.primary.index.min(range.secondary.index);
            let end_char = range.primary.index.max(range.secondary.index);
            
            if start_char == end_char {
                self.insert_wrapper_at_cursor(wrapper);
                return;
            }
            
            let start_byte = self.char_index_to_byte_index(start_char);
            let end_byte = self.char_index_to_byte_index(end_char);
            
            let selected = self.content[start_byte..end_byte].to_string();
            
            let prefix_start_char = start_char.saturating_sub(wrapper.chars().count());
            let prefix_start_byte = self.char_index_to_byte_index(prefix_start_char);
            
            let suffix_end_char = end_char + wrapper.chars().count();
            let suffix_end_byte = if suffix_end_char >= self.content.chars().count() {
                self.content.len()
            } else {
                self.char_index_to_byte_index(suffix_end_char)
            };
            
            let has_prefix = start_char >= wrapper.chars().count() && 
                &self.content[prefix_start_byte..start_byte] == wrapper;
            let has_suffix = suffix_end_byte <= self.content.len() && 
                &self.content[end_byte..suffix_end_byte] == wrapper;
            
            if has_prefix && has_suffix {
                self.content.replace_range(end_byte..suffix_end_byte, "");
                self.content.replace_range(prefix_start_byte..start_byte, "");
                self.pending_cursor_pos = Some(start_char + selected.chars().count());
            } else {
                let wrapped = format!("{}{}{}", wrapper, selected, wrapper);
                let wrapper_char_count = wrapper.chars().count();
                self.content.replace_range(start_byte..end_byte, &wrapped);
                self.pending_cursor_pos = Some(start_char + selected.chars().count() + wrapper_char_count * 2);
            }
            
            self.dirty = true;
        }
    }

    fn format_bold(&mut self) { self.wrap_selection("**"); }
    fn format_italic(&mut self) { self.wrap_selection("*"); }
    fn format_underline(&mut self) { self.wrap_selection("__"); }
    fn format_strikethrough(&mut self) { self.wrap_selection("~~"); }
    fn format_code(&mut self) { self.wrap_selection("`"); }

    fn find_link_at_offset(chars: &[char], cursor_idx: usize) -> Option<String> {
        let search_start = cursor_idx.saturating_sub(1000); 
        let mut start_bracket = None;
        
        for i in (search_start..=cursor_idx).rev() {
            if i < chars.len() && chars[i] == '[' {
                start_bracket = Some(i);
                break;
            }
        }

        if let Some(start) = start_bracket {
            if let Some(text_end) = Self::find_closing_bracket(chars, start + 1) {
                if text_end + 1 < chars.len() && chars[text_end + 1] == '(' {
                    if let Some(url_end) = Self::find_closing_paren(chars, text_end + 2) {
                        let end = url_end + 1;
                        if cursor_idx >= start && cursor_idx <= end {
                            let url: String = chars[text_end + 2..url_end].iter().collect();
                            return Some(url);
                        }
                    }
                }
            }
        }
        None
    }

    fn format_heading(&mut self, level: usize) {
        if let Some(range) = self.last_cursor_range {
            let byte_idx = self.char_index_to_byte_index(range.primary.index);
            let start_byte = self.content[..byte_idx].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let end_byte = self.content[byte_idx..].find('\n').map(|i| byte_idx + i).unwrap_or(self.content.len());
            
            let line = &self.content[start_byte..end_byte];
            let content_start = line.find(|c: char| c != '#' && !c.is_whitespace()).unwrap_or(line.len());
            let clean_line = &line[content_start..];
            
            let new_line = if level > 0 {
                format!("{} {}", "#".repeat(level), clean_line)
            } else {
                clean_line.to_string()
            };
            
            self.content.replace_range(start_byte..end_byte, &new_line);
            self.dirty = true;
        }
    }

    fn markdown_editable(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        use egui::{pos2, vec2, Rect, Sense};

        egui::ScrollArea::vertical().show(ui, |ui| {
            let font_size = self.font_size;
            let font_family = self.font_family.clone();
            let cursor_pos = self.last_cursor_range.map(|r| r.primary.index);
            let is_dark_mode = ui.visuals().dark_mode;
            let available_width = ui.available_width();
            let top_padding = 2.0_f32;
            let wrap_width = (available_width).max(10.0);

            let mut lines: Vec<&str> = Vec::new();
            let mut code_line_flags: Vec<bool> = Vec::new();
            let mut fence_line_flags: Vec<bool> = Vec::new();
            let mut in_code_block = false;
            
            for line in self.content.lines() {
                let is_fence = line.trim().starts_with("```");
                if is_fence {
                    in_code_block = !in_code_block;
                    lines.push(line);
                    code_line_flags.push(false);
                    fence_line_flags.push(true);
                } else {
                    lines.push(line);
                    code_line_flags.push(in_code_block);
                    fence_line_flags.push(false);
                }
            }

            let mut per_line_row_heights: Vec<Vec<f32>> = Vec::with_capacity(lines.len());
            ui.fonts_mut(|fonts| {
                for (idx, line) in lines.iter().enumerate() {
                    let mut job = egui::text::LayoutJob::default();
                    job.wrap.max_width = wrap_width;

                    if fence_line_flags[idx] {
                        if let Some(start_idx) = line.find("```") {
                            let prefix = &line[..start_idx];
                            let rest = &line[start_idx + 3..];
                            let label = rest.trim_end();
                            let suffix_len = rest.len() - label.len();
                            let has_label = !label.is_empty();

                            if !prefix.is_empty() {
                                job.append(prefix, 0.0, Self::transparent_format_static(font_size));
                            }

                            let marker_fmt = if has_label {
                                Self::zero_width_format_static()
                            } else {
                                Self::transparent_format_static(font_size)
                            };
                            job.append("```", 0.0, marker_fmt);

                            if has_label {
                                job.append(label, 0.0, Self::code_block_label_format_static(font_size, is_dark_mode));
                            }

                            if suffix_len > 0 {
                                let suffix = &rest[label.len()..];
                                job.append(suffix, 0.0, Self::zero_width_format_static());
                            }
                        } else {
                            job.append(line, 0.0, Self::transparent_format_static(font_size));
                        }
                    } else if code_line_flags[idx] {
                        let fmt = Self::code_block_background_format_static(font_size, is_dark_mode, available_width);
                        job.append(line, 0.0, fmt);
                    } else {
                        if line.trim().is_empty() {
                            let fmt = Self::default_format_static(font_size, &font_family, is_dark_mode);
                            job.append(line, 0.0, fmt);
                        } else {
                            Self::parse_markdown_line_static(
                                line, 
                                &mut job, 
                                font_size, 
                                &font_family, 
                                cursor_pos, 
                                0, 
                                is_dark_mode
                            );
                        }
                    }

                    let galley = fonts.layout_job(job);
                    let mut row_heights: Vec<f32> = Vec::new();
                    for row in &galley.rows {
                        row_heights.push(row.height());
                    }

                    if row_heights.is_empty() {
                        row_heights.push((font_size * 1.25).max(16.0));
                    }
                    per_line_row_heights.push(row_heights);
                }
            });

            let desired_size = ui.available_size();
            let (outer_rect, _response) = ui.allocate_exact_size(desired_size, Sense::click());
            let painter = ui.painter();

            let mut y = outer_rect.min.y + top_padding;
            let full_width = (outer_rect.width()).max(0.0);
            let code_bg = if is_dark_mode { ColorPalette::ZINC_800 } else { ColorPalette::ZINC_200 };

            for (line_idx, row_heights) in per_line_row_heights.iter().enumerate() {
                if fence_line_flags[line_idx] || code_line_flags[line_idx] {
                    for &h in row_heights {
                        let rect = Rect::from_min_size(
                            pos2(outer_rect.min.x, y),
                            vec2(full_width, h),
                        );
                        painter.rect_filled(rect, 0.0, code_bg);
                        y += h;
                    }
                } else {
                    for &h in row_heights {
                        y += h;
                    }
                }
            }

            let mut layouter = |ui: &egui::Ui, text_buffer: &dyn egui::TextBuffer, wrap_width_closure: f32| {
                let text = text_buffer.as_str();
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = wrap_width_closure;
                let mut char_offset = 0;
                let mut in_code_block = false;

                let lines_vec: Vec<&str> = text.lines().collect();
                let ends_with_newline = text.ends_with('\n');
                
                for (line_idx, line) in lines_vec.iter().enumerate() {
                    let is_last_line = line_idx == lines_vec.len() - 1;
                    
                    if line.trim().starts_with("```") {
                        in_code_block = !in_code_block;

                        let marker_end = char_offset + line.chars().count();
                        let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= char_offset && pos <= marker_end);

                        if cursor_in_range {
                            job.append(line, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            if let Some(start_idx) = line.find("```") {
                                let prefix = &line[..start_idx];
                                let rest = &line[start_idx + 3..];
                                let label = rest.trim_end();
                                let suffix_len = rest.len() - label.len();
                                let has_label = !label.is_empty();

                                if !prefix.is_empty() {
                                    job.append(prefix, 0.0, Self::transparent_format_static(font_size));
                                }

                                let marker_fmt = if has_label {
                                    Self::zero_width_format_static()
                                } else {
                                    Self::transparent_format_static(font_size)
                                };
                                job.append("```", 0.0, marker_fmt);

                                if has_label {
                                    job.append(label, 0.0, Self::code_block_label_format_static(font_size, is_dark_mode));
                                }

                                if suffix_len > 0 {
                                    let suffix = &rest[label.len()..];
                                    job.append(suffix, 0.0, Self::zero_width_format_static());
                                }
                            } else {
                                job.append(line, 0.0, Self::transparent_format_static(font_size));
                            }
                        }
                    } else if in_code_block {
                        let bg_format = Self::code_block_background_format_static(font_size, is_dark_mode, available_width);
                        job.append(line, 0.0, bg_format);
                    } else {
                        Self::parse_markdown_line_static(line, &mut job, font_size, &font_family, cursor_pos, char_offset, is_dark_mode);
                    }
                    
                    if !is_last_line || ends_with_newline {
                        job.append("\n", 0.0, Self::default_format_static(font_size, &font_family, is_dark_mode));
                    }
                    char_offset += line.chars().count() + 1;
                }
                ui.fonts_mut(|f| f.layout_job(job))
            };

            let text_edit = egui::TextEdit::multiline(&mut self.content)
                .layouter(&mut layouter)
                .lock_focus(true)
                .frame(false);

            let response = ui.put(outer_rect, text_edit);
            
            if response.clicked() && ctx.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                if let Some(cursor_range) = self.last_cursor_range {
                    let chars: Vec<char> = self.content.chars().collect();
                    if let Some(url) = Self::find_link_at_offset(&chars, cursor_range.primary.index) {
                        let final_url = if url.starts_with("http://") || url.starts_with("https://") {
                            url
                        } else {
                            format!("https://{}", url)
                        };
                        ctx.open_url(egui::OpenUrl::new_tab(&final_url));
                    }
                }
            }

            if let Some(new_pos) = self.pending_cursor_pos.take() {
                if let Some(mut state) = egui::TextEdit::load_state(ctx, response.id) {
                    let ccursor = egui::text::CCursor::new(new_pos);
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                    state.store(ctx, response.id);
                }
            }

            if let Some(state) = egui::TextEdit::load_state(ctx, response.id) {
                if let Some(cursor_range) = state.cursor.char_range() {
                    self.last_cursor_range = Some(cursor_range);
                }
            }

            if response.changed() {
                self.dirty = true;
            }
        });
    }

    fn parse_markdown_line_static(
        line: &str,
        job: &mut egui::text::LayoutJob,
        font_size: f32,
        font_family: &egui::FontFamily,
        cursor_pos: Option<usize>,
        line_start_offset: usize,
        is_dark_mode: bool,
    ) {
        if let Some(rest) = line.strip_prefix("#### ") {
            let header_start = line_start_offset;
            let header_end = line_start_offset + line.chars().count();
            let cursor_in_header = cursor_pos.map_or(false, |pos| pos >= header_start && pos <= header_end);
            
            if cursor_in_header {
                job.append("#### ", 0.0, Self::markdown_syntax_format_static(font_size));
                job.append(rest, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
            } else {
                let invisible_prefix = "#### ";
                job.append(invisible_prefix, 0.0, Self::invisible_format_static());
                job.append(rest, 0.0, Self::heading_format_static(font_size, 1.1, is_dark_mode));
            }
            return;
        }
        if let Some(rest) = line.strip_prefix("### ") {
            let header_start = line_start_offset;
            let header_end = line_start_offset + line.chars().count();
            let cursor_in_header = cursor_pos.map_or(false, |pos| pos >= header_start && pos <= header_end);
            
            if cursor_in_header {
                job.append("### ", 0.0, Self::markdown_syntax_format_static(font_size));
                job.append(rest, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
            } else {
                let invisible_prefix = "### ";
                job.append(invisible_prefix, 0.0, Self::invisible_format_static());
                job.append(rest, 0.0, Self::heading_format_static(font_size, 1.2, is_dark_mode));
            }
            return;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            let header_start = line_start_offset;
            let header_end = line_start_offset + line.chars().count();
            let cursor_in_header = cursor_pos.map_or(false, |pos| pos >= header_start && pos <= header_end);
            
            if cursor_in_header {
                job.append("## ", 0.0, Self::markdown_syntax_format_static(font_size));
                job.append(rest, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
            } else {
                let invisible_prefix = "## ";
                job.append(invisible_prefix, 0.0, Self::invisible_format_static());
                job.append(rest, 0.0, Self::heading_format_static(font_size, 1.4, is_dark_mode));
            }
            return;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            let header_start = line_start_offset;
            let header_end = line_start_offset + line.chars().count();
            let cursor_in_header = cursor_pos.map_or(false, |pos| pos >= header_start && pos <= header_end);
            
            if cursor_in_header {
                job.append("# ", 0.0, Self::markdown_syntax_format_static(font_size));
                job.append(rest, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
            } else {
                let invisible_prefix = "# ";
                job.append(invisible_prefix, 0.0, Self::invisible_format_static());
                job.append(rest, 0.0, Self::heading_format_static(font_size, 1.6, is_dark_mode));
            }
            return;
        }

        let list_offset = if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
            2
        } else {
            0
        };

        if let Some(rest) = line.strip_prefix("- ") {
            job.append("• ", 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
            Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + list_offset, is_dark_mode);
            return;
        }
        if let Some(rest) = line.strip_prefix("* ") {
            job.append("• ", 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
            Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + list_offset, is_dark_mode);
            return;
        }
        if let Some(rest) = line.strip_prefix("+ ") {
            job.append("• ", 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
            Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + list_offset, is_dark_mode);
            return;
        }

        let mut i = 0;
        while i < line.len() && line.chars().nth(i).map_or(false, |c| c.is_ascii_digit()) {
            i += 1;
        }
        if i > 0 && line.chars().nth(i) == Some('.') && line.chars().nth(i + 1) == Some(' ') {
            let (prefix, rest) = line.split_at(line.char_indices().nth(i + 2).map(|(idx, _)| idx).unwrap_or(line.len()));
            job.append(prefix, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
            let prefix_chars = prefix.chars().count();
            Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + prefix_chars, is_dark_mode);
            return;
        }

        Self::parse_inline_formatting_static(line, job, font_size, font_family, cursor_pos, line_start_offset, is_dark_mode);
    }

    fn invisible_format_static() -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(0.001, egui::FontFamily::Monospace), 
            color: egui::Color32::TRANSPARENT,
            ..Default::default()
        }
    }

    fn transparent_format_static(font_size: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Monospace),
            color: egui::Color32::TRANSPARENT,
            ..Default::default()
        }
    }

    fn zero_width_format_static() -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(0.01, egui::FontFamily::Monospace), 
            color: egui::Color32::TRANSPARENT,
            ..Default::default()
        }
    }

    fn parse_inline_formatting_static(
        text: &str,
        job: &mut egui::text::LayoutJob,
        font_size: f32,
        font_family: &egui::FontFamily,
        cursor_pos: Option<usize>,
        text_start_offset: usize,
        is_dark_mode: bool,
    ) {
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        let mut current_text = String::new();

        while i < chars.len() {
            let current_pos = text_start_offset + i;
            
            let is_preceded_by_whitespace_or_start = |pos: usize| -> bool {
                pos == 0 || chars[pos - 1].is_whitespace()
            };
            
            let is_valid_marker_start = |pos: usize, marker_len: usize| -> bool {
                if pos + marker_len >= chars.len() {
                    return false;
                }
                is_preceded_by_whitespace_or_start(pos) && !chars[pos + marker_len].is_whitespace()
            };

            if i + 1 < chars.len() && chars[i] == '~' && chars[i + 1] == '~' && is_valid_marker_start(i, 2) {
                if let Some(end) = Self::find_closing_marker(&chars, i + 2, "~~") {
                    if end > i + 2 { 
                        let marker_end_pos = end + 2;
                        if !current_text.is_empty() {
                            job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                            current_text.clear();
                        }

                        let marker_end = text_start_offset + marker_end_pos;
                        let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= current_pos && pos < marker_end);
                        
                        if cursor_in_range {
                            let region: String = chars[i..marker_end_pos].iter().collect();
                            job.append(&region, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            let open_mark: String = chars[i..i+2].iter().collect();
                            job.append(&open_mark, 0.0, Self::invisible_format_static());

                            let content: String = chars[i + 2..end].iter().collect();
                            job.append(&content, 0.0, Self::strikethrough_format_static(font_size, is_dark_mode));

                            let close_mark: String = chars[end..marker_end_pos].iter().collect();
                            job.append(&close_mark, 0.0, Self::invisible_format_static());
                        }
                        i = marker_end_pos;
                        continue;
                    }
                }
            }

            if chars[i] == '~' && i + 1 < chars.len() && !chars[i + 1].is_whitespace() && chars[i + 1] != '~' {
                let mut end = i + 1;
                while end < chars.len() && chars[end] != '~' {
                    if chars[end].is_whitespace() || chars[end].is_ascii_punctuation() {
                        break;
                    }
                    end += 1;
                }
                if end > i + 1 {
                    if !current_text.is_empty() {
                        job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                        current_text.clear();
                    }

                    let marker_end = text_start_offset + end;
                    let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= current_pos && pos <= marker_end);
                    
                    if cursor_in_range {
                        let region: String = chars[i..end].iter().collect();
                        job.append(&region, 0.0, Self::markdown_syntax_format_static(font_size));
                    } else {
                        let marker: String = chars[i..i+1].iter().collect();
                        job.append(&marker, 0.0, Self::invisible_format_static());
                        
                        let content: String = chars[i + 1..end].iter().collect();
                        job.append(&content, 0.0, Self::subscript_format_static(font_size, is_dark_mode));
                    }
                    i = end;
                    continue;
                }
            }

            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' && is_valid_marker_start(i, 2) {
                if let Some(end) = Self::find_closing_marker(&chars, i + 2, "**") {
                    if end > i + 2 { 
                        let marker_end_pos = end + 2;
                        if !current_text.is_empty() {
                            job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                            current_text.clear();
                        }

                        let marker_end = text_start_offset + marker_end_pos;
                        let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= current_pos && pos < marker_end);
                        
                        if cursor_in_range {
                            let region: String = chars[i..marker_end_pos].iter().collect();
                            job.append(&region, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            let open_mark: String = chars[i..i+2].iter().collect();
                            job.append(&open_mark, 0.0, Self::invisible_format_static());

                            let content: String = chars[i + 2..end].iter().collect();
                            job.append(&content, 0.0, Self::bold_format_static(font_size, is_dark_mode));

                            let close_mark: String = chars[end..marker_end_pos].iter().collect();
                            job.append(&close_mark, 0.0, Self::invisible_format_static());
                        }
                        i = marker_end_pos;
                        continue;
                    }
                }
            }

            if chars[i] == '*' {
                let is_start_of_bold = i + 1 < chars.len() && chars[i + 1] == '*';
                
                if !is_start_of_bold && is_valid_marker_start(i, 1) {
                    if let Some(end) = Self::find_closing_marker(&chars, i + 1, "*") {
                        if end > i + 1 {
                            let marker_end_pos = end + 1;
                            if !current_text.is_empty() {
                                job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                                current_text.clear();
                            }

                            let marker_end = text_start_offset + marker_end_pos;
                            let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= current_pos && pos < marker_end);
                            
                            if cursor_in_range {
                                let region: String = chars[i..marker_end_pos].iter().collect();
                                job.append(&region, 0.0, Self::markdown_syntax_format_static(font_size));
                            } else {
                                let open_mark: String = chars[i..i+1].iter().collect();
                                job.append(&open_mark, 0.0, Self::invisible_format_static());

                                let content: String = chars[i + 1..end].iter().collect();
                                job.append(&content, 0.0, Self::italic_format_static(font_size, is_dark_mode));

                                let close_mark: String = chars[end..marker_end_pos].iter().collect();
                                job.append(&close_mark, 0.0, Self::invisible_format_static());
                            }
                            i = marker_end_pos;
                            continue;
                        }
                    }
                }
            }

            if i + 1 < chars.len() && chars[i] == '_' && chars[i + 1] == '_' && is_valid_marker_start(i, 2) {
                if let Some(end) = Self::find_closing_marker(&chars, i + 2, "__") {
                    if end > i + 2 {
                        let marker_end_pos = end + 2;
                        if !current_text.is_empty() {
                            job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                            current_text.clear();
                        }

                        let marker_end = text_start_offset + marker_end_pos;
                        let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= current_pos && pos < marker_end);
                        
                        if cursor_in_range {
                            let region: String = chars[i..marker_end_pos].iter().collect();
                            job.append(&region, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            let open_mark: String = chars[i..i+2].iter().collect();
                            job.append(&open_mark, 0.0, Self::invisible_format_static());

                            let content: String = chars[i + 2..end].iter().collect();
                            job.append(&content, 0.0, Self::underline_format_static(font_size, is_dark_mode));

                            let close_mark: String = chars[end..marker_end_pos].iter().collect();
                            job.append(&close_mark, 0.0, Self::invisible_format_static());
                        }
                        i = marker_end_pos;
                        continue;
                    }
                }
            }

            if chars[i] == '`' && is_valid_marker_start(i, 1) {
                if i + 2 < chars.len() && chars[i + 1] == '`' && chars[i + 2] == '`' {
                    current_text.push(chars[i]);
                    i += 1;
                    continue;
                }
                
                if let Some(end) = Self::find_closing_marker(&chars, i + 1, "`") {
                    if end > i + 1 {
                        let marker_end_pos = end + 1;
                        if !current_text.is_empty() {
                            job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                            current_text.clear();
                        }

                        let marker_end = text_start_offset + marker_end_pos;
                        let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= current_pos && pos < marker_end);
                        
                        if cursor_in_range {
                            let region: String = chars[i..marker_end_pos].iter().collect();
                            job.append(&region, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            let open_mark: String = chars[i..i+1].iter().collect();
                            job.append(&open_mark, 0.0, Self::invisible_format_static());

                            let content: String = chars[i + 1..end].iter().collect();
                            job.append(&content, 0.0, Self::code_format_static(font_size, is_dark_mode));

                            let close_mark: String = chars[end..marker_end_pos].iter().collect();
                            job.append(&close_mark, 0.0, Self::invisible_format_static());
                        }
                        i = marker_end_pos;
                        continue;
                    }
                }
            }

            if chars[i] == '^' && i + 1 < chars.len() && !chars[i + 1].is_whitespace() {
                let mut end = i + 1;
                while end < chars.len() && chars[end] != '^' {
                    if chars[end].is_whitespace() || chars[end].is_ascii_punctuation() {
                        break;
                    }
                    end += 1;
                }
                if end > i + 1 {
                    if !current_text.is_empty() {
                        job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                        current_text.clear();
                    }

                    let marker_end = text_start_offset + end;
                    let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= current_pos && pos <= marker_end);
                    
                    if cursor_in_range {
                        let region: String = chars[i..end].iter().collect();
                        job.append(&region, 0.0, Self::markdown_syntax_format_static(font_size));
                    } else {
                        let marker: String = chars[i..i+1].iter().collect();
                        job.append(&marker, 0.0, Self::invisible_format_static());
                        
                        let content: String = chars[i + 1..end].iter().collect();
                        job.append(&content, 0.0, Self::superscript_format_static(font_size, is_dark_mode));
                    }
                    i = end;
                    continue;
                }
            }

            if chars[i] == '[' {
                if let Some(text_end) = Self::find_closing_bracket(&chars, i + 1) {
                    if text_end + 1 < chars.len() && chars[text_end + 1] == '(' {
                        if let Some(url_end) = Self::find_closing_paren(&chars, text_end + 2) {
                            let marker_end_pos = url_end + 1;
                            if !current_text.is_empty() {
                                job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family, is_dark_mode));
                                current_text.clear();
                            }

                            let marker_end = text_start_offset + marker_end_pos;
                            let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= current_pos && pos <= marker_end);
                            
                            if cursor_in_range {
                                let region: String = chars[i..marker_end_pos].iter().collect();
                                job.append(&region, 0.0, Self::markdown_syntax_format_static(font_size));
                            } else {
                                let open_bracket: String = chars[i..i+1].iter().collect();
                                job.append(&open_bracket, 0.0, Self::invisible_format_static());

                                let link_text: String = chars[i + 1..text_end].iter().collect();
                                job.append(&link_text, 0.0, Self::link_format_static(font_size));
                                
                                let hidden_tail: String = chars[text_end..marker_end_pos].iter().collect();
                                job.append(&hidden_tail, 0.0, Self::invisible_format_static());
                            }
                            i = marker_end_pos;
                            continue;
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

    fn bold_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color = if is_dark_mode { 
            ColorPalette::ZINC_100 
        } else { 
            ColorPalette::ZINC_900 
        };
        
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 1.15, egui::FontFamily::Proportional),
            extra_letter_spacing: 2.5,
            color,
            ..Default::default()
        }
    }

    fn italic_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color = if is_dark_mode { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Proportional),
            italics: true,
            color,
            ..Default::default()
        }
    }

    fn underline_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color = if is_dark_mode { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Proportional),
            underline: egui::Stroke::new(1.0, ColorPalette::ZINC_500),
            color,
            ..Default::default()
        }
    }

    fn strikethrough_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color = if is_dark_mode { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Proportional),
            strikethrough: egui::Stroke::new(1.0, ColorPalette::ZINC_500),
            color,
            ..Default::default()
        }
    }

    fn code_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
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

    fn code_block_background_format_static(font_size: f32, is_dark_mode: bool, _available_width: f32) -> egui::TextFormat {
        let text_color = if is_dark_mode {
            ColorPalette::SLATE_300
        } else {
            ColorPalette::ZINC_800
        };
        
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 1.0, egui::FontFamily::Monospace),
            color: text_color,
            ..Default::default()
        }
    }

    fn code_block_label_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let text_color = if is_dark_mode {
            ColorPalette::BLUE_400
        } else {
            ColorPalette::BLUE_600
        };
        
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 0.7, egui::FontFamily::Monospace),
            color: text_color,
            ..Default::default()
        }
    }

    fn superscript_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color = if is_dark_mode { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 0.7, egui::FontFamily::Proportional),
            valign: egui::Align::TOP,
            color,
            ..Default::default()
        }
    }

    fn subscript_format_static(font_size: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color = if is_dark_mode { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 0.7, egui::FontFamily::Proportional),
            valign: egui::Align::BOTTOM,
            color,
            ..Default::default()
        }
    }

    fn link_format_static(font_size: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Proportional),
            underline: egui::Stroke::new(1.0, ColorPalette::BLUE_500),
            color: ColorPalette::BLUE_500,
            ..Default::default()
        }
    }

    fn find_closing_marker(chars: &[char], start: usize, marker: &str) -> Option<usize> {
        let marker_chars: Vec<char> = marker.chars().collect();
        let marker_len = marker_chars.len();
        
        let mut i = start;
        while i + marker_len <= chars.len() {
            let mut matches = true;
            for (j, &mc) in marker_chars.iter().enumerate() {
                if chars[i + j] != mc {
                    matches = false;
                    break;
                }
            }
            if matches {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn find_closing_bracket(chars: &[char], start: usize) -> Option<usize> {
        for i in start..chars.len() {
            if chars[i] == ']' {
                return Some(i);
            }
        }
        None
    }

    fn find_closing_paren(chars: &[char], start: usize) -> Option<usize> {
        for i in start..chars.len() {
            if chars[i] == ')' {
                return Some(i);
            }
        }
        None
    }

    fn markdown_syntax_format_static(font_size: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Monospace),
            color: ColorPalette::ZINC_500,
            ..Default::default()
        }
    }

    fn heading_format_static(font_size: f32, scale: f32, is_dark_mode: bool) -> egui::TextFormat {
        let color = if is_dark_mode { ColorPalette::ZINC_200 } else { ColorPalette::ZINC_800 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * scale, egui::FontFamily::Proportional),
            color,
            ..Default::default()
        }
    }

    fn default_format_static(font_size: f32, font_family: &egui::FontFamily, is_dark_mode: bool) -> egui::TextFormat {
        let color = if is_dark_mode { ColorPalette::ZINC_300 } else { ColorPalette::ZINC_700 };
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, font_family.clone()),
            color,
            ..Default::default()
        }
    }

    fn count_visible_chars(&self) -> usize {
        if self.view_mode != ViewMode::Markdown {
            return self.content.chars().count();
        }

        let mut count = 0;
        let chars: Vec<char> = self.content.chars().collect();
        let mut i = 0;
        let mut in_code_block = false;

        while i < chars.len() {
            let line_start = i;
            let mut line_end = i;
            while line_end < chars.len() && chars[line_end] != '\n' {
                line_end += 1;
            }

            let line: String = chars[line_start..line_end].iter().collect();

            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
                count += line.chars().count();
                if line_end < chars.len() {
                    count += 1;
                }
                i = line_end + 1;
                continue;
            }

            if in_code_block {
                count += line.chars().count();
                if line_end < chars.len() {
                    count += 1;
                }
                i = line_end + 1;
                continue;
            }

            let mut j = 0;
            let line_chars: Vec<char> = line.chars().collect();
            
            if let Some(rest) = line.strip_prefix("#### ") {
                count += rest.chars().count();
                if line_end < chars.len() {
                    count += 1;
                }
                i = line_end + 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("### ") {
                count += rest.chars().count();
                if line_end < chars.len() {
                    count += 1;
                }
                i = line_end + 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("## ") {
                count += rest.chars().count();
                if line_end < chars.len() {
                    count += 1;
                }
                i = line_end + 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("# ") {
                count += rest.chars().count();
                if line_end < chars.len() {
                    count += 1;
                }
                i = line_end + 1;
                continue;
            }

            if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
                count += 1;
                j = 2;
            }

            let mut k = 0;
            while k < line_chars.len() && line_chars[k].is_ascii_digit() {
                k += 1;
            }
            if k > 0 && k < line_chars.len() && line_chars[k] == '.' && k + 1 < line_chars.len() && line_chars[k + 1] == ' ' {
                count += k + 2;
                j = k + 2;
            }

            while j < line_chars.len() {
                if j + 1 < line_chars.len() && line_chars[j] == '~' && line_chars[j + 1] == '~' {
                    if let Some(end) = Self::find_closing_marker(&line_chars, j + 2, "~~") {
                        if end > j + 2 {
                            count += end - (j + 2);
                            j = end + 2;
                            continue;
                        }
                    }
                }

                if line_chars[j] == '~' && j + 1 < line_chars.len() && !line_chars[j + 1].is_whitespace() && line_chars[j + 1] != '~' {
                    let mut end = j + 1;
                    while end < line_chars.len() && line_chars[end] != '~' {
                        if line_chars[end].is_whitespace() || line_chars[end].is_ascii_punctuation() {
                            break;
                        }
                        end += 1;
                    }
                    if end > j + 1 {
                        count += end - (j + 1);
                        j = end;
                        continue;
                    }
                }

                if j + 1 < line_chars.len() && line_chars[j] == '*' && line_chars[j + 1] == '*' {
                    if let Some(end) = Self::find_closing_marker(&line_chars, j + 2, "**") {
                        if end > j + 2 {
                            count += end - (j + 2);
                            j = end + 2;
                            continue;
                        }
                    }
                }

                if line_chars[j] == '*' {
                    let is_start_of_bold = j + 1 < line_chars.len() && line_chars[j + 1] == '*';
                    if !is_start_of_bold {
                        if let Some(end) = Self::find_closing_marker(&line_chars, j + 1, "*") {
                            if end > j + 1 {
                                count += end - (j + 1);
                                j = end + 1;
                                continue;
                            }
                        }
                    }
                }

                if j + 1 < line_chars.len() && line_chars[j] == '_' && line_chars[j + 1] == '_' {
                    if let Some(end) = Self::find_closing_marker(&line_chars, j + 2, "__") {
                        if end > j + 2 {
                            count += end - (j + 2);
                            j = end + 2;
                            continue;
                        }
                    }
                }

                if line_chars[j] == '`' {
                    if j + 2 < line_chars.len() && line_chars[j + 1] == '`' && line_chars[j + 2] == '`' {
                        count += 1;
                        j += 1;
                        continue;
                    }

                    if let Some(end) = Self::find_closing_marker(&line_chars, j + 1, "`") {
                        if end > j + 1 {
                            count += end - (j + 1);
                            j = end + 1;
                            continue;
                        }
                    }
                }

                if line_chars[j] == '^' && j + 1 < line_chars.len() && !line_chars[j + 1].is_whitespace() {
                    let mut end = j + 1;
                    while end < line_chars.len() && line_chars[end] != '^' {
                        if line_chars[end].is_whitespace() || line_chars[end].is_ascii_punctuation() {
                            break;
                        }
                        end += 1;
                    }
                    if end > j + 1 {
                        count += end - (j + 1);
                        j = end;
                        continue;
                    }
                }

                if line_chars[j] == '[' {
                    if let Some(text_end) = Self::find_closing_bracket(&line_chars, j + 1) {
                        if text_end + 1 < line_chars.len() && line_chars[text_end + 1] == '(' {
                            if let Some(url_end) = Self::find_closing_paren(&line_chars, text_end + 2) {
                                count += text_end - (j + 1);
                                j = url_end + 1;
                                continue;
                            }
                        }
                    }
                }

                count += 1;
                j += 1;
            }

            if line_end < chars.len() {
                count += 1;
            }
            i = line_end + 1;
        }

        count
    }

    fn count_words(&self) -> usize {
        self.content
            .split_whitespace()
            .filter(|word| !word.is_empty())
            .count()
    }

    fn get_file_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }
}

impl EditorModule for TextEditor {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_title(&self) -> String {
        let name = self.file_path.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled");
        
        if self.dirty { format!("{} *", name) } else { name.to_string() }
    }

    fn save(&mut self) -> Result<(), String> {
        if self.file_path.is_none() {
            return self.save_as();
        }

        let path = self.file_path.as_ref().unwrap();
        let f = File::create(path).map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(f);
        
        let rope = Rope::from_str(&self.content);
        rope.write_to(&mut writer).map_err(|e| e.to_string())?;
        
        self.dirty = false;
        Ok(())
    }

    fn save_as(&mut self) -> Result<(), String> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text", &["txt", "md"])
            .save_file() 
        {
            self.file_path = Some(path);
            self.save()
        } else {
            Err("Cancelled".to_string())
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, show_toolbar: bool, show_file_info: bool) {
        if show_toolbar {
            ui.horizontal(|ui| {
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("B").strong())
                        .on_hover_text("Bold (Ctrl+B)")
                        .clicked() 
                    {
                        self.format_bold();
                    }
                    if ui.button(egui::RichText::new("I").italics())
                        .on_hover_text("Italic (Ctrl+I)")
                        .clicked() 
                    {
                        self.format_italic();
                    }
                    if ui.button(egui::RichText::new("U").underline())
                        .on_hover_text("Underline (Ctrl+U)")
                        .clicked() 
                    {
                        self.format_underline();
                    }
                    if ui.button(egui::RichText::new("S").strikethrough())
                        .on_hover_text("Strikethrough (Ctrl+Shift+S)")
                        .clicked() 
                    {
                        self.format_strikethrough();
                    }
                    if ui.button(egui::RichText::new("C").monospace())
                        .on_hover_text("Code (Ctrl+E)")
                        .clicked() 
                    {
                        self.format_code();
                    }
                    
                    ui.separator();
                    
                    if ui.button("H1").on_hover_text("Heading 1 (Ctrl+1)").clicked() {
                        self.format_heading(1);
                    }
                    if ui.button("H2").on_hover_text("Heading 2 (Ctrl+2)").clicked() {
                        self.format_heading(2);
                    }
                    if ui.button("H3").on_hover_text("Heading 3 (Ctrl+3)").clicked() {
                        self.format_heading(3);
                    }
                    if ui.button("H4").on_hover_text("Heading 4 (Ctrl+4)").clicked() {
                        self.format_heading(4);
                    }
                });

                ui.separator();
                ui.label("View:");

                ui.vertical(|ui| {
                    egui::ComboBox::from_id_salt("view_mode")
                        .selected_text(match self.view_mode {
                            ViewMode::Markdown => "Markdown",
                            ViewMode::Plain => "Plain Text",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.view_mode, ViewMode::Markdown, "Markdown");
                            ui.selectable_value(&mut self.view_mode, ViewMode::Plain, "Plain Text");
                        });
                });

                ui.separator();
                ui.label("Font:");

                ui.vertical(|ui| {
                egui::ComboBox::from_id_salt("font_fam")
                    .selected_text(if matches!(self.font_family, egui::FontFamily::Proportional) { "Sans" } else { "Mono" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.font_family, egui::FontFamily::Monospace, "Monospace");
                        ui.selectable_value(&mut self.font_family, egui::FontFamily::Proportional, "Sans-Serif");
                    });
                });

                ui.separator();
                ui.label("Size:");
                
                ui.vertical(|ui| {
                    ui.add(egui::DragValue::new(&mut self.font_size).speed(0.5).range(8.0..=72.0));
                });
            });

            ui.separator();
        }

        if show_file_info {
            ui.horizontal(|ui| {
                let is_dark_mode = ui.visuals().dark_mode;
                
                ui.label(format!("File: {}", self.get_file_name()));
                ui.separator();
                
                let save_status = if self.dirty { "Unsaved" } else { "Saved" };
                let save_color = if self.dirty {
                    if is_dark_mode { ColorPalette::AMBER_400 } else { ColorPalette::AMBER_600 }
                } else {
                    if is_dark_mode { ColorPalette::GREEN_400 } else { ColorPalette::GREEN_600 }
                };
                
                ui.label(egui::RichText::new(save_status).color(save_color));
                ui.separator();
                
                ui.label(format!("Characters: {}", self.count_visible_chars()));
                ui.separator();
                ui.label(format!("Words: {}", self.count_words()));
            });
            ui.separator();
        }

        match self.view_mode {
            ViewMode::Markdown => {
                self.markdown_editable(ui, ctx);
            }
            ViewMode::Plain => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let font_id = egui::FontId::new(self.font_size, self.font_family.clone());

                    let text_edit = egui::TextEdit::multiline(&mut self.content)
                        .font(font_id)
                        .lock_focus(true)
                        .frame(false);

                    let response = ui.add_sized(ui.available_size(), text_edit);

                    if let Some(new_pos) = self.pending_cursor_pos.take() {
                        if let Some(mut state) = egui::TextEdit::load_state(ctx, response.id) {
                            let ccursor = egui::text::CCursor::new(new_pos);
                            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                            state.store(ctx, response.id);
                        }
                    }

                    if let Some(state) = egui::TextEdit::load_state(ctx, response.id) {
                        if let Some(cursor_range) = state.cursor.char_range() {
                            self.last_cursor_range = Some(cursor_range);
                        }
                    }

                    if response.changed() {
                        self.dirty = true;
                    }
                });
            }
        }

        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::S) {
                if !i.modifiers.shift {
                    let _ = self.save();
                }
                else {
                    self.format_strikethrough();
                }
            }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::B) {
                self.format_bold();
            }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::I) {
                self.format_italic();
            }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::U) {
                self.format_underline();
            }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::E) {
                self.format_code();
            }
            if i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::A) {
                let _ = self.save_as();
            }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num1) {
                self.format_heading(1);
            }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num2) {
                self.format_heading(2);
            }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num3) {
                self.format_heading(3);
            }
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num4) {
                self.format_heading(4);
            }
        });
    }
}
