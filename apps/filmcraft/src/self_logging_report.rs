//! Ours (self-logging, not upstream): FilmCraft's own fields of the diagnostic report, shared by
//! the window app and `filmcraft-cli` (which includes this file by path, so the two programs say
//! the same things the same way). craft-log gathers the fields every app shares; this adds video
//! decoding, the folders, how the previous window run ended, and where the logs are.

use std::path::{Path, PathBuf};

use craft_log::report::{Report, created, last_line_with, mask_home, modified, newest_file};

/// The window app's log folder inside the data folder, and its log file (upstream's names).
pub const LOG_DIR: &str = "Logs";
pub const LOG_FILE: &str = "filmcraft.log";
/// The previous window run's log after a start rotated it.
pub const PREVIOUS_LOG_FILE: &str = "filmcraft.1.log";
/// The command-line programs' shared log (craft-log's name).
pub const CLI_LOG_FILE: &str = "filmcraft-cli.log";
/// Environment switches FilmCraft reads besides craft-log's own: its own and wgpu's backend choice.
pub const SWITCH_PREFIXES: &[&str] = &["FILMCRAFT_", "WGPU_"];
/// Switches whose values the report never shows: none today (no FILMCRAFT_ or WGPU_ variable
/// carries a secret; craft-log also hides any whose name says TOKEN, SECRET, PASSWORD…).
pub const SECRET_SWITCHES: &[&str] = &[];
/// The window app's last line when it ends (`run ended: …`); the next start looks for it.
pub const END_MARK: &str = "run ended:";
/// How much of the end of the previous log is searched for [`END_MARK`].
const TAIL_BYTES: u64 = 64 * 1024;

/// Whether `name` is one of upstream's crash files (`crash-<day>.log`).
fn is_crash(name: &str) -> bool {
    name.starts_with("crash-") && name.ends_with(".log")
}

/// Video: hardware decoding (the switch, the decoder in front, VA-API and its driver, NVDEC) and
/// encoding, as registered at start-up. Asks the hardware nothing new: the VA-API answer is the
/// one start-up registration already got (it is kept), and nothing is asked at all when hardware
/// decoding was switched off before registration (`--no-hardware`).
pub fn video(r: &mut Report) {
    let d = filmcraft_engine::perf::decode_json();
    let hw = &d["hardware"];
    let enabled = hw["enabled"].as_bool().unwrap_or(true);
    let backend = hw["backend"].as_str().map(str::to_owned);
    r.add("video", "hardware decoding", if enabled { "on" } else { "off" });
    r.add("video", "decoder in front", backend.clone().unwrap_or_else(|| "none: software decoders only".into()));
    #[cfg(target_os = "linux")]
    {
        let va = if !enabled && backend.is_none() {
            "not asked (hardware decoding is off)".to_owned()
        } else {
            match filmcraft_platform::vaapi::va::probe() {
                Ok(driver) => format!("available: {driver}"),
                Err(why) => format!("not available: {why}"),
            }
        };
        r.add("video", "VA-API", va);
        r.add("video", "NVDEC", if backend.as_deref() == Some("NVDEC") { "in use" } else { "not in use" });
        r.add(
            "video",
            "hardware encoding",
            if enabled { "NVENC H.264 registered (acts only with NVIDIA's driver, when an export asks); no VA-API encoder" } else { "none (hardware is off)" },
        );
    }
    #[cfg(not(target_os = "linux"))]
    r.add("video", "hardware encoding", craft_log::report::UNKNOWN);
}

/// Folders: data, settings (the preferences file), media cache.
pub fn folders(r: &mut Report, data: Option<&Path>) {
    let show = |p: Option<PathBuf>| p.map(|p| mask_home(&p.display().to_string())).unwrap_or_default();
    r.add("folders", "data", show(data.map(Path::to_path_buf)));
    let prefs = data.map(|d| d.join("preferences.json"));
    let settings = match &prefs {
        Some(p) if p.is_file() => show(Some(p.clone())),
        Some(p) => format!("{} (not written yet: defaults)", show(Some(p.clone()))),
        None => String::new(),
    };
    r.add("folders", "settings", settings);
    // the configured media cache location, else `<data>/Media Cache`
    let cache_prefs = prefs
        .as_deref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<filmcraft_engine::autosave::Preferences>(&t).ok())
        .map(|p| p.media_cache)
        .unwrap_or_default();
    r.add("folders", "media cache", show(filmcraft_engine::settings::media_cache_dir(&cache_prefs, data)));
}

/// How the previous window run ended, from its log (`previous_log`), the crash files and the
/// graphics start-up state.
pub fn previous_run(r: &mut Report, data: Option<&Path>, previous_log: Option<&Path>) {
    let logs = data.map(|d| d.join(LOG_DIR));
    let ended = match previous_log.filter(|p| p.is_file()) {
        None => "no earlier window log".to_owned(),
        Some(p) => match last_line_with(p, END_MARK, TAIL_BYTES) {
            Some(line) => line.split_once(END_MARK).map_or(line.clone(), |(_, rest)| format!("yes:{rest}")),
            None => format!("no end line in {} (killed, crashed, or still running)", mask_home(&p.display().to_string())),
        },
    };
    r.add("previous run", "ended cleanly", ended);
    let crash = logs.as_deref().and_then(|d| newest_file(d, &is_crash));
    let during = match (previous_log.and_then(created), crash.as_deref().and_then(modified)) {
        (_, None) => "no crash file".to_owned(),
        (Some(start), Some(at)) if at >= start => format!("yes: {}", mask_home(&crash.as_deref().map(|p| p.display().to_string()).unwrap_or_default())),
        (Some(_), Some(_)) => "no (the newest crash file is older)".to_owned(),
        (None, Some(_)) => String::new(),
    };
    r.add("previous run", "crash file written during it", during);
    // graphics-start.json: the mode the window starts with and the failed starts in a row in it
    let state = data.and_then(|d| std::fs::read_to_string(d.join("graphics-start.json")).ok()).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
    let graphics = match state {
        Some(v) => format!("mode {}, {} failed start(s) in a row", v["mode"].as_str().unwrap_or("?"), v["failures"].as_u64().unwrap_or(0)),
        None => "no saved state (default mode, no failed starts)".to_owned(),
    };
    r.add("previous run", "graphics start", graphics);
}

/// The logs, last: the folder, the window log, the command-line log, the newest crash file.
pub fn logs(r: &mut Report, data: Option<&Path>) {
    let dir = data.map(|d| d.join(LOG_DIR));
    let existing = |name: &str| dir.as_deref().map(|d| d.join(name)).filter(|p| p.is_file());
    let window = existing(LOG_FILE);
    let cli = existing(CLI_LOG_FILE);
    let crash = dir.as_deref().and_then(|d| newest_file(d, &is_crash));
    r.add_logs(dir.as_deref(), &[("window log", window.as_deref()), ("command-line log", cli.as_deref())], crash.as_deref());
}
