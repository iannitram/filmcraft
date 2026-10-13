//! Synthetic native object graphs, authored for this importer; no user projects or Adobe assets.

use std::io::Write;

use filmcraft_geom::Vec2;
use filmcraft_interchange::{ExportOptions, Format, ImportOptions, detect, export, import_with, premiere};
use filmcraft_project::{Interpolation, ItemKind};
use filmcraft_time::{FrameRate, TICKS_PER_SECOND as TPS, Tick};

fn project_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<PremiereData Version="3">
 <Project ObjectRef="1"/>
 <Project ObjectID="1"><RootProjectItem ObjectURef="root"/></Project>
 <RootProjectItem ObjectUID="root"><ProjectItem><Name>Root</Name></ProjectItem><ProjectItemContainer><Items>
  <Item Index="0" ObjectURef="sequence-item"/><Item Index="1" ObjectURef="folder"/>
 </Items></ProjectItemContainer></RootProjectItem>
 <BinProjectItem ObjectUID="folder"><ProjectItem><Name>Footage</Name></ProjectItem><ProjectItemContainer><Items>
  <Item Index="0" ObjectURef="movie-item"/>
 </Items></ProjectItemContainer></BinProjectItem>
 <ClipProjectItem ObjectUID="movie-item"><ProjectItem><Name>Example movie</Name></ProjectItem><MasterClip ObjectURef="movie-master"/></ClipProjectItem>
 <ClipProjectItem ObjectUID="sequence-item"><ProjectItem><Name>Example sequence</Name></ProjectItem><MasterClip ObjectURef="sequence-master"/></ClipProjectItem>
 <MasterClip ObjectUID="movie-master"><Name>Example movie</Name><Clips><Clip Index="0" ObjectRef="20"/><Clip Index="1" ObjectRef="21"/></Clips></MasterClip>
 <MasterClip ObjectUID="sequence-master"><Name>Example sequence</Name><Clips><Clip Index="0" ObjectRef="24"/></Clips></MasterClip>
 <VideoClip ObjectID="20"><Clip><Source ObjectRef="22"/></Clip></VideoClip>
 <AudioClip ObjectID="21"><Clip><Source ObjectRef="23"/></Clip></AudioClip>
 <VideoMediaSource ObjectID="22"><MediaSource><Media ObjectURef="movie"/></MediaSource></VideoMediaSource>
 <AudioMediaSource ObjectID="23"><MediaSource><Media ObjectURef="movie"/></MediaSource></AudioMediaSource>
 <VideoClip ObjectID="24"><Clip><Source ObjectRef="25"/></Clip></VideoClip>
 <VideoSequenceSource ObjectID="25"><SequenceSource><Sequence ObjectURef="sequence"/></SequenceSource></VideoSequenceSource>
 <Media ObjectUID="movie"><FilePath>/media/example.mov</FilePath><VideoStream ObjectRef="40"/><AudioStream ObjectRef="41"/></Media>
 <VideoStream ObjectID="40"><FrameRate>10594584000</FrameRate><FrameRect>0,0,1920,1080</FrameRect><Duration>{twelve}</Duration></VideoStream>
 <AudioStream ObjectID="41"><FrameRate>5292000</FrameRate><Duration>{twelve}</Duration></AudioStream>
 <Sequence ObjectUID="sequence"><Name>Example sequence</Name><TrackGroups>
  <TrackGroup Index="0"><Second ObjectRef="30"/></TrackGroup><TrackGroup Index="1"><Second ObjectRef="31"/></TrackGroup>
 </TrackGroups><PersistentGroupContainer><LinkContainer><Links><Link Index="0" ObjectRef="72"/></Links></LinkContainer></PersistentGroupContainer></Sequence>
 <VideoTrackGroup ObjectID="30"><FrameRect>0,0,1920,1080</FrameRect><TrackGroup><FrameRate>10594584000</FrameRate><Tracks><Track Index="0" ObjectURef="video"/></Tracks></TrackGroup></VideoTrackGroup>
 <AudioTrackGroup ObjectID="31"><TrackGroup><FrameRate>5292000</FrameRate><Tracks><Track Index="0" ObjectURef="audio"/></Tracks></TrackGroup></AudioTrackGroup>
 <VideoClipTrack ObjectUID="video"><ClipTrack><Track><IsLocked>true</IsLocked></Track><ClipItems><TrackItems><TrackItem Index="0" ObjectRef="60"/></TrackItems></ClipItems></ClipTrack></VideoClipTrack>
 <AudioClipTrack ObjectUID="audio"><ClipTrack><Track><IsMuted>true</IsMuted></Track><ClipItems><TrackItems><TrackItem Index="0" ObjectRef="68"/></TrackItems></ClipItems></ClipTrack></AudioClipTrack>
 <VideoClipTrackItem ObjectID="60"><ClipTrackItem><TrackItem><Start>0</Start><End>{two}</End></TrackItem><SubClip ObjectRef="61"/><ComponentOwner><Components ObjectRef="63"/></ComponentOwner></ClipTrackItem></VideoClipTrackItem>
 <SubClip ObjectID="61"><Name>Example cut</Name><Clip ObjectRef="62"/></SubClip>
 <VideoClip ObjectID="62"><Clip><Source ObjectRef="22"/><InPoint>{five}</InPoint><OutPoint>{seven}</OutPoint></Clip></VideoClip>
 <VideoComponentChain ObjectID="63"><ComponentChain><Components><Component Index="0" ObjectRef="64"/></Components></ComponentChain></VideoComponentChain>
 <VideoFilterComponent ObjectID="64"><MatchName>AE.ADBE Motion</MatchName><Component><Params><Param Index="0" ObjectRef="65"/><Param Index="1" ObjectRef="66"/><Param Index="2" ObjectRef="67"/></Params></Component></VideoFilterComponent>
 <PointComponentParam ObjectID="65"><ParameterID>1</ParameterID><StartKeyframe>-91445760000000000,0.25:0.75,0,0,0,0,0,0</StartKeyframe></PointComponentParam>
 <VideoComponentParam ObjectID="66"><ParameterID>2</ParameterID><IsTimeVarying>true</IsTimeVarying><StartKeyframe>-91445760000000000,150,0,0,0,0,0,0</StartKeyframe><Keyframes>{five},150,0,0,0,0,0,0;{seven},200,0,0,0,0,0,0;</Keyframes></VideoComponentParam>
 <PointComponentParam ObjectID="67"><ParameterID>6</ParameterID><StartKeyframe>-91445760000000000,0.5:0.5,0,0,0,0,0,0</StartKeyframe></PointComponentParam>
 <AudioClipTrackItem ObjectID="68"><ClipTrackItem><TrackItem><Start>0</Start><End>{two}</End></TrackItem><SubClip ObjectRef="69"/></ClipTrackItem></AudioClipTrackItem>
 <SubClip ObjectID="69"><Name>Example audio</Name><Clip ObjectRef="70"/></SubClip>
 <AudioClip ObjectID="70"><Clip><Source ObjectRef="23"/><InPoint>{five}</InPoint><OutPoint>{seven}</OutPoint></Clip></AudioClip>
 <Link ObjectID="72"><TrackItemGroup><TrackItems><TrackItem Index="0" ObjectRef="60"/><TrackItem Index="1" ObjectRef="68"/></TrackItems></TrackItemGroup></Link>
</PremiereData>"#,
        two = 2 * TPS,
        five = 5 * TPS,
        seven = 7 * TPS,
        twelve = 12 * TPS
    )
}

