use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use ld60_geometry::triangles::{Skin, Triangles};
use rand::{Rng, RngExt};

/// Segments along each feather, so it can curl.
const SEGMENTS: u32 = 3;
/// Feathers lift off the body by this angle, so the rows overlap like
/// shingles.
const LIFT: f32 = 0.12;
/// The shading normal leans this far from the card towards the body's,
/// so the coat shades soft instead of in facets.
const SOFT: f32 = 0.85;

/// Matches the constants in `plumage.wesl`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in super::super) enum Kind {
    Contour = 0,
    /// Shimmers in changing colors, on the neck.
    Shimmer = 1,
    /// Long and narrow, for crests and tails.
    Plume = 2,
    /// The soft down under the feathers, a solid surface.
    Down = 3,
}

/// Feathers written into a mesh. The UV runs across a card in X and from
/// the root to the tip in Y. The vertex color holds a shade in red, the
/// [`Kind`] in green and a random number in blue.
pub(in super::super) struct Feathers<'a>(pub(in super::super) &'a mut Triangles);

/// One feather.
#[derive(Debug, Clone, Copy)]
pub(in super::super) struct Feather {
    pub(in super::super) root: Vec3,
    /// Along the body, the way the feather lies.
    pub(in super::super) along: Vec3,
    /// Out of the body.
    pub(in super::super) out: Vec3,
    pub(in super::super) length: f32,
    pub(in super::super) width: f32,
    /// How far the tip bends back towards the body, in radians.
    pub(in super::super) curl: f32,
    pub(in super::super) kind: Kind,
}

/// Feathers covering an ellipsoid, see [`Feathers::coat`].
#[derive(Debug, Clone, Copy)]
pub(in super::super) struct Cover {
    pub(in super::super) center: Vec3,
    pub(in super::super) radii: Vec3,
    pub(in super::super) count: usize,
    /// The way the feathers lie, before it follows the surface.
    pub(in super::super) flow: Vec3,
    pub(in super::super) length: f32,
    pub(in super::super) kind: Kind,
}

impl Feathers<'_> {
    /// The down under a coat, a little smaller than it.
    pub(in super::super) fn down(&mut self, center: Vec3, radii: Vec3) {
        self.0
            .set_color([1.0, Kind::Down as u32 as f32 / 2.0, 0.0, 1.0]);
        self.0
            .ellipsoid(radii, &Transform::from_translation(center), 10, 14);
    }

    /// Feathers spread evenly over an ellipsoid, all lying the way of its
    /// `flow`, except where `bare` says so.
    pub(in super::super) fn coat(
        &mut self,
        coat: Cover,
        bare: impl Fn(Vec3) -> bool,
        rng: &mut impl Rng,
    ) {
        for index in 0..coat.count {
            let y = 1.0 - 2.0 * (index as f32 + 0.5) / coat.count as f32;
            let ring = (1.0 - y * y).sqrt();
            let angle = index as f32 * PI * (3.0 - 5f32.sqrt());
            let unit = Vec3::new(ring * angle.cos(), y, ring * angle.sin());
            let root = coat.center + unit * coat.radii * 0.95;
            if bare(root) {
                continue;
            }
            let out = (unit / coat.radii).normalize();
            let along = (coat.flow - out * coat.flow.dot(out))
                .try_normalize()
                .unwrap_or(Vec3::NEG_Y);
            let length = coat.length * rng.random_range(0.85..1.15);
            self.push(
                Feather {
                    root,
                    along,
                    out,
                    length,
                    width: length * 0.85,
                    curl: 0.6,
                    kind: coat.kind,
                },
                rng,
            );
        }
    }

    /// Adds a feather. Plumes stand free of the body, so they get a second
    /// card across the first, to be seen from any side.
    pub(in super::super) fn push(&mut self, feather: Feather, rng: &mut impl Rng) {
        if feather.kind == Kind::Plume {
            let turned = Quat::from_axis_angle(feather.along, FRAC_PI_2) * feather.out;
            self.card(
                Feather {
                    out: turned,
                    ..feather
                },
                rng,
            );
        }
        self.card(feather, rng);
    }

    /// Binds the feathers and down added since `mark` with `skin`.
    pub(in super::super) fn bind_since(&mut self, mark: usize, skin: impl Fn(Vec3) -> Skin) {
        self.0.bind_since(mark, skin);
    }

    pub(in super::super) fn mark(&self) -> usize {
        self.0.mark()
    }

    fn card(&mut self, feather: Feather, rng: &mut impl Rng) {
        let Feather {
            root,
            along,
            out,
            length,
            width,
            curl,
            kind,
        } = feather;
        let across = out.cross(along).normalize_or_zero();
        let lifted = (along * LIFT.cos() + out * LIFT.sin()).normalize();
        let shade = rng.random_range(0.85..1.15);
        self.0
            .set_color([shade, kind as u32 as f32 / 2.0, rng.random(), 1.0]);
        let mut at = root;
        let mut direction = lifted;
        let mut last: Option<[u32; 2]> = None;
        for segment in 0..=SEGMENTS {
            let t = segment as f32 / SEGMENTS as f32;
            let face = direction.cross(across).normalize_or_zero();
            let normal = face.lerp(out, SOFT).normalize_or_zero();
            let pair = [0.0, 1.0].map(|side| {
                self.0.vertex_at(
                    at + across * (side - 0.5) * width,
                    normal,
                    Vec2::new(side, t),
                )
            });
            if let Some([a, b]) = last {
                self.0.triangle([a, b, pair[1]]);
                self.0.triangle([a, pair[1], pair[0]]);
            }
            last = Some(pair);
            at += direction * length / SEGMENTS as f32;
            direction = Quat::from_axis_angle(across, curl / SEGMENTS as f32) * direction;
        }
    }
}
