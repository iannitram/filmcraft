//! Text subtitle tracks as caption cues. Fixtures are built here with a minimal EBML writer (no
//! committed media); one oracle test compares against ffmpeg when it is installed.

use filmcraft_captions::{Cue, srt};
use filmcraft_matroska::{SubtitleError, SubtitleFormat, SubtitleTrack, list_subtitle_tracks, read_subtitle_cues};
use filmcraft_time::{TICKS_PER_SECOND, Tick};

const ID_EBML: &[u8] = &[0x1A, 0x45, 0xDF, 0xA3];
const ID_DOC_TYPE: &[u8] = &[0x42, 0x82];
const ID_DOC_TYPE_VERSION: &[u8] = &[0x42, 0x87];
const ID_DOC_TYPE_READ_VERSION: &[u8] = &[0x42, 0x85];
const ID_SEGMENT: &[u8] = &[0x18, 0x53, 0x80, 0x67];
const ID_INFO: &[u8] = &[0x15, 0x49, 0xA9, 0x66];
const ID_TIMESTAMP_SCALE: &[u8] = &[0x2A, 0xD7, 0xB1];
const ID_TRACKS: &[u8] = &[0x16, 0x54, 0xAE, 0x6B];
const ID_TRACK_ENTRY: &[u8] = &[0xAE];
const ID_TRACK_NUMBER: &[u8] = &[0xD7];
const ID_TRACK_UID: &[u8] = &[0x73, 0xC5];
const ID_TRACK_TYPE: &[u8] = &[0x83];
const ID_FLAG_DEFAULT: &[u8] = &[0x88];
const ID_FLAG_FORCED: &[u8] = &[0x55, 0xAA];
const ID_CODEC_ID: &[u8] = &[0x86];
const ID_LANGUAGE: &[u8] = &[0x22, 0xB5, 0x9C];
const ID_NAME: &[u8] = &[0x53, 0x6E];
const ID_CLUSTER: &[u8] = &[0x1F, 0x43, 0xB6, 0x75];
const ID_TIMECODE: &[u8] = &[0xE7];
const ID_SIMPLE_BLOCK: &[u8] = &[0xA3];
const ID_BLOCK_GROUP: &[u8] = &[0xA0];
const ID_BLOCK: &[u8] = &[0xA1];
const ID_BLOCK_DURATION: &[u8] = &[0x9B];

const TRACK_AUDIO: u64 = 2;
const TRACK_SUBTITLE: u64 = 0x11;

/// Element with an 8-byte size field (always valid, whatever the body length).
fn el(id: &[u8], body: &[u8]) -> Vec<u8> {
    let mut v = id.to_vec();
    v.push(0x01);
    v.extend_from_slice(&(body.len() as u64).to_be_bytes()[1..]);
    v.extend_from_slice(body);
    v
}

fn uint(id: &[u8], value: u64) -> Vec<u8> {
    el(id, &value.to_be_bytes())
}

fn text(id: &[u8], value: &str) -> Vec<u8> {
    el(id, value.as_bytes())
}

fn track(number: u64, kind: u64, codec: &str, language: Option<&str>, name: Option<&str>, default: bool, forced: bool) -> Vec<u8> {
    let mut body = uint(ID_TRACK_NUMBER, number);
    body.extend(uint(ID_TRACK_UID, 1000 + number));
    body.extend(uint(ID_TRACK_TYPE, kind));
    body.extend(uint(ID_FLAG_DEFAULT, u64::from(default)));
    body.extend(uint(ID_FLAG_FORCED, u64::from(forced)));
    body.extend(text(ID_CODEC_ID, codec));
    if let Some(l) = language {
        body.extend(text(ID_LANGUAGE, l));
    }
    if let Some(n) = name {
        body.extend(text(ID_NAME, n));
    }
    el(ID_TRACK_ENTRY, &body)
}

/// One block: `duration` in ticks makes it a BlockGroup with BlockDuration, `None` a SimpleBlock.
struct Blk {
    track: u64,
    time: i16,
    duration: Option<u64>,
    data: Vec<u8>,
}

fn blk(track: u64, time: i16, duration: Option<u64>, data: &str) -> Blk {
    Blk { track, time, duration, data: data.as_bytes().to_vec() }
}

fn block_body(b: &Blk, keyframe_flag: u8) -> Vec<u8> {
    let mut body = vec![0x80 | b.track as u8];
    body.extend_from_slice(&b.time.to_be_bytes());
    body.push(keyframe_flag);
    body.extend_from_slice(&b.data);
    body
}

