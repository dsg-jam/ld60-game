use std::f32::consts::{PI, TAU};

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
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
#[derive(Debug, Clone, Copy)]
enum Kind {
    Contour = 0,
    /// Shimmers in changing colors, on the neck and head.
    Shimmer = 1,
    /// Long and narrow, for crests and tails.
    Plume = 2,
}

/// Feather cards merged into one mesh. The UV runs across the card in X
/// and from the root to the tip in Y. The vertex color holds a shade in
/// red, the [`Kind`] in green and a random number in blue.
#[derive(Debug, Default)]
pub(in super::super) struct Feathers {
    points: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

/// One feather.
#[derive(Debug, Clone, Copy)]
struct Feather {
    root: Vec3,
    /// Along the body, the way the feather lies.
    along: Vec3,
    /// Out of the body.
    out: Vec3,
    length: f32,
    width: f32,
    /// How far the tip bends back towards the body, in radians.
    curl: f32,
    kind: Kind,
}

impl Feathers {
    /// The coat of the hull, on a sphere one unit across, lying back and
    /// down from the front.
    pub(in super::super) fn body(rng: &mut impl Rng) -> Mesh {
        const COUNT: usize = 300;
        let mut feathers = Self::default();
        for index in 0..COUNT {
            // Spread evenly over the sphere.
            let y = 1.0 - 2.0 * (index as f32 + 0.5) / COUNT as f32;
            let ring = (1.0 - y * y).sqrt();
            let angle = index as f32 * PI * (3.0 - 5f32.sqrt());
            let out = Vec3::new(ring * angle.cos(), y, ring * angle.sin());
            let flow = Vec3::new(0.0, -0.45, -1.0);
            let along = (flow - out * flow.dot(out))
                .try_normalize()
                .unwrap_or(Vec3::NEG_Y);
            // Longer on the back and the flanks, short on the breast.
            let length = 0.27 * (1.0 - 0.35 * out.z.max(0.0)) * rng.random_range(0.85..1.15);
            feathers.push(
                Feather {
                    root: out * 0.47,
                    along,
                    out,
                    length,
                    width: length * 0.85,
                    curl: 0.6,
                    kind: Kind::Contour,
                },
                rng,
            );
        }
        feathers.into_mesh()
    }

    /// Feathers down the neck from `from` at the hull to `to` at the head,
    /// in meters, with a ruff of long ones where it meets the hull.
    pub(in super::super) fn neck(from: Vec3, to: Vec3, radius: f32, rng: &mut impl Rng) -> Mesh {
        const RINGS: usize = 9;
        const AROUND: usize = 7;
        let mut feathers = Self::default();
        let axis = (to - from).normalize();
        let side = axis.any_orthonormal_vector();
        for ring in 0..RINGS {
            let t = 1.0 - ring as f32 / RINGS as f32;
            let center = from.lerp(to, t);
            let girth = radius * (1.15 - 0.3 * t);
            for step in 0..AROUND {
                let angle = (step as f32 + 0.5 * (ring % 2) as f32) * TAU / AROUND as f32;
                let out = Quat::from_axis_angle(axis, angle) * side;
                let length = girth * 1.8 * rng.random_range(0.9..1.1);
                feathers.push(
                    Feather {
                        root: center + out * girth * 0.7,
                        along: -axis,
                        out,
                        length,
                        width: length * 0.6,
                        curl: 0.3,
                        kind: Kind::Shimmer,
                    },
                    rng,
                );
            }
        }
        for step in 0..14 {
            let angle = step as f32 * TAU / 14.0;
            let out = Quat::from_axis_angle(axis, angle) * side;
            let along = (out * 0.8 - axis * 0.6).normalize();
            let length = radius * 3.2 * rng.random_range(0.85..1.15);
            feathers.push(
                Feather {
                    root: from + axis * radius * 0.6 + out * radius,
                    along,
                    out: (out + axis * 0.5).normalize(),
                    length,
                    width: length * 0.45,
                    curl: 0.5,
                    kind: Kind::Contour,
                },
                rng,
            );
        }
        feathers.into_mesh()
    }

