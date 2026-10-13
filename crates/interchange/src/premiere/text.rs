//! Text layers of graphics made inside the application: the `AE.ADBE Text` component.
//!
//! Observed in user-authored project files of versions 34 to 38: the component's first
//! parameter, Source Text, holds base64 of an 8-byte header (the byte length of what follows as
//! a little-endian number, then zeros) and then a JSON document in UTF-16LE. Its `mTextParam`
//! object names the text, the font (by PostScript name), the size, alignment, fill, stroke and
//! shadow; the style values are lists of `[first character, value]` runs. The parameters after
//! Source Text are the layer's own position (normalised to the sequence frame), scale, rotation
//! and opacity. Newer versions write Source Text in a binary form that is not read here: such a
//! layer is reported and left out, as before.
//!
//! A file writes each distinct binary value once: a later parameter holding the same value has
//! an empty element that names it by its `BinaryHash` attribute ([`shared_values`]).

use std::collections::HashMap;

use filmcraft_project::graphic::{ALIGN_OPTS, CAPS_OPTS, TEXT_LAYER};
use filmcraft_project::{EffectInstance, Param, ParamValue, find_effect};
use filmcraft_time::Tick;
use roxmltree::Node;
use serde_json::Value;

use super::effects::{Conversion, parameter};
use super::graph::Graph;
use crate::{Report, Result};

pub(super) const MATCH_NAME: &str = "AE.ADBE Text";

/// Source Text larger than this (as base64 text) is not a title: it is not decoded.
const MAX_SOURCE_TEXT_BASE64: usize = 4 << 20;

