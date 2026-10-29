use bevy::{
    asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path},
    material::OpaqueRendererMethod,
    mesh::{Indices, PrimitiveTopology},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

use super::POINTS;

/// Rings of the tube from one point of the cable to the next.
const RINGS: usize = 6;
/// Vertices around each ring.
const SIDES: usize = 16;

pub(super) struct RopePlugin;

impl Plugin for RopePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "rope.wesl");
        embedded_asset!(app, "braid.wesl");
        app.add_plugins(MaterialPlugin::<RopeMaterial>::default());
    }
}

pub(super) type RopeMaterial = ExtendedMaterial<StandardMaterial, Rope>;

/// Bends the tube of [`Rope::mesh`] through the points of the cable in the
/// vertex shader, so the cable is one smooth surface, and covers it in a
/// braided sleeve.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(super) struct Rope {
    #[uniform(100)]
    params: RopeParams,
}

/// Matches `Rope` in `rope.wesl`.
#[derive(ShaderType, Reflect, Debug, Clone)]
struct RopeParams {
    /// In world space.
    points: [Vec4; POINTS],
    /// A normal at each point that turns along with the cable, so the tube
    /// does not twist.
    normals: [Vec4; POINTS],
    radius: f32,
    /// The length of the cable from one point to the next.
    spacing: f32,
}

impl Rope {
    pub(super) fn material(
        color: Color,
        radius: f32,
        points: &[Vec3],
        length: f32,
    ) -> RopeMaterial {
        let mut rope = Self {
            params: RopeParams {
                points: [Vec4::ZERO; POINTS],
                normals: [Vec4::ZERO; POINTS],
                radius,
                spacing: 0.0,
            },
        };
        rope.follow(points, length);
        ExtendedMaterial {
            base: StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.5,
                reflectance: 0.35,
                anisotropy_strength: 1.0,
                // The deferred G-buffer has no room for anisotropy.
                opaque_render_method: OpaqueRendererMethod::Forward,
                ..default()
            },
            extension: rope,
        }
    }

    /// A tube to bend with this material. The x of its UVs counts the points
    /// along the cable, and the y goes once around it. Each ring starts and
    /// ends on the same spot, so the y runs from 0 to 1 without a jump.
    pub(super) fn mesh() -> Mesh {
        let rings = (POINTS - 1) * RINGS + 1;
        let row = SIDES + 1;
        let uvs: Vec<[f32; 2]> = (0..rings)
            .flat_map(|ring| {
                (0..row).map(move |side| [ring as f32 / RINGS as f32, side as f32 / SIDES as f32])
            })
            .collect();
        let mut indices = Vec::with_capacity((rings - 1) * SIDES * 6);
        for ring in 0..rings - 1 {
            for side in 0..SIDES {
                let [a, b] = [ring * row + side, ring * row + side + 1];
                let [c, d] = [a + row, b + row];
                indices.extend([a, b, c, b, d, c].map(|index| index as u32));
            }
        }
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0; 3]; uvs.len()])
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; uvs.len()])
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_TANGENT,
            vec![[1.0, 0.0, 0.0, 1.0]; uvs.len()],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
    }

    /// Moves the tube onto `points` of a cable `length` long.
    pub(super) fn follow(&mut self, points: &[Vec3], length: f32) {
        self.params.spacing = length / (POINTS - 1) as f32;
        let mut last: Option<(Vec3, Vec3)> = None;
        for index in 0..POINTS {
            let tangent = (points[(index + 1).min(POINTS - 1)] - points[index.saturating_sub(1)])
                .normalize_or(Vec3::X);
            let normal = match last {
                Some((before, normal)) => Quat::from_rotation_arc(before, tangent) * normal,
                None => tangent.any_orthonormal_vector(),
            };
            self.params.points[index] = points[index].extend(1.0);
            self.params.normals[index] = normal.extend(0.0);
            last = Some((tangent, normal));
        }
    }

    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("rope.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl MaterialExtension for Rope {
    fn fragment_shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("braid.wesl"))
            .with_source("embedded")
            .into()
    }

    fn vertex_shader() -> ShaderRef {
        Self::shader()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        Self::shader()
    }

    fn deferred_vertex_shader() -> ShaderRef {
        Self::shader()
    }
}