fn preset_xml(kind: u32) -> String {
    let origin = 3600 * TPS;
    format!(
        r#"<PremiereData Version="3">
 <Tree ObjectID="10"><RootBin ObjectRef="11"/></Tree>
 <BinTreeItem ObjectID="11"><TreeItemBase><Name>Root</Name></TreeItemBase><Items><Item Index="0" ObjectRef="1"/></Items></BinTreeItem>
 <TreeItem ObjectID="1"><TreeItemBase><Name>Synthetic animation</Name><Data ObjectRef="2"/></TreeItemBase></TreeItem>
 <FilterPresetItem ObjectID="2"><FilterPresets><FilterPreset Index="0" ObjectRef="3"/></FilterPresets></FilterPresetItem>
 <FilterPreset ObjectID="3"><FilterMatchName>AE.ADBE Geometry2</FilterMatchName><Component ObjectRef="4"/><AnchorInPoint>{origin}</AnchorInPoint><AnchorOutPoint>{end}</AnchorOutPoint><Type>{kind}</Type><Speed>1</Speed><Description>Synthetic test</Description></FilterPreset>
 <VideoFilterComponent ObjectID="4"><MatchName>AE.ADBE Geometry2</MatchName><Component><Params><Param Index="0" ObjectRef="5"/><Param Index="1" ObjectRef="6"/></Params></Component></VideoFilterComponent>
 <VideoComponentParam ObjectID="5"><ParameterID>3</ParameterID><IsTimeVarying>true</IsTimeVarying><StartKeyframe>-91445760000000000,100,0,0,0,0,0,0</StartKeyframe><Keyframes>{first},150,0,0,0,0,0,0;{last},100,0,0,0,0,0,0;</Keyframes></VideoComponentParam>
 <PointComponentParam ObjectID="6"><ParameterID>2</ParameterID><StartKeyframe>-91445760000000000,0.25:0.75,0,0,0,0,0,0</StartKeyframe></PointComponentParam>
</PremiereData>"#,
        end = origin + 3 * TPS,
        first = origin + TPS,
        last = origin + 2 * TPS
    )
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn premiere_native_project_keeps_cuts_links_bins_and_motion() {
    let xml = project_xml();
    assert_eq!(detect(xml.as_bytes(), None), Some(Format::PremiereProject));
    assert_eq!(Format::from_extension(".PRPROJ"), Some(Format::PremiereProject));
    assert!(!Format::EXPORTABLE.contains(&Format::PremiereProject));
    let (imported, report) = import_with(xml.as_bytes(), Format::PremiereProject, &ImportOptions::default()).unwrap();
    assert!(!report.has_warnings(), "{report}");
    assert_eq!(imported.sequences.len(), 1);
    let id = imported.sequences[0];
    let sequence = imported.project.sequence(id).unwrap();
    assert_eq!(sequence.settings.frame_rate, FrameRate::FPS_23_976);
    assert_eq!(sequence.settings.sample_rate, 48_000);
    assert_eq!((sequence.settings.width, sequence.settings.height), (1920, 1080));
    assert!(sequence.video_tracks[0].locked);
    assert!(sequence.audio_tracks[0].muted);
    assert!(!sequence.audio_tracks[0].enabled);
    let video = &sequence.video_tracks[0].items[0];
    let audio = &sequence.audio_tracks[0].items[0];
    assert_eq!((video.start, video.duration, video.source_in), (Tick::ZERO, Tick(2 * TPS), Tick(5 * TPS)));
    assert_eq!(video.item, audio.item);
    assert!(video.link.is_some());
    assert_eq!(video.link, audio.link);
    let media = imported.project.item(video.item).unwrap().as_media().unwrap();
    assert!(media.offline);
    assert!(matches!(&media.media,filmcraft_project::MediaRef::File{path} if path=="/media/example.mov"));
    assert!(imported.project.root.parent_of(video.item).is_some());
    let motion = video.effect("motion").unwrap();
    assert_eq!(motion.vec2_at("position", Tick(5 * TPS)), Vec2::new(480.0, 810.0));
    assert_eq!(motion.vec2_at("anchor", Tick(5 * TPS)), Vec2::new(960.0, 540.0));
    let scale = motion.param("scale").unwrap();
    assert_eq!(scale.keyframes.iter().map(|k| k.time).collect::<Vec<_>>(), [Tick(5 * TPS), Tick(7 * TPS)]);
    assert_eq!(scale.f64_at(Tick(6 * TPS)), 175.0);
    assert!(export(&imported.project, id, Format::PremiereProject, &ExportOptions::default()).is_err());
}

#[test]
fn premiere_gzip_and_utf16_decode_the_same_project() {
    let xml = project_xml();
    let compressed = gzip(xml.as_bytes());
    assert_eq!(detect(&compressed, None), Some(Format::PremiereProject));
    let (plain, _) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    let (compressed, _) = premiere::import_project(&compressed, &ImportOptions::default()).unwrap();
    assert_eq!(plain.project, compressed.project);
    let mut utf16 = vec![0xff, 0xfe];
    for c in xml.encode_utf16() {
        utf16.extend_from_slice(&c.to_le_bytes())
    }
    let (utf16, _) = premiere::import_project(&utf16, &ImportOptions::default()).unwrap();
    assert_eq!(plain.project, utf16.project);
}

#[test]
fn premiere_presets_preserve_all_three_timing_modes_and_relative_animation() {
    for (kind, expected) in [(0, premiere::PresetTiming::Scale), (1, premiere::PresetTiming::AnchorToIn), (2, premiere::PresetTiming::AnchorToOut)] {
        let (presets, report) = premiere::import_presets(preset_xml(kind).as_bytes()).unwrap();
        assert!(!report.has_warnings(), "{report}");
        assert_eq!(presets.len(), 1);
        let p = &presets[0];
        assert_eq!(p.name, "Synthetic animation");
        assert_eq!(p.timing, expected);
        assert_eq!(p.source_duration, Tick(3 * TPS));
        let effect = &p.effects[0];
        assert_eq!(effect.effect, "transform");
        assert_eq!(effect.vec2_at("position", Tick::ZERO), Vec2::new(480.0, 810.0));
        let scale = effect.param("scale_height").unwrap();
        assert_eq!(scale.keyframes.iter().map(|k| k.time).collect::<Vec<_>>(), [Tick(TPS), Tick(2 * TPS)]);
        assert_eq!(scale.f64_at(Tick(TPS + TPS / 2)), 125.0);
    }
}

#[test]
fn premiere_unknown_effects_and_nonrepresentable_easing_have_named_reports() {
    let xml = preset_xml(0).replace("AE.ADBE Geometry2", "AE.Example.Unsupported");
    let (presets, report) = premiere::import_presets(xml.as_bytes()).unwrap();
    assert!(presets.is_empty());
    assert!(report.mentions("AE.Example.Unsupported"));
    assert!(report.mentions("Synthetic animation"));
    let xml = preset_xml(0).replace(",150,0,0,0,0,0,0;", ",150,0,0,0,0.25,20,0.5;");
    let (presets, report) = premiere::import_presets(xml.as_bytes()).unwrap();
    assert!(report.mentions("approximated"));
    let first = &presets[0].effects[0].param("scale_height").unwrap().keyframes[0];
    assert_eq!(first.interp, Interpolation::Bezier);
    assert_eq!(first.out_influence, 0.5);
    assert_eq!(first.in_influence, 0.25);
}

#[test]
fn premiere_native_nested_sequence_and_cycle_detection() {
    let extra = r#"<Sequence ObjectUID="nested"><Name>Nested</Name><TrackGroups><TrackGroup><Second ObjectRef="90"/></TrackGroup></TrackGroups></Sequence>
    <VideoTrackGroup ObjectID="90"><FrameRect>0,0,1280,720</FrameRect><TrackGroup><FrameRate>10584000000</FrameRate><Tracks/></TrackGroup></VideoTrackGroup>
    <VideoSequenceSource ObjectID="91"><SequenceSource><Sequence ObjectURef="nested"/></SequenceSource></VideoSequenceSource>"#;
    let xml = project_xml()
        .replace("</PremiereData>", &format!("{extra}</PremiereData>"))
        .replace("<VideoClip ObjectID=\"62\"><Clip><Source ObjectRef=\"22\"/>", "<VideoClip ObjectID=\"62\"><Clip><Source ObjectRef=\"91\"/>");
    let (imported, _) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    assert_eq!(imported.sequences.len(), 2);
    let sequence = imported.project.sequence(imported.sequences[0]).unwrap();
    let clip = &sequence.video_tracks[0].items[0];
    assert!(matches!(imported.project.item(clip.item).unwrap().kind, ItemKind::Sequence(_)));
    let cyclic =
        project_xml().replace("<VideoClip ObjectID=\"62\"><Clip><Source ObjectRef=\"22\"/>", "<VideoClip ObjectID=\"62\"><Clip><Source ObjectRef=\"25\"/>");
    let error = premiere::import_project(cyclic.as_bytes(), &ImportOptions::default()).unwrap_err();
    assert!(error.to_string().contains("cyclic"));
}

#[test]
fn premiere_hostile_truncation_mutation_references_and_numbers_never_panic() {
    let xml = project_xml();
    let compressed = gzip(xml.as_bytes());
    for cut in 0..compressed.len() {
        let r = std::panic::catch_unwind(|| premiere::import_project(&compressed[..cut], &ImportOptions::default()));
        assert!(r.is_ok(), "panic at truncation {cut}");
        assert!(r.unwrap().is_err(), "accepted gzip truncation {cut}");
    }
    for index in (0..xml.len()).step_by(13) {
        let mut data = xml.as_bytes().to_vec();
        data[index] ^= 0x7f;
        assert!(std::panic::catch_unwind(|| premiere::import_project(&data, &ImportOptions::default())).is_ok());
    }
    for invalid in [
        xml.replace("ObjectID=\"41\"", "ObjectID=\"40\""),
        xml.replace("ObjectRef=\"63\"", "ObjectRef=\"missing\""),
        xml.replace("<FrameRate>10594584000</FrameRate>", "<FrameRate>0</FrameRate>"),
        xml.replace("<FrameRect>0,0,1920,1080</FrameRect>", "<FrameRect>0,0,9223372036854775807,1080</FrameRect>"),
        xml.replace("<FrameRect>0,0,1920,1080</FrameRect>", "<FrameRect>0,0,1920,1080,0,0</FrameRect>"),
        xml.replace("<InPoint>1270080000000</InPoint>", "<InPoint>9223372036854775807</InPoint>"),
        xml.replace("ObjectURef=\"movie-item\"", "ObjectURef=\"root\""),
    ] {
        assert!(premiere::import_project(invalid.as_bytes(), &ImportOptions::default()).is_err());
    }
    for invalid in [
        preset_xml(0).replace(",150,", ",NaN,"),
        preset_xml(99),
        preset_xml(0).replace(&format!("<AnchorOutPoint>{}</AnchorOutPoint>", 3603 * TPS), "<AnchorOutPoint>-1</AnchorOutPoint>"),
        preset_xml(0).replace("0.25:0.75", "Infinity:0.75"),
    ] {
        let r = std::panic::catch_unwind(|| premiere::import_presets(invalid.as_bytes()));
        assert!(r.is_ok());
        assert!(r.unwrap().is_err());
    }
    let entity = r#"<!DOCTYPE PremiereData [<!ENTITY x "boom">]><PremiereData Version="3"><Project ObjectID="1"><Name>&x;</Name></Project></PremiereData>"#;
    assert!(premiere::import_project(entity.as_bytes(), &ImportOptions::default()).is_err());
}

#[test]
fn premiere_hostile_gzip_expansion_is_bounded() {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let block = [b' '; 8192];
    for _ in 0..(premiere::MAX_DOCUMENT_BYTES / block.len() + 1) {
        encoder.write_all(&block).unwrap();
    }
    let compressed = encoder.finish().unwrap();
    let error = premiere::import_project(&compressed, &ImportOptions::default()).unwrap_err();
    assert!(error.to_string().contains("512 MiB"));
}

#[test]
fn premiere_hostile_shared_text_fields_are_bounded_before_expansion() {
    let xml = project_xml();
    for invalid in [
        xml.replace("<Name>Example cut</Name>", &format!("<Name>{}</Name>", "x".repeat(4097))),
        xml.replace("ObjectUID=\"movie\"", &format!("ObjectUID=\"{}\"", "x".repeat(257))),
        xml.replace("<FrameRate>10594584000</FrameRate>", &format!("<FrameRate>{}10594584000</FrameRate>", "0".repeat(129))),
    ] {
        let result = std::panic::catch_unwind(|| premiere::import_project(invalid.as_bytes(), &ImportOptions::default()));
        assert!(result.is_ok());
        assert!(result.unwrap().unwrap_err().to_string().contains("exceeds"));
    }
    let valid = xml.replace("<Name>Example cut</Name>", &format!("<Name>{}</Name>", "x".repeat(4096)));
    assert!(premiere::import_project(valid.as_bytes(), &ImportOptions::default()).is_ok());
}

#[test]
fn premiere_preset_tree_preserves_custom_bins_and_ignores_unreferenced_items() {
    let xml=preset_xml(0).replace("<Item Index=\"0\" ObjectRef=\"1\"/>","<Item Index=\"0\" ObjectRef=\"12\"/>")
        .replace("</PremiereData>",r#"<BinTreeItem ObjectID="12"><TreeItemBase><Name>Custom folder</Name></TreeItemBase><Items><Item ObjectRef="1"/></Items></BinTreeItem><TreeItem ObjectID="13"><TreeItemBase><Name>Orphan</Name><Data ObjectRef="2"/></TreeItemBase></TreeItem></PremiereData>"#);
    let (presets, _) = premiere::import_presets(xml.as_bytes()).unwrap();
    assert_eq!(presets.len(), 1);
    assert_eq!(presets[0].name, "Custom folder/Synthetic animation");
    let cyclic = xml.replace("<Item ObjectRef=\"1\"/>", "<Item ObjectRef=\"11\"/>");
    assert!(premiere::import_presets(cyclic.as_bytes()).is_err());
}

#[test]
fn premiere_project_without_sequences_imports_its_media_and_bins() {
    let xml = project_xml()
        .replace("<Item Index=\"0\" ObjectURef=\"sequence-item\"/>", "")
        .replace("<Sequence ObjectUID=", "<UnusedSequence ObjectUID=")
        .replace("</Sequence>", "</UnusedSequence>");
    let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    assert!(imported.sequences.is_empty());
    assert_eq!(imported.project.items.len(), 1);
    assert!(report.mentions("no sequences"));
}

#[test]
fn premiere_native_audio_transition_is_a_timeline_transition() {
    let transition = format!(
        r#"<AudioTransitionTrackItem ObjectID="80"><TransitionTrackItem><TrackItem><Start>{start}</Start><End>{end}</End></TrackItem><MatchName>Constant Power</MatchName><Alignment>{duration}</Alignment><HasOutgoingClip>true</HasOutgoingClip><HasIncomingClip>false</HasIncomingClip></TransitionTrackItem></AudioTransitionTrackItem>"#,
        start = TPS + TPS / 2,
        end = 2 * TPS,
        duration = TPS / 2
    );
    let xml = project_xml()
        .replace(
            "</ClipItems></ClipTrack></AudioClipTrack>",
            "</ClipItems><TransitionItems><TrackItems><TrackItem ObjectRef=\"80\"/></TrackItems></TransitionItems></ClipTrack></AudioClipTrack>",
        )
        .replace("</PremiereData>", &format!("{transition}</PremiereData>"));
    let (imported, _) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    let track = &imported.project.sequence(imported.sequences[0]).unwrap().audio_tracks[0];
    assert_eq!(track.transitions.len(), 1);
    let transition = &track.transitions[0];
    assert_eq!(transition.effect.effect, "constant_power");
    assert_eq!(transition.start, Tick(TPS + TPS / 2));
    assert_eq!(transition.duration, Tick(TPS / 2));
    assert_eq!(transition.from, Some(track.items[0].id));
    assert!(transition.to.is_none());
}

#[test]
fn premiere_native_volume_converts_linear_gain_to_decibels() {
    let xml = preset_xml(0)
        .replace("VideoFilterComponent", "AudioFilterComponent")
        .replace("<MatchName>AE.ADBE Geometry2</MatchName><Component>", "<FilterMatchName>Internal Volume Stereo</FilterMatchName><AudioComponent><Component>")
        .replace("</Component></AudioFilterComponent>", "</Component></AudioComponent></AudioFilterComponent>")
        .replace("<Param Index=\"1\" ObjectRef=\"6\"/>", "")
        .replace("<ParameterID>3</ParameterID>", "<ParameterID>2</ParameterID>")
        .replace("<IsTimeVarying>true</IsTimeVarying>", "<IsTimeVarying>false</IsTimeVarying>")
        .replace(",100,0,0,0,0,0,0</StartKeyframe>", ",0.1,0,0,0,0,0,0</StartKeyframe>");
    let (presets, _) = premiere::import_presets(xml.as_bytes()).unwrap();
    assert_eq!(presets[0].effects[0].effect, "volume");
    assert!((presets[0].effects[0].f64_at("level", Tick::ZERO) + 20.0).abs() < 1e-9);
}

/// A project the size real long-running ones reach (tens of MiB of XML, millions of nodes) is
/// inside the limits; the old 64 MiB / two-million-node bounds refused 93 of 291 real projects.
#[test]
fn premiere_project_far_larger_than_64_mib_imports() {
    let filler = "<ArbVideoComponentParam ObjectID=\"9000000\"><Name>pad</Name><StartKeyframe>0,0,0,0,0,0,0,0</StartKeyframe></ArbVideoComponentParam>\n";
    let mut pad = String::with_capacity(80 * 1024 * 1024);
    let mut id = 9_000_000_u32;
    while pad.len() < 72 * 1024 * 1024 {
        pad.push_str(&filler.replace("9000000", &id.to_string()));
        id += 1;
    }
    assert!(id - 9_000_000 > 400_000, "more objects than the old 200,000 limit");
    let xml = project_xml().replace("</PremiereData>", &format!("{pad}</PremiereData>"));
    assert!(xml.len() > 64 * 1024 * 1024);
    let (imported, _) = premiere::import_project(&gzip(xml.as_bytes()), &ImportOptions::default()).unwrap();
    assert_eq!(imported.sequences.len(), 1);
}

#[test]
fn premiere_sequence_pixel_aspect_is_read() {
    let sd = |par: &str| {
        project_xml().replace(
            "<VideoTrackGroup ObjectID=\"30\"><FrameRect>0,0,1920,1080</FrameRect>",
            &format!("<VideoTrackGroup ObjectID=\"30\"><FrameRect>0,0,720,480</FrameRect><PixelAspectRatio>{par}</PixelAspectRatio>"),
        )
    };
    for (text, par) in [("40,33", (40, 33)), ("10,11", (10, 11)), ("1,1", (1, 1)), (" 4 , 3 ", (4, 3)), ("80,66", (40, 33))] {
        let (imported, report) = premiere::import_project(sd(text).as_bytes(), &ImportOptions::default()).unwrap();
        let settings = &imported.project.sequence(imported.sequences[0]).unwrap().settings;
        assert_eq!((settings.width, settings.height, settings.par), (720, 480, par), "{text}");
        assert!(!report.to_string().contains("pixel aspect"), "{report}");
    }
    // a value that is no ratio is reported and leaves square pixels: the project still opens
    for garbage in ["0,0", "40", "1,0", "wide", "1000,1", "40,33,2"] {
        let (imported, report) = premiere::import_project(sd(garbage).as_bytes(), &ImportOptions::default()).unwrap();
        assert_eq!(imported.project.sequence(imported.sequences[0]).unwrap().settings.par, (1, 1), "{garbage}");
        assert!(report.to_string().contains("pixel aspect ratio"), "{garbage}: {report}");
    }
    // no PixelAspectRatio at all: square, nothing to report
    let (imported, report) = premiere::import_project(project_xml().as_bytes(), &ImportOptions::default()).unwrap();
    assert_eq!(imported.project.sequence(imported.sequences[0]).unwrap().settings.par, (1, 1));
    assert!(!report.has_warnings(), "{report}");
}

#[test]
fn premiere_project_keeps_importing_past_an_unreadable_effect_value() {
    // one parameter of one effect holds something that is not a number
    let xml = project_xml().replace(
        "<ParameterID>6</ParameterID><StartKeyframe>-91445760000000000,0.5:0.5,",
        "<ParameterID>6</ParameterID><StartKeyframe>-91445760000000000,oops:0.5,",
    );
    let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    let text = report.to_string();
    assert!(text.contains("AE.ADBE Motion parameter 6 could not be read"), "{text}");
    let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
    let motion = video.effect("motion").unwrap();
    // the readable parameters of the same effect are kept
    assert_eq!(motion.vec2_at("position", Tick(5 * TPS)), Vec2::new(480.0, 810.0));
    assert_eq!(motion.param("scale").unwrap().keyframes.len(), 2);
    // a preset with the same damage is still refused: it would be saved without the value
    assert!(premiere::import_presets(preset_xml(0).replace("0.25:0.75", "oops:0.75").as_bytes()).is_err());
}

#[test]
fn premiere_gaussian_blur_checkbox_sharing_the_blurriness_id_is_told_apart() {
    // as real project files have it: Blurriness, Blur Dimensions and the Repeat Edge Pixels
    // checkbox carry ParameterID 1, 2 and 1
    let blur = r#"<VideoFilterComponent ObjectID="80"><MatchName>AE.ADBE Gaussian Blur 2</MatchName><Component><Params><Param Index="0" ObjectRef="81"/><Param Index="1" ObjectRef="82"/><Param Index="2" ObjectRef="83"/></Params></Component></VideoFilterComponent>
 <VideoComponentParam ObjectID="81"><ParameterID>1</ParameterID><StartKeyframe>-91445760000000000,24.,0,0,0,0,0,0</StartKeyframe></VideoComponentParam>
 <VideoComponentParam ObjectID="82"><ParameterID>2</ParameterID><StartKeyframe>-91445760000000000,0,0,0,0,0,0,0</StartKeyframe></VideoComponentParam>
 <VideoComponentParam ObjectID="83"><ParameterID>1</ParameterID><StartKeyframe>-91445760000000000,true,0,0,0,0,0,0</StartKeyframe></VideoComponentParam>"#;
    let xml = project_xml()
        .replace(
            "<Component Index=\"0\" ObjectRef=\"64\"/></Components>",
            "<Component Index=\"0\" ObjectRef=\"64\"/><Component Index=\"1\" ObjectRef=\"80\"/></Components>",
        )
        .replace("</PremiereData>", &format!("{blur}</PremiereData>"));
    let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    assert!(!report.to_string().contains("could not be read"), "{report}");
    let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
    let blur = video.effect("gaussian_blur").unwrap();
    assert_eq!(blur.f64_at("blurriness", Tick::ZERO), 24.0);
    assert_eq!(blur.param("repeat_edge").map(|p| p.value.clone()), Some(filmcraft_project::ParamValue::Bool(true)));
}

#[test]
fn premiere_clip_that_starts_before_its_media_is_trimmed_not_refused() {
    // sound slipped a second ahead of its file: source range -1 s .. 1 s on a two-second clip
    let slipped = project_xml().replace(
        &format!("<AudioClip ObjectID=\"70\"><Clip><Source ObjectRef=\"23\"/><InPoint>{}</InPoint><OutPoint>{}</OutPoint>", 5 * TPS, 7 * TPS),
        &format!("<AudioClip ObjectID=\"70\"><Clip><Source ObjectRef=\"23\"/><InPoint>{}</InPoint><OutPoint>{}</OutPoint>", -TPS, TPS),
    );
    assert_ne!(slipped, project_xml());
    let (imported, report) = premiere::import_project(slipped.as_bytes(), &ImportOptions::default()).unwrap();
    assert!(report.to_string().contains("starts before its media"), "{report}");
    let sequence = imported.project.sequence(imported.sequences[0]).unwrap();
    let audio = &sequence.audio_tracks[0].items[0];
    assert_eq!((audio.start, audio.duration, audio.source_in), (Tick(TPS), Tick(TPS), Tick::ZERO));
    assert_eq!(audio.speed, 1.0);
    // the picture it is linked to is untouched, and still linked
    let video = &sequence.video_tracks[0].items[0];
    assert_eq!((video.start, video.duration, video.source_in), (Tick::ZERO, Tick(2 * TPS), Tick(5 * TPS)));
    assert_eq!(video.link, audio.link);
    let with_range = |source_in: i64, source_out: i64| {
        project_xml().replace(
            &format!("<AudioClip ObjectID=\"70\"><Clip><Source ObjectRef=\"23\"/><InPoint>{}</InPoint><OutPoint>{}</OutPoint>", 5 * TPS, 7 * TPS),
            &format!("<AudioClip ObjectID=\"70\"><Clip><Source ObjectRef=\"23\"/><InPoint>{source_in}</InPoint><OutPoint>{source_out}</OutPoint>"),
        )
    };
    // a clip that ends before its media starts plays nothing: it is left out, and said so
    for (source_in, source_out) in [(-2 * TPS, -TPS), (-TPS, 0)] {
        let (imported, report) = premiere::import_project(with_range(source_in, source_out).as_bytes(), &ImportOptions::default()).unwrap();
        assert!(report.to_string().contains("lies wholly before its media"), "{source_in}..{source_out}: {report}");
        let sequence = imported.project.sequence(imported.sequences[0]).unwrap();
        assert!(sequence.audio_tracks[0].items.is_empty());
        assert_eq!(sequence.video_tracks[0].items.len(), 1);
    }
    // a range that runs backwards is still refused
    assert!(premiere::import_project(with_range(7 * TPS, 5 * TPS).as_bytes(), &ImportOptions::default()).is_err());
}

#[test]
fn premiere_media_with_a_time_base_for_a_frame_rate_does_not_refuse_the_project() {
    // 2822400 ticks is 1/90000 s: the file's time base, stored where a frame duration belongs
    let xml =
        project_xml().replace("<VideoStream ObjectID=\"40\"><FrameRate>10594584000</FrameRate>", "<VideoStream ObjectID=\"40\"><FrameRate>2822400</FrameRate>");
    assert_ne!(xml, project_xml());
    let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    assert!(report.to_string().contains("frame duration FilmCraft cannot use (2822400 ticks)"), "{report}");
    let sequence = imported.project.sequence(imported.sequences[0]).unwrap();
    assert_eq!(sequence.settings.frame_rate, FrameRate::FPS_23_976, "the sequence keeps its own rate");
    let media = imported.project.item(sequence.video_tracks[0].items[0].item).unwrap().as_media().unwrap();
    assert_eq!(media.info.video.as_ref().map(|v| v.frame_rate), Some(FrameRate::FPS_24));
    // a sequence with such a rate is a different matter: still refused
    let bad = project_xml().replace("<TrackGroup><FrameRate>10594584000</FrameRate>", "<TrackGroup><FrameRate>2822400</FrameRate>");
    assert_ne!(bad, project_xml());
    assert!(premiere::import_project(bad.as_bytes(), &ImportOptions::default()).is_err());
}

#[test]
fn premiere_project_with_an_object_defined_twice_uses_the_first() {
    let twice = project_xml().replace(
        "</PremiereData>",
        "<Media ObjectUID=\"movie\"><FilePath>/media/second-definition.mov</FilePath><VideoStream ObjectRef=\"40\"/></Media></PremiereData>",
    );
    let (imported, report) = premiere::import_project(twice.as_bytes(), &ImportOptions::default()).unwrap();
    assert!(report.to_string().contains("Media is defined twice"), "{report}");
    let sequence = imported.project.sequence(imported.sequences[0]).unwrap();
    let media = imported.project.item(sequence.video_tracks[0].items[0].item).unwrap().as_media().unwrap();
    assert!(matches!(&media.media, filmcraft_project::MediaRef::File { path } if path == "/media/example.mov"));
    // an empty id is no id: still refused, and so is a preset file with a repeated id
    assert!(premiere::import_project(project_xml().replace("ObjectUID=\"movie\"", "ObjectUID=\"\"").as_bytes(), &ImportOptions::default()).is_err());
    let preset_twice =
        preset_xml(0).replace("</PremiereData>", "<TreeItem ObjectID=\"1\"><TreeItemBase><Name>Again</Name></TreeItemBase></TreeItem></PremiereData>");
    assert!(premiere::import_presets(preset_twice.as_bytes()).is_err());
}

#[test]
fn premiere_effects_without_controls_are_carried_over() {
    for (match_name, id) in [
        ("AE.ADBE Horizontal Flip", "horizontal_flip"),
        ("PR.ADBE Horizontal Flip", "horizontal_flip"),
        ("AE.ADBE Vertical Flip", "vertical_flip"),
        ("PR.ADBE Vertical Flip", "vertical_flip"),
        ("AE.ADBE Black &amp; White", "black_white"),
        ("PR.ADBE Black &amp; White", "black_white"),
    ] {
        for bypass in [false, true] {
            // the older names carry one empty placeholder where a control would be
            let (params, placeholder) = if match_name.starts_with("PR.") {
                (
                    "<Params><Param Index=\"0\" ObjectRef=\"81\"/></Params>",
                    "<ArbVideoComponentParam ObjectID=\"81\"><IsTimeVarying>false</IsTimeVarying><ParameterControlType>9</ParameterControlType><StartKeyframePosition>-91445760000000000</StartKeyframePosition><ParameterID>-1</ParameterID></ArbVideoComponentParam>",
                )
            } else {
                ("<Params/>", "")
            };
            let effect = format!(
                "<VideoFilterComponent ObjectID=\"80\"><MatchName>{match_name}</MatchName><Component><Bypass>{bypass}</Bypass>{params}</Component></VideoFilterComponent>{placeholder}"
            );
            let xml = project_xml()
                .replace(
                    "<Component Index=\"0\" ObjectRef=\"64\"/></Components>",
                    "<Component Index=\"0\" ObjectRef=\"64\"/><Component Index=\"1\" ObjectRef=\"80\"/></Components>",
                )
                .replace("</PremiereData>", &format!("{effect}</PremiereData>"));
            let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
            assert!(!report.has_warnings(), "{match_name}: {report}");
            let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
            let carried = video.effect(id).unwrap_or_else(|| panic!("{match_name} was dropped"));
            assert_eq!(carried.enabled, !bypass, "{match_name}");
            // Motion is still there, and still after the effect it frames
            assert!(video.effect("motion").is_some());
        }
    }
}

/// `project_xml` with one more effect on its video clip: `match_name` and its controls as
/// (ParameterID, value) pairs.
fn project_with_effect(match_name: &str, controls: &[(i64, &str)]) -> String {
    let refs: String = (0..controls.len()).map(|i| format!("<Param Index=\"{i}\" ObjectRef=\"{}\"/>", 81 + i)).collect();
    let params: String = controls
        .iter()
        .enumerate()
        .map(|(i, (id, value))| {
            format!(
                "<VideoComponentParam ObjectID=\"{}\"><ParameterID>{id}</ParameterID><StartKeyframe>-91445760000000000,{value},0,0,0,0,0,0</StartKeyframe></VideoComponentParam>",
                81 + i
            )
        })
        .collect();
    let effect = format!(
        "<VideoFilterComponent ObjectID=\"80\"><MatchName>{match_name}</MatchName><Component><Params>{refs}</Params></Component></VideoFilterComponent>{params}"
    );
    let xml = project_xml()
        .replace(
            "<Component Index=\"0\" ObjectRef=\"64\"/></Components>",
            "<Component Index=\"0\" ObjectRef=\"64\"/><Component Index=\"1\" ObjectRef=\"80\"/></Components>",
        )
        .replace("</PremiereData>", &format!("{effect}</PremiereData>"));
    assert_ne!(xml, project_xml());
    xml
}

/// Brightness & Contrast, Drop Shadow and Basic 3D have the same controls here: their values
/// come across, written the way the files write them (`87.`, an opacity out of 255).
#[test]
fn premiere_brightness_contrast_drop_shadow_and_basic_3d_keep_their_values() {
    let import = |xml: String| {
        let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
        let video = imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0].clone();
        (video, report)
    };
    let (video, report) = import(project_with_effect("AE.ADBE Brightness &amp; Contrast 2", &[(1, "15."), (2, "-74.5")]));
    assert!(!report.has_warnings(), "{report}");
    let effect = video.effect("brightness_contrast").expect("Brightness & Contrast was dropped");
    assert_eq!((effect.f64_at("brightness", Tick::ZERO), effect.f64_at("contrast", Tick::ZERO)), (15.0, -74.5));

    let shadow = [(1, "18374686479671623680"), (2, "127.5"), (3, "140."), (4, "60."), (5, "26."), (6, "true")];
    let (video, report) = import(project_with_effect("AE.ADBE Drop Shadow", &shadow));
    assert!(!report.has_warnings(), "{report}");
    let effect = video.effect("drop_shadow").expect("Drop Shadow was dropped");
    assert!((effect.f64_at("opacity", Tick::ZERO) - 50.0).abs() < 1e-9, "127.5 of 255 is 50 %");
    assert_eq!((effect.f64_at("direction", Tick::ZERO), effect.f64_at("distance", Tick::ZERO), effect.f64_at("softness", Tick::ZERO)), (140.0, 60.0, 26.0));
    assert!(matches!(effect.param("only").unwrap().value, filmcraft_project::ParamValue::Bool(true)));
    // a shadow colour other than black is not read: the report says so, the rest still comes across
    let mut tinted = shadow;
    tinted[0] = (1, "18374967954648334335");
    let (video, report) = import(project_with_effect("AE.ADBE Drop Shadow", &tinted));
    assert!(report.to_string().contains("AE.ADBE Drop Shadow parameter 1 is not supported"), "{report}");
    assert_eq!(video.effect("drop_shadow").unwrap().f64_at("distance", Tick::ZERO), 60.0);

    let (video, report) =
        import(project_with_effect("AE.ADBE Basic 3D", &[(1, "-38.801406860352"), (2, "-69.659805297852"), (3, "41."), (4, "false"), (5, "false")]));
    assert!(!report.has_warnings(), "{report}");
    let effect = video.effect("basic_3d").expect("Basic 3D was dropped");
    assert_eq!(
        (effect.f64_at("swivel", Tick::ZERO), effect.f64_at("tilt", Tick::ZERO), effect.f64_at("distance", Tick::ZERO)),
        (-38.801406860352, -69.659805297852, 41.0)
    );
    // Motion is still there beside each of them
    assert!(video.effect("motion").is_some());
}

/// `project_xml` with a transition at the end of its video clip (`video`) or its audio clip.
fn project_with_transition(video: bool, match_name: &str, extra: &str) -> String {
    let (tag, track) = if video { ("VideoTransitionTrackItem", "VideoClipTrack") } else { ("AudioTransitionTrackItem", "AudioClipTrack") };
    let transition = format!(
        "<{tag} ObjectID=\"80\"><TransitionTrackItem><TrackItem><Start>{start}</Start><End>{end}</End></TrackItem><MatchName>{match_name}</MatchName><Alignment>{duration}</Alignment><HasOutgoingClip>true</HasOutgoingClip><HasIncomingClip>false</HasIncomingClip></TransitionTrackItem>{extra}</{tag}>",
        start = TPS + TPS / 2,
        end = 2 * TPS,
        duration = TPS / 2
    );
    let xml = project_xml()
        .replace(
            &format!("</ClipItems></ClipTrack></{track}>"),
            &format!("</ClipItems><TransitionItems><TrackItems><TrackItem ObjectRef=\"80\"/></TrackItems></TransitionItems></ClipTrack></{track}>"),
        )
        .replace("</PremiereData>", &format!("{transition}</PremiereData>"));
    assert_ne!(xml, project_xml());
    xml
}

#[test]
fn premiere_dissolves_without_controls_are_timeline_transitions() {
    for (match_name, id) in [
        ("AE.ADBE Dip To Black", "dip_to_black"),
        ("ADBE Dip To Black", "dip_to_black"),
        ("AE.ADBE Dip To White", "dip_to_white"),
        ("ADBE Dip To White", "dip_to_white"),
        ("ADBE Additive Dissolve", "additive_dissolve"),
        ("ADBE Film Dissolve", "film_dissolve"),
    ] {
        let xml = project_with_transition(true, match_name, "");
        let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
        assert!(!report.has_warnings(), "{match_name}: {report}");
        let track = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0];
        assert_eq!(track.transitions.len(), 1, "{match_name} was skipped");
        let transition = &track.transitions[0];
        assert_eq!(transition.effect.effect, id, "{match_name}");
        assert_eq!((transition.start, transition.duration), (Tick(TPS + TPS / 2), Tick(TPS / 2)), "{match_name}");
        assert_eq!(transition.from, Some(track.items[0].id), "{match_name}");
    }
}