/// Assembles a Segment with the given TimestampScale (ns per tick), tracks and clusters.
fn mkv(timestamp_scale: u64, tracks: &[Vec<u8>], clusters: &[(u64, Vec<Blk>)]) -> Vec<u8> {
    let mut header = text(ID_DOC_TYPE, "matroska");
    header.extend(uint(ID_DOC_TYPE_VERSION, 4));
    header.extend(uint(ID_DOC_TYPE_READ_VERSION, 2));
    let mut out = el(ID_EBML, &header);

    let mut segment = el(ID_INFO, &uint(ID_TIMESTAMP_SCALE, timestamp_scale));
    segment.extend(el(ID_TRACKS, &tracks.concat()));
    for (timecode, blocks) in clusters {
        let mut body = uint(ID_TIMECODE, *timecode);
        for b in blocks {
            match b.duration {
                None => body.extend(el(ID_SIMPLE_BLOCK, &block_body(b, 0x80))),
                Some(d) => {
                    let mut group = el(ID_BLOCK, &block_body(b, 0));
                    group.extend(uint(ID_BLOCK_DURATION, d));
                    body.extend(el(ID_BLOCK_GROUP, &group));
                }
            }
        }
        segment.extend(el(ID_CLUSTER, &body));
    }
    out.extend(el(ID_SEGMENT, &segment));
    out
}

/// `ms` milliseconds as a caption tick.
fn ms(v: i64) -> Tick {
    Tick(v * (TICKS_PER_SECOND / 1000))
}

fn span(c: &Cue) -> (i64, i64, &str) {
    (c.start.0 / (TICKS_PER_SECOND / 1000), c.end.0 / (TICKS_PER_SECOND / 1000), c.text.as_str())
}

const MS_SCALE: u64 = 1_000_000;

fn srt_and_ass_file() -> Vec<u8> {
    let tracks = [
        track(1, TRACK_SUBTITLE, "S_TEXT/UTF8", Some("eng"), Some("English"), true, false),
        track(2, TRACK_SUBTITLE, "S_TEXT/ASS", Some("fre"), Some("Styled"), false, true),
        track(3, TRACK_SUBTITLE, "S_HDMV/PGS", Some("eng"), None, false, false),
    ];
    let clusters = [(0, vec![blk(1, 200, None, "Hello"), blk(2, 500, Some(1000), "0,0,Default,,0,0,0,,Bonjour")])];
    mkv(MS_SCALE, &tracks, &clusters)
}

#[test]
fn lists_text_subtitle_tracks_with_language_and_name() {
    let tracks = list_subtitle_tracks(srt_and_ass_file().as_slice()).unwrap();
    assert_eq!(
        tracks,
        vec![
            SubtitleTrack { number: 1, format: SubtitleFormat::SubRip, language: "eng".into(), name: "English".into(), default: true, forced: false },
            SubtitleTrack { number: 2, format: SubtitleFormat::Ass, language: "fre".into(), name: "Styled".into(), default: false, forced: true },
        ],
        "bitmap (PGS) track 3 is left out"
    );
}

#[test]
fn file_without_subtitle_track_lists_none() {
    let bytes = mkv(MS_SCALE, &[track(1, TRACK_AUDIO, "A_PCM/INT/LIT", Some("eng"), None, true, false)], &[(0, vec![blk(1, 0, None, "pcm")])]);
    assert!(list_subtitle_tracks(bytes.as_slice()).unwrap().is_empty());
    assert!(matches!(read_subtitle_cues(bytes.as_slice(), 1), Err(SubtitleError::NotTextSubtitle(1))));
}

#[test]
fn unknown_track_number_is_an_error() {
    assert!(matches!(read_subtitle_cues(srt_and_ass_file().as_slice(), 9), Err(SubtitleError::NoSuchTrack(9))));
}

#[test]
fn srt_track_cues_use_block_duration_and_cluster_timecode() {
    let tracks = [track(1, TRACK_SUBTITLE, "S_TEXT/UTF8", Some("eng"), None, true, false)];
    let clusters = [
        (0, vec![blk(1, 200, None, "Hello"), blk(1, 1000, Some(500), "World\n<i>two</i>")]),
        // a second cluster: its Timecode is added to the block offset
        (2000, vec![blk(1, 1000, None, "last")]),
    ];
    let cues = read_subtitle_cues(mkv(MS_SCALE, &tracks, &clusters).as_slice(), 1).unwrap();
    let got: Vec<_> = cues.iter().map(span).collect();
    assert_eq!(
        got,
        vec![
            (200, 1000, "Hello"),              // no duration: ends where the next cue starts
            (1000, 1500, "World\n<i>two</i>"), // BlockDuration 500 ms
            (3000, 5000, "last"),              // last cue, no duration: 2 s fallback
        ]
    );
    assert_eq!(cues[1].start, ms(1000));
    assert_eq!(cues[1].end, ms(1500));
}

