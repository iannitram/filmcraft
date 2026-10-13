//! Ours (self-logging, not upstream): what the command-line log does to a run, measured on the
//! built program. A run that only prints help or the version leaves no trace; a usage error is a
//! failure and is logged; the log never changes an exit code and never hangs a run.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// A data folder of this test's own that does not exist yet.
fn data_dir(tag: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!("filmcraft-cli-self-logging-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn cli(data: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_filmcraft-cli"));
    c.env("FILMCRAFT_DATA_DIR", data).env("FILMCRAFT_NO_HARDWARE", "1").env_remove("CRAFT_LOG_STDERR").stdin(Stdio::null());
    c
}

fn run(data: &Path, args: &[&str]) -> Output {
    cli(data).args(args).output().expect("spawn filmcraft-cli")
}

#[test]
fn help_and_version_leave_no_log_folder() {
    for args in [&["--version"][..], &["-V"], &["version"], &["--help"], &["-h"], &["help"], &["export", "--help"]] {
        let data = data_dir("quiet");
        let o = run(&data, args);
        assert!(o.status.success(), "{args:?}: {}", String::from_utf8_lossy(&o.stderr));
        assert!(!data.exists(), "{args:?} created {}", data.display());
    }
}

#[test]
fn a_usage_error_is_logged_and_points_at_its_log() {
    let data = data_dir("usage");
    let o = run(&data, &["frobnicate"]);
    assert_eq!(o.status.code(), Some(2));
    let log = data.join("Logs").join("filmcraft-cli.log");
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert_eq!(stderr.lines().last(), Some(format!("log: {}", log.display()).as_str()), "{stderr}");
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(text.contains("ERROR") && text.contains("frobnicate"), "{text}");
    let _ = std::fs::remove_dir_all(&data);
}

/// The exit codes are those of the program without craft-log (measured on Linux with a build of
/// `ours` 3343f4de: an unknown non-Unicode subcommand exits 2, probing a missing non-Unicode file 1).
#[test]
fn a_non_unicode_argument_keeps_the_programs_own_exit_code() {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let bad = std::ffi::OsString::from_vec(vec![0xff]);
        let data = data_dir("nonunicode");
        let o = cli(&data).arg(&bad).output().unwrap();
        assert_eq!(o.status.code(), Some(2), "{}", String::from_utf8_lossy(&o.stderr));
        let o = cli(&data).arg("probe").arg(&bad).output().unwrap();
        assert_eq!(o.status.code(), Some(1), "{}", String::from_utf8_lossy(&o.stderr));
        let text = std::fs::read_to_string(data.join("Logs").join("filmcraft-cli.log")).unwrap();
        assert!(text.contains("command line: ") && text.contains(" probe \u{fffd}"), "{text}");
        let _ = std::fs::remove_dir_all(&data);
    }
}

#[test]
fn a_failure_with_standard_error_full_exits_as_without_the_log() {
    let Ok(full) = std::fs::OpenOptions::new().write(true).open("/dev/full") else { return };
    let data = data_dir("devfull");
    let status = cli(&data).args(["probe", "/nonexistent/x.mov"]).stdout(Stdio::null()).stderr(full).status().unwrap();
    assert_eq!(status.code(), Some(1), "{status}");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_fifo_in_place_of_the_log_does_not_hang_the_run() {
    let data = data_dir("fifo");
    std::fs::create_dir_all(data.join("Logs")).unwrap();
    let fifo = data.join("Logs").join("filmcraft-cli.log");
    if !Command::new("mkfifo").arg(&fifo).status().is_ok_and(|s| s.success()) {
        return;
    }
    let mut child = cli(&data).arg("frobnicate").stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let t0 = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break Some(s);
        }
        if t0.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(status.and_then(|s| s.code()), Some(2), "hung on the FIFO (killed after 10 s) or failed: {status:?}");
    let _ = std::fs::remove_dir_all(&data);
}
