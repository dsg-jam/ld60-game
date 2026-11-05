use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use ld60_geometry::triangles::{Rings, Skin, Triangles};
use rand::{Rng, RngExt};

use super::super::plumage::Feathers;
use super::model::{hull_place, neck_skin};
use super::skeleton::{Bone, HULL, HULL_RADII, NECK};

/// The way the neck leaves the hull.
const NECK_UP: Vec3 = Vec3::new(0.0, 0.876, 0.482);
/// The neck's thickness where the cloth goes round it.
const NECK_RADIUS: f32 = 0.034;
/// Cloth stays this far outside the hull.
const CLEAR: f32 = 0.004;
/// The scarf's ends hang from its knot, here, and swing on their own bone.
pub(super) const KNOT: Vec3 = Vec3::new(-0.03, 0.835, 0.135);
const AROUND: usize = 16;
/// The scarf is a knitted band this thick and half as wide.
const BAND_THICK: f32 = 0.0065;
const BAND_HALF: f32 = 0.014;

/// What a bird wears over its feathers, if anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum Dress {
    Bare,
    /// A knitted scarf wound twice round the root of the neck, its ends
    /// hanging over the breast.
    Scarf,
    /// A rolled knitted collar round the root of the neck.
    Cowl,
}

impl Dress {
    /// The dress named `name`, as the debug port names it.
    #[cfg(feature = "debug_port")]
    pub(in super::super) fn named(name: &str) -> Option<Self> {
        match name {
            "bare" => Some(Self::Bare),
            "scarf" => Some(Self::Scarf),
            "cowl" => Some(Self::Cowl),
            _ => None,
        }
    }

    pub(in super::super) fn pick(rng: &mut impl Rng) -> Self {
        match rng.random_range(0..3) {
            0 => Self::Bare,
            1 => Self::Scarf,
            _ => Self::Cowl,
        }
    }
}

/// `point` pushed out of the hull until it is `margin` clear of it.
fn outside_hull(point: Vec3, margin: f32) -> Vec3 {
    let hull = hull_place();
    let local = hull.rotation.inverse() * (point - HULL) / HULL_RADII;
    let reach = 1.0 + margin / HULL_RADII.x;
    let length = local.length();
    if length >= reach || length < 1e-4 {
        return point;
    }
    HULL + hull.rotation * (local * (reach / length) * HULL_RADII)
}

/// Sideways and forward across the neck where it leaves the hull.
fn neck_frame() -> (Vec3, Vec3) {
    let side = Vec3::X;
    (side, NECK_UP.cross(side))
}

/// A scarf, bound to the neck, the hull and the knot.
pub(super) fn scarf(rng: &mut impl Rng) -> Mesh {
    let mut mesh = Triangles::default();
    let mut cloth = Feathers(&mut mesh);
    let (side, front) = neck_frame();
    for (wrap, rise) in [0.016, 0.038].into_iter().enumerate() {
        let center = NECK + NECK_UP * rise;
        let reach = NECK_RADIUS + BAND_THICK * (1.0 + wrap as f32) + CLEAR;
        let lump = rng.random_range(0.0..TAU);
        let mark = cloth.mark();
        let rings = Rings::new((0..=48).map(|step| {
            let angle = TAU * step as f32 / 48.0 + wrap as f32 * 0.4;
            let out = side * angle.cos() + front * angle.sin();
            // Loose at the front, where it sags.
            let sag = angle.sin().max(0.0);
            let spine = center + out * (reach + 0.008 * sag) - NECK_UP * 0.012 * sag;
            let spine = outside_hull(spine, BAND_THICK + CLEAR);
            // The band bunches up in soft ripples as it winds round.
            let half = BAND_HALF * (1.0 + 0.15 * (angle * 3.0 + lump).sin());
            let ripple = 0.0025 * (angle * 7.0 + lump).sin();
            (0..AROUND).map(move |index| {
                let turn = TAU * index as f32 / AROUND as f32;
                let flat = turn.sin().signum() * turn.sin().abs().powf(0.5);
                spine + out * (turn.cos() * BAND_THICK + ripple) + NECK_UP * flat * half
            })
        }));
        cloth.0.rings(&rings, 0..rings.len());
        cloth.paint_cloth(mark, Vec2::new(8.0, 70.0));
        cloth.bind_since(mark, neck_skin);
    }

    let mark = cloth.mark();
    for (index, (length, lean)) in [(0.19, 0.012), (0.14, -0.01)].into_iter().enumerate() {
        let start = KNOT + Vec3::new(index as f32 * 0.018, -0.005, -0.004 * index as f32);
        end(&mut cloth, start, length, lean);
    }
    cloth.bind_since(mark, |point| {
        let hang = ((KNOT.y - point.y) / 0.15).clamp(0.0, 1.0);
        Skin::between(Bone::Hull.joint(), Bone::Scarf.joint(), hang)
    });
    mesh.build()
}

