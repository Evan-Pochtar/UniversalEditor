use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeMode {
    Light,
    Dark,
}

pub struct ColorPalette;

#[allow(dead_code)]
impl ColorPalette {
    pub const BLUE_50: egui::Color32 = egui::Color32::from_rgb(239, 246, 255);
    pub const BLUE_100: egui::Color32 = egui::Color32::from_rgb(219, 234, 254);
    pub const BLUE_200: egui::Color32 = egui::Color32::from_rgb(191, 219, 254);
    pub const BLUE_300: egui::Color32 = egui::Color32::from_rgb(147, 197, 253);
    pub const BLUE_400: egui::Color32 = egui::Color32::from_rgb(96, 165, 250);
    pub const BLUE_500: egui::Color32 = egui::Color32::from_rgb(59, 130, 246);
    pub const BLUE_600: egui::Color32 = egui::Color32::from_rgb(37, 99, 235);
    pub const BLUE_700: egui::Color32 = egui::Color32::from_rgb(29, 78, 216);
    pub const BLUE_800: egui::Color32 = egui::Color32::from_rgb(30, 64, 175);
    pub const BLUE_900: egui::Color32 = egui::Color32::from_rgb(30, 58, 138);

    pub const SLATE_50: egui::Color32 = egui::Color32::from_rgb(248, 250, 252);
    pub const SLATE_100: egui::Color32 = egui::Color32::from_rgb(241, 245, 249);
    pub const SLATE_200: egui::Color32 = egui::Color32::from_rgb(226, 232, 240);
    pub const SLATE_300: egui::Color32 = egui::Color32::from_rgb(203, 213, 225);
    pub const SLATE_400: egui::Color32 = egui::Color32::from_rgb(148, 163, 184);
    pub const SLATE_500: egui::Color32 = egui::Color32::from_rgb(100, 116, 139);
    pub const SLATE_600: egui::Color32 = egui::Color32::from_rgb(71, 85, 105);
    pub const SLATE_700: egui::Color32 = egui::Color32::from_rgb(51, 65, 85);
    pub const SLATE_800: egui::Color32 = egui::Color32::from_rgb(30, 41, 59);
    pub const SLATE_900: egui::Color32 = egui::Color32::from_rgb(15, 23, 42);

    pub const GRAY_50: egui::Color32 = egui::Color32::from_rgb(249, 250, 251);
    pub const GRAY_100: egui::Color32 = egui::Color32::from_rgb(243, 244, 246);
    pub const GRAY_200: egui::Color32 = egui::Color32::from_rgb(229, 231, 235);
    pub const GRAY_300: egui::Color32 = egui::Color32::from_rgb(209, 213, 219);
    pub const GRAY_400: egui::Color32 = egui::Color32::from_rgb(156, 163, 175);
    pub const GRAY_500: egui::Color32 = egui::Color32::from_rgb(107, 114, 128);
    pub const GRAY_600: egui::Color32 = egui::Color32::from_rgb(75, 85, 99);
    pub const GRAY_700: egui::Color32 = egui::Color32::from_rgb(55, 65, 81);
    pub const GRAY_800: egui::Color32 = egui::Color32::from_rgb(31, 41, 55);
    pub const GRAY_900: egui::Color32 = egui::Color32::from_rgb(17, 24, 39);

    pub const ZINC_50: egui::Color32 = egui::Color32::from_rgb(250, 250, 250);
    pub const ZINC_100: egui::Color32 = egui::Color32::from_rgb(244, 244, 245);
    pub const ZINC_200: egui::Color32 = egui::Color32::from_rgb(228, 228, 231);
    pub const ZINC_300: egui::Color32 = egui::Color32::from_rgb(212, 212, 216);
    pub const ZINC_400: egui::Color32 = egui::Color32::from_rgb(161, 161, 170);
    pub const ZINC_500: egui::Color32 = egui::Color32::from_rgb(113, 113, 122);
    pub const ZINC_600: egui::Color32 = egui::Color32::from_rgb(82, 82, 91);
    pub const ZINC_700: egui::Color32 = egui::Color32::from_rgb(63, 63, 70);
    pub const ZINC_800: egui::Color32 = egui::Color32::from_rgb(39, 39, 42);
    pub const ZINC_900: egui::Color32 = egui::Color32::from_rgb(24, 24, 27);

    pub const GREEN_50: egui::Color32 = egui::Color32::from_rgb(240, 253, 244);
    pub const GREEN_100: egui::Color32 = egui::Color32::from_rgb(220, 252, 231);
    pub const GREEN_200: egui::Color32 = egui::Color32::from_rgb(187, 247, 208);
    pub const GREEN_300: egui::Color32 = egui::Color32::from_rgb(134, 239, 172);
    pub const GREEN_400: egui::Color32 = egui::Color32::from_rgb(74, 222, 128);
    pub const GREEN_500: egui::Color32 = egui::Color32::from_rgb(34, 197, 94);
    pub const GREEN_600: egui::Color32 = egui::Color32::from_rgb(22, 163, 74);
    pub const GREEN_700: egui::Color32 = egui::Color32::from_rgb(21, 128, 61);
    pub const GREEN_800: egui::Color32 = egui::Color32::from_rgb(22, 101, 52);
    pub const GREEN_900: egui::Color32 = egui::Color32::from_rgb(20, 83, 45);

