use crate::engine::backend::MpvBackend;
use crate::engine::config::{BitDepth, DitherAlgo, FrameGenMultiplier, RenderScale, UpscaleMode};
use crate::engine::gl_renderer::FramebufferSize;
use eframe::egui::{self, Color32, CornerRadius, RichText, Sense, Stroke, Vec2};
use eframe::glow::{self, HasContext};
use std::sync::Arc;

pub struct MaplePlayerApp {
    backend: MpvBackend,
    show_settings: bool,
    show_telemetry: bool,
    fullscreen: bool,
    error_message: Option<String>,
    pending_seek: Option<f64>,
    last_activity: std::time::Instant,
}

impl MaplePlayerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        // Configure sleek modern dark aesthetic
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(14, 16, 22);
        visuals.window_fill = Color32::from_rgb(20, 24, 32);
        visuals.override_text_color = Some(Color32::from_rgb(230, 235, 245));
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
        visuals.widgets.active.corner_radius = CornerRadius::same(6);
        visuals.selection.bg_fill = Color32::from_rgb(255, 120, 40);
        cc.egui_ctx.set_visuals(visuals);

        let backend = MpvBackend::new(cc)?;
        Ok(Self {
            backend,
            show_settings: true,
            show_telemetry: false,
            fullscreen: false,
            error_message: None,
            pending_seek: None,
            last_activity: std::time::Instant::now(),
        })
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Videos", &["mp4", "mkv", "webm", "avi", "mov", "flv", "ts"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            if let Err(e) = self.backend.load_file(&path_str) {
                self.error_message = Some(e);
            } else {
                self.error_message = None;
            }
        }
    }

    pub fn load_file(&mut self, path: &str) {
        if let Err(e) = self.backend.load_file(path) {
            self.error_message = Some(e);
        } else {
            self.error_message = None;
        }
    }

    fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        self.fullscreen = !self.fullscreen;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
        self.last_activity = std::time::Instant::now();
        ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(true));
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            let _ = self.backend.toggle_pause();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            let _ = self.backend.seek_relative(-5.0);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            let _ = self.backend.seek_relative(5.0);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            let current = self.backend.get_volume();
            let _ = self.backend.set_volume(current + 5.0);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            let current = self.backend.get_volume();
            let _ = self.backend.set_volume(current - 5.0);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::M)) {
            let _ = self.backend.toggle_mute();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F) || i.key_pressed(egui::Key::F11)) {
            self.toggle_fullscreen(ctx);
        }
        if self.fullscreen && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.toggle_fullscreen(ctx);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::O)) {
            self.open_file_dialog();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::D)) {
            let mut cfg = self.backend.current_config.clone();
            cfg.deband_enabled = !cfg.deband_enabled;
            cfg.save_to_disk();
            let _ = self.backend.apply_config(&cfg);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::S)) {
            let mut cfg = self.backend.current_config.clone();
            cfg.upscale_mode = match cfg.upscale_mode {
                UpscaleMode::Off => UpscaleMode::Fsr,
                UpscaleMode::Fsr => UpscaleMode::SnapdragonGsr,
                UpscaleMode::SnapdragonGsr => UpscaleMode::Cas,
                UpscaleMode::Cas => UpscaleMode::Anime4k,
                UpscaleMode::Anime4k => UpscaleMode::Off,
            };
            cfg.save_to_disk();
            let _ = self.backend.apply_config(&cfg);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::I)) {
            let mut cfg = self.backend.current_config.clone();
            cfg.interpolation_enabled = !cfg.interpolation_enabled;
            cfg.save_to_disk();
            let _ = self.backend.apply_config(&cfg);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::T)) {
            self.show_telemetry = !self.show_telemetry;
        }
    }

    fn format_time(seconds: f64) -> String {
        let total = seconds.max(0.0) as u64;
        let s = total % 60;
        let m = (total / 60) % 60;
        let h = total / 3600;
        if h > 0 {
            format!("{h:02}:{m:02}:{s:02}")
        } else {
            format!("{m:02}:{s:02}")
        }
    }
}

