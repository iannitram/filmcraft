//! Titles made in the Title Designer (project versions before 34).
//!
//! Observed in user-authored project files: such a title is a `Media` object with a number where
//! the path belongs, like every source made inside the application, and an `ImporterPrefs` value:
//! base64 of a 16-byte header, the tag `CompressedTitle` with a zero byte, and a zlib stream of an
//! XML document in UTF-16LE. Like the binary parameter values, a value is written in full once
//! and named by its `BinaryHash` after that.
//!
//! What the document holds, as far as it is read here:
//!
//! - `LayoutDimension`: the frame the title was laid out in (`pXPIXELS`, `pYLINES`) and how wide
//!   the letters are drawn in that frame's pixels (`pSCREENAR`: 1 for square pixels, 0.825 for
//!   the widescreen pixels of a 720x480 frame, which are 1.2121 wide).
//! - `TextDescription` (by `Reference`): a font (family and style names) and its size.
//! - `Style` (by `ID`): a list of `Fragment`s, each one thing that is painted (the face of the
//!   letters, a stroke, the shadow), and a list that gives each painter number a shader.
//!   A fragment names its painter in `painterMix`; the shadow's painter is `-1`.
//! - `Shader` (by `cReference`): five colours. The first is the colour of a solid fill, with its
//!   transparency (`xpar`, 0 opaque to 255 clear).
//! - `TextChain`: one block of text. Its `TextLine`s are its lines, each with its words
//!   (`TRString`), its baseline (`txBase`), its alignment and a run-length list that gives every
//!   stretch of characters a style and a text description (each line counts one more character
//!   than it has: its end).
//!
//! Each line of a text chain becomes one FilmCraft text layer: point text placed by the line's
//! alignment point in the chain's box, on the line's own baseline. A line advances by the font
//! size in a title and by the font's own line height in a text layer here, so one layer for a
//! whole block would put its lines somewhere else; a layer for each line puts every line where
//! it was. Which field is which was worked out from the values in a few hundred titles. Not
//! read: shapes and pictures, gradients (the first colour is used), the stroke kinds other than
//! an edge, and tracking.

use std::collections::HashMap;
use std::io::Read;

use filmcraft_geom::Vec2;
use filmcraft_project::graphic::TEXT_LAYER;
use filmcraft_project::{CharStyle, EffectInstance, LayerExtra, Param, ParamValue, StyleRun, find_effect};
use roxmltree::{Document, Node, ParsingOptions};

use super::graph::at;
use super::text::decode_base64;
use crate::Report;

/// The element of a `Media` object that holds a title.
pub(super) const PREFS: &str = "ImporterPrefs";

const TAG: &[u8] = b"CompressedTitle\0";
/// A title is tens of kilobytes; anything far larger is not one and is not decoded.
const MAX_PREFS_BASE64: usize = 8 << 20;
const MAX_TITLE_BYTES: usize = 16 << 20;
const MAX_TITLE_NODES: u32 = 1_000_000;
const MAX_LAYERS: usize = 256;
const MAX_CHARACTERS: usize = 100_000;
const MAX_FRAME: f64 = 100_000.0;

/// The fragment numbers (`annotation`) with a fixed meaning; strokes count up from 1.
const FACE: &str = "65538";
const SHADOW: &str = "65537";
/// The kind of fragment (`eFragmentType`) that is an edge around the letters.
const EDGE: &str = "2";
/// The colour option of a shader that is one solid colour.
const SOLID: &str = "4";

/// The text layers of one title, one for each line, placed in the frame the title was laid out in.
pub(super) struct Title {
    frame: (f64, f64),
    /// How wide a letter is drawn in the title's own pixels: one over their pixel aspect.
    screen: f64,
    layers: Vec<EffectInstance>,
}