    pub const RED_50: egui::Color32 = egui::Color32::from_rgb(254, 242, 242);
    pub const RED_100: egui::Color32 = egui::Color32::from_rgb(254, 226, 226);
    pub const RED_200: egui::Color32 = egui::Color32::from_rgb(254, 202, 202);
    pub const RED_300: egui::Color32 = egui::Color32::from_rgb(252, 165, 165);
    pub const RED_400: egui::Color32 = egui::Color32::from_rgb(248, 113, 113);
    pub const RED_500: egui::Color32 = egui::Color32::from_rgb(239, 68, 68);
    pub const RED_600: egui::Color32 = egui::Color32::from_rgb(220, 38, 38);
    pub const RED_700: egui::Color32 = egui::Color32::from_rgb(185, 28, 28);
    pub const RED_800: egui::Color32 = egui::Color32::from_rgb(153, 27, 27);
    pub const RED_900: egui::Color32 = egui::Color32::from_rgb(127, 29, 29);

    pub const PURPLE_50: egui::Color32 = egui::Color32::from_rgb(250, 245, 255);
    pub const PURPLE_100: egui::Color32 = egui::Color32::from_rgb(243, 232, 255);
    pub const PURPLE_200: egui::Color32 = egui::Color32::from_rgb(233, 213, 255);
    pub const PURPLE_300: egui::Color32 = egui::Color32::from_rgb(216, 180, 254);
    pub const PURPLE_400: egui::Color32 = egui::Color32::from_rgb(192, 132, 252);
    pub const PURPLE_500: egui::Color32 = egui::Color32::from_rgb(168, 85, 247);
    pub const PURPLE_600: egui::Color32 = egui::Color32::from_rgb(147, 51, 234);
    pub const PURPLE_700: egui::Color32 = egui::Color32::from_rgb(126, 34, 206);
    pub const PURPLE_800: egui::Color32 = egui::Color32::from_rgb(107, 33, 168);
    pub const PURPLE_900: egui::Color32 = egui::Color32::from_rgb(88, 28, 135);

    pub const AMBER_50: egui::Color32 = egui::Color32::from_rgb(255, 251, 235);
    pub const AMBER_100: egui::Color32 = egui::Color32::from_rgb(254, 243, 199);
    pub const AMBER_200: egui::Color32 = egui::Color32::from_rgb(253, 230, 138);
    pub const AMBER_300: egui::Color32 = egui::Color32::from_rgb(252, 211, 77);
    pub const AMBER_400: egui::Color32 = egui::Color32::from_rgb(251, 191, 36);
    pub const AMBER_500: egui::Color32 = egui::Color32::from_rgb(245, 158, 11);
    pub const AMBER_600: egui::Color32 = egui::Color32::from_rgb(217, 119, 6);
    pub const AMBER_700: egui::Color32 = egui::Color32::from_rgb(180, 83, 9);
    pub const AMBER_800: egui::Color32 = egui::Color32::from_rgb(146, 64, 14);
    pub const AMBER_900: egui::Color32 = egui::Color32::from_rgb(120, 53, 15);

    pub const TEAL_50: egui::Color32 = egui::Color32::from_rgb(240, 253, 250);
    pub const TEAL_100: egui::Color32 = egui::Color32::from_rgb(204, 251, 241);
    pub const TEAL_200: egui::Color32 = egui::Color32::from_rgb(153, 246, 228);
    pub const TEAL_300: egui::Color32 = egui::Color32::from_rgb(94, 234, 212);
    pub const TEAL_400: egui::Color32 = egui::Color32::from_rgb(45, 212, 191);
    pub const TEAL_500: egui::Color32 = egui::Color32::from_rgb(20, 184, 166);
    pub const TEAL_600: egui::Color32 = egui::Color32::from_rgb(13, 148, 136);
    pub const TEAL_700: egui::Color32 = egui::Color32::from_rgb(15, 118, 110);
    pub const TEAL_800: egui::Color32 = egui::Color32::from_rgb(17, 94, 89);
    pub const TEAL_900: egui::Color32 = egui::Color32::from_rgb(19, 78, 74);
}

pub fn apply_theme(ctx: &egui::Context, theme: ThemeMode) {
    let mut style = (*ctx.style()).clone();

    style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);
    
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(10);

    match theme {
        ThemeMode::Dark => apply_dark_theme(&mut style),
        ThemeMode::Light => apply_light_theme(&mut style),
    }

    ctx.set_style(style);
}

