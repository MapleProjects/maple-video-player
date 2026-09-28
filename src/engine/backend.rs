#![allow(dead_code)]
use super::config::EnhancementConfig;
use super::shaders::ShaderManager;
use super::vulkan_runner::VulkanRunner;
use std::process::Child;

pub struct MpvBackend {
    pub current_config: EnhancementConfig,
    pub shader_mgr: ShaderManager,
    pub is_file_loaded: bool,
    pub current_file_path: Option<String>,
    pub vulkan_process: Option<Child>,
}

impl MpvBackend {
    pub fn new() -> Result<Self, String> {
        let shader_mgr = ShaderManager::new().map_err(|e| format!("Shader manager init error: {e}"))?;
        let current_config = EnhancementConfig::load_from_disk();

        Ok(Self {
            current_config,
            shader_mgr,
            is_file_loaded: false,
            current_file_path: None,
            vulkan_process: None,
        })
    }

    pub fn apply_config(&mut self, config: &EnhancementConfig) -> Result<(), String> {
        self.current_config = config.clone();
        self.current_config.sync_lsfg_conf();
        Ok(())
    }

    pub fn load_file(&mut self, path: &str) -> Result<(), String> {
        self.current_file_path = Some(path.to_string());
        self.is_file_loaded = true;
        self.play_vulkan(None)
    }

    pub fn play_vulkan(&mut self, start_time: Option<f64>) -> Result<(), String> {
        self.stop();
        let target_path = self.current_file_path.as_deref();
        let child = VulkanRunner::launch(target_path, start_time, &self.current_config, &self.shader_mgr)
            .map_err(|e| format!("Failed to spawn Vulkan engine: {e}"))?;
        self.vulkan_process = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.vulkan_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn is_vulkan_running(&mut self) -> bool {
        if let Some(child) = &mut self.vulkan_process {
            match child.try_wait() {
                Ok(None) => true,
                _ => {
                    self.vulkan_process = None;
                    false
                }
            }
        } else {
            false
        }
    }
}

impl Drop for MpvBackend {
    fn drop(&mut self) {
        self.stop();
    }
}
