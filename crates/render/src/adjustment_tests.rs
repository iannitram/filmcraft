//! Adjustment layers (M5.6): effects apply to the composite of every track below, opacity and
//! blend mode mix the adjusted picture over the original, Motion and masks restrict the region,
//! and nesting scopes them to their own sequence.

use super::*;
use filmcraft_media::generators::GeneratorSource;
use filmcraft_media::{DemoScene, Generator, MediaSource};
use filmcraft_project::{ClipId, Label, Mask, MaskPath, MediaClip, MediaRef, SequenceSettings, TrackKind, find_effect};
use filmcraft_time::{FrameRate, TICKS_PER_SECOND, TimeRange};

const R: FrameRate = FrameRate::FPS_24;

struct Scene {
    p: Project,
    seq: ItemId,
    map: SourceMap,
}

fn media(p: &mut Project, map: &mut SourceMap, g: GeneratorSource) -> ItemId {
    let info = g.info().clone();
    let generator = g.generator.clone();
    let id = p.add_item(
        &info.name.clone(),
        Label::Iris,
        ItemKind::Media(MediaClip {
            media: MediaRef::Generator(generator),
            info,
            interpret: Default::default(),
            mark_in: None,
            mark_out: None,
            markers: vec![],
            offline: false,
            proxy: None,
            identity: None,
        }),
        None,
    );
    map.0.insert(id, Arc::new(g) as SharedSource);
    id
}

fn matte(p: &mut Project, map: &mut SourceMap, c: [f32; 4]) -> ItemId {
    media(p, map, GeneratorSource::new(Generator::ColorMatte { color: c }, 320, 180, R, Tick(10 * TICKS_PER_SECOND)))
}

fn place(p: &mut Project, seq: ItemId, track: usize, item: ItemId) -> ClipId {
    let ti = p.make_track_item(item, TrackKind::Video, Tick::ZERO, TimeRange::new(Tick::ZERO, R.tick_of(48)), R).unwrap();
    let id = ti.id;
    let mut ti = ti;
    ti.scale_to_frame = true;
    let s = p.sequence_mut(seq).unwrap();
    s.video_tracks[track].items.push(ti);
    s.video_tracks[track].sort();
    id
}

fn adjustment(p: &mut Project, seq: ItemId, track: usize) -> ClipId {
    let a = p.add_item("Adjustment Layer", Label::Lavender, ItemKind::AdjustmentLayer { width: 320, height: 180, rate: R, duration: R.tick_of(48) }, None);
    place(p, seq, track, a)
}

fn scene(tracks: usize) -> Scene {
    let mut p = Project::new("adj");
    let seq = p.new_sequence("s", SequenceSettings { width: 320, height: 180, frame_rate: R, ..Default::default() }, tracks, 0, None);
    Scene { p, seq, map: SourceMap::default() }
}

fn clip(s: &mut Scene, c: ClipId) -> &mut TrackItem {
    s.p.sequence_mut(s.seq).unwrap().find_item_mut(c).unwrap().1
}

fn fx(s: &mut Scene, c: ClipId, id: &str) {
    let e = find_effect(id).unwrap().instance();
    let it = clip(s, c);
    let pos = it.effects.iter().position(|e| e.def().is_some_and(|d| d.intrinsic)).unwrap_or(it.effects.len());
    it.effects.insert(pos, e);
}

fn render(s: &Scene) -> Image {
    render_sequence(&s.p, s.seq, R.tick_of(12), RenderOptions::default(), &s.map)
}

fn near(a: [f32; 4], b: [f32; 3], tol: f32) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() < tol)
}

#[test]
fn applies_to_everything_below_but_not_above() {
    let mut s = scene(3);
    let red = matte(&mut s.p, &mut s.map, [1.0, 0.0, 0.0, 1.0]);
    let blue = matte(&mut s.p, &mut s.map, [0.0, 0.0, 1.0, 1.0]);
    place(&mut s.p, s.seq, 0, red);
    let adj = adjustment(&mut s.p, s.seq, 1);
    fx(&mut s, adj, "invert");
    let img = render(&s);
    assert!(near(img.get(160, 90), [0.0, 1.0, 1.0], 1e-4), "red inverted to cyan: {:?}", img.get(160, 90));
    // a small blue layer above the adjustment layer is untouched
    let top = place(&mut s.p, s.seq, 2, blue);
    clip(&mut s, top).effect_mut("motion").unwrap().params.get_mut("scale").unwrap().value = ParamValue::Float(25.0);
    let img = render(&s);
    assert!(near(img.get(160, 90), [0.0, 0.0, 1.0], 1e-4), "{:?}", img.get(160, 90));
    assert!(near(img.get(10, 10), [0.0, 1.0, 1.0], 1e-4));
}

