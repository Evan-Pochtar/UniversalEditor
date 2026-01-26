use eframe::egui;
pub mod text_editor;

#[allow(dead_code)]
pub trait EditorModule {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context);
    fn save(&mut self) -> Result<(), String>;
    fn save_as(&mut self) -> Result<(), String>;
    fn get_title(&self) -> String;
}
