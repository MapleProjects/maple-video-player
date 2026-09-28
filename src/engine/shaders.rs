use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const FSR_SHADER: &str = include_str!("../shaders/fsr.glsl");
pub const SNAPDRAGON_GSR_SHADER: &str = include_str!("../shaders/snapdragon_gsr.glsl");
pub const CAS_SHADER: &str = include_str!("../shaders/cas.glsl");
pub const DEBAND_EXT_SHADER: &str = include_str!("../shaders/deband_ext.glsl");
pub const ANIME4K_SHADER: &str = include_str!("../shaders/anime4k_upscale_cnn.glsl");

#[derive(Debug)]
pub struct ShaderManager {
    shader_dir: PathBuf,
    generation: AtomicUsize,
}

impl ShaderManager {
    pub fn new() -> Result<Self, std::io::Error> {
        let base_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/home/maple/.config"))
            .join("maple-player")
            .join("shaders");

        fs::create_dir_all(&base_dir)?;

        let mgr = Self {
            shader_dir: base_dir,
            generation: AtomicUsize::new(0),
        };
        mgr.install_default_shaders()?;
        Ok(mgr)
    }

    fn install_default_shaders(&self) -> Result<(), std::io::Error> {
        fs::write(self.shader_dir.join("fsr.glsl"), FSR_SHADER)?;
        fs::write(self.shader_dir.join("snapdragon_gsr.glsl"), SNAPDRAGON_GSR_SHADER)?;
        fs::write(self.shader_dir.join("cas.glsl"), CAS_SHADER)?;
        fs::write(self.shader_dir.join("deband_ext.glsl"), DEBAND_EXT_SHADER)?;
        fs::write(self.shader_dir.join("anime4k.glsl"), ANIME4K_SHADER)?;
        Ok(())
    }

    pub fn get_fsr_shader(
        &self,
        sharpness: f32,
        denoise: f32,
        edge_threshold: f32,
        deringing: bool,
    ) -> Result<PathBuf, std::io::Error> {
        let gen = self.generation.fetch_add(1, Ordering::SeqCst) % 100;
        let file_path = self.shader_dir.join(format!("fsr_v{gen}.glsl"));
        let s = sharpness.clamp(0.0, 2.5);
        let dn = denoise.clamp(0.0, 1.0);
        let et = edge_threshold.clamp(0.05, 1.0);
        let dering_val = if deringing { 1 } else { 0 };

        let modified = FSR_SHADER
            .replace("#define SHARPNESS 0.80", &format!("#define SHARPNESS {:.2}", s))
            .replace("#define FSR_RCAS_DENOISE 0.20", &format!("#define FSR_RCAS_DENOISE {:.2}", dn))
            .replace("#define FSR_EASU_DIR_THRESHOLD 0.25", &format!("#define FSR_EASU_DIR_THRESHOLD {:.2}", et))
            .replace("#define FSR_EASU_DERING 1", &format!("#define FSR_EASU_DERING {}", dering_val));

        fs::write(&file_path, modified)?;
        Ok(file_path)
    }

    pub fn get_snapdragon_gsr_shader(
        &self,
        edge_threshold: f32,
        edge_sharpness: f32,
        use_edge_dir: bool,
    ) -> Result<PathBuf, std::io::Error> {
        let gen = self.generation.fetch_add(1, Ordering::SeqCst) % 100;
        let file_path = self.shader_dir.join(format!("snapdragon_gsr_v{gen}.glsl"));
        let et = edge_threshold.clamp(0.5, 20.0);
        let es = edge_sharpness.clamp(0.5, 4.0);
        let ed_val = if use_edge_dir { 1 } else { 0 };

        let modified = SNAPDRAGON_GSR_SHADER
            .replace("#define EdgeThreshold 4.0", &format!("#define EdgeThreshold {:.1}", et))
            .replace("#define EdgeSharpness 2.0", &format!("#define EdgeSharpness {:.2}", es))
            .replace("#define UseEdgeDirection 1", &format!("#define UseEdgeDirection {}", ed_val));

        fs::write(&file_path, modified)?;
        Ok(file_path)
    }

    pub fn get_cas_shader(&self, sharpness: f32) -> Result<PathBuf, std::io::Error> {
        let gen = self.generation.fetch_add(1, Ordering::SeqCst) % 100;
        let file_path = self.shader_dir.join(format!("cas_v{gen}.glsl"));
        let s = sharpness.clamp(0.0, 2.5);
        let modified = CAS_SHADER.replace(
            "#define SHARPNESS 0.60",
            &format!("#define SHARPNESS {:.2}", s),
        );
        fs::write(&file_path, modified)?;
        Ok(file_path)
    }

    pub fn get_anime4k_shader(&self) -> PathBuf {
        self.shader_dir.join("anime4k.glsl")
    }

    pub fn get_deband_ext_path(&self) -> PathBuf {
        self.shader_dir.join("deband_ext.glsl")
    }
}
