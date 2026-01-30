//! Wayland layer-shell based recording indicator
//!
//! Uses smithay-client-toolkit to create a layer-shell surface that:
//! - Renders on the overlay layer (always on top)
//! - Has no keyboard interactivity (click-through)
//! - Doesn't appear in taskbar
//! - Properly positions in screen corners

use std::sync::mpsc;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};
use tokio::sync::broadcast;
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use crate::cli::args::IndicatorPosition;
use crate::domain::daemon::{DaemonState, StateUpdate};

/// Window dimensions (compact for time-only display)
const WIDTH: u32 = 100;
const HEIGHT: u32 = 44;

/// Margin from screen edge
const MARGIN: i32 = 20;

/// Embedded 7-segment LCD font (DSEG7 Classic Bold, OFL license)
const FONT_DATA: &[u8] = include_bytes!("../../assets/DSEG7Classic-Bold.ttf");

/// Color helpers (Color::from_rgba8 is not const)
fn bg_color() -> Color {
    Color::from_rgba8(30, 30, 30, 220)
}

fn recording_color() -> Color {
    Color::from_rgba8(220, 50, 50, 255)
}

fn processing_color() -> Color {
    Color::from_rgba8(255, 180, 50, 255)
}

/// Error type for layer shell indicator
#[derive(Debug, thiserror::Error)]
pub enum LayerShellError {
    #[error("Failed to connect to Wayland: {0}")]
    Connection(#[from] wayland_client::ConnectError),
    #[error("Failed to initialize registry: {0}")]
    Registry(#[from] wayland_client::globals::GlobalError),
    #[error("Layer shell not available (compositor doesn't support wlr-layer-shell)")]
    LayerShellNotAvailable,
    #[error("Wayland dispatch error: {0}")]
    Dispatch(#[from] wayland_client::DispatchError),
    #[error("Wayland error: {0}")]
    Wayland(#[from] wayland_client::backend::WaylandError),
    #[error("Failed to create buffer pool: {0}")]
    BufferPool(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Run the layer-shell indicator
///
/// Returns Ok(()) if the indicator ran and exited normally.
/// Returns Err if Wayland/layer-shell is not available (caller should fallback).
pub fn run_indicator(
    position: IndicatorPosition,
    state_rx: broadcast::Receiver<StateUpdate>,
) -> Result<(), LayerShellError> {
    // Bridge broadcast to mpsc for blocking receive
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut state_rx = state_rx;
        while let Ok(update) = state_rx.blocking_recv() {
            if tx.send(update).is_err() {
                break;
            }
        }
    });

    // Connect to Wayland
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    // Create app state
    let mut app = LayerShellIndicator::new(&globals, &qh, position, rx)?;

    // Initial roundtrip to get outputs
    event_queue.roundtrip(&mut app)?;

    // Main event loop
    loop {
        // Process any pending state updates (non-blocking)
        app.process_state_updates();

        // Update surface visibility based on state
        app.update_visibility(&qh);

        // If surface is mapped and dirty, redraw
        if app.surface_mapped && app.dirty {
            if let Err(e) = app.draw(&qh) {
                eprintln!("Layer-shell draw error: {}", e);
            }
            app.dirty = false;
        }

        // Dispatch Wayland events (blocking with timeout)
        event_queue.flush()?;
        if let Some(guard) = event_queue.prepare_read() {
            // Use a short timeout so we can check for state updates
            let fd = guard.connection_fd();
            let mut poll_fds = [nix::poll::PollFd::new(fd, nix::poll::PollFlags::POLLIN)];
            let _ = nix::poll::poll(&mut poll_fds, nix::poll::PollTimeout::from(100u16));
            // Read events, ignoring WouldBlock errors
            match guard.read() {
                Ok(_) => {}
                Err(e) => {
                    // Check if it's a WouldBlock by examining the inner io::Error
                    if let wayland_client::backend::WaylandError::Io(ref io_err) = e {
                        if io_err.kind() != std::io::ErrorKind::WouldBlock {
                            return Err(LayerShellError::Wayland(e));
                        }
                    } else {
                        return Err(LayerShellError::Wayland(e));
                    }
                }
            }
        }
        event_queue.dispatch_pending(&mut app)?;
    }
}

/// Layer shell indicator state
struct LayerShellIndicator {
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm: Shm,
    layer_shell: LayerShell,

    position: IndicatorPosition,
    state_rx: mpsc::Receiver<StateUpdate>,

    // Current daemon state
    daemon_state: DaemonState,
    elapsed_ms: u64,

    // Surface state
    layer_surface: Option<LayerSurface>,
    surface_mapped: bool,
    dirty: bool,

    // Buffer management
    pool: SlotPool,
    buffer: Option<Buffer>,

    // Font for rendering text
    font: fontdue::Font,

    // Track if we've ever created a surface
    surface_created: bool,
}

impl LayerShellIndicator {
    fn new(
        globals: &wayland_client::globals::GlobalList,
        qh: &QueueHandle<Self>,
        position: IndicatorPosition,
        state_rx: mpsc::Receiver<StateUpdate>,
    ) -> Result<Self, LayerShellError> {
        let registry_state = RegistryState::new(globals);
        let output_state = OutputState::new(globals, qh);
        let compositor_state =
            CompositorState::bind(globals, qh).map_err(|_| LayerShellError::LayerShellNotAvailable)?;
        let shm = Shm::bind(globals, qh).map_err(|_| LayerShellError::LayerShellNotAvailable)?;
        let layer_shell =
            LayerShell::bind(globals, qh).map_err(|_| LayerShellError::LayerShellNotAvailable)?;

        // Create buffer pool for rendering
        let pool = SlotPool::new((WIDTH * HEIGHT * 4) as usize, &shm)
            .map_err(|e| LayerShellError::BufferPool(e.to_string()))?;

        // Load embedded 7-segment LCD font
        let font = fontdue::Font::from_bytes(FONT_DATA, fontdue::FontSettings::default())
            .expect("Failed to load embedded font");

        Ok(Self {
            registry_state,
            output_state,
            compositor_state,
            shm,
            layer_shell,
            position,
            state_rx,
            daemon_state: DaemonState::Idle,
            elapsed_ms: 0,
            layer_surface: None,
            surface_mapped: false,
            dirty: false,
            pool,
            buffer: None,
            font,
            surface_created: false,
        })
    }

    fn process_state_updates(&mut self) {
        while let Ok(update) = self.state_rx.try_recv() {
            let state_changed = self.daemon_state != update.state;
            self.daemon_state = update.state;
            self.elapsed_ms = update.elapsed_ms;

            // Mark dirty if state changed or we're recording (timer updates)
            if state_changed || self.daemon_state == DaemonState::Recording {
                self.dirty = true;
            }
        }
    }

    fn update_visibility(&mut self, qh: &QueueHandle<Self>) {
        let should_be_visible = self.daemon_state != DaemonState::Idle;

        if should_be_visible && !self.surface_mapped {
            // Create and map surface
            self.create_surface(qh);
        } else if !should_be_visible && self.surface_mapped {
            // Destroy surface (unmap)
            self.destroy_surface();
        }
    }

    fn create_surface(&mut self, qh: &QueueHandle<Self>) {
        if self.layer_surface.is_some() {
            return;
        }

        let surface = self.compositor_state.create_surface(qh);

        let layer_surface = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Overlay,
            Some("smart-scribe-indicator"),
            None, // Use default output
        );

        // Configure anchoring based on position
        let anchor = match self.position {
            IndicatorPosition::TopRight => Anchor::TOP | Anchor::RIGHT,
            IndicatorPosition::TopLeft => Anchor::TOP | Anchor::LEFT,
            IndicatorPosition::BottomRight => Anchor::BOTTOM | Anchor::RIGHT,
            IndicatorPosition::BottomLeft => Anchor::BOTTOM | Anchor::LEFT,
        };
        layer_surface.set_anchor(anchor);

        // Set margins from screen edge
        match self.position {
            IndicatorPosition::TopRight => {
                layer_surface.set_margin(MARGIN, MARGIN, 0, 0);
            }
            IndicatorPosition::TopLeft => {
                layer_surface.set_margin(MARGIN, 0, 0, MARGIN);
            }
            IndicatorPosition::BottomRight => {
                layer_surface.set_margin(0, MARGIN, MARGIN, 0);
            }
            IndicatorPosition::BottomLeft => {
                layer_surface.set_margin(0, 0, MARGIN, MARGIN);
            }
        }

        // Set size
        layer_surface.set_size(WIDTH, HEIGHT);

        // No keyboard interactivity (click-through)
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

        // Don't grab focus
        layer_surface.set_exclusive_zone(-1);

        // Commit to apply configuration
        layer_surface.commit();

        self.layer_surface = Some(layer_surface);
        self.surface_created = true;
        self.dirty = true;
    }

    fn destroy_surface(&mut self) {
        if let Some(surface) = self.layer_surface.take() {
            drop(surface);
        }
        self.surface_mapped = false;
        self.buffer = None;
    }

    fn draw(&mut self, _qh: &QueueHandle<Self>) -> Result<(), LayerShellError> {
        if self.layer_surface.is_none() {
            return Ok(());
        }

        // Render to pixmap first (before borrowing pool)
        let pixmap = self.render();

        // Allocate buffer
        let (buffer, canvas) = self
            .pool
            .create_buffer(
                WIDTH as i32,
                HEIGHT as i32,
                (WIDTH * 4) as i32,
                wl_shm::Format::Argb8888,
            )
            .map_err(|e| LayerShellError::BufferPool(e.to_string()))?;

        // Copy pixmap data to buffer (convert RGBA to ARGB)
        let src = pixmap.data();
        for (i, chunk) in canvas.chunks_exact_mut(4).enumerate() {
            let si = i * 4;
            // tiny-skia uses RGBA, wayland expects ARGB (actually BGRA on little-endian)
            chunk[0] = src[si + 2]; // B
            chunk[1] = src[si + 1]; // G
            chunk[2] = src[si];     // R
            chunk[3] = src[si + 3]; // A
        }

        // Now access layer_surface for attaching
        let layer_surface = self.layer_surface.as_ref().unwrap();

        // Attach buffer to surface
        buffer.attach_to(layer_surface.wl_surface()).map_err(|e| {
            LayerShellError::BufferPool(format!("Failed to attach buffer: {}", e))
        })?;

        // Damage the entire surface
        layer_surface
            .wl_surface()
            .damage_buffer(0, 0, WIDTH as i32, HEIGHT as i32);

        // Commit the surface
        layer_surface.commit();

        // Store buffer to keep it alive
        self.buffer = Some(buffer);

        Ok(())
    }

    fn render(&self) -> Pixmap {
        let mut pixmap = Pixmap::new(WIDTH, HEIGHT).unwrap();

        // Fill with transparent
        pixmap.fill(Color::TRANSPARENT);

        // Draw rounded background
        let mut paint = Paint::default();
        paint.set_color(bg_color());
        paint.anti_alias = true;

        let radius = 8.0;
        let rect_path = {
            let mut pb = PathBuilder::new();
            pb.move_to(radius, 0.0);
            pb.line_to(WIDTH as f32 - radius, 0.0);
            pb.quad_to(WIDTH as f32, 0.0, WIDTH as f32, radius);
            pb.line_to(WIDTH as f32, HEIGHT as f32 - radius);
            pb.quad_to(WIDTH as f32, HEIGHT as f32, WIDTH as f32 - radius, HEIGHT as f32);
            pb.line_to(radius, HEIGHT as f32);
            pb.quad_to(0.0, HEIGHT as f32, 0.0, HEIGHT as f32 - radius);
            pb.line_to(0.0, radius);
            pb.quad_to(0.0, 0.0, radius, 0.0);
            pb.close();
            pb.finish().unwrap()
        };

        pixmap.fill_path(
            &rect_path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );

        // Get color based on state (red = recording, orange = processing)
        let indicator_color = match self.daemon_state {
            DaemonState::Recording => recording_color(),
            DaemonState::Processing => processing_color(),
            DaemonState::Idle => return pixmap, // Should not reach here
        };

        // Draw colored circle indicator
        paint.set_color(indicator_color);
        let circle_x = 16.0;
        let circle_y = HEIGHT as f32 / 2.0;
        let circle_radius = 7.0;

        let circle_path = {
            let mut pb = PathBuilder::new();
            pb.push_circle(circle_x, circle_y, circle_radius);
            pb.finish().unwrap()
        };
        pixmap.fill_path(
            &circle_path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );

        // Draw time in LCD style (same color as indicator)
        let time_text = self.format_elapsed();
        self.draw_time(&mut pixmap, &time_text, indicator_color);

        pixmap
    }

    fn draw_time(&self, pixmap: &mut Pixmap, text: &str, color: Color) {
        let font_size = 18.0;
        // Get actual glyph height from a representative digit for proper centering
        let (metrics, _) = self.font.rasterize('0', font_size);
        let glyph_height = metrics.height as f32;
        let y_baseline = (HEIGHT as f32 + glyph_height) / 2.0;

        // Calculate total text width for horizontal centering
        // Text area starts after the indicator dot (circle at x=16, radius=7, plus margin)
        let text_area_start = 26.0;
        let text_area_width = WIDTH as f32 - text_area_start;
        let text_width: f32 = text
            .chars()
            .map(|ch| self.font.rasterize(ch, font_size).0.advance_width)
            .sum();
        let mut x = text_area_start + (text_area_width - text_width) / 2.0;
        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, font_size);
            if bitmap.is_empty() {
                x += metrics.advance_width;
                continue;
            }

            let glyph_x = x + metrics.xmin as f32;
            let glyph_y = y_baseline - metrics.height as f32 - metrics.ymin as f32;

            // Draw each pixel of the glyph
            for gy in 0..metrics.height {
                for gx in 0..metrics.width {
                    let coverage = bitmap[gy * metrics.width + gx];
                    if coverage == 0 {
                        continue;
                    }

                    let px = (glyph_x + gx as f32) as i32;
                    let py = (glyph_y + gy as f32) as i32;

                    if px >= 0 && px < WIDTH as i32 && py >= 0 && py < HEIGHT as i32 {
                        let alpha = (coverage as f32 / 255.0) * color.alpha();
                        let pixel_color = Color::from_rgba(
                            color.red(),
                            color.green(),
                            color.blue(),
                            alpha,
                        )
                        .unwrap_or(color);

                        // Blend with existing pixel
                        if let Some(existing) = pixmap.pixel(px as u32, py as u32) {
                            let blended = blend_pixel(existing, pixel_color);
                            pixmap.pixels_mut()[(py as u32 * WIDTH + px as u32) as usize] = blended;
                        }
                    }
                }
            }

            x += metrics.advance_width;
        }
    }

    fn format_elapsed(&self) -> String {
        let secs = self.elapsed_ms / 1000;
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{}:{:02}", mins, secs)
    }
}

/// Blend two pixels using alpha compositing
fn blend_pixel(
    dst: tiny_skia::PremultipliedColorU8,
    src: Color,
) -> tiny_skia::PremultipliedColorU8 {
    let src_a = src.alpha();
    let dst_a = dst.alpha() as f32 / 255.0;

    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a == 0.0 {
        return tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 0).unwrap();
    }

    let blend = |src_c: f32, dst_c: u8| -> u8 {
        let dst_c = dst_c as f32 / 255.0;
        let out_c = (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
        (out_c * 255.0).clamp(0.0, 255.0) as u8
    };

    tiny_skia::PremultipliedColorU8::from_rgba(
        blend(src.red(), dst.red()),
        blend(src.green(), dst.green()),
        blend(src.blue(), dst.blue()),
        (out_a * 255.0) as u8,
    )
    .unwrap()
}

// SCTK delegate implementations

impl CompositorHandler for LayerShellIndicator {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        self.dirty = true;
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        self.dirty = true;
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.dirty = true;
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for LayerShellIndicator {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for LayerShellIndicator {
    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
    ) {
        self.destroy_surface();
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // Surface is now configured and can be drawn to
        self.surface_mapped = true;
        self.dirty = true;

        // Acknowledge the configure
        layer.wl_surface().commit();
    }
}

impl ShmHandler for LayerShellIndicator {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for LayerShellIndicator {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}

delegate_compositor!(LayerShellIndicator);
delegate_output!(LayerShellIndicator);
delegate_shm!(LayerShellIndicator);
delegate_layer!(LayerShellIndicator);
delegate_registry!(LayerShellIndicator);
