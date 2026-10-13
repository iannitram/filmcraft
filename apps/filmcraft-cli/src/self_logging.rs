//! Ours (self-logging, not upstream): the command-line program's log, through craft-log.
//!
//! Every run appends to `<data dir>/Logs/filmcraft-cli.log` (`--data-dir`, `FILMCRAFT_DATA_DIR`
//! or the default data directory), next to the desktop app's `filmcraft.log`; a failing run ends
//! with `log: <path>` on standard error. Standard output and standard error are otherwise unchanged.
//! `filmcraft-cli diagnostics [--json]` prints the diagnostic report, and writes it into the log.

use std::path::PathBuf;

use craft_log::report::Report;

/// FilmCraft's own report fields, shared with the window app (its file).
#[path = "../../filmcraft/src/self_logging_report.rs"]
#[allow(dead_code)] // the window app's names; not all are used here
mod report;

/// FilmCraft, as craft-log names it.
pub const APP: craft_log::App =
    craft_log::App { name: "filmcraft", version: env!("CARGO_PKG_VERSION"), ours: "filmcraft*", default_filter: "warn,filmcraft*=info" };

/// Arguments whose values are never logged: none today (no filmcraft-cli argument carries a
/// secret; `--bridge` is an address).
const SECRET_ARGS: &[&str] = &[];

fn data_dir(given: Option<&str>) -> Option<PathBuf> {
    given.map(PathBuf::from).or_else(filmcraft_engine::autosave::default_data_dir)
}

/// Start this run's log (after `--help` and `--version`, which leave no trace).
pub fn install(given: Option<&str>) {
    let dir = data_dir(given).map(|d| d.join(report::LOG_DIR));
    let _ = craft_log::cli::install(&APP, dir.as_deref(), SECRET_ARGS);
}

/// The run is about to exit with an error: log why, and point at the log.
pub fn failed(reason: &dyn std::fmt::Display) {
    craft_log::cli::failed(&APP, &reason.to_string());
}

/// `filmcraft-cli diagnostics [--json]`: the diagnostic report as text (or one JSON object), also
/// written into this run's log. Ends with the log folder and the newest logs and crash file.
pub fn diagnostics(given: Option<&str>, json: bool) -> String {
    let data = data_dir(given);
    let mut r = Report::new(&APP);
    graphics(&mut r);
    report::video(&mut r);
    r.add("screens", "screens", "unknown (no window)");
    report::folders(&mut r, data.as_deref());
    r.add_switches(report::SWITCH_PREFIXES, report::SECRET_SWITCHES);
    r.add_helpers();
    // the window app's newest log is its last run (or the one running now)
    let window_log = data.as_deref().map(|d| d.join(report::LOG_DIR).join(report::LOG_FILE));
    report::previous_run(&mut r, data.as_deref(), window_log.as_deref());
    report::logs(&mut r, data.as_deref());
    r.log(module_path!());
    if json { format!("{}\n", r.json()) } else { r.text() }
}

/// Graphics: every adapter Vulkan, Metal and DirectX 12 list (not OpenGL, whose drivers can crash
/// on creation), and which one the window and the GPU export renderer would take by wgpu's rule.
/// Opens no device.
fn graphics(r: &mut Report) {
    use filmcraft_gpu::adapter::{Candidate, choose};
    if wgpu::Instance::enabled_backend_features().is_empty() {
        r.add("graphics", "adapters", "unknown: this build of filmcraft-cli has no wgpu backend");
        return;
    }
    let backends = wgpu::Backends::PRIMARY;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends, ..wgpu::InstanceDescriptor::new_without_display_handle() });
    let adapters = block_on(instance.enumerate_adapters(backends));
    let candidates: Vec<Candidate> = adapters.iter().map(|a| Candidate::of(a, None)).collect();
    if candidates.is_empty() {
        r.add("graphics", "adapters", "none listed by Vulkan, Metal or DirectX 12");
    }
    for c in &candidates {
        r.add("graphics", "adapter", c.to_string());
    }
    let pick = |power| choose(&candidates, power).and_then(|i| candidates.get(i)).map(|c| c.name.clone()).unwrap_or_else(|| "none".into());
    r.add(
        "graphics",
        "window would use",
        format!("{} (high performance, wgpu's rule; the window's surface decides)", pick(wgpu::PowerPreference::HighPerformance)),
    );
    r.add("graphics", "export renderer would use", format!("{} (no preference, wgpu's rule)", pick(wgpu::PowerPreference::None)));
}

/// Run a future that wgpu's native backends complete on their first poll.
fn block_on<F: std::future::Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
