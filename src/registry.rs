use eframe::egui::Color32;
use crate::style::ColorPalette;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CreateModule { TextEditor, ImageEditor, JsonEditor, ImageConverter }

pub struct ScreenDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub color: Color32,
    pub sidebar_letter: &'static str,
    pub accepted_extensions: &'static [&'static str],
    pub create: CreateModule,
}

pub struct ConverterDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub color: Color32,
    pub sidebar_letter: &'static str,
    pub create: CreateModule,
}

pub static SCREENS: &[ScreenDef] = &[
    ScreenDef {
        id: "text_editor",
        name: "Text Editor",
        description: "Rich editing in both markdown and plaintext",
        color: ColorPalette::BLUE_500,
        sidebar_letter: "T",
        accepted_extensions: &["txt", "md"],
        create: CreateModule::TextEditor,
    },
    ScreenDef {
        id: "image_editor",
        name: "Image Editor",
        description: "Edit, crop, and transform images",
        color: ColorPalette::PURPLE_500,
        sidebar_letter: "I",
        accepted_extensions: &["jpg", "jpeg", "png", "webp", "bmp", "tiff", "tif", "gif", "ico"],
        create: CreateModule::ImageEditor,
    },
    ScreenDef {
        id: "json_editor",
        name: "Json Editor",
        description: "Edit JSON with Tree and Text views",
        color: ColorPalette::AMBER_500,
        sidebar_letter: "J",
        accepted_extensions: &["json"],
        create: CreateModule::JsonEditor,
    },
];

pub static CONVERTERS: &[ConverterDef] = &[
    ConverterDef {
        id: "image_converter",
        name: "Image Converter",
        description: "Batch-convert between image formats",
        color: ColorPalette::TEAL_500,
        sidebar_letter: "C",
        create: CreateModule::ImageConverter,
    },
];

pub fn all_accepted_extensions() -> Vec<&'static str> {
    let mut exts: Vec<&'static str> = SCREENS.iter().flat_map(|s| s.accepted_extensions.iter().copied()).collect();
    exts.sort_unstable();
    exts.dedup();
    exts
}

pub fn screen_for_extension(ext: &str) -> Option<&'static ScreenDef> {
    let lower = ext.to_lowercase();
    SCREENS.iter().find(|s| s.accepted_extensions.iter().any(|&e| e == lower.as_str()))
}