/// Newer projects write an audio crossfade with an adjustable curve as "Custom Fade", with a
/// shape type and value. It becomes the nearest of the three fades FilmCraft has, and the
/// report says the curve itself was not kept.
#[test]
fn premiere_custom_audio_fade_becomes_the_nearest_named_fade() {
    let shape = |kind: i32, value: i32| {
        format!("<FadeShapeType>{kind}</FadeShapeType><FadeShapeValue>{value}</FadeShapeValue><CrossfadeSymmetry>0</CrossfadeSymmetry>")
    };
    for (extra, id, label) in [
        (shape(1, 19), "constant_power", "Constant Power"),
        (shape(1, 16), "constant_power", "Constant Power"),
        (shape(1, 0), "constant_gain", "Constant Gain"),
        (shape(0, 0), "constant_gain", "Constant Gain"),
        (shape(0, -35), "exponential_fade", "Exponential Fade"),
        (String::new(), "constant_power", "Constant Power"),
        ("<FadeShapeType>x</FadeShapeType><FadeShapeValue>NaN</FadeShapeValue>".to_string(), "constant_power", "Constant Power"),
    ] {
        let xml = project_with_transition(false, "Custom Fade", &extra);
        let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
        let track = &imported.project.sequence(imported.sequences[0]).unwrap().audio_tracks[0];
        assert_eq!(track.transitions.len(), 1, "{extra}: skipped");
        assert_eq!(track.transitions[0].effect.effect, id, "{extra}");
        let text = report.to_string();
        assert!(text.contains(&format!("\"Custom Fade\" was imported as {label}")), "{extra}: {text}");
    }
}

