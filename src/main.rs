mod engine;

use engine::config::EnhancementConfig;
use engine::shaders::ShaderManager;
use engine::vulkan_runner::VulkanRunner;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let mut initial_file: Option<String> = None;

    for arg in args.iter().skip(1) {
        if !arg.starts_with('-') && initial_file.is_none() {
            initial_file = Some(arg.clone());
        }
    }

    let config = EnhancementConfig::load_from_disk();
    let shader_mgr = ShaderManager::new()?;

    // Sync Lossless Scaling LSFG profile
    config.sync_lsfg_conf();

    let target_file = initial_file.as_deref();
    let mut child = VulkanRunner::launch(target_file, None, &config, &shader_mgr)?;

    let status = child.wait()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
