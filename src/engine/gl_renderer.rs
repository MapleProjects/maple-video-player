use eframe::glow::{self, HasContext};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferSize {
    pub width: i32,
    pub height: i32,
}

pub struct GlResources {
    pub texture: glow::NativeTexture,
    pub framebuffer: glow::NativeFramebuffer,
    pub size: FramebufferSize,
}

pub struct GlRenderer {
    gl: Arc<glow::Context>,
    resources: Option<GlResources>,
}

impl GlRenderer {
    pub fn new(gl: Arc<glow::Context>) -> Self {
        Self {
            gl,
            resources: None,
        }
    }

    pub fn ensure_framebuffer(
        &mut self,
        size: FramebufferSize,
    ) -> Result<&GlResources, String> {
        if size.width <= 0 || size.height <= 0 {
            return Err("Invalid framebuffer dimensions".to_string());
        }

        let needs_realloc = match &self.resources {
            Some(res) => res.size != size,
            None => true,
        };

        if needs_realloc {
            self.destroy_resources();
            let new_res = unsafe { self.allocate_fbo(size)? };
            self.resources = Some(new_res);
        }

        Ok(self.resources.as_ref().unwrap())
    }

    unsafe fn allocate_fbo(&self, size: FramebufferSize) -> Result<GlResources, String> {
        let gl = &self.gl;

        let texture = gl.create_texture().map_err(|e| format!("Create texture failed: {e}"))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));

        // Use RGBA16F for 16-bit float high bit depth to eliminate 8-bit color banding
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA16F as i32,
            size.width,
            size.height,
            0,
            glow::RGBA,
            glow::HALF_FLOAT,
            glow::PixelUnpackData::Slice(None),
        );

        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        gl.bind_texture(glow::TEXTURE_2D, None);

        let framebuffer = gl.create_framebuffer().map_err(|e| format!("Create framebuffer failed: {e}"))?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );

        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        if status != glow::FRAMEBUFFER_COMPLETE {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.delete_framebuffer(framebuffer);
            gl.delete_texture(texture);

            // Fallback to RGBA8 if RGBA16F is not supported
            tracing::warn!("RGBA16F framebuffer incomplete (0x{:x}), falling back to RGBA8", status);
            return self.allocate_fbo_rgba8(size);
        }

        gl.bind_framebuffer(glow::FRAMEBUFFER, None);

        Ok(GlResources {
            texture,
            framebuffer,
            size,
        })
    }

    unsafe fn allocate_fbo_rgba8(&self, size: FramebufferSize) -> Result<GlResources, String> {
        let gl = &self.gl;
        let texture = gl.create_texture().map_err(|e| format!("Create fallback texture failed: {e}"))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));

        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            size.width,
            size.height,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );

        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        gl.bind_texture(glow::TEXTURE_2D, None);

        let framebuffer = gl.create_framebuffer().map_err(|e| format!("Create fallback fbo failed: {e}"))?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );

        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);

        if status != glow::FRAMEBUFFER_COMPLETE {
            return Err(format!("Framebuffer incomplete: 0x{:x}", status));
        }

        Ok(GlResources {
            texture,
            framebuffer,
            size,
        })
    }

    pub fn destroy_resources(&mut self) {
        if let Some(res) = self.resources.take() {
            unsafe {
                self.gl.delete_framebuffer(res.framebuffer);
                self.gl.delete_texture(res.texture);
            }
        }
    }
}

impl Drop for GlRenderer {
    fn drop(&mut self) {
        self.destroy_resources();
    }
}
