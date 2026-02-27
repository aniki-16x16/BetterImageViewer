use eframe::egui;
use std::path::PathBuf;

use crate::config::AppConfig;
use crate::image_loader::{ImageCommand, ImageLoader, ImageResult};
use crate::view_state::ViewState;

pub struct ImageViewer {
    // Communication
    loader: ImageLoader,

    // Image State
    texture: Option<egui::TextureHandle>,
    // current_path: Option<PathBuf>, // Currently unused
    error_msg: Option<String>,
    is_loading: bool,

    // View State
    view_state: ViewState,

    // Debug info
    last_loaded_path: Option<String>,
    image_size: Option<[usize; 2]>,

    // Folder State
    current_folder_images: Vec<PathBuf>,
    current_image_index: usize,

    // Config
    config: AppConfig,
}

impl ImageViewer {
    pub fn new(cc: &eframe::CreationContext<'_>, config: AppConfig) -> Self {
        Self {
            loader: ImageLoader::new(cc.egui_ctx.clone()),
            texture: None,
            // current_path: None,
            error_msg: None,
            is_loading: false,
            view_state: ViewState::default(),
            last_loaded_path: None,
            image_size: None,
            current_folder_images: Vec::new(),
            current_image_index: 0,
            config,
        }
    }

    fn load_file(&mut self, path: PathBuf) {
        println!("UI: Requesting load for {:?}", path);
        self.is_loading = true;
        self.error_msg = None;
        self.loader.tx.send(ImageCommand::Load(path)).unwrap();
    }

    fn load_path(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.load_folder_contents(&path);
            if self.current_folder_images.is_empty() {
                self.error_msg = Some("No images found in the folder.".to_string());
                self.texture = None;
            } else {
                self.current_image_index = 0;
                self.load_file(self.current_folder_images[0].clone());
            }
        } else {
            if let Some(parent) = path.parent() {
                self.load_folder_contents(parent);
                if let Some(idx) = self.current_folder_images.iter().position(|p| p == &path) {
                    self.current_image_index = idx;
                } else {
                    self.current_folder_images = vec![path.clone()];
                    self.current_image_index = 0;
                }
            } else {
                self.current_folder_images = vec![path.clone()];
                self.current_image_index = 0;
            }
            self.load_file(path);
        }
    }

    fn load_folder_contents(&mut self, folder_path: &std::path::Path) {
        let mut images = Vec::new();
        if let Ok(entries) = std::fs::read_dir(folder_path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        let ext = ext.to_lowercase();
                        if matches!(
                            ext.as_str(),
                            "jpg"
                                | "jpeg"
                                | "png"
                                | "gif"
                                | "webp"
                                | "bmp"
                                | "ico"
                                | "tiff"
                                | "avif"
                        ) {
                            images.push(p);
                        }
                    }
                }
            }
        }
        // Sort alphabetically
        images.sort();
        self.current_folder_images = images;
    }

    fn next_image(&mut self) {
        if self.current_folder_images.is_empty() {
            return;
        }
        self.current_image_index =
            (self.current_image_index + 1) % self.current_folder_images.len();
        self.load_file(self.current_folder_images[self.current_image_index].clone());
    }

    fn prev_image(&mut self) {
        if self.current_folder_images.is_empty() {
            return;
        }
        if self.current_image_index == 0 {
            self.current_image_index = self.current_folder_images.len() - 1;
        } else {
            self.current_image_index -= 1;
        }
        self.load_file(self.current_folder_images[self.current_image_index].clone());
    }
}

impl eframe::App for ImageViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Handle Async Results
        while let Ok(result) = self.loader.rx.try_recv() {
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
                    self.view_state.reset();

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
                    self.load_path(path.clone());
                }
            }
        }

        // Handle Keyboard Navigation
        if !self.current_folder_images.is_empty() && !self.is_loading {
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                self.next_image();
            } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                self.prev_image();
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
                self.view_state.process_input(ui);

                // 5. Drawing
                let center_pos = ui.clip_rect().center() + self.view_state.pan;
                let final_size = texture_size * self.view_state.zoom;
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
                        self.view_state.zoom,
                        self.view_state.pan.x,
                        self.view_state.pan.y,
                        texture_size.x,
                        texture_size.y
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
                    if ui.button("Open Image or Folder").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.load_path(path);
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
                        ui.colored_label(text_color, "Drag & Drop an image or folder here");
                    });
            }
        });

        // Save window state periodically or on close
        let window_info = ctx.input(|i| i.viewport().clone());
        let mut changed = false;

        if let Some(pos) = window_info.inner_rect.map(|r| r.min) {
            let new_pos = [pos.x, pos.y];
            if self.config.window_pos != Some(new_pos) {
                self.config.window_pos = Some(new_pos);
                changed = true;
            }
        }

        if let Some(size) = window_info.inner_rect.map(|r| r.size()) {
            let new_size = [size.x, size.y];
            if self.config.window_size != Some(new_size) {
                self.config.window_size = Some(new_size);
                changed = true;
            }
        }

        // In a real app, you might want to debounce this save operation
        // or only save on exit. For simplicity, we save when it changes.
        if changed {
            self.config.save();
        }
    }
}
