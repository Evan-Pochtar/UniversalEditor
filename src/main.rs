use eframe::egui;
use ropey::Rope;
use std::fs::File;
use std::io::{BufReader};
use std::path::{PathBuf};

// --- KERNEL ---
trait EditorModule {
    fn ui(&mut self, ui: &mut egui::Ui);
    fn save(&mut self) -> Result<(), String>;
}

// --- TEXT EDITOR ---
struct TextEditor {
    file_path: Option<PathBuf>,
    content: Rope
}

impl TextEditor {
    fn new_empty() -> Self {
        Self {
            file_path: None,
            content: Rope::from_str("")
        }
    }

    fn load(path: PathBuf) -> Self {
        let file = File::open(&path).expect("Failed to open file");
        let reader = BufReader::new(file);
        let rope = Rope::from_reader(reader).expect("Failed to parse text");
        
        Self {
            file_path: Some(path),
            content: rope
        }
    }
}

impl EditorModule for TextEditor {
    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut text = self.content.to_string();
            let response = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut text)
                    .code_editor() // Monospace font
                    .lock_focus(true)
            );

            if response.changed() {
                self.content = Rope::from_str(&text);
            }
        });
    }

    fn save(&mut self) -> Result<(), String> {
        if let Some(path) = &self.file_path {
            let f = File::create(path).map_err(|e| e.to_string())?;
            self.content.write_to(f).map_err(|e| e.to_string())?;
            return Ok(());
        }
        Ok(())
    }
}

// --- SHELL ---
struct UniversalEditor {
    active_module: Option<Box<dyn EditorModule>>,
    sidebar_open: bool,
}

impl UniversalEditor {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_modern_style(&cc.egui_ctx);
        Self {
            active_module: None,
            sidebar_open: true,
        }
    }
}

impl eframe::App for UniversalEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- LAYOUT  ---
        // Top Bar 
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            egui::menu::bar(ui, |ui| {
                ui.style_mut().spacing.item_spacing.x = 15.0;
                ui.menu_button("File", |ui| {
                    if ui.button("New Text File").clicked() {
                        self.active_module = Some(Box::new(TextEditor::new_empty()));
                        ui.close_menu();
                    }
                    if ui.button("Open...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.active_module = Some(Box::new(TextEditor::load(path)));
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        if let Some(module) = &mut self.active_module {
                            let _ = module.save();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                
                ui.menu_button("Modules", |ui| {
                    ui.label("Image Editor (Coming Soon)");
                    ui.label("Hex Editor (Coming Soon)");
                });
            });
            ui.add_space(4.0);
        });

        // Sidebar
        if self.sidebar_open {
            egui::SidePanel::left("sidebar")
                .resizable(true)
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.heading("Workspace");
                    ui.separator();
                    ui.label("Recent Files:");
                    ui.label("📄 notes.txt");
                    ui.label("📄 main.rs");
                    
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        ui.label(format!("FPS: {:.1}", 1.0 / ctx.input(|i| i.unstable_dt)));
                    });
                });
        }

        // Central Canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(module) = &mut self.active_module {
                module.ui(ui);
            } else {
                render_homepage(ui);
            }
        });
    }
}

// --- HOMEPAGE / DASHBOARD ---
fn render_homepage(ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.heading("UNIVERSAL EDITOR");
            ui.label("The last editor you will ever need.");
            ui.add_space(20.0);
            
            if ui.add(egui::Button::new("  Create New Text  ").min_size(egui::vec2(200.0, 40.0))).clicked() {
                // Action would go here (requires passing state down)
            }
            ui.add_space(10.0);
            if ui.add(egui::Button::new("  Open File...  ").min_size(egui::vec2(200.0, 40.0))).clicked() {
                // Action
            }
        });
    });
}

// --- STYLING ---
fn configure_modern_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    style.visuals.window_rounding = egui::Rounding::same(10.0);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(6.0);
    
    style.visuals.dark_mode = true;
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 35);
    style.visuals.panel_fill = egui::Color32::from_rgb(20, 20, 25);
    
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(15.0, 8.0);
    
    ctx.set_style(style);
}

// --- ENTRY POINT ---
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Universal Editor"),
        ..Default::default()
    };
    eframe::run_native(
        "Universal Editor",
        options,
        Box::new(|cc| Box::new(UniversalEditor::new(cc))),
    )
}
