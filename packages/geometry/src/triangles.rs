use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

/// Collects the triangles of a mesh, each facing the way it is told.
///
/// It has no UVs to speak of, they are all zero.
#[derive(Default)]
pub struct Triangles {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    indices: Vec<u32>,
}

impl Triangles {
    pub fn vertex(&mut self, position: Vec3, normal: Vec3) -> u32 {
        self.positions.push(position);
        self.normals.push(normal);
        (self.positions.len() - 1) as u32
    }

    /// A triangle of shared vertices, facing along their normals.
    pub fn triangle(&mut self, [a, b, c]: [u32; 3]) {
        let [pa, pb, pc] = [a, b, c].map(|index| self.positions[index as usize]);
        let normal: Vec3 = [a, b, c]
            .map(|index| self.normals[index as usize])
            .iter()
            .sum();
        let facing = (pb - pa).cross(pc - pa).dot(normal) >= 0.0;
        self.indices
            .extend(if facing { [a, b, c] } else { [a, c, b] });
    }

    /// A triangle of its own, lit flat and facing about `towards`.
    pub fn flat(&mut self, [a, b, c]: [Vec3; 3], towards: Vec3) {
        let mut normal = (b - a).cross(c - a).normalize_or_zero();
        if normal.dot(towards) < 0.0 {
            normal = -normal;
        }
        let indices = [a, b, c].map(|point| self.vertex(point, normal));
        self.triangle(indices);
    }

    /// Quads between two loops of the same length.
    pub fn strip(&mut self, near: &[u32], far: &[u32]) {
        for index in 0..near.len() {
            let next = (index + 1) % near.len();
            self.triangle([near[index], near[next], far[next]]);
            self.triangle([near[index], far[next], far[index]]);
        }
    }

    /// Triangles from `middle` to each side of a loop.
    pub fn fan(&mut self, middle: u32, ring: &[u32]) {
        for index in 0..ring.len() {
            self.triangle([middle, ring[index], ring[(index + 1) % ring.len()]]);
        }
    }

    pub fn positions(&self) -> &[Vec3] {
        &self.positions
    }

    pub fn build(self) -> Mesh {
        let count = self.positions.len();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![Vec2::ZERO; count])
        .with_inserted_indices(Indices::U32(self.indices))
    }
}
