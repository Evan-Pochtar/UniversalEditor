use eframe::egui;
use std::any::Any;

pub mod text_editor;
pub mod image_editor;
pub mod converters;
pub mod helpers;

pub mod image_edit { pub use super::image_editor::ImageEditor; }
pub mod image_converter { pub use super::converters::image_converter::ImageConverter; }
pub mod image_export { pub use super::helpers::image_export::{ExportFormat, export_image}; }
pub mod text_edit { pub use super::text_editor::TextEditor; }

#[derive(Clone, Debug)]
pub enum MenuAction {
    Undo,
    Redo,
    Export,
    None,
    Custom(String),
}

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
}

#[derive(Default)]
pub struct MenuContribution {
    pub file_items: Vec<(MenuItem, MenuAction)>,
    pub edit_items: Vec<(MenuItem, MenuAction)>,
    pub view_items: Vec<(MenuItem, MenuAction)>,
    pub image_items: Vec<(MenuItem, MenuAction)>,
    pub filter_items: Vec<(MenuItem, MenuAction)>,
}

#[allow(dead_code)]
pub trait EditorModule {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, show_toolbar: bool, show_file_info: bool);
    fn save(&mut self) -> Result<(), String>;
    fn save_as(&mut self) -> Result<(), String>;
    fn get_title(&self) -> String;
    fn as_any(&self) -> &dyn Any;
    fn get_menu_contributions(&self) -> MenuContribution { MenuContribution::default() }
    fn handle_menu_action(&mut self, action: MenuAction) -> bool { let _ = action; false }
}
