use eframe::egui::{self};

use tracing_subscriber;

mod canvas_app;
mod custom_widgets;
mod canvas_state_sync;

#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result {
    // App
    tracing_subscriber::fmt::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    let app = canvas_app::App::new();

    eframe::run_native(
        "Muse",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            eframe::Result::Ok(Box::new(app))
        }),
    )
}
