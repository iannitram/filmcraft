//! Text subtitle tracks read as caption cues.
//!
//! [`list_subtitle_tracks`] finds the text subtitle tracks of a Matroska or WebM file: TrackType 17
//! with codec `S_TEXT/UTF8` (SubRip), `S_TEXT/WEBVTT`, `S_TEXT/ASS` or `S_TEXT/SSA`. Bitmap
//! subtitles (PGS, VobSub) are not text and are left out. [`read_subtitle_cues`] reads one track as
//! [`Cue`]s.
//!
//! Timing: a cue starts at its block timestamp (cluster `Timecode` plus the block offset, in
//! `TimestampScale` ticks) and ends `BlockDuration` later. A block with no duration ends where the
//! next cue of the track starts, or [`FALLBACK_CUE_NS`] after its start when it is the last one.
//!
//! ASS and SSA blocks carry `ReadOrder,Layer,Style,Name,MarginL,MarginR,MarginV,Effect,Text`: only
//! the text is kept, with `{...}` override tags removed and `\N` turned into a newline.

use crate::Cue;
use filmcraft_matroska::{ByteSource, OpenOptions, Track, TrackKind, open, open_with};
use filmcraft_time::{TICKS_PER_SECOND, Tick};

/// End time, after its start, of a cue that has no duration and no following cue (2 s).
pub const FALLBACK_CUE_NS: i64 = 2_000_000_000;

/// The text subtitle formats this module reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubtitleFormat {
    /// `S_TEXT/UTF8`.
    SubRip,
    /// `S_TEXT/WEBVTT`.
    WebVtt,
    /// `S_TEXT/ASS`.
    Ass,
    /// `S_TEXT/SSA`.
    Ssa,
}

impl SubtitleFormat {
    fn from_codec_id(id: &str) -> Option<Self> {
        match id {
            "S_TEXT/UTF8" => Some(Self::SubRip),
            "S_TEXT/WEBVTT" => Some(Self::WebVtt),
            "S_TEXT/ASS" => Some(Self::Ass),
            "S_TEXT/SSA" => Some(Self::Ssa),
            _ => None,
        }
    }
}

/// A text subtitle track of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubtitleTrack {
    /// Matroska `TrackNumber`; pass it to [`read_subtitle_cues`].
    pub number: u64,
    pub format: SubtitleFormat,
    /// `Language` (ISO 639-2; Matroska defaults to `eng`).
    pub language: String,
    /// `Name` of the track, empty if the file has none.
    pub name: String,
    pub default: bool,
    pub forced: bool,
}

