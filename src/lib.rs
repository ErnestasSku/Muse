#[cfg(target_os = "android")]
mod canvas_app;
#[cfg(target_os = "android")]
mod canvas_image;


use eframe::{egui, NativeOptions};

#[cfg(target_os = "android")]
use egui_winit::winit;
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use eframe::Renderer;
    use winit::platform::android::EventLoopBuilderExtAndroid;

    std::env::set_var("RUST_BACKTRACE", "full");
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let options = NativeOptions {
        event_loop_builder: Some(Box::new(|builder| {
            builder.with_android_app(app);
        })),
        renderer: Renderer::Wgpu,
        ..Default::default()
    };

    let app = canvas_app::App::new();
    eframe::run_native(
        "Muse-android",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            eframe::Result::Ok(Box::new(app))
        }),
    );
}