    /// Feathers over the back of the head and a crest of plumes, in meters
    /// for a head of size 1, its face towards +Z.
    pub(in super::super) fn crest(rng: &mut impl Rng) -> Mesh {
        let mut feathers = Self::default();
        for step in 0..12 {
            let angle = PI * (step as f32 / 11.0 - 0.5);
            let out = Vec3::new(angle.sin(), angle.cos() * 0.8, -0.4).normalize();
            feathers.push(
                Feather {
                    root: Vec3::new(0.0, 0.0, -0.02) + out * 0.08,
                    along: Vec3::new(0.0, -0.3, -1.0).normalize(),
                    out,
                    length: 0.09,
                    width: 0.05,
                    curl: 0.4,
                    kind: Kind::Shimmer,
                },
                rng,
            );
        }
        let plumes = rng.random_range(3..7);
        for index in 0..plumes {
            let spread = (index as f32 / (plumes - 1) as f32 - 0.5) * 0.5;
            let along = Vec3::new(spread, rng.random_range(0.5..0.9), -0.9).normalize();
            let length = rng.random_range(0.14..0.24);
            feathers.push(
                Feather {
                    root: Vec3::new(spread * 0.06, 0.07, 0.02),
                    along,
                    out: Vec3::new(spread, 0.3, 0.6).normalize(),
                    length,
                    width: 0.055,
                    curl: -0.6,
                    kind: Kind::Plume,
                },
                rng,
            );
        }
        feathers.into_mesh()
    }

    /// A fan of plumes towards -Z from the origin, in meters for a body of
    /// size 1.
    pub(in super::super) fn tail(rng: &mut impl Rng) -> Mesh {
        let mut feathers = Self::default();
        let plumes = rng.random_range(5..9);
        for index in 0..plumes {
            let spread = index as f32 / (plumes - 1) as f32 - 0.5;
            let along = Vec3::new(spread * 0.9, rng.random_range(-0.1..0.25), -1.0).normalize();
            let length = rng.random_range(0.3..0.5) * (1.0 - 0.4 * spread.abs());
            feathers.push(
                Feather {
                    root: Vec3::new(spread * 0.08, 0.0, 0.05),
                    along,
                    out: Vec3::new(spread * 0.3, 1.0, 0.0).normalize(),
                    length,
                    width: 0.07,
                    curl: -0.25,
                    kind: Kind::Plume,
                },
                rng,
            );
        }
        feathers.into_mesh()
    }

    /// Feathers hanging around the top of a limb along -Y from the origin,
    /// `length` meters down, for a limb of size 1.
    pub(in super::super) fn cuff(length: f32, radius: f32, rng: &mut impl Rng) -> Mesh {
        const AROUND: usize = 8;
        let mut feathers = Self::default();
        for ring in 0..3 {
            let y = -length * ring as f32 / 3.0;
            for step in 0..AROUND {
                let angle = (step as f32 + 0.5 * ring as f32) * TAU / AROUND as f32;
                let out = Vec3::new(angle.cos(), 0.0, angle.sin());
                let long = length * rng.random_range(0.5..0.7);
                feathers.push(
                    Feather {
                        root: Vec3::Y * y + out * radius,
                        along: Vec3::NEG_Y,
                        out,
                        length: long,
                        width: long * 0.6,
                        curl: 0.3,
                        kind: Kind::Contour,
                    },
                    rng,
                );
            }
        }
        feathers.into_mesh()
    }

    /// Adds a feather. Plumes stand free of the body, so they get a second
    /// card across the first, to be seen from any side.
    fn push(&mut self, feather: Feather, rng: &mut impl Rng) {
        if let Kind::Plume = feather.kind {
            let turned = Quat::from_axis_angle(feather.along, PI / 2.0) * feather.out;
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
        let seed = rng.random::<f32>();
        let color = [shade, kind as u32 as f32 / 2.0, seed, 1.0];
        let base = self.points.len() as u32;
        let mut at = root;
        let mut direction = lifted;
        for segment in 0..=SEGMENTS {
            let t = segment as f32 / SEGMENTS as f32;
            let face = direction.cross(across).normalize_or_zero();
            let normal = face.lerp(out, SOFT).normalize_or_zero();
            for side in [0.0, 1.0] {
                let point = at + across * (side - 0.5) * width;
                self.points.push(point.to_array());
                self.normals.push(normal.to_array());
                self.uvs.push([side, t]);
                self.colors.push(color);
            }
            at += direction * length / SEGMENTS as f32;
            direction = Quat::from_axis_angle(across, curl / SEGMENTS as f32) * direction;
        }
        for segment in 0..SEGMENTS {
            let corner = base + segment * 2;
            self.indices.extend([
                corner,
                corner + 1,
                corner + 2,
                corner + 1,
                corner + 3,
                corner + 2,
            ]);
        }
    }

    fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.points)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}
