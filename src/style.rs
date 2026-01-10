use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeMode {
    Light,
    Dark,
}

pub fn apply_theme(ctx: &egui::Context, theme: ThemeMode) {
    let mut style = (*ctx.style()).clone();

    // Common styling - using CornerRadius instead of deprecated Rounding
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
    
    // Background colors
    style.visuals.panel_fill = egui::Color32::from_rgb(18, 18, 22);
    style.visuals.window_fill = egui::Color32::from_rgb(18, 18, 22);
    style.visuals.faint_bg_color = egui::Color32::from_rgb(25, 25, 30);
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(12, 12, 15);
    
    // Widget colors
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(25, 25, 30);
    style.visuals.widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(22, 22, 26);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 45));
    
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(30, 30, 35);
    style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(25, 25, 30);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 60));
    
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(40, 40, 48);
    style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(35, 35, 42);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 70, 85));
    
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(50, 50, 60);
    style.visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(45, 45, 55);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 90, 110));
    
    // Text colors
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 210));
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(210, 210, 220));
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(230, 230, 240));
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 255, 255));
    
    // Selection and hyperlinks
    style.visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(60, 120, 240, 100);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 140, 255));
    style.visuals.hyperlink_color = egui::Color32::from_rgb(100, 160, 255);
}

fn apply_light_theme(style: &mut egui::Style) {
    style.visuals.dark_mode = false;
    
    // Background colors - soft, modern light theme
    style.visuals.panel_fill = egui::Color32::from_rgb(248, 249, 250);
    style.visuals.window_fill = egui::Color32::from_rgb(248, 249, 250);
    style.visuals.faint_bg_color = egui::Color32::from_rgb(240, 242, 245);
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(255, 255, 255);
    
    // Widget colors
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(255, 255, 255);
    style.visuals.widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(250, 251, 252);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 223, 228));
    
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(252, 253, 254);
    style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(248, 249, 250);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(210, 214, 220));
    
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(245, 247, 250);
    style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(242, 244, 248);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 195, 205));
    
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(235, 238, 243);
    style.visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(230, 234, 240);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(170, 178, 190));
    
    // Text colors
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 65, 75));
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 45, 55));
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(20, 25, 35));
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 0, 0));
    
    // Selection and hyperlinks
    style.visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(60, 120, 240, 80);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 100, 220));
    style.visuals.hyperlink_color = egui::Color32::from_rgb(30, 100, 220);
}

pub fn primary_button(ui: &mut egui::Ui, text: &str, theme: ThemeMode) -> egui::Response {
    let (bg_color, hover_color) = match theme {
        ThemeMode::Dark => (
            egui::Color32::from_rgb(60, 120, 240),
            egui::Color32::from_rgb(70, 130, 250),
        ),
        ThemeMode::Light => (
            egui::Color32::from_rgb(45, 105, 220),
            egui::Color32::from_rgb(55, 115, 230),
        ),
    };
    
    let button = egui::Button::new(
        egui::RichText::new(text)
            .size(16.0)
            .color(egui::Color32::WHITE)
    )
    .min_size(egui::vec2(220.0, 45.0))
    .fill(bg_color)
    .stroke(egui::Stroke::NONE);
    
    let response = ui.add(button);
    
    if response.hovered() {
        ui.painter().rect_filled(
            response.rect,
            4.0,
            hover_color,
        );
    }
    
    response
}

pub fn secondary_button(ui: &mut egui::Ui, text: &str, theme: ThemeMode) -> egui::Response {
    let (bg_color, stroke_color, text_color) = match theme {
        ThemeMode::Dark => (
            egui::Color32::from_rgb(45, 45, 55),
            egui::Color32::from_rgb(80, 80, 90),
            egui::Color32::from_rgb(220, 220, 230),
        ),
        ThemeMode::Light => (
            egui::Color32::from_rgb(255, 255, 255),
            egui::Color32::from_rgb(200, 205, 215),
            egui::Color32::from_rgb(40, 45, 55),
        ),
    };
    
    let button = egui::Button::new(
        egui::RichText::new(text)
            .size(16.0)
            .color(text_color)
    )
    .min_size(egui::vec2(220.0, 45.0))
    .fill(bg_color)
    .stroke(egui::Stroke::new(1.0, stroke_color));
    
    ui.add(button)
}
