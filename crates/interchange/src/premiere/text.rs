//! Text layers of graphics made inside the application: the `AE.ADBE Text` component.
//!
//! Observed in user-authored project files: the component's first parameter, Source Text, holds
//! base64 of an 8-byte header (a byte length as a little-endian number, then zeros) and then the
//! text in one of two forms. The length is that of everything after the header in the first
//! form, and of what follows the marker in the second.
//!
//! - Project versions 34 to 38: a JSON document in UTF-16LE. Its `mTextParam` object names the
//!   text, the font (by PostScript name), the size, alignment, fill, stroke and shadow; the style
//!   values are lists of `[first character, value]` runs.
//! - Project versions 40 to 43: four marker bytes (`44 33 22 11`) and a table-and-offset binary
//!   (tables that find their fields through a field-offset list in front of them, strings and
//!   lists with a length in front; a field left at its default is not written). The same
//!   things are in it, in numbered fields ([`binary`]): a list of runs, each its words and a
//!   style, and a list of the font names the styles pick from. It is turned into the JSON shape
//!   of the older form, so both are mapped by the same code. Which field is which was worked
//!   out from the values in several hundred titles; the fields whose meaning stayed unclear are
//!   not read.
//!
//! The parameters after Source Text are the layer's own position (normalised to the sequence
//! frame), scale, rotation and opacity. A Source Text in neither form is reported and its layer
//! left out, as before.
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

/// The text document inside a Source Text value, in the shape of the JSON form, or `None` for
/// anything that is neither form (damaged data, text that is too long to be a title).
fn source_text(base64: &str) -> Option<Value> {
    if base64.len() > MAX_SOURCE_TEXT_BASE64 {
        return None;
    }
    let bytes = decode_base64(base64)?;
    let body = bytes.get(8..)?;
    let declared = u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?) as usize;
    // the binary form counts what follows its marker, the JSON form everything after the header
    if let Some(tables) = body.strip_prefix(&binary::MARKER) {
        return if declared == tables.len() { binary::document(tables) } else { None };
    }
    if declared != body.len() || body.len() % 2 != 0 {
        return None;
    }
    let units: Vec<u16> = body.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    let text = String::from_utf16(&units).ok()?;
    serde_json::from_str(&text).ok().filter(Value::is_object)
}

/// The binary form of Source Text (project versions 40 to 43).
mod binary {
    use serde_json::{Value, json};

    pub(super) const MARKER: [u8; 4] = [0x44, 0x33, 0x22, 0x11];

    /// Runs, fonts and the bytes of one string are capped: a title has a handful.
    const MAX_ITEMS: usize = 4096;
    const MAX_STRING: usize = 1 << 20;

