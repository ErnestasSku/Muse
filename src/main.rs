use std::{io::Read, path::PathBuf, sync::mpsc, thread};

use anyhow::{Ok, Result};
use canvas_image::{canvas_image, CanvasImageData};
use eframe::egui::{self, Widget};
use egui::emath::TSTransform;
use tracing_subscriber;

mod canvas_image;
mod canvas_app;

#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result {
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
