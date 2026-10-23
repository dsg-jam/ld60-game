mod light;

use bevy::{
    asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path},
    mesh::{Indices, MeshBuilder, PrimitiveTopology},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

pub use self::light::CrtLight;
use super::UiCanvas;

const CORNER_RADIUS: f32 = 0.06;
const BRIGHTNESS: f32 = 1.1;
/// Canvas pixels per scanline.
const SCANLINE_PITCH: f32 = 4.0;
/// How dark the gaps between scanlines get, from 0 to 1.
const SCANLINE_DARKNESS: f32 = 0.55;
/// The phosphor glows faintly even where the picture is black.
const BLACK_LEVEL: Vec3 = Vec3::new(0.02, 0.045, 0.03);

pub(super) struct CrtPlugin;

impl Plugin for CrtPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "crt.wesl");
        app.add_plugins((
            MaterialPlugin::<CrtMaterial>::default(),
            light::CrtLightPlugin,
        ));
    }
}

/// Glass of a CRT monitor that shows a [`UiCanvas`].
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CrtMaterial {
    #[texture(0)]
    #[sampler(1)]
    screen: Handle<Image>,
    #[uniform(2)]
    params: CrtParams,
}

/// Matches `Crt` in `crt.wesl`.
#[derive(ShaderType, Debug, Clone)]
struct CrtParams {
    /// In UV units.
    corner_radius: f32,
    scanlines: f32,
    brightness: f32,
    scanline_darkness: f32,
    black_level: Vec3,
}

impl CrtMaterial {
    pub fn new(canvas: &UiCanvas) -> Self {
        Self {
            screen: canvas.image.clone(),
            params: CrtParams {
                corner_radius: CORNER_RADIUS,
                scanlines: canvas.size.y as f32 / SCANLINE_PITCH,
                brightness: BRIGHTNESS,
                scanline_darkness: SCANLINE_DARKNESS,
                black_level: BLACK_LEVEL,
            },
        }
    }
}

impl Material for CrtMaterial {
    fn fragment_shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("crt.wesl"))
            .with_source("embedded")
            .into()
    }
}

/// Slightly bulging CRT glass.
///
/// UVs span the whole surface with (0, 0) in the top left corner.
#[derive(Clone, Copy, Debug)]
pub struct CrtGlass {
    pub size: Vec2,
    pub bulge: f32,
}

impl MeshBuilder for CrtGlass {
    fn build(&self) -> Mesh {
        const RESOLUTION: u32 = 16;
        let half = self.size / 2.0;
        let vertices = ((RESOLUTION + 1) * (RESOLUTION + 1)) as usize;
        let mut positions = Vec::with_capacity(vertices);
        let mut normals = Vec::with_capacity(vertices);
        let mut uvs = Vec::with_capacity(vertices);
        for row in 0..=RESOLUTION {
            for column in 0..=RESOLUTION {
                let uv = Vec2::new(column as f32, row as f32) / RESOLUTION as f32;
                let centered = Vec2::new(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
                let falloff = (1.0 - centered.x * centered.x) * (1.0 - centered.y * centered.y);
                let slope = Vec2::new(
                    -2.0 * centered.x * (1.0 - centered.y * centered.y) / half.x,
                    -2.0 * centered.y * (1.0 - centered.x * centered.x) / half.y,
                ) * self.bulge;
                positions.push([
                    centered.x * half.x,
                    centered.y * half.y,
                    self.bulge * falloff,
                ]);
                normals.push(Vec3::new(-slope.x, -slope.y, 1.0).normalize().to_array());
                uvs.push(uv.to_array());
            }
        }
        let stride = RESOLUTION + 1;
        let mut indices = Vec::new();
        for row in 0..RESOLUTION {
            for column in 0..RESOLUTION {
                let i = row * stride + column;
                indices.extend([i, i + stride, i + 1, i + 1, i + stride, i + stride + 1]);
            }
        }
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
    }
}
