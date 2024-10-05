use std::thread;

use tokio::sync::mpsc;
// use std::sync::mpsc;

use eframe::egui::{self};

use tracing_subscriber;

mod canvas_app;
mod canvas_image;
mod communication;
mod p2p;

#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result {
    // Channels

    use communication::MessageType;
    let (gui_sender, gui_receiver) = mpsc::channel::<MessageType>(1);
    let (p2p_sender, p2p_receiver) = mpsc::channel::<MessageType>(1);

    // P2P
    let _p2p_thread = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(p2p::p2p(gui_receiver, p2p_sender));
    });

    // App
    tracing_subscriber::fmt::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    let mut app = canvas_app::App::new();
    app.p2p_receiver = Some(p2p_receiver);
    app.gui_sender = Some(gui_sender);

    eframe::run_native(
        "Muse",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            eframe::Result::Ok(Box::new(app))
        }),
    )
}
