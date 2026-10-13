//! Ours (self-logging, not upstream): the desktop app's logger, through craft-log.
//!
//! Upstream's [`logging::AppLogger`] stays the logger that decides and writes (same file, folder,
//! rotation and format); craft-log wraps it to add the build line, `CRAFT_LOG`, the repeat
//! collapse, the ring of recent lines and the stall watch.

use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use craft_log::report::Report;

use crate::graphics::GraphicsStart;
use crate::logging::{self, AppLogger, Filter};

#[path = "self_logging_report.rs"]
pub mod report;

/// The window's inner size `native_options` in main.rs asks for, in points (keep the two equal).
pub const REQUESTED_INNER_SIZE: [f32; 2] = [1600.0, 980.0];
/// The frame at which the window's size is compared with the size asked for: by then the
/// compositor has configured the window (Wayland sends the size it decided after the first frame).
const SIZE_CHECK_FRAME: u64 = 3;

/// FilmCraft, as craft-log names it.
pub const APP: craft_log::App =
    craft_log::App { name: "filmcraft", version: env!("CARGO_PKG_VERSION"), ours: "filmcraft*", default_filter: logging::DEFAULT_FILTER };

/// In place of [`logging::install`]: the same logger, wrapped. `None` when another logger was
/// installed first.
pub fn install() -> Option<&'static AppLogger> {
    static INNER: OnceLock<AppLogger> = OnceLock::new();
    let (spec, _) = APP.filter_spec();
    let filter = Filter::parse(&spec);
    let max = filter.max();
    // upstream's own installer stays in their file, unused while this one is in place; naming it
    // here keeps it compiled and warning-free without editing their file
    let _upstream: fn() -> Option<&'static AppLogger> = logging::install;
    let inner = INNER.get_or_init(|| AppLogger::new(filter, true));
    craft_log::install_wrapped(&APP, inner, max).then_some(inner)
}

/// The log directory is known: record the log file's path for the lines that point at it.
pub fn log_dir(dir: &Path) {
    craft_log::set_log_path(Some(dir.join(logging::LOG_FILE)));
}

/// Start-up: write the diagnostic report into the log (after the log file, the crash hook, the
/// hardware decoders and the graphics mode are settled, before the window opens).
pub fn start_report(data: Option<&Path>, graphics: &GraphicsStart) {
    let mut r = Report::new(&APP);
    let mode = graphics.mode().label();
    r.add("graphics", "start mode", if graphics.degraded() { format!("{mode} (a fallback after failed starts)") } else { mode.to_owned() });
    r.add("graphics", "backends", std::env::var("WGPU_BACKEND").map_or_else(|_| "the start mode's".to_owned(), |b| format!("WGPU_BACKEND={b}")));
    r.add("graphics", "adapter", "chosen when the window opens: the \"graphics:\" line after this report");
    report::video(&mut r);
    r.add("screens", "screens and window", "known when the window opens: the \"screen\" and \"window:\" lines after this report");
    report::folders(&mut r, data);
    r.add_switches(report::SWITCH_PREFIXES);
    r.add_helpers();
    let previous = data.map(|d| d.join(report::LOG_DIR).join(report::PREVIOUS_LOG_FILE));
    report::previous_run(&mut r, data, previous.as_deref());
    report::logs(&mut r, data);
    r.log(module_path!());
}