#[test]
fn ass_track_text_is_stripped_to_plain_cues() {
    let cues = read_subtitle_cues(srt_and_ass_file().as_slice(), 2).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(span(&cues[0]), (500, 1500, "Bonjour"));

    let tracks = [track(2, TRACK_SUBTITLE, "S_TEXT/ASS", None, None, false, false)];
    let clusters = [(0, vec![blk(2, 100, Some(900), "0,0,Default,,0,0,0,,{\\an8}Top\\Nline two"), blk(2, 1500, Some(800), "1,0,Default,,0,0,0,,Bye, friend")])];
    let cues = read_subtitle_cues(mkv(MS_SCALE, &tracks, &clusters).as_slice(), 2).unwrap();
    let got: Vec<_> = cues.iter().map(span).collect();
    assert_eq!(got, vec![(100, 1000, "Top\nline two"), (1500, 2300, "Bye, friend")]);
}

#[test]
fn nonstandard_timestamp_scale_scales_timecodes_and_durations() {
    // 0.5 ms ticks: cluster 2000 ticks = 1000 ms, block +2 ticks = 1 ms, BlockDuration 2000 ticks = 1000 ms
    let tracks = [track(1, TRACK_SUBTITLE, "S_TEXT/UTF8", None, None, true, false)];
    let clusters = [(2000, vec![blk(1, 2, Some(2000), "scaled")])];
    let cues = read_subtitle_cues(mkv(500_000, &tracks, &clusters).as_slice(), 1).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].start, ms(1001));
    assert_eq!(cues[0].end, ms(2001));
}

#[test]
fn file_bytes_are_read_through_a_file() {
    // read_subtitle_cues takes any ByteSource: a file on disk works the same
    let path = std::env::temp_dir().join(format!("filmcraft-mkv-subs-file-{}.mkv", std::process::id()));
    std::fs::write(&path, srt_and_ass_file()).unwrap();
    let file = std::fs::File::open(&path).unwrap();
    let cues = read_subtitle_cues(&file, 1).unwrap();
    let _ = std::fs::remove_file(&path);
    // the lone SRT cue has no duration and no following cue: 2 s fallback
    assert_eq!(cues.iter().map(span).collect::<Vec<_>>(), vec![(200, 2200, "Hello")]);
}

/// Removes the scratch directory when dropped.
struct ScratchDir(std::path::PathBuf);

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const ORACLE_SRT: &str =
    "1\n00:00:00,200 --> 00:00:00,900\nHello\n\n2\n00:00:01,000 --> 00:00:01,500\nWorld <i>two</i>\n\n3\n00:00:02,123 --> 00:00:03,456\nThird cue\n\n";

/// ffmpeg muxes the same cues as SRT and as ASS; reading our cues back must match ffmpeg's own SRT
/// output of each file to within 1 ms. Skipped (with a note) when ffmpeg is not installed.
#[test]
fn ffmpeg_oracle_matches_srt_and_ass_cue_times() {
    let Some(ffmpeg) = filmcraft_testkit::ffmpeg_or_skip("matroska subtitle oracle") else { return };
    let dir = ScratchDir(std::env::temp_dir().join(format!("filmcraft-mkv-subs-oracle-{}", std::process::id())));
    std::fs::create_dir_all(&dir.0).unwrap();
    std::fs::write(dir.0.join("cues.srt"), ORACLE_SRT).unwrap();

    for codec in ["srt", "ass"] {
        let mkv_path = dir.0.join(format!("{codec}.mkv"));
        let out_srt = dir.0.join(format!("{codec}-back.srt"));
        let run = |args: &[&str]| {
            let st = std::process::Command::new(&ffmpeg).args(["-y", "-v", "error"]).args(args).status().unwrap();
            assert!(st.success(), "ffmpeg {args:?} failed");
        };
        run(&["-i", dir.0.join("cues.srt").to_str().unwrap(), "-c:s", codec, mkv_path.to_str().unwrap()]);
        run(&["-i", mkv_path.to_str().unwrap(), "-map", "0:s:0", out_srt.to_str().unwrap()]);

        let bytes = std::fs::read(&mkv_path).unwrap();
        let tracks = list_subtitle_tracks(bytes.as_slice()).unwrap();
        assert_eq!(tracks.len(), 1, "{codec}: one text subtitle track");
        let ours = read_subtitle_cues(bytes.as_slice(), tracks[0].number).unwrap();
        let theirs = srt::parse(&std::fs::read_to_string(&out_srt).unwrap()).unwrap().cues;

        assert_eq!(ours.len(), theirs.len(), "{codec}: cue count");
        assert_eq!(ours.len(), 3, "{codec}: three cues");
        let tol = TICKS_PER_SECOND / 1000;
        for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
            assert!((a.start.0 - b.start.0).abs() <= tol, "{codec}: cue {i} start {:?} vs ffmpeg {:?}", a.start, b.start);
            assert!((a.end.0 - b.end.0).abs() <= tol, "{codec}: cue {i} end {:?} vs ffmpeg {:?}", a.end, b.end);
        }
    }
}
