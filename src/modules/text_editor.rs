use eframe::egui;
use ropey::Rope;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
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
        }
    }

    pub fn load(path: PathBuf) -> Self {
        let file = File::open(&path).ok().map(BufReader::new);
        
        let content = if let Some(reader) = file {
            let rope = Rope::from_reader(reader).unwrap_or_default();
            rope.to_string()
        } else {
            String::new()
        };

        Self {
            file_path: Some(path),
            content,
            dirty: false,
            font_size: 14.0,
            font_family: egui::FontFamily::Monospace,
            view_mode: ViewMode::Plain,
            last_cursor_range: None,
        }
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
            self.last_cursor_range = None;
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
            
            let selected = &self.content[start_byte..end_byte];
            
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
            } else {
                let wrapped = format!("{}{}{}", wrapper, selected, wrapper);
                self.content.replace_range(start_byte..end_byte, &wrapped);
            }
            
            self.dirty = true;
            self.last_cursor_range = None;
        }
    }

    fn format_bold(&mut self) {
        self.wrap_selection("**");
    }

    fn format_italic(&mut self) {
        self.wrap_selection("*");
    }

    fn format_underline(&mut self) {
        self.wrap_selection("__");
    }

    fn render_markdown_editable(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let font_size = self.font_size;
            let font_family = self.font_family.clone();
            let cursor_pos = self.last_cursor_range.map(|r| r.primary.index);
            
            let mut layouter = |ui: &egui::Ui, text_buffer: &dyn egui::TextBuffer, wrap_width: f32| {
                let text = text_buffer.as_str();
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = wrap_width;
                
                let mut char_offset = 0;
                for line in text.lines() {
                    Self::parse_markdown_line_static(line, &mut job, font_size, &font_family, cursor_pos, char_offset);
                    job.append("\n", 0.0, Self::default_format_static(font_size, &font_family));
                    char_offset += line.chars().count() + 1;
                }
                
                ui.fonts_mut(|f| f.layout_job(job))
            };

            let text_edit = egui::TextEdit::multiline(&mut self.content)
                .layouter(&mut layouter)
                .lock_focus(true)
                .frame(false);

            let response = ui.add_sized(ui.available_size(), text_edit);

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
    ) {
        if let Some(rest) = line.strip_prefix("### ") {
            let header_start = line_start_offset;
            let header_end = line_start_offset + line.chars().count();
            let cursor_in_header = cursor_pos.map_or(false, |pos| pos >= header_start && pos <= header_end);
            
            if cursor_in_header {
                job.append("### ", 0.0, Self::markdown_syntax_format_static(font_size));
                job.append(rest, 0.0, Self::default_format_static(font_size, font_family));
            } else {
                job.append(rest, 0.0, Self::heading_format_static(font_size, 1.2));
            }
            return;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            let header_start = line_start_offset;
            let header_end = line_start_offset + line.chars().count();
            let cursor_in_header = cursor_pos.map_or(false, |pos| pos >= header_start && pos <= header_end);
            
            if cursor_in_header {
                job.append("## ", 0.0, Self::markdown_syntax_format_static(font_size));
                job.append(rest, 0.0, Self::default_format_static(font_size, font_family));
            } else {
                job.append(rest, 0.0, Self::heading_format_static(font_size, 1.4));
            }
            return;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            let header_start = line_start_offset;
            let header_end = line_start_offset + line.chars().count();
            let cursor_in_header = cursor_pos.map_or(false, |pos| pos >= header_start && pos <= header_end);
            
            if cursor_in_header {
                job.append("# ", 0.0, Self::markdown_syntax_format_static(font_size));
                job.append(rest, 0.0, Self::default_format_static(font_size, font_family));
            } else {
                job.append(rest, 0.0, Self::heading_format_static(font_size, 1.6));
            }
            return;
        }

        let list_offset = if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
            2
        } else {
            0
        };

        if let Some(rest) = line.strip_prefix("- ") {
            job.append("• ", 0.0, Self::default_format_static(font_size, font_family));
            Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + list_offset);
            return;
        }
        if let Some(rest) = line.strip_prefix("* ") {
            job.append("• ", 0.0, Self::default_format_static(font_size, font_family));
            Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + list_offset);
            return;
        }
        if let Some(rest) = line.strip_prefix("+ ") {
            job.append("• ", 0.0, Self::default_format_static(font_size, font_family));
            Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + list_offset);
            return;
        }

        let mut i = 0;
        while i < line.len() && line.chars().nth(i).map_or(false, |c| c.is_ascii_digit()) {
            i += 1;
        }
        if i > 0 && line.chars().nth(i) == Some('.') && line.chars().nth(i + 1) == Some(' ') {
            let (prefix, rest) = line.split_at(line.char_indices().nth(i + 2).map(|(idx, _)| idx).unwrap_or(line.len()));
            job.append(prefix, 0.0, Self::default_format_static(font_size, font_family));
            let prefix_chars = prefix.chars().count();
            Self::parse_inline_formatting_static(rest, job, font_size, font_family, cursor_pos, line_start_offset + prefix_chars);
            return;
        }

        Self::parse_inline_formatting_static(line, job, font_size, font_family, cursor_pos, line_start_offset);
    }

    fn parse_inline_formatting_static(
        text: &str,
        job: &mut egui::text::LayoutJob,
        font_size: f32,
        font_family: &egui::FontFamily,
        cursor_pos: Option<usize>,
        text_start_offset: usize,
    ) {
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        let mut current_text = String::new();

        while i < chars.len() {
            let current_pos = text_start_offset + i;
            
            let is_followed_by_whitespace = |pos: usize| -> bool {
                pos >= chars.len() || chars[pos].is_whitespace()
            };

            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
                if let Some(end) = Self::find_closing_marker(&chars, i + 2, "**") {
                    let marker_end_pos = end + 2;
                    if is_followed_by_whitespace(marker_end_pos) {
                        if !current_text.is_empty() {
                            job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family));
                            current_text.clear();
                        }

                        let marker_end = text_start_offset + marker_end_pos;
                        let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= current_pos && pos <= marker_end);
                        
                        if cursor_in_range {
                            let region: String = chars[i..marker_end_pos].iter().collect();
                            job.append(&region, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            let content: String = chars[i + 2..end].iter().collect();
                            job.append(&content, 0.0, Self::bold_format_static(font_size));
                        }
                        i = marker_end_pos;
                        continue;
                    }
                }
            }

            if chars[i] == '*' {
                if let Some(end) = Self::find_closing_marker(&chars, i + 1, "*") {
                    let marker_end_pos = end + 1;
                    if is_followed_by_whitespace(marker_end_pos) {
                        if !current_text.is_empty() {
                            job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family));
                            current_text.clear();
                        }

                        let marker_end = text_start_offset + marker_end_pos;
                        let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= current_pos && pos <= marker_end);
                        
                        if cursor_in_range {
                            let region: String = chars[i..marker_end_pos].iter().collect();
                            job.append(&region, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            let content: String = chars[i + 1..end].iter().collect();
                            job.append(&content, 0.0, Self::italic_format_static(font_size));
                        }
                        i = marker_end_pos;
                        continue;
                    }
                }
            }

            if i + 1 < chars.len() && chars[i] == '_' && chars[i + 1] == '_' {
                if let Some(end) = Self::find_closing_marker(&chars, i + 2, "__") {
                    let marker_end_pos = end + 2;
                    if is_followed_by_whitespace(marker_end_pos) {
                        if !current_text.is_empty() {
                            job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family));
                            current_text.clear();
                        }

                        let marker_end = text_start_offset + marker_end_pos;
                        let cursor_in_range = cursor_pos.map_or(false, |pos| pos >= current_pos && pos <= marker_end);
                        
                        if cursor_in_range {
                            let region: String = chars[i..marker_end_pos].iter().collect();
                            job.append(&region, 0.0, Self::markdown_syntax_format_static(font_size));
                        } else {
                            let content: String = chars[i + 2..end].iter().collect();
                            job.append(&content, 0.0, Self::underline_format_static(font_size));
                        }
                        i = marker_end_pos;
                        continue;
                    }
                }
            }

            current_text.push(chars[i]);
            i += 1;
        }

        if !current_text.is_empty() {
            job.append(&current_text, 0.0, Self::default_format_static(font_size, font_family));
        }
    }

    fn bold_format_static(font_size: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * 1.05, egui::FontFamily::Proportional),
            extra_letter_spacing: 1.5,
            ..Default::default()
        }
    }

    fn italic_format_static(font_size: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Proportional),
            italics: true,
            ..Default::default()
        }
    }

    fn underline_format_static(font_size: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Proportional),
            underline: egui::Stroke::new(1.0, egui::Color32::GRAY),
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

    fn markdown_syntax_format_static(font_size: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, egui::FontFamily::Monospace),
            color: egui::Color32::from_rgb(120, 120, 120),
            ..Default::default()
        }
    }

    fn heading_format_static(font_size: f32, scale: f32) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size * scale, egui::FontFamily::Proportional),
            ..Default::default()
        }
    }

    fn default_format_static(font_size: f32, font_family: &egui::FontFamily) -> egui::TextFormat {
        egui::TextFormat {
            font_id: egui::FontId::new(font_size, font_family.clone()),
            ..Default::default()
        }
    }
}

