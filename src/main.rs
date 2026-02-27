use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

// --- Data Types ---

enum ImageCommand {
    Load(PathBuf),
}

enum ImageResult {
    Success(PathBuf, egui::ColorImage),
    Error(String),
}

struct ImageViewer {
    // Communication
    tx: Sender<ImageCommand>,
    rx: Receiver<ImageResult>,

    // Image State
    texture: Option<egui::TextureHandle>,
    // current_path: Option<PathBuf>, // Currently unused
    error_msg: Option<String>,
    is_loading: bool,

    // View State
    zoom: f32,
    pan: egui::Vec2,

    // Debug info
    last_loaded_path: Option<String>,
    image_size: Option<[usize; 2]>,
}

impl ImageViewer {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx_ui, rx_worker) = channel::<ImageCommand>();
        let (tx_worker, rx_ui) = channel::<ImageResult>();

        // Repaint signal for the background thread
        let ctx = cc.egui_ctx.clone();

        // Background Loader Thread
        thread::spawn(move || {
            while let Ok(cmd) = rx_worker.recv() {
                match cmd {
                    ImageCommand::Load(path) => {
                        println!("Thread: Start loading {:?}", path);
                        // Using explicit Reader for better error context if needed, but open is fine
                        match image::open(&path) {
                            Ok(dynamic_image) => {
                                let width = dynamic_image.width() as usize;
                                let height = dynamic_image.height() as usize;
                                println!("Thread: Image decoded {}x{}", width, height);

                                // Convert to rgba8 for egui
                                let image_buffer = dynamic_image.to_rgba8();
                                // IMPORTANT: Use into_raw() or as_raw() to get robust pixel data
                                let pixels = image_buffer.into_raw();
                                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                    [width, height],
                                    &pixels,
                                );

                                // Send back
                                if let Err(e) =
                                    tx_worker.send(ImageResult::Success(path.clone(), color_image))
                                {
                                    println!("Thread: Failed to send Success result: {}", e);
                                } else {
                                    println!("Thread: Sent Success result");
                                }
                            }
                            Err(err) => {
                                println!("Thread: Error decoding image: {}", err);
                                let _ = tx_worker
                                    .send(ImageResult::Error(format!("Load error: {}", err)));
                            }
                        }
                        // Request repaint to update UI
                        ctx.request_repaint();
                    }
                }
            }
        });

        Self {
            tx: tx_ui,
            rx: rx_ui,
            texture: None,
            // current_path: None,
            error_msg: None,
            is_loading: false,
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            last_loaded_path: None,
            image_size: None,
        }
    }

    fn load_file(&mut self, path: PathBuf) {
        println!("UI: Requesting load for {:?}", path);
        self.is_loading = true;
        self.error_msg = None;
        self.tx.send(ImageCommand::Load(path)).unwrap();
    }
}

impl eframe::App for ImageViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Handle Async Results
        while let Ok(result) = self.rx.try_recv() {
            println!("UI: Received result");
            self.is_loading = false;
            match result {
                ImageResult::Success(path, image) => {
                    println!("UI: Creating texture");
                    self.last_loaded_path = Some(path.to_string_lossy().to_string());
                    self.image_size = Some(image.size);
                    self.texture =
                        Some(ctx.load_texture("image", image, egui::TextureOptions::LINEAR));
                    // Reset view
                    self.zoom = 1.0;
                    self.pan = egui::Vec2::ZERO;

                    // Auto-fit logic could go here
                }
                ImageResult::Error(err) => {
                    println!("UI: Received Error: {}", err);
                    self.error_msg = Some(err);
                }
            }
        }

        // 2. Handle File Drops
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
            let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
            if let Some(file) = dropped_files.first() {
                // Check if the path is provided (it might not be on web, but this is native)
                if let Some(path) = &file.path {
                    self.load_file(path.clone());
                }
            }
        }

        // 3. UI Layout
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.is_loading {
                ui.centered_and_justified(|ui| ui.spinner());
                // IMPORTANT: Do NOT return here if you want debug overlays or other persistent UI
                // But generally for a modal loading screen, returning is fine,
                // provided we are sure is_loading will flip back.
                return;
            }

            if let Some(err) = &self.error_msg {
                ui.centered_and_justified(|ui| ui.heading(format!("Error: {}", err)));
                return;
            }

            if let Some(texture) = &self.texture {
                let texture_size = texture.size_vec2();
                // let available_size = ui.available_size(); // unused

                // 4. Zoom & Pan Logic
                let (scroll, delta) = ui.input(|i| (i.smooth_scroll_delta, i.pointer.delta()));

                // Handle Zoom
                let zoom_factor = if scroll.y != 0.0 {
                    1.0 + (scroll.y * 0.001)
                } else {
                    1.0
                };

                // Apply Zoom centered on mouse
                if zoom_factor != 1.0 {
                    let pointer_pos = ui
                        .input(|i| i.pointer.hover_pos())
                        .unwrap_or(ui.clip_rect().center());

                    let old_zoom = self.zoom;
                    self.zoom *= zoom_factor;
                    // Clamp zoom
                    self.zoom = self.zoom.max(0.01).min(500.0);

                    // Math:
                    // RelM = M - Center
                    // Pan_new = RelM - (RelM - Pan_old) * (Zoom_new / Zoom_old)

                    let center_screen = ui.clip_rect().center().to_vec2();
                    let rel_m = pointer_pos.to_vec2() - center_screen;

                    self.pan = rel_m - (rel_m - self.pan) * (self.zoom / old_zoom);
                }

                // Apply Pan (Mouse Drag)
                let is_dragging = ui.input(|i| {
                    i.pointer.button_down(egui::PointerButton::Primary)
                        || i.pointer.button_down(egui::PointerButton::Middle)
                });
                if is_dragging {
                    self.pan += delta;
                }

                // 5. Drawing
                let center_pos = ui.clip_rect().center() + self.pan;
                let final_size = texture_size * self.zoom;
                let image_rect = egui::Rect::from_center_size(center_pos, final_size);

                ui.painter().image(
                    texture.id(),
                    image_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );

                // Debug overlay
                ui.scope(|ui| {
                    // Make it semi-transparent black
                    // Accessing visuals directly is tricky without creating a proper Frame
                    // Let's just use a window frame for simplicity or a simple label
                    // To make it float on top, we draw it *after* image

                    let debug_text = format!(
                        "Zoom: {:.2}x\nPan: {:.0}, {:.0}\nSize: {}x{}",
                        self.zoom, self.pan.x, self.pan.y, texture_size.x, texture_size.y
                    );

                    ui.painter().text(
                        ui.clip_rect().min + egui::vec2(10.0, 10.0),
                        egui::Align2::LEFT_TOP,
                        debug_text,
                        egui::FontId::monospace(14.0),
                        egui::Color32::YELLOW,
                    );
                });
            } else {
                ui.centered_and_justified(|ui| {
                    if ui.button("Open Image").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.load_file(path);
                        }
                    }
                });

                // Show drop text
                let text_color = if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
                    egui::Color32::LIGHT_BLUE
                } else {
                    egui::Color32::GRAY
                };

                egui::Area::new(egui::Id::new("drop_text_area"))
                    .order(egui::Order::Background)
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 40.0))
                    .show(ctx, |ui| {
                        ui.colored_label(text_color, "Drag & Drop an image here");
                    });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Better Image Viewer",
        options,
        Box::new(|cc| Ok(Box::new(ImageViewer::new(cc)))),
    )
}