impl Title {
    pub(super) fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// The layers placed in a sequence frame of `size` whose pixels have the aspect `par`: a
    /// title of another size sits in the middle of the frame, as a clip does. In pixels that
    /// are not square the letters are drawn narrower or wider by that much, so they have their
    /// shape when the frame is shown; a title made for other pixels than the sequence's is as
    /// much wider or narrower about the middle as its clip was.
    pub(super) fn layers_in(&self, size: (u32, u32), par: (u32, u32)) -> Vec<EffectInstance> {
        let pixel = if par.0 > 0 && par.1 > 0 { f64::from(par.0) / f64::from(par.1) } else { 1.0 };
        let stretch = 1.0 / (self.screen * pixel);
        let letters = 100.0 / pixel;
        let mut layers = self.layers.clone();
        for layer in &mut layers {
            if let Some(param) = layer.params.get_mut("position")
                && let ParamValue::Vec2(place) = &param.value
            {
                let x = f64::from(size.0) / 2.0 + (place.x - self.frame.0 / 2.0) * stretch;
                let y = place.y + (f64::from(size.1) - self.frame.1) / 2.0;
                *param = Param::new(ParamValue::Vec2(Vec2::new(x, y)));
            }
            if (letters - 100.0).abs() > 0.05 {
                for (id, value) in [("uniform_scale", ParamValue::Bool(false)), ("scale_width", ParamValue::Float(letters))] {
                    if let Some(param) = layer.params.get_mut(id) {
                        *param = Param::new(value);
                    }
                }
            }
        }
        layers
    }
}

/// The title inside an `ImporterPrefs` value, or `None` for a value that is not a title or
/// cannot be read (the caller then keeps the clip as a placeholder).
pub(super) fn read(prefs_base64: &str, report: &mut Report) -> Option<Title> {
    if prefs_base64.len() > MAX_PREFS_BASE64 {
        return None;
    }
    let bytes = decode_base64(prefs_base64)?;
    let head = bytes.get(..bytes.len().min(64))?;
    let tag = head.windows(TAG.len()).position(|window| window == TAG)?;
    let stream = bytes.get(tag.checked_add(TAG.len())?..)?;
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(stream).take(MAX_TITLE_BYTES as u64 + 1).read_to_end(&mut raw).ok()?;
    if raw.len() > MAX_TITLE_BYTES {
        return None;
    }
    let units: Vec<u16> = raw.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    let text = String::from_utf16(&units).ok()?;
    // past the byte-order mark and the declaration, which names an encoding the text no longer has
    let body = text.get(text.find("<Adobe_Root")?..)?;
    let document = Document::parse_with_options(body, ParsingOptions { allow_dtd: false, nodes_limit: MAX_TITLE_NODES, ..Default::default() }).ok()?;
    let root = document.root_element();

    let layout = root.descendants().find(|n| n.has_tag_name("LayoutDimension"))?;
    let side = |name: &str| number(layout, &[name]).filter(|v| (1.0..=MAX_FRAME).contains(v));
    let frame = (side("pXPIXELS")?, side("pYLINES")?);
    let screen = number(layout, &["pSCREENAR"]).filter(|v| (0.1..=10.0).contains(v)).unwrap_or(1.0);
    let mut tables = Tables::default();
    for node in root.descendants() {
        match node.tag_name().name() {
            "Shader" => {
                if let Some(id) = words(node, &["cReference"]) {
                    tables.shaders.entry(id).or_insert(node);
                }
            }
            "Style" => {
                if let Some(id) = node.attribute("ID") {
                    tables.styles.entry(id).or_insert(node);
                }
            }
            "TextDescription" => {
                if let Some(id) = node.attribute("Reference") {
                    tables.types.entry(id).or_insert(node);
                }
            }
            _ => {}
        }
    }
    let mut notes = Notes::default();
    let mut layers = Vec::new();
    'chains: for chain in root.descendants().filter(|n| n.has_tag_name("TextChain")) {
        for line in chain.children().filter(|n| n.has_tag_name("TextLine")) {
            if layers.len() >= MAX_LAYERS {
                break 'chains;
            }
            if let Some(layer) = line_layer(chain, line, &tables, &mut notes) {
                layers.push(layer);
            }
        }
    }
    if root.descendants().any(|n| n.has_tag_name("DrawObject")) {
        report.warn("Premiere legacy title shapes and pictures are not imported; the title's text is");
    }
    if notes.gradient {
        report.warn("Premiere legacy title text with a gradient fill was imported with the gradient's first colour");
    }
    if notes.strokes {
        report.warn("Premiere legacy title strokes other than the first two edges (depth, drop face, inner) are not imported");
    }
    if notes.spacing {
        report.info("Premiere legacy title tracking is not imported; the font's own spacing is used");
    }
    Some(Title { frame, screen, layers })
}