#[test]
fn opacity_and_blend_mode_mix_over_the_original() {
    let mut s = scene(2);
    let red = matte(&mut s.p, &mut s.map, [1.0, 0.0, 0.0, 1.0]);
    place(&mut s.p, s.seq, 0, red);
    let adj = adjustment(&mut s.p, s.seq, 1);
    fx(&mut s, adj, "invert");
    clip(&mut s, adj).effect_mut("opacity").unwrap().params.get_mut("opacity").unwrap().value = ParamValue::Float(25.0);
    let c = render(&s).get(100, 100);
    assert!(near(c, [0.75, 0.25, 0.25], 1e-3), "{c:?}");
    // Multiply: red × cyan = black at 100 %
    let op = clip(&mut s, adj).effect_mut("opacity").unwrap();
    op.params.get_mut("opacity").unwrap().value = ParamValue::Float(100.0);
    op.params.get_mut("blend").unwrap().value =
        ParamValue::Choice(filmcraft_project::effect::BLEND_MODES.iter().position(|b| *b == "Multiply").unwrap() as u32);
    let c = render(&s).get(100, 100);
    assert!(near(c, [0.0, 0.0, 0.0], 1e-3), "{c:?}");
}

#[test]
fn motion_and_masks_restrict_the_region() {
    let mut s = scene(2);
    let red = matte(&mut s.p, &mut s.map, [1.0, 0.0, 0.0, 1.0]);
    place(&mut s.p, s.seq, 0, red);
    let adj = adjustment(&mut s.p, s.seq, 1);
    fx(&mut s, adj, "invert");
    // Motion: scaled to 50 % around the centre → only the middle is inverted
    clip(&mut s, adj).effect_mut("motion").unwrap().params.get_mut("scale").unwrap().value = ParamValue::Float(50.0);
    let img = render(&s);
    assert!(near(img.get(160, 90), [0.0, 1.0, 1.0], 1e-4), "{:?}", img.get(160, 90));
    assert!(near(img.get(10, 10), [1.0, 0.0, 0.0], 1e-4), "{:?}", img.get(10, 10));
    clip(&mut s, adj).effect_mut("motion").unwrap().params.get_mut("scale").unwrap().value = ParamValue::Float(100.0);
    // a mask on the adjustment layer's effect (sequence pixels): left half
    let inv = clip(&mut s, adj).effects.iter_mut().find(|e| e.effect == "invert").unwrap();
    let mut m = Mask::new("m", MaskPath::rect(0.0, 0.0, 160.0, 180.0));
    m.feather.value = ParamValue::Float(0.0);
    inv.masks.push(m);
    let img = render(&s);
    assert!(near(img.get(40, 90), [0.0, 1.0, 1.0], 1e-4));
    assert!(near(img.get(280, 90), [1.0, 0.0, 0.0], 1e-4));
    // the same at half resolution (mask in sequence pixels scales with the output)
    let half = render_sequence(&s.p, s.seq, R.tick_of(12), RenderOptions { scale: 0.5, ..Default::default() }, &s.map);
    assert!(near(half.get(20, 45), [0.0, 1.0, 1.0], 1e-4));
    assert!(near(half.get(140, 45), [1.0, 0.0, 0.0], 1e-4));
    // an opacity mask on the adjustment layer: right half only (with the effect mask → nothing)
    let op = clip(&mut s, adj).effect_mut("opacity").unwrap();
    let mut m = Mask::new("o", MaskPath::rect(160.0, 0.0, 320.0, 180.0));
    m.feather.value = ParamValue::Float(0.0);
    op.masks.push(m);
    let img = render(&s);
    assert!(near(img.get(40, 90), [1.0, 0.0, 0.0], 1e-4), "{:?}", img.get(40, 90));
    assert!(near(img.get(280, 90), [1.0, 0.0, 0.0], 1e-4));
}

#[test]
fn nested_adjustment_layers_stay_in_their_sequence() {
    let mut s = scene(2);
    let red = matte(&mut s.p, &mut s.map, [1.0, 0.0, 0.0, 1.0]);
    let ocean = media(&mut s.p, &mut s.map, GeneratorSource::demo(DemoScene::OceanSunset));
    // inner sequence: red + adjustment (invert) → cyan
    let inner = s.p.new_sequence("inner", SequenceSettings { width: 320, height: 180, frame_rate: R, ..Default::default() }, 2, 0, None);
    place(&mut s.p, inner, 0, red);
    let a = s.p.add_item("Adjustment Layer", Label::Lavender, ItemKind::AdjustmentLayer { width: 320, height: 180, rate: R, duration: R.tick_of(48) }, None);
    let ac = place(&mut s.p, inner, 1, a);
    {
        let it = s.p.sequence_mut(inner).unwrap().find_item_mut(ac).unwrap().1;
        it.effects.insert(0, find_effect("invert").unwrap().instance());
    }
    // outer: ocean on V1, the nested sequence (scaled to 50 %) on V2
    place(&mut s.p, s.seq, 0, ocean);
    let nest = place(&mut s.p, s.seq, 1, inner);
    clip(&mut s, nest).effect_mut("motion").unwrap().params.get_mut("scale").unwrap().value = ParamValue::Float(50.0);
    let img = render(&s);
    assert!(near(img.get(160, 90), [0.0, 1.0, 1.0], 1e-4), "inner adjusted: {:?}", img.get(160, 90));
    // the outer V1 (outside the nest) is untouched
    let plain = {
        let mut p2 = s.p.clone();
        p2.sequence_mut(s.seq).unwrap().video_tracks[1].items.clear();
        render_sequence(&p2, s.seq, R.tick_of(12), RenderOptions::default(), &s.map)
    };
    assert_eq!(img.get(5, 5), plain.get(5, 5));
}

