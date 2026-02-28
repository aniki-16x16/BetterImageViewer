#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod animation;
mod app;
mod config;
mod image_loader;
mod thumbnail_list;
mod view_state;

use app::ImageViewer;
use config::AppConfig;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let config = AppConfig::load();

    // Parse command line arguments to get the initial image path
    let args: Vec<String> = std::env::args().collect();
    let initial_path = if args.len() > 1 {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    };

    let mut viewport = eframe::egui::ViewportBuilder::default().with_drag_and_drop(true);

    if let Some(size) = config.window_size {
        viewport = viewport.with_inner_size(size);
    } else {
        viewport = viewport.with_inner_size([800.0, 600.0]);
    }

    if let Some(pos) = config.window_pos {
        viewport = viewport.with_position(pos);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Better Image Viewer",
        options,
        Box::new(|cc| Ok(Box::new(ImageViewer::new(cc, config, initial_path)))),
    )
}
