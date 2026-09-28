mod app;
mod engine;

use app::MaplePlayerApp;
use eframe::egui;
use engine::config::EnhancementConfig;
use engine::shaders::ShaderManager;
use engine::vulkan_runner::VulkanRunner;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let mut is_vulkan = false;
    let mut initial_file = None;

    for arg in args.iter().skip(1) {
        if arg == "--vulkan" || arg == "-v" {
            is_vulkan = true;
        } else if !arg.starts_with('-') && initial_file.is_none() {
            initial_file = Some(arg.clone());
        }
    }

    if is_vulkan {
        if let Some(ref file_path) = initial_file {
            let config = EnhancementConfig::load_from_disk();
            if let Ok(shader_mgr) = ShaderManager::new() {
                if let Ok(mut child) = VulkanRunner::launch(file_path, None, &config, &shader_mgr) {
                    let _ = child.wait();
                    return Ok(());
                }
            }
        }
    }

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
