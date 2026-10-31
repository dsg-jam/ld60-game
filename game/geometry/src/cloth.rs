//! Cloth covered boards, like the covers of a file, and the strips where
//! they bend into each other.

use std::f32::consts::FRAC_PI_2;

use bevy::{asset::RenderAssetUsages, mesh::Indices, prelude::*, render::mesh::PrimitiveTopology};

/// Segments of a rounded corner or a bent strip.
const SEGMENTS: usize = 6;

#[derive(Default)]
struct Builder {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    indices: Vec<u32>,
}

impl Builder {
    fn vertex(&mut self, position: Vec3, normal: Vec3) -> u32 {
        self.positions.push(position);
        self.normals.push(normal);
        self.positions.len() as u32 - 1
    }

    fn build(self) -> Mesh {
        let uvs = vec![Vec2::ZERO; self.positions.len()];
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// A board `size.x` wide from its bound edge at x = 0, `size.y` thick
/// from y = 0 and `size.z` long around z = 0. The corners of its free edge
/// are rounded by `corner`.
pub fn board(size: Vec3, corner: f32) -> Mesh {
    let half = size.z / 2.0;
    // Around the outline counterclockwise in x and z, each point with the
    // normal of the edge there. The bound corners are sharp, so their
    // points come twice.
    let mut outline = vec![
        (Vec2::new(0.0, half), Vec2::NEG_X),
        (Vec2::new(0.0, -half), Vec2::NEG_X),
        (Vec2::new(0.0, -half), Vec2::NEG_Y),
    ];
    for (center, start) in [
        (Vec2::new(size.x - corner, -half + corner), -FRAC_PI_2),
        (Vec2::new(size.x - corner, half - corner), 0.0),
    ] {
        for step in 0..=SEGMENTS {
            let normal = Vec2::from_angle(start + FRAC_PI_2 * step as f32 / SEGMENTS as f32);
            outline.push((center + normal * corner, normal));
        }
    }
    outline.push((Vec2::new(0.0, half), Vec2::Y));
    let mut builder = Builder::default();
    for (y, normal) in [(size.y, Vec3::Y), (0.0, Vec3::NEG_Y)] {
        let start = builder.positions.len() as u32;
        for &(point, _) in &outline {
            builder.vertex(Vec3::new(point.x, y, point.y), normal);
        }
        for k in 1..outline.len() as u32 - 1 {
            // Counterclockwise in x and z faces down.
            builder.indices.extend(if normal.y > 0.0 {
                [start, start + k + 1, start + k]
            } else {
                [start, start + k, start + k + 1]
            });
        }
    }
    let start = builder.positions.len() as u32;
    for &(point, normal) in &outline {
        let normal = Vec3::new(normal.x, 0.0, normal.y);
        builder.vertex(Vec3::new(point.x, 0.0, point.y), normal);
        builder.vertex(Vec3::new(point.x, size.y, point.y), normal);
    }
    for k in 0..outline.len() as u32 - 1 {
        let (low, high) = (start + 2 * k, start + 2 * k + 1);
        builder
            .indices
            .extend([low, high, low + 2, low + 2, high, high + 2]);
    }
    builder.build()
}

/// The cloth bent around a hinge on the z axis, filling the gap between
/// two boards that meet there at an angle. It reaches `radius` out from
/// the axis between the angles `from` and `to`, counterclockwise in x and
/// y, and runs `length` along z. `to` is past `from`.
pub fn hinge(from: f32, to: f32, radius: f32, length: f32) -> Mesh {
    let mut builder = Builder::default();
    let half = length / 2.0;
    let around: Vec<Vec2> = (0..=SEGMENTS)
        .map(|step| Vec2::from_angle(from + (to - from) * step as f32 / SEGMENTS as f32))
        .collect();
    let start = builder.positions.len() as u32;
    for &out in &around {
        let normal = out.extend(0.0);
        builder.vertex((out * radius).extend(-half), normal);
        builder.vertex((out * radius).extend(half), normal);
    }
    for k in 0..SEGMENTS as u32 {
        let (low, high) = (start + 2 * k, start + 2 * k + 1);
        builder
            .indices
            .extend([low, low + 2, high, low + 2, high + 2, high]);
    }
    for (z, normal) in [(half, Vec3::Z), (-half, Vec3::NEG_Z)] {
        let center = builder.vertex(Vec3::new(0.0, 0.0, z), normal);
        let rim = builder.positions.len() as u32;
        for &out in &around {
            builder.vertex((out * radius).extend(z), normal);
        }
        for k in 0..SEGMENTS as u32 {
            builder.indices.extend(if z > 0.0 {
                [center, rim + k, rim + k + 1]
            } else {
                [center, rim + k + 1, rim + k]
            });
        }
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use bevy::mesh::VertexAttributeValues;

    use super::*;

    /// Whether every triangle of `mesh` faces the way its vertex normals
    /// point.
    fn faces_out(mesh: &Mesh) -> bool {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            return false;
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            return false;
        };
        let Some(Indices::U32(indices)) = mesh.indices() else {
            return false;
        };
        indices.chunks(3).all(|triangle| {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(positions[triangle[i] as usize]));
            let face = (b - a).cross(c - a);
            let normal: Vec3 = triangle
                .iter()
                .map(|&i| Vec3::from(normals[i as usize]))
                .sum();
            face.length() < 1e-12 || face.dot(normal) > 0.0
        })
    }

    #[test]
    fn boards_and_hinges_face_out() {
        assert!(faces_out(&board(Vec3::new(0.2, 0.004, 0.24), 0.006)));
        assert!(faces_out(&hinge(-FRAC_PI_2, 0.3, 0.004, 0.24)));
        assert!(faces_out(&hinge(-2.0, -FRAC_PI_2, 0.004, 0.24)));
    }
}
