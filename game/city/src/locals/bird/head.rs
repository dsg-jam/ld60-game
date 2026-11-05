use std::f32::consts::TAU;

use bevy::prelude::*;
use ld60_geometry::triangles::{Rings, Skin, Triangles};
use rand::Rng;

use super::super::plumage::{Feather, Feathers, Kind, Region};
use super::skeleton::{Bone, HEAD, Side};

/// The head and the bill are one body along +Z, from the back of the head
/// to the tip of the bill, relative to the middle of the head. The bill
/// starts at ring [`JOIN`].
const BACK: f32 = -0.05;
const TIP: f32 = 0.217;
const RINGS: usize = 40;
const AROUND: usize = 24;
pub(super) const JOIN: usize = 14;
/// How far the tip of the bill hangs below the middle of the head.
const TIP_DROP: f32 = 0.03;
/// The eye: how far along the head, how far above its side in radians,
/// and how far it sits inside the surface. Matches `plumage.wesl`.
const EYE_ALONG: f32 = 0.012;
const EYE_RISE: f32 = 0.3;
const EYE_SINK: f32 = 0.0055;
pub(super) const EYE_RADIUS: f32 = 0.0105;
/// The upper lid covers this much of the eye, from its top, in radians.
const LID: f32 = 0.75;
/// Rows of small feathers on the head, around and along.
const CELLS: Vec2 = Vec2::new(26.0, 30.0);

/// How far along the head ring `ring` is.
fn ring_z(ring: usize) -> f32 {
    BACK + (TIP - BACK) * ring as f32 / RINGS as f32
}

/// The middle height, half height and half width at `z`, and how square
/// the cross section is. The crown runs on in one straight line to the
/// tip of the bill, the chin in another, so head and bill make one wedge.
fn profile(z: f32) -> (f32, f32, f32, f32) {
    const CROWN: f32 = 0.03;
    const CHIN: f32 = -0.027;
    const BROW: f32 = -0.005;
    let round = (1.0 - (1.0 - ((z - BACK) / 0.025).clamp(0.0, 1.0)).powi(2)).sqrt();
    let ahead = ((z - BROW) / (TIP - BROW)).clamp(0.0, 1.0);
    let top = CROWN + (-TIP_DROP - CROWN) * ahead;
    let bottom = CHIN + (-TIP_DROP - CHIN) * ahead;
    let width = 0.023 * round * (1.0 - ahead).powf(1.3);
    let s = ((z - ring_z(JOIN)) / (TIP - ring_z(JOIN))).clamp(0.0, 1.0);
    (
        (top + bottom) / 2.0,
        (top - bottom) / 2.0 * round,
        width,
        2.4 - 0.4 * s,
    )
}

/// The point on a cross section at `angle` around from the right side
/// towards the top.
fn around(angle: f32, middle: f32, height: f32, width: f32, square: f32) -> Vec2 {
    let bend = |value: f32| value.signum() * value.abs().powf(2.0 / square);
    Vec2::new(
        width * bend(angle.cos()),
        middle + height * bend(angle.sin()),
    )
}

/// The head and the bill in the rest pose.
pub(super) fn rings() -> Rings {
    Rings::new((0..=RINGS).map(|ring| {
        let z = ring_z(ring);
        let (middle, height, width, square) = profile(z);
        let bill = ring > JOIN;
        let s = (ring as f32 - JOIN as f32) / (RINGS - JOIN) as f32;
        (0..AROUND).map(move |index| {
            let angle = TAU * index as f32 / AROUND as f32;
            // The gape: a groove along each side of the bill.
            let rise = angle.sin().asin();
            let gape = if bill && s < 0.85 && rise.abs() < 0.14 {
                0.88
            } else {
                1.0
            };
            let at = around(angle, middle, height, width * gape, square);
            HEAD + Vec3::new(at.x, at.y, z)
        })
    }))
}

/// The right eye, relative to the middle of the head.
pub(super) fn eye() -> Vec3 {
    let (middle, height, width, square) = profile(EYE_ALONG);
    let at = around(EYE_RISE, middle, height, width, square);
    Vec3::new(at.x - EYE_SINK, at.y, EYE_ALONG)
}

/// The feathered part of the head, the lids and the crest, bound to the
/// head.
pub(super) fn plumage(feathers: &mut Feathers, rng: &mut impl Rng) {
    let mark = feathers.mark();
    feathers.0.rings(&rings(), 0..JOIN + 1);
    feathers.paint_down(mark, CELLS, |_, _| Region::Face);
    for side in Side::BOTH {
        lid(feathers, HEAD + side.mirror(eye()), side);
    }
    for index in 0..2 {
        let spread = index as f32 - 0.5;
        feathers.push(
            Feather {
                root: HEAD + Vec3::new(spread * 0.008, 0.024, -0.03),
                along: Vec3::new(spread * 0.15, -0.7, -0.7).normalize(),
                out: Vec3::new(0.0, 0.85, -0.5).normalize(),
                length: 0.12 + 0.02 * index as f32,
                width: 0.012,
                curl: 0.45,
                kind: Kind::Plume,
                region: Region::Dark,
            },
            rng,
        );
    }
    feathers.bind_since(mark, |_| Skin::one(Bone::Head.joint()));
}

/// The upper lid over an eye: a cap of bare skin over its top, a little
/// lowered, so the eye looks calm.
fn lid(feathers: &mut Feathers, eye: Vec3, side: Side) {
    let radius = EYE_RADIUS * 1.12;
    let profile: Vec<Vec2> = (0..=5)
        .map(|step| {
            let angle = LID * step as f32 / 5.0;
            Vec2::new(angle.sin(), angle.cos()) * radius
        })
        .collect();
    let mark = feathers.mark();
    feathers.0.lathe(&profile, 12);
    let place = Transform::from_translation(eye).with_rotation(
        Quat::from_rotation_y(0.4 * side.sign()) * Quat::from_rotation_z(-0.1 * side.sign()),
    );
    feathers.0.transform_since(mark, &place);
    feathers.paint_down(mark, Vec2::new(6.0, 2.0), |_, _| Region::Lore);
}

/// The bill, bound to the head.
pub(super) fn bill() -> Mesh {
    let mut mesh = Triangles::default();
    mesh.rings(&rings(), JOIN..RINGS + 1);
    mesh.bind_since(0, |_| Skin::one(Bone::Head.joint()));
    mesh.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eye_shows_through_the_surface() {
        let (middle, height, width, square) = profile(EYE_ALONG);
        let surface = around(EYE_RISE, middle, height, width, square).x;
        let eye = eye();
        assert!(eye.x < surface && eye.x + EYE_RADIUS > surface + 0.003);
    }

    #[test]
    fn bill_closes_at_its_tip() {
        let (_, height, width, _) = profile(TIP);
        assert!(height < 1e-4 && width < 1e-4);
    }

    #[test]
    fn head_meets_the_bill_without_a_step() {
        let join = ring_z(JOIN);
        let before = profile(join - 1e-4);
        let after = profile(join + 1e-4);
        assert!((before.1 - after.1).abs() < 1e-3);
        assert!((before.2 - after.2).abs() < 1e-3);
    }
}
