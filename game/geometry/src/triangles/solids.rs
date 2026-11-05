use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::prelude::*;

use super::Triangles;

/// Rings in the round end of a tube.
const ROUND_RINGS: u32 = 3;

/// How a tube ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum End {
    /// Left open.
    Open,
    /// Closed by a flat disc.
    Flat,
    /// Closed by half a ball.
    Round,
}

/// Rings of points around a body, each running the same way around it,
/// from one end to the other. Rings that shrink to a point close an end.
///
/// ```
/// # use bevy::prelude::*;
/// # use ld60_geometry::triangles::{Rings, Triangles};
/// let rings = Rings::new((0..=8).map(|step| {
///     let z = step as f32 / 8.0;
///     let radius = (std::f32::consts::PI * z).sin() * 0.1;
///     (0..12).map(move |side| {
///         let angle = std::f32::consts::TAU * side as f32 / 12.0;
///         Vec3::new(angle.cos() * radius, angle.sin() * radius, z)
///     })
/// }));
/// let mut mesh = Triangles::default();
/// mesh.rings(&rings, 0..rings.len());
/// ```
#[derive(Debug, Clone)]
pub struct Rings {
    points: Vec<Vec<Vec3>>,
}

impl Rings {
    /// Panics if the rings differ in length.
    pub fn new<I: IntoIterator<Item = Vec3>>(rings: impl IntoIterator<Item = I>) -> Self {
        let points: Vec<Vec<Vec3>> = rings
            .into_iter()
            .map(|ring| ring.into_iter().collect())
            .collect();
        assert!(
            points.windows(2).all(|pair| pair[0].len() == pair[1].len()),
            "rings of different lengths"
        );
        Self { points }
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    fn middle(&self, ring: usize) -> Vec3 {
        let points = &self.points[ring];
        points.iter().sum::<Vec3>() / points.len() as f32
    }

    /// The smooth normal at a point, from the points around it on all the
    /// rings, so a body cut into parts shades without a seam.
    fn normal(&self, ring: usize, index: usize) -> Vec3 {
        let around = self.points[ring].len();
        let last = self.points.len() - 1;
        let here = self.points[ring][index];
        let across = self.points[ring][(index + 1) % around]
            - self.points[ring][(index + around - 1) % around];
        let along =
            self.points[(ring + 1).min(last)][index] - self.points[ring.saturating_sub(1)][index];
        let outward = if ring == 0 {
            here - self.middle(1.min(last))
        } else if ring == last {
            here - self.middle(last - 1)
        } else {
            here - self.middle(ring)
        };
        let normal = along.cross(across);
        let normal = if normal.length_squared() < 1e-12 {
            outward
        } else if normal.dot(outward) < 0.0 {
            -normal
        } else {
            normal
        };
        normal.normalize_or(Vec3::Y)
    }
}

impl Triangles {
    /// The strip of `rings` over the rings in `range`, smooth across the
    /// ends of the range. The UV runs around in X and along all the rings
    /// in Y.
    pub fn rings(&mut self, rings: &Rings, range: std::ops::Range<usize>) {
        let last = (rings.len() - 1).max(1) as f32;
        let mut before: Option<Vec<u32>> = None;
        for ring in range {
            let around = rings.points[ring].len();
            let indices: Vec<u32> = (0..=around)
                .map(|index| {
                    let wrapped = index % around;
                    self.vertex_at(
                        rings.points[ring][wrapped],
                        rings.normal(ring, wrapped),
                        Vec2::new(index as f32 / around as f32, ring as f32 / last),
                    )
                })
                .collect();
            if let Some(before) = &before {
                for index in 0..around {
                    self.triangle([before[index], before[index + 1], indices[index + 1]]);
                    self.triangle([before[index], indices[index + 1], indices[index]]);
                }
            }
            before = Some(indices);
        }
    }

    /// A tube along `spine`, `radius(along)` thick from `along` 0 at the
    /// start to 1 at the end, with `sides` around. One piece, so its
    /// shading has no seams where the spine bends.
    pub fn tube(
        &mut self,
        spine: &[Vec3],
        radius: impl Fn(f32) -> f32,
        sides: u32,
        ends: [End; 2],
    ) {
        let count = spine.len();
        if count < 2 {
            return;
        }
        let lengths: Vec<f32> = std::iter::once(0.0)
            .chain(spine.windows(2).scan(0.0, |total, pair| {
                *total += pair[0].distance(pair[1]);
                Some(*total)
            }))
            .collect();
        let total = lengths[count - 1].max(1e-6);
        let tangents: Vec<Vec3> = (0..count)
            .map(|index| {
                let before = spine[index.saturating_sub(1)];
                let after = spine[(index + 1).min(count - 1)];
                (after - before).normalize_or(Vec3::Y)
            })
            .collect();
        // Frames carried along the spine without twisting.
        let mut side = tangents[0].any_orthonormal_vector();
        let mut rings = Vec::with_capacity(count);
        for index in 0..count {
            let tangent = tangents[index];
            side =
                (side - tangent * side.dot(tangent)).normalize_or(tangent.any_orthonormal_vector());
            let up = tangent.cross(side);
            let along = lengths[index] / total;
            let r = radius(along);
            let step = 0.5 / count as f32;
            let slope = (radius((along + step).min(1.0)) - radius((along - step).max(0.0)))
                / (2.0 * step * total);
            let ring: Vec<u32> = (0..sides)
                .map(|around| {
                    let angle = TAU * around as f32 / sides as f32;
                    let out = side * angle.cos() + up * angle.sin();
                    let normal = (out - tangent * slope).normalize();
                    self.vertex_at(
                        spine[index] + out * r,
                        normal,
                        Vec2::new(around as f32 / sides as f32, along),
                    )
                })
                .collect();
            rings.push((ring, side, up, r));
        }
        for pair in rings.windows(2) {
            self.strip(&pair[0].0, &pair[1].0);
        }
        for (end, (ring, side, up, r), center, outward) in [
            (ends[0], &rings[0], spine[0], -tangents[0]),
            (
                ends[1],
                &rings[count - 1],
                spine[count - 1],
                tangents[count - 1],
            ),
        ] {
            self.cap(end, ring, *side, *up, *r, center, outward, sides);
        }
    }

