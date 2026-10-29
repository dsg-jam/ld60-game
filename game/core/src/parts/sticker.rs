use std::ops::Range;

use bevy::{
    asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path},
    mesh::{Indices, PrimitiveTopology},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use rand::{RngExt, SeedableRng, rngs::SmallRng};

const POINTS_PER_SIDE: usize = 24;
/// How deep the edges are torn in, in meters.
const RAGGED: f32 = 0.0012;
/// The corner that got caught and torn off.
const CORNER_TEAR: Range<f32> = 0.15..0.3;
const ROUGHNESS: f32 = 0.5;
/// The shader gets one of this many seeds.
const SEEDS: u64 = 1000;

pub struct StickerPlugin;

impl Plugin for StickerPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "sticker.wesl");
        app.add_plugins(MaterialPlugin::<StickerMaterial>::default());
    }
}

pub type StickerMaterial = ExtendedMaterial<StandardMaterial, Sticker>;

/// A printed paper sticker that has seen some use: torn edges, dirt along
/// them, stains, smudges and scratches through the print.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct Sticker {
    #[uniform(100)]
    params: StickerParams,
}

/// Matches `Sticker` in `sticker.wesl`.
#[derive(ShaderType, Reflect, Debug, Clone)]
struct StickerParams {
    size: Vec2,
    seed: f32,
    /// How used it looks, from 0 to 1.
    wear: f32,
}

impl Sticker {
    /// A sticker of `size` showing `print`. The `seed` decides where it is
    /// torn and stained.
    pub fn material(print: Handle<Image>, size: Vec2, seed: u64, wear: f32) -> StickerMaterial {
        ExtendedMaterial {
            base: StandardMaterial {
                base_color_texture: Some(print),
                perceptual_roughness: ROUGHNESS,
                ..default()
            },
            extension: Self {
                params: StickerParams {
                    size,
                    seed: (seed % SEEDS) as f32,
                    wear,
                },
            },
        }
    }

    /// A seed for a sticker printed with `serial`, so each unit is worn in
    /// its own way.
    pub fn seed(serial: &str) -> u64 {
        serial.bytes().fold(0, |hash, byte| {
            hash.wrapping_mul(31).wrapping_add(u64::from(byte))
        })
    }

    /// The outline of a sticker of `size`, facing +Z like a [`Rectangle`],
    /// with ragged edges and one corner torn off.
    pub fn mesh(size: Vec2, seed: u64) -> Mesh {
        let mut rng = SmallRng::seed_from_u64(seed);
        let half = size / 2.0;
        let corners = [
            Vec2::new(-half.x, -half.y),
            Vec2::new(half.x, -half.y),
            Vec2::new(half.x, half.y),
            Vec2::new(-half.x, half.y),
        ];
        let torn = corners[rng.random_range(0..4)];
        // Towards the middle, away from the torn corner.
        let towards = (-torn).normalize();
        let tear = size.min_element() * rng.random_range(CORNER_TEAR);
        let mut outline = Vec::new();
        let mut depth: f32 = 0.0;
        for (side, &from) in corners.iter().enumerate() {
            let to = corners[(side + 1) % 4];
            let inward = (-(from + to) / 2.0).normalize();
            for step in 0..POINTS_PER_SIDE {
                let t = step as f32 / POINTS_PER_SIDE as f32;
                let mut point = from.lerp(to, t);
                // A random walk, so the tear wanders in and out.
                depth = (depth + rng.random_range(-0.4..0.4) * RAGGED).clamp(0.0, RAGGED);
                point += inward * depth;
                let off = (point - torn).abs();
                if off.x + off.y < tear {
                    // Cut the corner along a jagged diagonal.
                    point += towards * (tear - off.x - off.y) * 0.7
                        + towards.perp() * rng.random_range(-0.3..0.3) * RAGGED;
                }
                outline.push(point.clamp(-half, half));
            }
        }

        let mut positions = vec![Vec3::ZERO];
        let mut uvs = vec![Vec2::splat(0.5)];
        for point in &outline {
            positions.push(point.extend(0.0));
            uvs.push(Vec2::new(point.x / size.x + 0.5, 0.5 - point.y / size.y));
        }
        let count = outline.len() as u32;
        let indices = (0..count)
            .flat_map(|index| [0, 1 + index, 1 + (index + 1) % count])
            .collect();
        let normals = vec![Vec3::Z; positions.len()];
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
    }

    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("sticker.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl MaterialExtension for Sticker {
    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        Self::shader()
    }
}
