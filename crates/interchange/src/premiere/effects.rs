use filmcraft_geom::Vec2;
use filmcraft_project::{EffectInstance, Interpolation, Keyframe, Param, ParamValue, find_effect};
use filmcraft_time::Tick;
use roxmltree::Node;

use super::graph::{Graph, error, static_text, text_at};
use crate::xml::child_text;
use crate::{Report, Result};

#[derive(Clone, Copy)]
pub(super) enum Conversion {
    Native,
    Point((u32, u32)),
    Gain,
    /// A number on another scale: multiplied by this factor.
    Scaled(f64),
}

/// Opaque black as a native colour value: the default of every colour control seen so far. How
/// the other colours are packed into that number is not known, so they are not read.
const OPAQUE_BLACK: &str = "18374686479671623680";

const VECTOR_MOTION: &str = "AE.ADBE Graphic Group";

/// What to do with a parameter whose value cannot be read.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BadValue {
    /// Fail the import: a preset that would silently lose a value is not added.
    Reject,
    /// Keep the FilmCraft default and say so in the report: one odd value in one effect must
    /// not keep a whole project, with years of edits in it, from opening.
    ReportAndDefault,
}

/// Coordinates are normalised in native files. Motion's position is in sequence pixels;
/// anchors and standard effects are in source pixels.
pub(super) fn component(
    graph: &Graph<'_, '_>,
    node: Node<'_, '_>,
    origin: Tick,
    sequence_size: (u32, u32),
    source_size: (u32, u32),
    bad_value: BadValue,
    report: &mut Report,
) -> Result<Option<EffectInstance>> {
    let audio = node.has_tag_name("AudioFilterComponent");
    let match_name = child_text(node, if audio { "FilterMatchName" } else { "MatchName" }).unwrap_or("");
    let id = match match_name {
        "AE.ADBE Motion" => "motion",
        "AE.ADBE Geometry2" => "transform",
        // Vector Motion moves a whole graphic before the clip's own Motion does, which is what
        // the Transform effect does here. It has Motion's controls in Motion's order.
        VECTOR_MOTION => "transform",
        "AE.ADBE Opacity" => "opacity",
        "AE.ADBE AECrop" => "crop",
        "AE.ADBE Offset" => "offset",
        "AE.ADBE Motion Blur" => "directional_blur",
        "AE.ADBE Gaussian Blur 2" => "gaussian_blur",
        "AE.ADBE Brightness & Contrast 2" => "brightness_contrast",
        "AE.ADBE Drop Shadow" => "drop_shadow",
        "AE.ADBE Basic 3D" => "basic_3d",
        // effects without controls: the match name is all there is to carry over (each has an
        // older "PR." and a newer "AE." name)
        "AE.ADBE Horizontal Flip" | "PR.ADBE Horizontal Flip" => "horizontal_flip",
        "AE.ADBE Vertical Flip" | "PR.ADBE Vertical Flip" => "vertical_flip",
        "AE.ADBE Black & White" | "PR.ADBE Black & White" => "black_white",
        "Internal Volume Mono" | "Internal Volume Stereo" => "volume",
        _ => {
            let name = text_at(node, &["Component", "DisplayName"]).or_else(|| child_text(node, "CustomDisplayName")).unwrap_or(match_name);
            report.warn(format!("Premiere effect \"{name}\" ({match_name}) is not supported and was skipped"));
            return Ok(None);
        }
    };
    let Some(def) = find_effect(id) else { return Ok(None) };
    let mut effect = def.instance();
    let component_path: &[&str] = if audio { &["AudioComponent", "Component"] } else { &["Component"] };
    effect.enabled = !graph.boolean(node, &component_path.iter().copied().chain(["Bypass"]).collect::<Vec<_>>(), false)?;
    let params_path: Vec<&str> = component_path.iter().copied().chain(["Params"]).collect();
    let parameters = graph.references(node, &params_path)?;
    let mut composition_shutter = false;
    let mut volume_muted = false;
    for (index, native) in parameters.into_iter().enumerate() {
        let number = graph.integer(native, &["ParameterID"], (index + 1) as i64)?;
        // Observed project files give Gaussian Blur's Repeat Edge Pixels checkbox the same
        // ParameterID as Blurriness (1, 2, 1): tell the two apart by what they hold.
        let number = if id == "gaussian_blur" && number == 1 && matches!(static_text(native), Some("true" | "false")) { 3 } else { number };
        // Vector Motion's controls, renumbered to Transform's: position, scale, scale width,
        // uniform scale, rotation, anchor point
        let number = if match_name == VECTOR_MOTION {
            match number {
                1 => 2,
                2 => 3,
                3 => 4,
                4 => 11,
                5 => 7,
                6 => 1,
                _ => 0,
            }
        } else {
            number
        };
        let mapped = match (id, number) {
            ("motion", 1) => Some(("position", Conversion::Point(sequence_size))),
            ("motion", 2) => Some(("scale", Conversion::Native)),
            ("motion", 3) => Some(("scale_width", Conversion::Native)),
            ("motion", 4) => Some(("uniform_scale", Conversion::Native)),
            ("motion", 5) => Some(("rotation", Conversion::Native)),
            ("motion", 6) => Some(("anchor", Conversion::Point(source_size))),
            ("motion", 7) => Some(("anti_flicker", Conversion::Native)),
            ("transform", 1) => Some(("anchor", Conversion::Point(source_size))),
            ("transform", 2) => Some(("position", Conversion::Point(source_size))),
            ("transform", 3) => Some(("scale_height", Conversion::Native)),
            ("transform", 4) => Some(("scale_width", Conversion::Native)),
            ("transform", 5) => Some(("skew", Conversion::Native)),
            ("transform", 6) => Some(("skew_axis", Conversion::Native)),
            ("transform", 7) => Some(("rotation", Conversion::Native)),
            ("transform", 8) | ("opacity", 1) => Some(("opacity", Conversion::Native)),
            ("transform", 10) => Some(("shutter_angle", Conversion::Native)),
            ("transform", 11) => Some(("uniform_scale", Conversion::Native)),
            ("crop", 1) => Some(("left", Conversion::Native)),
            ("crop", 2) => Some(("top", Conversion::Native)),
            ("crop", 3) => Some(("right", Conversion::Native)),
            ("crop", 4) => Some(("bottom", Conversion::Native)),
            ("crop", 5) => Some(("zoom", Conversion::Native)),
            ("crop", 6) => Some(("feather", Conversion::Native)),
            ("offset", 1) => Some(("shift", Conversion::Point(source_size))),
            ("offset", 2) => Some(("blend", Conversion::Native)),
            ("directional_blur", 1) => Some(("direction", Conversion::Native)),
            ("directional_blur", 2) => Some(("length", Conversion::Native)),
            ("gaussian_blur", 1) => Some(("blurriness", Conversion::Native)),
            ("gaussian_blur", 3) => Some(("repeat_edge", Conversion::Native)),
            ("brightness_contrast", 1) => Some(("brightness", Conversion::Native)),
            ("brightness_contrast", 2) => Some(("contrast", Conversion::Native)),
            // Drop Shadow's opacity runs 0 to 255 in the file, 0 to 100 % here
            ("drop_shadow", 2) => Some(("opacity", Conversion::Scaled(100.0 / 255.0))),
            ("drop_shadow", 3) => Some(("direction", Conversion::Native)),
            ("drop_shadow", 4) => Some(("distance", Conversion::Native)),
            ("drop_shadow", 5) => Some(("softness", Conversion::Native)),
            ("drop_shadow", 6) => Some(("only", Conversion::Native)),
            ("basic_3d", 1) => Some(("swivel", Conversion::Native)),
            ("basic_3d", 2) => Some(("tilt", Conversion::Native)),
            ("basic_3d", 3) => Some(("distance", Conversion::Native)),
            ("volume", 2) => Some(("level", Conversion::Gain)),
            _ => None,
        };
        if let Some((target, conversion)) = mapped {
            let Some(default) = effect.params.get(target).map(|p| p.value.clone()) else { continue };
            match parameter(graph, native, origin, &default, conversion, match_name, report) {
                Ok(param) => {
                    effect.params.insert(target.to_string(), param);
                }
                // out of budget is never a property of one value: stop
                Err(e) if bad_value == BadValue::ReportAndDefault && !graph.exhausted() => {
                    let reason = match e {
                        crate::Error::Parse { message, .. } => message,
                        other => other.to_string(),
                    };
                    report.warn(format!("Premiere effect {match_name} parameter {number} could not be read ({reason}); the FilmCraft default was used"));
                }
                Err(e) => return Err(e),
            }
        } else if id == "transform" && number == 9 {
            composition_shutter = matches!(static_text(native), Some("true" | "1"));
        } else if id == "volume" && number == 1 {
            volume_muted = matches!(static_text(native), Some("true" | "1"));
            if graph.boolean(native, &["IsTimeVarying"], false)? {
                report.warn("Premiere animated Volume mute is not supported; its static state was used");
            }
        } else {
            // These are known neutral legacy controls, not unknown active effects.
            let neutral = !graph.boolean(native, &["IsTimeVarying"], false)?
                && ((id == "motion" && (8..=11).contains(&number) && static_text(native).is_none_or(|v| v.parse::<f64>().ok() == Some(0.0)))
                    || (id == "transform" && number == 12 && static_text(native) == Some("0"))
                    || (id == "opacity" && ((number == 2 && static_text(native) == Some("18")) || (number == 3 && static_text(native) == Some("0"))))
                    // the shadow colour left at black, which is the default here too
                    || (id == "drop_shadow" && number == 1 && static_text(native) == Some(OPAQUE_BLACK))
                    // Specular Highlight and Draw Preview Wireframe, switched off
                    || (id == "basic_3d" && (4..=5).contains(&number) && static_text(native) == Some("false"))
                    // the older "PR." effects without controls carry one empty placeholder
                    || (matches!(id, "horizontal_flip" | "vertical_flip" | "black_white")
                        && number == -1
                        && native.has_tag_name("ArbVideoComponentParam")
                        && static_text(native).is_none()));
            if !neutral {
                report.warn(format!("Premiere effect {match_name} parameter {number} is not supported; the FilmCraft default was used"));
            }
        }
    }
    if composition_shutter {
        effect.params.insert("shutter_angle".into(), Param::new(ParamValue::Float(180.0)));
        report.warn("Premiere Transform uses the composition shutter angle; imported as 180 degrees");
    }
    if volume_muted {
        effect.params.insert("level".into(), Param::new(ParamValue::Float(-287.5)));
    }
    if node.descendants().any(|n| n.is_element() && matches!(n.tag_name().name(), "Mask" | "Masks" | "MaskComponents")) {
        report.warn(format!("Premiere effect {match_name} masks are not supported and were skipped"));
    }
    Ok(Some(effect))
}

