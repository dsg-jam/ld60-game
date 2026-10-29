//! Sheets and cards with real holes, for the pages in the stacks and the
//! divider cards.

use std::f32::consts::TAU;

use bevy::{asset::RenderAssetUsages, mesh::Indices, prelude::*, render::mesh::PrimitiveTopology};

/// Segments around a hole. A multiple of 8, so the square around each hole
/// has a vertex in each corner.
const SEGMENTS: usize = 16;

#[derive(Default)]
struct Builder {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    uvs: Vec<Vec2>,
    indices: Vec<u32>,
}

impl Builder {
    /// The convex `face` on top at `height` and below at `-height`.
    fn cap(&mut self, face: &[Vec2], height: f32, size: Vec2) {
        let mut points = face.to_vec();
        points.dedup_by(|a, b| a.distance(*b) < 1e-7);
        if points.len() < 3 {
            return;
        }
        if signed_area(&points) < 0.0 {
            points.reverse();
        }
        for (y, normal) in [(height, Vec3::Y), (-height, Vec3::NEG_Y)] {
            let start = self.positions.len() as u32;
            for &point in &points {
                self.positions.push(Vec3::new(point.x, y, point.y));
                self.normals.push(normal);
                self.uvs.push(point / size + 0.5);
            }
            for k in 1..points.len() as u32 - 1 {
                // Counterclockwise with +X right and +Z up faces -Y.
                let triangle = if normal.y > 0.0 {
                    [start, start + k + 1, start + k]
                } else {
                    [start, start + k, start + k + 1]
                };
                self.indices.extend(triangle);
            }
        }
    }

    /// An upright wall from `from` to `to`, facing to the right of that way
    /// with +X right and +Z up.
    fn wall(&mut self, from: Vec2, to: Vec2, height: f32) {
        let along = (to - from).normalize_or_zero();
        let normal = Vec3::new(along.y, 0.0, -along.x);
        let start = self.positions.len() as u32;
        for (point, y) in [(from, -height), (to, -height), (to, height), (from, height)] {
            self.positions.push(Vec3::new(point.x, y, point.y));
            self.normals.push(normal);
            self.uvs.push(Vec2::ZERO);
        }
        self.indices
            .extend([start, start + 2, start + 1, start, start + 3, start + 2]);
    }

    fn build(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// A box of `size` lying flat, punched through from top to bottom with
/// `holes` of `radius`.
///
/// The holes are centers in the box's XZ plane. They lie on one line along
/// Z, with their distance to the -X edge the closest they come to anything
/// else.
pub(super) fn punched_box(size: Vec3, holes: &[Vec2], radius: f32) -> Mesh {
    let half = size / 2.0;
    let mut holes = holes.to_vec();
    holes.sort_by(|a, b| a.y.total_cmp(&b.y));
    // Each hole sits in a square cell that reaches the -X edge.
    let reach = holes
        .first()
        .map_or(0.0, |hole| hole.x + half.x)
        .clamp(0.0, half.x);
    let strip = -half.x + 2.0 * reach;
    let at = |hole: Vec2, k: usize, scale: f32| {
        let dir = Vec2::from_angle(TAU * (k % SEGMENTS) as f32 / SEGMENTS as f32);
        hole + dir * scale
    };
    let square = |hole: Vec2, k: usize| {
        let dir = Vec2::from_angle(TAU * (k % SEGMENTS) as f32 / SEGMENTS as f32);
        hole + dir * reach / dir.abs().max_element()
    };
    let quarter = SEGMENTS / 4;
    let mut faces: Vec<Vec<Vec2>> = Vec::new();
    // Below each gap in the strip of cells, from -X to +X, and above it,
    // from +X to -X.
    let mut lower = vec![Vec2::new(-half.x, -half.z), Vec2::new(strip, -half.z)];
    // The +X side of the strip, from +Z to -Z.
    let mut side = vec![Vec2::new(strip, half.z)];
    let mut sides = Vec::new();
    for &hole in &holes {
        for k in 0..SEGMENTS {
            faces.push(vec![
                at(hole, k, radius),
                square(hole, k),
                square(hole, k + 1),
                at(hole, k + 1, radius),
            ]);
        }
        let upper: Vec<Vec2> = (0..=quarter)
            .map(|k| square(hole, 3 * quarter - quarter / 2 + quarter - k))
            .collect();
        faces.push(lower.iter().chain(&upper).copied().collect());
        lower = (0..=quarter)
            .map(|k| square(hole, quarter + quarter / 2 - k))
            .collect();
        sides.push((0..=quarter).map(move |k| square(hole, quarter / 2 + SEGMENTS - k)));
    }
    faces.push(
        lower
            .into_iter()
            .chain([Vec2::new(strip, half.z), Vec2::new(-half.x, half.z)])
            .collect(),
    );
    for points in sides.into_iter().rev() {
        side.extend(points);
    }
    side.push(Vec2::new(strip, -half.z));
    if strip < half.x {
        faces.push(
            [Vec2::new(half.x, -half.z), Vec2::new(half.x, half.z)]
                .into_iter()
                .chain(side)
                .collect(),
        );
    }

    let mut mesh = Builder::default();
    for face in &faces {
        mesh.cap(face, half.y, size.xz());
    }
    let corners = [
        Vec2::new(-half.x, -half.z),
        Vec2::new(half.x, -half.z),
        Vec2::new(half.x, half.z),
        Vec2::new(-half.x, half.z),
    ];
    for (index, &from) in corners.iter().enumerate() {
        mesh.wall(from, corners[(index + 1) % 4], half.y);
    }
    for &hole in &holes {
        for k in 0..SEGMENTS {
            mesh.wall(at(hole, k + 1, radius), at(hole, k, radius), half.y);
        }
    }
    mesh.build()
}

/// Twice the area of `points` in XZ, positive when they go counterclockwise
/// seen with +Z up and +X right.
fn signed_area(points: &[Vec2]) -> f32 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| a.perp_dot(*b))
        .sum()
}
