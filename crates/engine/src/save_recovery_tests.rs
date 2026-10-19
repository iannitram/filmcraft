//! Save, auto-save and recovery: the project file and the recovery journal are never replaced by
//! older or unconfirmed state (Codex review 2026-10-19: FX2, FX3, FX-S1).

use super::*;
use serde_json::json;

fn demo() -> Session {
    let mut s = Session::default();
    s.execute("file.openDemoProject", json!({})).unwrap();
    s
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("fc-sr-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn wait_for(what: &str, mut f: impl FnMut() -> bool) {
    let t0 = std::time::Instant::now();
    while !f() {
        assert!(t0.elapsed() < std::time::Duration::from_secs(10), "timed out waiting for {what}");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn start(s: &mut Session, data: &std::path::Path) {
    let mut cfg = autosave::AutosaveConfig::new(data);
    cfg.journal_debounce = std::time::Duration::from_millis(20);
    s.start_autosave(cfg).unwrap();
}

fn journaled(s: &mut Session) -> u64 {
    s.poll_persistence();
    s.persistence.as_ref().unwrap().status.journaled_revision
}

#[test]
fn a_stale_auto_save_never_replaces_a_newer_manual_save() {
    // FX2: "Auto Save also saves the current project" wrote the worker's snapshot over the project
    // file without regard to a Save made in between, so revision N could replace a saved N+1.
    let d = temp_dir("stale");
    let proj = d.join("film.fcproj").to_string_lossy().to_string();
    let mut s = demo();
    start(&mut s, &d);
    s.execute("prefs.set", json!({"key": "autoSave.saveCurrentProject", "value": true})).unwrap();
    s.execute("file.saveAs", json!({"path": proj})).unwrap();
    s.execute("sequence.addEdit", json!({"seconds": 1.5})).unwrap();
    let (stale, stale_rev) = (s.project.clone(), s.revision);
    s.execute("sequence.addEdit", json!({"seconds": 2.5})).unwrap();
    s.execute("file.save", json!({})).unwrap();
    let saved = std::fs::read(&proj).unwrap();
    // the worker still holds (or only now gets) the older revision and auto-saves it
    let per = s.persistence.as_ref().unwrap();
    per.send_dirty(stale, stale_rev, Some(proj.clone()));
    per.auto_save_now().unwrap();
    assert_eq!(std::fs::read(&proj).unwrap(), saved, "an older auto-save replaced the newer Save");
    s.shutdown();
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn recover_keeps_the_original_unless_the_new_journal_holds_it() {
    // FX3: Recover deleted the dead session's snapshot after flush() without knowing whether its own
    // journal had written the recovered state; with journalling off it wrote nothing at all.
    let d = temp_dir("recover-keep");
    let proj = d.join("film.fcproj").to_string_lossy().to_string();
    let mut a = demo();
    start(&mut a, &d);
    a.execute("file.saveAs", json!({"path": proj})).unwrap();
    a.execute("sequence.addEdit", json!({"seconds": 1.5})).unwrap();
    let rev = a.revision;
    wait_for("journal", || journaled(&mut a) == rev);
    a.persistence.take().unwrap().simulate_crash();

    let mut c = Session::default();
    start(&mut c, &d);
    c.execute("prefs.set", json!({"key": "autoSave.recoveryJournal", "value": false})).unwrap();
    assert_eq!(c.recovery_candidates().len(), 1);
    c.execute("file.recover", json!({})).unwrap();
    c.shutdown();

    let mut e = Session::default();
    start(&mut e, &d);
    assert_eq!(e.recovery_candidates().len(), 1, "the only copy of the unsaved changes was deleted");
    e.shutdown();
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_snapshot_newer_than_its_metadata_is_not_tied_to_that_metadatas_file() {
    // FX-S1: the snapshot and session.json are replaced one after the other. A crash between the
    // two leaves project B's snapshot under project A's path; recovering it and saving then
    // overwrote A.
    let d = temp_dir("meta-mismatch");
    let proj = d.join("a.fcproj").to_string_lossy().to_string();
    let mut a = demo();
    start(&mut a, &d);
    a.execute("file.saveAs", json!({"path": proj})).unwrap();
    a.execute("sequence.addEdit", json!({"seconds": 1.5})).unwrap();
    let rev = a.revision;
    wait_for("journal", || journaled(&mut a) == rev);
    let dir = a.persistence.as_ref().unwrap().session_dir.clone();
    a.persistence.take().unwrap().simulate_crash();
    // the next journal write got as far as the snapshot (another project) before the crash
    let b = filmcraft_project::Project::new("B");
    std::fs::write(dir.join("snapshot.fcproj"), filmcraft_format::encode(&b, false)).unwrap();
    let found = autosave::scan_candidates(&d, None);
    assert_eq!(found.len(), 1, "the snapshot is still offered");
    assert_eq!(found[0].meta.project_path, None, "B's snapshot was tied to A's file");
    let _ = std::fs::remove_dir_all(&d);
}