fn finite(text: &str, graph: &Graph<'_, '_>) -> Result<f64> {
    let v = text.trim().parse::<f64>().map_err(|e| error(graph.format, format!("invalid effect value: {e}")))?;
    if !v.is_finite() || v.abs() > 1e12 {
        return Err(error(graph.format, "effect value is not finite or is out of range"));
    }
    Ok(v)
}

fn value(text: &str, default: &ParamValue, conversion: Conversion, graph: &Graph<'_, '_>) -> Result<ParamValue> {
    match conversion {
        Conversion::Point((w, h)) => {
            let (x, y) = text.split_once(':').ok_or_else(|| error(graph.format, "point parameter must contain x:y"))?;
            let x = finite(x, graph)? * f64::from(w);
            let y = finite(y, graph)? * f64::from(h);
            if x.abs() > 1e12 || y.abs() > 1e12 {
                return Err(error(graph.format, "point parameter is out of range"));
            }
            Ok(ParamValue::Vec2(Vec2::new(x, y)))
        }
        Conversion::Gain => {
            let gain = finite(text, graph)?;
            if gain < 0.0 {
                return Err(error(graph.format, "negative audio gain"));
            }
            Ok(ParamValue::Float(if gain == 0.0 { -287.5 } else { (20.0 * gain.log10()).max(-287.5) }))
        }
        Conversion::Scaled(factor) => Ok(ParamValue::Float(finite(text, graph)? * factor)),
        Conversion::Native => match default {
            ParamValue::Bool(_) => match text {
                "true" | "1" => Ok(ParamValue::Bool(true)),
                "false" | "0" => Ok(ParamValue::Bool(false)),
                _ => Err(error(graph.format, "invalid effect boolean")),
            },
            ParamValue::Choice(_) => text.parse::<u32>().map(ParamValue::Choice).map_err(|e| error(graph.format, e.to_string())),
            _ => Ok(ParamValue::Float(finite(text, graph)?)),
        },
    }
}

