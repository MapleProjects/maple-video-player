use crate::engine::config::{EnhancementConfig, UpscaleMode};
use crate::engine::shaders::ShaderManager;
use std::process::{Child, Command};

pub struct VulkanRunner;

impl VulkanRunner {
    pub fn build_mpv_args(
        file_path: Option<&str>,
        start_time: Option<f64>,
        config: &EnhancementConfig,
        shader_mgr: &ShaderManager,
    ) -> Vec<String> {
        let mut args = Vec::new();

        // 1. Core Vulkan video output and Wayland context
        args.push("--vo=gpu-next".to_string());
        args.push("--gpu-api=vulkan".to_string());
        args.push("--gpu-context=waylandvk".to_string());
        args.push("--hwdec=nvdec-copy".to_string());
        args.push("--framedrop=no".to_string());
        args.push("--fbo-format=rgba16f".to_string());

        // 2. High dynamic range & 10-bit wide gamut colorspace tags
        args.push("--target-colorspace-hint=yes".to_string());
        args.push("--target-colorspace-hint-mode=source-dynamic".to_string());
        args.push("--target-prim=auto".to_string());
        args.push("--target-trc=auto".to_string());
        args.push("--hdr-compute-peak=yes".to_string());
        args.push("--vo-image-tag-colorspace=yes".to_string());
        args.push("--keep-open=yes".to_string());
        args.push("--title=🍁 Maple Video Player (Vulkan GPU-Next • LSFG Layer)".to_string());
        args.push("--input-ipc-server=/tmp/maple-player.sock".to_string());

        // 3. Debanding and Bit Depth
        if config.deband_enabled {
            args.push("--deband=yes".to_string());
            args.push(format!("--deband-iterations={}", config.deband_iterations));
            args.push(format!("--deband-threshold={}", config.deband_threshold));
            args.push(format!("--deband-range={}", config.deband_range));
            args.push(format!("--deband-grain={}", config.deband_grain));
        } else {
            args.push("--deband=no".to_string());
        }

        args.push(format!("--dither-depth={}", config.bit_depth.as_str()));
        args.push(format!("--dither={}", config.dither_algo.as_str()));
        args.push(format!("--temporal-dither={}", if config.temporal_dither { "yes" } else { "no" }));

        // 4. Shaders (Qualcomm GSR / FSR 1.0 / CAS / Anime4K / Deband)
        let mut shader_paths = Vec::new();
        match config.upscale_mode {
            UpscaleMode::Off => {}
            UpscaleMode::Fsr => {
                if let Ok(p) = shader_mgr.get_fsr_shader(
                    config.fsr_sharpness,
                    config.fsr_denoise,
                    config.fsr_edge_threshold,
                    config.fsr_deringing,
                ) {
                    shader_paths.push(p.to_string_lossy().to_string());
                }
            }
            UpscaleMode::SnapdragonGsr => {
                if let Ok(p) = shader_mgr.get_snapdragon_gsr_shader(
                    config.sgsr_edge_threshold,
                    config.sgsr_edge_sharpness,
                    config.sgsr_edge_direction,
                ) {
                    shader_paths.push(p.to_string_lossy().to_string());
                }
            }
            UpscaleMode::Cas => {
                if let Ok(p) = shader_mgr.get_cas_shader(config.cas_sharpness) {
                    shader_paths.push(p.to_string_lossy().to_string());
                }
            }
            UpscaleMode::Anime4k => {
                let p = shader_mgr.get_anime4k_shader();
                if p.exists() {
                    shader_paths.push(p.to_string_lossy().to_string());
                }
            }
        }

        if config.extra_deband_shader {
            let p = shader_mgr.get_deband_ext_path();
            if p.exists() {
                shader_paths.push(p.to_string_lossy().to_string());
            }
        }

        if !shader_paths.is_empty() {
            for p in shader_paths {
                args.push(format!("--glsl-shader={p}"));
            }
        }

        // 5. Internal render scale
        let scale_factor = config.render_scale.factor();
        if (scale_factor - 1.0).abs() > 0.01 {
            args.push(format!("--vf=scale=w=iw*{scale_factor:.2}:h=ih*{scale_factor:.2}"));
        }

        // 6. Resume position
        if let Some(t) = start_time {
            if t > 0.5 {
                args.push(format!("--start={t:.2}"));
            }
        }

        // 7. Load Maple Player Integrated On-Screen HUD script
        let hud_script = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/home/maple/.config"))
            .join("maple-player")
            .join("scripts")
            .join("maple_hud.lua");
        if hud_script.exists() {
            args.push(format!("--script={}", hud_script.to_string_lossy()));
        }

        // 8. Target file or idle window
        if let Some(path) = file_path {
            args.push(path.to_string());
        } else {
            args.push("--idle=yes".to_string());
            args.push("--force-window=yes".to_string());
        }

        args
    }

    pub fn launch(
        file_path: Option<&str>,
        start_time: Option<f64>,
        config: &EnhancementConfig,
        shader_mgr: &ShaderManager,
    ) -> Result<Child, std::io::Error> {
        // Sync LSFG conf.toml before launch
        config.sync_lsfg_conf();

        // Clean up any stale socket
        let _ = std::fs::remove_file("/tmp/maple-player.sock");

        let args = Self::build_mpv_args(file_path, start_time, config, shader_mgr);

        let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-1".to_string());
        let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/run/user/{}", libc_getuid()));

        Command::new("mpv")
            .args(&args)
            .env("WAYLAND_DISPLAY", wayland_display)
            .env("XDG_RUNTIME_DIR", xdg_runtime_dir)
            .env("ENABLE_LSFG", "1")
            .env("DISABLE_LSFGVK", "0")
            .env("ENABLE_HDR_WSI", "1")
            .env("DXVK_HDR", "1")
            .spawn()
    }
}

fn libc_getuid() -> u32 {
    unsafe { libc_getuid_sys() }
}

extern "C" {
    #[link_name = "getuid"]
    fn libc_getuid_sys() -> u32;
}