    /// Bounds-checked reads of the table-and-offset data: every position comes from the file.
    struct Tables<'a>(&'a [u8]);

    impl<'a> Tables<'a> {
        fn bytes<const N: usize>(&self, at: usize) -> Option<[u8; N]> {
            self.0.get(at..at.checked_add(N)?)?.try_into().ok()
        }
        fn u8(&self, at: usize) -> Option<u8> {
            self.0.get(at).copied()
        }
        fn u16(&self, at: usize) -> Option<usize> {
            self.bytes(at).map(|b| usize::from(u16::from_le_bytes(b)))
        }
        fn u32(&self, at: usize) -> Option<u32> {
            self.bytes(at).map(u32::from_le_bytes)
        }
        fn f32(&self, at: usize) -> Option<f64> {
            self.bytes(at).map(|b| f64::from(f32::from_le_bytes(b))).filter(|v| v.is_finite())
        }
        /// What the offset stored at `at` points to (a table, a string or a list).
        fn target(&self, at: usize) -> Option<usize> {
            at.checked_add(usize::try_from(self.u32(at)?).ok()?).filter(|to| *to < self.0.len())
        }
        /// Where field `index` of the table at `table` is stored, when the file wrote it.
        fn field(&self, table: usize, index: usize) -> Option<usize> {
            let back = i64::from(i32::from_le_bytes(self.bytes(table)?));
            let list = usize::try_from(i64::try_from(table).ok()?.checked_sub(back)?).ok()?;
            let slot = index.checked_mul(2)?.checked_add(4)?;
            if slot.checked_add(2)? > self.u16(list)? {
                return None;
            }
            let offset = self.u16(list.checked_add(slot)?)?;
            (offset != 0).then(|| table.checked_add(offset)).flatten()
        }
        fn table(&self, table: usize, index: usize) -> Option<usize> {
            self.target(self.field(table, index)?)
        }
        fn string(&self, at: usize) -> Option<&'a str> {
            let start = self.target(at)?;
            let length = usize::try_from(self.u32(start)?).ok().filter(|n| *n <= MAX_STRING)?;
            let first = start.checked_add(4)?;
            std::str::from_utf8(self.0.get(first..first.checked_add(length)?)?).ok()
        }
        /// The positions of the entries of the list that field `index` of `table` points to.
        fn list(&self, table: usize, index: usize) -> Option<impl Iterator<Item = usize>> {
            let start = self.table(table, index)?;
            let count = usize::try_from(self.u32(start)?).ok().filter(|n| *n <= MAX_ITEMS)?;
            let first = start.checked_add(4)?;
            Some((0..count).filter_map(move |i| first.checked_add(i.checked_mul(4)?)))
        }
        /// A colour: a table of red, green and blue bytes, each 255 when it is not written.
        fn colour(&self, table: usize, index: usize) -> Option<u32> {
            let colour = self.table(table, index)?;
            let channel = |i: usize| u32::from(self.field(colour, i).and_then(|at| self.u8(at)).unwrap_or(255));
            Some(channel(0) << 16 | channel(1) << 8 | channel(2))
        }
        fn flag(&self, table: usize, index: usize) -> Option<bool> {
            self.field(table, index).and_then(|at| self.u8(at)).map(|v| v != 0)
        }
        fn number(&self, table: usize, index: usize) -> Option<f64> {
            self.field(table, index).and_then(|at| self.f32(at))
        }
    }

    /// The text of `data` (what follows the marker) as the JSON form would hold it.
    ///
    /// Fields read, by number. The text table (field 0 of the root): 0 the runs, 1 the font
    /// names, 4 the alignment, 10 the shadow colour, 11 shadow shown, 12 its opacity, 14 its
    /// distance, 16 its angle. A run: 0 its words, 1 its style. A style: 0 the number of its
    /// font in the list, 1 the size, 2 the fill colour, 3 fill shown, 4 the stroke colour,
    /// 5 stroke shown, 6 the stroke width.
    pub(super) fn document(data: &[u8]) -> Option<Value> {
        let t = Tables(data);
        let root = usize::try_from(t.u32(0)?).ok()?;
        let text = t.table(root, 0)?;
        let fonts: Vec<&str> = t.list(text, 1).map(|entries| entries.filter_map(|at| t.string(at)).collect()).unwrap_or_default();
        let mut words = String::new();
        let mut runs: [Vec<Value>; 7] = Default::default();
        const KEYS: [&str; 7] = ["mFontName", "mFontSize", "mFillColor", "mFillVisible", "mStrokeColor", "mStrokeVisible", "mStrokeWidth"];
        for run in t.list(text, 0)?.filter_map(|at| t.target(at)) {
            let first = words.chars().count();
            words.push_str(t.field(run, 0).and_then(|at| t.string(at)).unwrap_or(""));
            // many runs may name the same long string: the text as a whole is capped too
            if words.len() > MAX_STRING {
                return None;
            }
            let Some(style) = t.table(run, 1) else { continue };
            let font = t.field(style, 0).and_then(|at| t.u32(at)).unwrap_or(0);
            let values = [
                fonts.get(usize::try_from(font).ok()?).map(|name| json!(name)),
                t.number(style, 1).map(|v| json!(v)),
                t.colour(style, 2).map(|v| json!(v)),
                t.flag(style, 3).map(|v| json!(v)),
                t.colour(style, 4).map(|v| json!(v)),
                t.flag(style, 5).map(|v| json!(v)),
                t.number(style, 6).map(|v| json!(v)),
            ];
            for (list, value) in runs.iter_mut().zip(values) {
                if let Some(value) = value {
                    list.push(json!([first, value]));
                }
            }
        }
        let mut sheet = serde_json::Map::new();
        sheet.insert("mText".into(), json!(words));
        for (key, list) in KEYS.iter().zip(runs) {
            if !list.is_empty() {
                sheet.insert((*key).into(), json!({ "mParamValues": list }));
            }
        }
        let mut param = serde_json::Map::new();
        param.insert("mStyleSheet".into(), Value::Object(sheet));
        let mut put = |key: &str, value: Option<Value>| {
            if let Some(value) = value {
                param.insert(key.into(), value);
            }
        };
        put("mAlignment", t.field(text, 4).and_then(|at| t.u32(at)).map(|v| json!(v)));
        put("mShadowColor", t.colour(text, 10).map(|v| json!(v)));
        put("mShadowVisible", t.flag(text, 11).map(|v| json!(v)));
        put("mShadowOpacity", t.number(text, 12).map(|v| json!(v)));
        put("mShadowOffset", t.number(text, 14).map(|v| json!(v)));
        put("mShadowAngle", t.number(text, 16).map(|v| json!(v)));
        Some(json!({ "mTextParam": param }))
    }
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