/// The FilmCraft text layer for one `AE.ADBE Text` component, or `None` when its text cannot be
/// read (the caller reports it). Keyframe times are taken relative to `origin`; `shared` is the
/// document's [`shared_values`].
pub(super) fn layer(
    graph: &Graph<'_, '_>,
    node: Node<'_, '_>,
    origin: Tick,
    sequence_size: (u32, u32),
    shared: &HashMap<&str, &str>,
    report: &mut Report,
) -> Result<Option<EffectInstance>> {
    let Some(def) = find_effect(TEXT_LAYER) else { return Ok(None) };
    let parameters = graph.references(node, &["Component", "Params"])?;
    let mut source = None;
    for (index, native) in parameters.iter().enumerate() {
        if graph.integer(*native, &["ParameterID"], (index + 1) as i64)? == 1 {
            source = Some(*native);
        }
    }
    let Some(source) = source else { return Ok(None) };
    let encoded = source.children().find(|n| n.has_tag_name("StartKeyframeValue")).and_then(|value| {
        value.text().filter(|text| !text.trim().is_empty()).or_else(|| value.attribute("BinaryHash").and_then(|hash| shared.get(hash).copied()))
    });
    let Some(document) = encoded.and_then(source_text) else { return Ok(None) };
    let Some(text_param) = document.get("mTextParam") else { return Ok(None) };
    let sheet = text_param.get("mStyleSheet").unwrap_or(&Value::Null);
    let Some(text) = sheet.get("mText").and_then(Value::as_str) else { return Ok(None) };

    let mut layer = def.instance();
    layer.enabled = !graph.boolean(node, &["Component", "Bypass"], false)?;
    let mut set = |id: &str, value: ParamValue| {
        if let Some(p) = layer.params.get_mut(id) {
            *p = Param::new(value);
        }
    };
    set("text", ParamValue::Text(text.to_string()));
    let mut mixed = false;
    let mut style = |key: &str| first_run(sheet, key, &mut mixed);
    if let Some(name) = style("mFontName").and_then(Value::as_str) {
        let (family, face) = font(name);
        set("font", ParamValue::Text(family));
        set("font_style", ParamValue::Text(face));
    }
    if let Some(size) = style("mFontSize").and_then(number) {
        set("size", ParamValue::Float(size.clamp(1.0, 2000.0)));
    }
    if let Some(v) = style("mTracking").and_then(number) {
        set("tracking", ParamValue::Float(v.clamp(-1000.0, 10_000.0)));
    }
    if let Some(v) = style("mBaselineShift").and_then(number) {
        set("baseline_shift", ParamValue::Float(v.clamp(-5000.0, 5000.0)));
    }
    if let Some(v) = style("mFauxBold").and_then(Value::as_bool) {
        set("faux_bold", ParamValue::Bool(v));
    }
    if let Some(v) = style("mFauxItalic").and_then(Value::as_bool) {
        set("faux_italic", ParamValue::Bool(v));
    }
    if let Some(v) = style("mUnderline").and_then(Value::as_bool) {
        set("underline", ParamValue::Bool(v));
    }
    if let Some(v) = style("mCapsOption").and_then(choice(CAPS_OPTS.len())) {
        set("caps", ParamValue::Choice(v));
    }
    if let Some(v) = style("mFillVisible").and_then(Value::as_bool) {
        set("fill", ParamValue::Bool(v));
    }
    if let Some(c) = style("mFillColor").and_then(colour) {
        set("fill_color", ParamValue::Color(c));
    }
    let stroke_width = style("mStrokeWidth").and_then(number);
    if let Some(v) = style("mStrokeVisible").and_then(Value::as_bool) {
        set("stroke", ParamValue::Bool(v && stroke_width.is_none_or(|w| w > 0.0)));
    }
    if let Some(w) = stroke_width {
        set("stroke_width", ParamValue::Float(w.clamp(0.0, 1000.0)));
    }
    if let Some(c) = style("mStrokeColor").and_then(colour) {
        set("stroke_color", ParamValue::Color(c));
    }
    let of = |key: &str| text_param.get(key);
    if let Some(v) = of("mAlignment").and_then(choice(ALIGN_OPTS.len())) {
        set("align", ParamValue::Choice(v));
    }
    if let Some(v) = of("mLeading").and_then(number) {
        set("leading", ParamValue::Float(v.clamp(-5000.0, 5000.0)));
    }
    if let Some(v) = of("mLigatures").and_then(Value::as_bool) {
        set("ligatures", ParamValue::Bool(v));
    }
    if let Some(v) = of("mIsVerticalText").and_then(Value::as_bool) {
        set("vertical", ParamValue::Bool(v));
    }
    // a text box (paragraph text) has a size; point text has none
    if let (Some(w), Some(h)) = (of("mWidth").and_then(number), of("mHeight").and_then(number))
        && w > 0.0
        && h > 0.0
    {
        set("box_width", ParamValue::Float(w.min(100_000.0)));
        set("box_height", ParamValue::Float(h.min(100_000.0)));
    }
    if let Some(v) = of("mShadowVisible").and_then(Value::as_bool) {
        set("shadow", ParamValue::Bool(v));
    }
    if let Some(c) = of("mShadowColor").and_then(colour) {
        set("shadow_color", ParamValue::Color(c));
    }
    for (key, id, max) in [
        ("mShadowOpacity", "shadow_opacity", 100.0),
        ("mShadowOffset", "shadow_distance", 1000.0),
        ("mShadowSize", "shadow_size", 1000.0),
        ("mShadowBlur", "shadow_blur", 1000.0),
    ] {
        if let Some(v) = of(key).and_then(number) {
            set(id, ParamValue::Float(v.clamp(0.0, max)));
        }
    }
    if let Some(v) = of("mShadowAngle").and_then(number) {
        set("shadow_angle", ParamValue::Float(v.clamp(-36_000.0, 36_000.0)));
    }
    if let (Some(true), Some(c)) = (of("mBackFillVisible").and_then(Value::as_bool), of("mBackFillColor").and_then(colour)) {
        set("background", ParamValue::Bool(true));
        set("background_color", ParamValue::Color(c));
        if let Some(v) = of("mBackFillOpacity").and_then(number) {
            set("background_opacity", ParamValue::Float(v.clamp(0.0, 100.0)));
        }
        if let Some(v) = of("mBackFillSize").and_then(number) {
            set("background_size", ParamValue::Float(v.clamp(0.0, 1000.0)));
        }
    }
    if mixed {
        report.warn("Premiere text with more than one style in it was imported with the style of its first character");
    }
    if graph.boolean(source, &["IsTimeVarying"], false)? {
        report.warn("Premiere animated Source Text is not supported; its first text was used");
    }

    // the layer's own transform: the parameters after Source Text
    for (index, native) in parameters.into_iter().enumerate() {
        let number = graph.integer(native, &["ParameterID"], (index + 1) as i64)?;
        let mapped = match number {
            3 => Some(("position", Conversion::Point(sequence_size))),
            4 => Some(("scale", Conversion::Native)),
            5 => Some(("scale_width", Conversion::Native)),
            6 => Some(("uniform_scale", Conversion::Native)),
            7 => Some(("rotation", Conversion::Native)),
            8 => Some(("opacity", Conversion::Native)),
            _ => None,
        };
        let Some((target, conversion)) = mapped else { continue };
        let Some(default) = layer.params.get(target).map(|p| p.value.clone()) else { continue };
        match parameter(graph, native, origin, &default, conversion, MATCH_NAME, report) {
            Ok(param) => {
                layer.params.insert(target.to_string(), param);
            }
            Err(_) if !graph.exhausted() => {
                report.warn(format!("Premiere text layer parameter {number} could not be read; the FilmCraft default was used"));
            }
            Err(e) => return Err(e),
        }
    }
    Ok(Some(layer))
}