impl EditorModule for TextEditor {
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
            .add_filter("Text", &["txt", "md", "rs", "toml", "json"])
            .save_file() 
        {
            self.file_path = Some(path);
            self.save()
        } else {
            Err("Cancelled".to_string())
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.label("Format:");
            
            ui.vertical(|ui| {
                ui.add_space(1.0);
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("B").strong()).clicked() {
                        self.format_bold();
                    }
                    if ui.button(egui::RichText::new("I").italics()).clicked() {
                        self.format_italic();
                    }
                    if ui.button(egui::RichText::new("U").underline()).clicked() {
                        self.format_underline();
                    }
                });
            });

            ui.separator();

            ui.label("View:");
            
            egui::ComboBox::from_id_salt("view_mode")
                .selected_text(match self.view_mode {
                    ViewMode::Markdown => "Markdown",
                    ViewMode::Plain => "Plain Text",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.view_mode, ViewMode::Markdown, "Markdown");
                    ui.selectable_value(&mut self.view_mode, ViewMode::Plain, "Plain Text");
                });

            ui.separator();

            ui.label("Font:");
            
            egui::ComboBox::from_id_salt("font_fam")
                .selected_text(if matches!(self.font_family, egui::FontFamily::Proportional) { "Sans" } else { "Mono" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.font_family, egui::FontFamily::Monospace, "Monospace");
                    ui.selectable_value(&mut self.font_family, egui::FontFamily::Proportional, "Sans-Serif");
                });

            ui.label("Size:");
            ui.add(egui::DragValue::new(&mut self.font_size).speed(0.5).range(8.0..=72.0));
        });

        ui.separator();

        match self.view_mode {
            ViewMode::Markdown => {
                self.render_markdown_editable(ui, ctx);
            }
            ViewMode::Plain => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let font_id = egui::FontId::new(self.font_size, self.font_family.clone());

                    let text_edit = egui::TextEdit::multiline(&mut self.content)
                        .font(font_id)
                        .lock_focus(true)
                        .frame(false);

                    let response = ui.add_sized(ui.available_size(), text_edit);

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
                let _ = self.save();
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
        });
    }
}
