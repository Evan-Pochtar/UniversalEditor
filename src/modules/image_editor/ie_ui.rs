use eframe::egui;
use crate::style::{ColorPalette, ThemeMode, toolbar_action_btn, toolbar_toggle_btn};
use crate::modules::helpers::image_export::ExportFormat;
use super::ie_main::{ImageEditor, Tool, FilterPanel, TransformHandleSet, THandle, RgbaColor, CropState, TextDrag, HANDLE_HIT, BrushShape, BrushTextureMode, BrushPreset, SavedBrush, RetouchMode, LayerKind, BlendMode, TextLayer, ColorHistory, MAX_COLOR_FAVORITES, COLOR_FAV_HOTKEYS, ImageDrag};
use super::ie_helpers::{rgb_to_hsv_f32, hsv_to_rgb_f32, crop_hit_handle, draw_crop_handles};

impl ImageEditor {
    pub(super) fn render_toolbar(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let (bg, border) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::ZINC_700)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::GRAY_300)
        };

        egui::Frame::new()
            .fill(bg).stroke(egui::Stroke::new(1.0, border))
            .corner_radius(6.0)
            .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 4 })
            .show(ui, |ui: &mut egui::Ui| {
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, true])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                    .min_scrolled_height(32.0)
                    .show(ui, |ui: &mut egui::Ui| {
                        ui.horizontal(|ui: &mut egui::Ui| {
                            self.tool_btn(ui, "Brush", Tool::Brush, Some("B"), theme);
                            self.tool_btn(ui, "Eraser", Tool::Eraser, Some("E"), theme);
                            self.tool_btn(ui, "Fill", Tool::Fill, Some("F"), theme);
                            self.tool_btn(ui, "Text", Tool::Text, Some("T"), theme);
                            self.tool_btn(ui, "Eyedrop", Tool::Eyedropper, Some("D"), theme);
                            self.tool_btn(ui, "Crop", Tool::Crop, Some("C"), theme);
                            self.tool_btn(ui, "Select/Pan", Tool::Pan, Some("P"), theme);
                            self.tool_btn(ui, "Retouch", Tool::Retouch, Some("R"), theme);
                        });
                    });
            });
    }

    fn tool_btn(&mut self, ui: &mut egui::Ui, label: &str, tool: Tool, shortcut: Option<&str>, theme: ThemeMode) {
        let active: bool = self.tool == tool;
        let btn = toolbar_toggle_btn(ui, egui::RichText::new(label).size(12.0), active, theme);
        let response: egui::Response = if let Some(sc) = shortcut { btn.on_hover_text(sc) } else { btn };

        if response.clicked() {
            if tool != Tool::Text { self.commit_or_discard_active_text(); }
            self.tool = tool;
        }
    }

    pub(super) fn render_options_bar(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        ui.spacing_mut().slider_width = 200.0;
        let (bg, border, label_col) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::ZINC_700, ColorPalette::ZINC_400)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::GRAY_300, ColorPalette::ZINC_600)
        };

        egui::Frame::new()
            .fill(bg).stroke(egui::Stroke::new(1.0, border))
            .corner_radius(6.0)
            .inner_margin(egui::Margin { left: 8, right: 8, top: 3, bottom: 3 })
            .show(ui, |ui: &mut egui::Ui| {
                ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 28.0), egui::Layout::left_to_right(egui::Align::Center), |ui: &mut egui::Ui| {
                    ui.style_mut().spacing.interact_size.y = 28.0;
                    match self.tool {
                        Tool::Brush => {
                            ui.label(egui::RichText::new("Size:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.brush.size, 1.0..=200.0));
                            ui.label(egui::RichText::new("Opacity:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.brush.opacity, 0.0..=1.0).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)));
                            ui.separator();

                            let settings_active = self.filter_panel == FilterPanel::Brush;
                            if toolbar_toggle_btn(ui, egui::RichText::new("Brush Settings").size(12.0), settings_active, theme).clicked() {
                                self.filter_panel = if settings_active { FilterPanel::None } else { FilterPanel::Brush };
                            }
                        }
                        Tool::Eraser => {
                            ui.label(egui::RichText::new("Size:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.eraser_size, 1.0..=200.0));
                            ui.separator();
                            let cb = ui.add(egui::Checkbox::new(&mut self.eraser_transparent, egui::RichText::new("Remove Background").size(12.0).color(label_col)));
                            cb.on_hover_text("When checked, erases pixels to transparent instead of white.\nUseful for removing image backgrounds.");
                        }
                        Tool::Text => {
                            ui.label(egui::RichText::new("Font:").size(12.0).color(label_col));
                            egui::ComboBox::from_id_salt("text_font_pick")
                                .selected_text(self.text_font_name.clone()).width(90.0)
                                .show_ui(ui, |ui| {
                                    for f in &["Ubuntu", "Roboto"] {
                                        if ui.selectable_label(self.text_font_name == *f, *f).clicked() {
                                            self.text_font_name = f.to_string();
                                            if let Some(id) = self.selected_text {
                                                if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) {
                                                    layer.font_name = f.to_string();
                                                }
                                            }
                                        }
                                    }
                                });
                            ui.separator();
                            ui.label(egui::RichText::new("Size:").size(12.0).color(label_col));
                            let mut fs: f32 = self.text_font_size;
                            if ui.add(egui::DragValue::new(&mut fs).range(6.0..=400.0).speed(1.0)).changed() {
                                self.text_font_size = fs;
                                if let Some(id) = self.selected_text {
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) { layer.font_size = fs; }
                                }
                            }
                            ui.separator();

                            if toolbar_toggle_btn(ui, egui::RichText::new("B").strong().size(13.0), self.text_bold, theme).clicked() {
                                self.text_bold = !self.text_bold;
                                if let Some(id) = self.selected_text {
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) { layer.bold = self.text_bold; }
                                }
                            }
                            if toolbar_toggle_btn(ui, egui::RichText::new("I").italics().size(13.0), self.text_italic, theme).clicked() {
                                self.text_italic = !self.text_italic;
                                if let Some(id) = self.selected_text {
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) { layer.italic = self.text_italic; }
                                }
                            }
                            if toolbar_toggle_btn(ui, egui::RichText::new("U").underline().size(13.0), self.text_underline, theme).clicked() {
                                self.text_underline = !self.text_underline;
                                if let Some(id) = self.selected_text {
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) { layer.underline = self.text_underline; }
                                }
                            }

                            if let Some(id) = self.selected_text {
                                let cur_color = self.color;
                                if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) {
                                    if layer.color != cur_color { layer.color = cur_color; }
                                }
                                if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) {
                                    ui.separator();
                                    ui.label(egui::RichText::new("Rot:").size(12.0).color(label_col));
                                    ui.add(egui::DragValue::new(&mut layer.rotation).speed(1.0).range(-360.0..=360.0).suffix("°")).on_hover_text("Rotation in degrees");
                                }
                                if ui.button("Deselect").clicked() { self.commit_or_discard_active_text(); }
                                if ui.button("Delete").clicked() {
                                    self.text_layers.retain(|l: &TextLayer| l.id != id);
                                    self.selected_text = None; self.editing_text = false;
                                }
                            }
                            if !self.text_layers.is_empty() {
                                ui.separator();
                                ui.label(egui::RichText::new(format!("{} layer(s)", self.text_layers.len())).size(11.0).color(label_col));
                            }
                        }
                        Tool::Pan => {
                            let has_img_sel = self.selected_image_layer.is_some();
                            let has_txt_sel = self.selected_text.is_some();
                            let in_select = has_img_sel || has_txt_sel;
                            let mode_label = if in_select { "Mode: Select" } else { "Mode: Pan" };
                            ui.label(egui::RichText::new(mode_label).size(12.0).color(if in_select { ColorPalette::GREEN_400 } else { label_col }));
                            if in_select {
                                ui.separator();
                                if ui.button(egui::RichText::new("Deselect").size(12.0)).clicked() {
                                    self.selected_image_layer = None;
                                    self.commit_or_discard_active_text();
                                }
                            }
                            if let Some(iid) = self.image_layer_for_active() {
                                if let Some(ild) = self.image_layer_data.get(&iid) {
                                    let ow = ild.orig_w(); let oh = ild.orig_h();
                                    let dw = ild.display_w; let dh = ild.display_h;
                                    let aspect = ild.native_aspect();
                                    let gcd = { let mut a = ow; let mut b = oh; while b != 0 { let t = b; b = a % b; a = t; } a.max(1) };
                                    let (rw, rh) = (ow / gcd, oh / gcd);
                                    ui.separator();
                                    ui.label(egui::RichText::new(format!("{}x{}", ow, oh)).size(12.0).color(label_col)).on_hover_text("Native resolution");
                                    ui.label(egui::RichText::new(format!("{}:{}", rw, rh)).size(12.0).color(label_col)).on_hover_text("Aspect ratio");
                                    ui.separator();
                                    let mut dw_edit = dw.round() as i32;
                                    let mut dh_edit = dh.round() as i32;
                                    ui.label(egui::RichText::new("W:").size(12.0).color(label_col));
                                    if ui.add(egui::DragValue::new(&mut dw_edit).range(1..=16000).speed(1.0).suffix("px")).changed() {
                                        if let Some(ild2) = self.image_layer_data.get_mut(&iid) {
                                            ild2.display_w = dw_edit as f32;
                                            if self.image_aspect_lock { ild2.display_h = (dw_edit as f32 / aspect).round().max(1.0); }
                                            self.composite_dirty = true; self.dirty = true;
                                        }
                                    }
                                    ui.label(egui::RichText::new("H:").size(12.0).color(label_col));
                                    if ui.add(egui::DragValue::new(&mut dh_edit).range(1..=16000).speed(1.0).suffix("px")).changed() {
                                        if let Some(ild2) = self.image_layer_data.get_mut(&iid) {
                                            ild2.display_h = dh_edit as f32;
                                            if self.image_aspect_lock { ild2.display_w = (dh_edit as f32 * aspect).round().max(1.0); }
                                            self.composite_dirty = true; self.dirty = true;
                                        }
                                    }
                                    let lock_label = if self.image_aspect_lock { "Lock" } else { "Free" };
                                    if toolbar_toggle_btn(ui, egui::RichText::new(lock_label).size(12.0), self.image_aspect_lock, theme).on_hover_text("Lock aspect ratio").clicked() {
                                        self.image_aspect_lock = !self.image_aspect_lock;
                                    }
                                    ui.separator();
                                    let mut rot = self.image_layer_data.get(&iid).map(|d| d.rotation).unwrap_or(0.0);
                                    ui.label(egui::RichText::new("Rot:").size(12.0).color(label_col));
                                    if ui.add(egui::DragValue::new(&mut rot).range(-360.0..=360.0).speed(0.5).suffix("°")).changed() {
                                        if let Some(ild2) = self.image_layer_data.get_mut(&iid) { ild2.rotation = rot; self.composite_dirty = true; self.dirty = true; }
                                    }
                                    ui.separator();
                                    if toolbar_action_btn(ui, egui::RichText::new("Flip H").size(12.0), theme).clicked() { self.push_undo(); self.flip_image_layer_h(); }
                                    if toolbar_action_btn(ui, egui::RichText::new("Flip V").size(12.0), theme).clicked() { self.push_undo(); self.flip_image_layer_v(); }
                                    if toolbar_action_btn(ui, egui::RichText::new("Fit").size(12.0), theme).on_hover_text("Fit image layer to canvas").clicked() { self.push_undo(); self.fit_image_layer_to_canvas(); }
                                    if toolbar_action_btn(ui, egui::RichText::new("1:1").size(12.0), theme).on_hover_text("Reset to native size").clicked() { self.push_undo(); self.reset_image_layer_size(); }
                                    if toolbar_action_btn(ui, egui::RichText::new("Rasterize").size(12.0), theme).on_hover_text("Merge image layer into a raster layer").clicked() { self.rasterize_image_layer(); }
                                }
                            }
                        }
                        Tool::Eyedropper | Tool::Fill => {}
                        Tool::Crop => {
                            if self.crop_state.start.is_some() && self.crop_state.end.is_some() {
                                let is_img_layer = self.image_layer_for_active().is_some();
                                if ui.button("Apply Crop").clicked() {
                                    if is_img_layer { self.apply_crop_to_image_layer(); }
                                    else { self.push_undo(); self.apply_crop(); }
                                }
                                if ui.button("Cancel").clicked() { self.crop_state = CropState::default(); }
                                if is_img_layer {
                                    ui.separator();
                                    ui.label(egui::RichText::new("Cropping image layer").size(11.0).color(label_col));
                                }
                            }
                        }
                        Tool::Retouch => {
                            egui::ScrollArea::horizontal()
                                .auto_shrink([false, true])
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                                .show(ui, |ui: &mut egui::Ui| {
                                    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 36.0), egui::Layout::left_to_right(egui::Align::Center), |ui: &mut egui::Ui| {
                                        ui.style_mut().spacing.interact_size.y = 28.0;
                                        for mode in RetouchMode::all() {
                                            let active = self.retouch_mode == *mode;
                                            if toolbar_toggle_btn(ui, egui::RichText::new(mode.label()).size(11.5), active, theme).clicked() {
                                                self.retouch_mode = *mode;
                                            }
                                        }
                                        ui.separator();
                                        ui.label(egui::RichText::new("Size:").size(12.0).color(label_col));
                                        ui.add(egui::Slider::new(&mut self.retouch_size, 1.0..=200.0));
                                        ui.separator();
                                        match self.retouch_mode {
                                            RetouchMode::Brightness => {
                                                ui.spacing_mut().slider_width = 230.0;
                                                ui.add_space(4.0);
                                                ui.label(egui::RichText::new("Amount:").size(12.0).color(label_col));
                                                ui.add_space(8.0);
                                                gradient_slider_ui(ui, &mut self.retouch_strength, 0.0, 1.0,
                                                    egui::Color32::from_rgb(18, 18, 18), egui::Color32::from_rgb(255, 255, 240),
                                                    "Dark", "Light", |v| format!("{:.0}%", v * 100.0), true, 100.0, "%",
                                                );
                                            }
                                            RetouchMode::Temperature => {
                                                ui.spacing_mut().slider_width = 230.0;
                                                ui.add_space(4.0);
                                                ui.label(egui::RichText::new("Shift:").size(12.0).color(label_col));
                                                ui.add_space(8.0);
                                                gradient_slider_ui(ui, &mut self.retouch_strength, 0.0, 1.0,
                                                    egui::Color32::from_rgb(70, 130, 220), egui::Color32::from_rgb(250, 150, 40),
                                                    "Cool", "Warm", |v| format!("{:.0}%", v * 100.0), true, 100.0, "%",
                                                );
                                            }
                                            RetouchMode::Vibrance => {
                                                ui.spacing_mut().slider_width = 230.0;
                                                ui.add_space(4.0);
                                                ui.label(egui::RichText::new("Boost:").size(12.0).color(label_col));
                                                ui.add_space(8.0);
                                                gradient_slider_ui(ui, &mut self.retouch_strength, 0.0, 1.0,
                                                    egui::Color32::from_rgb(130, 130, 130), egui::Color32::from_rgb(60, 190, 230),
                                                    "Muted", "Vivid", |v| format!("{:.0}%", v * 100.0), true, 100.0, "%",
                                                );
                                            }
                                            RetouchMode::Saturation => {
                                                ui.spacing_mut().slider_width = 230.0;
                                                ui.add_space(4.0);
                                                ui.label(egui::RichText::new("Amount:").size(12.0).color(label_col));
                                                ui.add_space(8.0);
                                                gradient_slider_ui(ui, &mut self.retouch_strength, 0.0, 1.0,
                                                    egui::Color32::from_rgb(100, 100, 100), egui::Color32::from_rgb(220, 50, 180),
                                                    "Muted", "Vivid", |v| format!("{:.0}%", v * 100.0), true, 100.0, "%",
                                                );
                                            }
                                            RetouchMode::Pixelate => {
                                                ui.label(egui::RichText::new("Block Size:").size(12.0).color(label_col));
                                                ui.add(egui::DragValue::new(&mut self.retouch_pixelate_block).range(2..=80).speed(0.5).suffix("px"));
                                            }
                                            _ => {
                                                ui.label(egui::RichText::new(format!("{}:", self.retouch_mode.strength_label())).size(12.0).color(label_col));
                                                ui.add(egui::Slider::new(&mut self.retouch_strength, 0.0..=1.0).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)));
                                            }
                                        }
                                        ui.separator();
                                        ui.spacing_mut().slider_width = 120.0;
                                        ui.label(egui::RichText::new("Softness:").size(12.0).color(label_col));
                                        ui.add(egui::Slider::new(&mut self.retouch_softness, 0.0..=1.0).show_value(false));
                                        let mut pct: i32 = (self.retouch_softness * 100.0).round() as i32;
                                        if ui.add(egui::DragValue::new(&mut pct).range(0..=100).speed(1).suffix("%")).changed() {
                                            self.retouch_softness = pct as f32 / 100.0;
                                        }
                                    });
                            });
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                        if self.tool != Tool::Retouch || self.tool != Tool::Pan {
                            if ui.add(egui::Button::new("").fill(self.color).min_size(egui::vec2(28.0, 28.0))).clicked() { self.show_color_picker = !self.show_color_picker; }
                            ui.label(egui::RichText::new("Color:").size(12.0).color(label_col));

                            if let Some(img) = &self.image {
                                ui.label(egui::RichText::new(format!("{}x{}", img.width(), img.height())).size(12.0).color(label_col));
                                ui.label(egui::RichText::new(format!("{:.0}%", self.zoom * 100.0)).size(12.0).color(label_col));
                                ui.label(egui::RichText::new("Zoom:").size(12.0).color(label_col));
                            }
                        }
                    });
                });
            });
    }

    pub(super) fn render_filter_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, theme: ThemeMode) {
        if self.filter_panel == FilterPanel::None {
            if self.filter_preview_active {
                self.cancel_filter_preview();
            }
            return;
        }
        let (bg, border, text_col, label_col) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::BLUE_600, ColorPalette::ZINC_100, ColorPalette::ZINC_400)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::BLUE_600, ColorPalette::GRAY_900, ColorPalette::ZINC_600)
        };
        let title = match self.filter_panel {
            FilterPanel::BrightnessContrast => "Brightness / Contrast",
            FilterPanel::HueSaturation => "Hue / Saturation",
            FilterPanel::Blur => "Gaussian Blur",
            FilterPanel::Sharpen => "Sharpen",
            FilterPanel::Resize => "Resize",
            FilterPanel::Export => "Export",
            FilterPanel::Brush => return self.render_brush_panel(ui, ctx, theme),
            FilterPanel::None => "",
        };

        let canvas_origin: egui::Pos2 = ui.available_rect_before_wrap().min;
        let modal_pos: egui::Pos2 = canvas_origin + egui::vec2(10.0, 10.0);
        let win_resp: Option<egui::InnerResponse<Option<()>>> = egui::Window::new(title)
            .collapsible(false).resizable(false)
            .fixed_pos(modal_pos)
            .fixed_size(egui::vec2(380.0, 0.0))
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.5, border)).corner_radius(8.0).inner_margin(16.0))
            .show(ctx, |ui: &mut egui::Ui| {
                ui.spacing_mut().slider_width = 250.0;
                if self.is_processing {
                    let progress_val: f32 = *self.filter_progress.lock().unwrap();
                    ui.label(egui::RichText::new("Processing Filter...").size(13.0).color(text_col));
                    ui.add_space(8.0);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(300.0), 28.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_200 });
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * progress_val, rect.height())),
                        4.0, ColorPalette::BLUE_500,
                    );
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
                        format!("{:.0}%", progress_val * 100.0),
                        egui::FontId::proportional(13.0), egui::Color32::WHITE,
                    );
                    return;
                }
                match self.filter_panel {
                    FilterPanel::BrightnessContrast => {
                        ui.horizontal(|ui: &mut egui::Ui| {
                            ui.label(egui::RichText::new("Brightness:").size(12.0).color(label_col));
                            gradient_slider_ui(
                                ui, &mut self.brightness, -100.0, 100.0,
                                egui::Color32::from_rgb(20, 20, 20), egui::Color32::from_rgb(255, 255, 240),
                                "Dark", "Light", |v| format!("{:.0}", v), true, 1.0, "",
                            );
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui: &mut egui::Ui| {
                            ui.label(egui::RichText::new("Contrast:    ").size(12.0).color(label_col));
                            gradient_slider_ui(
                                ui, &mut self.contrast, -100.0, 100.0,
                                egui::Color32::from_rgb(130, 130, 130), egui::Color32::from_rgb(10, 10, 10),
                                "Flat", "Bold", |v| format!("{:.0}", v), true, 1.0, "",
                            );
                        });
                        ui.add_space(8.0);
                        match filter_action_row(ui, theme, self.filter_preview_active) {
                            FilterAction::Preview => {
                                if self.filter_preview_active { self.cancel_filter_preview(); }
                                else {
                                    self.filter_preview_snapshot = Some(self.take_undo_snapshot());
                                    self.filter_preview_active = true;
                                    self.processing_is_preview = true;
                                    self.apply_brightness_contrast();
                                }
                            }
                            FilterAction::Apply => {
                                if self.filter_preview_active { self.accept_filter_preview(); } else { self.push_undo(); self.apply_brightness_contrast(); }
                                self.brightness = 0.0; self.contrast = 0.0; self.filter_panel = FilterPanel::None;
                            }
                            FilterAction::Cancel => {
                                if self.filter_preview_active { self.cancel_filter_preview(); }
                                self.brightness = 0.0; self.contrast = 0.0; self.filter_panel = FilterPanel::None;
                            }
                            FilterAction::None => {}
                        }
                    }
                    FilterPanel::HueSaturation => {
                        ui.horizontal(|ui: &mut egui::Ui| {
                            ui.label(egui::RichText::new("Saturation:").size(12.0).color(label_col));
                            gradient_slider_ui(
                                ui, &mut self.saturation, -100.0, 100.0,
                                egui::Color32::from_rgb(130, 130, 130), egui::Color32::from_rgb(220, 60, 60),
                                "Muted", "Vivid", |v| format!("{:.0}", v), true, 1.0, "",
                            );
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui: &mut egui::Ui| {
                            ui.label(egui::RichText::new("Hue:            ").size(12.0).color(label_col));
                            gradient_slider_ui(
                                ui, &mut self.hue, -180.0, 180.0,
                                egui::Color32::from_rgb(100, 80, 200), egui::Color32::from_rgb(230, 100, 40),
                                "-180", "+180", |v| format!("{:.0}deg", v), true, 1.0, "deg",
                            );
                        });
                        ui.add_space(8.0);
                        match filter_action_row(ui, theme, self.filter_preview_active) {
                            FilterAction::Preview => {
                                if self.filter_preview_active { self.cancel_filter_preview(); }
                                else {
                                    self.filter_preview_snapshot = Some(self.take_undo_snapshot());
                                    self.filter_preview_active = true;
                                    self.processing_is_preview = true;
                                    self.apply_hue_saturation();
                                }
                            }
                            FilterAction::Apply => {
                                if self.filter_preview_active { self.accept_filter_preview(); } else { self.push_undo(); self.apply_hue_saturation(); }
                                self.hue = 0.0; self.saturation = 0.0; self.filter_panel = FilterPanel::None;
                            }
                            FilterAction::Cancel => {
                                if self.filter_preview_active { self.cancel_filter_preview(); }
                                self.hue = 0.0; self.saturation = 0.0; self.filter_panel = FilterPanel::None;
                            }
                            FilterAction::None => {}
                        }
                    }
                    FilterPanel::Blur => {
                        ui.horizontal(|ui: &mut egui::Ui| {
                            ui.label(egui::RichText::new("Radius:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.blur_radius, 0.5..=20.0));
                        });
                        ui.add_space(4.0);
                        match filter_action_row(ui, theme, self.filter_preview_active) {
                            FilterAction::Preview => {
                                if self.filter_preview_active { self.cancel_filter_preview(); }
                                else {
                                    self.filter_preview_snapshot = Some(self.take_undo_snapshot());
                                    self.filter_preview_active = true;
                                    self.processing_is_preview = true;
                                    self.apply_blur();
                                }
                            }
                            FilterAction::Apply => {
                                if self.filter_preview_active { self.accept_filter_preview(); } else { self.push_undo(); self.apply_blur(); }
                                self.blur_radius = 3.0; self.filter_panel = FilterPanel::None;
                            }
                            FilterAction::Cancel => {
                                if self.filter_preview_active { self.cancel_filter_preview(); }
                                self.blur_radius = 3.0; self.filter_panel = FilterPanel::None;
                            }
                            FilterAction::None => {}
                        }
                    }
                    FilterPanel::Sharpen => {
                        ui.horizontal(|ui: &mut egui::Ui| { ui.label(egui::RichText::new("Amount:").size(12.0).color(label_col)); ui.add(egui::Slider::new(&mut self.sharpen_amount, 0.1..=1.5)); });
                        ui.add_space(4.0);
                        match filter_action_row(ui, theme, self.filter_preview_active) {
                            FilterAction::Preview => {
                                if self.filter_preview_active { self.cancel_filter_preview(); }
                                else {
                                    self.filter_preview_snapshot = Some(self.take_undo_snapshot());
                                    self.filter_preview_active = true;
                                    self.processing_is_preview = true;
                                    self.apply_sharpen();
                                }
                            }
                            FilterAction::Apply => {
                                if self.filter_preview_active { self.accept_filter_preview(); } else { self.push_undo(); self.apply_sharpen(); }
                                self.sharpen_amount = 1.0; self.filter_panel = FilterPanel::None;
                            }
                            FilterAction::Cancel => {
                                if self.filter_preview_active { self.cancel_filter_preview(); }
                                self.sharpen_amount = 1.0; self.filter_panel = FilterPanel::None;
                            }
                            FilterAction::None => {}
                        }
                    }
                    FilterPanel::Resize => {
                        ui.horizontal(|ui: &mut egui::Ui| {
                            ui.label(egui::RichText::new("Width:").size(12.0).color(label_col));
                            let old_w: u32 = self.resize_w;
                            ui.add(egui::DragValue::new(&mut self.resize_w).range(1..=8192));
                            if self.resize_locked && self.resize_w != old_w && old_w > 0 {
                                let ratio: f64 = self.resize_w as f64 / old_w as f64;
                                self.resize_h = (self.resize_h as f64 * ratio).max(1.0) as u32;
                            }
                            ui.label(egui::RichText::new("Height:").size(12.0).color(label_col));
                            let old_h: u32 = self.resize_h;
                            ui.add(egui::DragValue::new(&mut self.resize_h).range(1..=8192));
                            if self.resize_locked && self.resize_h != old_h && old_h > 0 {
                                let ratio: f64 = self.resize_h as f64 / old_h as f64;
                                self.resize_w = (self.resize_w as f64 * ratio).max(1.0) as u32;
                            }
                        });
                        ui.checkbox(&mut self.resize_locked,  "Lock Aspect Ratio");
                        ui.checkbox(&mut self.resize_stretch, "Stretch Image").on_hover_text("If unchecked, resizes canvas and pads with white/crops");
                        ui.horizontal(|ui: &mut egui::Ui| {
                            if ui.button("Apply").clicked()  { self.push_undo(); self.apply_resize(); }
                            if ui.button("Cancel").clicked() {
                                if let Some(img) = &self.image { self.resize_w = img.width(); self.resize_h = img.height(); }
                                self.filter_panel = FilterPanel::None;
                            }
                        });
                    }
                    FilterPanel::Export => {
                        ui.label(egui::RichText::new("Format:").size(12.0).color(label_col));
                        ui.horizontal_wrapped(|ui: &mut egui::Ui| {
                            for format in ExportFormat::all() {
                                let is_selected: bool = self.export_format == format;
                                let (bg_color, txt_color) = if is_selected {
                                    (ColorPalette::BLUE_600, egui::Color32::WHITE)
                                } else if matches!(theme, ThemeMode::Dark) {
                                    (ColorPalette::ZINC_700, ColorPalette::ZINC_300)
                                } else {
                                    (ColorPalette::GRAY_200, ColorPalette::GRAY_800)
                                };
                                let button: egui::Button<'_> = egui::Button::new(egui::RichText::new(format.as_str()).size(11.0).color(txt_color))
                                    .fill(bg_color).stroke(egui::Stroke::NONE).corner_radius(4.0).min_size(egui::vec2(50.0, 24.0));
                                if ui.add(button).clicked() { self.export_format = format; }
                            }
                        });
                        ui.add_space(8.0);
                        match self.export_format {
                            ExportFormat::Jpeg => {
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Quality:").size(12.0).color(label_col));
                                    ui.add(egui::Slider::new(&mut self.export_jpeg_quality, 1..=100).suffix("%"));
                                });
                            }
                            ExportFormat::Avif => {
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Quality:").size(12.0).color(label_col));
                                    ui.add(egui::Slider::new(&mut self.export_avif_quality, 1..=100).suffix("%"));
                                });
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Encode Speed:").size(12.0).color(label_col));
                                    ui.add(egui::Slider::new(&mut self.export_avif_speed, 0..=10));
                                });
                                let speed_desc = match self.export_avif_speed {
                                    0..=2 => "Slowest encode, smallest file size",
                                    3..=5 => "Balanced encode time and file size",
                                    6..=8 => "Faster encode, larger file size",
                                    _ =>     "Fastest encode, largest file size",
                                };
                                ui.label(egui::RichText::new(speed_desc).size(11.0).color(label_col).italics());
                            }
                            ExportFormat::Ico => {
                                ui.checkbox(&mut self.export_auto_scale_ico,
                                    egui::RichText::new("Auto-scale to 256px").size(12.0).color(label_col));
                            }
                            _ => {}
                        }
                        ui.checkbox(&mut self.export_preserve_metadata, egui::RichText::new("Preserve metadata").size(12.0).color(label_col));
                        ui.add_space(4.0);
                        ui.horizontal(|ui: &mut egui::Ui| {
                            if ui.button("Export").clicked() {
                                match self.export_image_to_file() {
                                    Ok(path) => { if let Some(cb) = &self.export_callback { cb(path); } }
                                    Err(e) => { eprintln!("Export error: {}", e); }
                                }
                            }
                            if ui.button("Cancel").clicked() { self.filter_panel = FilterPanel::None; }
                        });
                    }
                    FilterPanel::None | FilterPanel::Brush => {}
                }
            });
        self.filter_panel_rect = win_resp.map(|r| r.response.rect);
    }

    pub(super) fn render_color_picker(&mut self, _ui: &mut egui::Ui, ctx: &egui::Context, theme: ThemeMode) {
        if !self.show_color_picker { return; }
        let (bg, border, text_col, weak_col) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_800, ColorPalette::BLUE_600, ColorPalette::ZINC_100, ColorPalette::ZINC_400)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::BLUE_600, ColorPalette::GRAY_900, ColorPalette::ZINC_600)
        };
        let win_resp = egui::Window::new("Color Picker")
            .collapsible(false).resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 60.0))
            .default_size(egui::vec2(330.0, 580.0))
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.5, border)).corner_radius(8.0).inner_margin(16.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("cp_scroll")
                    .auto_shrink([false, false])
                    .min_scrolled_height(540.0)
                    .max_height(ctx.content_rect().height() - 120.0)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                    .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                let mut rgb: [f32; 3] = [self.color.r() as f32 / 255.0, self.color.g() as f32 / 255.0, self.color.b() as f32 / 255.0];
                let (h_curr, s_curr, v_curr) = rgb_to_hsv_f32(rgb[0], rgb[1], rgb[2]);
                let hue_id = egui::Id::new("ie_cp_hue");
                let sv_id  = egui::Id::new("ie_cp_sv");
                let mut h: f32 = ctx.data(|d| d.get_temp(hue_id)).unwrap_or(h_curr);
                let (mut s, mut v): (f32, f32) = ctx.data(|d| d.get_temp(sv_id)).unwrap_or((s_curr, v_curr));

                let picker_w: f32 = 280.0;
                let avail_w = ui.available_width();
                let x_offset = ((avail_w - picker_w) / 2.0).max(0.0);
                let mut color_changed = false;
                let mut sq_used = false;
                let mut hue_used = false;

                let (outer_sq, _) = ui.allocate_exact_size(egui::vec2(avail_w, picker_w), egui::Sense::hover());
                let rect = egui::Rect::from_min_size(egui::pos2(outer_sq.min.x + x_offset, outer_sq.min.y), egui::vec2(picker_w, picker_w));
                let response = ui.interact(rect, ui.id().with("cp_sq"), egui::Sense::click_and_drag());
                if ui.is_rect_visible(rect) {
                    let painter = ui.painter_at(rect);
                    let steps = 40i32;
                    let (cw, ch) = (rect.width() / steps as f32, rect.height() / steps as f32);
                    for cy in 0..steps {
                        for cx in 0..steps {
                            let (sc, vc) = (cx as f32 / (steps - 1) as f32, 1.0 - cy as f32 / (steps - 1) as f32);
                            let (r, g, b) = hsv_to_rgb_f32(h, sc, vc);
                            painter.rect_filled(egui::Rect::from_min_size(
                                egui::pos2(rect.min.x + cx as f32 * cw, rect.min.y + cy as f32 * ch),
                                egui::vec2(cw.ceil(), ch.ceil()),
                            ), 0.0, egui::Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8));
                        }
                    }
                    let cur = egui::pos2(rect.min.x + s * rect.width(), rect.min.y + (1.0 - v) * rect.height());
                    painter.circle_stroke(cur, 6.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
                    painter.circle_stroke(cur, 6.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
                }
                if response.dragged() || response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let (x, y) = (((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0), ((pos.y - rect.min.y) / rect.height()).clamp(0.0, 1.0));
                        s = x; v = 1.0 - y;
                        ctx.data_mut(|d| d.insert_temp(sv_id, (s, v)));
                        let (r, g, b) = hsv_to_rgb_f32(h, s, v);
                        rgb = [r, g, b]; color_changed = true; sq_used = true;
                    }
                }

                ui.add_space(4.0);

                let (outer_hue, _) = ui.allocate_exact_size(egui::vec2(avail_w, 24.0), egui::Sense::hover());
                let hue_rect = egui::Rect::from_min_size(egui::pos2(outer_hue.min.x + x_offset, outer_hue.min.y), egui::vec2(picker_w, 24.0));
                let hue_resp = ui.interact(hue_rect, ui.id().with("cp_hue"), egui::Sense::click_and_drag());
                if ui.is_rect_visible(hue_rect) {
                    let painter = ui.painter_at(hue_rect);
                    let steps = 60i32;
                    let sw = hue_rect.width() / steps as f32;
                    for i in 0..steps {
                        let (r, g, b) = hsv_to_rgb_f32((i as f32 / steps as f32) * 360.0, 1.0, 1.0);
                        painter.rect_filled(egui::Rect::from_min_size(
                            egui::pos2(hue_rect.min.x + i as f32 * sw, hue_rect.min.y),
                            egui::vec2(sw.ceil(), hue_rect.height()),
                        ), 0.0, egui::Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8));
                    }
                    painter.rect_stroke(hue_rect, 2.0, egui::Stroke::new(1.0,
                        if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 }
                    ), egui::StrokeKind::Outside);
                    let hx = hue_rect.min.x + (h / 360.0) * hue_rect.width();
                    let hcr = egui::Rect::from_center_size(egui::pos2(hx, hue_rect.center().y), egui::vec2(4.0, hue_rect.height() + 2.0));
                    painter.rect_filled(hcr, 2.0, egui::Color32::WHITE);
                    painter.rect_stroke(hcr, 2.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Outside);
                }
                if hue_resp.dragged() || hue_resp.clicked() {
                    if let Some(pos) = hue_resp.interact_pointer_pos() {
                        h = ((pos.x - hue_rect.min.x) / hue_rect.width()).clamp(0.0, 1.0) * 360.0;
                        ctx.data_mut(|d| d.insert_temp(hue_id, h));
                        let (r, g, b) = hsv_to_rgb_f32(h, s, v);
                        rgb = [r, g, b]; color_changed = true; hue_used = true;
                    }
                }

                if !sq_used && !hue_used {
                    let (er, eg, eb) = hsv_to_rgb_f32(h, s, v);
                    let expected = egui::Color32::from_rgb((er * 255.0) as u8, (eg * 255.0) as u8, (eb * 255.0) as u8);
                    if expected != self.color {
                        ctx.data_mut(|d| { d.insert_temp(hue_id, h_curr); d.insert_temp(sv_id, (s_curr, v_curr)); });
                    }
                }

                self.color = egui::Color32::from_rgb((rgb[0] * 255.0) as u8, (rgb[1] * 255.0) as u8, (rgb[2] * 255.0) as u8);
                if color_changed { self.hex_input = RgbaColor::from_egui(self.color).to_hex(); }

                ui.add_space(4.0); ui.separator(); ui.add_space(4.0);
                ui.label(egui::RichText::new("Color Values").size(13.0).color(text_col));
                ui.horizontal(|ui: &mut egui::Ui| {
                    ui.label(egui::RichText::new("RGB:").size(12.0).color(weak_col));
                    let rgb_str: String = RgbaColor::from_egui(self.color).to_rgb_string();
                    ui.label(egui::RichText::new(&rgb_str).size(12.0).color(text_col).monospace());
                    if ui.small_button("Copy").clicked() { ctx.copy_text(rgb_str); }
                });

                ui.horizontal(|ui: &mut egui::Ui| {
                    ui.label(egui::RichText::new("Hex:").size(12.0).color(weak_col));
                    let response: egui::Response = ui.add(egui::TextEdit::singleline(&mut self.hex_input).desired_width(120.0));
                    if response.changed() {
                        if let Some(mut c) = RgbaColor::from_hex(&self.hex_input) { c.a = 255; self.color = c.to_egui(); }
                    }
                    if response.lost_focus() { self.hex_input = RgbaColor::from_egui(self.color).to_hex(); }
                    if ui.small_button("Copy").clicked() { ctx.copy_text(self.hex_input.clone()); }
                });

                ui.add_space(4.0); ui.separator(); ui.add_space(4.0);
                ui.horizontal(|ui: &mut egui::Ui| {
                    ui.label(egui::RichText::new("Recent").size(13.0).color(text_col));
                    if ui.small_button("Clear").clicked() { self.color_history = ColorHistory::default(); }
                });

                {
                    let history = self.color_history.get_colors().clone();
                    let n = history.len();
                    let (sw, sp) = (28.0f32, 4.0f32);
                    let avail = ui.available_width();
                    let per_row = ((avail + sp) / (sw + sp)).floor().max(1.0) as usize;
                    let rows = if n == 0 { 1 } else { (n + per_row - 1) / per_row };
                    let total_h = if n == 0 { sw } else { rows as f32 * (sw + sp) - sp };
                    let origin = ui.cursor().min;
                    let (rec_rect, _) = ui.allocate_exact_size(egui::vec2(avail, total_h), egui::Sense::hover());
                    let painter = ui.painter_at(rec_rect);
                    let ptr = ctx.pointer_latest_pos();
                    let released = ctx.input(|i| i.pointer.any_released());
                    for (idx, color) in history.iter().enumerate() {
                        let (row, col) = (idx / per_row, idx % per_row);
                        let items_this_row = if (row + 1) * per_row <= n { per_row } else { n - row * per_row };
                        let row_w = items_this_row as f32 * (sw + sp) - sp;
                        let lpad = ((avail - row_w) / 2.0).max(0.0);
                        let sr = egui::Rect::from_min_size(
                            egui::pos2(origin.x + lpad + col as f32 * (sw + sp), origin.y + row as f32 * (sw + sp)),
                            egui::vec2(sw, sw),
                        );
                        painter.rect_filled(sr, 4.0, color.to_egui());
                        painter.rect_stroke(sr, 4.0, egui::Stroke::new(1.0,
                            if matches!(theme, ThemeMode::Dark) { egui::Color32::from_rgba_unmultiplied(255,255,255,40) }
                            else { egui::Color32::from_rgba_unmultiplied(0,0,0,40) }
                        ), egui::StrokeKind::Outside);
                        if let Some(pp) = ptr {
                            if sr.contains(pp) {
                                ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                                if released { let mut c = *color; c.a = 255; self.color = c.to_egui(); self.hex_input = c.to_hex(); }
                            }
                        }
                    }
                }

                ui.add_space(4.0); ui.separator(); ui.add_space(4.0);

                let current_rgba = RgbaColor::from_egui(self.color);
                let is_fav = self.color_favorites.contains(current_rgba);
                let fav_count = self.color_favorites.colors.len();
                ui.horizontal(|ui: &mut egui::Ui| {
                    ui.label(egui::RichText::new("Favorites").size(13.0).color(text_col));
                    ui.label(egui::RichText::new(format!("{}/30", fav_count)).size(11.0).color(weak_col));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let add_label = if is_fav { "Remove" } else { "Add Current" };
                        let add_enabled = is_fav || fav_count < MAX_COLOR_FAVORITES;
                        if ui.add_enabled(add_enabled, egui::Button::new(egui::RichText::new(add_label).size(11.0))).clicked() {
                            self.color_favorites.toggle(current_rgba);
                        }
                    });
                });
                ui.label(egui::RichText::new("Keys 1-9, 0 activate the first 10. Drag to reorder.").size(10.0).color(weak_col));
                ui.add_space(2.0);

                let swatch_size: f32 = 28.0;
                let swatch_spacing: f32 = 4.0;
                let available_w: f32 = ui.available_width();
                let swatches_per_row: usize = ((available_w + swatch_spacing) / (swatch_size + swatch_spacing)).max(1.0) as usize;
                let fav_colors_snapshot: Vec<RgbaColor> = self.color_favorites.colors.clone();
                let n = fav_colors_snapshot.len();
                let rows = (n + swatches_per_row - 1).max(1) / swatches_per_row.max(1);
                let total_h = rows as f32 * (swatch_size + swatch_spacing) - if rows > 0 { swatch_spacing } else { 0.0 };
                let grid_start = ui.cursor().min;
                let (grid_rect, _) = ui.allocate_exact_size(
                    egui::vec2(available_w, total_h.max(swatch_size)),
                    egui::Sense::hover(),
                );

                let pointer_pos: Option<egui::Pos2> = ctx.pointer_latest_pos();
                let pointer_released: bool = ctx.input(|i| i.pointer.any_released());
                let pointer_down: bool = ctx.input(|i| i.pointer.any_down());

                let mut swatch_rects: Vec<egui::Rect> = Vec::with_capacity(n);
                for idx in 0..n {
                    let row = idx / swatches_per_row.max(1);
                    let col = idx % swatches_per_row.max(1);
                    let items_in_row = if (row + 1) * swatches_per_row <= n { swatches_per_row } else { n - row * swatches_per_row };
                    let row_w = items_in_row as f32 * (swatch_size + swatch_spacing) - swatch_spacing;
                    let row_pad = ((available_w - row_w) / 2.0).max(0.0);
                    let x = grid_start.x + row_pad + col as f32 * (swatch_size + swatch_spacing);
                    let y = grid_start.y + row as f32 * (swatch_size + swatch_spacing);
                    swatch_rects.push(egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(swatch_size, swatch_size)));
                }

                let hovered_drop_idx: Option<usize> = if self.color_fav_drag_src.is_some() {
                    pointer_pos.and_then(|pp| swatch_rects.iter().position(|r| r.expand(2.0).contains(pp)))
                } else { None };

                if self.color_fav_drag_src.is_none() && pointer_down {
                    if let Some(pp) = pointer_pos {
                        if let Some(drag_idx) = swatch_rects.iter().position(|r| r.contains(pp)) {
                            let pressed_this_frame = ctx.input(|i| i.pointer.any_pressed());
                            if pressed_this_frame {
                                self.color_fav_drag_src = Some(drag_idx);
                            }
                        }
                    }
                }

                if pointer_released {
                    if let (Some(src), Some(dst)) = (self.color_fav_drag_src, hovered_drop_idx) {
                        if src != dst {
                            self.color_favorites.move_item(src, dst);
                        }
                    }
                    if let Some(src) = self.color_fav_drag_src {
                        if hovered_drop_idx.is_none() || hovered_drop_idx == Some(src) {
                            let drag_delta = ctx.input(|i| i.pointer.delta().length());
                            if drag_delta < 2.0 {
                                if let Some(c) = fav_colors_snapshot.get(src) {
                                    let mut col = *c; col.a = 255; self.color = col.to_egui(); self.hex_input = col.to_hex();
                                }
                            }
                        }
                    }
                    self.color_fav_drag_src = None;
                }

                let painter = ui.painter_at(grid_rect);
                let is_dragging = self.color_fav_drag_src.is_some();

                for (idx, (color, rect)) in fav_colors_snapshot.iter().zip(swatch_rects.iter()).enumerate() {
                    let egui_color = color.to_egui();
                    let is_drag_src = self.color_fav_drag_src == Some(idx);
                    let is_drop_target = hovered_drop_idx == Some(idx) && self.color_fav_drag_src.map_or(false, |s| s != idx);
                    let is_active_key_slot = idx < COLOR_FAV_HOTKEYS;
                    let alpha = if is_drag_src { 80u8 } else { 255u8 };
                    let draw_color = egui::Color32::from_rgba_premultiplied(
                        ((egui_color.r() as u32 * alpha as u32) / 255) as u8,
                        ((egui_color.g() as u32 * alpha as u32) / 255) as u8,
                        ((egui_color.b() as u32 * alpha as u32) / 255) as u8,
                        alpha,
                    );
                    painter.rect_filled(*rect, 4.0, draw_color);

                    if is_drop_target {
                        painter.rect_stroke(*rect, 4.0, egui::Stroke::new(2.5, egui::Color32::WHITE), egui::StrokeKind::Outside);
                        let line_x = rect.min.x - 3.0;
                        painter.line_segment(
                            [egui::pos2(line_x, rect.min.y), egui::pos2(line_x, rect.max.y)],
                            egui::Stroke::new(3.0, egui::Color32::WHITE),
                        );
                    } else {
                        let border_col = if matches!(theme, ThemeMode::Dark) {
                            egui::Color32::from_rgba_unmultiplied(255,255,255,40)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(0,0,0,40)
                        };
                        painter.rect_stroke(*rect, 4.0, egui::Stroke::new(1.0, border_col), egui::StrokeKind::Outside);
                    }

                    if is_active_key_slot && !is_drag_src {
                        let key_label = if idx == 9 { "0".to_string() } else { format!("{}", idx + 1) };
                        let badge_rect = egui::Rect::from_min_size(rect.min, egui::vec2(12.0, 12.0));
                        painter.rect_filled(badge_rect, egui::CornerRadius { nw: 4, ne: 0, sw: 0, se: 4 },
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180));
                        painter.text(badge_rect.center(), egui::Align2::CENTER_CENTER, &key_label,
                            egui::FontId::monospace(8.0), egui::Color32::WHITE);
                    }

                    if !is_dragging {
                        if let Some(pp) = pointer_pos {
                            if rect.contains(pp) {
                                ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                            }
                        }
                    }
                }

                if let Some(src_idx) = self.color_fav_drag_src {
                    if pointer_down {
                        if let Some(pp) = pointer_pos {
                            if let Some(drag_col) = fav_colors_snapshot.get(src_idx) {
                                let float_rect = egui::Rect::from_center_size(pp, egui::vec2(swatch_size, swatch_size));
                                let float_painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("fav_drag_float")));
                                float_painter.rect_filled(float_rect, 4.0, drag_col.to_egui());
                                float_painter.rect_stroke(float_rect, 4.0, egui::Stroke::new(2.0, egui::Color32::WHITE), egui::StrokeKind::Outside);
                                ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
                            }
                        }
                    }
                }

                if let Some(pp) = pointer_pos {
                    if let Some(ctx_idx) = swatch_rects.iter().position(|r| r.contains(pp)) {
                        if ctx.input(|i| i.pointer.secondary_clicked()) {
                            self.color_favorites.colors.remove(ctx_idx);
                            self.color_favorites.save();
                        }
                    }
                }

                ui.add_space(8.0);
                ui.horizontal(|ui: &mut egui::Ui| {
                    if ui.button("Apply").clicked()  { self.add_color_to_history(); self.show_color_picker = false; }
                    if ui.button("Cancel").clicked() { self.show_color_picker = false; }
                });
                    });
            });
        self.color_picker_rect = win_resp.map(|r| r.response.rect);
    }

    pub(super) fn render_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let canvas_rect: egui::Rect = ui.available_rect_before_wrap();
        self.canvas_rect = Some(canvas_rect);
        if self.fit_on_next_frame { self.fit_image(); self.fit_on_next_frame = false; }
        self.ensure_texture(ctx);
        let (rect, response) = ui.allocate_exact_size(canvas_rect.size(), egui::Sense::click_and_drag());
        let painter: egui::Painter = ui.painter_at(rect);

        let checker_tid = self.ensure_checker_texture(ctx);
        let tile = 32.0_f32;
        let uv = egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(rect.width() / tile, rect.height() / tile),
        );
        painter.image(checker_tid, rect, uv, egui::Color32::WHITE);

        if let (Some(tex), Some(img)) = (&self.texture, &self.image) {
            let (img_w, img_h) = (img.width() as f32, img.height() as f32);
            let center: egui::Pos2  = canvas_rect.center();
            let img_rect: egui::Rect = egui::Rect::from_center_size(
                egui::pos2(center.x + self.pan.x, center.y + self.pan.y),
                egui::vec2(img_w * self.zoom, img_h * self.zoom),
            );
            painter.image(*tex, img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            painter.rect_stroke(img_rect, 0.0, egui::Stroke::new(1.0, ColorPalette::ZINC_500), egui::StrokeKind::Outside);
        }

        self.ensure_raster_layer_textures(ctx);
        self.ensure_image_layer_textures(ctx);

        let zoom = self.zoom;
        let editing_text = self.editing_text;
        let selected_text = self.selected_text;
        let text_cursor = self.text_cursor;
        let text_sel_anchor = self.text_sel_anchor;
        let mut text_galleys: std::collections::HashMap<u64, std::sync::Arc<egui::Galley>> = std::collections::HashMap::new();
        for i in 0..self.text_layers.len() {
            let tl = &self.text_layers[i];
            let font_size_screen = tl.font_size * zoom;
            let font_family = egui::FontFamily::Name(tl.font_family_name().into());
            let font_id = egui::FontId::new(font_size_screen, font_family);
            let box_w_screen = tl.box_width.map(|w| w * zoom).unwrap_or(f32::INFINITY);
            let layer_color = tl.color;
            let layer_underline = tl.underline;
            let content_snap = tl.content.clone();
            let layer_font_size = tl.font_size;
            let tid = tl.id;
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = box_w_screen;
            job.append(&content_snap, 0.0, egui::TextFormat {
                font_id: font_id.clone(), color: layer_color, italics: false,
                underline: if layer_underline {
                    egui::Stroke::new((font_size_screen * 0.06).max(1.0), layer_color)
                } else { egui::Stroke::NONE },
                ..Default::default()
            });
            let galley = ui.painter().layout_job(job);
            self.text_layers[i].rendered_height = (galley.rect.height() / zoom).max(layer_font_size);
            let content_chars: Vec<char> = content_snap.chars().collect();
            let mut char_ptr = 0usize;
            let mut new_cached: Vec<String> = Vec::with_capacity(galley.rows.len());
            for row in &galley.rows {
                let n = row.glyphs.len();
                let end = (char_ptr + n).min(content_chars.len());
                new_cached.push(content_chars[char_ptr..end].iter().collect());
                char_ptr = end;
                if char_ptr < content_chars.len() && content_chars[char_ptr] == '\n' { char_ptr += 1; }
            }
            self.text_layers[i].cached_lines = new_cached;
            text_galleys.insert(tid, galley);
        }

        {
            let (img_w, img_h) = self.image.as_ref().map(|i| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
            let selected_iid = self.selected_image_layer;
            let layers_snap: Vec<(u64, LayerKind, Option<u64>, f32, bool)> = self.layers.iter().filter(|l| l.visible && l.kind != LayerKind::Background)
                .map(|l| (l.id, l.kind, l.linked_image_id.or(l.linked_text_id), l.opacity, l.id == self.active_layer_id)).collect();
            for (lid, kind, linked_id, layer_opacity, is_active) in &layers_snap {
                let alpha = (layer_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
                match kind {
                    LayerKind::Background => {}
                    LayerKind::Raster => {
                        if let Some(&tid) = self.raster_layer_textures.get(lid) {
                            let center = canvas_rect.center();
                            let raster_rect = egui::Rect::from_center_size(
                                egui::pos2(center.x + self.pan.x, center.y + self.pan.y),
                                egui::vec2(img_w * self.zoom, img_h * self.zoom),
                            );
                            painter.image(tid, raster_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha));
                        }
                    }
                    LayerKind::Image => {
                        if let Some(iid) = linked_id {
                            let iid = *iid;
                            if let (Some(&tex_id), Some(ild)) = (
                                self.image_layer_textures.get(&iid),
                                self.image_layer_data.get(&iid),
                            ) {
                                let screen_rect = ild.screen_rect(img_w, img_h, canvas_rect, self.zoom, self.pan);
                                let angle_rad = ild.rotation.to_radians();
                                let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
                                if angle_rad == 0.0 && !ild.flip_h && !ild.flip_v {
                                    painter.image(tex_id, screen_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), tint);
                                } else {
                                    let (u0, v0, u1, v1) = (
                                        if ild.flip_h { 1.0 } else { 0.0 },
                                        if ild.flip_v { 1.0 } else { 0.0 },
                                        if ild.flip_h { 0.0 } else { 1.0 },
                                        if ild.flip_v { 0.0 } else { 1.0 },
                                    );
                                    let center = screen_rect.center();
                                    let (cos_a, sin_a) = (angle_rad.cos(), angle_rad.sin());
                                    let rot = |p: egui::Pos2| -> egui::Pos2 {
                                        let d = p - center;
                                        center + egui::vec2(d.x*cos_a - d.y*sin_a, d.x*sin_a + d.y*cos_a)
                                    };
                                    let corners = [
                                        rot(screen_rect.left_top()), rot(screen_rect.right_top()),
                                        rot(screen_rect.right_bottom()), rot(screen_rect.left_bottom()),
                                    ];
                                    let uvs = [
                                        egui::pos2(u0,v0), egui::pos2(u1,v0),
                                        egui::pos2(u1,v1), egui::pos2(u0,v1),
                                    ];
                                    painter.add(egui::Shape::mesh({
                                        let mut mesh = egui::Mesh::with_texture(tex_id);
                                        mesh.vertices = corners.iter().zip(uvs.iter())
                                            .map(|(&pos, &uv)| egui::epaint::Vertex { pos, uv, color: tint })
                                            .collect();
                                        mesh.indices = vec![0,1,2, 0,2,3];
                                        mesh
                                    }));
                                }
                                if *is_active && selected_iid == Some(iid) {
                                    if let Some(handles) = self.image_layer_transform_handles() {
                                        handles.draw(&painter, ColorPalette::GREEN_400);
                                    }
                                    let gcd = { let mut a = ild.orig_w(); let mut b = ild.orig_h(); while b != 0 { let t = b; b = a % b; a = t; } a.max(1) };
                                    let (rw, rh) = (ild.orig_w()/gcd, ild.orig_h()/gcd);
                                    let aspect_label = format!("{}x{}  {}:{}", ild.orig_w(), ild.orig_h(), rw, rh);
                                    let screen_r = ild.screen_rect(img_w, img_h, canvas_rect, self.zoom, self.pan);
                                    let label_pos = egui::pos2(screen_r.min.x, screen_r.min.y - 18.0)
                                        .max(canvas_rect.min + egui::vec2(2.0, 2.0));
                                    painter.text(label_pos + egui::vec2(1.0, 1.0), egui::Align2::LEFT_TOP, &aspect_label, egui::FontId::proportional(11.0), egui::Color32::from_black_alpha(180));
                                    painter.text(label_pos, egui::Align2::LEFT_TOP, &aspect_label, egui::FontId::proportional(11.0), egui::Color32::WHITE);
                                }
                            }
                        }
                    }
                    LayerKind::Text => {
                        if let Some(tid) = linked_id {
                            let tid = *tid;
                            if let Some(tl_idx) = self.text_layers.iter().position(|t| t.id == tid) {
                                let tl = &self.text_layers[tl_idx];
                                let anchor = self.image_to_screen(tl.img_x, tl.img_y);
                                let font_size_screen = tl.font_size * zoom;
                                let layer_color = tl.color;
                                let content_snap = tl.content.clone();
                                let angle_rad = tl.rotation.to_radians();
                                let (cos_a, sin_a) = (angle_rad.cos(), angle_rad.sin());
                                let sel_rect = tl.screen_rect(anchor, zoom);
                                let center = sel_rect.center();
                                let d = anchor - center;
                                let text_pos = center + egui::vec2(d.x * cos_a - d.y * sin_a, d.x * sin_a + d.y * cos_a);
                                let is_editing = editing_text && selected_text == Some(tid);
                                let effective_alpha = (layer_color.a() as f32 * layer_opacity).clamp(0.0, 255.0) as u8;
                                let draw_color = egui::Color32::from_rgba_unmultiplied(
                                    layer_color.r(), layer_color.g(), layer_color.b(), effective_alpha);

                                if let Some(galley) = text_galleys.get(&tid).cloned() {
                                    let mut text_shape = egui::epaint::TextShape::new(text_pos, galley.clone(), draw_color);
                                    text_shape.angle = angle_rad;

                                    if is_editing {
                                        let cursor_byte = text_cursor;
                                        let sel_anchor_opt = text_sel_anchor;
                                        let galley_to_canvas = |lp: egui::Pos2| -> egui::Pos2 {
                                            text_pos + egui::vec2(lp.x * cos_a - lp.y * sin_a, lp.x * sin_a + lp.y * cos_a)
                                        };
                                        let glyph_pos_for = |byte_off: usize| -> egui::Pos2 {
                                            let char_idx = content_snap[..byte_off.min(content_snap.len())].chars().count();
                                            let mut ci = 0usize;
                                            for row in &galley.rows {
                                                for g in &row.glyphs {
                                                    if ci == char_idx { return egui::pos2(g.pos.x, row.rect().min.y); }
                                                    ci += 1;
                                                }
                                                if ci == char_idx { return egui::pos2(row.rect().max.x, row.rect().min.y); }
                                            }
                                            galley.rows.last().map(|r: &egui::epaint::text::PlacedRow| egui::pos2(r.rect().max.x, r.rect().min.y)).unwrap_or(egui::pos2(0.0, 0.0))
                                        };
                                        if let Some(anchor_sel) = sel_anchor_opt {
                                            let (lo, hi) = (anchor_sel.min(cursor_byte), anchor_sel.max(cursor_byte));
                                            let char_lo = content_snap[..lo.min(content_snap.len())].chars().count();
                                            let char_hi = content_snap[..hi.min(content_snap.len())].chars().count();
                                            let mut ci = 0usize;
                                            for row in &galley.rows {
                                                let row_start = ci; let row_end = ci + row.glyphs.len();
                                                let sel_start = char_lo.max(row_start);
                                                let sel_end = char_hi.min(row_end);
                                                if sel_start < sel_end || (char_lo <= row_start && char_hi >= row_end) {
                                                    let x0 = if sel_start <= row_start { row.rect().min.x } else { row.glyphs.get(sel_start - row_start).map(|g| g.pos.x).unwrap_or(row.rect().min.x) };
                                                    let x1 = if sel_end >= row_end { row.rect().max.x } else { row.glyphs.get(sel_end - row_start).map(|g| g.pos.x).unwrap_or(row.rect().max.x) };
                                                    let corners = [
                                                        galley_to_canvas(egui::pos2(x0, row.rect().min.y)),
                                                        galley_to_canvas(egui::pos2(x1, row.rect().min.y)),
                                                        galley_to_canvas(egui::pos2(x1, row.rect().max.y)),
                                                        galley_to_canvas(egui::pos2(x0, row.rect().max.y)),
                                                    ];
                                                    painter.add(egui::Shape::convex_polygon(corners.to_vec(), egui::Color32::from_rgba_unmultiplied(100, 140, 255, 80), egui::Stroke::NONE));
                                                }
                                                ci = row_end;
                                            }
                                        }
                                        let blink = (ctx.input(|i: &egui::InputState| i.time) * 2.0) as u32 % 2 == 0;
                                        if blink {
                                            let lp = glyph_pos_for(cursor_byte);
                                            let row_h = galley.rows.iter()
                                                .find(|r| r.rect().min.y <= lp.y && lp.y <= r.rect().max.y)
                                                .map(|r| r.rect().height()).unwrap_or(font_size_screen);
                                            painter.line_segment(
                                                [galley_to_canvas(lp), galley_to_canvas(egui::pos2(lp.x, lp.y + row_h))],
                                                egui::Stroke::new(2.0, layer_color));
                                        }
                                        ctx.request_repaint_after(std::time::Duration::from_millis(500));
                                    }
                                    painter.add(egui::Shape::Text(text_shape));
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(sel_tid) = self.selected_text {
            if let Some(tl) = self.text_layers.iter().find(|t| t.id == sel_tid) {
                let anchor = self.image_to_screen(tl.img_x, tl.img_y);
                let sel_rect = tl.screen_rect(anchor, self.zoom);
                let angle_rad = tl.rotation.to_radians();
                TransformHandleSet::with_rotation(sel_rect, angle_rad)
                    .draw(&painter, ColorPalette::BLUE_400);
            }
        }

        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        for dropped in dropped_files {
            if let Some(path) = &dropped.path {
                let img_opt = image::ImageReader::open(path)
                    .ok()
                    .and_then(|r| r.with_guessed_format().ok())
                    .and_then(|r| r.decode().ok())
                    .or_else(|| image::open(path).ok());
                if let Some(img) = img_opt {
                    self.insert_image_layer(img, true);
                }
            } else if let Some(bytes) = &dropped.bytes {
                if let Ok(img) = image::load_from_memory(bytes) {
                    self.insert_image_layer(img, true);
                }
            }
        }
        let dragging_over = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if dragging_over && canvas_rect.contains(ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO))) {
            painter.rect_stroke(canvas_rect, 4.0, egui::Stroke::new(3.0, ColorPalette::GREEN_400), egui::StrokeKind::Inside);
            painter.text(canvas_rect.center(), egui::Align2::CENTER_CENTER, "Drop image to place", egui::FontId::proportional(18.0), egui::Color32::WHITE);
        }

        if self.tool == Tool::Crop {
            if let (Some(s), Some(e)) = (self.crop_state.start, self.crop_state.end) {
                let p0: egui::Pos2 = self.image_to_screen(s.0, s.1);
                let p1: egui::Pos2 = self.image_to_screen(e.0, e.1);
                let crop_rect = egui::Rect::from_two_pos(p0, p1);
                let overlay: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, 60);

                if crop_rect.min.y > canvas_rect.min.y { painter.rect_filled(egui::Rect::from_min_max(canvas_rect.min, egui::pos2(canvas_rect.max.x, crop_rect.min.y)), 0.0, overlay); }
                if crop_rect.max.y < canvas_rect.max.y { painter.rect_filled(egui::Rect::from_min_max(egui::pos2(canvas_rect.min.x, crop_rect.max.y), canvas_rect.max), 0.0, overlay); }
                if crop_rect.min.x > canvas_rect.min.x { painter.rect_filled(egui::Rect::from_min_max(egui::pos2(canvas_rect.min.x, crop_rect.min.y), egui::pos2(crop_rect.min.x, crop_rect.max.y)), 0.0, overlay); }
                if crop_rect.max.x < canvas_rect.max.x { painter.rect_filled(egui::Rect::from_min_max(egui::pos2(crop_rect.max.x, crop_rect.min.y), egui::pos2(canvas_rect.max.x, crop_rect.max.y)), 0.0, overlay); }

                painter.rect_stroke(crop_rect, 0.0, egui::Stroke::new(2.0, ColorPalette::BLUE_400), egui::StrokeKind::Outside);
                draw_crop_handles(&painter, crop_rect, ColorPalette::BLUE_400);

                let (img_w, img_h) = self.image.as_ref().map(|i| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
                let min_img = egui::pos2(s.0.min(e.0).clamp(0.0, img_w), s.1.min(e.1).clamp(0.0, img_h));
                let max_img = egui::pos2(s.0.max(e.0).clamp(0.0, img_w), s.1.max(e.1).clamp(0.0, img_h));
                let pw = (max_img.x - min_img.x).round() as u32;
                let ph = (max_img.y - min_img.y).round() as u32;
                let label = format!("{} x {}", pw, ph);
                let raw_tp = egui::pos2(crop_rect.min.x + 4.0, crop_rect.min.y - 18.0);
                let text_pos = egui::pos2(raw_tp.x.max(canvas_rect.min.x + 4.0), raw_tp.y.max(canvas_rect.min.y + 4.0));

                painter.text(text_pos + egui::vec2(1.0, 1.0), egui::Align2::LEFT_TOP, &label, egui::FontId::proportional(12.0), egui::Color32::from_black_alpha(160));
                painter.text(text_pos, egui::Align2::LEFT_TOP, &label, egui::FontId::proportional(12.0), egui::Color32::WHITE);
            }
        }

        let mouse_pos: Option<egui::Pos2> = ui.input(|i: &egui::InputState| i.pointer.latest_pos());
        if let Some(mp) = mouse_pos {
            let over_picker: bool = self.show_color_picker && self.color_picker_rect.map_or(false, |r| r.contains(mp));
            let over_filter: bool = self.filter_panel != FilterPanel::None && self.filter_panel_rect.map_or(false, |r| r.contains(mp));
            let over_modal: bool = over_picker || over_filter;
            if response.hovered() && !over_modal {
                match self.tool {
                    Tool::Brush | Tool::Eraser => ctx.set_cursor_icon(egui::CursorIcon::None),
                    Tool::Fill | Tool::Eyedropper | Tool::Crop => ctx.set_cursor_icon(egui::CursorIcon::Crosshair),
                    Tool::Pan => {
                        let dragging = response.dragged_by(egui::PointerButton::Primary);
                        if let Some(h) = self.image_layer_transform_handles().and_then(|hs| hs.hit_test(mp)) {
                            ctx.set_cursor_icon(TransformHandleSet::cursor_for(h));
                        } else if let Some(h) = self.text_transform_handles().and_then(|hs| hs.hit_test(mp)) {
                            ctx.set_cursor_icon(TransformHandleSet::cursor_for(h));
                        } else {
                            let over_image = self.layers.iter().any(|l| l.kind == LayerKind::Image && l.visible && l.linked_image_id.map_or(false, |iid| {
                                self.image_layer_data.get(&iid).map_or(false, |ild| {
                                    let (img_w, img_h) = self.image.as_ref().map(|i|(i.width() as f32,i.height() as f32)).unwrap_or((1.0,1.0));
                                    ild.hit_test(mp, img_w, img_h, canvas_rect, self.zoom, self.pan)
                                })
                            }));
                            let over_text = self.hit_text_layer(mp).is_some();
                            ctx.set_cursor_icon(if dragging { egui::CursorIcon::Grabbing } else if over_image || over_text { egui::CursorIcon::PointingHand } else { egui::CursorIcon::AllScroll });
                        }
                    }
                    Tool::Text => ctx.set_cursor_icon(egui::CursorIcon::Text),
                    Tool::Retouch => ctx.set_cursor_icon(egui::CursorIcon::None),
                }
                match self.tool {
                    Tool::Brush  => { painter.circle_stroke(mp, self.brush.size  / 2.0 * self.zoom, egui::Stroke::new(1.5, self.color)); }
                    Tool::Eraser => { painter.circle_stroke(mp, self.eraser_size / 2.0 * self.zoom, egui::Stroke::new(1.5, ColorPalette::RED_400)); }
                    Tool::Retouch => {
                        let r: f32 = self.retouch_size / 2.0 * self.zoom;
                        painter.circle_stroke(mp, r, egui::Stroke::new(1.5, ColorPalette::PURPLE_400));
                        let tick: f32 = 4.0;
                        painter.line_segment([mp - egui::vec2(tick, 0.0), mp + egui::vec2(tick, 0.0)], egui::Stroke::new(1.0, ColorPalette::PURPLE_400));
                        painter.line_segment([mp - egui::vec2(0.0, tick), mp + egui::vec2(0.0, tick)], egui::Stroke::new(1.0, ColorPalette::PURPLE_400));
                    }
                    Tool::Text => {
                        if let Some(handles) = self.text_transform_handles() {
                            if let Some(h) = handles.hit_test(mp) { ctx.set_cursor_icon(TransformHandleSet::cursor_for(h)); }
                        }
                    }
                    Tool::Crop => {
                        if let (Some(s), Some(e)) = (self.crop_state.start, self.crop_state.end) {
                            let p0 = self.image_to_screen(s.0, s.1);
                            let p1 = self.image_to_screen(e.0, e.1);
                            let cr = egui::Rect::from_two_pos(p0, p1);
                            if let Some(h) = crop_hit_handle(mp, cr) {
                                ctx.set_cursor_icon(match h {
                                    THandle::Move => egui::CursorIcon::Move,
                                    THandle::N | THandle::S => egui::CursorIcon::ResizeVertical,
                                    THandle::E | THandle::W => egui::CursorIcon::ResizeHorizontal,
                                    THandle::NE | THandle::SW => egui::CursorIcon::ResizeNeSw,
                                    THandle::NW | THandle::SE => egui::CursorIcon::ResizeNwSe,
                                    _ => egui::CursorIcon::Crosshair,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary) && self.tool == Tool::Retouch {
            let pos: egui::Pos2 = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            if self.image_layer_for_active().is_some() {
                let (img_w, img_h) = self.image.as_ref().map(|i| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
                let canvas = self.canvas_rect.unwrap_or(egui::Rect::NOTHING);
                let ox = canvas.center().x - img_w * self.zoom / 2.0 + self.pan.x;
                let oy = canvas.center().y - img_h * self.zoom / 2.0 + self.pan.y;
                let cx = (pos.x - ox) / self.zoom; let cy = (pos.y - oy) / self.zoom;
                self.init_smudge_sample_image_layer(cx, cy);
            } else if let Some((ix, iy)) = self.screen_to_image(pos) {
                self.init_smudge_sample(ix, iy);
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary) {
            let pos: egui::Pos2 = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            if let Some(iid) = self.selected_image_layer {
                let allow_move = self.tool == Tool::Pan;
                if let Some(handles) = self.image_layer_transform_handles() {
                    if let Some(h) = handles.hit_test(pos) {
                        let use_handle = allow_move || h != THandle::Move;
                        if use_handle {
                            if let Some(ild) = self.image_layer_data.get(&iid) {
                                let rot_start = (pos - handles.rect.center()).angle();
                                self.image_drag = Some(ImageDrag {
                                    handle: h, start: pos,
                                    orig_x: ild.canvas_x, orig_y: ild.canvas_y,
                                    orig_w: ild.display_w, orig_h: ild.display_h,
                                    orig_rotation: ild.rotation, orig_rot_start_angle: rot_start,
                                });
                            }
                        }
                    }
                }
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary) && self.tool == Tool::Crop {
            let pos = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            let handle_hit = if let (Some(s), Some(e)) = (self.crop_state.start, self.crop_state.end) {
                let p0 = self.image_to_screen(s.0, s.1);
                let p1 = self.image_to_screen(e.0, e.1);
                let cr = egui::Rect::from_two_pos(p0, p1);
                if cr.width() > HANDLE_HIT && cr.height() > HANDLE_HIT { crop_hit_handle(pos, cr) } else { None }
            } else { None };
            if let Some(h) = handle_hit {
                let (s, e) = (self.crop_state.start.unwrap(), self.crop_state.end.unwrap());
                self.crop_drag = Some(h);
                self.crop_drag_orig = Some((s.0, s.1, e.0, e.1));
            } else {
                self.crop_state = CropState::default();
                self.crop_drag = None; self.crop_drag_orig = None;
                if let Some((ix, iy)) = self.screen_to_image(pos) {
                    self.crop_state.start = Some((ix as f32, iy as f32));
                }
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary) && (self.tool == Tool::Text || self.tool == Tool::Pan) {
            let pos: egui::Pos2 = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            self.text_drag = None;
            if self.tool == Tool::Pan && self.selected_text.is_none() {
                if let Some(hit) = self.hit_text_layer(pos) {
                    if self.selected_text != Some(hit) { self.commit_or_discard_active_text(); }
                    self.selected_text = Some(hit);
                    self.selected_image_layer = None;
                    self.image_drag = None;
                    if let Some(linked_layer) = self.layers.iter().find(|l| l.linked_text_id == Some(hit)) {
                        self.active_layer_id = linked_layer.id;
                    }
                }
            }

            if let Some(id) = self.selected_text {
                if let Some(handles) = self.text_transform_handles() {
                    if let Some(h) = handles.hit_test(pos) {
                        if let Some(layer) = self.text_layers.iter().find(|l: &&TextLayer| l.id == id) {
                            let anchor: egui::Pos2 = self.image_to_screen(layer.img_x, layer.img_y);
                            let rot_start: f32 = (pos - layer.screen_rect(anchor, self.zoom).center()).angle();
                            self.text_drag = Some(TextDrag {
                                handle: h, start: pos,
                                orig_img_x: layer.img_x, orig_img_y: layer.img_y,
                                orig_font_size: layer.font_size, orig_box_width: layer.box_width,
                                orig_box_height: layer.box_height, orig_rotation: layer.rotation,
                                orig_rot_start_angle: rot_start,
                            });
                        }
                    }
                }
            }
        }

        if response.dragged_by(egui::PointerButton::Primary) {
            let pos: egui::Pos2 = response.interact_pointer_pos().unwrap_or(canvas_rect.center());

            if let Some(drag_data) = self.image_drag.as_ref().map(|d| (d.handle, d.start, d.orig_x, d.orig_y, d.orig_w, d.orig_h, d.orig_rotation, d.orig_rot_start_angle)) {
                let (handle, drag_start, orig_x, orig_y, orig_w, orig_h, orig_rot, orig_rot_start) = drag_data;
                let zoom = self.zoom;
                if let Some(iid) = self.selected_image_layer {
                    if let Some(ild) = self.image_layer_data.get_mut(&iid) {
                        let aspect = ild.native_aspect();
                        let aspect_lock = self.image_aspect_lock;
                        let (img_w, img_h) = self.image.as_ref().map(|i| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
                        let canvas = self.canvas_rect.unwrap_or(egui::Rect::NOTHING);
                        let ox = canvas.center().x - img_w * zoom / 2.0 + self.pan.x;
                        let oy = canvas.center().y - img_h * zoom / 2.0 + self.pan.y;
                        let anchor_screen = egui::pos2(ox + orig_x * zoom, oy + orig_y * zoom);
                        let orig_w_screen = orig_w * zoom;
                        let orig_h_screen = orig_h * zoom;
                        let rot_center = anchor_screen + egui::vec2(orig_w_screen / 2.0, orig_h_screen / 2.0);
                        let min_sz = 8.0_f32;
                        match handle {
                            THandle::Move => { let delta = pos - drag_start; ild.canvas_x = orig_x + delta.x / zoom; ild.canvas_y = orig_y + delta.y / zoom; }
                            THandle::E => { ild.display_w = ((pos.x - anchor_screen.x).max(min_sz) / zoom).max(1.0); if aspect_lock { ild.display_h = (ild.display_w / aspect).max(1.0); } }
                            THandle::W => { let right = anchor_screen.x + orig_w_screen; let nw = (right - pos.x).max(min_sz); ild.display_w = (nw / zoom).max(1.0); ild.canvas_x = (pos.x - ox) / zoom; if aspect_lock { ild.display_h = (ild.display_w / aspect).max(1.0); } }
                            THandle::S => { ild.display_h = ((pos.y - anchor_screen.y).max(min_sz) / zoom).max(1.0); if aspect_lock { ild.display_w = (ild.display_h * aspect).max(1.0); } }
                            THandle::N => { let bottom = anchor_screen.y + orig_h_screen; let nh = (bottom - pos.y).max(min_sz); ild.display_h = (nh / zoom).max(1.0); ild.canvas_y = (pos.y - oy) / zoom; if aspect_lock { ild.display_w = (ild.display_h * aspect).max(1.0); } }
                            THandle::SE => { ild.display_w = ((pos.x - anchor_screen.x).max(min_sz) / zoom).max(1.0); ild.display_h = if aspect_lock { (ild.display_w / aspect).max(1.0) } else { ((pos.y - anchor_screen.y).max(min_sz) / zoom).max(1.0) }; }
                            THandle::NE => { let bottom = anchor_screen.y + orig_h_screen; let nh = (bottom - pos.y).max(min_sz); ild.display_w = ((pos.x - anchor_screen.x).max(min_sz) / zoom).max(1.0); ild.display_h = if aspect_lock { (ild.display_w / aspect).max(1.0) } else { (nh / zoom).max(1.0) }; ild.canvas_y = if aspect_lock { orig_y + orig_h - ild.display_h } else { (pos.y - oy) / zoom }; }
                            THandle::NW => { let right = anchor_screen.x + orig_w_screen; let bottom = anchor_screen.y + orig_h_screen; let nw = (right - pos.x).max(min_sz); let nh = (bottom - pos.y).max(min_sz); ild.display_w = (nw / zoom).max(1.0); ild.display_h = if aspect_lock { (ild.display_w / aspect).max(1.0) } else { (nh / zoom).max(1.0) }; ild.canvas_x = orig_x + orig_w - ild.display_w; ild.canvas_y = if aspect_lock { orig_y + orig_h - ild.display_h } else { ((bottom - nh) - (canvas_rect.center().y - img_h * zoom / 2.0 + self.pan.y)) / zoom }; }
                            THandle::SW => { let right = anchor_screen.x + orig_w_screen; let nw = (right - pos.x).max(min_sz); ild.display_w = (nw / zoom).max(1.0); ild.canvas_x = orig_x + orig_w - ild.display_w; ild.display_h = if aspect_lock { (ild.display_w / aspect).max(1.0) } else { ((pos.y - anchor_screen.y).max(min_sz) / zoom).max(1.0) }; }
                            THandle::Rotate => { let cur_angle = (pos - rot_center).angle(); ild.rotation = orig_rot + (cur_angle - orig_rot_start).to_degrees(); }
                        }
                        self.dirty = true;
                    }
                }
            } else {
            match self.tool {
                Tool::Brush | Tool::Eraser => {
                    if !self.is_dragging {
                        self.push_undo(); self.is_dragging = true; self.stroke_points.clear();
                        let aid = self.active_layer_id;
                        let needs_backdrop = self.tool == Tool::Brush && self.brush.wetness > 0.0
                            && self.layers.iter().find(|l| l.id == aid).map_or(false, |l| l.kind == LayerKind::Raster);
                        self.stroke_backdrop = if needs_backdrop {
                            self.backdrop_cache.lock().unwrap().clone()
                        } else { None };
                    }
                    if self.image_layer_for_active().is_some() {
                        let (img_w, img_h) = self.image.as_ref().map(|i| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
                        let ox = canvas_rect.center().x - img_w * self.zoom / 2.0 + self.pan.x;
                        let oy = canvas_rect.center().y - img_h * self.zoom / 2.0 + self.pan.y;
                        let cx = (pos.x - ox) / self.zoom; let cy = (pos.y - oy) / self.zoom;
                        self.stroke_points.push((cx, cy));
                        if self.stroke_points.len() >= 2 {
                            self.apply_brush_stroke();
                            let last = *self.stroke_points.last().unwrap();
                            self.stroke_points.clear(); self.stroke_points.push(last);
                        }
                    } else if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.stroke_points.push((ix as f32, iy as f32));
                        if self.stroke_points.len() >= 2 {
                            self.apply_brush_stroke();
                            let last: (f32, f32) = *self.stroke_points.last().unwrap();
                            self.stroke_points.clear(); self.stroke_points.push(last);
                        }
                    }
                }
                Tool::Retouch => {
                    if !self.is_dragging {
                        self.push_undo(); self.is_dragging = true; self.stroke_points.clear();
                        self.stroke_backdrop = None;
                    }
                    if self.image_layer_for_active().is_some() {
                        let (img_w, img_h) = self.image.as_ref().map(|i| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
                        let ox = canvas_rect.center().x - img_w * self.zoom / 2.0 + self.pan.x;
                        let oy = canvas_rect.center().y - img_h * self.zoom / 2.0 + self.pan.y;
                        let cx = (pos.x - ox) / self.zoom; let cy = (pos.y - oy) / self.zoom;
                        self.stroke_points.push((cx, cy));
                        if self.stroke_points.len() >= 2 {
                            self.apply_retouch_stroke();
                            let last = *self.stroke_points.last().unwrap();
                            self.stroke_points.clear(); self.stroke_points.push(last);
                        }
                    } else if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.stroke_points.push((ix as f32, iy as f32));
                        if self.stroke_points.len() >= 2 {
                            self.apply_retouch_stroke();
                            let last: (f32, f32) = *self.stroke_points.last().unwrap();
                            self.stroke_points.clear(); self.stroke_points.push(last);
                        }
                    }
                }
                Tool::Crop => {
                    if let Some(handle) = self.crop_drag {
                        if let Some((ox1, oy1, ox2, oy2)) = self.crop_drag_orig {
                            let (min_ix, min_iy) = (ox1.min(ox2), oy1.min(oy2));
                            let (max_ix, max_iy) = (ox1.max(ox2), oy1.max(oy2));
                            if let Some((ix, iy)) = self.screen_to_image(pos).map(|(x,y)|(x as f32, y as f32)) {
                                let (mut s, mut e) = ((min_ix, min_iy), (max_ix, max_iy));
                                match handle {
                                    THandle::N => s.1 = iy.min(e.1 - 1.0),
                                    THandle::S => e.1 = iy.max(s.1 + 1.0),
                                    THandle::W => s.0 = ix.min(e.0 - 1.0),
                                    THandle::E => e.0 = ix.max(s.0 + 1.0),
                                    THandle::NW => { s.0 = ix.min(e.0 - 1.0); s.1 = iy.min(e.1 - 1.0); }
                                    THandle::NE => { e.0 = ix.max(s.0 + 1.0); s.1 = iy.min(e.1 - 1.0); }
                                    THandle::SW => { s.0 = ix.min(e.0 - 1.0); e.1 = iy.max(s.1 + 1.0); }
                                    THandle::SE => { e.0 = ix.max(s.0 + 1.0); e.1 = iy.max(s.1 + 1.0); }
                                    THandle::Move => {
                                        let delta_screen = response.drag_delta();
                                        let zoom = self.zoom;
                                        let dx = delta_screen.x / zoom;
                                        let dy = delta_screen.y / zoom;
                                        let w = max_ix - min_ix; let h = max_iy - min_iy;
                                        let ns = (min_ix + dx, min_iy + dy);
                                        s = ns; e = (ns.0 + w, ns.1 + h);
                                        self.crop_drag_orig = Some((ns.0, ns.1, ns.0 + w, ns.1 + h));
                                    }
                                    _ => {}
                                }
                                self.crop_state.start = Some(s);
                                self.crop_state.end   = Some(e);
                            }
                        }
                    } else if !response.drag_started_by(egui::PointerButton::Primary) {
                        if let Some((ix, iy)) = self.screen_to_image(pos) {
                            if self.crop_state.start.is_none() { self.crop_state.start = Some((ix as f32, iy as f32)); }
                            self.crop_state.end = Some((ix as f32, iy as f32));
                        }
                    }
                }
                Tool::Text | Tool::Pan => {
                    let drag_data: Option<(THandle, egui::Pos2, f32, f32, f32, Option<f32>, Option<f32>, f32, f32)> =
                        self.text_drag.as_ref().map(|d| (d.handle, d.start, d.orig_img_x, d.orig_img_y, d.orig_font_size, d.orig_box_width, d.orig_box_height, d.orig_rotation, d.orig_rot_start_angle));

                    if let (Some(id), Some((handle, drag_start, orig_ix, orig_iy, orig_fs, orig_bw, orig_bh, orig_rot, orig_rot_start))) = (self.selected_text, drag_data) {
                        let zoom: f32 = self.zoom;
                        let anchor_screen: egui::Pos2 = self.image_to_screen(orig_ix, orig_iy);
                        let canvas: egui::Rect = self.canvas_rect.unwrap_or(egui::Rect::NOTHING);
                        let (img_w, img_h) = self.image.as_ref().map(|i| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
                        let ox: f32 = canvas.center().x - img_w * zoom / 2.0 + self.pan.x;
                        let oy: f32 = canvas.center().y - img_h * zoom / 2.0 + self.pan.y;

                        let orig_w_screen: f32 = orig_bw.map(|bw| bw * zoom).unwrap_or_else(|| {
                            self.text_layers.iter().find(|l| l.id == id).map(|l| l.max_line_chars()).unwrap_or(1) as f32 * orig_fs * 0.58 * zoom
                        });
                        let orig_h_screen: f32 = orig_bh.map(|bh| bh * zoom).unwrap_or_else(|| {
                            self.text_layers.iter().find(|l| l.id == id).map(|l| l.line_count()).unwrap_or(1) as f32 * orig_fs * 1.35 * zoom
                        });

                        let rot_center: egui::Pos2 = anchor_screen + egui::vec2(orig_w_screen / 2.0, orig_h_screen / 2.0);
                        if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                            let min_sz: f32 = orig_fs * 0.5 * zoom;
                            match handle {
                                THandle::Move => { let delta: egui::Vec2 = pos - drag_start; layer.img_x = orig_ix + delta.x / zoom; layer.img_y = orig_iy + delta.y / zoom; }
                                THandle::E => { layer.box_width  = Some(((pos.x - anchor_screen.x).max(min_sz) / zoom).max(1.0)); }
                                THandle::W => { let orig_right: f32 = anchor_screen.x + orig_w_screen; let new_w: f32 = (orig_right - pos.x).max(min_sz); layer.box_width = Some((new_w / zoom).max(1.0)); layer.img_x = (pos.x - ox) / zoom; }
                                THandle::S => { layer.box_height = Some(((pos.y - anchor_screen.y).max(min_sz) / zoom).max(1.0)); }
                                THandle::N => { let orig_bottom: f32 = anchor_screen.y + orig_h_screen; let new_h: f32 = (orig_bottom - pos.y).max(min_sz); layer.box_height = Some((new_h / zoom).max(1.0)); layer.img_y = ((orig_bottom - new_h) - oy) / zoom; }
                                THandle::SE => { layer.box_width  = Some(((pos.x - anchor_screen.x).max(min_sz) / zoom).max(1.0)); layer.box_height = Some(((pos.y - anchor_screen.y).max(min_sz) / zoom).max(1.0)); }
                                THandle::NE => { let orig_bottom: f32 = anchor_screen.y + orig_h_screen; let new_h: f32 = (orig_bottom - pos.y).max(min_sz); layer.box_width = Some(((pos.x - anchor_screen.x).max(min_sz) / zoom).max(1.0)); layer.box_height = Some((new_h / zoom).max(1.0)); layer.img_y = ((orig_bottom - new_h) - oy) / zoom; }
                                THandle::NW => { let orig_right: f32 = anchor_screen.x + orig_w_screen; let orig_bottom: f32 = anchor_screen.y + orig_h_screen; let new_w: f32 = (orig_right - pos.x).max(min_sz); let new_h: f32 = (orig_bottom - pos.y).max(min_sz); layer.box_width = Some((new_w / zoom).max(1.0)); layer.box_height = Some((new_h / zoom).max(1.0)); layer.img_x = (pos.x - ox) / zoom; layer.img_y = ((orig_bottom - new_h) - oy) / zoom; }
                                THandle::SW => { let orig_right: f32 = anchor_screen.x + orig_w_screen; let new_w: f32 = (orig_right - pos.x).max(min_sz); layer.box_width = Some((new_w / zoom).max(1.0)); layer.box_height = Some(((pos.y - anchor_screen.y).max(min_sz) / zoom).max(1.0)); layer.img_x = (pos.x - ox) / zoom; }
                                THandle::Rotate => { let cur_angle: f32 = (pos - rot_center).angle(); layer.rotation = orig_rot + (cur_angle - orig_rot_start).to_degrees(); }
                            }
                        }
                    }
                }
                _ => {}
            }
            }
        }

        if response.drag_stopped_by(egui::PointerButton::Primary) {
            match self.tool {
                Tool::Brush | Tool::Eraser | Tool::Retouch => { self.composite_dirty = true; self.stroke_points.clear(); self.is_dragging = false; self.stroke_backdrop = None; }
                Tool::Text | Tool::Pan => { if self.text_drag.is_some() { self.composite_dirty = true; } self.text_drag = None; }
                Tool::Crop => { self.crop_drag = None; self.crop_drag_orig = None; }
                _ => {}
            }
            if self.image_drag.is_some() { self.image_drag = None; self.composite_dirty = true; self.dirty = true; }
        }

        if response.clicked_by(egui::PointerButton::Primary) {
            let pos: egui::Pos2 = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            let (img_w, img_h) = self.image.as_ref().map(|i| (i.width() as f32, i.height() as f32)).unwrap_or((1.0, 1.0));
            let ox = canvas_rect.center().x - img_w * self.zoom / 2.0 + self.pan.x;
            let oy = canvas_rect.center().y - img_h * self.zoom / 2.0 + self.pan.y;
            let canvas_pos = ((pos.x - ox) / self.zoom, (pos.y - oy) / self.zoom);

            let hit_image_iid = self.layers.iter().rev()
                .filter(|l| l.kind == LayerKind::Image && l.visible)
                .find_map(|l| {
                    let iid = l.linked_image_id?;
                    let ild = self.image_layer_data.get(&iid)?;
                    if ild.hit_test(pos, img_w, img_h, canvas_rect, self.zoom, self.pan) { Some((l.id, iid)) } else { None }
                });

            if let Some((lid, iid)) = hit_image_iid {
                if self.tool != Tool::Text && self.selected_image_layer != Some(iid) {
                    self.selected_image_layer = Some(iid);
                    self.active_layer_id = lid;
                    self.composite_dirty = true;
                }
            }

            match self.tool {
                Tool::Brush | Tool::Eraser => {
                    if self.image_layer_for_active().is_some() {
                        self.push_undo();
                        self.stroke_points.clear();
                        self.stroke_points.push(canvas_pos);
                        self.stroke_points.push((canvas_pos.0 + 0.1, canvas_pos.1 + 0.1));
                        self.apply_brush_stroke();
                        self.stroke_points.clear();
                        self.composite_dirty = true;
                        if self.tool == Tool::Brush { self.add_color_to_history(); }
                    } else if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.push_undo();
                        let aid = self.active_layer_id;
                        let needs_backdrop = self.tool == Tool::Brush && self.brush.wetness > 0.0
                            && self.layers.iter().find(|l| l.id == aid).map_or(false, |l| l.kind == LayerKind::Raster);
                        self.stroke_backdrop = if needs_backdrop { self.backdrop_cache.lock().unwrap().clone() } else { None };
                        self.stroke_points.clear();
                        self.stroke_points.push((ix as f32, iy as f32));
                        self.stroke_points.push((ix as f32 + 0.1, iy as f32 + 0.1));
                        self.apply_brush_stroke();
                        self.stroke_points.clear();
                        self.stroke_backdrop = None;
                        self.composite_dirty = true;
                        if self.tool == Tool::Brush { self.add_color_to_history(); }
                    }
                }
                Tool::Retouch => {
                    if self.image_layer_for_active().is_some() {
                        self.push_undo();
                        self.init_smudge_sample_image_layer(canvas_pos.0, canvas_pos.1);
                        self.stroke_points.clear();
                        self.stroke_points.push(canvas_pos);
                        self.stroke_points.push((canvas_pos.0 + 0.1, canvas_pos.1 + 0.1));
                        self.apply_retouch_stroke();
                        self.stroke_points.clear();
                        self.composite_dirty = true;
                    } else if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.push_undo();
                        self.stroke_backdrop = None;
                        self.init_smudge_sample(ix, iy);
                        self.stroke_points.clear();
                        self.stroke_points.push((ix as f32, iy as f32));
                        self.stroke_points.push((ix as f32 + 0.1, iy as f32 + 0.1));
                        self.apply_retouch_stroke();
                        self.stroke_points.clear();
                        self.stroke_backdrop = None;
                        self.composite_dirty = true;
                    }
                }
                Tool::Fill => {
                    if self.image_layer_for_active().is_some() {
                        self.push_undo();
                        self.flood_fill_image_layer(canvas_pos.0 as u32, canvas_pos.1 as u32);
                        self.add_color_to_history();
                        self.composite_dirty = true;
                    } else if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.push_undo(); self.flood_fill(ix, iy); self.add_color_to_history();
                    }
                }
                Tool::Eyedropper => {
                    if let Some((ix, iy)) = self.screen_to_image(pos) { self.sample_color(ix, iy); }
                }
                Tool::Text => {
                    if let Some(hit) = self.hit_text_layer(pos) {
                        if self.selected_text != Some(hit) { self.commit_or_discard_active_text(); }
                        self.selected_text = Some(hit); self.editing_text = true; self.text_sel_anchor = None;
                        self.composite_dirty = true;
                        if let Some(layer) = self.text_layers.iter().find(|l| l.id == hit) {
                            self.text_font_size = layer.font_size; self.text_bold = layer.bold;
                            self.text_italic = layer.italic; self.text_underline = layer.underline;
                            self.text_font_name = layer.font_name.clone(); self.text_cursor = layer.content.len();
                        }
                        if let Some(linked_layer) = self.layers.iter().find(|l| l.linked_text_id == Some(hit)) {
                            self.active_layer_id = linked_layer.id;
                        }
                    } else {
                        self.commit_or_discard_active_text();
                        if let Some((ix, iy)) = self.screen_to_image(pos) {
                            let id: u64 = self.next_text_id; self.next_text_id += 1;
                            self.text_layers.push(TextLayer {
                                id, content: String::new(),
                                img_x: ix as f32, img_y: iy as f32,
                                font_size: self.text_font_size, box_width: Some(300.0), box_height: None,
                                rotation: 0.0, color: self.color,
                                bold: self.text_bold, italic: self.text_italic, underline: self.text_underline,
                                font_name: self.text_font_name.clone(), rendered_height: 0.0, cached_lines: Vec::new(),
                            });
                            self.ensure_layer_entry_for_text(id);
                            self.selected_text = Some(id); self.editing_text = true;
                            self.text_cursor = 0; self.text_sel_anchor = None;
                        }
                    }
                }
                Tool::Pan => {
                    if let Some(hit) = self.hit_text_layer(pos) {
                        if self.selected_text != Some(hit) { self.commit_or_discard_active_text(); }
                        self.selected_text = Some(hit);
                        self.selected_image_layer = None;
                        self.composite_dirty = true;
                        if let Some(linked_layer) = self.layers.iter().find(|l| l.linked_text_id == Some(hit)) {
                            self.active_layer_id = linked_layer.id;
                        }
                    } else if hit_image_iid.is_none() {
                        let handles_hit = self.image_layer_transform_handles().and_then(|h| h.hit_test(pos)).is_some();
                        if !handles_hit {
                            self.selected_image_layer = None;
                            self.commit_or_discard_active_text();
                        }
                    }
                }
                _ => {
                    if hit_image_iid.is_none() && self.selected_image_layer.is_some() {
                        let handles_hit = self.image_layer_transform_handles()
                            .and_then(|h| h.hit_test(pos))
                            .is_some();
                        if !handles_hit { self.selected_image_layer = None; }
                    }
                }
            }
        }

        let scroll: f32 = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            let mp = mouse_pos.unwrap_or(canvas_rect.center());
            let over_filter_panel: bool = self.filter_panel != FilterPanel::None
                && self.filter_panel_rect.map_or(false, |r| r.contains(mp));
            let over_color_picker: bool = self.show_color_picker
                && self.color_picker_rect.map_or(false, |r| r.contains(mp));
            if canvas_rect.contains(mp) && !over_filter_panel && !over_color_picker {
                let factor: f32 = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
                self.zoom = (self.zoom * factor).clamp(0.01, 50.0);
            }
        }
        if response.dragged_by(egui::PointerButton::Middle) { self.pan += response.drag_delta(); }
    }

    pub(super) fn render_brush_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, theme: ThemeMode) {
        let (bg, border, text_col, label_col) = if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_900, ColorPalette::BLUE_600, ColorPalette::ZINC_100, ColorPalette::ZINC_400)
        } else {
            (ColorPalette::GRAY_50, ColorPalette::BLUE_600, ColorPalette::GRAY_900, ColorPalette::ZINC_600)
        };
        let accent: egui::Color32 = ColorPalette::BLUE_500;
        let screen_h = ctx.content_rect().height();
        let panel_max_h = (screen_h - 130.0).max(300.0);
        let canvas_origin: egui::Pos2 = ui.available_rect_before_wrap().min;
        let modal_pos: egui::Pos2 = canvas_origin + egui::vec2(10.0, 10.0);

        self.ensure_brush_preview(ctx);
        let win_resp = egui::Window::new("Brush Settings")
            .collapsible(false)
            .resizable(true)
            .fixed_pos(modal_pos)
            .min_size(egui::vec2(420.0, (screen_h * 0.55).min(560.0).max(300.0)))
            .max_size(egui::vec2(460.0, panel_max_h))
            .frame(egui::Frame::new()
                .fill(bg)
                .stroke(egui::Stroke::new(1.5, border))
                .corner_radius(10.0)
                .inner_margin(5.0))
            .show(ctx, |ui: &mut egui::Ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .min_scrolled_height(540.0)
                    .max_height(panel_max_h)
                    .show(ui, |ui: &mut egui::Ui| {
                        ui.set_min_width(420.0);
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        let pad = 16.0;
                        let section_label = |ui: &mut egui::Ui, label: &str| {
                            ui.add_space(4.0);
                            egui::Frame::new()
                                .fill(if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_800 } else { ColorPalette::GRAY_200 })
                                .inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 4, bottom: 4 })
                                .show(ui, |ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new(label).size(10.0).color(label_col).strong());
                                });
                        };

                        if let Some(preview_tid) = self.brush_preview_texture {
                            let avail_w = ui.available_width();
                            let preview_h = 66.0_f32;
                            let frame_fill = if matches!(theme, ThemeMode::Dark) {
                                egui::Color32::from_rgb(14, 14, 19)
                            } else {
                                egui::Color32::from_rgb(230, 230, 236)
                            };
                            egui::Frame::new()
                                .fill(frame_fill)
                                .stroke(egui::Stroke::new(1.0, if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 }))
                                .corner_radius(6.0)
                                .inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 4, bottom: 4 })
                                .show(ui, |ui: &mut egui::Ui| {
                                    let (preview_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(avail_w - pad * 2.0, preview_h),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().image(
                                        preview_tid,
                                        preview_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::WHITE,
                                    );
                                    let note = format!(
                                        "{}  ·  {:.0}px  ·  {:.0}% opacity  ·  {:.0}% softness",
                                        self.brush.shape.label(),
                                        self.brush.size,
                                        self.brush.opacity * 100.0,
                                        self.brush.softness * 100.0,
                                    );
                                    ui.painter().text(
                                        preview_rect.right_bottom() - egui::vec2(4.0, 4.0),
                                        egui::Align2::RIGHT_BOTTOM,
                                        &note,
                                        egui::FontId::proportional(9.5),
                                        egui::Color32::from_rgba_unmultiplied(180, 180, 180, 200),
                                    );
                                });
                        }

                        section_label(ui, "SHAPE");
                        egui::Frame::new()
                            .inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 6, bottom: 6 })
                            .show(ui, |ui: &mut egui::Ui| {
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    for shape in BrushShape::all() {
                                        let is_active = self.brush.shape == *shape;
                                        let (frame_fill, lbl_col) = if is_active {
                                            (ColorPalette::BLUE_600, egui::Color32::WHITE)
                                        } else if matches!(theme, ThemeMode::Dark) {
                                            (ColorPalette::ZINC_800, ColorPalette::ZINC_300)
                                        } else {
                                            (ColorPalette::GRAY_200, ColorPalette::GRAY_700)
                                        };
                                        let border_col = if is_active { ColorPalette::BLUE_400 } else { if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 } };
                                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(68.0, 46.0), egui::Sense::click());
                                        if ui.is_rect_visible(rect) {
                                            let painter = ui.painter_at(rect);
                                            painter.rect_filled(rect, 6.0, frame_fill);
                                            painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, border_col), egui::StrokeKind::Inside);
                                            let icon_area = egui::Rect::from_min_size(rect.min + egui::vec2(4.0, 4.0), egui::vec2(rect.width() - 8.0, rect.height() - 18.0));
                                            let ic = icon_area.center();
                                            let ir = icon_area.height() * 0.42;
                                            let ic_col = egui::Color32::from_rgba_unmultiplied(lbl_col.r(), lbl_col.g(), lbl_col.b(), 220);
                                            match shape {
                                                BrushShape::Circle => {
                                                    painter.circle_filled(ic, ir, ic_col);
                                                }
                                                BrushShape::Square => {
                                                    painter.rect_filled(egui::Rect::from_center_size(ic, egui::vec2(ir * 2.0, ir * 2.0)), 1.0, ic_col);
                                                }
                                                BrushShape::Diamond => {
                                                    let pts = vec![
                                                        egui::pos2(ic.x, ic.y - ir),
                                                        egui::pos2(ic.x + ir, ic.y),
                                                        egui::pos2(ic.x, ic.y + ir),
                                                        egui::pos2(ic.x - ir, ic.y),
                                                    ];
                                                    painter.add(egui::Shape::convex_polygon(pts, ic_col, egui::Stroke::NONE));
                                                }
                                                BrushShape::CalligraphyFlat => {
                                                    let a_rad = 45_f32.to_radians();
                                                    let (cos_a, sin_a) = (a_rad.cos(), a_rad.sin());
                                                    let pts: Vec<egui::Pos2> = (0..20).map(|k| {
                                                        let t = k as f32 / 20.0 * std::f32::consts::TAU;
                                                        let lx_p = t.cos() * ir;
                                                        let ly_p = t.sin() * ir * 0.22;
                                                        egui::pos2(
                                                            ic.x + lx_p * cos_a - ly_p * sin_a,
                                                            ic.y + lx_p * sin_a + ly_p * cos_a,
                                                        )
                                                    }).collect();
                                                    painter.add(egui::Shape::convex_polygon(pts, ic_col, egui::Stroke::NONE));
                                                }
                                            }
                                            painter.text(
                                                egui::pos2(rect.center().x, rect.max.y - 9.0),
                                                egui::Align2::CENTER_CENTER,
                                                shape.label(),
                                                egui::FontId::proportional(9.5),
                                                lbl_col,
                                            );
                                        }
                                        if resp.clicked() {
                                            self.brush.shape = *shape;
                                            self.brush_preview_cache_key = None;
                                        }
                                    }
                                });
                            });

                        section_label(ui, "CORE PARAMETERS");
                        egui::Frame::new()
                            .inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 8, bottom: 8 })
                            .show(ui, |ui: &mut egui::Ui| {
                                ui.spacing_mut().slider_width = 230.0;

                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Size").size(12.0).color(label_col)).on_hover_text("Brush diameter in pixels.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.add(egui::DragValue::new(&mut self.brush.size).range(1.0..=200.0).speed(0.5).suffix("px"));
                                        if ui.add(egui::Slider::new(&mut self.brush.size, 1.0..=200.0).show_value(false)).changed() {
                                            self.brush_preview_cache_key = None;
                                        }
                                    });
                                });
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Opacity").size(12.0).color(label_col)).on_hover_text("Maximum alpha of the overall stroke.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("{:.0}%", self.brush.opacity * 100.0)).size(11.0).color(text_col));
                                        if ui.add(egui::Slider::new(&mut self.brush.opacity, 0.0..=1.0).show_value(false)).changed() {
                                            self.brush_preview_cache_key = None;
                                        }
                                    });
                                });
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Softness").size(12.0).color(label_col)).on_hover_text("0% = hard pixel-sharp edge.\n100% = fully feathered, airbrushed falloff.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("{:.0}%", self.brush.softness * 100.0)).size(11.0).color(text_col));
                                        if ui.add(egui::Slider::new(&mut self.brush.softness, 0.0..=1.0).show_value(false)).changed() {
                                            self.brush_preview_cache_key = None;
                                        }
                                    });
                                });
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Flow").size(12.0).color(label_col)).on_hover_text("Per-stamp opacity. Low flow builds color gradually;\nhigh flow paints solidly each stamp.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("{:.0}%", self.brush.flow * 100.0)).size(11.0).color(text_col));
                                        if ui.add(egui::Slider::new(&mut self.brush.flow, 0.01..=1.0).show_value(false)).changed() {
                                            self.brush_preview_cache_key = None;
                                        }
                                    });
                                });
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Spacing").size(12.0).color(label_col)).on_hover_text("Distance between consecutive stamp positions,\nas a fraction of brush diameter.\nLow = dense/continuous; high = dotted.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("{:.0}%", self.brush.step * 100.0)).size(11.0).color(text_col));
                                        if ui.add(egui::Slider::new(&mut self.brush.step, 0.02..=3.0).show_value(false)).changed() {
                                            self.brush_preview_cache_key = None;
                                        }
                                    });
                                });
                            });

                        let needs_angle = !matches!(self.brush.shape, BrushShape::Circle);
                        let needs_aspect = matches!(self.brush.shape, BrushShape::CalligraphyFlat);
                        if needs_angle {
                            section_label(ui, "SHAPE CONTROLS");
                            egui::Frame::new()
                                .inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 8, bottom: 8 })
                                .show(ui, |ui: &mut egui::Ui| {
                                    ui.spacing_mut().slider_width = 230.0;
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(egui::RichText::new("Angle").size(12.0).color(label_col)).on_hover_text("Rotation of the stamp shape in degrees.");
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(egui::RichText::new(format!("{:.0}°", self.brush.angle)).size(11.0).color(text_col));
                                            if ui.add(egui::Slider::new(&mut self.brush.angle, -180.0..=180.0).show_value(false)).changed() {
                                                self.brush_preview_cache_key = None;
                                            }
                                        });
                                    });
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(egui::RichText::new("Angle Jitter").size(12.0).color(label_col)).on_hover_text("Max random rotation added per stamp. Creates organic, hand-drawn variation.");
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(egui::RichText::new(format!("±{:.0}°", self.brush.angle_jitter)).size(11.0).color(text_col));
                                            if ui.add(egui::Slider::new(&mut self.brush.angle_jitter, 0.0..=180.0).show_value(false)).changed() {
                                                self.brush_preview_cache_key = None;
                                            }
                                        });
                                    });
                                    if needs_aspect {
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            ui.label(egui::RichText::new("Aspect Ratio").size(12.0).color(label_col)).on_hover_text("Width-to-height ratio of the flat calligraphy nib.\n0.05 = very thin stroke; 1.0 = circular.");
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(egui::RichText::new(format!("{:.2}", self.brush.aspect_ratio)).size(11.0).color(text_col));
                                                if ui.add(egui::Slider::new(&mut self.brush.aspect_ratio, 0.05..=1.0).show_value(false)).changed() {
                                                    self.brush_preview_cache_key = None;
                                                }
                                            });
                                        });
                                    }
                                });
                        }

                        section_label(ui, "EFFECTS");
                        egui::Frame::new()
                            .inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 8, bottom: 8 })
                            .show(ui, |ui: &mut egui::Ui| {
                                ui.spacing_mut().slider_width = 230.0;

                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Scatter").size(12.0).color(label_col)).on_hover_text("Max random offset (in pixels) added to each stamp position.\nCreates a spray or scattered feel.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("{:.1}px", self.brush.scatter)).size(11.0).color(text_col));
                                        if ui.add(egui::Slider::new(&mut self.brush.scatter, 0.0..=200.0).show_value(false)).changed() {
                                            self.brush_preview_cache_key = None;
                                        }
                                    });
                                });

                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Wetness").size(12.0).color(label_col)).on_hover_text("Blends new paint color toward the existing pixel color before compositing.\nSimulates wet watercolor bleeding into the canvas.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("{:.0}%", self.brush.wetness * 100.0)).size(11.0).color(text_col));
                                        ui.add(egui::Slider::new(&mut self.brush.wetness, 0.0..=1.0).show_value(false));
                                    });
                                });

                                ui.add_space(2.0);
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Texture").size(12.0).color(label_col)).on_hover_text("Overlays a hash-based noise pattern that masks alpha,\nsimulating brush texture against paper or canvas.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        for mode in BrushTextureMode::all().iter().rev() {
                                            let is_active = self.brush.texture_mode == *mode;
                                            let (bg_c, txt_c) = if is_active { (accent, egui::Color32::WHITE) } else if matches!(theme, ThemeMode::Dark) { (ColorPalette::ZINC_700, ColorPalette::ZINC_300) } else { (ColorPalette::GRAY_200, ColorPalette::GRAY_700) };
                                            if styled_btn(ui, mode.label(), 11.0, egui::vec2(0.0, 20.0), bg_c, bg_c, txt_c) {
                                                self.brush.texture_mode = *mode;
                                                self.brush_preview_cache_key = None;
                                            }
                                        }
                                    });
                                });
                                if self.brush.texture_mode != BrushTextureMode::None {
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(egui::RichText::new("Texture Strength").size(12.0).color(label_col));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(egui::RichText::new(format!("{:.0}%", self.brush.texture_strength * 100.0)).size(11.0).color(text_col));
                                            if ui.add(egui::Slider::new(&mut self.brush.texture_strength, 0.0..=1.0).show_value(false)).changed() {
                                                self.brush_preview_cache_key = None;
                                            }
                                        });
                                    });
                                }

                                ui.add_space(4.0);
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Spray Mode").size(12.0).color(label_col)).on_hover_text("Replaces solid stamp with randomly-scattered individual dots\nfor an aerosol spray-can effect.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.add(egui::Checkbox::new(&mut self.brush.spray_mode, "")).changed() {
                                            self.brush_preview_cache_key = None;
                                        }
                                    });
                                });
                                if self.brush.spray_mode {
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(egui::RichText::new("Particles").size(12.0).color(label_col)).on_hover_text("Number of dots emitted per cursor position when spray mode is on.");
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.add(egui::DragValue::new(&mut self.brush.spray_particles).range(5..=200).speed(1.0));
                                        });
                                    });
                                }
                            });

                        section_label(ui, "PRESETS");
                        egui::Frame::new()
                            .inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 8, bottom: 8 })
                            .show(ui, |ui: &mut egui::Ui| {
                                let presets = BrushPreset::all();
                                let cols = 5_usize;
                                let spacing = ui.spacing().item_spacing.x;
                                let btn_w = ((ui.available_width() - spacing * (cols as f32 - 1.0)) / cols as f32).floor();
                                let btn_h = 34.0_f32;
                                let (bg_c, txt_c) = if matches!(theme, ThemeMode::Dark) { (ColorPalette::ZINC_800, ColorPalette::ZINC_200) } else { (ColorPalette::GRAY_200, ColorPalette::GRAY_800) };
                                let border_c = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 };
                                let hover_c = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 };

                                for row in presets.chunks(cols) {
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        for preset in row {
                                            ui.scope(|ui: &mut egui::Ui| {
                                                let s = ui.style_mut();
                                                s.visuals.widgets.inactive.bg_fill = bg_c;
                                                s.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, border_c);
                                                s.visuals.widgets.hovered.bg_fill = hover_c;
                                                s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent);
                                                let btn = ui.add(egui::Button::new(
                                                    egui::RichText::new(preset.label()).size(11.5).color(txt_c)
                                                ).min_size(egui::vec2(btn_w, btn_h)))
                                                .on_hover_ui(|ui| {
                                                    let p = preset.settings(self.brush.size);
                                                    ui.label(egui::RichText::new(format!(
                                                        "Shape: {}\nSoftness: {:.0}%\nFlow: {:.0}%\nSpacing: {:.0}%",
                                                        p.shape.label(), p.softness * 100.0, p.flow * 100.0, p.step * 100.0,
                                                    )).size(11.0));
                                                });
                                                if btn.clicked() {
                                                    self.brush = preset.settings(self.brush.size);
                                                    self.brush_preview_cache_key = None;
                                                }
                                            });
                                        }
                                    });
                                }
                            });

                        section_label(ui, "FAVORITES (CTRL+1-9, 0)");
                        egui::Frame::new()
                            .inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 8, bottom: 10 })
                            .show(ui, |ui: &mut egui::Ui| {
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Name:").size(12.0).color(label_col));
                                    ui.add(egui::TextEdit::singleline(&mut self.brush_fav_name)
                                        .desired_width(160.0)
                                        .font(egui::TextStyle::Body)
                                        .hint_text("Name for this brush…")
                                    );
                                    let can_save = !self.brush_fav_name.trim().is_empty();
                                    ui.scope(|ui: &mut egui::Ui| {
                                        let s = ui.style_mut();
                                        let save_bg = if can_save { ColorPalette::BLUE_600 } else if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 };
                                        let save_hover = if can_save { ColorPalette::BLUE_500 } else { save_bg };
                                        s.visuals.widgets.inactive.bg_fill = save_bg;
                                        s.visuals.widgets.inactive.weak_bg_fill = save_bg;
                                        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                        s.visuals.widgets.hovered.bg_fill = save_hover;
                                        s.visuals.widgets.hovered.weak_bg_fill = save_hover;
                                        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                                        s.visuals.widgets.active.bg_fill = save_hover;
                                        s.visuals.widgets.active.weak_bg_fill = save_hover;
                                        s.visuals.override_text_color = Some(if can_save { egui::Color32::WHITE } else if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_500 } else { ColorPalette::GRAY_500 });
                                        if ui.add_enabled(can_save, egui::Button::new(egui::RichText::new("Save").size(12.0)).min_size(egui::vec2(48.0, 24.0))).clicked() {
                                            let name = self.brush_fav_name.trim().to_string();
                                            if let Some(existing) = self.brush_favorites.brushes.iter_mut().find(|b| b.name == name) {
                                                existing.settings = self.brush.clone();
                                            } else {
                                                self.brush_favorites.brushes.push(SavedBrush { name, settings: self.brush.clone() });
                                            }
                                            self.brush_favorites.save();
                                            self.brush_fav_name.clear();
                                        }
                                    });
                                });

                                let mut do_export = false; let mut do_import = false;
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    let (ebg, ehov, etxt) = theme_btn(theme);
                                    if styled_btn(ui, "⬆ Export Favorites", 11.0, egui::vec2(150.0, 24.0), ebg, ehov, etxt) {
                                        do_export = true;
                                    }
                                    if styled_btn(ui, "⬇ Import Favorites", 11.0, egui::vec2(150.0, 24.0), ebg, ehov, etxt) {
                                        do_import = true;
                                    }
                                });

                                if do_export {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .set_file_name("brush_favorites.json")
                                        .add_filter("JSON", &["json"])
                                        .save_file()
                                    {
                                        if let Ok(json) = serde_json::to_string_pretty(&self.brush_favorites.brushes) {
                                            let _ = std::fs::write(path, json);
                                        }
                                    }
                                }
                                if do_import {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("JSON", &["json"])
                                        .pick_file()
                                    {
                                        if let Ok(content) = std::fs::read_to_string(&path) {
                                            if let Ok(imported) = serde_json::from_str::<Vec<SavedBrush>>(&content) {
                                                for b in imported {
                                                    if let Some(existing) = self.brush_favorites.brushes.iter_mut().find(|e| e.name == b.name) {
                                                        existing.settings = b.settings;
                                                    } else {
                                                        self.brush_favorites.brushes.push(b);
                                                    }
                                                }
                                                self.brush_favorites.save();
                                            }
                                        }
                                    }
                                }

                                ui.add_space(6.0);

                                if self.brush_favorites.brushes.is_empty() {
                                    ui.label(egui::RichText::new("No saved brushes yet. Configure a brush above and save it.").size(11.0).color(label_col).italics());
                                } else {
                                    let mut to_load: Option<usize> = None;
                                    let mut to_delete: Option<usize> = None;
                                    const MAX_HOTKEYS: usize = 10;

                                    for (idx, saved) in self.brush_favorites.brushes.iter().enumerate() {
                                        let row_fill = if matches!(theme, ThemeMode::Dark) {
                                            if idx % 2 == 0 { ColorPalette::ZINC_800 } else { ColorPalette::ZINC_900 }
                                        } else {
                                            if idx % 2 == 0 { ColorPalette::GRAY_100 } else { ColorPalette::GRAY_50 }
                                        };
                                        egui::Frame::new()
                                            .fill(row_fill)
                                            .corner_radius(4.0)
                                            .inner_margin(egui::Margin { left: 8, right: 6, top: 4, bottom: 4 })
                                            .show(ui, |ui: &mut egui::Ui| {
                                                ui.horizontal(|ui: &mut egui::Ui| {
                                                    let (pr, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                                                    if ui.is_rect_visible(pr) {
                                                        let painter = ui.painter_at(pr);
                                                        let circ_r = 9.0_f32;
                                                        let softness = saved.settings.softness;
                                                        let steps = 16_u32;
                                                        for si in 0..steps {
                                                            for ri in 0..6_u32 {
                                                                let frac = ri as f32 / 5.0;
                                                                let a_p = si as f32 / steps as f32 * std::f32::consts::TAU;
                                                                let rr = frac * circ_r;
                                                                let dx = a_p.cos() * rr;
                                                                let dy = a_p.sin() * rr;
                                                                let fo = super::ie_tools::brush_shape_falloff(
                                                                    dx, dy, circ_r, saved.settings.aspect_ratio,
                                                                    saved.settings.angle.to_radians(), softness,
                                                                    saved.settings.shape,
                                                                );
                                                                if fo > 0.0 {
                                                                    let alpha_p = (fo * 200.0) as u8;
                                                                    painter.circle_filled(pr.center() + egui::vec2(dx, dy), 0.8, egui::Color32::from_rgba_unmultiplied(130, 130, 220, alpha_p));
                                                                }
                                                            }
                                                        }
                                                        if idx < MAX_HOTKEYS {
                                                            let key_label = if idx == 9 { "0".to_string() } else { format!("{}", idx + 1) };
                                                            let badge = egui::Rect::from_min_size(pr.min, egui::vec2(13.0, 13.0));
                                                            painter.rect_filled(badge, egui::CornerRadius { nw: 4, ne: 0, sw: 0, se: 4 }, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 190));
                                                            painter.text(badge.center(), egui::Align2::CENTER_CENTER, &key_label, egui::FontId::monospace(8.0), egui::Color32::from_rgba_unmultiplied(200, 220, 255, 240));
                                                        }
                                                    }

                                                    ui.label(egui::RichText::new(&saved.name).size(12.0).color(text_col));
                                                    let desc = format!("{} / {:.0}px / S{:.0}%", saved.settings.shape.label(), saved.settings.size, saved.settings.softness * 100.0);
                                                    ui.label(egui::RichText::new(desc).size(10.0).color(label_col));
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                        if styled_btn(ui, "Delete", 11.0, egui::vec2(52.0, 22.0), egui::Color32::from_rgb(180, 60, 60), egui::Color32::from_rgb(200, 80, 80), egui::Color32::WHITE) {
                                                            to_delete = Some(idx);
                                                        }
                                                        if styled_btn(ui, "Load", 11.0, egui::vec2(46.0, 22.0), accent, ColorPalette::BLUE_400, egui::Color32::WHITE) {
                                                            to_load = Some(idx);
                                                        }
                                                    });
                                                });
                                            });
                                        ui.add_space(2.0);
                                    }

                                    if let Some(idx) = to_load {
                                        self.brush = self.brush_favorites.brushes[idx].settings.clone();
                                        self.brush_preview_cache_key = None;
                                    }
                                    if let Some(idx) = to_delete {
                                        self.brush_favorites.brushes.remove(idx);
                                        self.brush_favorites.save();
                                    }
                                }
                            });

                        egui::Frame::new()
                            .inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 6, bottom: 10 })
                            .show(ui, |ui: &mut egui::Ui| {
                                let avail_w = ui.available_width();
                                let (cbg, chover, ctxt) = theme_btn(theme);
                                if styled_btn(ui, "Close Panel", 12.0, egui::vec2(avail_w, 28.0), cbg, chover, ctxt) {
                                    self.filter_panel = FilterPanel::None;
                                }
                            });
                    });
            });
        self.filter_panel_rect = win_resp.map(|r| r.response.rect);
    }

    pub(super) fn render_layers_panel(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        let is_dark = matches!(theme, ThemeMode::Dark);
        let bg_deep = if is_dark { ColorPalette::ZINC_800 } else { egui::Color32::from_rgb(245, 245, 248) };
        let bg_row = if is_dark { egui::Color32::from_rgb(38, 38, 44) } else { egui::Color32::from_rgb(252, 252, 255) };
        let bg_active = if is_dark { egui::Color32::from_rgb(45, 75, 120) } else { egui::Color32::from_rgb(210, 228, 255) };
        let border = if is_dark { egui::Color32::from_rgb(55, 55, 65) } else { egui::Color32::from_rgb(210, 210, 220) };
        let text_prim = if is_dark { egui::Color32::from_rgb(220, 220, 228) } else { egui::Color32::from_rgb(30, 30, 40) };
        let text_mute = if is_dark { egui::Color32::from_rgb(130, 130, 150) } else { egui::Color32::from_rgb(140, 140, 160) };
        let accent = ColorPalette::BLUE_500;
        let danger = egui::Color32::from_rgb(200, 60, 60);

        egui::Frame::new()
            .fill(if is_dark { ColorPalette::ZINC_800 } else { egui::Color32::from_rgb(235, 235, 242) })
            .stroke(egui::Stroke::new(0.0, border))
            .inner_margin(egui::Margin { left: 10, right: 6, top: 8, bottom: 8 })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Layers").size(13.0).strong().color(text_prim));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if layer_icon_btn(ui, "+", "New Raster Layer (Ctrl+Shift+N)", accent, egui::Color32::WHITE, is_dark) {
                            self.new_raster_layer();
                        }
                    });
                });
            });

        ui.separator();

        let available_h = ui.available_height() - 120.0;
        egui::ScrollArea::vertical()
            .id_salt("layers_scroll")
            .max_height(available_h.max(80.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                let n = self.layers.len();
                let mut action: Option<LayerPanelAction> = None;
                let pointer_pos = ui.input(|i| i.pointer.hover_pos());
                let pointer_released = ui.input(|i| i.pointer.any_released());
                let mut drop_indicator: Option<usize> = None;

                for disp_idx in 0..n {
                    let stack_idx = n - 1 - disp_idx;
                    let layer = &self.layers[stack_idx];
                    let is_active = layer.id == self.active_layer_id;
                    let layer_id = layer.id;
                    let layer_kind = layer.kind;
                    let layer_name = layer.name.clone();
                    let layer_visible = layer.visible;
                    let layer_locked  = layer.locked;
                    let is_background = layer_kind == LayerKind::Background;
                    let row_fill = if is_active { bg_active } else { bg_row };

                    if is_background && n > 1 {
                        let sep_color = if is_dark {
                            egui::Color32::from_rgb(80, 80, 100)
                        } else {
                            egui::Color32::from_rgb(180, 180, 200)
                        };
                        let (sep_rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(sep_rect, 0.0, sep_color);
                        ui.add_space(2.0);
                    }

                    let row_resp = egui::Frame::new()
                        .fill(row_fill)
                        .stroke(if is_active { egui::Stroke::new(1.0, accent) } else { egui::Stroke::new(0.0, border) })
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin { left: 6, right: 6, top: 4, bottom: 4 })
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width() - 4.0);
                            ui.horizontal(|ui| {
                                if is_background {
                                    ui.add(egui::Label::new(egui::RichText::new("👁").size(13.0).color(text_mute))
                                        .sense(egui::Sense::hover()))
                                        .on_hover_text("Background layer is always visible");
                                    ui.add(egui::Label::new(egui::RichText::new("🔒").size(12.0).color(text_mute))
                                        .sense(egui::Sense::hover()))
                                        .on_hover_text("Background layer cannot be moved or deleted");
                                } else {
                                    let eye = if layer_visible { "👁" } else { "🚫" };
                                    if ui.add(egui::Button::new(egui::RichText::new(eye).size(13.0)).frame(false).min_size(egui::vec2(18.0, 18.0))).clicked() {
                                        action = Some(LayerPanelAction::ToggleVisible(stack_idx));
                                    }
                                    let lock_icon = if layer_locked { "🔒" } else { "🔓" };
                                    if ui.add(egui::Button::new(egui::RichText::new(lock_icon).size(12.0)).frame(false).min_size(egui::vec2(18.0, 18.0))).clicked() {
                                        action = Some(LayerPanelAction::ToggleLocked(stack_idx));
                                    }
                                }

                                let kind_badge = match layer_kind {
                                    LayerKind::Background => ("BG", egui::Color32::from_rgb(80, 140, 80)),
                                    LayerKind::Raster => ("R",  egui::Color32::from_rgb(80, 100, 180)),
                                    LayerKind::Text => ("T",  egui::Color32::from_rgb(180, 100, 60)),
                                    LayerKind::Image => ("I",  egui::Color32::from_rgb(100, 60, 180)),
                                };
                                egui::Frame::new()
                                    .fill(kind_badge.1.linear_multiply(if is_dark { 0.6 } else { 0.3 }))
                                    .corner_radius(3.0)
                                    .inner_margin(egui::Margin { left: 4, right: 4, top: 1, bottom: 1 })
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new(kind_badge.0).size(9.5).color(if is_dark { egui::Color32::WHITE } else { egui::Color32::from_rgb(20,20,40) }));
                                    });

                                ui.add_space(2.0);

                                if !is_background && self.layer_rename_id == Some(layer_id) {
                                    let rename_resp = ui.add(
                                        egui::TextEdit::singleline(&mut self.layer_rename_buf)
                                            .desired_width(ui.available_width() - 28.0)
                                            .font(egui::TextStyle::Small)
                                    );
                                    rename_resp.request_focus();
                                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                                    if enter || escape || rename_resp.lost_focus() {
                                        action = Some(LayerPanelAction::CommitRename(stack_idx));
                                    }
                                } else {
                                    let name_color = if is_active {
                                        egui::Color32::WHITE
                                    } else if is_background {
                                        text_mute
                                    } else {
                                        text_prim
                                    };
                                    let rich = egui::RichText::new(&layer_name).size(12.0).color(name_color);
                                    let rich = if is_background { rich.italics() } else { rich };
                                    let name_resp = ui.add(
                                        egui::Label::new(rich)
                                            .truncate()
                                            .sense(egui::Sense::click())
                                    );
                                    if name_resp.clicked() {
                                        action = Some(LayerPanelAction::Select(stack_idx));
                                    }
                                    if !is_background && name_resp.double_clicked() {
                                        action = Some(LayerPanelAction::StartRename(stack_idx, layer_name.clone()));
                                    }
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if !is_background && n > 1 {
                                        if ui.add(egui::Button::new(egui::RichText::new("🗑").size(11.0).color(if is_active { egui::Color32::from_rgb(255, 120, 120) } else { text_mute })).frame(false).min_size(egui::vec2(18.0, 18.0))).clicked() {
                                            action = Some(LayerPanelAction::Delete(stack_idx));
                                        }
                                    }
                                });
                            });
                        });

                    if row_resp.response.clicked() && action.is_none() {
                        action = Some(LayerPanelAction::Select(stack_idx));
                    }
                    if !is_background && row_resp.response.drag_started() {
                        self.layer_drag_src = Some(stack_idx);
                    }
                    if let (Some(drag_src), Some(pp)) = (self.layer_drag_src, pointer_pos) {
                        if row_resp.response.rect.contains(pp) && drag_src != stack_idx {
                            if stack_idx > 0 {
                                drop_indicator = Some(stack_idx);
                            }
                        }
                    }

                    if is_background {
                        row_resp.response.context_menu(|ui| {
                            ui.add_enabled(false, egui::Button::new(
                                egui::RichText::new("Background layer").size(11.0).color(text_mute)
                            ));
                            ui.separator();
                            if ui.button("Duplicate as Raster Layer").clicked() {
                                action = Some(LayerPanelAction::Duplicate(stack_idx));
                                ui.close();
                            }
                            if ui.button("Flatten Image").clicked() {
                                action = Some(LayerPanelAction::Flatten);
                                ui.close();
                            }
                        });
                    } else {
                        row_resp.response.context_menu(|ui| {
                            if ui.button("Rename").clicked() {
                                action = Some(LayerPanelAction::StartRename(stack_idx, layer_name.clone()));
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Duplicate").clicked() {
                                action = Some(LayerPanelAction::Duplicate(stack_idx));
                                ui.close();
                            }
                            if n > 1 {
                                let can_ctx_merge = stack_idx > 0
                                    && !matches!(self.layers.get(stack_idx.saturating_sub(1)).map(|l| l.kind), Some(LayerKind::Text) | Some(LayerKind::Image))
                                    && !matches!(layer_kind, LayerKind::Image);
                                if ui.add_enabled(can_ctx_merge, egui::Button::new("Merge Down")).clicked() {
                                    action = Some(LayerPanelAction::MergeDown(stack_idx));
                                    ui.close();
                                }
                            }
                            ui.separator();
                            if ui.add_enabled(n > 1, egui::Button::new(egui::RichText::new("Delete").color(danger))).clicked() {
                                action = Some(LayerPanelAction::Delete(stack_idx));
                                ui.close();
                            }
                        });
                    }

                    ui.add_space(1.0);
                }

                if let Some(di) = drop_indicator {
                    let _ = di;
                }

                if pointer_released {
                    if let (Some(src), Some(dst)) = (self.layer_drag_src, drop_indicator) {
                        if src != dst && src > 0 && dst > 0 {
                            action = Some(LayerPanelAction::Reorder(src, dst));
                        }
                    }
                    self.layer_drag_src = None;
                }

                if let Some(act) = action {
                    match act {
                        LayerPanelAction::Select(idx) => {
                            let prev_editing = self.editing_text;
                            if prev_editing { self.commit_or_discard_active_text(); }
                            self.active_layer_id = self.layers[idx].id;
                            let aid = self.active_layer_id;
                            self.kick_backdrop_compute(aid);
                            match self.layers[idx].kind {
                                LayerKind::Text => {
                                    if let Some(tid) = self.layers[idx].linked_text_id {
                                        self.tool = Tool::Text;
                                        self.selected_text = Some(tid);
                                        self.editing_text = true;
                                        self.text_cursor = self.text_layers.iter()
                                            .find(|t| t.id == tid)
                                            .map(|t| t.content.len())
                                            .unwrap_or(0);
                                        self.text_sel_anchor = None;
                                        self.composite_dirty = true;
                                    }
                                    self.selected_image_layer = None;
                                }
                                LayerKind::Image => {
                                    self.selected_image_layer = self.layers[idx].linked_image_id;
                                    self.selected_text = None;
                                    self.editing_text = false;
                                    self.composite_dirty = true;
                                }
                                _ => {
                                    self.selected_text = None;
                                    self.editing_text = false;
                                    self.composite_dirty = true;
                                    self.selected_image_layer = None;
                                }
                            }
                        }
                        LayerPanelAction::ToggleVisible(idx) => {
                            self.layers[idx].visible = !self.layers[idx].visible;
                            self.composite_dirty = true;
                            self.dirty = true;
                            self.backdrop_cache_for = u64::MAX;
                        }
                        LayerPanelAction::ToggleLocked(idx) => {
                            self.layers[idx].locked = !self.layers[idx].locked;
                        }
                        LayerPanelAction::Delete(idx) => {
                            if self.layers[idx].kind == LayerKind::Background { return; }
                            let id = self.layers[idx].id;
                            if self.active_layer_id == id {
                                self.active_layer_id = self.layers[if idx > 0 { idx - 1 } else { 1.min(self.layers.len()-1) }].id;
                            }
                            self.push_undo();
                            if let Some(tid) = self.layers[idx].linked_text_id {
                                self.text_layers.retain(|t| t.id != tid);
                            }
                            self.layer_images.remove(&self.layers[idx].id);
                            self.layers.remove(idx);
                            self.composite_dirty = true;
                            self.dirty = true;
                        }
                        LayerPanelAction::StartRename(idx, name) => {
                            self.layer_rename_id  = Some(self.layers[idx].id);
                            self.layer_rename_buf = name;
                        }
                        LayerPanelAction::CommitRename(_idx) => {
                            if let Some(id) = self.layer_rename_id {
                                if let Some(l) = self.layers.iter_mut().find(|l| l.id == id) {
                                    let new_name = self.layer_rename_buf.trim().to_string();
                                    if !new_name.is_empty() { l.name = new_name; }
                                }
                            }
                            self.layer_rename_id = None;
                        }
                        LayerPanelAction::Duplicate(idx) => {
                            self.active_layer_id = self.layers[idx].id;
                            self.duplicate_active_layer();
                        }
                        LayerPanelAction::MergeDown(idx) => {
                            self.active_layer_id = self.layers[idx].id;
                            self.merge_down();
                        }
                        LayerPanelAction::Flatten => {
                            self.flatten_all_layers();
                        }
                        LayerPanelAction::Reorder(src, dst) => {
                            self.push_undo();
                            self.layers.swap(src, dst);
                            self.composite_dirty = true;
                            self.dirty = true;
                            self.backdrop_cache_for = u64::MAX;
                        }
                    }
                }
            });

        ui.separator();

        let active_idx = self.layers.iter().position(|l| l.id == self.active_layer_id);
        if let Some(idx) = active_idx {
            let panel_w = ui.available_width();
            egui::Frame::new()
                .inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 6 })
                .fill(bg_deep)
                .show(ui, |ui| {
                    let is_bg_layer = self.layers[idx].kind == LayerKind::Background;
                    ui.scope(|ui| {
                        if is_bg_layer { ui.disable(); }

                        ui.label(egui::RichText::new("Opacity").size(11.0).color(text_mute));
                        let old_opacity = self.layers[idx].opacity;
                        let mut opacity_pct = if is_bg_layer { 100 } else { (old_opacity * 100.0).round() as i32 };
                        ui.horizontal(|ui| {
                            ui.spacing_mut().slider_width = panel_w - 80.0;
                            let resp = ui.add(egui::Slider::new(&mut opacity_pct, 0..=100).show_value(false));
                            if resp.changed() {
                                self.layers[idx].opacity = opacity_pct as f32 / 100.0;
                                self.dirty = true;
                            }
                            if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                                self.composite_dirty = true;
                            }
                            ui.label(egui::RichText::new(format!("{}%", opacity_pct)).size(11.0).color(text_prim));
                        });

                        ui.add_space(4.0);

                        ui.label(egui::RichText::new("Blend Mode").size(11.0).color(text_mute));
                        let current_blend = self.layers[idx].blend_mode;
                        egui::ComboBox::from_id_salt("layer_blend_mode")
                            .selected_text(egui::RichText::new(
                                if is_bg_layer { "Normal" } else { current_blend.label() }
                            ).size(12.0))
                            .width(panel_w - 18.0)
                            .show_ui(ui, |ui| {
                                for &mode in BlendMode::all() {
                                    if ui.selectable_label(current_blend == mode, mode.label()).clicked() {
                                        self.layers[idx].blend_mode = mode;
                                        self.composite_dirty = true;
                                        self.dirty = true;
                                    }
                                }
                            });
                    });

                    ui.add_space(6.0);

                    ui.horizontal_wrapped(|ui| {
                        let is_bg = self.layers[idx].kind == LayerKind::Background;
                        let can_up = !is_bg && idx < self.layers.len() - 1;
                        let can_down = !is_bg && idx > 1;
                        let can_merge = !is_bg && idx > 0
                            && !matches!(self.layers[idx - 1].kind, LayerKind::Text | LayerKind::Image)
                            && !matches!(self.layers[idx].kind, LayerKind::Image);

                        if ui.add_enabled(can_up, egui::Button::new(egui::RichText::new("⬆").size(11.0)).min_size(egui::vec2(28.0, 24.0))).on_hover_text("Move layer up").clicked() {
                            self.push_undo();
                            self.move_layer_up();
                        }
                        if ui.add_enabled(can_down, egui::Button::new(egui::RichText::new("⬇").size(11.0)).min_size(egui::vec2(28.0, 24.0))).on_hover_text("Move layer down").clicked() {
                            self.push_undo();
                            self.move_layer_down();
                        }
                        ui.add_space(4.0);
                        if ui.add(egui::Button::new(egui::RichText::new("Dup").size(11.0)).min_size(egui::vec2(46.0, 24.0))).on_hover_text("Duplicate layer").clicked() {
                            self.duplicate_active_layer();
                        }
                        if ui.add_enabled(can_merge, egui::Button::new(egui::RichText::new("⬇ Merge").size(11.0)).min_size(egui::vec2(60.0, 24.0))).on_hover_text("Merge down").clicked() {
                            self.merge_down();
                        }
                        if ui.add_enabled(self.layers.len() > 1, egui::Button::new(egui::RichText::new("Flatten All Layers").size(11.0)).min_size(egui::vec2(46.0, 24.0))).on_hover_text("Flatten all layers").clicked() {
                            self.flatten_all_layers();
                        }
                    });
                });
        }
    }

    pub(super) fn ensure_brush_preview(&mut self, ctx: &egui::Context) {
        let is_dark = ctx.style().visuals.dark_mode;
        let key = (self.brush.clone(), self.color, is_dark);
        if let Some(ref cached) = self.brush_preview_cache_key {
            if *cached == key && self.brush_preview_texture.is_some() { return; }
        }
        let pw = 300u32;
        let ph = 66u32;
        let pixels = self.render_brush_preview_to_pixels(pw, ph, is_dark);
        let ci = egui::ColorImage {
            size: [pw as usize, ph as usize],
            source_size: egui::vec2(pw as f32, ph as f32),
            pixels,
        };
        let opts = egui::TextureOptions {
            magnification: egui::TextureFilter::Linear,
            minification:  egui::TextureFilter::Linear,
            ..Default::default()
        };
        if let Some(tid) = self.brush_preview_texture {
            ctx.tex_manager().write().set(tid, egui::epaint::ImageDelta::full(ci, opts));
        } else {
            self.brush_preview_texture = Some(
                ctx.tex_manager().write().alloc("brush_preview".into(), ci.into(), opts),
            );
        }
        self.brush_preview_cache_key = Some(key);
    }
}