    #[expect(clippy::too_many_arguments, reason = "the parts of a tube's end")]
    fn cap(
        &mut self,
        end: End,
        ring: &[u32],
        side: Vec3,
        up: Vec3,
        radius: f32,
        center: Vec3,
        outward: Vec3,
        sides: u32,
    ) {
        match end {
            End::Open => {}
            End::Flat => {
                let rim: Vec<u32> = ring
                    .iter()
                    .map(|&index| {
                        let position = self.positions[index as usize];
                        self.vertex(position, outward)
                    })
                    .collect();
                let middle = self.vertex(center, outward);
                self.fan(middle, &rim);
            }
            End::Round => {
                let mut last = ring.to_vec();
                for step in 1..ROUND_RINGS {
                    let bend = FRAC_PI_2 * step as f32 / ROUND_RINGS as f32;
                    let next: Vec<u32> = (0..sides)
                        .map(|around| {
                            let angle = TAU * around as f32 / sides as f32;
                            let out = side * angle.cos() + up * angle.sin();
                            let normal = out * bend.cos() + outward * bend.sin();
                            self.vertex(center + normal * radius, normal)
                        })
                        .collect();
                    self.strip(&last, &next);
                    last = next;
                }
                let tip = self.vertex(center + outward * radius, outward);
                self.fan(tip, &last);
            }
        }
    }

    /// A body turned around +Y, through `profile` points of radius and
    /// height. Points with no radius close it.
    pub fn lathe(&mut self, profile: &[Vec2], sides: u32) {
        let count = profile.len();
        let mut last: Option<Vec<u32>> = None;
        for index in 0..count {
            let before = profile[index.saturating_sub(1)];
            let after = profile[(index + 1).min(count - 1)];
            let tangent = (after - before).normalize_or(Vec2::Y);
            // Outward, to the right of the way the profile runs.
            let flat_normal = Vec2::new(tangent.y, -tangent.x);
            let point = profile[index];
            let ring: Vec<u32> = (0..sides)
                .map(|around| {
                    let angle = TAU * around as f32 / sides as f32;
                    let (sin, cos) = angle.sin_cos();
                    let normal = Vec3::new(flat_normal.x * cos, flat_normal.y, flat_normal.x * sin);
                    self.vertex_at(
                        Vec3::new(point.x * cos, point.y, point.x * sin),
                        normal.normalize_or(Vec3::Y),
                        Vec2::new(
                            around as f32 / sides as f32,
                            index as f32 / (count - 1) as f32,
                        ),
                    )
                })
                .collect();
            if let Some(last) = &last {
                self.strip(last, &ring);
            }
            last = Some(ring);
        }
    }

    /// An ellipsoid with `radii` along its axes, placed by `transform`
    /// without scale.
    pub fn ellipsoid(&mut self, radii: Vec3, transform: &Transform, rings: u32, sides: u32) {
        let mark = self.mark();
        let top = self.vertex_at(Vec3::Y * radii.y, Vec3::Y, Vec2::new(0.5, 0.0));
        let mut last: Option<Vec<u32>> = None;
        for ring in 1..rings {
            let polar = PI * ring as f32 / rings as f32;
            let row: Vec<u32> = (0..sides)
                .map(|around| {
                    let angle = TAU * around as f32 / sides as f32;
                    let unit = Vec3::new(
                        polar.sin() * angle.cos(),
                        polar.cos(),
                        polar.sin() * angle.sin(),
                    );
                    self.vertex_at(
                        unit * radii,
                        (unit / radii).normalize(),
                        Vec2::new(around as f32 / sides as f32, ring as f32 / rings as f32),
                    )
                })
                .collect();
            match &last {
                Some(last) => self.strip(last, &row),
                None => self.fan(top, &row),
            }
            last = Some(row);
        }
        let bottom = self.vertex_at(Vec3::NEG_Y * radii.y, Vec3::NEG_Y, Vec2::new(0.5, 1.0));
        if let Some(last) = &last {
            self.fan(bottom, last);
        }
        self.transform_since(mark, transform);
    }
}
