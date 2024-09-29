use std::{io::Read, path::PathBuf, sync::mpsc, thread};

use anyhow::{Ok, Result};
use canvas_image::{canvas_image, CanvasImageData};
use eframe::egui::{self, Widget};
use egui::emath::TSTransform;
use tracing_subscriber;

mod canvas_image;

fn main() -> eframe::Result {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    let app = App::new();
    eframe::run_native(
        "Muse",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            eframe::Result::Ok(Box::new(app))
        }),
    )
}

#[derive(Default)]
struct App {
    transform: TSTransform,
    images: Vec<CanvasImageData>,
    dropped_images: Vec<String>,
    dropped_bytes: Vec<Vec<u8>>,
    file_loader_channel: Option<mpsc::Receiver<Vec<u8>>>,
}

#[allow(dead_code)]
struct ImageData {
    position: egui::Pos2,
    size: egui::Vec2,
}

impl App {
    fn new() -> Self {
        Self {
            transform: TSTransform::default(),
            images: vec![CanvasImageData::new()],
            ..Default::default()
        }
    }

    fn manage_canvas_movement(&mut self, ui: &egui::Ui) {
        // TODO: interact_bg is deprecated.
        // TO get response, the UI needs to be built with UI Builder.
        // Central panel does not accept UI builder.
        #[allow(deprecated)]
        let response = ui.interact_bg(egui::Sense::click_and_drag());

        if response.dragged() {
            self.transform.translation += response.drag_delta();
        }

        if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            if response.hovered() {
                let pointer_in_layer = self.transform.inverse() * pointer;
                let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
                let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);

                // Zoom in on pointer:
                self.transform = self.transform
                    * TSTransform::from_translation(pointer_in_layer.to_vec2())
                    * TSTransform::from_scaling(zoom_delta)
                    * TSTransform::from_translation(-pointer_in_layer.to_vec2());

                // Pan:
                self.transform = TSTransform::from_translation(pan_delta) * self.transform;
            }
        }
    }

    #[allow(dead_code)]
    fn test_image(&self, ui: &egui::Ui, rect: egui::Rect, parent_window: egui::LayerId) {
        let id = egui::Area::new(egui::Id::from("image"))
            .default_pos(egui::pos2(50.0, 50.0))
            .order(egui::Order::Middle)
            .constrain(false)
            .show(ui.ctx(), |ui| {
                ui.set_clip_rect(self.transform.inverse() * rect);

                ui.image(egui::include_image!("./ferris.gif"));
            })
            .response
            .layer_id;

        ui.ctx().set_transform_layer(id, self.transform);
        ui.ctx().set_sublayer(parent_window, id);
    }

    fn add_floating_widget(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
        parent_window: egui::LayerId,
        widget: impl Widget,
        count: usize,
    ) {
        use egui::Id;
        let id = egui::Area::new(Id::new("floating_image").with(count))
            // .default_pos() // TODO: figure out position later. Also WINIT does not send pointer move events when draging files.
            .order(egui::Order::Middle)
            .constrain(false)
            .show(ui.ctx(), |ui| {
                ui.set_clip_rect(self.transform.inverse() * rect);

                ui.add(widget);
            })
            .response
            .layer_id;

        ui.ctx().set_transform_layer(id, self.transform);
        ui.ctx().set_sublayer(parent_window, id);
    }

    fn ui_file_drag_and_drop(&mut self, ctx: &egui::Context) {
        // TODO:
        // WINIT does not support dragging non files. Like images/text from things like browsers into the window.
        use egui::{Color32, Id, LayerId, Order, TextStyle};
        use std::fmt::Write as _;

        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let text = ctx.input(|i| {
                let mut text = "Dropping files:\n".to_owned();
                for file in &i.raw.hovered_files {
                    if let Some(path) = &file.path {
                        write!(text, "\n{}", path.display()).ok();
                    } else if !file.mime.is_empty() {
                        write!(text, "\n{}", file.mime).ok();
                    } else {
                        text += "\n???";
                    }
                }
                text
            });

            // Instead of this, we will paint image preview
            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));
            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }

        if let Some(receiver) = &self.file_loader_channel {
            if let anyhow::Result::Ok(bytes) = receiver.try_recv() {
                self.dropped_bytes.push(bytes);
                self.file_loader_channel = None;
            }
        }

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        let (sender, receiver) = mpsc::channel();
                        self.file_loader_channel = Some(receiver);

                        let path_clone = path.clone();
                        thread::spawn(move || {
                            if let anyhow::Result::Ok(bytes) = read_file_bytes(&path_clone) {
                                sender.send(bytes).unwrap();
                            } else {
                                // TODO: failing cases.
                            }
                        });
                    }
                }
            }
        })
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // CANVAS
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.min_rect();
            self.manage_canvas_movement(ui);

            let window_layer = ui.layer_id();
            // self.test_image(ui, rect, window_layer);

            for image in self.images.iter_mut() {
                {
                    // image.x += 1.0;
                    // image.y += 1.0;
                }

                let i = canvas_image(image);
                ui.add(i);
            }

            for image in self.dropped_images.iter() {
                let image = egui::Image::new(image);
                ui.add(image);
            }

            for (count, bytes) in self.dropped_bytes.iter().enumerate() {
                let uri = format!("bytes://image_{}", count);
                let e_bytes = egui::load::Bytes::from(bytes.clone());
                let widget = egui::Image::from_bytes(uri, e_bytes);

                self.add_floating_widget(ui, rect, window_layer, widget, count);
            }
        });

        self.ui_file_drag_and_drop(ctx);
    }
}

fn read_file_bytes(file_path: &PathBuf) -> Result<Vec<u8>> {
    let file = std::fs::File::open(file_path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    Ok(buffer)
}
