//! Ours (self-logging, not upstream): the command-line program's log, through craft-log.
//!
//! Every run appends to `<data dir>/Logs/filmcraft-cli.log` (`--data-dir`, `FILMCRAFT_DATA_DIR`
//! or the default data directory), next to the desktop app's `filmcraft.log`; a failing run ends
//! with `log: <path>` on standard error. Standard output and standard error are otherwise unchanged.

/// FilmCraft, as craft-log names it.
pub const APP: craft_log::App =
    craft_log::App { name: "filmcraft", version: env!("CARGO_PKG_VERSION"), ours: "filmcraft*", default_filter: "warn,filmcraft*=info" };

/// Start this run's log (after `--help` and `--version`, which leave no trace).
pub fn install(data_dir: Option<&str>) {
    let dir = data_dir.map(std::path::PathBuf::from).or_else(filmcraft_engine::autosave::default_data_dir).map(|d| d.join("Logs"));
    let _ = craft_log::cli::install(&APP, dir.as_deref());
}

/// The run is about to exit with an error: log why, and point at the log.
pub fn failed(reason: &dyn std::fmt::Display) {
    craft_log::cli::failed(&APP, &reason.to_string());
}
