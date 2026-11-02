use crate::{EngineError, Services, Session};
use filmcraft_project::ItemId;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex, PoisonError},
};

/// Imports and save/reopen go through the real commands, with all files held in memory.
#[derive(Default)]
struct MemoryFiles(Mutex<BTreeMap<String, Vec<u8>>>);

impl Services for MemoryFiles {
    fn read_file(&self, path: &str) -> io::Result<Vec<u8>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).get(path).cloned().ok_or_else(|| io::ErrorKind::NotFound.into())
    }
    fn write_file(&self, path: &str, bytes: &[u8]) -> io::Result<()> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).insert(path.into(), bytes.to_vec());
        Ok(())
    }
}

fn setup() -> (Session, Arc<MemoryFiles>) {
    let files = Arc::new(MemoryFiles::default());
    let wav = filmcraft_media::wav::write_wav16(&[0.0; 96], 2, 48_000);
    for path in ["media/mine/voice.wav", "media/show/clip.wav", "media/music/song.wav", "media/other.wav"] {
        files.write_file(path, &wav).unwrap();
    }
    (Session::new(files.clone()), files)
}

fn import(s: &mut Session, path: &str, bin: Option<u64>) -> u64 {
    let mut params = json!({"paths":[path], "imageSequence":false});
    if let Some(bin) = bin {
        params["bin"] = json!(bin);
    }
    let out = s.execute("file.import", params).unwrap();
    out["items"][0].as_u64().unwrap_or_else(|| panic!("import failed: {out}"))
}

fn source(s: &mut Session, item: u64) -> Value {
    s.execute("ledger.inspectSources", json!({"item":item})).unwrap()["sources"][0].clone()
}

#[test]
fn commands_inherit_override_clear_and_undo_redo_without_a_sequence() {
    let (mut s, _) = setup();
    let bin = s.execute("file.newBin", json!({"name":"Selects"})).unwrap()["bin"].as_u64().unwrap();
    let item = import(&mut s, "media/show/clip.wav", Some(bin));
    assert!(s.is_enabled("ledger.setClass"));
    assert_eq!(source(&mut s, item)["class"], "unknown");
    s.execute("ledger.setClass", json!({"bin":bin,"class":"borrowedFootage","citation":"Example Show, S01E02, 2024"})).unwrap();
    assert_eq!(source(&mut s, item)["citation"], "Example Show, S01E02, 2024");
    s.execute("ledger.setClass", json!({"item":item,"class":"mine"})).unwrap();
    assert!(source(&mut s, item)["citation"].is_null());
    s.execute("ledger.setClass", json!({"bin":bin,"class":"borrowedMusic","citation":null})).unwrap();
    assert_eq!(source(&mut s, item)["class"], "mine");
    s.execute("ledger.setClass", json!({"item":item,"class":null})).unwrap();
    assert_eq!(source(&mut s, item)["class"], "borrowedMusic");
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(source(&mut s, item)["class"], "mine");
    s.execute("edit.redo", json!({})).unwrap();
    assert_eq!(source(&mut s, item)["class"], "borrowedMusic");
    let before = (s.project.clone(), s.revision, s.history.undo.len(), s.history.redo.len(), s.journal.len());
    let report = s.execute("ledger.inspectSources", json!({})).unwrap();
    assert_eq!(report["sources"].as_array().unwrap().len(), 1);
    assert_eq!(before, (s.project.clone(), s.revision, s.history.undo.len(), s.history.redo.len(), s.journal.len()));
}

#[test]
fn folder_rules_apply_to_later_imports_and_survive_save_reopen() {
    let (mut s, files) = setup();
    s.execute("ledger.setRule", json!({"folder":"media", "kind":"libraryIndex", "citation":"Example Film, 2023"})).unwrap();
    s.execute("ledger.setRule", json!({"folder":"media/mine/", "class":"mine"})).unwrap();
    s.execute("ledger.setRule", json!({"folder":"media/music", "class":"borrowedMusic", "citation":"Example Album, 2022"})).unwrap();
    let mine = import(&mut s, "media/mine/voice.wav", None);
    let footage = import(&mut s, "media/show/clip.wav", None);
    let music = import(&mut s, "media/music/song.wav", None);
    assert_eq!(source(&mut s, mine)["class"], "mine");
    assert_eq!(source(&mut s, footage)["class"], "borrowedFootage");
    assert_eq!(source(&mut s, music)["class"], "borrowedMusic");
    let bin = s.execute("file.newBin", json!({"name":"Originals"})).unwrap()["bin"].as_u64().unwrap();
    s.execute("ledger.setClass", json!({"bin":bin,"class":"mine"})).unwrap();
    s.execute("ledger.setClass", json!({"item":footage,"class":"unknown", "citation":"Needs review"})).unwrap();
    s.execute("file.saveAs", json!({"path":"sources.fcproj"})).unwrap();
    let expected = s.execute("ledger.inspectSources", json!({})).unwrap();
    let mut reopened = Session::new(files);
    reopened.execute("file.open", json!({"path":"sources.fcproj"})).unwrap();
    assert_eq!(reopened.execute("ledger.inspectSources", json!({})).unwrap(), expected);
    let later = import(&mut reopened, "media/other.wav", None);
    assert_eq!(source(&mut reopened, later)["class"], "borrowedFootage");
    assert_eq!(source(&mut reopened, later)["citation"], "Example Film, 2023");
    let binned = import(&mut reopened, "media/other.wav", Some(bin));
    assert_eq!(source(&mut reopened, binned)["class"], "mine");
    reopened.execute("ledger.setRule", json!({"folder":"media/music/", "class":null})).unwrap();
    assert_eq!(source(&mut reopened, music)["class"], "borrowedFootage");
    reopened.execute("edit.undo", json!({})).unwrap();
    assert_eq!(source(&mut reopened, music)["class"], "borrowedMusic");
    reopened.execute("edit.redo", json!({})).unwrap();
    assert_eq!(source(&mut reopened, music)["class"], "borrowedFootage");
}