enum LayerPanelAction {
    Select(usize),
    ToggleVisible(usize),
    ToggleLocked(usize),
    Delete(usize),
    StartRename(usize, String),
    CommitRename(usize),
    Duplicate(usize),
    MergeDown(usize),
    Reorder(usize, usize),
    Flatten,
}

fn layer_icon_btn(ui: &mut egui::Ui, text: &str, tooltip: &str, bg: egui::Color32, fg: egui::Color32, _dark: bool) -> bool {
    ui.scope(|ui| {
        let s = ui.style_mut();
        s.visuals.widgets.inactive.bg_fill = bg;
        s.visuals.widgets.inactive.weak_bg_fill = bg;
        s.visuals.widgets.hovered.bg_fill = bg.linear_multiply(1.2);
        s.visuals.widgets.hovered.weak_bg_fill = bg.linear_multiply(1.2);
        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        ui.add(egui::Button::new(egui::RichText::new(text).size(14.0).strong().color(fg)).min_size(egui::vec2(24.0, 24.0)))
            .on_hover_text(tooltip)
    }).inner.clicked()
}

fn styled_btn(ui: &mut egui::Ui, text: &str, font_size: f32, min_size: egui::Vec2, bg: egui::Color32, hover: egui::Color32, txt: egui::Color32) -> bool {
    ui.scope(|ui| {
        let s = ui.style_mut();
        s.visuals.widgets.inactive.bg_fill = bg;
        s.visuals.widgets.inactive.weak_bg_fill = bg;
        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        s.visuals.widgets.hovered.bg_fill = hover;
        s.visuals.widgets.hovered.weak_bg_fill = hover;
        s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        s.visuals.widgets.active.bg_fill = hover;
        s.visuals.widgets.active.weak_bg_fill = hover;
        ui.add(egui::Button::new(egui::RichText::new(text).size(font_size).color(txt)).min_size(min_size))
    }).inner.clicked()
}