/// The window exists (in eframe's app creator, after the GPU compositor was or was not started):
/// log the graphics adapter in use, every screen, and the window's size asked for and got.
pub fn window_ready(cc: &eframe::CreationContext<'_>, gpu_compositor: bool) {
    match &cc.wgpu_render_state {
        Some(rs) => {
            let i = rs.adapter.get_info();
            let what = format!("\"{}\" ({:?}, {:?}, driver {} {})", i.name, i.backend, i.device_type, i.driver, i.driver_info);
            if gpu_compositor {
                log::info!("graphics: the window and the GPU compositor use {what}");
            } else if std::env::var_os("FILMCRAFT_CPU_COMPOSITE").is_some() {
                log::info!("graphics: the window draws with {what}; the compositor runs on the CPU (FILMCRAFT_CPU_COMPOSITE is set)");
            } else {
                log::warn!("decision: graphics: the window draws with {what}, but the compositor runs on the CPU (the reason is the line before)");
            }
        }
        None => log::warn!("decision: graphics: the window has no wgpu device; the compositor runs on the CPU"),
    }
    let asked = REQUESTED_INNER_SIZE;
    if let Some(w) = cc.winit_window() {
        let current = w.current_monitor();
        let mut screens = 0;
        for (n, m) in w.available_monitors().enumerate() {
            screens += 1;
            let (p, s) = (m.position(), m.size());
            let hz = m.refresh_rate_millihertz().map(|h| format!(", {:.2} Hz", f64::from(h) / 1000.0)).unwrap_or_default();
            let here = if current.as_ref() == Some(&m) { ", the window is on it" } else { "" };
            log::info!(
                "screen {n}: \"{}\" at ({}, {}), {}x{} pixels as reported, scale {}{hz}, rotation unknown, usable area unknown{here}",
                m.name().unwrap_or_else(|| "unnamed".into()),
                p.x,
                p.y,
                s.width,
                s.height,
                m.scale_factor()
            );
        }
        if screens == 0 {
            log::info!("screens: none reported by the window system");
        }
        if current.is_none() {
            log::info!("screens: the window system does not say which screen holds the window");
        }
        let (inner, scale) = (w.inner_size(), w.scale_factor());
        let got = inner.to_logical::<f64>(scale);
        log::info!(
            "window: created at {:.0}x{:.0} points ({}x{} pixels, scale {scale}), asked {}x{}; maximized {}",
            got.width,
            got.height,
            inner.width,
            inner.height,
            asked[0],
            asked[1],
            w.is_maximized()
        );
    } else {
        log::info!("screens: unknown (eframe gave no window handle)");
    }
    // after the compositor's first configure: asked versus got, once. The size comes from the
    // window itself: egui's `inner_rect` stays empty on Wayland, where a window has no position.
    let window = cc.winit_window().cloned();
    let done = AtomicBool::new(false);
    cc.egui_ctx.on_end_pass(
        "self-logging-window-size",
        std::sync::Arc::new(move |ui| {
            let ctx = ui.ctx();
            if ctx.cumulative_frame_nr() < SIZE_CHECK_FRAME || done.swap(true, Ordering::Relaxed) {
                return;
            }
            let (rect, ppp) = ctx.input(|i| (i.viewport().inner_rect, i.viewport().native_pixels_per_point));
            let got = match &window {
                Some(w) => {
                    let (inner, scale) = (w.inner_size(), w.scale_factor());
                    let l = inner.to_logical::<f64>(scale);
                    // points, as asked (egui's unit is f32)
                    let points = [l.width as f32, l.height as f32];
                    Some((
                        points,
                        format!(
                            "{}x{} pixels, scale {scale}, maximized {}, full screen {}",
                            inner.width,
                            inner.height,
                            w.is_maximized(),
                            w.fullscreen().is_some()
                        ),
                    ))
                }
                None => rect.map(|r| ([r.width(), r.height()], "from egui".to_owned())),
            };
            let Some((points, detail)) = got else {
                log::info!("window: size unknown after {SIZE_CHECK_FRAME} frames");
                return;
            };
            let text = format!(
                "window: asked {}x{} points, got {:.0}x{:.0} points after {SIZE_CHECK_FRAME} frames ({detail}, egui pixels per point {})",
                asked[0],
                asked[1],
                points[0],
                points[1],
                ppp.map_or_else(|| "unknown".to_owned(), |p| p.to_string()),
            );
            if (points[0] - asked[0]).abs() > 1.0 || (points[1] - asked[1]).abs() > 1.0 {
                log::warn!("decision: {text}: the window system fitted or clamped it");
            } else {
                log::info!("{text}");
            }
        }),
    );
}

/// The window app is ending: its last line, which the next start's report looks for.
pub fn ended(result: &eframe::Result) {
    match result {
        Ok(()) => log::info!("{} the window closed normally", report::END_MARK),
        Err(e) => log::warn!("{} with an error: {e}", report::END_MARK),
    }
    log::logger().flush();
}