#[test]
fn premiere_generated_graphics_are_kept_switched_off_and_look_for_no_file() {
    // a graphic made inside the application: a four-character code as a number where the path belongs
    let xml = project_xml().replace(
        "<Media ObjectUID=\"movie\"><FilePath>/media/example.mov</FilePath>",
        "<Media ObjectUID=\"movie\"><FilePath>1196574294</FilePath><ActualMediaFilePath>1196574294</ActualMediaFilePath><Title>Graphic</Title><Infinite>true</Infinite>",
    );
    assert_ne!(xml, project_xml());
    let opts = ImportOptions { base_dir: Some("/projects".into()), ..Default::default() };
    let (imported, report) = premiere::import_project(xml.as_bytes(), &opts).unwrap();
    let text = report.to_string();
    assert!(text.contains("(Graphic) was preserved as an offline placeholder"), "{text}");
    assert!(text.contains("kept on the timeline switched off"), "{text}");
    let sequence = imported.project.sequence(imported.sequences[0]).unwrap();
    let video = &sequence.video_tracks[0].items[0];
    assert!(!video.enabled, "a placeholder that covered the picture");
    assert_eq!((video.start, video.duration), (Tick::ZERO, Tick(2 * TPS)), "the clip keeps its place");
    let media = imported.project.item(video.item).unwrap().as_media().unwrap();
    assert!(media.offline);
    assert!(matches!(&media.media, filmcraft_project::MediaRef::File { path } if path.is_empty()), "no file name made up from the code: {:?}", media.media);
    // a file-backed clip in the same place stays on
    let (plain, _) = premiere::import_project(project_xml().as_bytes(), &opts).unwrap();
    assert!(plain.project.sequence(plain.sequences[0]).unwrap().video_tracks[0].items[0].enabled);
}

#[test]
fn premiere_points_written_as_two_fields_by_older_versions_are_read() {
    // project version 36 and thereabouts: `time,x,y,…` where newer files have `time,x:y,…`
    let xml = project_xml().replace(
        "<ParameterID>1</ParameterID><StartKeyframe>-91445760000000000,0.25:0.75,0,0,0,0,0,0</StartKeyframe>",
        "<ParameterID>1</ParameterID><StartKeyframe>-91445760000000000,0.25,0.75,0,0,0,0,0,0,5,4,0,0,0,0</StartKeyframe>",
    );
    assert_ne!(xml, project_xml());
    let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    assert!(!report.has_warnings(), "{report}");
    let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
    let motion = video.effect("motion").unwrap();
    assert_eq!(motion.vec2_at("position", Tick(5 * TPS)), Vec2::new(480.0, 810.0));
    assert_eq!(motion.vec2_at("anchor", Tick(5 * TPS)), Vec2::new(960.0, 540.0), "the newer spelling beside it still reads");

    // and animated: two keyframes in the older spelling, linear
    let animated = project_xml().replace(
        "<PointComponentParam ObjectID=\"65\"><ParameterID>1</ParameterID><StartKeyframe>-91445760000000000,0.25:0.75,0,0,0,0,0,0</StartKeyframe>",
        &format!(
            "<PointComponentParam ObjectID=\"65\"><ParameterID>1</ParameterID><IsTimeVarying>true</IsTimeVarying><StartKeyframe>-91445760000000000,0.25,0.75,0,0,0,0,0,0,5,4,0,0,0,0</StartKeyframe><Keyframes>{},0.25,0.75,0,0,0,0,0,0,5,4,0,0,0,0;{},0.75,0.25,0,0,0,0,0,0,5,4,0,0,0,0;</Keyframes>",
            5 * TPS,
            7 * TPS
        ),
    );
    assert_ne!(animated, project_xml());
    let (imported, _) = premiere::import_project(animated.as_bytes(), &ImportOptions::default()).unwrap();
    let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
    let position = video.effect("motion").unwrap().param("position").unwrap();
    assert_eq!(position.keyframes.len(), 2);
    assert_eq!(position.vec2_at(Tick(5 * TPS)), Vec2::new(480.0, 810.0));
    assert_eq!(position.vec2_at(Tick(7 * TPS)), Vec2::new(1440.0, 270.0));
}

