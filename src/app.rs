use eframe::egui;
use super::style;
use super::modules::{EditorModule, text_editor::TextEditor};

pub struct UniversalEditor {
    active_module: Option<Box<dyn EditorModule>>,
    sidebar_open: bool,
}

impl UniversalEditor {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        style::configure_modern_style(&cc.egui_ctx);
        Self {
            active_module: None,
            sidebar_open: true,
        }
    }

    fn render_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Text File").clicked() {
                        self.active_module = Some(Box::new(TextEditor::new_empty()));
                        ui.close_menu();
                    }
                    if ui.button("Open...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("Text Files", &["txt", "md"]).pick_file() {
                            self.active_module = Some(Box::new(TextEditor::load(path)));
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Save (Ctrl+S)").clicked() {
                        if let Some(module) = &mut self.active_module {
                            let _ = module.save();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save As...").clicked() {
                        if let Some(module) = &mut self.active_module {
                            let _ = module.save_as();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                   ui.checkbox(&mut self.sidebar_open, "Show Sidebar"); 
                });
            });
            ui.add_space(4.0);
        });
    }

    fn render_sidebar(&mut self, ctx: &egui::Context) {
        if !self.sidebar_open { return; }
        
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.heading("Explorer");
                ui.separator();
                
                ui.label("Open Editors:");
                if let Some(module) = &self.active_module {
                    ui.label(format!("📄 {}", module.get_title()));
                } else {
                    ui.weak("No files open");
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.weak(format!("FPS: {:.0}", 1.0 / ctx.input(|i| i.unstable_dt)));
                });
            });
    }

    fn render_landing_page(&mut self, ui: &mut egui::Ui) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.heading("UNIVERSAL EDITOR");
                ui.add_space(20.0);

                if style::primary_button(ui, "New Text File").clicked() {
                    self.active_module = Some(Box::new(TextEditor::new_empty()));
                }
                ui.add_space(10.0);
                if style::secondary_button(ui, "Open File").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Text Files", &["txt", "md"]).pick_file() {
                        self.active_module = Some(Box::new(TextEditor::load(path)));
                    }
                }
            });
        });
    }
}

impl eframe::App for UniversalEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_top_bar(ctx);
        self.render_sidebar(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(module) = &mut self.active_module {
                module.ui(ui, ctx);
            } else {
                self.render_landing_page(ui);
            }
        });
    }
}
