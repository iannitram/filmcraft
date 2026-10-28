//! Export mode (header "Export"), laid out like Premiere 26: Destinations and the export queue on
//! the left; the Media File settings in the middle (File Name, Location, Preset, Format, then the
//! Video, Audio, Multiplexer, Captions, Effects and Metadata sections); the preview, Range,
//! Scaling, the Summary with the estimated file size, and Send to Queue / Export on the right.
//! Also the Preset Manager (Preset ▸ More presets…) and the header's Quick Export popup.
//!
//! Every setting lives in [`ExportUi`] (serde, `ui.inspect` / `ui.set {"export": {…}}`) and every
//! action runs an engine command (`file.exportMedia`, `export.queue.*`, `export.presets.*`,
//! `export.quick`).
//!
//! Automation ids: `export.fileName`, `export.location`, `export.preset`, `export.preset.more`,
//! `export.format`, `export.section.<video|audio|multiplexer|captions|effects|metadata>`,
//! `export.video.*`, `export.audio.*`, `export.effects.*`, `export.metadata.*`, `export.range`,
//! `export.scaling`, `export.summary`, `export.estimate`, `export.sendToQueue`, `export.button`,
//! `export.queue.start|stop|clear`, `export.queue.item.<id>` and `….<id>.<up|down|cancel|retry|remove>`;
//! Preset Manager: `presetManager.search`, `presetManager.favoritesOnly`,
//! `presetManager.item.<name>`, `presetManager.favorite.<name>`, `presetManager.saveName`,
//! `presetManager.save`, `presetManager.delete`, `presetManager.import`, `presetManager.export`,
//! `presetManager.ok`, `presetManager.cancel`; Quick Export: `quickExport.path` (the file name),
//! `quickExport.location` (the folder; opens the save dialog), `quickExport.preset.<name>`,
//! `quickExport.go`, `quickExport.close`.

use egui::{Align2, Color32, Rect, Sense, pos2, vec2};
use filmcraft_engine::export::presets::{DEFAULT_PRESET, preset_key};
use filmcraft_engine::export::{
    AudioCodec, BitrateMode, ExportSettings, ExrCompression, FieldOrder, Format, GpuRendering, H264Profile, HardwareEncoding, Multiplexer, MxfVideoCodec,
    Placement, SampleFormat, Scaling, TextOverlay, builtin_presets, format_bytes,
};
use filmcraft_engine::time::{FrameRate, Tick};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::FilmcraftApp;
use crate::frames::{FrameKey, Target};
use crate::native_dialogs::FileDialog;
use crate::theme::Tokens;

mod presets;
mod preview;
mod queue;
mod quick;
mod settings;

use presets::preset_manager;
use preview::preview_column;
#[cfg(test)]
pub(crate) use preview::{RANGES, preview_rects};
use queue::queue_column;
pub use quick::quick_export;
use settings::settings_column;
#[cfg(test)]
use settings::{MAX_LOCATION_DISPLAY_BYTES, location_tail};
#[cfg(test)]
pub(crate) use settings::{PARS, unset_par_label};

/// Export mode state.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExportUi {
    /// The preset the settings came from ("Custom" once edited).
    pub preset: String,
    pub settings: ExportSettings,
    /// File name without extension, and the folder.
    pub file_name: String,
    pub location: String,
    /// `entire` | `inOut` | `workArea` | `custom`.
    pub range: String,
    pub custom_start: f64,
    pub custom_end: f64,
    /// Expanded settings sections.
    pub open_sections: Vec<String>,
    /// The sequence the file name was taken from.
    pub for_sequence: Option<u64>,
    /// Preset Manager (None = closed).
    pub manager: Option<PresetManager>,
    /// Quick Export popup.
    pub quick_open: bool,
    pub quick_path: String,
    pub quick_preset: String,
}

impl Default for ExportUi {
    fn default() -> Self {
        let preset = filmcraft_engine::export::presets::find_builtin(DEFAULT_PRESET).map(|p| p.settings).unwrap_or_default();
        ExportUi {
            preset: DEFAULT_PRESET.into(),
            settings: preset,
            file_name: String::new(),
            location: String::new(),
            range: "entire".into(),
            custom_start: 0.0,
            custom_end: 10.0,
            open_sections: vec!["video".into(), "audio".into()],
            for_sequence: None,
            manager: None,
            quick_open: false,
            quick_path: String::new(),
            quick_preset: String::new(),
        }
    }
}

/// The Preset Manager dialog.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PresetManager {
    pub query: String,
    pub favorites_only: bool,
    pub selected: String,
    pub save_name: String,
}

pub const CUSTOM: &str = "Custom";

/// Collected automation ids (registered after drawing, so widgets can borrow the app freely).
#[derive(Default)]
struct Reg(Vec<(String, Rect, String)>);