/// Failure to list or read subtitle tracks.
#[derive(Debug, thiserror::Error)]
pub enum SubtitleError {
    #[error("Matroska: {0}")]
    Matroska(#[from] filmcraft_matroska::Error),
    #[error("no track with number {0}")]
    NoSuchTrack(u64),
    #[error("track {0} is not a text subtitle track (SubRip, WebVTT, ASS or SSA)")]
    NotTextSubtitle(u64),
}

pub type SubtitleResult<T> = Result<T, SubtitleError>;

fn text_format(t: &Track) -> Option<SubtitleFormat> {
    if t.kind == TrackKind::Subtitle { SubtitleFormat::from_codec_id(&t.codec_id) } else { None }
}

/// Lists the text subtitle tracks of a Matroska or WebM file, in file order. A file without one
/// gives an empty list.
pub fn list_subtitle_tracks<S: ByteSource>(src: S) -> SubtitleResult<Vec<SubtitleTrack>> {
    let file = open_with(&src, &OpenOptions { index: false, verify_crc: false })?;
    Ok(file
        .tracks
        .iter()
        .filter_map(|t| {
            let format = text_format(t)?;
            Some(SubtitleTrack { number: t.number, format, language: t.language.clone(), name: t.name.clone(), default: t.default, forced: t.forced })
        })
        .collect())
}

/// Reads every cue of one text subtitle track, sorted by start time.
pub fn read_subtitle_cues<S: ByteSource>(src: S, number: u64) -> SubtitleResult<Vec<Cue>> {
    let file = open(&src)?;
    let index = file.track_by_number(number).ok_or(SubtitleError::NoSuchTrack(number))?;
    let track = file.tracks.get(index).ok_or(SubtitleError::NoSuchTrack(number))?;
    let format = text_format(track).ok_or(SubtitleError::NotTextSubtitle(number))?;

    // (start ns, duration ns or 0 if unknown, text)
    let mut blocks: Vec<(i64, i64, String)> = Vec::with_capacity(track.samples.len());
    for (i, s) in track.samples.iter().enumerate() {
        let data = file.read_sample(&src, index, i)?;
        let start = track.ticks_to_ns(s.pts);
        let duration = track.ticks_to_ns(i64::try_from(s.duration).unwrap_or(i64::MAX));
        blocks.push((start, duration, payload_text(format, &data)));
    }
    blocks.sort_by_key(|b| b.0);

    let mut cues = Vec::with_capacity(blocks.len());
    for (k, (start, duration, text)) in blocks.iter().enumerate() {
        let end = if *duration > 0 {
            start.saturating_add(*duration)
        } else {
            match blocks.get(k + 1) {
                Some(&(next, _, _)) if next > *start => next,
                _ => start.saturating_add(FALLBACK_CUE_NS),
            }
        };
        cues.push(Cue { start: ns_to_tick(*start), end: ns_to_tick(end), text: text.clone(), ..Default::default() });
    }
    Ok(cues)
}

/// Nanoseconds to a caption [`Tick`], rounded to the nearest tick.
fn ns_to_tick(ns: i64) -> Tick {
    let scaled = i128::from(ns) * i128::from(TICKS_PER_SECOND);
    let rounded = (scaled + 500_000_000).div_euclid(1_000_000_000);
    Tick(rounded.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
}

fn payload_text(format: SubtitleFormat, data: &[u8]) -> String {
    let text = String::from_utf8_lossy(data);
    match format {
        SubtitleFormat::Ass | SubtitleFormat::Ssa => ass_plain_text(&text),
        SubtitleFormat::SubRip => text.trim_end_matches(['\r', '\n']).to_string(),
        SubtitleFormat::WebVtt => webvtt_cue_text(&text),
    }
}

/// The cue text of one WebVTT block payload. The block's timing comes from Matroska, so an
/// optional cue identifier (the line just before the timing line) and the timing line itself
/// (including its cue settings) are dropped. Payloads without a timing line in the first two
/// lines are plain cue text and keep their interior line endings.
fn webvtt_cue_text(payload: &str) -> String {
    let lines: Vec<&str> = payload.lines().collect();
    let timing = match (lines.first(), lines.get(1)) {
        (Some(first), _) if first.contains("-->") => Some(0),
        (Some(_), Some(second)) if second.contains("-->") => Some(1),
        _ => None,
    };
    let body = match timing {
        Some(index) => lines.get(index + 1..).unwrap_or_default().join("\n"),
        None => payload.to_string(),
    };
    body.trim_end_matches(['\r', '\n']).to_string()
}

/// The plain text of one ASS/SSA block payload: the text after the eight leading fields, with
/// `{...}` override blocks removed, `\N` as a newline, and `\n` and `\h` as spaces.
pub fn ass_plain_text(payload: &str) -> String {
    let text = payload.splitn(9, ',').nth(8).unwrap_or_default();
    let mut visible = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        match (in_tag, c) {
            (false, '{') => in_tag = true,
            (true, '}') => in_tag = false,
            (false, c) => visible.push(c),
            (true, _) => {}
        }
    }
    let mut out = String::with_capacity(visible.len());
    let mut chars = visible.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('N') => {
                out.push('\n');
                chars.next();
            }
            Some('n' | 'h') => {
                out.push(' ');
                chars.next();
            }
            _ => out.push('\\'),
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ass_keeps_text_after_eight_fields() {
        assert_eq!(ass_plain_text("0,0,Default,,0,0,0,,Hello"), "Hello");
        assert_eq!(ass_plain_text("7,1,Sign,Actor,10,20,30,fx,Bye, now, friend"), "Bye, now, friend");
    }

    #[test]
    fn ass_strips_overrides_and_maps_breaks() {
        assert_eq!(ass_plain_text("0,0,Default,,0,0,0,,{\\an8}{\\i1}Top\\Nline two\\hend"), "Top\nline two end");
        assert_eq!(ass_plain_text("0,0,Default,,0,0,0,,soft\\nbreak"), "soft break");
        assert_eq!(ass_plain_text("0,0,Default,,0,0,0,,{\\b1}bold{\\b0}"), "bold");
    }

    #[test]
    fn ass_short_payload_is_empty() {
        assert_eq!(ass_plain_text("0,0,Default"), "");
    }

    #[test]
    fn ticks_round_to_nearest() {
        assert_eq!(ns_to_tick(1_000_000_000), Tick(TICKS_PER_SECOND));
        assert_eq!(ns_to_tick(0), Tick(0));
        assert_eq!(ns_to_tick(-1_000_000_000), Tick(-TICKS_PER_SECOND));
    }

    #[test]
    fn webvtt_drops_identifier_and_timing_line() {
        assert_eq!(webvtt_cue_text("intro\n00:01.000 --> 00:02.000\nHello"), "Hello");
    }

    #[test]
    fn webvtt_without_identifier_drops_timing_line() {
        assert_eq!(webvtt_cue_text("00:01.000 --> 00:02.000\nHello\nworld"), "Hello\nworld");
    }

    #[test]
    fn webvtt_timing_line_with_settings_is_dropped() {
        assert_eq!(webvtt_cue_text("00:01.000 --> 00:02.000 align:start line:0\nHello"), "Hello");
    }

    #[test]
    fn webvtt_numeric_text_line_is_kept() {
        assert_eq!(webvtt_cue_text("1\n00:01.000 --> 00:02.000\n42"), "42");
        assert_eq!(webvtt_cue_text("42\nhello"), "42\nhello");
    }

    #[test]
    fn webvtt_plain_text_is_unchanged() {
        assert_eq!(webvtt_cue_text("Hello\r\nworld\r\n"), "Hello\r\nworld");
    }

    #[test]
    fn webvtt_crlf_framing() {
        assert_eq!(webvtt_cue_text("id\r\n00:01.000 --> 00:02.000 align:start\r\nHello\r\n"), "Hello");
    }
}