/// Standard base64, for building a Source Text value.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = u32::from(chunk[0]) << 16 | u32::from(*chunk.get(1).unwrap_or(&0)) << 8 | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// `project_xml` with its media made a graphic of the application's own (a number where the path
/// belongs) and one Text component on the video clip, holding `source_text` as its Source Text.
fn project_with_text_layer(source_text: &[u8]) -> String {
    let text = format!(
        concat!(
            "<VideoFilterComponent ObjectID=\"80\"><MatchName>AE.ADBE Text</MatchName><Component><Params>",
            "<Param Index=\"0\" ObjectRef=\"81\"/><Param Index=\"1\" ObjectRef=\"82\"/><Param Index=\"2\" ObjectRef=\"83\"/><Param Index=\"3\" ObjectRef=\"84\"/>",
            "</Params></Component></VideoFilterComponent>",
            "<ArbVideoComponentParam ObjectID=\"81\"><Name>Source Text</Name><IsTimeVarying>false</IsTimeVarying>",
            "<StartKeyframeValue Encoding=\"base64\">{value}</StartKeyframeValue><ParameterID>1</ParameterID></ArbVideoComponentParam>",
            "<PointComponentParam ObjectID=\"82\"><ParameterID>3</ParameterID><StartKeyframe>-91445760000000000,0.25:0.5,0,0,0,0,0,0</StartKeyframe></PointComponentParam>",
            "<VideoComponentParam ObjectID=\"83\"><ParameterID>4</ParameterID><StartKeyframe>-91445760000000000,50.,0,0,0,0,0,0</StartKeyframe></VideoComponentParam>",
            "<VideoComponentParam ObjectID=\"84\"><ParameterID>8</ParameterID><StartKeyframe>-91445760000000000,80.,0,0,0,0,0,0</StartKeyframe></VideoComponentParam>"
        ),
        value = base64(source_text)
    );
    let xml = project_xml()
        .replace(
            "<Media ObjectUID=\"movie\"><FilePath>/media/example.mov</FilePath>",
            "<Media ObjectUID=\"movie\"><FilePath>1196574294</FilePath><Title>Graphic</Title><Infinite>true</Infinite>",
        )
        .replace(
            "<Component Index=\"0\" ObjectRef=\"64\"/></Components>",
            "<Component Index=\"0\" ObjectRef=\"64\"/><Component Index=\"1\" ObjectRef=\"80\"/></Components>",
        )
        .replace("</PremiereData>", &format!("{text}</PremiereData>"));
    assert_eq!(xml.matches("AE.ADBE Text").count(), 1);
    assert!(xml.contains("1196574294"));
    xml
}

/// A Source Text value as project versions 34 to 38 write it: the byte length, four zero bytes,
/// then the JSON in UTF-16LE.
fn source_text_json(json: &str) -> Vec<u8> {
    let body: Vec<u8> = json.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut bytes = (body.len() as u32).to_le_bytes().to_vec();
    bytes.extend([0u8; 4]);
    bytes.extend(body);
    bytes
}

const TITLE_JSON: &str = r#"{"mTextParam":{"mAlignment":1,"mLeading":0,"mLigatures":false,"mIsVerticalText":false,"mWidth":0,"mHeight":0,
 "mShadowVisible":true,"mShadowColor":4144959,"mShadowOpacity":75.5,"mShadowAngle":135,"mShadowOffset":7,"mShadowSize":0,"mShadowBlur":40,
 "mStyleSheet":{"mText":"A synthetic title","mFontName":{"mParamValues":[[0,"ExampleSans-Bold"]]},"mFontSize":{"mParamValues":[[0,77.5]]},
  "mFillColor":{"mParamValues":[[0,16711680]]},"mFillVisible":{"mParamValues":[[0,true]]},
  "mStrokeVisible":{"mParamValues":[[0,true]]},"mStrokeWidth":{"mParamValues":[[0,4]]},"mStrokeColor":{"mParamValues":[[0,0]]},
  "mTracking":{"mParamValues":[[0,20]]},"mFauxBold":{"mParamValues":[[0,true]]},"mCapsOption":{"mParamValues":[[0,1]]}}},"mVersion":1}"#;

/// A title typed in the application is a graphic clip with a text layer here: the words, the
/// font, the size, the colours and the layer's place come across, the clip is switched on and
/// starts at its canvas's own start.
#[test]
fn premiere_text_of_a_graphic_becomes_a_text_layer() {
    use filmcraft_project::ParamValue;
    let xml = project_with_text_layer(&source_text_json(TITLE_JSON));
    let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    let said = report.to_string();
    assert!(said.contains("imported as FilmCraft text layers"), "{said}");
    assert!(!said.contains("AE.ADBE Text"), "the text component was read, not skipped: {said}");
    let sequence = imported.project.sequence(imported.sequences[0]).unwrap();
    let video = &sequence.video_tracks[0].items[0];
    assert!(video.enabled, "a graphic whose text was read is shown");
    assert!(
        matches!(imported.project.item(video.item).unwrap().kind, ItemKind::Graphic { width: 1920, height: 1080, .. }),
        "its source is a canvas of the sequence frame: {:?}",
        imported.project.item(video.item).unwrap().kind
    );
    assert_eq!((video.start, video.duration, video.source_in, video.speed), (Tick::ZERO, Tick(2 * TPS), Tick::ZERO, 1.0));
    // the clip's own Motion keyframes moved with the clip's start (they were at 5 s and 7 s of the source)
    let scale = video.effect("motion").unwrap().param("scale").unwrap();
    assert_eq!(scale.keyframes.iter().map(|k| k.time).collect::<Vec<_>>(), [Tick::ZERO, Tick(2 * TPS)]);

    let layers: Vec<_> = video.graphic_layers().collect();
    assert_eq!(layers.len(), 1);
    let layer = layers[0];
    let text = |id: &str| match &layer.param(id).unwrap().value {
        ParamValue::Text(s) => s.clone(),
        other => panic!("{id}: {other:?}"),
    };
    assert_eq!((text("text").as_str(), text("font").as_str(), text("font_style").as_str()), ("A synthetic title", "Example Sans", "Bold"));
    assert_eq!(layer.f64_at("size", Tick::ZERO), 77.5);
    assert_eq!(layer.f64_at("tracking", Tick::ZERO), 20.0);
    assert!(matches!(layer.param("align").unwrap().value, ParamValue::Choice(1)), "centred");
    assert!(matches!(layer.param("caps").unwrap().value, ParamValue::Choice(1)), "all caps");
    assert!(matches!(layer.param("faux_bold").unwrap().value, ParamValue::Bool(true)));
    assert!(matches!(layer.param("fill_color").unwrap().value, ParamValue::Color(c) if c == [1.0, 0.0, 0.0, 1.0]), "{:?}", layer.param("fill_color"));
    assert!(matches!(layer.param("stroke").unwrap().value, ParamValue::Bool(true)));
    assert_eq!(layer.f64_at("stroke_width", Tick::ZERO), 4.0);
    assert!(matches!(layer.param("stroke_color").unwrap().value, ParamValue::Color(c) if c == [0.0, 0.0, 0.0, 1.0]));
    assert!(matches!(layer.param("shadow").unwrap().value, ParamValue::Bool(true)));
    assert_eq!(
        (layer.f64_at("shadow_opacity", Tick::ZERO), layer.f64_at("shadow_distance", Tick::ZERO), layer.f64_at("shadow_blur", Tick::ZERO)),
        (75.5, 7.0, 40.0)
    );
    // the layer's own place: a quarter across and half-way down the 1920x1080 frame, half size, 80 %
    assert_eq!(layer.vec2_at("position", Tick::ZERO), Vec2::new(480.0, 540.0));
    assert_eq!((layer.f64_at("scale", Tick::ZERO), layer.f64_at("opacity", Tick::ZERO)), (50.0, 80.0));
    // and it is a layer the renderer can evaluate
    let spec = filmcraft_project::graphic::eval_layer(layer, Tick::ZERO, (1920, 1080)).expect("a text layer");
    assert!(matches!(&spec.content, filmcraft_project::graphic::LayerContent::Text(t) if t.text == "A synthetic title" && t.size == 77.5));

    // the sound of the same source has nothing to play: it stays as it was, switched off
    assert!(!sequence.audio_tracks[0].items[0].enabled);
}

/// Source Text that is not the JSON form (newer versions write a binary one), or is damaged, is
/// left alone: the clip stays on the timeline switched off and the report names the component.
#[test]
fn premiere_text_that_cannot_be_read_leaves_the_graphic_switched_off() {
    // the marker of the binary form with nothing readable behind it, and one whose length is off by the marker
    let mut binary = vec![0x20, 0x01, 0, 0, 0, 0, 0, 0, 0x44, 0x33, 0x22, 0x11];
    binary.extend([0u8; 0x120]);
    let mut counted_with_marker = source_text_binary();
    let length = u32::from_le_bytes(counted_with_marker[..4].try_into().unwrap()) + 4;
    counted_with_marker[..4].copy_from_slice(&length.to_le_bytes());
    let json = source_text_json(TITLE_JSON);
    let mut wrong_length = json.clone();
    wrong_length[0] ^= 1;
    for (what, value) in [
        ("binary", binary),
        ("binary, length counts the marker", counted_with_marker),
        ("empty", Vec::new()),
        ("cut short", json[..json.len() / 2].to_vec()),
        ("wrong length", wrong_length),
        ("no text in it", source_text_json(r#"{"mTextParam":{"mStyleSheet":{}}}"#)),
        ("not an object", source_text_json("[1,2,3]")),
    ] {
        let xml = project_with_text_layer(&value);
        let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap_or_else(|e| panic!("{what}: {e}"));
        let said = report.to_string();
        assert!(said.contains("(AE.ADBE Text) is not supported and was skipped"), "{what}: {said}");
        assert!(said.contains("kept on the timeline switched off"), "{what}: {said}");
        let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
        assert!(!video.enabled, "{what}");
        assert_eq!(video.graphic_layers().count(), 0, "{what}");
        assert!(imported.project.item(video.item).unwrap().as_media().is_some(), "{what}: still the placeholder");
    }
    // text that is not valid base64 at all
    let xml = project_with_text_layer(b"x").replace(
        "<StartKeyframeValue Encoding=\"base64\">eA==</StartKeyframeValue>",
        "<StartKeyframeValue Encoding=\"base64\">*** not base64 ***</StartKeyframeValue>",
    );
    assert!(xml.contains("not base64"));
    let (imported, _) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    assert!(!imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0].enabled);
}

/// Vector Motion, the transform of a whole graphic, has Motion's controls: it becomes the
/// Transform effect, which moves the composed graphic before the clip's own Motion does.
#[test]
fn premiere_vector_motion_becomes_a_transform_effect() {
    let xml = project_with_effect("AE.ADBE Graphic Group", &[(2, "76."), (3, "100."), (4, "true"), (5, "15.")]).replace(
        "</PremiereData>",
        concat!(
            "<PointComponentParam ObjectID=\"90\"><ParameterID>1</ParameterID><StartKeyframe>-91445760000000000,0.75:0.5,0,0,0,0,0,0</StartKeyframe></PointComponentParam>",
            "<PointComponentParam ObjectID=\"91\"><ParameterID>6</ParameterID><StartKeyframe>-91445760000000000,0.5:0.5,0,0,0,0,0,0</StartKeyframe></PointComponentParam>",
            "</PremiereData>"
        ),
    );
    let xml = xml.replace(
        "<Params><Param Index=\"0\" ObjectRef=\"81\"/>",
        "<Params><Param Index=\"4\" ObjectRef=\"90\"/><Param Index=\"5\" ObjectRef=\"91\"/><Param Index=\"0\" ObjectRef=\"81\"/>",
    );
    assert!(xml.contains("ObjectRef=\"90\""));
    let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    assert!(!report.has_warnings(), "{report}");
    let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
    let transform = video.effect("transform").expect("Vector Motion was dropped");
    assert_eq!(transform.vec2_at("position", Tick::ZERO), Vec2::new(1440.0, 540.0));
    assert_eq!(transform.vec2_at("anchor", Tick::ZERO), Vec2::new(960.0, 540.0));
    assert_eq!(
        (transform.f64_at("scale_height", Tick::ZERO), transform.f64_at("scale_width", Tick::ZERO), transform.f64_at("rotation", Tick::ZERO)),
        (76.0, 100.0, 15.0)
    );
    assert!(matches!(transform.param("uniform_scale").unwrap().value, filmcraft_project::ParamValue::Bool(true)));
    assert!(video.effect("motion").is_some());
}

/// A file writes each distinct Source Text in full once. The same text on a later clip is an
/// empty element that names the first by its hash.
#[test]
fn premiere_text_written_once_is_found_by_its_hash() {
    let full = project_with_text_layer(&source_text_json(TITLE_JSON));
    let value = base64(&source_text_json(TITLE_JSON));
    let xml = full.replace(&format!("<StartKeyframeValue Encoding=\"base64\">{value}</StartKeyframeValue>"), "<StartKeyframeValue Encoding=\"base64\" BinaryHash=\"example-hash\"/>").replace(
        "</PremiereData>",
        &format!(
            "<ArbVideoComponentParam ObjectID=\"95\"><StartKeyframeValue Encoding=\"base64\" BinaryHash=\"example-hash\">{value}</StartKeyframeValue></ArbVideoComponentParam></PremiereData>"
        ),
    );
    assert_eq!(xml.matches("example-hash").count(), 2);
    let (imported, _) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
    assert!(video.enabled);
    assert_eq!(video.graphic_layers().count(), 1);

    // a hash that nothing in the file carries the value of: there is no text to read
    let lost = xml.replace("BinaryHash=\"example-hash\">", "BinaryHash=\"another-hash\">");
    assert_ne!(lost, xml);
    let (imported, _) = premiere::import_project(lost.as_bytes(), &ImportOptions::default()).unwrap();
    assert!(!imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0].enabled);
}

