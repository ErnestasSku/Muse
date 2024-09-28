use canvas_image::{canvas_image, CanvasImageData};
use eframe::egui::{self};
use egui::emath::TSTransform;

mod canvas_image;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
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
            Ok(Box::new(app))
        }),
    )
}

#[derive(Default)]
struct App {
    transform: TSTransform,
    images: Vec<CanvasImageData>,
    dropped_images: Vec<String>,
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
            // images: vec![CanvasImageData::new()],
            ..Default::default()
        }
    }

    fn manage_canvas_movement(&mut self, ui: &egui::Ui) {
        // let a = UiBuilder
        let response = ui.interact_bg(egui::Sense::click_and_drag());
        // let response = ui.response();
        // let b = UiBuilder::new();

        // b.
        // let response = UiBuilder::sense(b, egui::Sense::click_and_drag());

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

    #[allow(dead_code)]
    fn test_button(&self, ui: &egui::Ui, rect: egui::Rect, parent_window: egui::LayerId) {
        let id = egui::Area::new(egui::Id::from("toggle"))
            .default_pos(egui::pos2(0.0, 0.0))
            .order(egui::Order::Middle)
            .constrain(false)
            .show(ui.ctx(), |ui| {
                ui.set_clip_rect(self.transform.inverse() * rect);
                ui.add(toggle(&mut false));
            })
            .response
            .layer_id;

        ui.ctx().set_transform_layer(id, self.transform);
        ui.ctx().set_sublayer(parent_window, id);
    }

    fn ui_file_drag_and_drop(&mut self, ctx: &egui::Context) {
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

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(file_path) = &file.path {
                        if ["jpg", "svg", "gif"]
                            .iter()
                            .any(|f| file_path.extension().unwrap().to_str().unwrap().eq(*f))
                        {
                            self.dropped_images.push(
                                String::from("file://")
                                    + &file_path.clone().into_os_string().into_string().unwrap(),
                            );
                        }
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

            // TEST button
            let window_layer = ui.layer_id();
            self.test_button(ui, rect, window_layer);
            self.test_image(ui, rect, window_layer);

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

            ui.input(|i| {
                for event in i.events.iter() {
                    if let egui::Event::Paste(string) = event {
                        println!("{}", string);
                    }
                }
            })
        });

        self.ui_file_drag_and_drop(ctx);
    }
}

// NOT NEEDED

fn toggle_ui_compact(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter()
            .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

pub fn toggle(on: &mut bool) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui_compact(ui, on)
}
