//! The first window's size on a Linux Wayland desktop (ours).
//!
//! winit 0.30.13 reports a Wayland monitor by its current mode and ignores the output's transform
//! (`platform_impl/linux/wayland/output.rs`), and its scale factor is the integer `wl_output.scale`
//! (a desktop at 1.25 reports 2). eframe 0.36.2 divides the monitor size by that scale and clamps
//! the first window to it (`native/epi_integration.rs`), so a 1920x1200 laptop panel at 1.25
//! (desktop 1536x960 points) opened FilmCraft at 960x600 points, and a handheld's sideways panel
//! got the unturned shape.
//!
//! Before the window opens we ask the compositor ourselves, once, over a short-lived second
//! connection with a hard time limit ([`QUERY_LIMIT`]): `zxdg_output_v1.logical_size` is the
//! screen in points with the rotation and the fractional scale already applied. Without it, a
//! screen turned a quarter is its mode swapped and divided by the integer scale (right for a
//! turned screen only). Otherwise, and on any error, timeout or odd answer, the window opens as
//! before and one info line says why. With a known screen the asked size is kept when it fits
//! (room left for panels), else the window opens maximized with a return size that fits, and
//! eframe's clamp to winit's monitor size is switched off.
//!
//! X11 (no `WAYLAND_DISPLAY`), a socket handed over in `WAYLAND_SOCKET` (that one is winit's) and
//! gamescope are left alone before anything connects. Screens plugged in or turned while the app
//! runs are not tracked. When winit applies the transform and the fractional scale itself, this
//! module can go.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::sync::mpsc;
use std::time::Duration;

/// Room left around a window that is not maximized (panels, title bar), in points; as
/// EffectCraft's `SCREEN_ROOM`.
pub const SCREEN_ROOM: [f32; 2] = [64.0, 128.0];

/// The longest the compositor query may delay the window.
pub const QUERY_LIMIT: Duration = Duration::from_millis(300);

/// A screen side, in points, outside this range is an odd answer and not used.
const SANE_SIDE: std::ops::RangeInclusive<f32> = 100.0..=16384.0;

/// One output as the compositor describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// Connector name (`eDP-1`), from `wl_output.name` or `zxdg_output_v1.name`.
    pub name: Option<String>,
    /// The current mode in pixels as the panel reports it (unturned): what winit calls its size.
    pub mode: Option<[i32; 2]>,
    /// Shown turned by 90 or 270 degrees.
    pub quarter_turned: bool,
    /// `wl_output.scale`, the integer winit reports.
    pub scale: i32,
    /// `zxdg_output_v1.logical_size`: the screen in points as the desktop uses it.
    pub logical: Option<[i32; 2]>,
}

impl Default for Output {
    fn default() -> Self {
        Self { name: None, mode: None, quarter_turned: false, scale: 1, logical: None }
    }
}

/// Where a screen size came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    XdgOutput,
    TurnedMode,
}

impl Source {
    fn label(self) -> &'static str {
        match self {
            Source::XdgOutput => "xdg-output",
            Source::TurnedMode => "turned mode / integer scale",
        }
    }
}

fn sane(size: [f32; 2]) -> bool {
    size.iter().all(|s| s.is_finite() && SANE_SIDE.contains(s))
}

/// The screen's size in points, if it can be told: the xdg-output logical size, else (only for a
/// screen turned a quarter) the mode swapped and divided by the integer scale. `None` for an
/// upright screen without a logical size (winit's own figure is then all there is) and for zero,
/// negative or absurd figures.
pub fn screen_points(output: &Output) -> Option<([f32; 2], Source)> {
    if let Some([w, h]) = output.logical {
        let size = [w as f32, h as f32];
        if sane(size) {
            return Some((size, Source::XdgOutput));
        }
    }
    let [w, h] = output.mode?;
    if !output.quarter_turned || output.scale < 1 {
        return None;
    }
    let scale = output.scale as f32;
    let size = [h as f32 / scale, w as f32 / scale];
    sane(size).then_some((size, Source::TurnedMode))
}