/// One field of a hand-written table-and-offset buffer (the binary form of Source Text).
enum Field {
    Absent,
    Byte(u8),
    Number(u32),
    Float(f32),
    Words(&'static str),
    Table(Vec<Field>),
    List(Vec<Field>),
}

/// Writes a table (its field-offset list, then its fields, then what they point to) and returns
/// where the table starts.
fn put_table(out: &mut Vec<u8>, fields: &[Field]) -> usize {
    let list = out.len();
    let mut next = 4u16;
    let offsets: Vec<u16> = fields
        .iter()
        .map(|f| {
            if matches!(f, Field::Absent) {
                0
            } else {
                next += 4;
                next - 4
            }
        })
        .collect();
    out.extend(((4 + 2 * fields.len()) as u16).to_le_bytes());
    out.extend(next.to_le_bytes());
    for offset in &offsets {
        out.extend(offset.to_le_bytes());
    }
    let table = out.len();
    out.extend(((table - list) as i32).to_le_bytes());
    let mut pointers = Vec::new();
    for field in fields {
        match field {
            Field::Absent => {}
            Field::Byte(v) => out.extend([*v, 0, 0, 0]),
            Field::Number(v) => out.extend(v.to_le_bytes()),
            Field::Float(v) => out.extend(v.to_le_bytes()),
            other => {
                pointers.push((out.len(), other));
                out.extend([0; 4]);
            }
        }
    }
    for (at, field) in pointers {
        let target = put_value(out, field);
        out[at..at + 4].copy_from_slice(&((target - at) as u32).to_le_bytes());
    }
    table
}

fn put_value(out: &mut Vec<u8>, field: &Field) -> usize {
    match field {
        Field::Words(text) => {
            let at = out.len();
            out.extend((text.len() as u32).to_le_bytes());
            out.extend(text.as_bytes());
            out.push(0);
            at
        }
        Field::Table(fields) => put_table(out, fields),
        Field::List(items) => {
            let at = out.len();
            out.extend((items.len() as u32).to_le_bytes());
            let slots = out.len();
            out.resize(slots + 4 * items.len(), 0);
            for (i, item) in items.iter().enumerate() {
                let slot = slots + 4 * i;
                let target = put_value(out, item);
                out[slot..slot + 4].copy_from_slice(&((target - slot) as u32).to_le_bytes());
            }
            at
        }
        _ => panic!("not something a field points to"),
    }
}

/// A Source Text value as project versions 40 to 43 write it: the byte length, four zero bytes,
/// the marker, then the tables. Two runs ("Two " and "fonts"), two fonts, the second run smaller.
fn source_text_binary() -> Vec<u8> {
    use Field::*;
    let black = || Table(vec![Byte(0), Byte(0), Byte(0)]);
    let style = |size: f32| Table(vec![Number(1), Float(size), Table(vec![Byte(236), Byte(246)]), Absent, black(), Byte(1), Float(4.0)]);
    let mut text = vec![
        List(vec![Table(vec![Words("Two "), style(70.0)]), Table(vec![Words("fonts"), style(40.0)])]),
        List(vec![Words("ExampleSans-Regular"), Words("ExampleSerif-Bold")]),
    ];
    text.extend([Absent, Absent, Number(1)]);
    text.extend([Absent, Absent, Absent, Absent, Absent]);
    text.extend([black(), Byte(1), Float(50.0), Absent, Float(10.0), Absent, Float(30.0)]);
    let mut tables = vec![0u8; 4];
    let root = put_table(&mut tables, &[Table(text)]);
    tables[..4].copy_from_slice(&(root as u32).to_le_bytes());
    // the length in the header counts the tables, not the marker in front of them
    let mut bytes = (tables.len() as u32).to_le_bytes().to_vec();
    bytes.extend([0u8; 4]);
    bytes.extend([0x44, 0x33, 0x22, 0x11]);
    bytes.extend(tables);
    bytes
}

/// The binary form newer project versions write holds the same things in numbered fields.
#[test]
fn premiere_text_in_the_binary_form_becomes_a_text_layer() {
    use filmcraft_project::ParamValue;
    let xml = project_with_text_layer(&source_text_binary());
    let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    let said = report.to_string();
    assert!(!said.contains("AE.ADBE Text"), "read, not skipped: {said}");
    assert!(!said.contains("more than one"), "the smaller second run is a style run, not a loss: {said}");
    let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
    assert!(video.enabled);
    let layers: Vec<_> = video.graphic_layers().collect();
    assert_eq!(layers.len(), 1);
    let layer = layers[0];
    let text = |id: &str| match &layer.param(id).unwrap().value {
        ParamValue::Text(s) => s.clone(),
        other => panic!("{id}: {other:?}"),
    };
    assert_eq!((text("text").as_str(), text("font").as_str(), text("font_style").as_str()), ("Two fonts", "Example Serif", "Bold"));
    assert_eq!(layer.f64_at("size", Tick::ZERO), 70.0);
    assert!(matches!(layer.param("align").unwrap().value, ParamValue::Choice(1)));
    // a colour channel that is not written is 255
    assert!(
        matches!(layer.param("fill_color").unwrap().value, ParamValue::Color(c) if c == [236.0 / 255.0, 246.0 / 255.0, 1.0, 1.0]),
        "{:?}",
        layer.param("fill_color")
    );
    assert!(matches!(layer.param("stroke").unwrap().value, ParamValue::Bool(true)));
    assert_eq!(layer.f64_at("stroke_width", Tick::ZERO), 4.0);
    assert!(matches!(layer.param("stroke_color").unwrap().value, ParamValue::Color(c) if c == [0.0, 0.0, 0.0, 1.0]));
    assert!(matches!(layer.param("shadow").unwrap().value, ParamValue::Bool(true)));
    assert!(matches!(layer.param("shadow_color").unwrap().value, ParamValue::Color(c) if c == [0.0, 0.0, 0.0, 1.0]));
    assert_eq!(
        (layer.f64_at("shadow_opacity", Tick::ZERO), layer.f64_at("shadow_distance", Tick::ZERO), layer.f64_at("shadow_angle", Tick::ZERO)),
        (50.0, 10.0, 30.0)
    );
    assert_eq!(layer.vec2_at("position", Tick::ZERO), Vec2::new(480.0, 540.0));
    // "fonts", the second run, is smaller: a style run over its five characters
    let runs = &layer.layer.as_ref().expect("a style run").runs;
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert_eq!((runs[0].start, runs[0].end, runs[0].style.size), (4, 9, Some(40.0)));
}

/// Every number in the binary form comes from the file: cut short anywhere, or with any byte
/// changed, it is read or refused, never a panic and never a failed import.
#[test]
fn premiere_text_in_a_damaged_binary_form_never_panics() {
    let good = source_text_binary();
    let fix_length = |mut bytes: Vec<u8>| {
        if bytes.len() >= 12 {
            let tables = (bytes.len() - 12) as u32;
            bytes[..4].copy_from_slice(&tables.to_le_bytes());
        }
        bytes
    };
    let mut cases: Vec<Vec<u8>> = (12..good.len()).map(|cut| fix_length(good[..cut].to_vec())).collect();
    for at in 12..good.len() {
        for value in [0x00, 0x7f, 0x80, 0xff] {
            let mut changed = good.clone();
            changed[at] = value;
            cases.push(changed);
        }
    }
    for case in cases {
        let xml = project_with_text_layer(&case);
        let result = std::panic::catch_unwind(|| premiere::import_project(xml.as_bytes(), &ImportOptions::default()).map(|_| ()));
        assert!(matches!(result, Ok(Ok(()))), "{case:?}");
    }
}

/// A text whose later characters have another size and colour keeps them as style runs: each
/// value runs from its first character to the next value's, and a value equal to the first
/// character's is the layer's own.
#[test]
fn premiere_text_with_several_styles_keeps_them_as_style_runs() {
    use filmcraft_project::{CharStyle, StyleRun};
    let json = TITLE_JSON
        .replace(r#""mFontSize":{"mParamValues":[[0,77.5]]}"#, r#""mFontSize":{"mParamValues":[[0,77.5],[2,30]]}"#)
        .replace(r#""mFillColor":{"mParamValues":[[0,16711680]]}"#, r#""mFillColor":{"mParamValues":[[0,16711680],[2,255],[11,16711680]]}"#);
    assert_ne!(json, TITLE_JSON);
    let xml = project_with_text_layer(&source_text_json(&json));
    let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    let said = report.to_string();
    assert!(!said.contains("more than one"), "nothing was dropped: {said}");
    let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
    let layer = video.graphic_layers().next().unwrap();
    assert_eq!(layer.f64_at("size", Tick::ZERO), 77.5, "the layer has the first character's size");
    let runs = &layer.layer.as_ref().expect("style runs").runs;
    assert_eq!(
        runs,
        &[
            StyleRun { start: 2, end: 17, style: CharStyle { size: Some(30.0), ..Default::default() } },
            StyleRun { start: 2, end: 11, style: CharStyle { fill: Some([0.0, 0.0, 1.0, 1.0]), ..Default::default() } },
        ]
    );
    // a single style leaves the layer without runs
    let plain = project_with_text_layer(&source_text_json(TITLE_JSON));
    let (imported, _) = premiere::import_project(plain.as_bytes(), &ImportOptions::default()).unwrap();
    let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
    assert!(video.graphic_layers().next().unwrap().layer.as_ref().is_none_or(|extra| extra.runs.is_empty()));
}

/// A title document as the Title Designer of the older project versions stores it, authored for
/// this test: a 1280x720 layout with one centred block of two lines. The first four characters
/// are in the first text description and style, the rest of the first line in the second.
const LEGACY_TITLE: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-16\" ?><Adobe_Root><InscriberLayouts Version=\"1.0\"><Layout>",
    "<LayoutDimension Version=\"2\"><pXPIXELS>1280</pXPIXELS><pYLINES>720</pYLINES></LayoutDimension>",
    "<TextDescriptions Version=\"4\">",
    "<TextDescription Reference=\"4096\"><TypeSpec><txHeight>80</txHeight><txKern>0</txKern><leading>0</leading><txSCapsOn>true</txSCapsOn>",
    "<txUnderline>false</txUnderline><fifontFamilyName>Example Sans</fifontFamilyName><fifontStyle>Bold</fifontStyle></TypeSpec></TextDescription>",
    "<TextDescription Reference=\"4097\"><TypeSpec><txHeight>40</txHeight><txKern>0</txKern><leading>0</leading><txSCapsOn>false</txSCapsOn>",
    "<txUnderline>false</txUnderline><fifontFamilyName>Example Serif</fifontFamilyName><fifontStyle>Regular</fifontStyle></TypeSpec></TextDescription>",
    "</TextDescriptions><Styles>",
    "<Style ID=\"4096\"><StyleBase Version=\"4\"><shadow_softness>30</shadow_softness></StyleBase><FragmentList Version=\"5\">",
    "<Fragment><size>0</size><offset>0</offset><angle>0</angle><eFragmentType>0</eFragmentType><fragmentOff>false</fragmentOff>",
    "<annotation>65538</annotation><painterMix>15 15 </painterMix></Fragment>",
    "<Fragment><size>6</size><offset>0</offset><angle>0</angle><eFragmentType>2</eFragmentType><fragmentOff>false</fragmentOff>",
    "<annotation>1</annotation><painterMix>13 13 </painterMix></Fragment>",
    "<Fragment><size>3</size><offset>12</offset><angle>-45</angle><eFragmentType>0</eFragmentType><fragmentOff>false</fragmentOff>",
    "<annotation>65537</annotation><painterMix>0 0 </painterMix></Fragment></FragmentList>",
    "<ShaderList Version=\"1\"><ShaderRef PainterNumber=\"13\"><shaderRef>4098</shaderRef></ShaderRef>",
    "<ShaderRef PainterNumber=\"15\"><shaderRef>4096</shaderRef></ShaderRef><ShaderRef PainterNumber=\"-1\"><shaderRef>4097</shaderRef></ShaderRef></ShaderList></Style>",
    "<Style ID=\"4097\"><StyleBase Version=\"4\"><shadow_softness>30</shadow_softness></StyleBase><FragmentList Version=\"5\">",
    "<Fragment><size>0</size><offset>0</offset><angle>0</angle><eFragmentType>0</eFragmentType><fragmentOff>false</fragmentOff>",
    "<annotation>65538</annotation><painterMix>15 15 </painterMix></Fragment></FragmentList>",
    "<ShaderList Version=\"1\"><ShaderRef PainterNumber=\"15\"><shaderRef>4099</shaderRef></ShaderRef></ShaderList></Style>",
    "</Styles><Shaders>",
    "<Shader Version=\"4\"><cReference>4096</cReference><colorOption>4</colorOption>",
    "<ColorSpec index=\"0\"><red>255</red><green>255</green><blue>0</blue><xpar>0</xpar></ColorSpec>",
    "<ColorSpec index=\"4\"><red>250</red><green>250</green><blue>250</blue><xpar>0</xpar></ColorSpec></Shader>",
    "<Shader Version=\"4\"><cReference>4097</cReference><colorOption>0</colorOption>",
    "<ColorSpec index=\"0\"><red>0</red><green>0</green><blue>0</blue><xpar>51</xpar></ColorSpec></Shader>",
    "<Shader Version=\"4\"><cReference>4098</cReference><colorOption>4</colorOption>",
    "<ColorSpec index=\"0\"><red>0</red><green>0</green><blue>255</blue><xpar>0</xpar></ColorSpec></Shader>",
    "<Shader Version=\"4\"><cReference>4099</cReference><colorOption>4</colorOption>",
    "<ColorSpec index=\"0\"><red>255</red><green>0</green><blue>0</blue><xpar>0</xpar></ColorSpec></Shader>",
    "</Shaders><Layers><Layer><TextPage><TextChain>",
    "<ChainProperty Version=\"9\"><wordWrap>false</wordWrap><Position><x>200</x><y>300</y></Position><Size><x>400</x><y>160</y></Size><leading>0</leading></ChainProperty>",
    "<TextLine Version=\"2\" objectID=\"2\"><BaseProperties Version=\"5\"><txBase>360</txBase><XPos>250</XPos><angle>0</angle></BaseProperties>",
    "<Alignment>center</Alignment><TRString>Big small</TRString><RunLengthEncodedCharacterAttributes>",
    "<CharacterAttributes RunCount=\"4\" StyleRef=\"4096\" TextRef=\"4096\"/><CharacterAttributes RunCount=\"6\" StyleRef=\"4097\" TextRef=\"4097\"/>",
    "</RunLengthEncodedCharacterAttributes></TextLine>",
    "<TextLine Version=\"2\" objectID=\"3\"><BaseProperties Version=\"5\"><txBase>440</txBase><XPos>300</XPos><angle>0</angle></BaseProperties>",
    "<Alignment>center</Alignment><TRString>below</TRString><RunLengthEncodedCharacterAttributes>",
    "<CharacterAttributes RunCount=\"6\" StyleRef=\"4096\" TextRef=\"4096\"/></RunLengthEncodedCharacterAttributes></TextLine>",
    "</TextChain></TextPage></Layer></Layers></Layout></InscriberLayouts></Adobe_Root>"
);

/// The value a title is stored in: 16 bytes of header, a tag, then the document as a zlib stream
/// of UTF-16LE with a byte-order mark.
fn legacy_title_prefs(document: &str) -> Vec<u8> {
    let text: Vec<u8> = std::iter::once(0xfeffu16).chain(document.encode_utf16()).flat_map(u16::to_le_bytes).collect();
    let mut bytes = 1u64.to_le_bytes().to_vec();
    bytes.extend((text.len() as u64).to_le_bytes());
    bytes.extend(b"CompressedTitle\0");
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(&text).unwrap();
    bytes.extend(encoder.finish().unwrap());
    bytes
}

/// `project_xml` with its media made a title of the Title Designer: a number where the path
/// belongs and the title's document in `ImporterPrefs` (`prefs` is that element, whole).
fn project_with_legacy_title(prefs: &str) -> String {
    let xml = project_xml().replace(
        "<Media ObjectUID=\"movie\"><FilePath>/media/example.mov</FilePath>",
        &format!("<Media ObjectUID=\"movie\"><FilePath>1414091852</FilePath><Title>Synthetic title</Title><Infinite>true</Infinite>{prefs}"),
    );
    assert!(xml.contains("1414091852"));
    xml
}

/// A title made in the Title Designer becomes a graphic clip with one text layer for each line
/// of its text: the words, the font, size, fill, edge and shadow of the line's first characters,
/// the line's alignment point on its own baseline (moved to where a 1280x720 title sits in a
/// 1920x1080 frame), and a style run for the characters that look different.
#[test]
fn premiere_legacy_title_becomes_a_text_layer() {
    use filmcraft_project::{CharStyle, ParamValue, StyleRun};
    let value = base64(&legacy_title_prefs(LEGACY_TITLE));
    let whole = project_with_legacy_title(&format!("<ImporterPrefs Encoding=\"base64\" BinaryHash=\"h1\">{value}</ImporterPrefs>"));
    // written once elsewhere in the file and named by its hash here
    let by_hash = project_with_legacy_title("<ImporterPrefs Encoding=\"base64\" BinaryHash=\"h1\"/>").replace(
        "</PremiereData>",
        &format!("<Media ObjectUID=\"spare\"><FilePath>1414091852</FilePath><ImporterPrefs Encoding=\"base64\" BinaryHash=\"h1\">{value}</ImporterPrefs></Media></PremiereData>"),
    );
    for xml in [whole, by_hash] {
        let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
        let said = report.to_string();
        assert!(said.contains("imported as FilmCraft text layers"), "{said}");
        let sequence = imported.project.sequence(imported.sequences[0]).unwrap();
        let video = &sequence.video_tracks[0].items[0];
        assert!(video.enabled, "a title whose text was read is shown");
        assert!(matches!(imported.project.item(video.item).unwrap().kind, ItemKind::Graphic { width: 1920, height: 1080, .. }));
        assert_eq!((video.start, video.duration, video.source_in), (Tick::ZERO, Tick(2 * TPS), Tick::ZERO));
        let layers: Vec<_> = video.graphic_layers().collect();
        assert_eq!(layers.len(), 2, "one for each line");
        let layer = layers[0];
        let text = |layer: &filmcraft_project::EffectInstance, id: &str| match &layer.param(id).unwrap().value {
            ParamValue::Text(s) => s.clone(),
            other => panic!("{id}: {other:?}"),
        };
        assert_eq!((text(layer, "text").as_str(), text(layer, "font").as_str(), text(layer, "font_style").as_str()), ("Big small", "Example Sans", "Bold"));
        assert_eq!(layer.f64_at("size", Tick::ZERO), 80.0);
        assert!(matches!(layer.param("align").unwrap().value, ParamValue::Choice(1)), "centred");
        assert!(matches!(layer.param("caps").unwrap().value, ParamValue::Choice(2)), "small capitals");
        // the middle of the block (200 + 400 / 2) on the line's baseline, in a frame 640x360 larger
        assert_eq!(layer.vec2_at("position", Tick::ZERO), Vec2::new(400.0 + 320.0, 360.0 + 180.0));
        assert!(matches!(layer.param("fill").unwrap().value, ParamValue::Bool(true)));
        assert!(matches!(layer.param("fill_color").unwrap().value, ParamValue::Color(c) if c == [1.0, 1.0, 0.0, 1.0]), "{:?}", layer.param("fill_color"));
        assert!(matches!(layer.param("stroke").unwrap().value, ParamValue::Bool(true)));
        assert!(matches!(layer.param("stroke_color").unwrap().value, ParamValue::Color(c) if c == [0.0, 0.0, 1.0, 1.0]));
        assert_eq!(layer.f64_at("stroke_width", Tick::ZERO), 6.0);
        assert!(matches!(layer.param("stroke2").unwrap().value, ParamValue::Bool(false)));
        assert!(matches!(layer.param("shadow").unwrap().value, ParamValue::Bool(true)));
        assert!(matches!(layer.param("shadow_color").unwrap().value, ParamValue::Color(c) if c == [0.0, 0.0, 0.0, 1.0]));
        assert!((layer.f64_at("shadow_opacity", Tick::ZERO) - 80.0).abs() < 0.001, "a fifth clear: {}", layer.f64_at("shadow_opacity", Tick::ZERO));
        assert_eq!(
            (
                layer.f64_at("shadow_distance", Tick::ZERO),
                layer.f64_at("shadow_angle", Tick::ZERO),
                layer.f64_at("shadow_size", Tick::ZERO),
                layer.f64_at("shadow_blur", Tick::ZERO)
            ),
            (12.0, 135.0, 3.0, 30.0)
        );
        // "small": the other font, half the size, red, no small capitals
        let runs = &layer.layer.as_ref().expect("a style run").runs;
        let other = CharStyle {
            font: Some("Example Serif".into()),
            font_style: Some("Regular".into()),
            size: Some(40.0),
            fill: Some([1.0, 0.0, 0.0, 1.0]),
            caps: Some(0),
            ..Default::default()
        };
        assert_eq!(runs, &[StyleRun { start: 4, end: 9, style: other }]);
        let spec = filmcraft_project::graphic::eval_layer(layer, Tick::ZERO, (1920, 1080)).expect("a text layer");
        assert!(matches!(&spec.content, filmcraft_project::graphic::LayerContent::Text(t) if t.text == "Big small" && t.size == 80.0));
        // the second line: its own layer on its own baseline, 80 lower, in one style
        let second = layers[1];
        assert_eq!((text(second, "text").as_str(), text(second, "font").as_str()), ("below", "Example Sans"));
        assert_eq!(second.vec2_at("position", Tick::ZERO), Vec2::new(400.0 + 320.0, 440.0 + 180.0));
        assert_eq!(second.f64_at("size", Tick::ZERO), 80.0);
        assert!(second.layer.as_ref().is_none_or(|extra| extra.runs.is_empty()));
        // the sound of the same source has nothing to play: it stays as it was, switched off
        assert!(!sequence.audio_tracks[0].items[0].enabled);
    }
}

/// A title that cannot be read, or has no text, leaves its clip as it was before titles were
/// read (switched off), and nothing in a damaged one is a panic or a failed import.
#[test]
fn premiere_legacy_title_that_is_damaged_never_panics_and_stays_a_placeholder() {
    let good = legacy_title_prefs(LEGACY_TITLE);
    let element = |bytes: &[u8]| format!("<ImporterPrefs Encoding=\"base64\">{}</ImporterPrefs>", base64(bytes));
    let import = |prefs: &str| {
        let xml = project_with_legacy_title(prefs);
        let result = std::panic::catch_unwind(|| premiere::import_project(xml.as_bytes(), &ImportOptions::default()));
        let Ok(Ok((imported, _))) = result else { panic!("not imported: {prefs}") };
        let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
        (video.enabled, video.graphic_layers().count())
    };
    // nothing where the title belongs, not base64, no tag, a stream cut short
    for prefs in ["<ImporterPrefs Encoding=\"base64\"/>", "<ImporterPrefs Encoding=\"base64\">***</ImporterPrefs>"] {
        assert_eq!(import(prefs), (false, 0), "{prefs}");
    }
    assert_eq!(import(&element(&good[..16])), (false, 0));
    assert_eq!(import(&element(b"0123456789abcdefNotATitleAtAll\0rest")), (false, 0));
    for cut in (0..good.len()).step_by(7) {
        assert_eq!(import(&element(&good[..cut])), (false, 0), "cut at {cut}");
    }
    for at in (32..good.len()).step_by(5) {
        let mut changed = good.clone();
        changed[at] ^= 0x55;
        import(&element(&changed));
    }
    // numbers that are not numbers, counts past the end, missing parts: read or left out
    let hostile = [
        ("RunCount=\"4\"", "RunCount=\"18446744073709551615\""),
        ("RunCount=\"6\"", "RunCount=\"-3\""),
        ("<pXPIXELS>1280</pXPIXELS>", "<pXPIXELS>NaN</pXPIXELS>"),
        ("<pYLINES>720</pYLINES>", "<pYLINES>1e999</pYLINES>"),
        ("<x>200</x>", "<x>1e999</x>"),
        ("<txBase>360</txBase>", "<txBase>inf</txBase>"),
        ("<txHeight>80</txHeight>", "<txHeight>-5</txHeight>"),
        ("<painterMix>15 15 </painterMix>", "<painterMix></painterMix>"),
        ("<size>6</size>", "<size>1e308</size>"),
        ("<angle>-45</angle>", "<angle>1e308</angle>"),
        ("<red>255</red>", "<red>-1e308</red>"),
        ("<xpar>51</xpar>", "<xpar>NaN</xpar>"),
        ("<TRString>Big small</TRString>", "<TRString/>"),
        ("<Shaders>", "<Shaders><Shader/>"),
        ("TextRef=\"4097\"", "TextRef=\"none\""),
        ("<Adobe_Root>", "<Adobe_Root><!DOCTYPE x [<!ENTITY a \"aaaa\">]>"),
        ("</Adobe_Root>", ""),
    ];
    for (from, to) in hostile {
        let document = LEGACY_TITLE.replace(from, to);
        assert_ne!(document, LEGACY_TITLE, "{from}");
        import(&element(&legacy_title_prefs(&document)));
    }
    // a title without words has no layer: the clip stays switched off
    let empty = LEGACY_TITLE.replace("<TRString>Big small</TRString>", "<TRString> </TRString>").replace("<TRString>below</TRString>", "<TRString></TRString>");
    assert_eq!(import(&element(&legacy_title_prefs(&empty))), (false, 0));
}

/// A colour matte or black video made inside the application is a generator here: it has no
/// file and needs none, its clip is shown, and the clip starts at the generator's own start with
/// its keyframes moved along. A matte whose colour cannot be read stays a placeholder.
#[test]
fn premiere_colour_matte_and_black_video_are_generators() {
    use filmcraft_media::Generator;
    use filmcraft_project::MediaRef;
    let with = |code: &str, title: &str, prefs: &str| {
        let xml = project_xml().replace(
            "<Media ObjectUID=\"movie\"><FilePath>/media/example.mov</FilePath>",
            &format!("<Media ObjectUID=\"movie\"><FilePath>{code}</FilePath><Title>{title}</Title><Infinite>true</Infinite>{prefs}"),
        );
        assert_ne!(xml, project_xml());
        premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap()
    };
    // red 255, green 0, blue 51, then the rest of the value as the files have it
    let colour = format!("<ImporterPrefs Encoding=\"base64\">{}</ImporterPrefs>", base64(&[0xff, 0x00, 0x33, 0x00, 1, 0, 0, 0]));
    for (code, title, prefs, want) in [
        ("1129270354", "Color Matte", colour.as_str(), Generator::ColorMatte { color: [1.0, 0.0, 0.2, 1.0] }),
        ("1112293707", "Black Video", "", Generator::BlackVideo),
    ] {
        let (imported, report) = with(code, title, prefs);
        let said = report.to_string();
        assert!(said.contains("imported as FilmCraft generators"), "{title}: {said}");
        assert!(!said.contains("offline placeholder"), "{title}: {said}");
        let sequence = imported.project.sequence(imported.sequences[0]).unwrap();
        let video = &sequence.video_tracks[0].items[0];
        assert!(video.enabled, "{title} is shown");
        let media = imported.project.item(video.item).unwrap().as_media().unwrap();
        assert!(matches!(&media.media, MediaRef::Generator(g) if *g == want), "{title}: {:?}", media.media);
        assert!(!media.offline, "{title}: a generator is never missing");
        let size = media.info.video.as_ref().map(|v| (v.width, v.height));
        assert_eq!(size, Some((1920, 1080)), "{title}: the size the file gives it");
        assert_eq!((video.start, video.duration, video.source_in, video.speed), (Tick::ZERO, Tick(2 * TPS), Tick::ZERO, 1.0), "{title}");
        // the clip's Motion keyframes were at 5 s and 7 s of a source without end
        let scale = video.effect("motion").unwrap().param("scale").unwrap();
        assert_eq!(scale.keyframes.iter().map(|k| k.time).collect::<Vec<_>>(), [Tick::ZERO, Tick(2 * TPS)], "{title}");
        assert!(!sequence.audio_tracks[0].items[0].enabled, "{title} has no sound");
    }
    // a matte without its colour, or with too little of it, is left as it was: a switched-off placeholder
    for prefs in ["", "<ImporterPrefs Encoding=\"base64\">/w==</ImporterPrefs>", "<ImporterPrefs Encoding=\"base64\">***</ImporterPrefs>"] {
        let (imported, report) = with("1129270354", "Color Matte", prefs);
        let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
        assert!(!video.enabled, "{prefs}");
        assert!(report.to_string().contains("offline placeholder"), "{prefs}");
    }
}

/// A title laid out for pixels that are not square says so (`pSCREENAR`, one over the pixel
/// aspect). In a sequence of the same pixels its letters are drawn that much narrower, so they
/// have their shape when the frame is shown, and every line stays where it was. In a sequence of
/// square pixels the letters are as they are and the title is that much wider about the middle.
#[test]
fn premiere_legacy_title_for_widescreen_pixels_keeps_its_shape() {
    use filmcraft_project::ParamValue;
    let document =
        LEGACY_TITLE.replace("<pXPIXELS>1280</pXPIXELS><pYLINES>720</pYLINES>", "<pXPIXELS>720</pXPIXELS><pYLINES>480</pYLINES><pSCREENAR>0.825</pSCREENAR>");
    assert_ne!(document, LEGACY_TITLE);
    let prefs = format!("<ImporterPrefs Encoding=\"base64\">{}</ImporterPrefs>", base64(&legacy_title_prefs(&document)));
    let first_layer = |xml: &str| {
        let (imported, _) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
        let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
        video.graphic_layers().next().expect("a text layer").clone()
    };
    // the same pixels as the title's: a 720x480 widescreen sequence (40:33)
    let same = project_with_legacy_title(&prefs).replace(
        "<VideoTrackGroup ObjectID=\"30\"><FrameRect>0,0,1920,1080</FrameRect>",
        "<VideoTrackGroup ObjectID=\"30\"><FrameRect>0,0,720,480</FrameRect><PixelAspectRatio>40,33</PixelAspectRatio>",
    );
    assert!(same.contains("40,33"));
    let layer = first_layer(&same);
    let place = layer.vec2_at("position", Tick::ZERO);
    assert!((place.x - 400.0).abs() < 0.01 && (place.y - 360.0).abs() < 0.01, "where the title has it: {place:?}");
    assert!(matches!(layer.param("uniform_scale").unwrap().value, ParamValue::Bool(false)));
    assert!((layer.f64_at("scale_width", Tick::ZERO) - 82.5).abs() < 0.01, "{}", layer.f64_at("scale_width", Tick::ZERO));
    assert_eq!(layer.f64_at("scale", Tick::ZERO), 100.0);
    // square pixels, 1920x1080: the title's picture is 1.2121 times as wide about the middle
    let layer = first_layer(&project_with_legacy_title(&prefs));
    let place = layer.vec2_at("position", Tick::ZERO);
    let want = 960.0 + (400.0 - 360.0) / 0.825;
    assert!((place.x - want).abs() < 0.01 && (place.y - (360.0 + 300.0)).abs() < 0.01, "{place:?} against {want}");
    assert!(matches!(layer.param("uniform_scale").unwrap().value, ParamValue::Bool(true)), "letters as they are");
}

/// A graphic kept in the project has its text layers on its master clip, not on the clips made
/// from it: a clip whose own chain has no text takes the master's.
#[test]
fn premiere_text_on_the_master_clip_of_a_graphic_becomes_a_text_layer() {
    use filmcraft_project::ParamValue;
    let on_the_clip = project_with_text_layer(&source_text_json(TITLE_JSON));
    let xml = on_the_clip
        .replace("<Component Index=\"0\" ObjectRef=\"64\"/><Component Index=\"1\" ObjectRef=\"80\"/></Components>", "<Component Index=\"0\" ObjectRef=\"64\"/></Components>")
        .replace("<SubClip ObjectID=\"61\"><Name>Example cut</Name>", "<SubClip ObjectID=\"61\"><MasterClip ObjectURef=\"movie-master\"/><Name>Example cut</Name>")
        .replace("<MasterClip ObjectUID=\"movie-master\"><Name>Example movie</Name>", "<MasterClip ObjectUID=\"movie-master\"><VideoComponentChain ObjectRef=\"90\"/><Name>Example movie</Name>")
        .replace(
            "</PremiereData>",
            "<VideoComponentChain ObjectID=\"90\"><ComponentChain><Components><Component Index=\"0\" ObjectRef=\"80\"/></Components></ComponentChain></VideoComponentChain></PremiereData>",
        );
    assert_eq!(xml.matches("ObjectRef=\"80\"").count(), 1, "the text component hangs on the master clip only");
    assert!(xml.contains("<MasterClip ObjectURef=\"movie-master\"/>") && xml.contains("<VideoComponentChain ObjectRef=\"90\"/>"));
    let (imported, report) = premiere::import_project(xml.as_bytes(), &ImportOptions::default()).unwrap();
    let said = report.to_string();
    assert!(said.contains("imported as FilmCraft text layers"), "{said}");
    let video = &imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0];
    assert!(video.enabled);
    let layers: Vec<_> = video.graphic_layers().collect();
    assert_eq!(layers.len(), 1);
    assert!(matches!(&layers[0].param("text").unwrap().value, ParamValue::Text(s) if s == "A synthetic title"));
    assert_eq!(layers[0].vec2_at("position", Tick::ZERO), Vec2::new(480.0, 540.0));
    // a master clip that is not there leaves the clip a switched-off placeholder, and the project imports
    let dangling = xml.replace("<VideoComponentChain ObjectRef=\"90\"/>", "<VideoComponentChain ObjectRef=\"999\"/>");
    let (imported, _) = premiere::import_project(dangling.as_bytes(), &ImportOptions::default()).unwrap();
    assert!(!imported.project.sequence(imported.sequences[0]).unwrap().video_tracks[0].items[0].enabled);
}