fn theme_btn(theme: ThemeMode) -> (egui::Color32, egui::Color32, egui::Color32) {
    if matches!(theme, ThemeMode::Dark) {
        (ColorPalette::ZINC_700, ColorPalette::ZINC_600, ColorPalette::ZINC_200)
    } else {
        (ColorPalette::GRAY_200, ColorPalette::GRAY_300, ColorPalette::GRAY_800)
    }
}

enum FilterAction { None, Preview, Apply, Cancel }

fn filter_action_row(ui: &mut egui::Ui, theme: ThemeMode, preview_active: bool) -> FilterAction {
    let mut action = FilterAction::None;
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if toolbar_toggle_btn(ui, egui::RichText::new("Preview").size(12.0), preview_active, theme).clicked() { action = FilterAction::Preview; }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if toolbar_action_btn(ui, egui::RichText::new("Apply").size(12.0), theme).clicked() { action = FilterAction::Apply; }
            if toolbar_action_btn(ui, egui::RichText::new("Cancel").size(12.0), theme).clicked() { action = FilterAction::Cancel; }
        });
    });
    action
}

fn gradient_slider_ui(ui: &mut egui::Ui, value: &mut f32, min: f32, max: f32, left_col: egui::Color32, right_col: egui::Color32, left_label: &str,
    right_label: &str, fmt: impl Fn(f32) -> String, drag_input: bool, drag_display_scale: f32, drag_suffix: &str) -> bool
{
    let mut changed: bool = false;
    let range: f32 = (max - min).max(1e-6_f32);
    let t_norm: f32 = ((*value - min) / range).clamp(0.0, 1.0);
    let val_str: String = fmt(*value);
    let slider_width: f32 = ui.spacing().slider_width;
    let track_h: f32 = 12.0;
    let label_h: f32 = 14.0;
    let handle_r: f32 = 9.0;
    let total_h: f32 = handle_r * 2.0 + label_h + 2.0;

    let inner = ui.horizontal(|ui: &mut egui::Ui| {
        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(slider_width, total_h),
            egui::Sense::click_and_drag(),
        );

        if ui.is_rect_visible(rect) {
            let painter: &egui::Painter = ui.painter();
            let track_top: f32 = rect.min.y + handle_r - track_h / 2.0;
            let track_rect: egui::Rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x, track_top),
                egui::vec2(rect.width(), track_h),
            );

            const STEPS: u32 = 32;
            for i in 0..STEPS {
                let t0: f32 = i as f32 / STEPS as f32;
                let t1: f32 = (i + 1) as f32 / STEPS as f32;
                let tm: f32 = (t0 + t1) * 0.5;
                let seg_col = egui::Color32::from_rgb(
                    (left_col.r() as f32 + (right_col.r() as f32 - left_col.r() as f32) * tm).round() as u8,
                    (left_col.g() as f32 + (right_col.g() as f32 - left_col.g() as f32) * tm).round() as u8,
                    (left_col.b() as f32 + (right_col.b() as f32 - left_col.b() as f32) * tm).round() as u8,
                );
                let x0 = track_rect.left() + t0 * track_rect.width();
                let x1 = track_rect.left() + t1 * track_rect.width();
                painter.rect_filled(
                    egui::Rect::from_min_max(egui::pos2(x0, track_rect.top()), egui::pos2(x1, track_rect.bottom())),
                    0.0, seg_col,
                );
            }
            painter.rect_stroke(
                track_rect, 0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 90)),
                egui::StrokeKind::Outside,
            );

            let label_y: f32 = track_rect.bottom() + 2.0;
            let label_font: egui::FontId = egui::FontId::proportional(9.5);
            let label_col_dim: egui::Color32 = egui::Color32::from_rgba_unmultiplied(180, 180, 180, 200);
            painter.text(egui::pos2(track_rect.left(), label_y), egui::Align2::LEFT_TOP, left_label, label_font.clone(), label_col_dim);
            painter.text(egui::pos2(track_rect.right(), label_y), egui::Align2::RIGHT_TOP, right_label, label_font, label_col_dim);

            let handle_x: f32 = (rect.min.x + t_norm * rect.width()).clamp(rect.min.x, rect.max.x);
            let handle_center: egui::Pos2 = egui::pos2(handle_x, track_rect.center().y);
            painter.circle_filled(handle_center, handle_r + 1.5, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 60));
            painter.circle_filled(handle_center, handle_r, egui::Color32::WHITE);
            painter.circle_stroke(handle_center, handle_r, egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 90, 90)));
        }

        ui.add_space(6.0);
        ui.vertical(|ui: &mut egui::Ui| {
            if drag_input {
                let mut display_val: i32 = (*value * drag_display_scale).round() as i32;
                let display_min: i32 = (min * drag_display_scale).round() as i32;
                let display_max: i32 = (max * drag_display_scale).round() as i32;
                let dv: egui::DragValue<'_> = egui::DragValue::new(&mut display_val)
                    .range(display_min..=display_max)
                    .speed(1)
                    .suffix(drag_suffix)
                    .min_decimals(0)
                    .max_decimals(0);
                if ui.add(dv).changed() {
                    *value = display_val as f32 / drag_display_scale;
                    changed = true;
                }
            } else {
                ui.label(egui::RichText::new(&val_str).size(11.0).strong().color(egui::Color32::from_rgba_unmultiplied(210, 210, 210, 240)));
            }
        });
        resp
    });

    let resp: egui::Response = inner.inner;
    if resp.dragged() || resp.clicked() {
        if let Some(ptr) = resp.interact_pointer_pos() {
            let new_t   = ((ptr.x - resp.rect.min.x) / resp.rect.width()).clamp(0.0, 1.0);
            let new_val = min + new_t * range;
            if (*value - new_val).abs() > range * 0.0005 {
                *value = new_val;
                changed = true;
            }
        }
    }
    changed
}
