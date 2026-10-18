use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

/// Many small quads that a vertex shader moves into place, like rain
/// streaks or droplets.
///
/// All corners of a quad share its point and data, and tell themselves apart
/// by their UV, from (0, 0) to (1, 1). The normal and the color carry any
/// data the shader needs.
#[derive(Debug, Default)]
pub(crate) struct Quads {
    points: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl Quads {
    pub(crate) fn push(&mut self, point: Vec3) {
        self.push_with(point, [0.0; 3], [0.0; 4]);
    }

    /// Adds a quad with `normal` and `color` for the shader.
    pub(crate) fn push_with(&mut self, point: Vec3, normal: [f32; 3], color: [f32; 4]) {
        let base = self.points.len() as u32;
        self.points.extend([point.to_array(); 4]);
        self.normals.extend([normal; 4]);
        self.colors.extend([color; 4]);
        self.uvs
            .extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        self.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub(crate) fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.points)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}
