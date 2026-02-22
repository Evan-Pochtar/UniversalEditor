use eframe::egui;
use crate::style::{ColorPalette, ThemeMode};
use crate::modules::helpers::image_export::ExportFormat;
use super::ie_main::{ImageEditor, Tool, FilterPanel, TransformHandleSet, THandle, RgbaColor, CropState, TextDrag, HANDLE_HIT, BrushShape, BrushTextureMode, BrushPreset, SavedBrush};
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
                            self.tool_btn(ui, "Pan", Tool::Pan, Some("P"), theme);
                        });
                    });
            });
    }

    fn tool_btn(&mut self, ui: &mut egui::Ui, label: &str, tool: Tool, shortcut: Option<&str>, theme: ThemeMode) {
        let active: bool = self.tool == tool;
        let (bg, hover, txt) = if active {
            (ColorPalette::BLUE_600, ColorPalette::BLUE_500, egui::Color32::WHITE)
        } else if matches!(theme, ThemeMode::Dark) {
            (ColorPalette::ZINC_700, ColorPalette::ZINC_600, ColorPalette::ZINC_200)
        } else {
            (ColorPalette::GRAY_200, ColorPalette::GRAY_300, ColorPalette::GRAY_800)
        };

        let response: egui::Response = ui.scope(|ui: &mut egui::Ui| {
            let s: &mut egui::Style = ui.style_mut();
            s.visuals.widgets.inactive.bg_fill = bg;
            s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            s.visuals.widgets.hovered.bg_fill = hover;
            s.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            s.visuals.widgets.active.bg_fill = hover;
            let btn: egui::Response = ui.add(egui::Button::new(egui::RichText::new(label).size(12.0).color(txt)).min_size(egui::vec2(0.0, 24.0)));
            if let Some(sc) = shortcut { btn.on_hover_text(sc) } else { btn }
        }).inner;

        if response.clicked() {
            if tool != Tool::Text { self.commit_or_discard_active_text(); }
            self.tool = tool;
        }
    }

    pub(super) fn render_options_bar(&mut self, ui: &mut egui::Ui, theme: ThemeMode) {
        ui.spacing_mut().slider_width = 100.0;
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
                ui.horizontal(|ui: &mut egui::Ui| {
                    match self.tool {
                        Tool::Brush => {
                            ui.label(egui::RichText::new("Size:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.brush.size, 1.0..=500.0));
                            ui.label(egui::RichText::new("Opacity:").size(12.0).color(label_col));
                            ui.add(egui::Slider::new(&mut self.brush.opacity, 0.0..=1.0).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)));
                            ui.separator();
                            let settings_active: bool = self.filter_panel == FilterPanel::Brush;
                            let (settings_bg, settings_txt) = if settings_active {
                                (ColorPalette::BLUE_600, egui::Color32::WHITE)
                            } else if matches!(theme, ThemeMode::Dark) {
                                (ColorPalette::ZINC_700, ColorPalette::ZINC_200)
                            } else {
                                (ColorPalette::GRAY_200, ColorPalette::GRAY_800)
                            };
                            ui.scope(|ui: &mut egui::Ui| {
                                ui.style_mut().visuals.widgets.inactive.bg_fill = settings_bg;
                                ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                ui.style_mut().visuals.widgets.hovered.bg_fill = settings_bg;
                                if ui.add(egui::Button::new(
                                    egui::RichText::new("Brush Settings").size(12.0).color(settings_txt)
                                ).min_size(egui::vec2(0.0, 24.0))).clicked() {
                                    self.filter_panel = if settings_active { FilterPanel::None } else { FilterPanel::Brush };
                                }
                            });
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
                            let cur_font: String = self.text_font_name.clone();
                            egui::ComboBox::from_id_salt("text_font_pick")
                                .selected_text(cur_font.as_str()).width(90.0)
                                .show_ui(ui, |ui| {
                                    for f in &["Ubuntu", "Roboto"] {
                                        if ui.selectable_label(self.text_font_name == *f, *f).clicked() {
                                            self.text_font_name = f.to_string();
                                            if let Some(id) = self.selected_text {
                                                if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut crate::modules::image_editor::ie_main::TextLayer| l.id == id) {
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
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut crate::modules::image_editor::ie_main::TextLayer| l.id == id) { layer.font_size = fs; }
                                }
                            }
                            ui.separator();

                            let style_btn = |ui: &mut egui::Ui, label: egui::RichText, active: bool, theme: ThemeMode| -> bool {
                                let (bg, txt) = if active {
                                    (ColorPalette::BLUE_600, egui::Color32::WHITE)
                                } else if matches!(theme, ThemeMode::Dark) {
                                    (ColorPalette::ZINC_700, ColorPalette::ZINC_200)
                                } else {
                                    (ColorPalette::GRAY_200, ColorPalette::GRAY_800)
                                };
                                ui.scope(|ui: &mut egui::Ui| {
                                    let s: &mut egui::Style = ui.style_mut();
                                    s.visuals.widgets.inactive.bg_fill = bg;
                                    s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                    s.visuals.widgets.hovered.bg_fill = bg;
                                    ui.add(egui::Button::new(label.color(txt)).min_size(egui::vec2(24.0, 24.0)))
                                }).inner.clicked()
                            };

                            if style_btn(ui, egui::RichText::new("B").strong().size(13.0), self.text_bold, theme) {
                                self.text_bold = !self.text_bold;
                                if let Some(id) = self.selected_text {
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut crate::modules::image_editor::ie_main::TextLayer| l.id == id) { layer.bold = self.text_bold; }
                                }
                            }
                            if style_btn(ui, egui::RichText::new("I").italics().size(13.0), self.text_italic, theme) {
                                self.text_italic = !self.text_italic;
                                if let Some(id) = self.selected_text {
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut crate::modules::image_editor::ie_main::TextLayer| l.id == id) { layer.italic = self.text_italic; }
                                }
                            }
                            if style_btn(ui, egui::RichText::new("U").underline().size(13.0), self.text_underline, theme) {
                                self.text_underline = !self.text_underline;
                                if let Some(id) = self.selected_text {
                                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut crate::modules::image_editor::ie_main::TextLayer| l.id == id) { layer.underline = self.text_underline; }
                                }
                            }

                            if let Some(id) = self.selected_text {
                                ui.separator();
                                if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut crate::modules::image_editor::ie_main::TextLayer| l.id == id) { layer.color = self.color; }
                                if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut crate::modules::image_editor::ie_main::TextLayer| l.id == id) {
                                    ui.separator();
                                    ui.label(egui::RichText::new("Rot:").size(12.0).color(label_col));
                                    ui.add(egui::DragValue::new(&mut layer.rotation).speed(1.0).range(-360.0..=360.0).suffix("°")).on_hover_text("Rotation in degrees");
                                }
                                if ui.button("Deselect").clicked() { self.commit_or_discard_active_text(); }
                                if ui.button("Delete").clicked() {
                                    let del_id: u64 = id;
                                    self.text_layers.retain(|l: &crate::modules::image_editor::ie_main::TextLayer| l.id != del_id);
                                    self.selected_text = None; self.editing_text = false;
                                }
                            }
                            if !self.text_layers.is_empty() {
                                ui.separator();
                                ui.label(egui::RichText::new(format!("{} layer(s)", self.text_layers.len())).size(11.0).color(label_col));
                            }
                        }
                        Tool::Crop => {
                            if self.crop_state.start.is_some() && self.crop_state.end.is_some() {
                                if ui.button("Apply Crop").clicked() { self.push_undo(); self.apply_crop(); }
                                if ui.button("Cancel").clicked()     { self.crop_state = CropState::default(); }
                            }
                        }
                        _ => {}
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                        let color_btn: egui::Button<'_> = egui::Button::new("").fill(self.color).min_size(egui::vec2(28.0, 28.0));
                        if ui.add(color_btn).clicked() { self.show_color_picker = !self.show_color_picker; }
                        ui.label(egui::RichText::new("Color:").size(12.0).color(label_col));

                        if let Some(img) = &self.image {
                            ui.label(egui::RichText::new(format!("{}x{}", img.width(), img.height())).size(12.0).color(label_col));
                            ui.label(egui::RichText::new(format!("{:.0}%", self.zoom * 100.0)).size(12.0).color(label_col));
                            ui.label(egui::RichText::new("Zoom:").size(12.0).color(label_col));
                        }
                    });
                });
            });
    }

    pub(super) fn render_filter_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, theme: ThemeMode) {
        if self.filter_panel == FilterPanel::None { return; }
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
            .fixed_size(egui::vec2(320.0, 0.0))
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.5, border)).corner_radius(8.0).inner_margin(16.0))
            .show(ctx, |ui: &mut egui::Ui| {
                ui.spacing_mut().slider_width = 200.0;
                if self.is_processing {
                    let progress_val: f32 = *self.filter_progress.lock().unwrap();
                    ui.label(egui::RichText::new("Processing Filter...").size(13.0).color(text_col));
                    ui.add_space(8.0);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(300.0), 28.0), egui::Sense::hover());
                    let progress_bg: egui::Color32 = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_200 };
                    ui.painter().rect_filled(rect, 4.0, progress_bg);
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
                        ui.horizontal(|ui: &mut egui::Ui| { ui.label(egui::RichText::new("Brightness:").size(12.0).color(label_col)); ui.add(egui::Slider::new(&mut self.brightness, -100.0..=100.0)); });
                        ui.horizontal(|ui: &mut egui::Ui| { ui.label(egui::RichText::new("Contrast:").size(12.0).color(label_col)); ui.add(egui::Slider::new(&mut self.contrast, -100.0..=100.0)); });
                        ui.horizontal(|ui: &mut egui::Ui| {
                            if ui.button("Apply").clicked() { self.push_undo(); self.apply_brightness_contrast(); }
                            if ui.button("Cancel").clicked() { self.brightness = 0.0; self.contrast = 0.0; self.filter_panel = FilterPanel::None; }
                        });
                    }
                    FilterPanel::HueSaturation => {
                        ui.horizontal(|ui: &mut egui::Ui| { ui.label(egui::RichText::new("Hue:").size(12.0).color(label_col)); ui.add(egui::Slider::new(&mut self.hue, -180.0..=180.0)); });
                        ui.horizontal(|ui: &mut egui::Ui| { ui.label(egui::RichText::new("Saturation:").size(12.0).color(label_col)); ui.add(egui::Slider::new(&mut self.saturation, -100.0..=100.0)); });
                        ui.horizontal(|ui: &mut egui::Ui| {
                            if ui.button("Apply").clicked() { self.push_undo(); self.apply_hue_saturation(); }
                            if ui.button("Cancel").clicked() { self.hue = 0.0; self.saturation = 0.0; self.filter_panel = FilterPanel::None; }
                        });
                    }
                    FilterPanel::Blur => {
                        ui.horizontal(|ui: &mut egui::Ui| { ui.label(egui::RichText::new("Radius:").size(12.0).color(label_col)); ui.add(egui::Slider::new(&mut self.blur_radius, 0.5..=20.0)); });
                        ui.horizontal(|ui: &mut egui::Ui| {
                            if ui.button("Apply").clicked() { self.push_undo(); self.apply_blur(); }
                            if ui.button("Cancel").clicked() { self.blur_radius = 3.0; self.filter_panel = FilterPanel::None; }
                        });
                    }
                    FilterPanel::Sharpen => {
                        ui.horizontal(|ui: &mut egui::Ui| { ui.label(egui::RichText::new("Amount:").size(12.0).color(label_col)); ui.add(egui::Slider::new(&mut self.sharpen_amount, 0.1..=5.0)); });
                        ui.horizontal(|ui: &mut egui::Ui| {
                            if ui.button("Apply").clicked() { self.push_undo(); self.apply_sharpen(); }
                            if ui.button("Cancel").clicked() { self.sharpen_amount = 1.0; self.filter_panel = FilterPanel::None; }
                        });
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
            .fixed_size(egui::vec2(340.0, 0.0))
            .frame(egui::Frame::new().fill(bg).stroke(egui::Stroke::new(1.5, border)).corner_radius(8.0).inner_margin(16.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                let mut rgb: [f32; 3] = [self.color.r() as f32 / 255.0, self.color.g() as f32 / 255.0, self.color.b() as f32 / 255.0];
                let (h_current, s, v) = rgb_to_hsv_f32(rgb[0], rgb[1], rgb[2]);
                let hue_id: egui::Id = ui.make_persistent_id("picker_hue_state");
                let mut h: f32  = ui.data(|d| d.get_temp(hue_id)).unwrap_or(h_current);
                if s > 0.005 && v > 0.005 { h = h_current; ui.data_mut(|d| d.insert_temp(hue_id, h)); }

                let mut color_changed: bool = false;
                let picker_size: egui::Vec2 = egui::vec2(280.0, 280.0);
                let (rect, response) = ui.allocate_exact_size(picker_size, egui::Sense::click_and_drag());

                if ui.is_rect_visible(rect) {
                    let painter: egui::Painter = ui.painter_at(rect);
                    let steps: i32 = 40;
                    let cell_w: f32 = rect.width() / steps as f32;
                    let cell_h: f32 = rect.height() / steps as f32;
                    for y in 0..steps {
                        for x in 0..steps {
                            let s_cell: f32 = x as f32 / (steps - 1) as f32;
                            let v_cell: f32 = 1.0 - (y as f32 / (steps - 1) as f32);
                            let (r, g, b) = hsv_to_rgb_f32(h, s_cell, v_cell);
                            let color: egui::Color32 = egui::Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
                            painter.rect_filled(egui::Rect::from_min_size(
                                egui::pos2(rect.min.x + x as f32 * cell_w, rect.min.y + y as f32 * cell_h),
                                egui::vec2(cell_w.ceil(), cell_h.ceil()),
                            ), 0.0, color);
                        }
                    }
                    let cursor_pos: egui::Pos2 = egui::pos2(rect.min.x + s * rect.width(), rect.min.y + (1.0 - v) * rect.height());
                    painter.circle_stroke(cursor_pos, 6.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
                    painter.circle_stroke(cursor_pos, 6.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
                }

                if response.clicked() || response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let x: f32 = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                        let y: f32 = ((pos.y - rect.min.y) / rect.height()).clamp(0.0, 1.0);
                        let (r, g, b) = hsv_to_rgb_f32(h, x, 1.0 - y);
                        rgb = [r, g, b]; color_changed = true;
                    }
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Hue:").size(12.0).color(weak_col));
                    let hue_size: egui::Vec2 = egui::vec2(ui.available_width(), 24.0);
                    let (hue_rect, hue_response) = ui.allocate_exact_size(hue_size, egui::Sense::click_and_drag());

                    if ui.is_rect_visible(hue_rect) {
                        let painter: egui::Painter = ui.painter_at(hue_rect);
                        let steps: i32 = 60;
                        let step_w: f32 = hue_rect.width() / steps as f32;

                        for i in 0..steps {
                            let h_step: f32 = (i as f32 / steps as f32) * 360.0;
                            let (r, g, b) = hsv_to_rgb_f32(h_step, 1.0, 1.0);
                            let color: egui::Color32 = egui::Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
                            painter.rect_filled(egui::Rect::from_min_size(
                                egui::pos2(hue_rect.min.x + i as f32 * step_w, hue_rect.min.y),
                                egui::vec2(step_w.ceil(), hue_rect.height()),
                            ), 0.0, color);
                        }

                        painter.rect_stroke(hue_rect, 2.0, egui::Stroke::new(1.0,
                            if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 }
                        ), egui::StrokeKind::Outside);

                        let hue_cursor_x: f32 = hue_rect.min.x + (h / 360.0) * hue_rect.width();
                        let hcr: egui::Rect = egui::Rect::from_center_size(egui::pos2(hue_cursor_x, hue_rect.center().y), egui::vec2(4.0, hue_rect.height() + 2.0));
                        painter.rect_filled(hcr, 2.0, egui::Color32::WHITE);
                        painter.rect_stroke(hcr, 2.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Outside);
                    }

                    if hue_response.clicked() || hue_response.dragged() {
                        if let Some(pos) = hue_response.interact_pointer_pos() {
                            let x: f32 = ((pos.x - hue_rect.min.x) / hue_rect.width()).clamp(0.0, 1.0);
                            h = x * 360.0;
                            ui.data_mut(|d: &mut egui::util::IdTypeMap| d.insert_temp(hue_id, h));
                            let (_, s_curr, v_curr) = rgb_to_hsv_f32(rgb[0], rgb[1], rgb[2]);
                            let (r, g, b) = hsv_to_rgb_f32(h, s_curr, v_curr);
                            rgb = [r, g, b]; color_changed = true;
                        }
                    }
                });

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
                    let response: egui::Response = ui.text_edit_singleline(&mut self.hex_input);
                    if response.changed() {
                        if let Some(mut c) = RgbaColor::from_hex(&self.hex_input) { c.a = 255; self.color = c.to_egui(); }
                    }
                    if response.lost_focus() { self.hex_input = RgbaColor::from_egui(self.color).to_hex(); }
                    if ui.small_button("Copy").clicked() { ctx.copy_text(self.hex_input.clone()); }
                });

                ui.add_space(4.0); ui.separator(); ui.add_space(4.0);
                ui.horizontal(|ui: &mut egui::Ui| {
                    ui.label(egui::RichText::new("Recent").size(13.0).color(text_col));
                    if ui.small_button("Clear").clicked() { self.color_history = super::ie_main::ColorHistory::new(); }
                });
                
                ui.horizontal_wrapped(|ui: &mut egui::Ui| {
                    let history: std::collections::VecDeque<RgbaColor> = self.color_history.get_colors().clone();
                    for color in history.iter() {
                        if ui.add(egui::Button::new("").fill(color.to_egui()).min_size(egui::vec2(28.0, 28.0))).clicked() {
                            let mut c: RgbaColor = *color; c.a = 255; self.color = c.to_egui(); self.hex_input = c.to_hex();
                        }
                    }
                });

                ui.add_space(8.0);
                ui.horizontal(|ui: &mut egui::Ui| {
                    if ui.button("Apply").clicked()  { self.add_color_to_history(); self.show_color_picker = false; }
                    if ui.button("Cancel").clicked() { self.show_color_picker = false; }
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

        let checker_size: f32 = 16.0;
        let (c1, c2) = if ui.visuals().dark_mode {
            (egui::Color32::from_rgb(40, 40, 40), egui::Color32::from_rgb(55, 55, 55))
        } else {
            (egui::Color32::from_rgb(200, 200, 200), egui::Color32::from_rgb(220, 220, 220))
        };

        let mut cy: f32 = rect.min.y;
        while cy < rect.max.y {
            let mut cx: f32 = rect.min.x;
            let row: i32 = ((cy - rect.min.y) / checker_size) as i32;
            while cx < rect.max.x {
                let col: i32 = ((cx - rect.min.x) / checker_size) as i32;
                let color: egui::Color32 = if (row + col) % 2 == 0 { c1 } else { c2 };
                painter.rect_filled(egui::Rect::from_min_size(egui::pos2(cx, cy), egui::vec2(checker_size, checker_size)), 0.0, color);
                cx += checker_size;
            }
            cy += checker_size;
        }

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

        for layer in &self.text_layers {
            let anchor: egui::Pos2 = self.image_to_screen(layer.img_x, layer.img_y);
            let font_size_screen: f32 = layer.font_size * self.zoom;
            let angle_rad: f32 = layer.rotation.to_radians();
            let (cos_a, sin_a) = (angle_rad.cos(), angle_rad.sin());
            let font_family: egui::FontFamily = egui::FontFamily::Name(layer.font_family_name().into());
            let font_id: egui::FontId = egui::FontId::new(font_size_screen, font_family);
            let box_w_screen: f32 = layer.box_width.map(|w| w * self.zoom).unwrap_or(f32::INFINITY);

            let make_job = |text: &str| {
                let mut job: egui::text::LayoutJob = egui::text::LayoutJob::default();
                job.wrap.max_width = box_w_screen;
                job.append(text, 0.0, egui::TextFormat {
                    font_id: font_id.clone(), color: layer.color, italics: false,
                    underline: if layer.underline { egui::Stroke::new((font_size_screen * 0.06).max(1.0), layer.color) } else { egui::Stroke::NONE },
                    ..Default::default()
                });
                job
            };

            let sel_rect: egui::Rect = layer.screen_rect(anchor, self.zoom);
            let center: egui::Pos2 = sel_rect.center();
            let d: egui::Vec2 = anchor - center;
            let text_pos: egui::Pos2 = center + egui::vec2(d.x * cos_a - d.y * sin_a, d.x * sin_a + d.y * cos_a);
            let galley: std::sync::Arc<egui::Galley> = ui.painter().layout_job(make_job(&layer.content));
            let mut text_shape: egui::epaint::TextShape = egui::epaint::TextShape::new(text_pos, galley.clone(), layer.color);
            text_shape.angle = angle_rad;

            if self.editing_text && self.selected_text == Some(layer.id) {
                let cursor_byte: usize = self.text_cursor;
                let sel_anchor: Option<usize> = self.text_sel_anchor;
                let content: &String = &layer.content;

                let glyph_pos_for = |byte_off: usize| -> egui::Pos2 {
                    let char_idx: usize = content[..byte_off.min(content.len())].chars().count();
                    let mut ci: usize = 0usize;
                    for row in &galley.rows {
                        for g in &row.glyphs {
                            if ci == char_idx { return egui::pos2(g.pos.x, row.rect().min.y); }
                            ci += 1;
                        }
                        if ci == char_idx { return egui::pos2(row.rect().max.x, row.rect().min.y); }
                    }
                    galley.rows.last().map(|r: &egui::epaint::text::PlacedRow| egui::pos2(r.rect().max.x, r.rect().min.y)).unwrap_or(egui::pos2(0.0, 0.0))
                };

                let galley_to_canvas = |lp: egui::Pos2| -> egui::Pos2 {
                    text_pos + egui::vec2(lp.x * cos_a - lp.y * sin_a, lp.x * sin_a + lp.y * cos_a)
                };

                if let Some(anchor_sel) = sel_anchor {
                    let (lo, hi) = (anchor_sel.min(cursor_byte), anchor_sel.max(cursor_byte));
                    let char_lo: usize = content[..lo.min(content.len())].chars().count();
                    let char_hi: usize = content[..hi.min(content.len())].chars().count();
                    let mut ci: usize = 0usize;
                    for row in &galley.rows {
                        let row_start: usize = ci; let row_end: usize = ci + row.glyphs.len();
                        let sel_start_in_row: usize = char_lo.max(row_start);
                        let sel_end_in_row: usize = char_hi.min(row_end);
                        if sel_start_in_row < sel_end_in_row || (char_lo <= row_start && char_hi >= row_end) {
                            let x0: f32 = if sel_start_in_row <= row_start { row.rect().min.x } else { row.glyphs.get(sel_start_in_row - row_start).map(|g| g.pos.x).unwrap_or(row.rect().min.x) };
                            let x1: f32 = if sel_end_in_row >= row_end { row.rect().max.x } else { row.glyphs.get(sel_end_in_row   - row_start).map(|g| g.pos.x).unwrap_or(row.rect().max.x) };
                            let corners: [egui::Pos2; 4] = [
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

                let blink: bool = (ctx.input(|i: &egui::InputState| i.time) * 2.0) as u32 % 2 == 0;
                if blink {
                    let lp: egui::Pos2 = glyph_pos_for(cursor_byte);
                    let row_h: f32 = galley.rows.iter().find(|r| r.rect().min.y <= lp.y && lp.y <= r.rect().max.y).map(|r| r.rect().height()).unwrap_or(font_size_screen);
                    painter.line_segment([galley_to_canvas(lp), galley_to_canvas(egui::pos2(lp.x, lp.y + row_h))], egui::Stroke::new(2.0, layer.color));
                }
                ctx.request_repaint();
            }

            painter.add(egui::Shape::Text(text_shape));
            if self.selected_text == Some(layer.id) {
                TransformHandleSet::with_rotation(sel_rect, angle_rad).draw(&painter, ColorPalette::BLUE_400);
            }
        }

        let zoom: f32 = self.zoom;
        let height_updates: Vec<(u64, f32)> = self.text_layers.iter().map(|layer| {
            let font_family: egui::FontFamily = egui::FontFamily::Name(layer.font_family_name().into());
            let font_id: egui::FontId = egui::FontId::new(layer.font_size * zoom, font_family);
            let box_w_screen: f32 = layer.box_width.map(|w| w * zoom).unwrap_or(f32::INFINITY);
            let mut job: egui::text::LayoutJob = egui::text::LayoutJob::default();
            job.wrap.max_width = box_w_screen;
            job.append(&layer.content, 0.0, egui::TextFormat { font_id, color: layer.color, ..Default::default() });
            (layer.id, ui.painter().layout_job(job).rect.height() / zoom)
        }).collect();
        for (id, h) in height_updates {
            if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) { layer.rendered_height = h.max(layer.font_size); }
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
                    Tool::Pan => ctx.set_cursor_icon(if response.dragged_by(egui::PointerButton::Primary) { egui::CursorIcon::Grabbing } else { egui::CursorIcon::AllScroll }),
                    Tool::Text => ctx.set_cursor_icon(egui::CursorIcon::Text),
                }
                match self.tool {
                    Tool::Brush  => { painter.circle_stroke(mp, self.brush.size  / 2.0 * self.zoom, egui::Stroke::new(1.5, self.color)); }
                    Tool::Eraser => { painter.circle_stroke(mp, self.eraser_size / 2.0 * self.zoom, egui::Stroke::new(1.5, ColorPalette::RED_400)); }
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

        if response.dragged_by(egui::PointerButton::Primary) {
            let pos: egui::Pos2 = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            match self.tool {
                Tool::Pan => { self.pan += response.drag_delta(); }
                Tool::Brush | Tool::Eraser => {
                    if !self.is_dragging { self.push_undo(); self.is_dragging = true; self.stroke_points.clear(); }
                    if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.stroke_points.push((ix as f32, iy as f32));
                        if self.stroke_points.len() >= 2 {
                            self.apply_brush_stroke();
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
                                    THandle::N  => s.1 = iy.min(e.1 - 1.0),
                                    THandle::S  => e.1 = iy.max(s.1 + 1.0),
                                    THandle::W  => s.0 = ix.min(e.0 - 1.0),
                                    THandle::E  => e.0 = ix.max(s.0 + 1.0),
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
                Tool::Text => {
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
                                THandle::Move => 
                                    { 
                                        let delta: egui::Vec2 = pos - drag_start; layer.img_x = orig_ix + delta.x / zoom; 
                                        layer.img_y = orig_iy + delta.y / zoom; 
                                    }
                                THandle::E => 
                                    { 
                                        layer.box_width  = Some(((pos.x - anchor_screen.x).max(min_sz) / zoom).max(1.0));
                                    }
                                THandle::W => 
                                    { 
                                        let orig_right: f32 = anchor_screen.x + orig_w_screen; 
                                        let new_w: f32 = (orig_right - pos.x).max(min_sz); layer.box_width = Some((new_w / zoom).max(1.0)); 
                                        layer.img_x = (pos.x - ox) / zoom; 
                                    }
                                THandle::S => 
                                    { 
                                        layer.box_height = Some(((pos.y - anchor_screen.y).max(min_sz) / zoom).max(1.0)); 
                                    }
                                THandle::N => 
                                    { 
                                        let orig_bottom: f32 = anchor_screen.y + orig_h_screen; 
                                        let new_h: f32 = (orig_bottom - pos.y).max(min_sz); 
                                        layer.box_height = Some((new_h / zoom).max(1.0)); 
                                        layer.img_y = ((orig_bottom - new_h) - oy) / zoom; 
                                    }
                                THandle::SE => 
                                    { 
                                        layer.box_width  = Some(((pos.x - anchor_screen.x).max(min_sz) / zoom).max(1.0)); 
                                        layer.box_height = Some(((pos.y - anchor_screen.y).max(min_sz) / zoom).max(1.0)); 
                                    }
                                THandle::NE => 
                                    { 
                                        let orig_bottom: f32 = anchor_screen.y + orig_h_screen; 
                                        let new_h: f32 = (orig_bottom - pos.y).max(min_sz); 
                                        layer.box_width = Some(((pos.x - anchor_screen.x).max(min_sz) / zoom).max(1.0)); 
                                        layer.box_height = Some((new_h / zoom).max(1.0)); 
                                        layer.img_y = ((orig_bottom - new_h) - oy) / zoom; 
                                    }
                                THandle::NW => 
                                    { 
                                        let orig_right: f32 = anchor_screen.x + orig_w_screen; 
                                        let orig_bottom: f32 = anchor_screen.y + orig_h_screen; 
                                        let new_w: f32 = (orig_right - pos.x).max(min_sz); 
                                        let new_h: f32 = (orig_bottom - pos.y).max(min_sz); 
                                        layer.box_width = Some((new_w / zoom).max(1.0)); 
                                        layer.box_height = Some((new_h / zoom).max(1.0)); 
                                        layer.img_x = (pos.x - ox) / zoom; 
                                        layer.img_y = ((orig_bottom - new_h) - oy) / zoom; 
                                    }
                                THandle::SW => 
                                    { 
                                        let orig_right: f32 = anchor_screen.x + orig_w_screen; 
                                        let new_w: f32 = (orig_right - pos.x).max(min_sz); 
                                        layer.box_width = Some((new_w / zoom).max(1.0)); 
                                        layer.box_height = Some(((pos.y - anchor_screen.y).max(min_sz) / zoom).max(1.0)); 
                                        layer.img_x = (pos.x - ox) / zoom; 
                                    }
                                THandle::Rotate => 
                                    { 
                                        let cur_angle: f32 = (pos - rot_center).angle(); 
                                        layer.rotation = orig_rot + (cur_angle - orig_rot_start).to_degrees(); 
                                    }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary) && self.tool == Tool::Crop {
            let pos = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            let handle_hit = if let (Some(s), Some(e)) = (self.crop_state.start, self.crop_state.end) {
                let p0 = self.image_to_screen(s.0, s.1);
                let p1 = self.image_to_screen(e.0, e.1);
                let cr = egui::Rect::from_two_pos(p0, p1);
                if cr.width() > HANDLE_HIT && cr.height() > HANDLE_HIT {
                    crop_hit_handle(pos, cr)
                } else { None }
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

        if response.drag_started_by(egui::PointerButton::Primary) && self.tool == Tool::Text {
            let pos: egui::Pos2 = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            self.text_drag = None;
            if let Some(id) = self.selected_text {
                if let Some(handles) = self.text_transform_handles() {
                    if let Some(h) = handles.hit_test(pos) {
                        if let Some(layer) = self.text_layers.iter().find(|l: &&super::ie_main::TextLayer| l.id == id) {
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

        if response.drag_stopped_by(egui::PointerButton::Primary) {
            match self.tool {
                Tool::Brush | Tool::Eraser => { self.texture_dirty = true; self.stroke_points.clear(); self.is_dragging = false; }
                Tool::Text => { self.text_drag = None; }
                Tool::Crop => { self.crop_drag = None; self.crop_drag_orig = None; }
                _ => {}
            }
        }

        if response.clicked_by(egui::PointerButton::Primary) {
            let pos: egui::Pos2 = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            match self.tool {
                Tool::Brush | Tool::Eraser => {
                    if let Some((ix, iy)) = self.screen_to_image(pos) {
                        self.push_undo();
                        self.stroke_points.clear();
                        self.stroke_points.push((ix as f32, iy as f32));
                        self.stroke_points.push((ix as f32 + 0.1, iy as f32 + 0.1));
                        self.apply_brush_stroke();
                        self.stroke_points.clear();
                        if self.tool == Tool::Brush { self.add_color_to_history(); }
                    }
                }
                Tool::Fill => {
                    if let Some((ix, iy)) = self.screen_to_image(pos) { self.push_undo(); self.flood_fill(ix, iy); self.add_color_to_history(); }
                }
                Tool::Eyedropper => {
                    if let Some((ix, iy)) = self.screen_to_image(pos) { self.sample_color(ix, iy); }
                }
                Tool::Text => {
                    if let Some(hit) = self.hit_text_layer(pos) {
                        self.selected_text = Some(hit); self.editing_text = true; self.text_sel_anchor = None;
                        if let Some(layer) = self.text_layers.iter().find(|l| l.id == hit) {
                            self.text_font_size = layer.font_size; self.text_bold = layer.bold;
                            self.text_italic = layer.italic; self.text_underline = layer.underline;
                            self.text_font_name = layer.font_name.clone(); self.text_cursor = layer.content.len();
                        }
                    } else {
                        self.commit_or_discard_active_text();
                        if let Some((ix, iy)) = self.screen_to_image(pos) {
                            let id: u64 = self.next_text_id; self.next_text_id += 1;
                            self.text_layers.push(super::ie_main::TextLayer {
                                id, content: String::new(),
                                img_x: ix as f32, img_y: iy as f32,
                                font_size: self.text_font_size, box_width: Some(300.0), box_height: None,
                                rotation: 0.0, color: self.color,
                                bold: self.text_bold, italic: self.text_italic, underline: self.text_underline,
                                font_name: self.text_font_name.clone(), rendered_height: 0.0,
                            });
                            self.selected_text = Some(id); self.editing_text = true;
                            self.text_cursor = 0; self.text_sel_anchor = None;
                        }
                    }
                }
                _ => {}
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
        let (bg, border, text_col, label_col, section_col, row_hover) = if matches!(theme, ThemeMode::Dark) {
            (
                ColorPalette::ZINC_900,
                ColorPalette::BLUE_600,
                ColorPalette::ZINC_100,
                ColorPalette::ZINC_400,
                ColorPalette::ZINC_500,
                ColorPalette::ZINC_800,
            )
        } else {
            (
                ColorPalette::GRAY_50,
                ColorPalette::BLUE_600,
                ColorPalette::GRAY_900,
                ColorPalette::ZINC_600,
                ColorPalette::ZINC_400,
                ColorPalette::GRAY_100,
            )
        };
        let _ = (section_col, row_hover);

        let accent: egui::Color32 = ColorPalette::BLUE_500;
        let sep_col: egui::Color32 = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 };
        let _ = sep_col;
        let screen_h = ctx.content_rect().height();
        let panel_max_h = (screen_h - 130.0).max(300.0);
        let canvas_origin: egui::Pos2 = ui.available_rect_before_wrap().min;
        let modal_pos: egui::Pos2 = canvas_origin + egui::vec2(10.0, 10.0);

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

                        section_label(ui, "SHAPE");
                        egui::Frame::new()
                            .inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 6, bottom: 6 })
                            .show(ui, |ui: &mut egui::Ui| {
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    for shape in BrushShape::all() {
                                        let is_active = self.brush.shape == *shape;
                                        let btn_size = egui::vec2(68.0, 46.0);
                                        let (frame_fill, lbl_col) = if is_active {
                                            (ColorPalette::BLUE_600, egui::Color32::WHITE)
                                        } else if matches!(theme, ThemeMode::Dark) {
                                            (ColorPalette::ZINC_800, ColorPalette::ZINC_300)
                                        } else {
                                            (ColorPalette::GRAY_200, ColorPalette::GRAY_700)
                                        };
                                        let border_col = if is_active { ColorPalette::BLUE_400 } else { if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 } };
                                        let (rect, resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());
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
                                                BrushShape::Star => {
                                                    let outer = ir;
                                                    let inner = ir * 0.40;
                                                    let n = 5_usize;
                                                    let mut pts = Vec::with_capacity(n * 2);
                                                    for k in 0..n * 2 {
                                                        let a_star = k as f32 * std::f32::consts::PI / n as f32 - std::f32::consts::PI / 2.0;
                                                        let r_star = if k % 2 == 0 { outer } else { inner };
                                                        pts.push(egui::pos2(ic.x + a_star.cos() * r_star, ic.y + a_star.sin() * r_star));
                                                    }
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
                                        if resp.clicked() { self.brush.shape = *shape; }
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
                                        ui.add(egui::Slider::new(&mut self.brush.size, 1.0..=200.0).show_value(false));
                                    });
                                });
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Opacity").size(12.0).color(label_col)).on_hover_text("Maximum alpha of the overall stroke.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("{:.0}%", self.brush.opacity * 100.0)).size(11.0).color(text_col));
                                        ui.add(egui::Slider::new(&mut self.brush.opacity, 0.0..=1.0).show_value(false));
                                    });
                                });
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Softness").size(12.0).color(label_col)).on_hover_text("0% = hard pixel-sharp edge.\n100% = fully feathered, airbrushed falloff.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("{:.0}%", self.brush.softness * 100.0)).size(11.0).color(text_col));
                                        ui.add(egui::Slider::new(&mut self.brush.softness, 0.0..=1.0).show_value(false));
                                    });
                                });
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Flow").size(12.0).color(label_col)).on_hover_text("Per-stamp opacity. Low flow builds color gradually;\nhigh flow paints solidly each stamp.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("{:.0}%", self.brush.flow * 100.0)).size(11.0).color(text_col));
                                        ui.add(egui::Slider::new(&mut self.brush.flow, 0.01..=1.0).show_value(false));
                                    });
                                });
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Spacing").size(12.0).color(label_col)).on_hover_text("Distance between consecutive stamp positions,\nas a fraction of brush diameter.\nLow = dense/continuous; high = dotted.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("{:.0}%", self.brush.step * 100.0)).size(11.0).color(text_col));
                                        ui.add(egui::Slider::new(&mut self.brush.step, 0.02..=3.0).show_value(false));
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
                                            ui.add(egui::Slider::new(&mut self.brush.angle, -180.0..=180.0).show_value(false));
                                        });
                                    });
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(egui::RichText::new("Angle Jitter").size(12.0).color(label_col)).on_hover_text("Max random rotation added per stamp. Creates organic, hand-drawn variation.");
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(egui::RichText::new(format!("±{:.0}°", self.brush.angle_jitter)).size(11.0).color(text_col));
                                            ui.add(egui::Slider::new(&mut self.brush.angle_jitter, 0.0..=180.0).show_value(false));
                                        });
                                    });
                                    if needs_aspect {
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            ui.label(egui::RichText::new("Aspect Ratio").size(12.0).color(label_col)).on_hover_text("Width-to-height ratio of the flat calligraphy nib.\n0.05 = very thin stroke; 1.0 = circular.");
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(egui::RichText::new(format!("{:.2}", self.brush.aspect_ratio)).size(11.0).color(text_col));
                                                ui.add(egui::Slider::new(&mut self.brush.aspect_ratio, 0.05..=1.0).show_value(false));
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
                                        ui.add(egui::Slider::new(&mut self.brush.scatter, 0.0..=200.0).show_value(false));
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
                                            let (bg_c, txt_c) = if is_active {
                                                (accent, egui::Color32::WHITE)
                                            } else if matches!(theme, ThemeMode::Dark) {
                                                (ColorPalette::ZINC_700, ColorPalette::ZINC_300)
                                            } else {
                                                (ColorPalette::GRAY_200, ColorPalette::GRAY_700)
                                            };
                                            ui.scope(|ui: &mut egui::Ui| {
                                                ui.style_mut().visuals.widgets.inactive.bg_fill = bg_c;
                                                ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                                ui.style_mut().visuals.widgets.hovered.bg_fill = bg_c;
                                                if ui.add(egui::Button::new(
                                                    egui::RichText::new(mode.label()).size(11.0).color(txt_c)
                                                ).min_size(egui::vec2(0.0, 20.0))).clicked() {
                                                    self.brush.texture_mode = *mode;
                                                }
                                            });
                                        }
                                    });
                                });
                                if self.brush.texture_mode != BrushTextureMode::None {
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(egui::RichText::new("Texture Strength").size(12.0).color(label_col));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(egui::RichText::new(format!("{:.0}%", self.brush.texture_strength * 100.0)).size(11.0).color(text_col));
                                            ui.add(egui::Slider::new(&mut self.brush.texture_strength, 0.0..=1.0).show_value(false));
                                        });
                                    });
                                }

                                ui.add_space(4.0);
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Spray Mode").size(12.0).color(label_col)).on_hover_text("Replaces solid stamp with randomly-scattered individual dots\nfor an aerosol spray-can effect.");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.add(egui::Checkbox::new(&mut self.brush.spray_mode, ""));
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

                                for row in presets.chunks(cols) {
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        for preset in row {
                                            let (bg_c, txt_c) = if matches!(theme, ThemeMode::Dark) {
                                                (ColorPalette::ZINC_800, ColorPalette::ZINC_200)
                                            } else {
                                                (ColorPalette::GRAY_200, ColorPalette::GRAY_800)
                                            };
                                            let border_c = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_400 };
                                            let hover_c  = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 };
                                            ui.scope(|ui: &mut egui::Ui| {
                                                let s = ui.style_mut();
                                                s.visuals.widgets.inactive.bg_fill   = bg_c;
                                                s.visuals.widgets.inactive.bg_stroke  = egui::Stroke::new(1.0, border_c);
                                                s.visuals.widgets.hovered.bg_fill    = hover_c;
                                                s.visuals.widgets.hovered.bg_stroke   = egui::Stroke::new(1.0, accent);
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
                                                    let current_size = self.brush.size;
                                                    self.brush = preset.settings(current_size);
                                                }
                                            });
                                        }
                                    });
                                }
                            });

                        section_label(ui, "FAVORITES");
                        egui::Frame::new()
                            .inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 8, bottom: 10 })
                            .show(ui, |ui: &mut egui::Ui| {
                                ui.horizontal(|ui: &mut egui::Ui| {
                                    ui.label(egui::RichText::new("Name:").size(12.0).color(label_col));
                                    ui.add(egui::TextEdit::singleline(&mut self.brush_fav_name)
                                        .desired_width(200.0)
                                        .font(egui::TextStyle::Body)
                                        .hint_text("Enter a name for this brush...")
                                    );
                                    let can_save = !self.brush_fav_name.trim().is_empty();
                                    ui.scope(|ui: &mut egui::Ui| {
                                        let s = ui.style_mut();
                                        s.visuals.widgets.inactive.bg_fill = if can_save { ColorPalette::BLUE_600 } else { if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_300 } };
                                        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                        s.visuals.widgets.hovered.bg_fill = if can_save { ColorPalette::BLUE_500 } else { s.visuals.widgets.inactive.bg_fill };
                                        s.visuals.override_text_color = Some(if can_save { egui::Color32::WHITE } else { if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_500 } else { ColorPalette::GRAY_500 } });
                                        let save_btn = ui.add_enabled(can_save, egui::Button::new(
                                            egui::RichText::new("Save").size(12.0)
                                        ).min_size(egui::vec2(54.0, 24.0)));
                                        if save_btn.clicked() {
                                            let name = self.brush_fav_name.trim().to_string();
                                            if let Some(existing) = self.brush_favorites.brushes.iter_mut().find(|b| b.name == name) {
                                                existing.settings = self.brush.clone();
                                            } else {
                                                self.brush_favorites.brushes.push(SavedBrush {
                                                    name,
                                                    settings: self.brush.clone(),
                                                });
                                            }
                                            self.brush_favorites.save();
                                            self.brush_fav_name.clear();
                                        }
                                    });
                                });

                                ui.add_space(6.0);

                                if self.brush_favorites.brushes.is_empty() {
                                    ui.label(egui::RichText::new("No saved brushes yet. Configure a brush above and save it.").size(11.0).color(label_col).italics());
                                } else {
                                    let mut to_load: Option<usize> = None;
                                    let mut to_delete: Option<usize> = None;

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
                                                    }
                                                    ui.label(egui::RichText::new(&saved.name).size(12.0).color(text_col));
                                                    let desc = format!("{} / {:.0}px / S{:.0}%", saved.settings.shape.label(), saved.settings.size, saved.settings.softness * 100.0, );
                                                    ui.label(egui::RichText::new(desc).size(10.0).color(label_col));
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                        let del_col = egui::Color32::from_rgb(180, 60, 60);
                                                        ui.scope(|ui: &mut egui::Ui| {
                                                            ui.style_mut().visuals.widgets.inactive.bg_fill = del_col;
                                                            ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                                            ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(200, 80, 80);
                                                            if ui.add(egui::Button::new(egui::RichText::new("Delete").size(11.0).color(egui::Color32::WHITE)).min_size(egui::vec2(52.0, 22.0))).clicked() {
                                                                to_delete = Some(idx);
                                                            }
                                                        });
                                                        ui.scope(|ui: &mut egui::Ui| {
                                                            ui.style_mut().visuals.widgets.inactive.bg_fill = accent;
                                                            ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                                            ui.style_mut().visuals.widgets.hovered.bg_fill = ColorPalette::BLUE_400;
                                                            if ui.add(egui::Button::new(egui::RichText::new("Load").size(11.0).color(egui::Color32::WHITE)).min_size(egui::vec2(46.0, 22.0))).clicked() {
                                                                to_load = Some(idx);
                                                            }
                                                        });
                                                    });
                                                });
                                            });
                                        ui.add_space(2.0);
                                    }

                                    if let Some(idx) = to_load {
                                        self.brush = self.brush_favorites.brushes[idx].settings.clone();
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
                                ui.scope(|ui: &mut egui::Ui| {
                                    ui.style_mut().visuals.widgets.inactive.bg_fill = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_700 } else { ColorPalette::GRAY_200 };
                                    ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                    ui.style_mut().visuals.widgets.hovered.bg_fill = if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_600 } else { ColorPalette::GRAY_300 };
                                    if ui.add(egui::Button::new(
                                        egui::RichText::new("Close Panel").size(12.0).color(if matches!(theme, ThemeMode::Dark) { ColorPalette::ZINC_300 } else { ColorPalette::GRAY_700 })
                                    ).min_size(egui::vec2(ui.available_width(), 28.0))).clicked() {
                                        self.filter_panel = FilterPanel::None;
                                    }
                                });
                            });
                    });
            });
        self.filter_panel_rect = win_resp.map(|r| r.response.rect);
    }
}
