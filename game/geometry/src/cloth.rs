//! Cloth covered boards, like the covers of a file, and the strips where
//! they bend into each other.

use std::f32::consts::{FRAC_PI_2, TAU};

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

/// How a sheet of cloth falls over a box, see [`drape`].
#[derive(Clone, Copy, Debug)]
pub struct Drape {
    /// The top of the box in x and z.
    pub top: Rect,
    /// The height of the top above the ground.
    pub height: f32,
    /// How far the cloth lies on the ground past the foot of each side.
    /// Below zero, the hem hangs this far above the ground.
    pub pool: f32,
    /// Radius the cloth rolls over the edges of the top with.
    pub roll: f32,
    /// Depth and length of the folds down the sides.
    pub folds: Vec2,
    /// Grid cells along each side of the sheet.
    pub cells: u32,
}

impl Drape {
    /// Where the point `flat` of the sheet, laid out flat over the top,
    /// ends up.
    fn fall(&self, flat: Vec2) -> Vec3 {
        let edge = flat.clamp(self.top.min, self.top.max);
        let off = flat - edge;
        let past = off.length();
        if past == 0.0 {
            return edge.extend(self.height).xzy();
        }
        let out = off / past;
        let quarter = FRAC_PI_2 * self.roll;
        if past < quarter {
            let angle = past / self.roll;
            let at = edge + out * self.roll * angle.sin();
            return Vec3::new(at.x, self.height - self.roll * (1.0 - angle.cos()), at.y);
        }
        let down = past - quarter;
        // Folds run down the side and grow towards the ground. They only
        // bulge out, so the cloth never dips into the box.
        let along = flat.dot(out.perp());
        let deep = (down / self.height).clamp(0.0, 1.0);
        let wave = |length: f32, phase: f32| 0.5 + 0.5 * (TAU * along / length + phase).sin();
        let fold = self.folds.x
            * deep
            * (0.7 * wave(self.folds.y, 0.0) + 0.3 * wave(0.37 * self.folds.y, 1.3));
        let mut at = edge + out * (self.roll + fold);
        let mut y = self.height - self.roll - down;
        if y < 0.0 {
            at += out * -y;
            y = 0.0;
        }
        Vec3::new(at.x, y, at.y)
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

/// A square sheet of cloth thrown over a box standing on the ground at
/// y = 0. It lies flat on the top, rolls over the edges, hangs down the
/// sides in folds that deepen towards the ground, and lies on the ground
/// around the foot. The UVs are the flat sheet in meters.
pub fn drape(drape: &Drape) -> Mesh {
    let reach = drape.height + drape.pool;
    let sheet = Rect::from_corners(
        drape.top.min - Vec2::splat(reach),
        drape.top.max + Vec2::splat(reach),
    );
    let cells = drape.cells;
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    for row in 0..=cells {
        for column in 0..=cells {
            let flat = sheet.min + sheet.size() * UVec2::new(column, row).as_vec2() / cells as f32;
            positions.push(drape.fall(flat));
            uvs.push(flat - sheet.min);
        }
    }
    let mut indices = Vec::new();
    let stride = cells + 1;
    for row in 0..cells {
        for column in 0..cells {
            let corner = row * stride + column;
            indices.extend([corner, corner + stride, corner + 1]);
            indices.extend([corner + 1, corner + stride, corner + stride + 1]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
    .with_computed_smooth_normals()
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