/// What the first window gets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fit {
    /// As today: eframe and winit decide.
    Untouched,
    /// The asked size (it fits), with eframe's clamp off.
    Open([f32; 2]),
    /// Maximized; un-maximizing returns to `restore`.
    Maximized { restore: [f32; 2] },
}

/// The asked size when it fits the screen with [`SCREEN_ROOM`] to spare, else maximized with a
/// return size that does (never below `min` unless the screen itself is smaller).
pub fn fit(asked: [f32; 2], min: [f32; 2], screen: Option<[f32; 2]>) -> Fit {
    let Some(screen) = screen.filter(|s| sane(*s)) else { return Fit::Untouched };
    let room = [(screen[0] - SCREEN_ROOM[0]).max(0.0), (screen[1] - SCREEN_ROOM[1]).max(0.0)];
    if asked[0] <= room[0] && asked[1] <= room[1] {
        return Fit::Open(asked);
    }
    let restore = [room[0].max(min[0]).min(screen[0]), room[1].max(min[1]).min(screen[1])];
    Fit::Maximized { restore }
}

/// Why the compositor is not asked; `None` when it may be. `var` reads an environment variable.
pub fn gate(var: impl Fn(&str) -> Option<String>) -> Option<&'static str> {
    if !cfg!(target_os = "linux") {
        return Some("not Linux");
    }
    if var("WAYLAND_DISPLAY").is_none_or(|d| d.is_empty()) {
        return Some("no WAYLAND_DISPLAY (X11 or XWayland), winit's monitor size is right there");
    }
    if var("WAYLAND_SOCKET").is_some() {
        return Some("WAYLAND_SOCKET is set (that socket is winit's)");
    }
    let gamescope = var("XDG_CURRENT_DESKTOP").is_some_and(|d| d.to_ascii_lowercase().contains("gamescope")) || var("GAMESCOPE_WAYLAND_DISPLAY").is_some();
    if gamescope {
        return Some("gamescope (Game Mode)");
    }
    None
}

/// Why a query gave no outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    TimedOut(Duration),
    Failed(String),
}

/// Runs `job` on its own thread and waits at most `limit` for it. A job still running then is
/// left to finish (or not) on its own; the caller goes on.
pub fn with_limit<T: Send + 'static>(limit: Duration, job: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, Failure> {
    let (tx, rx) = mpsc::sync_channel(1);
    let spawned = std::thread::Builder::new().name("window-fit-query".into()).spawn(move || {
        let _ = tx.send(job());
    });
    if let Err(e) = spawned {
        return Err(Failure::Failed(format!("no thread for the query: {e}")));
    }
    match rx.recv_timeout(limit) {
        Ok(answer) => answer.map_err(Failure::Failed),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(Failure::TimedOut(limit)),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(Failure::Failed("the query stopped without an answer".into())),
    }
}

/// What the query came back with.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// Not asked, and why.
    Gated(&'static str),
    Failed(Failure, Duration),
    Outputs(Vec<Output>, Duration),
}

/// The decision for the first window, and its log line.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub fit: Fit,
    /// A lowered minimum size, when the screen is smaller than the asked minimum.
    pub min: Option<[f32; 2]>,
    /// Log at warn (the size changed) rather than info.
    pub warn: bool,
    pub line: String,
}

fn wxh(size: [f32; 2]) -> String {
    format!("{}x{}", size[0].round(), size[1].round())
}

