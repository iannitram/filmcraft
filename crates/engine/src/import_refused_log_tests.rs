//! Ours: a refused import is in the app's own log (Legion Go 2026-10-19: two damaged `.prproj`
//! autosaves were refused with the log silent; the reason reached only the command's reply).

use serde_json::json;

use crate::Session;

#[test]
fn a_refused_premiere_project_is_logged_at_warn_by_its_file_name() {
    let _log = craft_log::testing::capture();
    let dir = std::env::temp_dir().join(format!("filmcraft-import-refused-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Broken autosave.prproj");
    // a gzip header and nothing after it: the shape of an autosave cut off mid-write
    std::fs::write(&path, [0x1f, 0x8b, 0x08, 0x00]).unwrap();
    let mut s = Session::default();
    let e = s.execute("file.import", json!({"paths": [path.to_string_lossy()]})).unwrap_err().to_string();
    assert!(e.contains("Broken autosave.prproj"), "{e}");
    let lines: Vec<String> = craft_log::testing::captured().into_iter().filter(|l| l.contains("import refused:")).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains(" WARN "), "{}", lines[0]);
    assert!(lines[0].contains("import refused: Broken autosave.prproj: "), "{}", lines[0]);
    assert!(!lines[0].contains(&*dir.to_string_lossy()), "the folder stays out of the log: {}", lines[0]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_good_import_logs_no_refusal() {
    let _log = craft_log::testing::capture();
    let dir = std::env::temp_dir().join(format!("filmcraft-import-ok-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cues.srt");
    std::fs::write(&path, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
    let mut s = Session::default();
    s.execute("file.import", json!({"paths": [path.to_string_lossy()]})).unwrap();
    assert!(!craft_log::testing::captured().iter().any(|l| l.contains("import refused:")));
    let _ = std::fs::remove_dir_all(&dir);
}
