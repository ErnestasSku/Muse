use crate::{
    canvas_state_sync::{
        p2p,
        sync_types::{MessageType, SyncableState},
    },
    custom_widgets::{
        canvas_image::{canvas_image, CanvasImageData},
        toggle::toggle,
    },
};
use anyhow::{Ok, Result};
use eframe::egui::{self, include_image, Grid, SidePanel, TopBottomPanel, Widget};
use egui::emath::TSTransform;
use std::{
    io::Read,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use tokio::sync::mpsc;

#[derive(Default)]
pub struct App {
    pub transform: TSTransform,
    pub images: Vec<CanvasImageData>,
    pub dropped_bytes: Vec<Vec<u8>>,
    pub file_loader_channel: Option<std::sync::mpsc::Receiver<Vec<u8>>>,

    // p2p communication fields
    pub p2p_receiver: Option<mpsc::Receiver<MessageType>>,
    pub gui_sender: Option<mpsc::Sender<MessageType>>,
    pub p2p_running: Arc<AtomicBool>,
    pub p2p_thread_handle: Option<std::thread::JoinHandle<()>>,

    // Panel
    pub show_menu_panel: bool,
    pub menu_p2p_enabled: bool,
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

        // TODO: improve this code.
        // read_file_bytes does not cover failing case.
        // If a file is not image, weird things might happen, since non images are also not handled.
        ctx.input(|i| {
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
                match message {
                    MessageType::NewImage { bytes } => {
                        self.dropped_bytes.push(bytes);
                    }
                    MessageType::CanvasState { state } => {
                        self.dropped_bytes = state.dropped_bytes;
                    }
                }
            }
        }
    }

    pub fn start_network_sync(&mut self) {
        if self.p2p_running.load(Ordering::Relaxed) {
            return;
        }

        self.p2p_running.store(true, Ordering::Relaxed);

        let (gui_sender, gui_receiver) = mpsc::channel::<MessageType>(1);
        let (p2p_sender, p2p_receiver) = mpsc::channel::<MessageType>(1);
        let p2p_running = Arc::clone(&self.p2p_running);

        self.p2p_receiver = Some(p2p_receiver);
        self.gui_sender = Some(gui_sender);

        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(p2p::p2p(gui_receiver, p2p_sender, p2p_running));
        });

        self.p2p_thread_handle = Some(handle);
    }

    pub fn stop_network_sync(&mut self) {
        if !self.p2p_running.load(Ordering::Relaxed) {
            return;
        }

        self.p2p_running.store(false, Ordering::Relaxed);

        self.p2p_receiver = None;
        self.gui_sender = None;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.menu_p2p_enabled {
            self.start_network_sync();
        } else {
            self.stop_network_sync();
        }

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Menu").clicked() {
                    self.show_menu_panel = !self.show_menu_panel;
                }
            })
        });

        if self.show_menu_panel {
            SidePanel::left("menu_panel").show(ctx, |ui| {
                ui.heading("Menu");
                ui.separator();

                Grid::new("menu_grid").show(ui, |ui| {
                    ui.label("P2P server");
                    ui.add(toggle(&mut self.menu_p2p_enabled));
                    ui.end_row();

                    // TODO: P2P status here
                    // ui.label("P2P status");
                    // ui.painter().circle(
                        // (0.0, 0.0).into(),
                        // 20.0,
                        // if self.p2p_running.load(Ordering::Relaxed) {
                            // egui::Color32::GREEN
                        // } else {
                            // egui::Color32::RED
                        // },
                        // egui::Stroke::new(1.0, egui::Color32::BLACK),
                    // );
                    // ui.end_row();

                    ui.label("Manual state sync");
                    if ui.add(egui::Button::new("Send state")).clicked() {
                        self.send_state();
                    }
                    ui.end_row();
                })
            });
        }

        // CANVAS
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.min_rect();
            let window_layer = ui.layer_id();
            self.manage_canvas_movement(ui);

            for image in self.images.iter_mut() {
                let i = canvas_image(image);
                ui.add(i);
            }

            for (count, bytes) in self.dropped_bytes.iter().enumerate() {
                let uri = format!("bytes://image_{}", count);
                let e_bytes = egui::load::Bytes::from(bytes.clone());
                let widget = egui::Image::from_bytes(uri, e_bytes);

                self.add_floating_widget(ui, rect, window_layer, widget, count);
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
