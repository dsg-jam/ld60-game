use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::prelude::*;
use ld60_geometry::triangles::{End, Skin, Triangles};
use rand::{Rng, RngExt};

/// Segments along each feather, so it can curl.
const SEGMENTS: u32 = 3;
/// Feathers lift off the body by this angle, so the rows overlap like
/// shingles.
const LIFT: f32 = 0.05;
/// The shading normal leans this far from the card towards the body's,
/// so the coat shades soft instead of in facets.
const SOFT: f32 = 0.9;
/// The most rows of small feathers down can have, as they are written
/// into the vertex color as a share of it. Matches `plumage.wesl`.
const CELLS: f32 = 40.0;
/// Feathers differ this much in shade, so the coat reads as one surface.
const SHADES: std::ops::Range<f32> = 0.95..1.05;

/// Matches the constants in `plumage.wesl`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in super::super) enum Kind {
    Contour = 0,
    /// Long and narrow, for crests, breasts and tails.
    Plume = 1,
    /// The soft down under the feathers, a solid surface.
    Down = 2,
}

/// Which color of the coat a feather takes. Matches the constants in
/// `plumage.wesl`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in super::super) enum Region {
    Back = 0,
    /// The throat, the breast and the face.
    Pale = 1,
    /// The crest and the stripe from the eye.
    Dark = 2,
    /// The flight feathers, from the back color to dark at the tips.
    Primary = 3,
}

/// Feathers written into a mesh. The UV runs across a card in X and from
/// the root to the tip in Y. The vertex color holds a shade in red, the
/// [`Kind`] and [`Region`] in green and a random number in blue. On down,
/// red and blue hold how many small feathers it shows along and around.
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
    pub(in super::super) region: Region,
}

/// Feathers covering an ellipsoid, see [`Feathers::coat`].
#[derive(Debug, Clone, Copy)]
pub(in super::super) struct Cover {
    pub(in super::super) center: Vec3,
    pub(in super::super) radii: Vec3,
    pub(in super::super) turn: Quat,
    pub(in super::super) count: usize,
    /// The way the feathers lie, before it follows the surface.
    pub(in super::super) flow: Vec3,
    pub(in super::super) length: f32,
}

impl Region {
    fn code(self, kind: Kind) -> f32 {
        (kind as u32 * 4 + self as u32) as f32 / 15.0
    }
}

impl Feathers<'_> {
    /// The down under a coat, a little smaller than it, colored by where
    /// it is and which way it faces.
    pub(in super::super) fn down(
        &mut self,
        radii: Vec3,
        place: &Transform,
        cells: Vec2,
        region: impl Fn(Vec3, Vec3) -> Region,
    ) {
        let mark = self.0.mark();
        self.0.ellipsoid(radii, place, 16, 24);
        self.paint_down(mark, cells, region);
    }

    /// Down around `spine`, like on a neck.
    pub(in super::super) fn down_tube(
        &mut self,
        spine: &[Vec3],
        radius: impl Fn(f32) -> f32,
        cells: Vec2,
        region: impl Fn(Vec3, Vec3) -> Region,
    ) {
        let mark = self.0.mark();
        self.0.tube(spine, radius, 20, [End::Round, End::Round]);
        self.paint_down(mark, cells, region);
    }

    /// Makes the vertices added since `mark` down, colored by `region`,
    /// with `cells` rows of small feathers around and along its UV.
    pub(in super::super) fn paint_down(
        &mut self,
        mark: usize,
        cells: Vec2,
        region: impl Fn(Vec3, Vec3) -> Region,
    ) {
        self.0.color_since(mark, |position, normal| {
            [
                cells.y / CELLS,
                region(position, normal).code(Kind::Down),
                cells.x / CELLS,
                1.0,
            ]
        });
    }

    /// Feathers spread evenly over an ellipsoid, all lying the way of its
    /// `flow`, in the region `region` gives them, or none where it gives
    /// none.
    pub(in super::super) fn coat(
        &mut self,
        coat: Cover,
        region: impl Fn(Vec3, Vec3) -> Option<Region>,
        rng: &mut impl Rng,
    ) {
        for index in 0..coat.count {
            let y = 1.0 - 2.0 * (index as f32 + 0.5) / coat.count as f32;
            let ring = (1.0 - y * y).sqrt();
            let angle = index as f32 * PI * (3.0 - 5f32.sqrt());
            let unit = Vec3::new(ring * angle.cos(), y, ring * angle.sin());
            let root = coat.center + coat.turn * (unit * coat.radii * 0.95);
            let out = coat.turn * (unit / coat.radii).normalize();
            let Some(region) = region(root, out) else {
                continue;
            };
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
                    width: length * 0.8,
                    curl: 0.5,
                    kind: Kind::Contour,
                    region,
                },
                rng,
            );
        }
    }

    /// Rings of feathers around `spine`, `radius(along)` thick, lying from
    /// its start towards its end.
    pub(in super::super) fn sleeve(
        &mut self,
        spine: &[Vec3],
        radius: impl Fn(f32) -> f32,
        length: f32,
        region: impl Fn(Vec3, Vec3) -> Region,
        rng: &mut impl Rng,
    ) {
        const AROUND: usize = 9;
        let total: f32 = spine.windows(2).map(|pair| pair[0].distance(pair[1])).sum();
        let rings = (total / (length * 0.4)).ceil().max(1.0) as usize;
        let mut side = (spine[1] - spine[0]).normalize().any_orthonormal_vector();
        for ring in 0..rings {
            let t = ring as f32 / rings as f32;
            let (center, axis) = point_along(spine, t * total);
            side = (side - axis * side.dot(axis)).normalize_or(axis.any_orthonormal_vector());
            for step in 0..AROUND {
                let angle = (step as f32 + 0.5 * (ring % 2) as f32) * TAU / AROUND as f32;
                let out = Quat::from_axis_angle(axis, angle) * side;
                let root = center + out * radius(t) * 0.85;
                let length = length * rng.random_range(0.85..1.15);
                self.push(
                    Feather {
                        root,
                        along: axis,
                        out,
                        length,
                        width: length * 0.75,
                        curl: 0.35,
                        kind: Kind::Contour,
                        region: region(root, out),
                    },
                    rng,
                );
            }
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
            region,
        } = feather;
        let across = out.cross(along).normalize_or_zero();
        let lifted = (along * LIFT.cos() + out * LIFT.sin()).normalize();
        self.0.set_color([
            rng.random_range(SHADES),
            region.code(kind),
            rng.random(),
            1.0,
        ]);
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

/// The point `distance` along `spine` and the way it runs there.
pub(in super::super) fn point_along(spine: &[Vec3], distance: f32) -> (Vec3, Vec3) {
    let mut left = distance;
    for pair in spine.windows(2) {
        let length = pair[0].distance(pair[1]);
        if left <= length {
            let axis = (pair[1] - pair[0]).normalize_or(Vec3::NEG_Y);
            return (pair[0] + axis * left, axis);
        }
        left -= length;
    }
    let count = spine.len();
    let axis = (spine[count - 1] - spine[count - 2]).normalize_or(Vec3::NEG_Y);
    (spine[count - 1], axis)
}
