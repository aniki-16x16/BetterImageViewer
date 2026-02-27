use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

pub enum ImageCommand {
    Load(PathBuf),
}

pub enum ImageResult {
    Success(PathBuf, egui::ColorImage),
    Error(PathBuf, String),
}

pub struct ImageLoader {
    pub tx: Sender<ImageCommand>,
    pub rx: Receiver<ImageResult>,
}

impl ImageLoader {
    pub fn new(ctx: egui::Context) -> Self {
        let (tx_ui, rx_worker) = channel::<ImageCommand>();
        let (tx_worker, rx_ui) = channel::<ImageResult>();

        // Background Loader Thread
        thread::spawn(move || {
            while let Ok(cmd) = rx_worker.recv() {
                match cmd {
                    ImageCommand::Load(path) => {
                        println!("Thread: Start loading {:?}", path);
                        match image::open(&path) {
                            Ok(dynamic_image) => {
                                let width = dynamic_image.width() as usize;
                                let height = dynamic_image.height() as usize;
                                println!("Thread: Image decoded {}x{}", width, height);

                                // Convert to rgba8 for egui
                                let image_buffer = dynamic_image.to_rgba8();
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
                                let _ = tx_worker.send(ImageResult::Error(
                                    path.clone(),
                                    format!("Load error: {}", err),
                                ));
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
        }
    }
}
