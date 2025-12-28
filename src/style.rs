use eframe::egui;

pub fn configure_modern_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Rounding
    style.visuals.window_rounding = egui::Rounding::same(8.0);
    style.visuals.menu_rounding = egui::Rounding::same(6.0);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);

    // Color Palette
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = egui::Color32::from_rgb(18, 18, 22);
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(25, 25, 30);
    
    // Spacing for readability
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(10.0);

    ctx.set_style(style);
}

pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let button = egui::Button::new(
        egui::RichText::new(text)
            .size(16.0)
    )
    .min_size(egui::vec2(220.0, 45.0))
    .fill(egui::Color32::from_rgb(60, 120, 240))
    .stroke(egui::Stroke::NONE);
    
    ui.add(button)
}

pub fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let button = egui::Button::new(
        egui::RichText::new(text)
            .size(16.0)
    )
    .min_size(egui::vec2(220.0, 45.0))
    .fill(egui::Color32::from_rgb(45, 45, 55))
    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 90)));
    
    ui.add(button)
}
