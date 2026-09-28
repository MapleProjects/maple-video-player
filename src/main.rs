mod app;
mod engine;

use app::MaplePlayerApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let initial_file = args.get(1).cloned();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Maple Video Player")
            .with_app_id("maple-video-player")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([640.0, 360.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Maple Video Player",
        options,
        Box::new(move |cc| {
            let mut app = MaplePlayerApp::new(cc).expect("Failed to initialize Maple Video Player");
            if let Some(file_path) = initial_file {
                app.load_file(&file_path);
            }
            Ok(Box::new(app))
        }),
    )
}