impl Reg {
    fn add(&mut self, id: impl Into<String>, r: Rect, label: impl Into<String>) {
        self.0.push((id.into(), r, label.into()));
    }
    fn flush(self, app: &mut FilmcraftApp) {
        for (id, r, l) in self.0 {
            app.auto.add(&id, r, &l);
        }
    }
}

fn settings_json(s: &ExportSettings) -> Value {
    serde_json::to_value(s).unwrap_or_default()
}

/// Apply a preset by name (from the engine's library).
pub fn apply_preset(app: &mut FilmcraftApp, name: &str) -> bool {
    match app.session.export_presets.find(name) {
        Some(p) => {
            app.ui.export.settings = p.settings;
            app.ui.export.preset = p.name;
            true
        }
        None => {
            app.ui.status = tlf!("No export preset named `{name}`", name);
            false
        }
    }
}

/// Full output path of the current settings.
pub fn output_path(ex: &ExportUi) -> String {
    let name = if ex.file_name.trim().is_empty() { "Export" } else { ex.file_name.trim() };
    let dir = filmcraft_engine::export_tools::expand_home(ex.location.trim());
    let file = format!("{name}.{}", ex.settings.extension());
    if dir.is_empty() { file } else { std::path::Path::new(&dir).join(file).to_string_lossy().to_string() }
}

/// Engine params for the current settings (`file.exportMedia` / `export.queue.add`).
pub fn export_params(app: &FilmcraftApp) -> Value {
    let ex = &app.ui.export;
    let mut p = json!({"path": output_path(ex), "settings": settings_json(&ex.settings), "range": ex.range});
    if ex.range == "custom" {
        p["startSeconds"] = json!(ex.custom_start);
        p["endSeconds"] = json!(ex.custom_end);
    }
    p
}

