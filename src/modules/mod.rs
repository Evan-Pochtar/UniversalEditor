use eframe::egui;
use std::any::Any;

pub mod text_editor;
pub mod image_converter;

#[allow(dead_code)]
pub trait EditorModule {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, show_toolbar: bool, show_file_info: bool);
    fn save(&mut self) -> Result<(), String>;
    fn save_as(&mut self) -> Result<(), String>;
    fn get_title(&self) -> String;
    fn as_any(&self) -> &dyn Any;
}