/// The binary parameter values of a document by their `BinaryHash`: the file writes a value in
/// full the first time and leaves the element empty, with the same hash, every time after.
pub(super) fn shared_values<'a>(root: Node<'a, '_>) -> HashMap<&'a str, &'a str> {
    let mut values = HashMap::new();
    for node in root.descendants() {
        if node.has_tag_name("StartKeyframeValue")
            && let Some(hash) = node.attribute("BinaryHash")
            && let Some(text) = node.text().filter(|text| !text.trim().is_empty())
        {
            values.entry(hash).or_insert(text);
        }
    }
    values
}

/// A style value: its first run's. `mixed` is set when a later run differs (more than one style
/// in one text).
fn first_run<'a>(sheet: &'a Value, key: &str, mixed: &mut bool) -> Option<&'a Value> {
    let runs = sheet.get(key)?.get("mParamValues")?.as_array()?;
    let first = runs.first()?.get(1)?;
    *mixed |= runs.iter().skip(1).any(|run| run.get(1) != Some(first));
    Some(first)
}

/// The JSON document inside a Source Text value, or `None` for anything else (the binary form
/// newer versions write, damaged data, text that is too long to be a title).
fn source_text(base64: &str) -> Option<Value> {
    if base64.len() > MAX_SOURCE_TEXT_BASE64 {
        return None;
    }
    let bytes = decode_base64(base64)?;
    let body = bytes.get(8..)?;
    let declared = u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?) as usize;
    if declared != body.len() || body.len() % 2 != 0 {
        return None;
    }
    let units: Vec<u16> = body.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    let text = String::from_utf16(&units).ok()?;
    serde_json::from_str(&text).ok().filter(Value::is_object)
}

/// Standard base64 with `=` padding; white space between characters is skipped.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut bits, mut count) = (0u32, 0u32);
    for byte in text.bytes() {
        let six = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b' ' | b'\t' | b'\r' | b'\n' => continue,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(six);
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    Some(out)
}

fn number(value: &Value) -> Option<f64> {
    value.as_f64().filter(|v| v.is_finite())
}