#[derive(Default)]
struct Tables<'a, 'i> {
    shaders: HashMap<&'a str, Node<'a, 'i>>,
    styles: HashMap<&'a str, Node<'a, 'i>>,
    types: HashMap<&'a str, Node<'a, 'i>>,
}

/// What was met and not carried over, for the report.
#[derive(Default)]
struct Notes {
    gradient: bool,
    strokes: bool,
    spacing: bool,
}

/// A font and its size, from a text description.
#[derive(Default, PartialEq)]
struct Type {
    family: Option<String>,
    face: Option<String>,
    size: Option<f64>,
    small_caps: bool,
    underline: bool,
}

/// One stretch of characters of a line and what it is drawn with.
struct Span<'a> {
    start: usize,
    end: usize,
    type_id: &'a str,
    style_id: &'a str,
}

/// The text layer for one line of a chain, or `None` for a line without words.
fn line_layer<'a, 'i>(chain: Node<'a, 'i>, line: Node<'a, 'i>, tables: &Tables<'a, 'i>, notes: &mut Notes) -> Option<EffectInstance> {
    let text = at(line, &["TRString"]).and_then(|n| n.text()).unwrap_or("");
    let count = text.chars().count();
    if text.trim().is_empty() || count > MAX_CHARACTERS {
        return None;
    }
    // the runs of a line count its characters and one more, its end
    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut from = 0usize;
    for run in at(line, &["RunLengthEncodedCharacterAttributes"]).into_iter().flat_map(|n| n.children()).filter(|n| n.has_tag_name("CharacterAttributes")) {
        let length = run.attribute("RunCount").and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(0).min(MAX_CHARACTERS);
        let to = from.checked_add(length)?.min(count);
        if let (Some(type_id), Some(style_id)) = (run.attribute("TextRef"), run.attribute("StyleRef"))
            && to > from
        {
            spans.push(Span { start: from, end: to, type_id, style_id });
        }
        from = to;
    }

    let mut layer = find_effect(TEXT_LAYER)?.instance();
    let mut set = |id: &str, value: ParamValue| {
        if let Some(p) = layer.params.get_mut(id) {
            *p = Param::new(value);
        }
    };
    set("text", ParamValue::Text(text.to_string()));

    // the line's alignment point in the chain's box, on the line's own baseline
    let property = at(chain, &["ChainProperty"]);
    let of = |path: &[&str]| property.and_then(|p| number(p, path));
    let align = match words(line, &["Alignment"]) {
        Some("center") => 1u32,
        Some("right") => 2,
        _ => 0,
    };
    let left = of(&["Position", "x"]).or_else(|| number(line, &["BaseProperties", "XPos"]));
    let width = of(&["Size", "x"]).unwrap_or(0.0).max(0.0);
    let baseline = number(line, &["BaseProperties", "txBase"]);
    if let (Some(left), Some(baseline)) = (left, baseline) {
        let x = match align {
            1 => left + width / 2.0,
            2 => left + width,
            _ => left,
        };
        if x.abs() <= MAX_FRAME && baseline.abs() <= MAX_FRAME {
            set("position", ParamValue::Vec2(Vec2::new(x, baseline)));
        }
    }
    set("align", ParamValue::Choice(align));

    // the look of the first stretch is the layer's own; the stretches that differ are style runs
    let base_type = spans.first().map(|s| type_of(tables, s.type_id, notes)).unwrap_or_default();
    let base_style = spans.first().and_then(|s| tables.styles.get(s.style_id).copied());
    if let Some(family) = &base_type.family {
        set("font", ParamValue::Text(family.clone()));
    }
    if let Some(face) = &base_type.face {
        set("font_style", ParamValue::Text(face.clone()));
    }
    if let Some(size) = base_type.size {
        set("size", ParamValue::Float(size));
    }
    set("caps", ParamValue::Choice(if base_type.small_caps { 2 } else { 0 }));
    set("underline", ParamValue::Bool(base_type.underline));

    let base_fill = base_style.and_then(|style| face_colour(tables, style, notes));
    set("fill", ParamValue::Bool(base_fill.is_some()));
    if let Some(colour) = base_fill {
        set("fill_color", ParamValue::Color(colour));
    }
    let mut edges = base_style.map(|style| strokes(tables, style, notes)).unwrap_or_default().into_iter();
    for (on, colour, width) in [("stroke", "stroke_color", "stroke_width"), ("stroke2", "stroke2_color", "stroke2_width")] {
        match edges.next() {
            Some((c, w)) => {
                set(on, ParamValue::Bool(true));
                set(colour, ParamValue::Color(c));
                set(width, ParamValue::Float(w));
            }
            None => set(on, ParamValue::Bool(false)),
        }
    }
    if edges.next().is_some() {
        notes.strokes = true;
    }
    set("shadow", ParamValue::Bool(false));
    if let Some(style) = base_style
        && let Some(fragment) = fragments(style).find(|f| words(*f, &["annotation"]) == Some(SHADOW))
    {
        set("shadow", ParamValue::Bool(words(fragment, &["fragmentOff"]) != Some("true")));
        if let Some((colour, opacity)) = shader_of(tables, style, "-1").and_then(first_colour) {
            set("shadow_color", ParamValue::Color([colour[0], colour[1], colour[2], 1.0]));
            set("shadow_opacity", ParamValue::Float(f64::from(opacity) * 100.0));
        }
        if let Some(v) = number(fragment, &["offset"]) {
            set("shadow_distance", ParamValue::Float(v.clamp(0.0, 1000.0)));
        }
        // stored half a turn from what the Title Designer shows: its default, 135°, is -45 here
        if let Some(v) = number(fragment, &["angle"]) {
            set("shadow_angle", ParamValue::Float((v + 180.0).rem_euclid(360.0)));
        }
        if let Some(v) = number(fragment, &["size"]) {
            set("shadow_size", ParamValue::Float(v.clamp(0.0, 1000.0)));
        }
        if let Some(v) = number(style, &["StyleBase", "shadow_softness"]) {
            set("shadow_blur", ParamValue::Float(v.clamp(0.0, 1000.0)));
        }
    }

    let mut runs = Vec::new();
    let base_ids = spans.first().map(|s| (s.type_id, s.style_id));
    for span in spans.iter().skip(1) {
        if Some((span.type_id, span.style_id)) == base_ids {
            continue;
        }
        let kind = type_of(tables, span.type_id, notes);
        let fill = tables.styles.get(span.style_id).and_then(|style| face_colour(tables, *style, notes));
        let style = CharStyle {
            font: kind.family.clone().filter(|_| kind.family != base_type.family),
            font_style: kind.face.clone().filter(|_| kind.face != base_type.face),
            size: kind.size.filter(|_| kind.size != base_type.size).map(|v| v as f32),
            fill: fill.filter(|_| fill != base_fill),
            underline: (kind.underline != base_type.underline).then_some(kind.underline),
            caps: (kind.small_caps != base_type.small_caps).then_some(if kind.small_caps { 2 } else { 0 }),
            ..Default::default()
        };
        if !style.is_empty() {
            runs.push(StyleRun { start: span.start, end: span.end.min(count), style });
        }
    }
    if !runs.is_empty() {
        layer.layer = Some(Box::new(LayerExtra { runs, ..Default::default() }));
    }
    Some(layer)
}