fn sync_sequence(app: &mut FilmcraftApp) {
    let Some(seq) = app.session.state.active_sequence else { return };
    if app.ui.export.for_sequence != Some(seq.0) || app.ui.export.file_name.is_empty() {
        app.ui.export.file_name = app.session.project.item(seq).map(|i| i.name.clone()).unwrap_or_else(|| "Sequence".into());
        app.ui.export.for_sequence = Some(seq.0);
    }
    if app.ui.export.location.is_empty() {
        app.ui.export.location = filmcraft_engine::export_tools::default_export_dir(&app.session).to_string_lossy().to_string();
    }
}

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let left = Rect::from_min_size(rect.min, vec2(300.0, rect.height()));
    let right = Rect::from_min_max(pos2(rect.max.x - (rect.width() * 0.4).clamp(360.0, 640.0), rect.min.y), rect.max);
    let mid = Rect::from_min_max(pos2(left.max.x + 4.0, rect.min.y), pos2(right.min.x - 4.0, rect.max.y));
    for r in [left, mid, right] {
        ui.painter().rect_filled(r, t.radius, t.panel_bg);
    }
    let Some(seq_id) = app.session.state.active_sequence else {
        crate::dock::placeholder(ui, mid, &t, tl!("Open a sequence to export"));
        queue_column(app, ui, left);
        return;
    };
    sync_sequence(app);
    let before = settings_json(&app.ui.export.settings);
    queue_column(app, ui, left);
    settings_column(app, ui, mid);
    preview_column(app, ui, right, seq_id);
    // editing any setting turns the preset into "Custom"
    if settings_json(&app.ui.export.settings) != before
        && app.ui.export.preset != CUSTOM
        && app.session.export_presets.find(&app.ui.export.preset).is_none_or(|p| settings_json(&p.settings) != settings_json(&app.ui.export.settings))
    {
        app.ui.export.preset = CUSTOM.into();
    }
    preset_manager(app, ui.ctx());
    if app.session.export_queue.is_active() || app.session.jobs.iter().any(|j| !j.progress.finished.load(std::sync::atomic::Ordering::Relaxed)) {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The preview draws the output frame and the picture at the shape they are shown at, fitted as
    /// the export fits them: an SD widescreen sequence (32:27 pixels) fills a 16:9 frame, a 4:3 one
    /// (8:9 pixels) sits between bars, and Match Source keeps the sequence's shape.
    #[test]
    fn the_preview_fits_the_picture_by_its_displayed_shape() {
        let area = Rect::from_min_size(pos2(10.0, 20.0), vec2(1600.0, 900.0));
        let aspect = |r: Rect| r.width() / r.height();
        let fills = |pic: Rect, inner: Rect| (inner.width() - pic.width()).abs() < 0.5 && (inner.height() - pic.height()).abs() < 0.5;
        let hd = ExportSettings { frame_size: Some((1920, 1080)), ..Default::default() };
        let (pic, inner) = preview_rects(area, &hd, (720, 480), (32, 27), (1920, 1080));
        assert!((aspect(pic) - 16.0 / 9.0).abs() < 1e-3, "{pic:?}");
        assert!(fills(pic, inner), "a 16:9 picture fills the 16:9 frame: {inner:?} in {pic:?}");
        let (pic, inner) = preview_rects(area, &hd, (720, 480), (8, 9), (1920, 1080));
        assert!(
            (inner.width() / pic.width() - 0.75).abs() < 1e-3 && (inner.height() - pic.height()).abs() < 0.5,
            "a 4:3 picture between bars: {inner:?} in {pic:?}"
        );
        let fill = ExportSettings { scaling: Scaling::ScaleToFill, ..hd.clone() };
        let (pic, inner) = preview_rects(area, &fill, (720, 480), (8, 9), (1920, 1080));
        assert!((inner.width() - pic.width()).abs() < 0.5 && (inner.height() / pic.height() - 4.0 / 3.0).abs() < 1e-3, "{inner:?} in {pic:?}");
        // Match Source: the file keeps the sequence's pixels, so the frame is shown 16:9 too
        let (pic, inner) = preview_rects(area, &ExportSettings::default(), (720, 480), (32, 27), (720, 480));
        assert!((aspect(pic) - 16.0 / 9.0).abs() < 1e-3, "{pic:?}");
        assert!(fills(pic, inner), "{inner:?} in {pic:?}");
        // square pixels chosen for an anamorphic sequence at its own frame size: 16:9 in a 3:2 frame
        let square = ExportSettings { frame_size: Some((720, 480)), ..Default::default() };
        let (pic, inner) = preview_rects(area, &square, (720, 480), (32, 27), (720, 480));
        assert!((aspect(pic) - 1.5).abs() < 1e-3 && (aspect(inner) - 16.0 / 9.0).abs() < 1e-3, "{inner:?} in {pic:?}");
        // a square-pixel sequence is drawn as before
        let (pic, inner) = preview_rects(area, &hd, (1920, 1080), (1, 1), (1920, 1080));
        assert!((aspect(pic) - 16.0 / 9.0).abs() < 1e-3 && fills(pic, inner), "{inner:?} in {pic:?}");
        // hostile sizes and ratios give a finite rectangle
        let odd = ExportSettings { pixel_aspect: Some((0, 0)), ..Default::default() };
        let (pic, inner) = preview_rects(area, &odd, (0, 0), (0, 7), (0, 0));
        assert!(pic.is_finite() && inner.is_finite(), "{inner:?} in {pic:?}");
    }

    /// The Aspect drop-down's entry for no pixel aspect of the export's own names what the file
    /// gets: at Match Source the ratio of a sequence with non-square pixels (it read "Square Pixels
    /// (1.0)" while the file kept 32:27), square pixels otherwise.
    #[test]
    fn the_aspect_entry_for_no_pixel_aspect_names_what_the_file_gets() {
        let matched = ExportSettings::default();
        assert_eq!(unset_par_label(&matched, (32, 27)), "Match Source (1.19)");
        assert_eq!(unset_par_label(&matched, (8, 9)), "Match Source (0.89)");
        assert_eq!(unset_par_label(&matched, (64, 45)), "Match Source (1.42)");
        assert_eq!(unset_par_label(&matched, (1, 1)), "Square Pixels (1.0)");
        // a chosen frame size gets square pixels: the picture is fitted into it
        let hd = ExportSettings { frame_size: Some((1920, 1080)), ..Default::default() };
        assert_eq!(unset_par_label(&hd, (32, 27)), "Square Pixels (1.0)");
        // the entry is named the same while another ratio is chosen
        let chosen = ExportSettings { pixel_aspect: Some((10, 11)), ..Default::default() };
        assert_eq!(unset_par_label(&chosen, (32, 27)), "Match Source (1.19)");
        // the Summary writes the same ratio
        assert!(matched.summary_for(720, 480, (32, 27), FrameRate::FPS_24, 48_000, Tick(0)).video.starts_with("720x480 (1.19), "));
        // hostile ratios are square pixels
        for par in [(0, 0), (0, 7), (5, 0), (u32::MAX, 1)] {
            assert_eq!(unset_par_label(&matched, par), "Square Pixels (1.0)", "{par:?}");
        }
        crate::i18n::set_current(crate::i18n::Language::Es);
        assert_eq!(unset_par_label(&matched, (32, 27)), "Igual que el origen (1.19)");
        crate::i18n::set_current(crate::i18n::Language::En);
    }
}

#[cfg(test)]
mod location_tests {
    use super::*;

    #[test]
    fn location_tail_is_bounded_and_keeps_utf8_intact() {
        let input = format!("/{}終", "é".repeat(MAX_LOCATION_DISPLAY_BYTES));
        let (tail, truncated) = location_tail(&input);
        assert!(truncated);
        assert!(tail.len() <= MAX_LOCATION_DISPLAY_BYTES);
        assert!(tail.ends_with('終'));
        assert!(std::str::from_utf8(tail.as_bytes()).is_ok());
    }
}
