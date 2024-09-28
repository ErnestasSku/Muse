use eframe::egui::{self, ImageSource};

#[derive(Default)]
pub struct CanvasImageData {
    pub position: Coordinates,
    // pub bytes:
    // pub a: (),
}

#[derive(Default)]
pub struct Coordinates {
    x: f32,
    y: f32,
}

impl CanvasImageData {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

fn canvas_image_component(ui: &mut egui::Ui, data: &mut CanvasImageData) -> egui::Response {
    // let img = egui::Image::new(a.as_bytes());
    let image = egui::Image::new(egui::include_image!("./ferris.gif"));
    // let image = image.fit_to_exact_size([data.x, data.y].into());

    let add = ui.add_sized([data.position.x, data.position.y], image);
    let rect = egui::Rect::from_min_size(add.rect.left_top(), add.rect.size());
    let painter = ui.painter();

    let _a = painter.rect_stroke(rect, 0.0, egui::Stroke::new(2.0, egui::Color32::BLUE));

    // OUTLINE test
    // painter.circle(
    //     [0.0, data.position.y / 2.0].into(),
    //     3.0,
    //     egui::Color32::RED,
    //     egui::Stroke::new(1.0, egui::Color32::GREEN),
    // );
    // painter.circle(
    //     [data.position.x / 2.0, 0.0].into(),
    //     3.0,
    //     egui::Color32::RED,
    //     egui::Stroke::new(1.0, egui::Color32::GREEN),
    // );
    // painter.circle(
    //     [data.position.x, data.position.y / 2.0].into(),
    //     3.0,
    //     egui::Color32::RED,
    //     egui::Stroke::new(1.0, egui::Color32::GREEN),
    // );
    // painter.circle(
    //     [data.position.x / 2.0, data.position.y].into(),
    //     3.0,
    //     egui::Color32::RED,
    //     egui::Stroke::new(1.0, egui::Color32::GREEN),
    // );

    add.clone().on_hover_cursor(egui::CursorIcon::Grab);

    add
}

pub fn canvas_image(data: &mut CanvasImageData) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| canvas_image_component(ui, data)
}