/// Upstream issue 1088: the Transform shutter must blur over the sequence's frame time. At 60 fps
/// the frames before a move starts and after it ends are at rest and stay sharp, on a clip and on
/// an adjustment layer alike (which once had no frame rate and blurred over a 24 fps frame).
#[test]
fn transform_shutter_blur_follows_the_sequence_frame_rate() {
    use filmcraft_project::{Interpolation, Keyframe};
    const R60: FrameRate = FrameRate::FPS_60;
    for on_adjustment in [false, true] {
        let mut p = Project::new("shutter");
        let seq = p.new_sequence("s", SequenceSettings { width: 320, height: 180, frame_rate: R60, ..Default::default() }, 2, 0, None);
        let mut map = SourceMap::default();
        let red =
            media(&mut p, &mut map, GeneratorSource::new(Generator::ColorMatte { color: [1.0, 0.0, 0.0, 1.0] }, 320, 180, R60, Tick(10 * TICKS_PER_SECOND)));
        let mut items = vec![(0, red)];
        if on_adjustment {
            items.push((
                1,
                p.add_item(
                    "Adjustment Layer",
                    Label::Lavender,
                    ItemKind::AdjustmentLayer { width: 320, height: 180, rate: R60, duration: R60.tick_of(120) },
                    None,
                ),
            ));
        }
        let mut target = None;
        for (track, item) in items {
            let mut ti = p.make_track_item(item, TrackKind::Video, Tick::ZERO, TimeRange::new(Tick::ZERO, R60.tick_of(120)), R60).unwrap();
            ti.scale_to_frame = true;
            if on_adjustment && track == 0 {
                // a box to move: a full-frame matte moved over itself would look unchanged
                ti.effect_mut("motion").unwrap().params.get_mut("scale").unwrap().value = ParamValue::Float(50.0);
            }
            target = Some(ti.id);
            let s = p.sequence_mut(seq).unwrap();
            s.video_tracks[track].items.push(ti);
        }
        let target = target.unwrap();
        let mut e = find_effect("transform").unwrap().instance();
        e.params.get_mut("shutter_override").unwrap().value = ParamValue::Bool(true);
        e.params.get_mut("shutter_angle").unwrap().value = ParamValue::Float(360.0);
        let key = |f: i64, x: f64| {
            let mut k = Keyframe::new(R60.tick_of(f), ParamValue::Vec2(filmcraft_geom::Vec2::new(x, 90.0)));
            k.interp = Interpolation::Linear;
            k
        };
        let pos = e.params.get_mut("position").unwrap();
        pos.keyframes = vec![key(30, 160.0), key(60, 260.0)];
        let it = p.sequence_mut(seq).unwrap().find_item_mut(target).unwrap().1;
        let at = it.effects.iter().position(|e| e.def().is_some_and(|d| d.intrinsic)).unwrap_or(it.effects.len());
        it.effects.insert(at, e);
        let mut sharp = p.clone();
        let it = sharp.sequence_mut(seq).unwrap().find_item_mut(target).unwrap().1;
        it.effects.iter_mut().find(|e| e.effect == "transform").unwrap().params.get_mut("shutter_override").unwrap().value = ParamValue::Bool(false);
        for frame in [28, 29, 61, 62] {
            let a = render_sequence(&p, seq, R60.tick_of(frame), RenderOptions::default(), &map);
            let b = render_sequence(&sharp, seq, R60.tick_of(frame), RenderOptions::default(), &map);
            let d = a.px.iter().zip(&b.px).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max);
            assert!(d < 1e-3, "adjustment layer: {on_adjustment}, frame {frame} at rest is blurred ({d})");
        }
        // in the middle of the move the shutter does blur
        let (a, b) = (
            render_sequence(&p, seq, R60.tick_of(45), RenderOptions::default(), &map),
            render_sequence(&sharp, seq, R60.tick_of(45), RenderOptions::default(), &map),
        );
        assert!(a.px.iter().zip(&b.px).any(|(x, y)| (x - y).abs() > 1e-2), "adjustment layer: {on_adjustment}: no blur mid-move");
        // the frame plan (GPU playback and export) blurs as the CPU compositor does
        let planned = plan::execute_cpu(&plan::plan_frame(&p, seq, R60.tick_of(45), RenderOptions::default(), &map));
        let d = planned.px.iter().zip(&a.px).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max);
        assert!(d < 1e-3, "adjustment layer: {on_adjustment}: the frame plan dropped the shutter blur ({d})");
    }
}