#[test]
fn old_project_envelopes_open_with_unknown_sources() {
    let (mut s, _) = setup();
    let item = import(&mut s, "media/show/clip.wav", None);
    let mut old: Value = serde_json::from_slice(&filmcraft_format::encode(&s.project, false)).unwrap();
    old["project"].as_object_mut().unwrap().remove("source_ledger");
    for version in [1, filmcraft_format::SCHEMA_VERSION] {
        // Version 1 stored the project without an envelope.
        let value = if version == 1 { old["project"].clone() } else { old.clone() };
        let loaded = filmcraft_format::decode(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(loaded.project.source_label(ItemId(item)).unwrap().class, filmcraft_project::SourceClass::Unknown);
        assert!(loaded.project.source_ledger.is_empty());
    }
}

#[test]
fn hostile_parameters_leave_project_dirty_state_and_history_unchanged() {
    let (mut s, _) = setup();
    let item = import(&mut s, "media/show/clip.wav", None);
    s.execute("file.newBin", json!({"name":"Redo"})).unwrap();
    s.execute("edit.undo", json!({})).unwrap();
    let before = (s.project.clone(), s.revision, s.history.undo.len(), s.history.redo.len(), s.is_dirty(), s.journal.len());
    for (command, params) in [
        ("ledger.setClass", json!([])),
        ("ledger.setClass", json!({"item":item,"class":"invalid"})),
        ("ledger.setClass", json!({"item":item,"class":7})),
        ("ledger.setClass", json!({"item":-1,"class":"mine"})),
        ("ledger.setClass", json!({"item":1.5,"class":"mine"})),
        ("ledger.setClass", json!({"bin":u64::MAX,"class":"mine"})),
        ("ledger.setClass", json!({"item":item,"bin":0,"class":"mine"})),
        ("ledger.setClass", json!({"item":item})),
        ("ledger.setClass", json!({"item":item,"class":"mine","citation":true})),
        ("ledger.setClass", json!({"item":item,"class":"mine","citation":"x".repeat(1025)})),
        ("ledger.setClass", json!({"item":item,"class":"mine","citation":"bad\nlabel"})),
        ("ledger.setClass", json!({"item":item,"class":null,"citation":"lost"})),
        ("ledger.setRule", json!({"folder":"media/../mine","class":"mine"})),
        ("ledger.setRule", json!({"folder":"","class":"mine"})),
        ("ledger.setRule", json!({"folder":"x".repeat(8193),"class":"mine"})),
        ("ledger.setRule", json!({"folder":"media","kind":"libraryIndex","class":"mine"})),
        ("ledger.setRule", json!({"folder":"media","kind":false,"class":"mine"})),
        ("ledger.setRule", json!({"folder":[],"class":"mine"})),
        ("ledger.setRule", json!({"folder":"media"})),
        ("ledger.setRule", json!({"folder":"media","class":null,"citation":"lost"})),
        ("ledger.setRule", json!({"folder":"media","class":"mine","citation":7})),
        ("ledger.inspectSources", json!({"item":u64::MAX})),
        ("ledger.inspectSources", json!({"item":[]})),
        ("ledger.inspectSources", json!(null)),
    ] {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.execute(command, params)));
        assert!(matches!(result.unwrap(), Err(EngineError::BadParams { .. })), "{command}");
        assert_eq!(before, (s.project.clone(), s.revision, s.history.undo.len(), s.history.redo.len(), s.is_dirty(), s.journal.len()));
    }
}

#[test]
fn rule_updates_are_bounded_and_keep_or_clear_citations() {
    let (mut s, _) = setup();
    s.execute("ledger.setRule", json!({"folder":"media/./mine/", "class":"mine", "citation":"Original, 2026"})).unwrap();
    s.execute("ledger.setRule", json!({"folder":r"media\mine", "class":"unknown"})).unwrap();
    assert_eq!(s.project.source_ledger.rules.len(), 1);
    let item = import(&mut s, "media/mine/voice.wav", None);
    assert_eq!(source(&mut s, item)["citation"], "Original, 2026");
    s.execute("ledger.setRule", json!({"folder":"media/mine", "class":"mine", "citation":null})).unwrap();
    assert!(source(&mut s, item)["citation"].is_null());
    s.execute("ledger.setClass", json!({"item":item, "class":"mine", "citation":"Own recording"})).unwrap();
    s.execute("ledger.setClass", json!({"item":item, "class":"unknown"})).unwrap();
    assert_eq!(source(&mut s, item)["citation"], "Own recording");
    s.execute("ledger.setClass", json!({"item":item, "class":"mine", "citation":""})).unwrap();
    assert!(source(&mut s, item)["citation"].is_null());
    let template = s.project.source_ledger.rules[0].clone();
    Arc::make_mut(&mut s.project)
        .source_ledger
        .rules
        .extend((1..256).map(|n| filmcraft_project::SourceRule { folder: format!("folder{n}"), ..template.clone() }));
    let before = s.project.clone();
    assert!(s.execute("ledger.setRule", json!({"folder":"overflow", "class":"mine"})).is_err());
    assert_eq!(s.project, before);
    // Updating/removing a rule at the limit still works.
    s.execute("ledger.setRule", json!({"folder":"media/mine", "class":"unknown"})).unwrap();
    s.execute("ledger.setRule", json!({"folder":"media/mine", "class":null})).unwrap();
    assert_eq!(s.project.source_ledger.rules.len(), 255);
}
