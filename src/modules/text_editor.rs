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
            view_mode: ViewMode::Markdown,
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
            view_mode: ViewMode::Markdown,
            last_cursor_range: None,
        }
    }

    fn wrap_selection(&mut self, wrapper: &str) {
        if let Some(range) = self.last_cursor_range {
            let start = range.primary.index.min(range.secondary.index);
            let end = range.primary.index.max(range.secondary.index);
            
            if start != end {
                let selected = self.content.chars()
                    .skip(start)
                    .take(end - start)
                    .collect::<String>();
                
                let prefix_start = start.saturating_sub(wrapper.len());
                let suffix_end = (end + wrapper.len()).min(self.content.len());
                
                let has_prefix = start >= wrapper.len() && 
                    &self.content[prefix_start..start] == wrapper;
                let has_suffix = end + wrapper.len() <= self.content.len() && 
                    &self.content[end..suffix_end] == wrapper;
                
                if has_prefix && has_suffix {
                    self.content.replace_range(end..suffix_end, "");
                    self.content.replace_range(prefix_start..start, "");
                } else {
                    let wrapped = format!("{}{}{}", wrapper, selected, wrapper);
                    self.content.replace_range(start..end, &wrapped);
                }
                
                self.dirty = true;
            }
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

    fn render_markdown(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            
            for line in self.content.lines() {
                self.render_markdown_line(ui, line);
            }
        });
    }

    fn render_markdown_line(&self, ui: &mut egui::Ui, line: &str) {
        // Handle headers
        if let Some(rest) = line.strip_prefix("### ") {
            ui.heading(egui::RichText::new(rest).size(self.font_size * 1.2));
            return;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            ui.heading(egui::RichText::new(rest).size(self.font_size * 1.4));
            return;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            ui.heading(egui::RichText::new(rest).size(self.font_size * 1.6));
            return;
        }

        // Parse inline formatting
        let mut job = egui::text::LayoutJob::default();
        let mut chars = line.chars().peekable();
        let mut current_text = String::new();
        let mut is_bold = false;
        let mut is_italic = false;
        let mut is_underline = false;

        while let Some(ch) = chars.next() {
            if ch == '*' {
                // Check for bold (**)
                if chars.peek() == Some(&'*') {
                    chars.next();
                    if !current_text.is_empty() {
                        self.append_formatted_text(&mut job, &current_text, is_bold, is_italic, is_underline);
                        current_text.clear();
                    }
                    is_bold = !is_bold;
                } else {
                    // Italic (*)
                    if !current_text.is_empty() {
                        self.append_formatted_text(&mut job, &current_text, is_bold, is_italic, is_underline);
                        current_text.clear();
                    }
                    is_italic = !is_italic;
                }
            } else if ch == '_' && chars.peek() == Some(&'_') {
                // Underline (__)
                chars.next();
                if !current_text.is_empty() {
                    self.append_formatted_text(&mut job, &current_text, is_bold, is_italic, is_underline);
                    current_text.clear();
                }
                is_underline = !is_underline;
            } else {
                current_text.push(ch);
            }
        }

        if !current_text.is_empty() {
            self.append_formatted_text(&mut job, &current_text, is_bold, is_italic, is_underline);
        }

        if job.text.is_empty() {
            ui.add_space(self.font_size);
        } else {
            ui.label(job);
        }
    }

    fn append_formatted_text(
        &self,
        job: &mut egui::text::LayoutJob,
        text: &str,
        bold: bool,
        italic: bool,
        underline: bool,
    ) {
        let mut format = egui::TextFormat {
            font_id: egui::FontId::new(
                self.font_size,
                if bold {
                    egui::FontFamily::Proportional
                } else {
                    self.font_family.clone()
                },
            ),
            ..Default::default()
        };

        if italic {
            format.italics = true;
        }
        if underline {
            format.underline = egui::Stroke::new(1.0, egui::Color32::GRAY);
        }

        job.append(text, 0.0, format);
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
        // --- TOOLBAR ---
        ui.horizontal(|ui| {
            // Formatting buttons
            ui.label("Format:");
            if ui.button(egui::RichText::new("B").strong()).clicked() {
                self.format_bold();
            }
            if ui.button(egui::RichText::new("I").italics()).clicked() {
                self.format_italic();
            }
            if ui.button(egui::RichText::new("U").underline()).clicked() {
                self.format_underline();
            }

            ui.separator();

            // View mode toggle
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

            // Font controls
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

        // --- MAIN CONTENT AREA ---
        match self.view_mode {
            ViewMode::Markdown => {
                self.render_markdown(ui);
            }
            ViewMode::Plain => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let font_id = egui::FontId::new(self.font_size, self.font_family.clone());

                    let text_edit = egui::TextEdit::multiline(&mut self.content)
                        .font(font_id)
                        .lock_focus(true)
                        .frame(false);

                    let response = ui.add_sized(ui.available_size(), text_edit);

                    // Track cursor position for formatting
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

        // --- KEYBOARD SHORTCUTS ---
        ctx.input_mut(|i| {
            // Ctrl+S to Save
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::S) {
                let _ = self.save();
            }
            // Ctrl+B for Bold
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::B) {
                self.format_bold();
            }
            // Ctrl+I for Italic
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::I) {
                self.format_italic();
            }
            // Ctrl+U for Underline
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::U) {
                self.format_underline();
            }
        });
    }
}