/// One end of a scarf, hanging from `start` over the breast.
fn end(cloth: &mut Feathers, start: Vec3, length: f32, lean: f32) {
    const STEPS: usize = 14;
    const WIDTH: f32 = 0.017;
    const THICK: f32 = 0.0035;
    let spine: Vec<Vec3> = (0..=STEPS)
        .map(|step| {
            let t = step as f32 / STEPS as f32;
            let at = start + Vec3::new(lean * t, -length * t, 0.0);
            outside_hull(at, THICK + CLEAR + 0.002)
        })
        .collect();
    let mark = cloth.mark();
    let rings = Rings::new((0..=STEPS).map(|step| {
        let before = spine[step.saturating_sub(1)];
        let after = spine[(step + 1).min(STEPS)];
        let along = (after - before).normalize_or(Vec3::NEG_Y);
        let out = along.cross(Vec3::X).normalize_or(Vec3::Z);
        let out = if out.z < 0.0 { -out } else { out };
        let across = out.cross(along).normalize_or(Vec3::X);
        // Each end closes at its tip, a little rounded.
        let close = if step == STEPS { 0.3 } else { 1.0 };
        let at = spine[step];
        (0..AROUND).map(move |index| {
            let turn = TAU * index as f32 / AROUND as f32;
            let flat = turn.cos().signum() * turn.cos().abs().powf(0.4);
            at + (across * flat * WIDTH + out * turn.sin() * THICK) * close
        })
    }));
    cloth.0.rings(&rings, 0..rings.len());
    cloth.paint_cloth(mark, Vec2::new(6.0, 40.0 * length / 0.19));
}

/// A rolled collar, bound to the hull and the neck.
pub(super) fn cowl(rng: &mut impl Rng) -> Mesh {
    let mut mesh = Triangles::default();
    let mut cloth = Feathers(&mut mesh);
    let (side, front) = neck_frame();
    let folds = rng.random_range(0.0..TAU);
    const ROWS: usize = 16;
    let mark = cloth.mark();
    let rings = Rings::new((0..=ROWS).map(|row| {
        let t = row as f32 / ROWS as f32;
        // Up the outside, over the rolled top and back down inside.
        let roll = PI * t;
        let rise = -0.01 + 0.075 * roll.sin().powf(0.5) * (1.0 - 0.3 * t);
        let reach =
            NECK_RADIUS + CLEAR + 0.01 + 0.014 * (1.0 - t) + 0.008 * (roll * 3.0).sin().abs();
        (0..48).map(move |index| {
            let angle = TAU * index as f32 / 48.0;
            let out = side * angle.cos() + front * angle.sin();
            let fold = 1.0 + 0.05 * (angle * 6.0 + folds + t * 2.0).sin();
            let at = NECK + NECK_UP * rise + out * reach * fold;
            outside_hull(at, CLEAR + 0.003)
        })
    }));
    cloth.0.rings(&rings, 0..rings.len());
    cloth.paint_cloth(mark, Vec2::new(40.0, 10.0));
    cloth.bind_since(mark, neck_skin);
    mesh.build()
}
