mod app;
mod image_loader;

use app::ImageViewer;

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
