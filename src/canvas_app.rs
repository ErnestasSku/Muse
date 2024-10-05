use crate::{
    canvas_image::{canvas_image, CanvasImageData},
    communication::{MessageType, SyncableState},
};
use anyhow::{Ok, Result};
use eframe::egui::{self, include_image, Widget};
use egui::emath::TSTransform;
use std::{io::Read, path::PathBuf, thread};

use tokio::sync::mpsc;
// use std::sync::mpsc;

#[derive(Default)]
pub struct App {
    pub transform: TSTransform,
    pub images: Vec<CanvasImageData>,
    pub dropped_bytes: Vec<Vec<u8>>,
    pub file_loader_channel: Option<std::sync::mpsc::Receiver<Vec<u8>>>,
    pub p2p_receiver: Option<mpsc::Receiver<MessageType>>,
    pub gui_sender: Option<mpsc::Sender<MessageType>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            transform: TSTransform::default(),
            images: vec![],
            ..Default::default()
        }
    }

    pub fn manage_canvas_movement(&mut self, ui: &egui::Ui) {
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

    pub fn add_floating_widget(
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

    pub fn ui_file_drag_and_drop(&mut self, ctx: &egui::Context) {
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
                    }
                }
                text
            });

            // Instead of this, we will paint image preview
            // once again, this is not possible now due to winit limitation
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
                        let (sender, receiver) = std::sync::mpsc::channel();
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

    pub fn send_state(&self) {
        if let Some(sender) = &self.gui_sender {
            let _a = sender
                .try_send(MessageType::CanvasState {
                    state: SyncableState::from(self),
                })
                .map_err(|err| println!("{:?}", err));
        }
    }

    pub fn handle_p2p_messages(&mut self) {
        if let Some(ref mut p2p) = self.p2p_receiver {
            if let anyhow::Result::Ok(message) = p2p.try_recv() {
                println!("Houston, UI has the message");
                match message {
                    MessageType::NewImage { bytes } => todo!(),
                    MessageType::CanvasState { state } => {
                        println!("overwriting state");
                        self.dropped_bytes = state.dropped_bytes.clone();
                    },
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // CANVAS
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.min_rect();
            let window_layer = ui.layer_id();
            self.manage_canvas_movement(ui);

            // let img = egui::Image::new(include_image!("./ferris.gif"));
            // self.add_floating_widget(ui, rect, window_layer, img, 150);

            for image in self.images.iter_mut() {
                {
                    // image.x += 1.0;
                    // image.y += 1.0;
                }

                let i = canvas_image(image);
                ui.add(i);
            }

            for (count, bytes) in self.dropped_bytes.iter().enumerate() {
                let uri = format!("bytes://image_{}", count);
                let e_bytes = egui::load::Bytes::from(bytes.clone());
                let widget = egui::Image::from_bytes(uri, e_bytes);

                self.add_floating_widget(ui, rect, window_layer, widget, count);
            }

            if ui.add(egui::Button::new("Send state")).clicked() {
                println!("clicked");
                self.send_state();
            }
        });

        self.ui_file_drag_and_drop(ctx);
        self.handle_p2p_messages();
    }
}

pub fn read_file_bytes(file_path: &PathBuf) -> Result<Vec<u8>> {
    let file = std::fs::File::open(file_path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    Ok(buffer)
}
