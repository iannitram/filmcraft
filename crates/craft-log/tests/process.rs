//! Whole-process checks of the command-line logger: things a unit test cannot reach because they
//! live in the process itself (its arguments, its standard error, a hang at open). This file has
//! its own `main` (`harness = false`): run as a test it starts copies of itself as the "program"
//! (`CRAFT_LOG_TEST_CHILD=<role>`), each with what the case needs, and checks how they end.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const APP: craft_log::App = craft_log::App { name: "app", version: "1.0.0", ours: "app", default_filter: "warn,app=info" };

fn main() {
    if let Ok(role) = std::env::var("CRAFT_LOG_TEST_CHILD") {
        child(&role);
        return;
    }
    let cases: &[(&str, fn())] = &[
        ("a_non_unicode_argument_is_logged_not_a_panic", a_non_unicode_argument_is_logged_not_a_panic),
        ("a_failing_command_with_standard_error_full_exits_1", a_failing_command_with_standard_error_full_exits_1),
        ("a_failing_test_capture_with_standard_error_full_does_not_abort", a_failing_test_capture_with_standard_error_full_does_not_abort),
        ("a_fifo_in_place_of_the_log_does_not_hang", a_fifo_in_place_of_the_log_does_not_hang),
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
        "fail" => {
            let _ = craft_log::cli::install(&APP, dir.as_deref());
            craft_log::cli::failed(&APP, "the reason it failed");
            std::process::exit(1);
        }
        "capture-panic" => {
            let r = std::thread::spawn(|| {
                let _log = craft_log::testing::capture();
                log::error!(target: "app", "the reason the test failed");
                panic!("deliberate");
            })
            .join();
            assert!(r.is_err());
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

/// Wait for `child` at most `limit`; kill it and return `None` when it does not end.
fn wait_at_most(child: &mut std::process::Child, limit: Duration) -> Option<ExitStatus> {
    let t0 = Instant::now();
    loop {
        if let Some(s) = child.try_wait().unwrap() {
            return Some(s);
        }
        if t0.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
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

/// Fault 2: `eprintln!` panics when standard error cannot be written, so a failing command with
/// standard error full (`2>/dev/full`) or closed exited 101 instead of 1.
fn a_failing_command_with_standard_error_full_exits_1() {
    let Ok(full) = std::fs::OpenOptions::new().write(true).open("/dev/full") else { return };
    let dir = temp_dir("devfull");
    let status = me("fail", &dir).stdout(Stdio::null()).stderr(full).status().unwrap();
    assert_eq!(status.code(), Some(1), "{status}");
    let log = std::fs::read_to_string(dir.join("app-cli.log")).unwrap();
    assert!(log.contains("the reason it failed"), "{log}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Fault 2, the test helper: a failing test's capture printed its lines with `eprint!` from a
/// destructor running during the panic; a second panic there aborts the whole process.
fn a_failing_test_capture_with_standard_error_full_does_not_abort() {
    let Ok(full) = std::fs::OpenOptions::new().write(true).open("/dev/full") else { return };
    let dir = temp_dir("capture");
    let status = me("capture-panic", &dir).stdout(Stdio::null()).stderr(full).status().unwrap();
    assert_eq!(status.code(), Some(0), "{status}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Fault 3: opening a FIFO for writing waits for a reader, so a FIFO in place of the log file
/// hung every command. The log is never waited for.
fn a_fifo_in_place_of_the_log_does_not_hang() {
    let dir = temp_dir("fifo");
    let fifo = dir.join("app-cli.log");
    match Command::new("mkfifo").arg(&fifo).status() {
        Ok(s) if s.success() => {}
        _ => return, // no mkfifo (Windows): nothing to check
    }
    let mut child = me("run", &dir).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().unwrap();
    let status = wait_at_most(&mut child, Duration::from_secs(10));
    assert_eq!(status.and_then(|s| s.code()), Some(0), "the run hung on the FIFO (killed after 10 s) or failed: {status:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