fn apply_dark_theme(style: &mut egui::Style) {
    style.visuals.dark_mode = true;
    
    style.visuals.panel_fill = ColorPalette::ZINC_900;
    style.visuals.window_fill = ColorPalette::ZINC_900;
    style.visuals.faint_bg_color = ColorPalette::ZINC_800;
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(12, 12, 15);
    
    style.visuals.widgets.noninteractive.bg_fill = ColorPalette::ZINC_800;
    style.visuals.widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(22, 22, 26);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, ColorPalette::ZINC_700);
    
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(30, 30, 35);
    style.visuals.widgets.inactive.weak_bg_fill = ColorPalette::ZINC_800;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, ColorPalette::ZINC_600);
    
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(40, 40, 48);
    style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(35, 35, 42);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ColorPalette::ZINC_500);
    
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(50, 50, 60);
    style.visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(45, 45, 55);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ColorPalette::ZINC_400);
    
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ColorPalette::SLATE_300);
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ColorPalette::SLATE_200);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ColorPalette::SLATE_100);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    
    style.visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(60, 120, 240, 100);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 0, 0));
    style.visuals.hyperlink_color = ColorPalette::BLUE_400;
}

fn apply_light_theme(style: &mut egui::Style) {
    style.visuals.dark_mode = false;
    
    style.visuals.panel_fill = ColorPalette::GRAY_50;
    style.visuals.window_fill = ColorPalette::GRAY_50;
    style.visuals.faint_bg_color = ColorPalette::GRAY_100;
    style.visuals.extreme_bg_color = egui::Color32::WHITE;
    
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::WHITE;
    style.visuals.widgets.noninteractive.weak_bg_fill = ColorPalette::GRAY_50;
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, ColorPalette::GRAY_300);
    
    style.visuals.widgets.inactive.bg_fill = ColorPalette::GRAY_50;
    style.visuals.widgets.inactive.weak_bg_fill = ColorPalette::GRAY_100;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, ColorPalette::GRAY_300);
    
    style.visuals.widgets.hovered.bg_fill = ColorPalette::GRAY_100;
    style.visuals.widgets.hovered.weak_bg_fill = ColorPalette::GRAY_200;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ColorPalette::GRAY_400);
    
    style.visuals.widgets.active.bg_fill = ColorPalette::GRAY_200;
    style.visuals.widgets.active.weak_bg_fill = ColorPalette::GRAY_300;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ColorPalette::GRAY_500);
    
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ColorPalette::GRAY_700);
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ColorPalette::GRAY_800);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ColorPalette::GRAY_900);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
    
    style.visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(60, 120, 240, 80);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, ColorPalette::BLUE_600);
    style.visuals.hyperlink_color = ColorPalette::BLUE_600;
}

pub fn primary_button(ui: &mut egui::Ui, text: &str, theme: ThemeMode) -> egui::Response {
    let (bg_color, hover_color) = match theme {
        ThemeMode::Dark => (ColorPalette::BLUE_600, ColorPalette::BLUE_500),
        ThemeMode::Light => (ColorPalette::BLUE_600, ColorPalette::BLUE_500),
    };
    
    ui.scope(|ui| {
        let style = ui.style_mut();
        style.visuals.widgets.inactive.bg_fill = bg_color;
        style.visuals.widgets.inactive.weak_bg_fill = bg_color;
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;

        style.visuals.widgets.hovered.bg_fill = hover_color;
        style.visuals.widgets.hovered.weak_bg_fill = hover_color;
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        
        style.visuals.widgets.active.bg_fill = bg_color;
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;

        let button = egui::Button::new(
            egui::RichText::new(text).size(16.0)
        )
        .min_size(egui::vec2(220.0, 45.0));
        
        ui.add(button)
    }).inner
}

pub fn secondary_button(ui: &mut egui::Ui, text: &str, theme: ThemeMode) -> egui::Response {
    let (bg_color, stroke_color, text_color, hover_bg) = match theme {
        ThemeMode::Dark => (
            ColorPalette::ZINC_800,
            ColorPalette::ZINC_600,
            ColorPalette::SLATE_200,
            ColorPalette::ZINC_700,
        ),
        ThemeMode::Light => (
            egui::Color32::WHITE,
            ColorPalette::GRAY_300,
            ColorPalette::GRAY_800,
            ColorPalette::GRAY_50,
        ),
    };
    
    ui.scope(|ui| {
        let style = ui.style_mut();
        
        style.visuals.widgets.inactive.bg_fill = bg_color;
        style.visuals.widgets.inactive.weak_bg_fill = bg_color;
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_color);
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, stroke_color);

        style.visuals.widgets.hovered.bg_fill = hover_bg;
        style.visuals.widgets.hovered.weak_bg_fill = hover_bg;
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, text_color);
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, stroke_color);

        style.visuals.widgets.active.bg_fill = bg_color;
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, text_color);

        let button = egui::Button::new(
            egui::RichText::new(text).size(16.0)
        )
        .min_size(egui::vec2(220.0, 45.0));
        
        ui.add(button)
    }).inner
}
