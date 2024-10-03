use std::{sync::mpsc, thread};

use eframe::egui::{self};

use tracing_subscriber;

mod canvas_app;
mod canvas_image;
mod p2p;

#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result {
    // P2P
    let _p2p_thread = thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(p2p::p2p());
    });

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
