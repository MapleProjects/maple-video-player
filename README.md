# Maple Video Player 🍁

Reproductor de video nativo para Linux desarrollado en Rust con aceleración por hardware en GPU, renderizado en alta precisión de color (RGBA16F), super-resolución AMD FidelityFX FSR 1.0 (EASU + RCAS), Qualcomm Snapdragon Game Super Resolution (GSR v1), reducción de bandas de compresión (debanding & dithering para extender 8 bits a 10/12/16 bits), persistencia de configuraciones y generación de fotogramas de alta fluidez sincrónica con pantallas de 60/120 Hz.

## Características Principales

1. **Renderizado Directo por GPU**:
   - Backend OpenGL acelerado por hardware con `glow` y `libmpv2`.
   - Pipeline de color en `RGBA16F` (punto flotante de 16 bits por canal), eliminando la cuantización a 8 bits.
   - Decodificación por hardware nativa priorizada en NVIDIA (`nvdec-copy`) y fallback seguro (`auto-safe`), compatible con AV1 10-bit (`p010`).
   - Declaración de metadatos de alto rango dinámico y espacio de color extendido (`target-colorspace-hint=yes`, `target-colorspace-hint-mode=source-dynamic`) para evitar que compositores como Hyprland apliquen filtros o clamping SDR.

2. **Super Resolución y Algoritmos de Reconstrucción**:
   - **Escala de Renderizado Interna (Pre-Pase Downscale GPU)**: 100% Nativo, 77% Ultra Calidad, 67% Calidad FSR, 59% Equilibrado y 50% Rendimiento.
   - **Qualcomm Snapdragon Game Super Resolution (GSR v1)**: Implementación oficial en plano LUMA con `HOOKED_gather`, detección direccional de aristas, umbral configurable (1.0-20.0), nitidez de arista (1.0-4.0x) y kernel Fast Lanczos2 con guardas numéricas.
   - **AMD FidelityFX FSR 1.0**:
     - Pase EASU (Edge Adaptive Spatial Upsampling): reconstruye detalles con umbral direccional y filtro anti-ringing.
     - Pase RCAS (Robust Contrast Adaptive Sharpening): nitidez selectiva ajustable con atenuación de ruido.
   - **AMD FidelityFX CAS**: Afilado adaptativo de contraste dinámico.
   - **Anime4K CNN**: Red neuronal convolucional en GLSL para reconstrucción de líneas finas y animación digital.

3. **Generación de Fotogramas (Frame Generation)**:
   - Sincronización precisa con la tasa de refresco física de la pantalla (60 Hz / 120.21 Hz) o multiplicadores personalizados (1x Nativo, 2x Doble, 2.5x Cine 60 fps, 3x Triple, VSync y Custom).
   - Modo `display-resample` no bloqueante con `framedrop=no`, eliminando el descarte y la acumulación de latencia en Wayland.

4. **Extensión de Bandas de Color (Debanding & Dithering)**:
   - Elimina artefactos de compresión y líneas de corte brusco en degradados oscuros o cielos en videos de 8 bits.
   - Control en tiempo real de iteraciones, umbral, rango y grano dinámico.
   - Algoritmos de dithering espacial y temporal (Error Diffusion, Fruit, Ordered) para proyectar transiciones suaves hacia 10 o 12 bits.
   - Shader bilateral secundario de dither TPDF (Triangular Probability Density Function).

5. **Persistencia de Configuraciones**:
   - Guarda automáticamente todas las preferencias de super resolución, generador de cuadros, nitidez y debanding en `~/.config/maple-player/settings.json`.
   - Carga instantánea al inicio de la aplicación sin perder ajustes entre sesiones.

6. **Interfaz Gráfica y Controles**:
   - Construido sobre `egui` y `eframe` nativo en Wayland (Hyprland) y X11.
   - Panel lateral retraíble de ajustes de imagen en caliente.
   - Barra de transporte con seekbar interactivo, volumen (hasta 150%) y control de pantalla completa.
   - Ventana de telemetría en tiempo real (tecla `T`): resolución nativa del contenedor, resolución interna escalada, porcentaje de render, FPS del contenedor, tasa de refresco física, códec, formato de pixel y contador de cuadros descartados.

## Atajos de Teclado

- `Espacio`: Reproducir / Pausa
- `Flecha Izquierda / Derecha`: Retroceder / Avanzar 5 segundos
- `Flecha Arriba / Abajo`: Subir / Bajar volumen
- `M`: Silenciar (Mute)
- `F` o `F11`: Pantalla completa
- `Escape`: Salir de pantalla completa
- `O`: Abrir diálogo de archivo
- `D`: Alternar Debanding on/off
- `S`: Alternar algoritmo de Super Resolución (Off -> FSR 1.0 -> Qualcomm GSR -> CAS -> Anime4K)
- `I`: Alternar Generación de Fotogramas on/off
- `T`: Mostrar / Ocultar ventana de Telemetría

## Compilación y Ejecución

```bash
# Compilar en modo release:
cargo build --release

# Ejecutar:
./target/release/maple-player /ruta/al/video.mp4
```