fn ratio(r: f32) -> String {
    let s = format!("{r:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn ms(d: Duration) -> u128 {
    d.as_millis()
}

/// The decision from what the compositor said; pure.
pub fn decide(asked: Option<[f32; 2]>, min: [f32; 2], answer: &Answer) -> Decision {
    let untouched =
        |head: String, reason: String| Decision { fit: Fit::Untouched, min: None, warn: false, line: format!("decision: window: {head}untouched: {reason}") };
    let Some(asked) = asked else { return untouched(String::new(), "no size asked".into()) };
    let head = format!("asked {} points; ", wxh(asked));
    let (outputs, took) = match answer {
        Answer::Gated(why) => return untouched(head, (*why).into()),
        Answer::Failed(Failure::TimedOut(limit), _) => {
            return untouched(head, format!("the compositor did not answer within {} ms", ms(*limit)));
        }
        Answer::Failed(Failure::Failed(e), took) => return untouched(head, format!("compositor query failed after {} ms: {e}", ms(*took))),
        Answer::Outputs(outputs, took) => (outputs, *took),
    };
    let known: Vec<(&Output, [f32; 2], Source)> = outputs.iter().filter_map(|o| screen_points(o).map(|(s, src)| (o, s, src))).collect();
    let Some(&(output, screen, source)) = known.iter().min_by(|a, b| (a.1[0] * a.1[1]).total_cmp(&(b.1[0] * b.1[1]))) else {
        let reason = if outputs.is_empty() {
            "the compositor listed no outputs".to_string()
        } else if let Some(odd) = outputs.iter().find_map(|o| o.logical.filter(|[w, h]| !sane([*w as f32, *h as f32]))) {
            format!("odd xdg-output size {}x{} and no screen turned a quarter", odd[0], odd[1])
        } else {
            "no xdg-output logical size and no screen turned a quarter".to_string()
        };
        return untouched(head, format!("{reason} ({} ms)", ms(took)));
    };
    let result = fit(asked, min, Some(screen));
    // What eframe would have clamped to: the largest monitor winit reports, mode / integer scale.
    let winit_largest = outputs
        .iter()
        .filter_map(|o| o.mode.filter(|_| o.scale >= 1).map(|[w, h]| [w as f32 / o.scale as f32, h as f32 / o.scale as f32]))
        .fold([0.0f32, 0.0f32], |acc, s| [acc[0].max(s[0]), acc[1].max(s[1])]);
    let eframe_would_clamp = winit_largest != [0.0, 0.0] && (asked[0] > winit_largest[0] || asked[1] > winit_largest[1]);
    let lowered_min = (min[0] > screen[0] || min[1] > screen[1]).then(|| [min[0].min(screen[0]), min[1].min(screen[1])]);
    let mut details = format!("{}, {} ms", source.label(), ms(took));
    if known.len() > 1 {
        details.push_str(&format!(", smallest of {} screens", known.len()));
    }
    if let Some([mw, mh]) = output.mode {
        details.push_str(&format!("; winit reports {mw}x{mh} at scale {}", output.scale));
        let shown_width = if output.quarter_turned { mh } else { mw } as f32;
        if source == Source::XdgOutput && screen[0] > 0.0 {
            details.push_str(&format!(", implied {}", ratio(shown_width / screen[0])));
        }
    }
    let offered = match result {
        Fit::Untouched => "untouched".to_string(),
        Fit::Open(size) => format!("offered {} as asked", wxh(size)),
        Fit::Maximized { restore } => format!("offered maximized, return size {}", wxh(restore)),
    };
    let mut line = format!("decision: window: {head}screen {} points ({details}); {offered}, eframe clamp off", wxh(screen));
    if let Some(m) = lowered_min {
        line.push_str(&format!(", minimum size lowered to {}", wxh(m)));
    }
    let warn = matches!(result, Fit::Maximized { .. }) || eframe_would_clamp || lowered_min.is_some();
    Decision { fit: result, min: lowered_min, warn, line }
}

impl Decision {
    /// Puts the decision into the main window's options.
    pub fn apply_to(&self, viewport: &mut egui::ViewportBuilder) {
        match self.fit {
            Fit::Untouched => return,
            Fit::Open(size) => viewport.inner_size = Some(egui::vec2(size[0], size[1])),
            Fit::Maximized { restore } => {
                viewport.inner_size = Some(egui::vec2(restore[0], restore[1]));
                viewport.maximized = Some(true);
            }
        }
        viewport.clamp_size_to_monitor_size = Some(false);
        if let Some(m) = self.min {
            viewport.min_inner_size = Some(egui::vec2(m[0], m[1]));
        }
    }
}

#[cfg(target_os = "linux")]
mod query {
    use super::Output;
    use wayland_client::protocol::{wl_output, wl_registry};
    use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
    use wayland_protocols::xdg::xdg_output::zv1::client::{zxdg_output_manager_v1, zxdg_output_v1};

    #[derive(Default)]
    struct State {
        outputs: Vec<Output>,
        wl_outputs: Vec<wl_output::WlOutput>,
        manager: Option<zxdg_output_manager_v1::ZxdgOutputManagerV1>,
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for State {
        fn event(state: &mut Self, registry: &wl_registry::WlRegistry, event: wl_registry::Event, (): &(), _: &Connection, qh: &QueueHandle<Self>) {
            let wl_registry::Event::Global { name, interface, version } = event else { return };
            if interface == "wl_output" {
                let index = state.outputs.len();
                state.outputs.push(Output::default());
                // 4 brings the connector name, which EffectCraft matches winit's monitor by.
                state.wl_outputs.push(registry.bind(name, version.min(4), qh, index));
            } else if interface == "zxdg_output_manager_v1" {
                state.manager = Some(registry.bind(name, version.min(3), qh, ()));
            }
        }
    }

    impl Dispatch<wl_output::WlOutput, usize> for State {
        fn event(state: &mut Self, _: &wl_output::WlOutput, event: wl_output::Event, index: &usize, _: &Connection, _: &QueueHandle<Self>) {
            let Some(output) = state.outputs.get_mut(*index) else { return };
            match event {
                wl_output::Event::Geometry { transform, .. } => {
                    use wl_output::Transform as T;
                    output.quarter_turned = matches!(transform, WEnum::Value(T::_90 | T::_270 | T::Flipped90 | T::Flipped270));
                }
                wl_output::Event::Mode { flags: WEnum::Value(flags), width, height, .. } if flags.contains(wl_output::Mode::Current) => {
                    output.mode = Some([width, height]);
                }
                wl_output::Event::Scale { factor } => output.scale = factor,
                wl_output::Event::Name { name } => output.name = Some(name),
                _ => {}
            }
        }
    }

    impl Dispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, ()> for State {
        fn event(
            _: &mut Self,
            _: &zxdg_output_manager_v1::ZxdgOutputManagerV1,
            _: zxdg_output_manager_v1::Event,
            (): &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<zxdg_output_v1::ZxdgOutputV1, usize> for State {
        fn event(state: &mut Self, _: &zxdg_output_v1::ZxdgOutputV1, event: zxdg_output_v1::Event, index: &usize, _: &Connection, _: &QueueHandle<Self>) {
            let Some(output) = state.outputs.get_mut(*index) else { return };
            match event {
                zxdg_output_v1::Event::LogicalSize { width, height } => output.logical = Some([width, height]),
                zxdg_output_v1::Event::Name { name } if output.name.is_none() => output.name = Some(name),
                _ => {}
            }
        }
    }

    /// Every output the compositor advertises. The first round trip delivers the globals (and
    /// sends our binds), the second each output's events and its xdg-output's.
    pub fn outputs() -> Result<Vec<Output>, String> {
        let conn = Connection::connect_to_env().map_err(|e| format!("connect: {e}"))?;
        let mut queue = conn.new_event_queue::<State>();
        let qh = queue.handle();
        let _registry = conn.display().get_registry(&qh, ());
        let mut state = State::default();
        queue.roundtrip(&mut state).map_err(|e| format!("roundtrip: {e}"))?;
        let mut xdg_outputs = Vec::new();
        if let Some(manager) = &state.manager {
            for (index, wl_output) in state.wl_outputs.iter().enumerate() {
                xdg_outputs.push(manager.get_xdg_output(wl_output, &qh, index));
            }
        }
        queue.roundtrip(&mut state).map_err(|e| format!("roundtrip: {e}"))?;
        for xdg_output in &xdg_outputs {
            xdg_output.destroy();
        }
        if let Some(manager) = &state.manager {
            manager.destroy();
        }
        let _ = conn.flush();
        Ok(state.outputs)
    }
}

/// Asks the compositor (unless [`gate`] says not to), within [`QUERY_LIMIT`].
#[cfg(target_os = "linux")]
pub fn answer_now() -> Answer {
    if let Some(why) = gate(|name| std::env::var_os(name).map(|v| v.to_string_lossy().into_owned())) {
        return Answer::Gated(why);
    }
    let started = std::time::Instant::now();
    match with_limit(QUERY_LIMIT, query::outputs) {
        Ok(outputs) => Answer::Outputs(outputs, started.elapsed()),
        Err(failure) => Answer::Failed(failure, started.elapsed()),
    }
}

/// Sizes the main window for the screen it opens on (see the module). Asks the compositor on the
/// first call only and logs the decision once; `main` builds the options again for each graphics
/// retry and gets the same answer.
#[cfg(target_os = "linux")]
static DECISION: std::sync::OnceLock<Decision> = std::sync::OnceLock::new();

pub fn apply(viewport: &mut egui::ViewportBuilder) {
    #[cfg(target_os = "linux")]
    {
        let decision = DECISION.get_or_init(|| {
            let asked = viewport.inner_size.map(|v| [v.x, v.y]);
            let min = viewport.min_inner_size.map_or([0.0, 0.0], |v| [v.x, v.y]);
            let decision = decide(asked, min, &answer_now());
            if decision.warn {
                log::warn!("{}", decision.line);
            } else {
                log::info!("{}", decision.line);
            }
            decision
        });
        decision.apply_to(viewport);
    }
    #[cfg(not(target_os = "linux"))]
    let _ = viewport;
}

/// After the main window opened maximized ([`Fit::Maximized`]), each time it is un-maximized:
/// `window: restored to WxH points (return size WxH)`, so checking the return size needs no
/// screenshot. Nothing when the window did not open maximized.
#[derive(Debug, Default)]
pub struct RestoreWatch {
    maximized: bool,
    /// Its size while maximized.
    maximized_size: Option<[f32; 2]>,
    /// Un-maximized, new size not seen yet (on Wayland it arrives a frame or more later).
    pending: bool,
}

impl RestoreWatch {
    /// One frame's view of the window: maximized (when known) and its inner size in points.
    /// The line to log, on the first frame after un-maximizing whose size differs from the
    /// maximized one.
    pub fn step(&mut self, maximized: Option<bool>, inner: Option<[f32; 2]>, restore: [f32; 2]) -> Option<String> {
        if let Some(now) = maximized {
            let was = std::mem::replace(&mut self.maximized, now);
            if now {
                self.pending = false;
                self.maximized_size = inner.or(self.maximized_size);
            } else if was {
                self.pending = true;
            }
        }
        let size = inner.filter(|s| self.pending && !self.maximized && Some(*s) != self.maximized_size)?;
        self.pending = false;
        Some(format!("window: restored to {} points (return size {})", wxh(size), wxh(restore)))
    }

    /// Waiting for the size after un-maximizing (the caller asks for another frame).
    pub fn waiting(&self) -> bool {
        self.pending
    }
}

/// Watch the main window for un-maximizing (see [`RestoreWatch`]); a no-op unless [`apply`]
/// opened it maximized.
pub fn watch(ctx: &egui::Context) {
    #[cfg(target_os = "linux")]
    {
        let Some(Fit::Maximized { restore }) = DECISION.get().map(|d| d.fit) else { return };
        let watch = std::sync::Mutex::new(RestoreWatch::default());
        ctx.on_end_pass(
            "window-fit-restore",
            std::sync::Arc::new(move |ui| {
                // inner_rect needs the window's position, which Wayland never gives: the content
                // area egui lays out is the same size
                let ctx = ui.ctx();
                let (maximized, inner) = ctx.input(|i| (i.viewport().maximized, i.viewport().inner_rect.map(|r| [r.width(), r.height()])));
                let inner = inner.or_else(|| Some(ctx.content_rect().size()).map(|v| [v.x, v.y]));
                let mut watch = watch.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(line) = watch.step(maximized, inner, restore) {
                    log::info!("{line}");
                } else if watch.waiting() {
                    ctx.request_repaint_after(std::time::Duration::from_millis(50));
                }
            }),
        );
    }
    #[cfg(not(target_os = "linux"))]
    let _ = ctx;
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASKED: [f32; 2] = [1600.0, 980.0];
    const MIN: [f32; 2] = [900.0, 560.0];
    const MS4: Duration = Duration::from_millis(4);

    fn legion_go(logical: bool, quarter: bool) -> Output {
        Output { name: Some("eDP-1".into()), mode: Some([1600, 2560]), quarter_turned: quarter, scale: 2, logical: logical.then_some([1280, 800]) }
    }

    fn z13(logical: bool) -> Output {
        Output { name: Some("eDP-1".into()), mode: Some([1920, 1200]), quarter_turned: false, scale: 2, logical: logical.then_some([1536, 960]) }
    }

    fn decide_for(outputs: Vec<Output>) -> Decision {
        decide(Some(ASKED), MIN, &Answer::Outputs(outputs, MS4))
    }

    #[test]
    fn legion_go_turned_90_is_maximized_with_and_without_the_logical_size() {
        for logical in [true, false] {
            let d = decide_for(vec![legion_go(logical, true)]);
            assert_eq!(d.fit, Fit::Maximized { restore: [1216.0, 672.0] }, "logical {logical}");
            assert!(d.warn);
        }
    }

    #[test]
    fn turned_270_is_the_same_as_90() {
        // `quarter_turned` covers 90, 270 and their flipped forms; the swap is the same.
        let out = legion_go(false, true);
        assert_eq!(screen_points(&out), Some(([1280.0, 800.0], Source::TurnedMode)));
        assert_eq!(decide_for(vec![out]).fit, Fit::Maximized { restore: [1216.0, 672.0] });
    }

    #[test]
    fn z13_at_125_is_maximized_and_logs_the_decision() {
        let d = decide_for(vec![z13(true)]);
        assert_eq!(d.fit, Fit::Maximized { restore: [1472.0, 832.0] });
        assert!(d.warn);
        assert_eq!(
            d.line,
            "decision: window: asked 1600x980 points; screen 1536x960 points (xdg-output, 4 ms; winit reports 1920x1200 at scale 2, implied 1.25); \
             offered maximized, return size 1472x832, eframe clamp off"
        );
    }

    #[test]
    fn z13_without_the_logical_size_is_untouched() {
        let d = decide_for(vec![z13(false)]);
        assert_eq!(d.fit, Fit::Untouched);
        assert!(!d.warn);
        assert!(d.line.contains("untouched: no xdg-output logical size"), "{}", d.line);
    }

    #[test]
    fn big_monitor_at_scale_1_opens_as_asked() {
        let big = Output { name: None, mode: Some([2560, 1440]), quarter_turned: false, scale: 1, logical: Some([2560, 1440]) };
        let d = decide_for(vec![big]);
        assert_eq!(d.fit, Fit::Open([1600.0, 980.0]));
        assert!(!d.warn, "eframe would not have clamped: {}", d.line);
    }

    #[test]
    fn square_turned_screen_is_its_own_shape() {
        let square = Output { name: None, mode: Some([2000, 2000]), quarter_turned: true, scale: 2, logical: None };
        assert_eq!(screen_points(&square), Some(([1000.0, 1000.0], Source::TurnedMode)));
        assert_eq!(decide_for(vec![square]).fit, Fit::Maximized { restore: [936.0, 872.0] });
    }

    #[test]
    fn zero_negative_and_absurd_sizes_are_not_used() {
        let mut out = z13(true);
        for odd in [[0, 0], [1536, 0], [-1536, 960], [100_000, 960]] {
            out.logical = Some(odd);
            assert_eq!(screen_points(&out), None, "{odd:?}");
            let d = decide_for(vec![out.clone()]);
            assert_eq!(d.fit, Fit::Untouched);
            assert!(d.line.contains("odd xdg-output size"), "{}", d.line);
        }
        let zero_mode = Output { mode: Some([0, 0]), quarter_turned: true, scale: 2, ..Output::default() };
        assert_eq!(screen_points(&zero_mode), None);
        let zero_scale = Output { mode: Some([1600, 2560]), quarter_turned: true, scale: 0, ..Output::default() };
        assert_eq!(screen_points(&zero_scale), None);
        assert_eq!(fit(ASKED, MIN, Some([0.0, 0.0])), Fit::Untouched);
        assert_eq!(fit(ASKED, MIN, None), Fit::Untouched);
        assert_eq!(decide_for(vec![]).fit, Fit::Untouched);
    }

    #[test]
    fn screen_smaller_than_the_minimum_lowers_the_minimum() {
        let small = Output { name: None, mode: Some([800, 480]), quarter_turned: false, scale: 1, logical: Some([800, 480]) };
        let d = decide_for(vec![small]);
        assert_eq!(d.fit, Fit::Maximized { restore: [800.0, 480.0] });
        assert_eq!(d.min, Some([800.0, 480.0]));
        assert!(d.line.ends_with("minimum size lowered to 800x480"), "{}", d.line);
        // Between the minimum and the room: the return size stays at the minimum.
        assert_eq!(fit(ASKED, MIN, Some([1000.0, 640.0])), Fit::Maximized { restore: [936.0, 560.0] });
    }

    #[test]
    fn several_screens_fit_the_smallest() {
        let big = Output { name: Some("DP-1".into()), mode: Some([2560, 1440]), quarter_turned: false, scale: 1, logical: Some([2560, 1440]) };
        let d = decide_for(vec![big, z13(true)]);
        assert_eq!(d.fit, Fit::Maximized { restore: [1472.0, 832.0] });
        assert!(d.line.contains("smallest of 2 screens"), "{}", d.line);
    }

    #[test]
    fn un_maximizing_logs_the_size_got_once_per_un_maximize() {
        let restore = [1472.0, 832.0];
        let full = Some([1536.0, 862.0]);
        let mut w = RestoreWatch::default();
        assert_eq!(w.step(None, None, restore), None, "not known yet");
        assert_eq!(w.step(Some(true), full, restore), None);
        // un-maximized; the first frames still show the maximized size (Wayland)
        assert_eq!(w.step(Some(false), full, restore), None);
        assert!(w.waiting());
        assert_eq!(w.step(Some(false), None, restore), None);
        assert_eq!(w.step(Some(false), Some([1472.0, 832.0]), restore).as_deref(), Some("window: restored to 1472x832 points (return size 1472x832)"));
        assert!(!w.waiting());
        assert_eq!(w.step(Some(false), Some([1472.0, 832.0]), restore), None, "once");
        // maximized again and back: logged again
        assert_eq!(w.step(Some(true), full, restore), None);
        assert_eq!(w.step(Some(false), Some([1400.0, 800.0]), restore).as_deref(), Some("window: restored to 1400x800 points (return size 1472x832)"));
        // never maximized: nothing
        let mut never = RestoreWatch::default();
        assert_eq!(never.step(Some(false), Some([800.0, 600.0]), restore), None);
    }

    #[test]
    fn gate_leaves_x11_handed_sockets_and_gamescope_alone() {
        let env = |pairs: &'static [(&'static str, &'static str)]| move |name: &str| pairs.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string());
        if !cfg!(target_os = "linux") {
            // elsewhere the window is always left to winit, whatever the environment says
            assert_eq!(gate(env(&[("WAYLAND_DISPLAY", "wayland-0"), ("XDG_CURRENT_DESKTOP", "KDE")])), Some("not Linux"));
            assert_eq!(gate(env(&[])), Some("not Linux"));
            return;
        }
        assert_eq!(gate(env(&[("WAYLAND_DISPLAY", "wayland-0"), ("XDG_CURRENT_DESKTOP", "KDE")])), None);
        assert!(gate(env(&[])).is_some());
        assert!(gate(env(&[("WAYLAND_DISPLAY", "")])).is_some());
        assert!(gate(env(&[("WAYLAND_DISPLAY", "wayland-0"), ("WAYLAND_SOCKET", "5")])).is_some());
        assert!(gate(env(&[("WAYLAND_DISPLAY", "gamescope-0"), ("XDG_CURRENT_DESKTOP", "gamescope")])).is_some());
        assert!(gate(env(&[("WAYLAND_DISPLAY", "wayland-0"), ("GAMESCOPE_WAYLAND_DISPLAY", "gamescope-0")])).is_some());
        // SteamGamepadUI alone is not a gamescope sign (coordinator's ruling).
        assert_eq!(gate(env(&[("WAYLAND_DISPLAY", "wayland-0"), ("SteamGamepadUI", "1")])), None);
    }

    #[test]
    fn a_slow_or_failing_query_falls_back_to_untouched_within_the_limit() {
        let started = std::time::Instant::now();
        let slow = with_limit(Duration::from_millis(50), || {
            std::thread::sleep(Duration::from_secs(3));
            Ok(vec![z13(true)])
        });
        assert!(started.elapsed() < Duration::from_secs(1), "waited {:?}", started.elapsed());
        assert_eq!(slow, Err(Failure::TimedOut(Duration::from_millis(50))));
        let d = decide(Some(ASKED), MIN, &Answer::Failed(Failure::TimedOut(Duration::from_millis(50)), started.elapsed()));
        assert_eq!(d.fit, Fit::Untouched);
        assert!(d.line.contains("did not answer within 50 ms"), "{}", d.line);

        let failed = with_limit::<Vec<Output>>(QUERY_LIMIT, || Err("connect: no compositor".into()));
        assert_eq!(failed, Err(Failure::Failed("connect: no compositor".into())));
        let d = decide(Some(ASKED), MIN, &Answer::Failed(Failure::Failed("connect: no compositor".into()), MS4));
        assert_eq!(d.fit, Fit::Untouched);
        assert!(d.line.contains("query failed after 4 ms: connect: no compositor"), "{}", d.line);

        let gated = decide(Some(ASKED), MIN, &Answer::Gated("gamescope (Game Mode)"));
        assert_eq!(gated.line, "decision: window: asked 1600x980 points; untouched: gamescope (Game Mode)");
    }

    #[test]
    fn apply_to_sets_maximized_size_and_clamp_off_only_when_known() {
        let mut vb = egui::ViewportBuilder::default().with_inner_size(ASKED).with_min_inner_size(MIN);
        decide_for(vec![z13(true)]).apply_to(&mut vb);
        assert_eq!(vb.maximized, Some(true));
        assert_eq!(vb.inner_size, Some(egui::vec2(1472.0, 832.0)));
        assert_eq!(vb.clamp_size_to_monitor_size, Some(false));
        assert_eq!(vb.min_inner_size, Some(egui::vec2(900.0, 560.0)));

        let mut vb = egui::ViewportBuilder::default().with_inner_size(ASKED);
        decide_for(vec![z13(false)]).apply_to(&mut vb);
        assert_eq!(vb.maximized, None);
        assert_eq!(vb.inner_size, Some(egui::vec2(1600.0, 980.0)));
        assert_eq!(vb.clamp_size_to_monitor_size, None);
    }
}