fn type_of(tables: &Tables<'_, '_>, id: &str, notes: &mut Notes) -> Type {
    let Some(spec) = tables.types.get(id).and_then(|node| at(*node, &["TypeSpec"])) else { return Type::default() };
    if number(spec, &["txKern"]).is_some_and(|v| v != 0.0) {
        notes.spacing = true;
    }
    let name = |tag: &str| words(spec, &[tag]).filter(|v| !v.is_empty() && v.len() <= 200).map(str::to_string);
    Type {
        family: name("fifontFamilyName"),
        face: name("fifontStyle"),
        size: number(spec, &["txHeight"]).filter(|v| *v > 0.0).map(|v| v.clamp(1.0, 2000.0)),
        small_caps: words(spec, &["txSCapsOn"]) == Some("true"),
        underline: words(spec, &["txUnderline"]) == Some("true"),
    }
}

fn fragments<'a, 'i>(style: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    at(style, &["FragmentList"]).into_iter().flat_map(|list| list.children()).filter(|n| n.has_tag_name("Fragment"))
}

/// The shader a style gives painter `painter`.
fn shader_of<'a, 'i>(tables: &Tables<'a, 'i>, style: Node<'a, 'i>, painter: &str) -> Option<Node<'a, 'i>> {
    let list = at(style, &["ShaderList"])?;
    let entry = list.children().find(|n| n.has_tag_name("ShaderRef") && n.attribute("PainterNumber") == Some(painter))?;
    tables.shaders.get(words(entry, &["shaderRef"])?).copied()
}

