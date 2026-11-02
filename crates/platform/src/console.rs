//! Borrowing the console of the process that started this one.
//!
//! A Windows release build of the app has no console of its own (`windows_subsystem =
//! "windows"`), so `filmcraft --version` started from a terminal would print nowhere. The one
//! `AttachConsole` call that fixes this lives here because `crates/platform` is the only crate
//! allowed `unsafe` (AGENTS.md §0.3, ADR 0001); the app crate forbids it.

/// Attach to the console of the parent process (a terminal), so standard output and error reach
/// it. False when there is none, when this process already has one, and on every other platform.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
pub fn attach_parent_console() -> bool {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    // SAFETY: a plain Win32 call with a constant argument; it only fails when there is no parent
    // console or this process already has one.
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) != 0 }
}

/// Attach to the console of the parent process: only Windows needs to, so this is always false.
#[cfg(not(target_os = "windows"))]
pub fn attach_parent_console() -> bool {
    false
}