pub(super) fn parameter(
    graph: &Graph<'_, '_>,
    node: Node<'_, '_>,
    origin: Tick,
    default: &ParamValue,
    conversion: Conversion,
    match_name: &str,
    report: &mut Report,
) -> Result<Param> {
    let is_point = matches!(conversion, Conversion::Point(_));
    let fixed = if is_point { static_point(node) } else { static_text(node).map(str::to_string) };
    let mut p = Param::new(fixed.map(|v| value(&v, default, conversion, graph)).transpose()?.unwrap_or_else(|| default.clone()));
    if !graph.boolean(node, &["IsTimeVarying"], false)? {
        return Ok(p);
    }
    let hold = graph.boolean(node, &["DiscontinuousInterpolate"], false)?;
    let mut approximated = false;
    for record in child_text(node, "Keyframes").unwrap_or("").split(';').filter(|r| !r.trim().is_empty()) {
        graph.charge()?;
        if p.keyframes.len() >= 65_536 {
            return Err(error(graph.format, "parameter exceeds the 65536 keyframe limit"));
        }
        let fields: Vec<&str> = record.split(',').take(65).map(str::trim).collect();
        if fields.len() > 64 {
            return Err(error(graph.format, "keyframe has too many fields"));
        }
        let timestamp =
            fields.first().ok_or_else(|| error(graph.format, "keyframe time is missing"))?.parse::<i64>().map_err(|e| error(graph.format, e.to_string()))?;
        let time = timestamp.checked_sub(origin.0).ok_or_else(|| error(graph.format, "keyframe time overflows"))?;
        if time.unsigned_abs() > super::MAX_TIME_TICKS as u64 {
            return Err(error(graph.format, "keyframe time is out of range"));
        }
        let data = fields.get(1).ok_or_else(|| error(graph.format, "keyframe value is missing"))?;
        // older files write a point as two fields (`time,x,y,…`): everything after it sits one further on
        let old_point = is_point && !data.contains(':');
        let joined;
        let data: &str = if old_point {
            joined = format!("{data}:{}", fields.get(2).ok_or_else(|| error(graph.format, "point keyframe has no y value"))?);
            &joined
        } else {
            data
        };
        let at = |index: usize| fields.get(index + usize::from(old_point));
        let mut key = Keyframe::new(Tick(time), value(data, default, conversion, graph)?);
        let incoming = at(5).map(|v| finite(v, graph)).transpose()?.unwrap_or(0.0);
        let outgoing = at(7).map(|v| finite(v, graph)).transpose()?.unwrap_or(0.0);
        let in_speed = at(4).map(|v| finite(v, graph)).transpose()?.unwrap_or(0.0);
        let out_speed = at(6).map(|v| finite(v, graph)).transpose()?.unwrap_or(0.0);
        if !(0.0..=1.0).contains(&incoming) || !(0.0..=1.0).contains(&outgoing) {
            return Err(error(graph.format, "keyframe influence is outside 0..1"));
        }
        key.in_influence = incoming;
        key.out_influence = outgoing;
        key.interp = if hold || !key.value.interpolates() {
            Interpolation::Hold
        } else if incoming > 0.0 || outgoing > 0.0 {
            Interpolation::Bezier
        } else {
            Interpolation::Linear
        };
        approximated |= in_speed != 0.0
            || out_speed != 0.0
            || fields.get(10 + usize::from(old_point)..).is_some_and(|v| v.iter().any(|s| s.parse::<f64>().is_ok_and(|v| v != 0.0)));
        p.keyframes.push(key);
    }
    p.keyframes.sort_by_key(|k| k.time);
    if p.keyframes.windows(2).any(|k| k[0].time == k[1].time) {
        return Err(error(graph.format, "duplicate parameter keyframe time"));
    }
    if approximated {
        report.warn(format!("Premiere effect {match_name} temporal velocities or spatial tangents were approximated by FilmCraft keyframe interpolation"));
    }
    Ok(p)
}

/// A point parameter's fixed value as `x:y`. Current files write it as one field of the start
/// keyframe (`time,x:y,…`); project versions up to 36 or so write two (`time,x,y,…`).
fn static_point(node: Node<'_, '_>) -> Option<String> {
    let Some(record) = child_text(node, "StartKeyframe") else { return child_text(node, "CurrentValue").map(str::to_string) };
    let mut fields = record.split(',').skip(1).map(str::trim);
    let first = fields.next()?;
    if first.contains(':') {
        return Some(first.to_string());
    }
    Some(format!("{first}:{}", fields.next()?))
}

pub(super) fn put_effect(effects: &mut Vec<EffectInstance>, effect: EffectInstance) {
    if effect.def().is_some_and(|e| e.intrinsic) {
        if let Some(old) = effects.iter_mut().find(|e| e.effect == effect.effect) {
            *old = effect;
        } else {
            effects.push(effect);
        }
    } else {
        let first_intrinsic = effects.iter().position(|e| e.def().is_some_and(|d| d.intrinsic)).unwrap_or(effects.len());
        effects.insert(first_intrinsic, effect);
    }
}