impl eframe::App for MaplePlayerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);

        let mouse_moved = ctx.input(|i| i.pointer.delta() != Vec2::ZERO || i.pointer.any_click());
        let any_key = ctx.input(|i| !i.raw.events.is_empty());
        if mouse_moved || any_key {
            self.last_activity = std::time::Instant::now();
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(true));
        } else if self.fullscreen && self.last_activity.elapsed().as_secs_f32() > 3.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(false));
        }

        // Top Header (hidden in fullscreen)
        if !self.fullscreen {
            egui::TopBottomPanel::top("top_panel")
                .frame(
                    egui::Frame::side_top_panel(&ctx.style())
                        .fill(Color32::from_rgb(12, 14, 18))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(30, 35, 48))),
                )
                .show(ctx, |ui| {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        // Logo badge
                        ui.label(
                            RichText::new("🍁 MAPLE VIDEO PLAYER")
                                .strong()
                                .size(15.0)
                                .color(Color32::from_rgb(255, 145, 60)),
                        );

                    ui.add_space(6.0);
                    if ui
                        .button(RichText::new("📂 Abrir Video").size(13.0))
                        .on_hover_text("Abrir archivo local (Atajo: O)")
                        .clicked()
                    {
                        self.open_file_dialog();
                    }

                    if let Some(title) = self.backend.get_media_title() {
                        ui.separator();
                        ui.label(RichText::new(title).color(Color32::from_rgb(180, 190, 205)).italics());
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let settings_active = self.show_settings;
                        if ui
                            .selectable_label(
                                settings_active,
                                RichText::new(if settings_active { "⚙ Ocultar Ajustes" } else { "⚙ Ajustes & IA" })
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.show_settings = !self.show_settings;
                        }

                        if ui
                            .selectable_label(
                                self.show_telemetry,
                                RichText::new("📊 Telemetría").color(Color32::from_rgb(160, 180, 220)),
                            )
                            .clicked()
                        {
                            self.show_telemetry = !self.show_telemetry;
                        }

                        // Status badges (Pills)
                        let cfg = &self.backend.current_config;

                        // Frame Generation Badge
                        let fg_text = if cfg.interpolation_enabled {
                            format!("🎞 LSFG {}", cfg.frame_gen_mode.label())
                        } else {
                            "🎞 LSFG: OFF".to_string()
                        };
                        let fg_col = if cfg.interpolation_enabled {
                            Color32::from_rgb(192, 132, 252) // Purple
                        } else {
                            Color32::from_rgb(110, 115, 130)
                        };
                        ui.label(RichText::new(fg_text).size(11.0).color(fg_col).strong());

                        // Deband Badge
                        let deband_text = if cfg.deband_enabled {
                            format!("🎨 Deband ({})", cfg.bit_depth.label())
                        } else {
                            "🎨 Deband: OFF".to_string()
                        };
                        let deband_col = if cfg.deband_enabled {
                            Color32::from_rgb(74, 222, 128) // Green
                        } else {
                            Color32::from_rgb(110, 115, 130)
                        };
                        ui.label(RichText::new(deband_text).size(11.0).color(deband_col).strong());

                        // Upscale Badge
                        let upscale_text = match cfg.upscale_mode {
                            UpscaleMode::Fsr => format!("⚡ FSR 1.0 ({})", cfg.render_scale.label()),
                            UpscaleMode::SnapdragonGsr => format!("⚡ Qualcomm GSR ({})", cfg.render_scale.label()),
                            UpscaleMode::Cas => "⚡ AMD CAS".to_string(),
                            UpscaleMode::Anime4k => format!("⚡ Anime4K CNN ({})", cfg.render_scale.label()),
                            UpscaleMode::Off => "⚡ Escalado: OFF".to_string(),
                        };
                        let upscale_col = match cfg.upscale_mode {
                            UpscaleMode::Fsr => Color32::from_rgb(56, 189, 248),
                            UpscaleMode::SnapdragonGsr => Color32::from_rgb(244, 63, 94),
                            UpscaleMode::Cas => Color32::from_rgb(96, 165, 250),
                            UpscaleMode::Anime4k => Color32::from_rgb(251, 191, 36),
                            UpscaleMode::Off => Color32::from_rgb(110, 115, 130),
                        };
                        ui.label(RichText::new(upscale_text).size(11.0).color(upscale_col).strong());
                    });
                });
                ui.add_space(2.0);
            });
        }

        // Bottom Controls
        let show_bottom_controls = !self.fullscreen || self.last_activity.elapsed().as_secs_f32() < 3.0;
        if show_bottom_controls {
            egui::TopBottomPanel::bottom("bottom_panel")
                .frame(
                    egui::Frame::side_top_panel(&ctx.style())
                        .fill(Color32::from_rgb(12, 14, 18))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(30, 35, 48))),
                )
                .show(ctx, |ui| {
                    ui.add_space(4.0);

                    // Progress Bar / Seek Slider
                    let time_pos = self.pending_seek.or_else(|| self.backend.get_time_pos()).unwrap_or(0.0);
                    let duration = self.backend.get_duration().unwrap_or(0.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(Self::format_time(time_pos))
                                .monospace()
                                .size(12.0)
                                .color(Color32::from_rgb(220, 225, 235)),
                        );

                        let mut seek_val = time_pos;
                        let slider = egui::Slider::new(&mut seek_val, 0.0..=duration.max(1.0))
                            .show_value(false)
                            .trailing_fill(true);

                        let res = ui.add_sized([ui.available_width() - 200.0, 16.0], slider);
                        if res.dragged() {
                            self.pending_seek = Some(seek_val);
                        }
                        if res.drag_stopped() {
                            if let Some(target) = self.pending_seek.take() {
                                let _ = self.backend.seek_to(target);
                            }
                        }

                        ui.label(
                            RichText::new(Self::format_time(duration))
                                .monospace()
                                .size(12.0)
                                .color(Color32::from_rgb(140, 150, 165)),
                        );

                        // Fullscreen button
                        if ui
                            .button(if self.fullscreen { "⤓ Normal" } else { "⛶ Pantalla Completa" })
                            .clicked()
                        {
                            self.toggle_fullscreen(ctx);
                        }
                    });

                    ui.add_space(2.0);

                    // Transport Controls
                    ui.horizontal(|ui| {
                        let paused = self.backend.is_paused();
                        let play_pause_icon = if paused { "▶ Reproducir" } else { "⏸ Pausa" };
                        if ui.button(RichText::new(play_pause_icon).strong()).clicked() {
                            let _ = self.backend.toggle_pause();
                        }

                        if ui.button("⏪ -5s").clicked() {
                            let _ = self.backend.seek_relative(-5.0);
                        }
                        if ui.button("⏩ +5s").clicked() {
                            let _ = self.backend.seek_relative(5.0);
                        }

                        ui.separator();

                        // Volume
                        let muted = self.backend.is_muted();
                        let vol_icon = if muted { "🔇" } else { "🔊" };
                        if ui.button(vol_icon).clicked() {
                            let _ = self.backend.toggle_mute();
                        }

                        let mut vol = self.backend.get_volume();
                        let vol_slider = egui::Slider::new(&mut vol, 0.0..=150.0)
                            .show_value(true)
                            .suffix("%");
                        if ui.add_sized([120.0, 16.0], vol_slider).changed() {
                            let _ = self.backend.set_volume(vol);
                        }

                        if let Some(err) = &self.error_message {
                            ui.separator();
                            ui.label(RichText::new(err).color(Color32::from_rgb(248, 113, 113)).small());
                        }
                    });

                    ui.add_space(4.0);
                });
        }

        // Settings / Enhancement Drawer
        if self.show_settings && !self.fullscreen {
            egui::SidePanel::right("settings_panel")
                .frame(
                    egui::Frame::side_top_panel(&ctx.style())
                        .fill(Color32::from_rgb(18, 21, 28))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(35, 40, 55))),
                )
                .default_width(360.0)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.heading(
                        RichText::new("⚡ Mejoras de Imagen & IA")
                            .color(Color32::from_rgb(255, 160, 60))
                            .strong(),
                    );
                    ui.label(
                        RichText::new("Procesamiento en GPU y Reconstrucción en Tiempo Real")
                            .color(Color32::from_rgb(140, 150, 168))
                            .small(),
                    );
                    ui.add_space(8.0);

                    let mut cfg = self.backend.current_config.clone();
                    let mut changed = false;

                    let card_frame = egui::Frame::NONE
                        .fill(Color32::from_rgb(24, 28, 38))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 52, 70)))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::same(10));

                    // Group 1: Super Resolution & Upscaling
                    card_frame.show(ui, |ui| {
                        ui.label(
                            RichText::new("1. Super Resolución (FSR / CAS / Anime4K)")
                                .strong()
                                .color(Color32::from_rgb(56, 189, 248)),
                        );
                        ui.label(
                            RichText::new("Escalado nítido con reconstrucción sub-píxel y redes neuronales")
                                .small()
                                .color(Color32::from_rgb(160, 170, 185)),
                        );
                        ui.add_space(6.0);

                        ui.label(RichText::new("Escala Interna de Renderizado (Render Scale):").strong().size(12.0));
                        ui.horizontal_wrapped(|ui| {
                            if ui.selectable_value(&mut cfg.render_scale, RenderScale::Native, "100% Nativo").clicked() {
                                changed = true;
                            }
                            if ui.selectable_value(&mut cfg.render_scale, RenderScale::UltraQuality, "77% Ultra").clicked() {
                                changed = true;
                            }
                            if ui.selectable_value(&mut cfg.render_scale, RenderScale::Quality, "67% Calidad FSR").clicked() {
                                changed = true;
                            }
                            if ui.selectable_value(&mut cfg.render_scale, RenderScale::Balanced, "59% Equilibrado").clicked() {
                                changed = true;
                            }
                            if ui.selectable_value(&mut cfg.render_scale, RenderScale::Performance, "50% Rendimiento").clicked() {
                                changed = true;
                            }
                        });
                        let (orig_w, orig_h) = self.backend.get_dimensions().unwrap_or((1920, 1080));
                        let scaled_w = ((orig_w as f32) * cfg.render_scale.factor()).round() as i64;
                        let scaled_h = ((orig_h as f32) * cfg.render_scale.factor()).round() as i64;
                        ui.label(
                            RichText::new(format!(
                                "Resolución Interna de Render: {}x{} ({:.0}%) ➔ Reconstrucción GPU: {}x{}",
                                scaled_w, scaled_h, cfg.render_scale.factor() * 100.0, orig_w, orig_h
                            ))
                            .small()
                            .strong()
                            .color(if cfg.render_scale == RenderScale::Native {
                                Color32::from_rgb(140, 150, 165)
                            } else {
                                Color32::from_rgb(56, 189, 248)
                            }),
                        );
                        ui.add_space(6.0);

                        ui.label(RichText::new("Algoritmo de Reconstrucción / Super Resolución:").strong().size(12.0));
                        ui.horizontal_wrapped(|ui| {
                            if ui.selectable_value(&mut cfg.upscale_mode, UpscaleMode::Off, "Off").clicked() {
                                changed = true;
                            }
                            if ui.selectable_value(&mut cfg.upscale_mode, UpscaleMode::Fsr, "AMD FSR 1.0 (EASU+RCAS)").clicked() {
                                changed = true;
                            }
                            if ui.selectable_value(&mut cfg.upscale_mode, UpscaleMode::SnapdragonGsr, "Qualcomm Snapdragon GSR").clicked() {
                                changed = true;
                            }
                            if ui.selectable_value(&mut cfg.upscale_mode, UpscaleMode::Cas, "AMD CAS").clicked() {
                                changed = true;
                            }
                            if ui.selectable_value(&mut cfg.upscale_mode, UpscaleMode::Anime4k, "Anime4K CNN").clicked() {
                                changed = true;
                            }
                        });

                        if cfg.upscale_mode == UpscaleMode::Fsr {
                            ui.add_space(6.0);
                            ui.label(RichText::new("Parámetros Manuales AMD FSR 1.0 (EASU + RCAS):").strong().color(Color32::from_rgb(56, 189, 248)));
                            ui.horizontal(|ui| {
                                ui.label("Nitidez RCAS (Sharpness):");
                                if ui.add(egui::Slider::new(&mut cfg.fsr_sharpness, 0.0..=2.0).step_by(0.05).suffix("x")).changed() {
                                    changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Atenuación de Ruido RCAS:");
                                if ui.add(egui::Slider::new(&mut cfg.fsr_denoise, 0.0..=1.0).step_by(0.02)).changed() {
                                    changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Umbral Direccional EASU:");
                                if ui.add(egui::Slider::new(&mut cfg.fsr_edge_threshold, 0.05..=1.0).step_by(0.05)).changed() {
                                    changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut cfg.fsr_deringing, "Filtro Anti-Ringing EASU (Previene halos de sobreimpulso)").changed() {
                                    changed = true;
                                }
                            });
                        } else if cfg.upscale_mode == UpscaleMode::SnapdragonGsr {
                            ui.add_space(6.0);
                            ui.label(RichText::new("Parámetros Qualcomm Snapdragon Game Super Resolution v1:").strong().color(Color32::from_rgb(244, 63, 94)));
                            ui.horizontal(|ui| {
                                ui.label("Umbral de Arista (EdgeThreshold):");
                                if ui.add(egui::Slider::new(&mut cfg.sgsr_edge_threshold, 0.5..=20.0).step_by(0.5)).changed() {
                                    changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Nitidez de Arista (EdgeSharpness):");
                                if ui.add(egui::Slider::new(&mut cfg.sgsr_edge_sharpness, 0.5..=4.0).step_by(0.1).suffix("x")).changed() {
                                    changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut cfg.sgsr_edge_direction, "Detección Direccional de Arista (Fast Lanczos2)").changed() {
                                    changed = true;
                                }
                            });
                        } else if cfg.upscale_mode == UpscaleMode::Cas {
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label("Nitidez AMD CAS:");
                                if ui.add(egui::Slider::new(&mut cfg.cas_sharpness, 0.0..=2.0).step_by(0.05).suffix("x")).changed() {
                                    changed = true;
                                }
                            });
                            ui.label(
                                RichText::new("Contrast Adaptive Sharpening: resalta micro-detalles y contornos.")
                                    .small()
                                    .color(Color32::from_rgb(130, 140, 155)),
                            );
                        } else if cfg.upscale_mode == UpscaleMode::Anime4k {
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("Anime4K: red neuronal convolucional (CNN) para reconstrucción de líneas finas y arte digital.")
                                    .small()
                                    .color(Color32::from_rgb(251, 191, 36)),
                            );
                        }
                    });

                    ui.add_space(8.0);

                    // Group 2: Frame Generation LSFG (GPU Direct)
                    card_frame.show(ui, |ui| {
                        ui.label(
                            RichText::new("2. Generación de Fotogramas LSFG (GPU Direct)")
                                .strong()
                                .color(Color32::from_rgb(192, 132, 252)),
                        );
                        ui.label(
                            RichText::new("Generación de cuadros estilo Lossless Scaling (LSFG) con aceleración directa por GPU y 0% carga de CPU")
                                .small()
                                .color(Color32::from_rgb(160, 170, 185)),
                        );
                        ui.add_space(6.0);

                        if ui
                            .checkbox(&mut cfg.interpolation_enabled, "Activar Generación de Cuadros LSFG")
                            .changed()
                        {
                            changed = true;
                        }

                        if cfg.interpolation_enabled {
                            ui.add_space(4.0);
                            ui.label(RichText::new("Multiplicador / Tasa de Cuadros LSFG:").strong().size(12.0));

                            ui.horizontal_wrapped(|ui| {
                                if ui.selectable_value(&mut cfg.frame_gen_mode, FrameGenMultiplier::Native, "1x Nativo").clicked() {
                                    changed = true;
                                }
                                if ui.selectable_value(&mut cfg.frame_gen_mode, FrameGenMultiplier::Double, "2x Doble").clicked() {
                                    changed = true;
                                }
                                if ui.selectable_value(&mut cfg.frame_gen_mode, FrameGenMultiplier::Cinematic2_5, "2.5x Cine (60)").clicked() {
                                    changed = true;
                                }
                                if ui.selectable_value(&mut cfg.frame_gen_mode, FrameGenMultiplier::Triple, "3x Triple").clicked() {
                                    changed = true;
                                }
                                if ui.selectable_value(&mut cfg.frame_gen_mode, FrameGenMultiplier::VsyncMatch, "VSync Pantalla (120 Hz)").clicked() {
                                    changed = true;
                                }
                                if ui.selectable_value(&mut cfg.frame_gen_mode, FrameGenMultiplier::Custom, "Personalizado").clicked() {
                                    changed = true;
                                }
                            });

                            if cfg.frame_gen_mode == FrameGenMultiplier::Custom {
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label("FPS Objetivo:");
                                    if ui.add(egui::Slider::new(&mut cfg.custom_target_fps, 24.0..=240.0).suffix(" fps")).changed() {
                                        changed = true;
                                    }
                                });
                            }

                            // Info readout
                            let native_fps = self.backend.get_container_fps().unwrap_or(24.0);
                            let target_fps_str = match cfg.frame_gen_mode {
                                FrameGenMultiplier::Native => format!("{native_fps:.1} fps"),
                                FrameGenMultiplier::Double => format!("{:.1} fps (2x)", native_fps * 2.0),
                                FrameGenMultiplier::Cinematic2_5 => format!("{:.1} fps (2.5x)", native_fps * 2.5),
                                FrameGenMultiplier::Triple => format!("{:.1} fps (3x)", native_fps * 3.0),
                                FrameGenMultiplier::VsyncMatch => "VSync (120.2 Hz)".to_string(),
                                FrameGenMultiplier::Custom => format!("{:.1} fps (Custom)", cfg.custom_target_fps),
                            };
                            ui.label(
                                RichText::new(format!("Objetivo: {target_fps_str} • Motor LSFG GPU Direct (0 Drops)"))
                                    .small()
                                    .color(Color32::from_rgb(192, 132, 252)),
                            );
                        }
                    });

                    ui.add_space(8.0);

                    // Group 3: Debanding & Bit-Depth Extension
                    card_frame.show(ui, |ui| {
                        ui.label(
                            RichText::new("3. Debanding & Extensión 8-bit a 10/12-bit")
                                .strong()
                                .color(Color32::from_rgb(74, 222, 128)),
                        );
                        ui.label(
                            RichText::new("Elimina artefactos de compresión y banding en degradados")
                                .small()
                                .color(Color32::from_rgb(160, 170, 185)),
                        );
                        ui.add_space(6.0);

                        if ui.checkbox(&mut cfg.deband_enabled, "Activar Deband en GPU").changed() {
                            changed = true;
                        }

                        if cfg.deband_enabled {
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label("Iteraciones:");
                                if ui.add(egui::Slider::new(&mut cfg.deband_iterations, 1..=8)).changed() {
                                    changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Umbral (Threshold):");
                                if ui.add(egui::Slider::new(&mut cfg.deband_threshold, 16..=128)).changed() {
                                    changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Rango (Range):");
                                if ui.add(egui::Slider::new(&mut cfg.deband_range, 8..=32)).changed() {
                                    changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Grano dinámico:");
                                if ui.add(egui::Slider::new(&mut cfg.deband_grain, 0..=64)).changed() {
                                    changed = true;
                                }
                            });

                            ui.separator();
                            ui.label("Profundidad de color objetivo (Dither Depth):");
                            egui::ComboBox::from_id_salt("bit_depth_select")
                                .selected_text(cfg.bit_depth.label())
                                .show_ui(ui, |ui| {
                                    if ui.selectable_value(&mut cfg.bit_depth, BitDepth::Bits8, BitDepth::Bits8.label()).clicked() {
                                        changed = true;
                                    }
                                    if ui.selectable_value(&mut cfg.bit_depth, BitDepth::Bits10, BitDepth::Bits10.label()).clicked() {
                                        changed = true;
                                    }
                                    if ui.selectable_value(&mut cfg.bit_depth, BitDepth::Bits12, BitDepth::Bits12.label()).clicked() {
                                        changed = true;
                                    }
                                    if ui.selectable_value(&mut cfg.bit_depth, BitDepth::Bits16, BitDepth::Bits16.label()).clicked() {
                                        changed = true;
                                    }
                                    if ui.selectable_value(&mut cfg.bit_depth, BitDepth::Auto, BitDepth::Auto.label()).clicked() {
                                        changed = true;
                                    }
                                });

                            ui.horizontal(|ui| {
                                ui.label("Algoritmo Dither:");
                                egui::ComboBox::from_id_salt("dither_select")
                                    .selected_text(format!("{:?}", cfg.dither_algo))
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_value(&mut cfg.dither_algo, DitherAlgo::ErrorDiffusion, "Error Diffusion").clicked() {
                                            changed = true;
                                        }
                                        if ui.selectable_value(&mut cfg.dither_algo, DitherAlgo::Fruit, "Fruit").clicked() {
                                            changed = true;
                                        }
                                        if ui.selectable_value(&mut cfg.dither_algo, DitherAlgo::Ordered, "Ordered").clicked() {
                                            changed = true;
                                        }
                                        if ui.selectable_value(&mut cfg.dither_algo, DitherAlgo::None, "Desactivado").clicked() {
                                            changed = true;
                                        }
                                    });
                            });

                            if ui.checkbox(&mut cfg.temporal_dither, "Dithering Temporal Dinámico").changed() {
                                changed = true;
                            }

                            if ui.checkbox(&mut cfg.extra_deband_shader, "Shader Bilateral de Dither TPDF").changed() {
                                changed = true;
                            }
                        }
                    });

                    if changed {
                        cfg.save_to_disk();
                        if let Err(e) = self.backend.apply_config(&cfg) {
                            self.error_message = Some(e);
                        }
                    }
                });
        }

        // Telemetry Overlay Window
        if self.show_telemetry {
            egui::Window::new("📊 Telemetría de Renderizado")
                .open(&mut self.show_telemetry)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("telemetry_grid").num_columns(2).striped(true).show(ui, |ui| {
                        ui.label("Decodificador HW:");
                        ui.label(self.backend.get_hwdec_current().unwrap_or_else(|| "Software".to_string()));
                        ui.end_row();

                        ui.label("Códec Video:");
                        ui.label(self.backend.get_video_codec().unwrap_or_else(|| "N/A".to_string()));
                        ui.end_row();

                        ui.label("Formato Contenedor:");
                        ui.label(self.backend.get_video_format().unwrap_or_else(|| "N/A".to_string()));
                        ui.end_row();

                        ui.label("Resolución Contenedor:");
                        if let Some((w, h)) = self.backend.get_dimensions() {
                            ui.label(format!("{w} x {h}"));
                        } else {
                            ui.label("N/A");
                        }
                        ui.end_row();

                        ui.label("Resolución Render (SuperRes):");
                        if let Some((ow, oh)) = self.backend.get_video_out_dimensions() {
                            let scale_pct = (self.backend.current_config.render_scale.factor() * 100.0).round() as i64;
                            if let Some((w, h)) = self.backend.get_dimensions() {
                                if ow != w || oh != h {
                                    ui.label(format!("{ow} x {oh} ({scale_pct}%) ➔ {w} x {h} Reconstruido"));
                                } else {
                                    ui.label(format!("{ow} x {oh} (100% Nativo)"));
                                }
                            } else {
                                ui.label(format!("{ow} x {oh}"));
                            }
                        } else {
                            ui.label("N/A");
                        }
                        ui.end_row();

                        ui.label("Cuadros Perdidos (Drops):");
                        let drops = self.backend.get_frame_drop_count().unwrap_or(0);
                        let drop_col = if drops == 0 { Color32::GREEN } else { Color32::RED };
                        ui.label(RichText::new(format!("{drops}")).color(drop_col).strong());
                        ui.end_row();

                        ui.label("FPS Contenedor:");
                        ui.label(format!("{:.2} fps", self.backend.get_container_fps().unwrap_or(0.0)));
                        ui.end_row();

                        ui.label("FPS Filtro (VF):");
                        ui.label(format!("{:.2} fps", self.backend.get_estimated_vf_fps().unwrap_or(0.0)));
                        ui.end_row();

                        ui.label("FPS Pantalla:");
                        ui.label(format!("{:.2} Hz", self.backend.get_display_fps().unwrap_or(0.0)));
                        ui.end_row();

                        ui.label("Modo FrameGen:");
                        let fg_status = if self.backend.current_config.interpolation_enabled {
                            format!("{:?} • LSFG GPU-Direct", self.backend.current_config.frame_gen_mode)
                        } else {
                            "Desactivado".to_string()
                        };
                        ui.label(fg_status);
                        ui.end_row();

                        ui.label("Shader SuperRes:");
                        let sr_desc = match self.backend.current_config.upscale_mode {
                            UpscaleMode::Off => "Desactivado".to_string(),
                            UpscaleMode::Fsr => format!("FSR 1.0 EASU+RCAS ({:.2}x • {})", self.backend.current_config.fsr_sharpness, self.backend.current_config.render_scale.label()),
                            UpscaleMode::SnapdragonGsr => format!("Qualcomm Snapdragon GSR v1 (Arista {:.1}x • {})", self.backend.current_config.sgsr_edge_sharpness, self.backend.current_config.render_scale.label()),
                            UpscaleMode::Cas => format!("AMD CAS ({:.2}x)", self.backend.current_config.cas_sharpness),
                            UpscaleMode::Anime4k => format!("Anime4K CNN ({})", self.backend.current_config.render_scale.label()),
                        };
                        ui.label(sr_desc);
                        ui.end_row();

                        ui.label("Formato Píxel Entrada:");
                        ui.label(self.backend.get_pixel_format().unwrap_or_else(|| "N/A".to_string()));
                        ui.end_row();

                        ui.label("Gama / Primarias:");
                        let gam = self.backend.get_color_gamma().unwrap_or_else(|| "auto".into());
                        let prim = self.backend.get_color_primaries().unwrap_or_else(|| "auto".into());
                        ui.label(format!("{gam} / {prim}"));
                        ui.end_row();

                        ui.label("Pipeline GPU (FBO):");
                        ui.label(RichText::new("RGBA16F (16-bit Float master)").color(Color32::GREEN));
                        ui.end_row();

                        ui.label("Tag Superficie Wayland:");
                        ui.label(RichText::new("Wide-Gamut Dynamic (Bypass SDR clamp)").color(Color32::from_rgb(56, 189, 248)));
                        ui.end_row();

                        ui.label("Cuadros Perdidos (Drops):");
                        ui.label(format!("{}", self.backend.get_frame_drop_count().unwrap_or(0)));
                        ui.end_row();
                    });
                });
        }

        // Central Video Viewport
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::BLACK))
            .show(ctx, |ui| {
                let available_size = ui.available_size();

                if !self.backend.is_file_loaded {
                    // Empty state
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(available_size.y * 0.22);
                            ui.label(
                                RichText::new("🍁 Maple Video Player")
                                    .size(30.0)
                                    .strong()
                                    .color(Color32::from_rgb(255, 150, 70)),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Reproductor de alto rendimiento con GPU & Shaders para Linux")
                                    .size(14.0)
                                    .color(Color32::from_rgb(180, 190, 205)),
                            );
                            ui.add_space(18.0);

                            if ui
                                .button(RichText::new("📂 Cargar un Archivo de Video").size(16.0).strong())
                                .clicked()
                            {
                                self.open_file_dialog();
                            }

                            ui.add_space(24.0);
                            ui.label(
                                RichText::new("✨ Tecnologías gráficas integradas:\n• Pipeline de renderizado en RGBA16F (extensión 8-bit a 10/12-bit sin banding)\n• Generación de cuadros x1, x2, x2.5 Cine (60 FPS), VSync (120 Hz) y Personalizado\n• Super Resolución AMD FidelityFX FSR 1.0 (EASU + RCAS), AMD CAS y Anime4K CNN\n• Debanding adaptable con dithering TPDF de alta precisión")
                                    .color(Color32::from_rgb(140, 150, 168))
                                    .small(),
                            );
                        });
                    });
                    return;
                }

                // Compute aspect ratio fitted rect
                let aspect_ratio = self.backend.get_aspect_ratio() as f32;
                let (w, h) = if available_size.x / aspect_ratio <= available_size.y {
                    (available_size.x, available_size.x / aspect_ratio)
                } else {
                    (available_size.y * aspect_ratio, available_size.y)
                };

                let ppp = ui.ctx().pixels_per_point();
                let fb_size = FramebufferSize {
                    width: (w * ppp).floor() as i32,
                    height: (h * ppp).floor() as i32,
                };

                let (rect, response) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
                if response.double_clicked() {
                    self.toggle_fullscreen(ctx);
                }

                // Trigger render into FBO
                match self.backend.render(fb_size) {
                    Ok((fb, actual_fb_size)) => {
                        let cb = eframe::egui_glow::CallbackFn::new(move |info, painter| {
                            let gl = painter.gl();
                            unsafe {
                                let prev_read_fb = gl.get_parameter_i32(glow::READ_FRAMEBUFFER_BINDING).cast_unsigned();
                                gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(fb));

                                let p_per_point = info.pixels_per_point;
                                let screen_h = info.screen_size_px[1] as f32;

                                gl.blit_framebuffer(
                                    0,
                                    0,
                                    actual_fb_size.width,
                                    actual_fb_size.height,
                                    (rect.min.x * p_per_point) as i32,
                                    (screen_h - rect.max.y * p_per_point) as i32,
                                    (rect.max.x * p_per_point) as i32,
                                    (screen_h - rect.min.y * p_per_point) as i32,
                                    glow::COLOR_BUFFER_BIT,
                                    glow::LINEAR,
                                );

                                let prev_fb_obj = std::num::NonZeroU32::new(prev_read_fb).map(glow::NativeFramebuffer);
                                gl.bind_framebuffer(glow::READ_FRAMEBUFFER, prev_fb_obj);
                            }
                        });

                        ui.painter().add(egui::PaintCallback {
                            rect,
                            callback: Arc::new(cb),
                        });
                    }
                    Err(e) => {
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("Render error: {e}"),
                            egui::FontId::default(),
                            Color32::RED,
                        );
                    }
                }
            });
    }
}
