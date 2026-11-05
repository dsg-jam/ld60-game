mod solids;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

pub use self::solids::{End, Rings};

/// Collects the triangles of a mesh, each facing the way it is told.
///
/// Vertices have a UV of zero unless they are given one. Vertices take the
/// color and the [`Skin`] set last, if any.
///
/// ```
/// # use bevy::prelude::*;
/// # use ld60_geometry::triangles::{End, Triangles};
/// let mut mesh = Triangles::default();
/// let leg = [Vec3::ZERO, Vec3::new(0.0, -0.3, 0.05), Vec3::new(0.0, -0.6, 0.0)];
/// mesh.tube(&leg, |along| 0.04 - 0.01 * along, 8, [End::Round, End::Round]);
/// let mesh = mesh.build();
/// ```
#[derive(Default)]
pub struct Triangles {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    uvs: Vec<Vec2>,
    indices: Vec<u32>,
    colors: Option<Vec<[f32; 4]>>,
    color: Option<[f32; 4]>,
    skins: Option<Vec<Skin>>,
    skin: Option<Skin>,
}

/// The joints that move a vertex of a skinned mesh, and how much each one
/// does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Skin {
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

impl Skin {
    /// Moved by `joint` alone.
    pub fn one(joint: u16) -> Self {
        Self {
            joints: [joint, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        }
    }

    /// Moved by `from` and `to`, by `to` as far as `t` from 0 to 1.
    pub fn between(from: u16, to: u16, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            joints: [from, to, 0, 0],
            weights: [1.0 - t, t, 0.0, 0.0],
        }
    }
}

impl Triangles {
    pub fn vertex(&mut self, position: Vec3, normal: Vec3) -> u32 {
        self.vertex_at(position, normal, Vec2::ZERO)
    }

    pub fn vertex_at(&mut self, position: Vec3, normal: Vec3, uv: Vec2) -> u32 {
        let index = self.positions.len();
        self.positions.push(position);
        self.normals.push(normal);
        self.uvs.push(uv);
        if let Some(color) = self.color {
            self.colors
                .get_or_insert_with(|| vec![[1.0; 4]; index])
                .push(color);
        } else if let Some(colors) = &mut self.colors {
            colors.push([1.0; 4]);
        }
        if let Some(skin) = self.skin {
            self.skins
                .get_or_insert_with(|| vec![Skin::one(0); index])
                .push(skin);
        } else if let Some(skins) = &mut self.skins {
            skins.push(Skin::one(0));
        }
        index as u32
    }

    /// Gives the vertices from now on `color`.
    pub fn set_color(&mut self, color: [f32; 4]) {
        self.color = Some(color);
    }

    /// Binds the vertices from now on with `skin`.
    pub fn set_skin(&mut self, skin: Skin) {
        self.skin = Some(skin);
    }

    /// How many vertices there are, to come back to the ones added after.
    pub fn mark(&self) -> usize {
        self.positions.len()
    }

    /// Moves the vertices added since `mark` by `transform`.
    pub fn transform_since(&mut self, mark: usize, transform: &Transform) {
        let normals = transform.compute_affine().matrix3.inverse().transpose();
        for position in &mut self.positions[mark..] {
            *position = transform.transform_point(*position);
        }
        for normal in &mut self.normals[mark..] {
            *normal = (normals * *normal).normalize_or_zero();
        }
    }

    /// Colors the vertices added since `mark` by where they are and which
    /// way they face.
    pub fn color_since(&mut self, mark: usize, color: impl Fn(Vec3, Vec3) -> [f32; 4]) {
        let count = self.positions.len();
        let colors = self.colors.get_or_insert_with(|| vec![[1.0; 4]; count]);
        for ((color_of, position), normal) in colors[mark..]
            .iter_mut()
            .zip(&self.positions[mark..])
            .zip(&self.normals[mark..])
        {
            *color_of = color(*position, *normal);
        }
    }

    /// Moves the vertices added since `mark` to where `to` puts them, from
    /// where they are and which way they face. Their normals stay.
    pub fn displace_since(&mut self, mark: usize, to: impl Fn(Vec3, Vec3) -> Vec3) {
        for (position, normal) in self.positions[mark..].iter_mut().zip(&self.normals[mark..]) {
            *position = to(*position, *normal);
        }
    }

    /// Binds the vertices added since `mark` by where they are.
    pub fn bind_since(&mut self, mark: usize, skin: impl Fn(Vec3) -> Skin) {
        let count = self.positions.len();
        let skins = self.skins.get_or_insert_with(|| vec![Skin::one(0); count]);
        for (skin_of, position) in skins[mark..].iter_mut().zip(&self.positions[mark..]) {
            *skin_of = skin(*position);
        }
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
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_indices(Indices::U32(self.indices));
        if let Some(colors) = self.colors {
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        }
        if let Some(skins) = self.skins {
            let (joints, weights): (Vec<_>, Vec<_>) = skins
                .into_iter()
                .map(|skin| (skin.joints, skin.weights))
                .unzip();
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_JOINT_INDEX,
                bevy::mesh::VertexAttributeValues::Uint16x4(joints),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
        }
        mesh
    }
}
