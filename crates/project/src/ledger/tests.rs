use super::*;
use crate::{ItemKind, Label, MediaClip};
use filmcraft_media::{DemoScene, MediaSource, generators::GeneratorSource};

fn file(p: &mut Project, path: &str, bin: Option<BinId>) -> ItemId {
    let info = GeneratorSource::demo(DemoScene::OceanSunset).info().clone();
    p.add_item(
        path,
        Label::Iris,
        ItemKind::Media(Box::new(MediaClip {
            media: MediaRef::File { path: path.into() },
            info,
            interpret: Default::default(),
            mark_in: None,
            mark_out: None,
            markers: vec![],
            offline: false,
            proxy: None,
            identity: None,
        })),
        bin,
    )
}

fn label(class: SourceClass) -> SourceLabel {
    SourceLabel { class, citation: None }
}

#[test]
fn item_nearest_bin_folder_and_unknown_precedence_is_live() {
    let mut p = Project::new("Sources");
    let parent = p.add_bin("Borrowed", None);
    let child = p.add_bin("Selects", Some(parent));
    let item = file(&mut p, "media/mine/shot.mov", Some(child));
    assert_eq!(p.source_label(item), Some(label(SourceClass::Unknown)));
    p.source_ledger.rules.push(SourceRule { folder: "media/mine".into(), kind: SourceRuleKind::Folder, label: label(SourceClass::Mine) });
    assert_eq!(p.source_label(item), Some(label(SourceClass::Mine)));
    let borrowed = SourceLabel { class: SourceClass::BorrowedFootage, citation: Some("Example Show, S02E03, 2024".into()) };
    p.source_ledger.bins.insert(parent, borrowed.clone());
    assert_eq!(p.source_label(item), Some(borrowed));
    p.source_ledger.bins.insert(child, label(SourceClass::BorrowedMusic));
    assert_eq!(p.source_label(item), Some(label(SourceClass::BorrowedMusic)));
    p.source_ledger.items.insert(item, label(SourceClass::Unknown));
    p.source_ledger.bins.insert(child, label(SourceClass::Mine));
    assert_eq!(p.source_label(item), Some(label(SourceClass::Unknown)));
    p.source_ledger.items.remove(&item);
    assert_eq!(p.source_label(item), Some(label(SourceClass::Mine)));
    p.move_to_bin(&[item], None).unwrap();
    p.source_ledger.rules.clear();
    assert_eq!(p.source_label(item), Some(label(SourceClass::Unknown)));
    assert_eq!(p.source_label(ItemId(999)), None);
}

#[test]
fn folder_rules_use_components_and_specificity_including_library_roots() {
    let mut p = Project::new("Rules");
    // Even a malformed library rule read from a project cannot claim library footage is mine.
    p.source_ledger.rules.push(SourceRule { folder: "media".into(), kind: SourceRuleKind::LibraryIndex, label: label(SourceClass::Mine) });
    p.source_ledger.rules.push(SourceRule { folder: "media/mine/".into(), kind: SourceRuleKind::Folder, label: label(SourceClass::Mine) });
    for (path, class) in [
        ("media/mine/voice.wav", SourceClass::Mine),
        (r"media\mine\graphic.png", SourceClass::Mine),
        ("media//mine/./graphic.png", SourceClass::Mine),
        ("media/mine-extra/shot.mov", SourceClass::BorrowedFootage),
        ("media/show/shot.mov", SourceClass::BorrowedFootage),
        ("media-extra/shot.mov", SourceClass::Unknown),
        ("media/mine/../show/shot.mov", SourceClass::Unknown),
    ] {
        let item = file(&mut p, path, None);
        assert_eq!(p.source_label(item), Some(label(class)), "{path}");
    }
    for (path, expected) in
        [("media/./mine/", Some("media/mine")), (r"C:\media\mine\", Some("C:/media/mine")), ("/", Some("/")), ("../media", None), ("", None), ("media\0", None)]
    {
        assert_eq!(folder_key(path).as_deref(), expected);
    }
}

#[test]
fn additive_migration_roundtrip_and_retained_ids() {
    let mut p = Project::new("Old");
    let item = file(&mut p, "media/shot.mov", None);
    let mut old = serde_json::to_value(&p).unwrap();
    old.as_object_mut().unwrap().remove("source_ledger");
    let mut loaded: Project = serde_json::from_value(old).unwrap();
    assert!(loaded.source_ledger.is_empty());
    assert_eq!(loaded.source_label(item), Some(SourceLabel::default()));
    loaded.source_ledger.items.insert(item, SourceLabel { class: SourceClass::Mine, citation: Some("Original work, 2026".into()) });
    loaded.source_ledger.bins.insert(BinId(500), label(SourceClass::BorrowedMusic));
    loaded.source_ledger.items.insert(ItemId(600), label(SourceClass::Unknown));
    let roundtrip = Project::from_json(&loaded.to_json()).unwrap();
    assert_eq!(roundtrip, loaded);
    assert_eq!(roundtrip.max_id(), 600, "deleted source owners still reserve their ids");
}

#[test]
fn cyclic_subclip_and_malformed_ledger_return_without_panicking() {
    let mut p = Project::new("Invalid");
    let item = p.add_item("Cycle", Label::Iris, ItemKind::Subclip { parent: ItemId(1), range: Default::default(), restrict_trims: false }, None);
    assert_eq!(std::panic::catch_unwind(|| p.source_label(item)).unwrap(), Some(SourceLabel::default()));
    for ledger in [serde_json::json!({"items":{"1":{"class":"invalid"}}}), serde_json::json!({"rules":[{"folder":7}]}), serde_json::json!(7)] {
        let mut value = serde_json::to_value(&p).unwrap();
        value["source_ledger"] = ledger;
        let parsed = std::panic::catch_unwind(|| serde_json::from_value::<Project>(value));
        assert!(parsed.unwrap().is_err());
    }
}

#[test]
fn source_ledger_json_mutations_do_not_panic() {
    let mut p = Project::new("Mutation");
    let item = file(&mut p, "media/shot.mov", None);
    p.source_ledger.items.insert(item, label(SourceClass::BorrowedFootage));
    p.source_ledger.rules.push(SourceRule { folder: "media".into(), kind: SourceRuleKind::LibraryIndex, label: label(SourceClass::BorrowedFootage) });
    let bytes = serde_json::to_vec(&p).unwrap();
    for at in 0..bytes.len() {
        assert!(std::panic::catch_unwind(|| serde_json::from_slice::<Project>(&bytes[..at])).is_ok());
        let mut flipped = bytes.clone();
        flipped[at] ^= 0x80;
        assert!(std::panic::catch_unwind(|| serde_json::from_slice::<Project>(&flipped)).is_ok());
    }
}
