//! Ours (self-logging, not upstream): the desktop app's logger, through craft-log.
//!
//! Upstream's [`logging::AppLogger`] stays the logger that decides and writes (same file, folder,
//! rotation and format); craft-log wraps it to add the build line, `CRAFT_LOG`, the repeat
//! collapse, the ring of recent lines and the stall watch.

use std::path::Path;
use std::sync::OnceLock;

use crate::logging::{self, AppLogger, Filter};

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
