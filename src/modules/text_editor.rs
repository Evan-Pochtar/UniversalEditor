use eframe::egui;
use ropey::Rope;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use super::EditorModule;

pub struct TextEditor {
    file_path: Option<PathBuf>,
    content: String, 
    dirty: bool,
    font_size: f32,
    font_family: egui::FontFamily,
}

impl TextEditor {
    pub fn new_empty() -> Self {
        Self {
            file_path: None,
            content: String::new(),
            dirty: false,
            font_size: 14.0,
            font_family: egui::FontFamily::Monospace,
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

    fn ui(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        // --- TOOLBAR ---
        ui.horizontal(|ui| {
            ui.label("Font Family:");
            egui::ComboBox::from_id_source("font_fam")
                .selected_text(if matches!(self.font_family, egui::FontFamily::Proportional) { "Sans" } else { "Mono" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.font_family, egui::FontFamily::Monospace, "Monospace");
                    ui.selectable_value(&mut self.font_family, egui::FontFamily::Proportional, "Sans-Serif");
                });

            ui.add_space(10.0);

            ui.label("Size:");
            ui.add(egui::DragValue::new(&mut self.font_size).speed(0.5).clamp_range(8.0..=72.0));
        });

        ui.separator();

        // --- EDITOR AREA ---
        egui::ScrollArea::vertical().show(ui, |ui| {
            let font_id = egui::FontId::new(self.font_size, self.font_family.clone());

            let response = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.content)
                    .font(font_id) 
                    .lock_focus(true)
                    .frame(false)
            );

            if response.changed() {
                self.dirty = true;
            }
            
            // --- KEYBOARD SHORTCUTS ---
            // Ctrl+S to Save
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S)) {
                let _ = self.save();
            }
        });
    }
}