/// The shader a fragment is painted with: the first number of its painter mix.
fn fragment_shader<'a, 'i>(tables: &Tables<'a, 'i>, style: Node<'a, 'i>, fragment: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
    let painter = words(fragment, &["painterMix"])?.split_ascii_whitespace().next()?;
    shader_of(tables, style, painter)
}

/// The first colour of a shader and how opaque it is (0 to 1).
fn first_colour(shader: Node<'_, '_>) -> Option<([f32; 4], f32)> {
    let spec = shader.children().find(|n| n.has_tag_name("ColorSpec") && n.attribute("index") == Some("0"))?;
    let channel = |tag: &str| number(spec, &[tag]).map(|v| (v.clamp(0.0, 255.0) / 255.0) as f32);
    let opacity = 1.0 - channel("xpar").unwrap_or(0.0);
    Some(([channel("red")?, channel("green")?, channel("blue")?, opacity], opacity))
}

/// The fill of the letters of a style, or `None` when the face is switched off.
fn face_colour<'a, 'i>(tables: &Tables<'a, 'i>, style: Node<'a, 'i>, notes: &mut Notes) -> Option<[f32; 4]> {
    let face = fragments(style).find(|f| words(*f, &["annotation"]) == Some(FACE))?;
    if words(face, &["fragmentOff"]) == Some("true") {
        return None;
    }
    let shader = fragment_shader(tables, style, face)?;
    if words(shader, &["colorOption"]) != Some(SOLID) {
        let colours: Vec<_> =
            shader.children().filter(|n| n.has_tag_name("ColorSpec")).take(4).map(|n| ["red", "green", "blue"].map(|tag| words(n, &[tag]))).collect();
        notes.gradient |= colours.windows(2).any(|pair| pair[0] != pair[1]);
    }
    first_colour(shader).map(|(colour, _)| colour)
}

/// The edges around the letters of a style, innermost first: colour and width.
fn strokes<'a, 'i>(tables: &Tables<'a, 'i>, style: Node<'a, 'i>, notes: &mut Notes) -> Vec<([f32; 4], f64)> {
    let mut found: Vec<(u32, [f32; 4], f64)> = Vec::new();
    for fragment in fragments(style) {
        let Some(number_of) = words(fragment, &["annotation"]) else { continue };
        if number_of == FACE || number_of == SHADOW || words(fragment, &["fragmentOff"]) == Some("true") {
            continue;
        }
        let order = number_of.parse::<u32>().ok().filter(|n| *n < 65_536);
        let width = number(fragment, &["size"]).filter(|w| *w > 0.0);
        let colour = fragment_shader(tables, style, fragment).and_then(first_colour);
        match (order, width, colour) {
            (Some(order), Some(width), Some((colour, _))) if words(fragment, &["eFragmentType"]) == Some(EDGE) => {
                found.push((order, colour, width.clamp(0.0, 1000.0)));
            }
            _ => notes.strokes = true,
        }
    }
    found.sort_by_key(|(order, _, _)| *order);
    found.into_iter().map(|(_, colour, width)| (colour, width)).collect()
}

/// The text of the element at `path`, without the white space around it.
fn words<'a>(node: Node<'a, '_>, path: &[&str]) -> Option<&'a str> {
    at(node, path).and_then(|n| n.text()).map(str::trim)
}

fn number(node: Node<'_, '_>, path: &[&str]) -> Option<f64> {
    words(node, path)?.parse::<f64>().ok().filter(|v| v.is_finite())
}
