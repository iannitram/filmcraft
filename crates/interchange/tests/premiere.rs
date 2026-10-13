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
    let xml = project_xml().replace("<ParameterID>6</ParameterID><StartKeyframe>-91445760000000000,0.5:0.5,", "<ParameterID>6</ParameterID><StartKeyframe>-91445760000000000,oops:0.5,");
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
        .replace("<Component Index=\"0\" ObjectRef=\"64\"/></Components>", "<Component Index=\"0\" ObjectRef=\"64\"/><Component Index=\"1\" ObjectRef=\"80\"/></Components>")
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
    let xml = project_xml().replace("<VideoStream ObjectID=\"40\"><FrameRate>10594584000</FrameRate>", "<VideoStream ObjectID=\"40\"><FrameRate>2822400</FrameRate>");
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
    let preset_twice = preset_xml(0).replace("</PremiereData>", "<TreeItem ObjectID=\"1\"><TreeItemBase><Name>Again</Name></TreeItemBase></TreeItem></PremiereData>");
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
            let effect = format!(
                "<VideoFilterComponent ObjectID=\"80\"><MatchName>{match_name}</MatchName><Component><Bypass>{bypass}</Bypass><Params/></Component></VideoFilterComponent>"
            );
            let xml = project_xml()
                .replace("<Component Index=\"0\" ObjectRef=\"64\"/></Components>", "<Component Index=\"0\" ObjectRef=\"64\"/><Component Index=\"1\" ObjectRef=\"80\"/></Components>")
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
