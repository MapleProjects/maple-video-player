use super::config::{EnhancementConfig, FrameGenMultiplier, UpscaleMode};
use super::gl_renderer::{FramebufferSize, GlRenderer};
use super::shaders::ShaderManager;
use eframe::glow;
use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType, mpv_render_update};
use libmpv2::Mpv;
use std::ffi::{c_void, CStr, CString};

#[derive(Clone, Copy)]
struct GlContextWrapper(*const (dyn Fn(&CStr) -> *const c_void + 'static));
unsafe impl Send for GlContextWrapper {}
unsafe impl Sync for GlContextWrapper {}

fn resolve_gl_proc(wrapper: &GlContextWrapper, name: &str) -> *mut c_void {
    if let Ok(cname) = CString::new(name) {
        unsafe {
            if let Some(f) = wrapper.0.as_ref() {
                return (f)(&cname) as *mut c_void;
            }
        }
    }
    std::ptr::null_mut()
}

pub struct MpvBackend {
    // Order is CRITICAL for Drop safety: render_ctx must drop before mpv
    render_ctx: RenderContext<'static>,
    mpv: Mpv,
    pub gl_renderer: GlRenderer,
    pub current_config: EnhancementConfig,
    pub shader_mgr: ShaderManager,
    pub is_file_loaded: bool,
    pub current_file_path: Option<String>,
}

impl MpvBackend {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        let gl = cc.gl.clone().ok_or("OpenGL glow context unavailable")?;
        let proc_addr = cc
            .get_proc_address
            .ok_or("OpenGL get_proc_address unavailable")?;

        let raw_ptr: *const dyn Fn(&CStr) -> *const c_void = proc_addr;
        let raw_static: *const (dyn Fn(&CStr) -> *const c_void + 'static) = unsafe { std::mem::transmute(raw_ptr) };
        let wrapper = GlContextWrapper(raw_static);

        let mpv = Mpv::new().map_err(|e| format!("Failed to initialize MPV: {e}"))?;

        // Base player options
        let _ = mpv.set_property("vo", "libmpv");
        let _ = mpv.set_property("gpu-api", "opengl");
        let _ = mpv.set_property("hwdec", "nvdec-copy,auto-safe");
        let _ = mpv.set_property("framedrop", "no");
        let _ = mpv.set_property("fbo-format", "rgba16f"); // High precision 16-bit float pipeline
        // Declare high-bit / wide color space to Wayland compositor to bypass standard SDR saturation clamping
        let _ = mpv.set_property("target-colorspace-hint", "yes");
        let _ = mpv.set_property("target-colorspace-hint-mode", "source-dynamic");
        let _ = mpv.set_property("target-prim", "auto");
        let _ = mpv.set_property("target-trc", "auto");
        let _ = mpv.set_property("vo-image-tag-colorspace", "yes");
        let _ = mpv.set_property("keep-open", "yes");
        let _ = mpv.set_property("tone-mapping", "auto");
        let _ = mpv.set_property("video-timing-offset", 0.0_f64);

        let render_ctx = mpv
            .create_render_context([
                RenderParam::ApiType(RenderParamApiType::OpenGl),
                RenderParam::InitParams(OpenGLInitParams {
                    get_proc_address: resolve_gl_proc,
                    ctx: wrapper,
                }),
            ])
            .map_err(|e| format!("Failed to create MPV render context: {e}"))?;

        // Erase lifetime for struct storage; dropped in safe declaration order
        let mut render_ctx: RenderContext<'static> = unsafe { std::mem::transmute(render_ctx) };

        let egui_ctx = cc.egui_ctx.clone();
        render_ctx.set_update_callback(move || {
            egui_ctx.request_repaint();
        });

        let gl_renderer = GlRenderer::new(gl);
        let shader_mgr = ShaderManager::new().map_err(|e| format!("Shader manager init error: {e}"))?;
        let current_config = EnhancementConfig::load_from_disk();

        let mut backend = Self {
            render_ctx,
            mpv,
            gl_renderer,
            current_config: current_config.clone(),
            shader_mgr,
            is_file_loaded: false,
            current_file_path: None,
        };

        backend.apply_config(&current_config)?;

        Ok(backend)
    }

    pub fn apply_config(&mut self, config: &EnhancementConfig) -> Result<(), String> {
        self.current_config = config.clone();

        // 1. Deband configuration
        let _ = self.mpv.set_property("deband", if config.deband_enabled { "yes" } else { "no" });
        let _ = self.mpv.set_property("deband-iterations", config.deband_iterations as i64);
        let _ = self.mpv.set_property("deband-threshold", config.deband_threshold as i64);
        let _ = self.mpv.set_property("deband-range", config.deband_range as i64);
        let _ = self.mpv.set_property("deband-grain", config.deband_grain as i64);

        // 2. Dither and Bit-Depth Extension
        let _ = self.mpv.set_property("dither-depth", config.bit_depth.as_str());
        let _ = self.mpv.set_property("dither", config.dither_algo.as_str());
        let _ = self.mpv.set_property("temporal-dither", if config.temporal_dither { "yes" } else { "no" });

        // 3. Super Resolution Scaling & Frame Generation filter chain
        let mut vf_filters = Vec::new();

        let scale_factor = config.render_scale.factor();
        if (scale_factor - 1.0).abs() > 0.01 {
            vf_filters.push(format!("scale=w=iw*{scale_factor:.2}:h=ih*{scale_factor:.2}"));
        }

        let vf_str = vf_filters.join(",");
        let _ = self.mpv.set_property("vf", &*vf_str);

        if config.interpolation_enabled {
            let container_fps = self.get_container_fps().unwrap_or(24.0).max(1.0);
            let target_fps = match config.frame_gen_mode {
                FrameGenMultiplier::Native => container_fps,
                FrameGenMultiplier::Double => container_fps * 2.0,
                FrameGenMultiplier::Cinematic2_5 => 60.0,
                FrameGenMultiplier::Triple => container_fps * 3.0,
                FrameGenMultiplier::VsyncMatch => 120.212,
                FrameGenMultiplier::Custom => config.custom_target_fps,
            };

            let _ = self.mpv.set_property("override-display-fps", target_fps);
            let _ = self.mpv.set_property("video-sync", "display-resample");
            let _ = self.mpv.set_property("interpolation", "yes");
            let _ = self.mpv.set_property("tscale", "oversample");
            let _ = self.mpv.set_property("tscale-clamp", 0.0_f64);
        } else {
            let _ = self.mpv.set_property("override-display-fps", 0.0_f64);
            let _ = self.mpv.set_property("video-sync", "audio");
            let _ = self.mpv.set_property("interpolation", "no");
        }

        // 4. Shaders (FSR 1.0 / Snapdragon GSR / CAS / Anime4K / Deband Extension)
        let mut shader_paths = Vec::new();

        match config.upscale_mode {
            UpscaleMode::Off => {}
            UpscaleMode::Fsr => {
                if let Ok(p) = self.shader_mgr.get_fsr_shader(
                    config.fsr_sharpness,
                    config.fsr_denoise,
                    config.fsr_edge_threshold,
                    config.fsr_deringing,
                ) {
                    shader_paths.push(p.to_string_lossy().to_string());
                }
            }
            UpscaleMode::SnapdragonGsr => {
                if let Ok(p) = self.shader_mgr.get_snapdragon_gsr_shader(
                    config.sgsr_edge_threshold,
                    config.sgsr_edge_sharpness,
                    config.sgsr_edge_direction,
                ) {
                    shader_paths.push(p.to_string_lossy().to_string());
                }
            }
            UpscaleMode::Cas => {
                if let Ok(p) = self.shader_mgr.get_cas_shader(config.cas_sharpness) {
                    shader_paths.push(p.to_string_lossy().to_string());
                }
            }
            UpscaleMode::Anime4k => {
                let p = self.shader_mgr.get_anime4k_shader();
                if p.exists() {
                    shader_paths.push(p.to_string_lossy().to_string());
                }
            }
        }

        if config.extra_deband_shader {
            let p = self.shader_mgr.get_deband_ext_path();
            if p.exists() {
                shader_paths.push(p.to_string_lossy().to_string());
            }
        }

        let _ = self.mpv.command("change-list", &["glsl-shaders", "clr", ""]);
        if !shader_paths.is_empty() {
            for p in &shader_paths {
                let _ = self.mpv.command("change-list", &["glsl-shaders", "append", p]);
            }
            let shader_str = shader_paths.join(":");
            let _ = self.mpv.set_property("glsl-shaders", &*shader_str);
        } else {
            let _ = self.mpv.set_property("glsl-shaders", "");
        }

        // 5. Hardware decoding: nvdec-copy priority with fallback to auto-copy / auto-safe
        let hwdec_mode = if (scale_factor - 1.0).abs() > 0.01 {
            "nvdec-copy,auto-copy"
        } else {
            "nvdec-copy,auto-safe"
        };
        let _ = self.mpv.set_property("hwdec", hwdec_mode);
        let _ = self.mpv.set_property("framedrop", "no");

        Ok(())
    }

    pub fn load_file(&mut self, path: &str) -> Result<(), String> {
        self.mpv
            .command("loadfile", &[path, "replace"])
            .map_err(|e| format!("Failed to load file: {e}"))?;
        self.is_file_loaded = true;
        self.current_file_path = Some(path.to_string());
        // Re-apply config on new file load
        let cfg = self.current_config.clone();
        let _ = self.apply_config(&cfg);
        Ok(())
    }

    pub fn toggle_pause(&mut self) -> Result<(), String> {
        let paused = self.is_paused();
        self.mpv
            .set_property("pause", !paused)
            .map_err(|e| format!("Failed to toggle pause: {e}"))
    }

    pub fn play(&mut self) -> Result<(), String> {
        self.mpv.set_property("pause", false).map_err(|e| format!("{e}"))
    }

    pub fn pause(&mut self) -> Result<(), String> {
        self.mpv.set_property("pause", true).map_err(|e| format!("{e}"))
    }

    pub fn is_paused(&self) -> bool {
        self.mpv.get_property("pause").unwrap_or(false)
    }

    pub fn is_muted(&self) -> bool {
        self.mpv.get_property("mute").unwrap_or(false)
    }

    pub fn toggle_mute(&mut self) -> Result<(), String> {
        let muted = self.is_muted();
        self.mpv.set_property("mute", !muted).map_err(|e| format!("{e}"))
    }

    pub fn get_volume(&self) -> f64 {
        self.mpv.get_property("volume").unwrap_or(100.0)
    }

    pub fn set_volume(&mut self, vol: f64) -> Result<(), String> {
        self.mpv
            .set_property("volume", vol.clamp(0.0, 150.0))
            .map_err(|e| format!("{e}"))
    }

    pub fn get_time_pos(&self) -> Option<f64> {
        self.mpv.get_property("time-pos").ok()
    }

    pub fn get_duration(&self) -> Option<f64> {
        self.mpv.get_property("duration").ok()
    }

    pub fn seek_to(&mut self, seconds: f64) -> Result<(), String> {
        self.mpv
            .set_property("time-pos", seconds)
            .map_err(|e| format!("{e}"))
    }

    pub fn seek_relative(&mut self, seconds: f64) -> Result<(), String> {
        let s = seconds.to_string();
        self.mpv
            .command("seek", &[&s, "relative+exact"])
            .map_err(|e| format!("{e}"))
    }

    // Telemetry getters
    pub fn get_aspect_ratio(&self) -> f64 {
        if let Some((w, h)) = self.get_dimensions() {
            if h > 0 {
                return w as f64 / h as f64;
            }
        }
        16.0 / 9.0
    }

    pub fn get_video_codec(&self) -> Option<String> {
        self.mpv.get_property("video-codec").ok()
    }

    pub fn get_video_format(&self) -> Option<String> {
        self.mpv.get_property("video-format").ok()
    }

    pub fn get_hwdec_current(&self) -> Option<String> {
        self.mpv.get_property("hwdec-current").ok()
    }

    pub fn get_container_fps(&self) -> Option<f64> {
        self.mpv.get_property("container-fps").ok()
    }

    pub fn get_estimated_vf_fps(&self) -> Option<f64> {
        self.mpv.get_property("estimated-vf-fps").ok()
    }

    pub fn get_display_fps(&self) -> Option<f64> {
        self.mpv.get_property("display-fps").ok()
    }

    pub fn get_dimensions(&self) -> Option<(i64, i64)> {
        let w: i64 = self.mpv.get_property("video-params/w").ok()?;
        let h: i64 = self.mpv.get_property("video-params/h").ok()?;
        Some((w, h))
    }

    pub fn get_video_out_dimensions(&self) -> Option<(i64, i64)> {
        let w: i64 = self.mpv.get_property("video-out-params/w").ok()?;
        let h: i64 = self.mpv.get_property("video-out-params/h").ok()?;
        Some((w, h))
    }

    pub fn get_pixel_format(&self) -> Option<String> {
        self.mpv.get_property("video-params/pixelformat").ok()
    }

    pub fn get_color_primaries(&self) -> Option<String> {
        self.mpv.get_property("video-params/primaries").ok()
    }

    pub fn get_color_gamma(&self) -> Option<String> {
        self.mpv.get_property("video-params/gamma").ok()
    }

    pub fn get_media_title(&self) -> Option<String> {
        self.mpv.get_property("media-title").ok()
    }

    pub fn get_frame_drop_count(&self) -> Option<i64> {
        self.mpv.get_property("frame-drop-count").ok()
    }

    // Render step
    pub fn render(&mut self, size: FramebufferSize) -> Result<(glow::NativeFramebuffer, FramebufferSize), String> {
        let gl_res = self.gl_renderer.ensure_framebuffer(size)?;
        let fb = gl_res.framebuffer;
        let fb_size = gl_res.size;

        let flags = self.render_ctx.update().unwrap_or(0);
        let frame_updated = flags & mpv_render_update::Frame != 0;

        if frame_updated || true {
            let fb_val: i32 = fb.0.get() as i32;
            let _ = self.render_ctx.render::<()>(fb_val, fb_size.width, fb_size.height, true);
            self.render_ctx.report_swap();
        }

        Ok((fb, fb_size))
    }
}
