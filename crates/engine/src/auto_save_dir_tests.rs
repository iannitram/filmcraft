//! The default Auto-Save folder never leads outside the project folder (Codex review 2026-10-19:
//! FX4).

use super::*;
use serde_json::json;

#[cfg(unix)]
#[test]
fn a_linked_auto_save_folder_outside_the_project_is_not_used() {
    // A project folder whose `Auto-Save` is a link to another folder: auto-save wrote there and
    // rotation deleted that folder's files matching the project's name pattern.
    let d = std::env::temp_dir().join(format!("fc-fx4-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let (project_dir, outside, data) = (d.join("project"), d.join("outside"), d.join("data"));
    for p in [&project_dir, &outside, &data] {
        std::fs::create_dir_all(p).unwrap();
    }
    std::os::unix::fs::symlink(&outside, project_dir.join("Auto-Save")).unwrap();
    // someone else's file in the outside folder that matches the auto-save name pattern
    let victim = outside.join("film-2000-01-01_00-00-00.fcproj");
    std::fs::write(&victim, b"not ours").unwrap();

    let proj = project_dir.join("film.fcproj").to_string_lossy().to_string();
    let mut s = Session::default();
    s.execute("file.openDemoProject", json!({})).unwrap();
    let mut cfg = autosave::AutosaveConfig::new(&data);
    cfg.journal_debounce = std::time::Duration::from_millis(20);
    s.start_autosave(cfg).unwrap();
    s.execute("prefs.set", json!({"key": "autoSave.maxVersions", "value": 1})).unwrap();
    s.execute("file.saveAs", json!({"path": proj})).unwrap();
    s.execute("sequence.addEdit", json!({"seconds": 1.5})).unwrap();
    s.sync_persistence();
    let written = s.persistence.as_ref().unwrap().auto_save_now().unwrap().expect("an auto-save");
    assert!(victim.is_file(), "rotation deleted a file outside the project folder");
    assert!(!written.starts_with(&outside), "auto-save wrote outside the project folder: {}", written.display());
    s.shutdown();
    let _ = std::fs::remove_dir_all(&d);
}