/// A choice stored as a small number, when it is one of `options` choices.
fn choice(options: usize) -> impl Fn(&Value) -> Option<u32> {
    move |value| value.as_u64().filter(|v| (*v as usize) < options).map(|v| v as u32)
}

/// A colour stored as one 24-bit number, read as red, green, blue from the high byte down.
/// Black, white and greys are certain; for the others the byte order is an assumption (no file
/// seen so far settles it).
fn colour(value: &Value) -> Option<[f32; 4]> {
    let v = value.as_u64().filter(|v| *v <= 0xff_ffff)?;
    let channel = |shift: u32| ((v >> shift) & 0xff) as f32 / 255.0;
    Some([channel(16), channel(8), channel(0), 1.0])
}

/// Family and face from a PostScript font name (`Family-Face`; no face means Regular), each with
/// its spaces put back: fonts are looked up by family name here.
fn font(postscript: &str) -> (String, String) {
    let (family, face) = match postscript.rsplit_once('-') {
        Some((family, face)) if !family.is_empty() && !face.is_empty() => (family, face),
        _ => (postscript, "Regular"),
    };
    (spaced(family), spaced(face))
}

/// A PostScript name has no spaces. Put one where a capital or a number starts a new word
/// (`MinionPro` is Minion Pro, `TrajanPro3` is Trajan Pro 3), after dropping the `PSMT`, `PS` or
/// `MT` some foundries append (`CourierNewPSMT` is Courier New). A guess: a family this does
/// not find is shown in the stand-in font and listed as missing, like any font not installed.
fn spaced(name: &str) -> String {
    let stem = ["PSMT", "PS", "MT"].iter().find_map(|tail| name.strip_suffix(tail).filter(|rest| rest.chars().last().is_some_and(char::is_lowercase)));
    let mut out = String::with_capacity(name.len() + 4);
    let mut before: Option<char> = None;
    for c in stem.unwrap_or(name).chars() {
        if before.is_some_and(|b| (b.is_lowercase() && c.is_uppercase()) || (b.is_alphabetic() && c.is_ascii_digit())) {
            out.push(' ');
        }
        out.push(c);
        before = Some(c);
    }
    out
}

/// Move every keyframe of `effect` earlier by `by` (a clip whose source start is set to zero).
pub(super) fn shift_keyframes(effect: &mut EffectInstance, by: Tick) {
    for param in effect.params.values_mut() {
        for key in &mut param.keyframes {
            key.time = Tick(key.time.0.saturating_sub(by.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postscript_names_get_their_spaces_back() {
        let pairs = [
            ("MinionPro-Regular", ("Minion Pro", "Regular")),
            ("TrajanPro3-Regular", ("Trajan Pro 3", "Regular")),
            ("Futura-CondensedMedium", ("Futura", "Condensed Medium")),
            ("CourierNewPSMT", ("Courier New", "Regular")),
            ("ArialMT", ("Arial", "Regular")),
            ("Candara", ("Candara", "Regular")),
            ("DIN-Bold", ("DIN", "Bold")),
            ("-", ("-", "Regular")),
            ("", ("", "Regular")),
        ];
        for (name, (family, face)) in pairs {
            assert_eq!(font(name), (family.to_string(), face.to_string()), "{name}");
        }
    }

    #[test]
    fn base64_and_source_text_refuse_what_they_cannot_read() {
        assert_eq!(decode_base64("eA==").as_deref(), Some(&b"x"[..]));
        assert_eq!(decode_base64("e A\n=="), Some(b"x".to_vec()));
        assert_eq!(decode_base64("e*A="), None);
        for bad in ["", "eA==", "AAAAAAAAAAA=", "BAAAAAAAAAB7AH0A"] {
            assert!(source_text(bad).is_none() || bad == "BAAAAAAAAAB7AH0A", "{bad}");
        }
        // four bytes of UTF-16LE: `{}`
        assert!(source_text("BAAAAAAAAAB7AH0A").is_some_and(|v| v.is_object()));
    }
}
