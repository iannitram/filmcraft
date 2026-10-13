//! Whole-process checks of the command-line logger: things a unit test cannot reach because they
//! live in the process itself (its arguments, its standard error, a hang at open). This file has
//! its own `main` (`harness = false`): run as a test it starts copies of itself as the "program"
//! (`CRAFT_LOG_TEST_CHILD=<role>`), each with what the case needs, and checks how they end.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const APP: craft_log::App = craft_log::App { name: "app", version: "1.0.0", ours: "app", default_filter: "warn,app=info" };

fn main() {
    if let Ok(role) = std::env::var("CRAFT_LOG_TEST_CHILD") {
        child(&role);
        return;
    }
    let cases: &[(&str, fn())] = &[
        ("a_non_unicode_argument_is_logged_not_a_panic", a_non_unicode_argument_is_logged_not_a_panic),
    ];
    let filter = std::env::args().skip(1).find(|a| !a.starts_with('-'));
    let mut failed = Vec::new();
    for (name, case) in cases {
        if filter.as_deref().is_some_and(|f| !name.contains(f)) {
            continue;
        }
        let ok = std::panic::catch_unwind(case).is_ok();
        println!("test {name} ... {}", if ok { "ok" } else { "FAILED" });
        if !ok {
            failed.push(*name);
        }
    }
    if !failed.is_empty() {
        println!("failures: {failed:?}");
        std::process::exit(1);
    }
}

/// The "program": what a command-line run does with craft-log, by role.
fn child(role: &str) {
    let dir = std::env::var_os("CRAFT_LOG_TEST_DIR").map(PathBuf::from);
    match role {
        "run" => {
            let _ = craft_log::cli::install(&APP, dir.as_deref());
            log::warn!(target: "app", "child ran");
            log::logger().flush();
        }
        other => panic!("unknown role {other}"),
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!("craft-log-process-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn me(role: &str, dir: &Path) -> Command {
    let mut c = Command::new(std::env::current_exe().unwrap());
    c.env("CRAFT_LOG_TEST_CHILD", role).env("CRAFT_LOG_TEST_DIR", dir).env_remove("CRAFT_LOG").env_remove("RUST_LOG").stdin(Stdio::null());
    c
}

/// Fault 1: `std::env::args()` panics on an argument that is not Unicode (a Latin-1 file name on
/// Linux), which turned a usage error (exit 2) into a crash (exit 101).
fn a_non_unicode_argument_is_logged_not_a_panic() {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let dir = temp_dir("nonunicode");
        let arg = std::ffi::OsString::from_vec(vec![b'f', 0xff, b'.', b'm', b'o', b'v']);
        let out = me("run", &dir).arg(&arg).output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
        let log = std::fs::read_to_string(dir.join("app-cli.log")).unwrap();
        assert!(log.contains("f\u{fffd}.mov"), "{log}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

