use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpscaleMode {
    Off,
    Fsr,
    SnapdragonGsr,
    Cas,
    Anime4k,
}

impl UpscaleMode {
    pub fn label(&self) -> &'static str {
        match self {
            UpscaleMode::Off => "Desactivado (Nativo)",
            UpscaleMode::Fsr => "AMD FidelityFX FSR 1.0 (EASU + RCAS)",
            UpscaleMode::SnapdragonGsr => "Qualcomm Snapdragon GSR v1",
            UpscaleMode::Cas => "AMD Contrast Adaptive Sharpening (CAS)",
            UpscaleMode::Anime4k => "Anime4K CNN Edge Reconstruction",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RenderScale {
    Native,        // 100%
    UltraQuality,  // 77%
    Quality,       // 67%
    Balanced,      // 59%
    Performance,   // 50%
}

impl RenderScale {
    pub fn factor(&self) -> f32 {
        match self {
            RenderScale::Native => 1.0,
            RenderScale::UltraQuality => 0.77,
            RenderScale::Quality => 0.67,
            RenderScale::Balanced => 0.59,
            RenderScale::Performance => 0.50,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            RenderScale::Native => "100% (Nativo)",
            RenderScale::UltraQuality => "77% (Ultra Calidad)",
            RenderScale::Quality => "67% (Calidad FSR)",
            RenderScale::Balanced => "59% (Equilibrado)",
            RenderScale::Performance => "50% (Rendimiento)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitDepth {
    Auto,
    Bits8,
    Bits10,
    Bits12,
    Bits16,
}

impl BitDepth {
    pub fn as_str(&self) -> &'static str {
        match self {
            BitDepth::Auto => "auto",
            BitDepth::Bits8 => "8",
            BitDepth::Bits10 => "10",
            BitDepth::Bits12 => "12",
            BitDepth::Bits16 => "16",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            BitDepth::Auto => "Auto",
            BitDepth::Bits8 => "8-bit (SDR)",
            BitDepth::Bits10 => "10-bit (HDR / Extended)",
            BitDepth::Bits12 => "12-bit (Deep Color)",
            BitDepth::Bits16 => "16-bit (Half Float Master)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DitherAlgo {
    ErrorDiffusion,
    Fruit,
    Ordered,
    None,
}

impl DitherAlgo {
    pub fn as_str(&self) -> &'static str {
        match self {
            DitherAlgo::ErrorDiffusion => "error-diffusion",
            DitherAlgo::Fruit => "fruit",
            DitherAlgo::Ordered => "ordered",
            DitherAlgo::None => "no",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterpFilter {
    Oversample,
    Mitchell,
    Spline,
    Linear,
}

impl InterpFilter {
    pub fn as_str(&self) -> &'static str {
        match self {
            InterpFilter::Oversample => "oversample",
            InterpFilter::Mitchell => "mitchell",
            InterpFilter::Spline => "spline",
            InterpFilter::Linear => "linear",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FrameGenMultiplier {
    Native,        // 1x (Original video fps)
    Double,        // 2x (e.g. 24 -> 48 fps, 30 -> 60 fps)
    Cinematic2_5,  // 2.5x (24 -> 60 fps)
    Triple,        // 3x (e.g. 24 -> 72 fps)
    VsyncMatch,    // VSync (Matches exact physical display rate, 60/120 Hz)
    Custom,        // Custom target FPS
}

impl FrameGenMultiplier {
    pub fn label(&self) -> &'static str {
        match self {
            FrameGenMultiplier::Native => "1x Nativo",
            FrameGenMultiplier::Double => "2x Doble",
            FrameGenMultiplier::Cinematic2_5 => "2.5x Cine (60 fps)",
            FrameGenMultiplier::Triple => "3x Triple",
            FrameGenMultiplier::VsyncMatch => "VSync Pantalla",
            FrameGenMultiplier::Custom => "Personalizado",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementConfig {
    // Upscaling & Super Resolution
    pub upscale_mode: UpscaleMode,
    pub render_scale: RenderScale,

    // AMD FSR 1.0 Manual Parameters
    pub fsr_sharpness: f32,
    pub fsr_denoise: f32,
    pub fsr_edge_threshold: f32,
    pub fsr_deringing: bool,

    // Qualcomm Snapdragon GSR Parameters
    pub sgsr_edge_threshold: f32,
    pub sgsr_edge_sharpness: f32,
    pub sgsr_edge_direction: bool,

    // CAS
    pub cas_sharpness: f32,

    // Debanding & Bit Depth Extension
    pub deband_enabled: bool,
    pub deband_iterations: u32,
    pub deband_threshold: u32,
    pub deband_range: u32,
    pub deband_grain: u32,
    pub bit_depth: BitDepth,
    pub dither_algo: DitherAlgo,
    pub temporal_dither: bool,
    pub extra_deband_shader: bool,

    // Frame Generation (LSFG GPU-Direct Native)
    pub interpolation_enabled: bool,
    pub frame_gen_mode: FrameGenMultiplier,
    pub custom_target_fps: f64,
    pub vsync_target_hz: f64,

    // Hardware decoding
    pub hwdec: String,
}

impl EnhancementConfig {
    pub fn config_file_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/home/maple/.config"))
            .join("maple-player")
            .join("settings.json")
    }

    pub fn load_from_disk() -> Self {
        let path = Self::config_file_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<Self>(&content) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save_to_disk(&self) {
        let path = Self::config_file_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(serialized) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, serialized);
        }
    }
}

impl Default for EnhancementConfig {
    fn default() -> Self {
        Self {
            upscale_mode: UpscaleMode::Off, // Default Off for instantaneous, pristine video display
            render_scale: RenderScale::Native, // 100% Native default

            fsr_sharpness: 0.80,
            fsr_denoise: 0.20,
            fsr_edge_threshold: 0.25,
            fsr_deringing: true,

            sgsr_edge_threshold: 4.0,
            sgsr_edge_sharpness: 2.0,
            sgsr_edge_direction: true,

            cas_sharpness: 0.80,

            deband_enabled: true,
            deband_iterations: 4,
            deband_threshold: 48,
            deband_range: 16,
            deband_grain: 32,
            bit_depth: BitDepth::Bits10,
            dither_algo: DitherAlgo::ErrorDiffusion,
            temporal_dither: true,
            extra_deband_shader: false,

            interpolation_enabled: false,
            frame_gen_mode: FrameGenMultiplier::VsyncMatch,
            custom_target_fps: 60.0,
            vsync_target_hz: 120.21,

            hwdec: "auto-safe".to_string(),
        }
    }
}
