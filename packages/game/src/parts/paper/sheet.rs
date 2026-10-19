use std::f32::consts::{PI, TAU};

use bevy::{
    asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path},
    mesh::{Indices, PrimitiveTopology},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use rand::{RngExt, SeedableRng, rngs::SmallRng};

use super::{PAPER_REFLECTANCE, PAPER_SHADE};

/// About one vertex per this many meters. Wrinkles are broad, so this is
/// enough. The shader draws the sharp fold lines.
const GRID: f32 = 0.01;
const ROUGHNESS: f32 = 0.95;
/// The shader gets one of this many seeds.
const SEEDS: u64 = 1000;
/// Width of the bulge along a fold, in meters.
const FOLD_WIDTH: f32 = 0.02;

pub(super) struct SheetPlugin;

impl Plugin for SheetPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "sheet.wesl");
        app.add_plugins(MaterialPlugin::<PaperMaterial>::default())
            .add_observer(Sheet::add_material);
    }
}

pub(crate) type PaperMaterial = ExtendedMaterial<StandardMaterial, Sheet>;

/// A handled sheet of paper: wrinkled, folded, with grime, a coffee ring and
/// fingerprints. Each seed gives a different sheet.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(crate) struct Sheet {
    #[uniform(100)]
    params: SheetParams,
    #[texture(101)]
    #[sampler(102)]
    print: Handle<Image>,
}

/// Matches `Sheet` in `sheet.wesl`.
#[derive(ShaderType, Reflect, Debug, Clone, Copy)]
struct SheetParams {
    size: Vec2,
    /// Picks the stains.
    seed: f32,
    /// Picks the stains for each page, on top of `seed`.
    page: f32,
    /// Two folds as a point on the line, its angle and its depth, in the
    /// sheet's space: meters from the top left corner, with y down.
    folds: [Vec4; 2],
}

/// Gives a sheet its paper material once spawned.
#[derive(Component, Debug)]
pub(super) struct SheetPrint {
    pub(super) print: Handle<Image>,
    pub(super) wrinkles: Wrinkles,
}

/// The shape of a crumpled sheet that was flattened again.
#[derive(Debug, Clone, Copy)]
pub(super) struct Wrinkles {
    size: Vec2,
    seed: u64,
    folds: [Vec4; 2],
    /// Direction and length in waves per meter, phase and height.
    waves: [(Vec2, f32, f32); 4],
    /// Corner, reach and lift.
    curls: [(Vec2, f32, f32); 2],
}

impl Wrinkles {
    pub(super) fn new(size: Vec2, seed: u64) -> Self {
        let mut rng = SmallRng::seed_from_u64(seed);
        let fold = |rng: &mut SmallRng| {
            if rng.random_bool(0.35) {
                return Vec4::ZERO;
            }
            // Mostly folded in thirds or half, across or along the page.
            let across = rng.random_bool(0.7);
            let at =
                [1.0 / 3.0, 0.5, 2.0 / 3.0][rng.random_range(0..3)] + rng.random_range(-0.03..0.03);
            let point = if across {
                Vec2::new(0.5, at)
            } else {
                Vec2::new(at, 0.5)
            } * size;
            let angle = if across { 0.0 } else { PI / 2.0 } + rng.random_range(-0.04..0.04);
            let depth =
                rng.random_range(0.0008..0.0018) * if rng.random_bool(0.5) { 1.0 } else { -1.0 };
            point.extend(angle).extend(depth)
        };
        let folds = [fold(&mut rng), fold(&mut rng)];
        let waves = std::array::from_fn(|index| {
            let angle = rng.random_range(0.0..TAU);
            let length = if index < 2 {
                rng.random_range(0.08..0.16)
            } else {
                rng.random_range(0.025..0.045)
            };
            let height = if index < 2 { 0.0012 } else { 0.0003 };
            (
                Vec2::from_angle(angle) * TAU / length,
                rng.random_range(0.0..TAU),
                height * rng.random_range(0.5..1.0),
            )
        });
        let curls = std::array::from_fn(|_| {
            let corner = Vec2::new(
                if rng.random_bool(0.5) { 0.0 } else { size.x },
                if rng.random_bool(0.6) { size.y } else { 0.0 },
            );
            let lift = if rng.random_bool(0.6) {
                rng.random_range(0.002..0.005)
            } else {
                0.0
            };
            (corner, rng.random_range(0.03..0.06), lift)
        });
        Self {
            size,
            seed,
            folds,
            waves,
            curls,
        }
    }

    /// Height above the flat sheet at `p` in the sheet's space, before it is
    /// lifted to rest on its lowest point.
    fn height(&self, p: Vec2) -> f32 {
        let mut height = 0.0;
        for fold in self.folds {
            let normal = Vec2::from_angle(fold.z).perp();
            let distance = (p - fold.xy()).dot(normal).abs();
            height += fold.w * (-distance / FOLD_WIDTH).exp();
        }
        for (wave, phase, amplitude) in self.waves {
            height += amplitude * (wave.dot(p) + phase).sin();
        }
        for (corner, reach, lift) in self.curls {
            let near = (1.0 - p.distance(corner) / reach).max(0.0);
            height += lift * near * near;
        }
        height
    }

    /// A grid facing +Z like a [`Rectangle`], displaced by the wrinkles.
    pub(super) fn mesh(&self) -> Mesh {
        let size = self.size;
        let cells = (size / GRID).ceil().as_uvec2().max(UVec2::splat(4));
        let mut heights = Vec::new();
        let mut uvs = Vec::new();
        for y in 0..=cells.y {
            for x in 0..=cells.x {
                let uv = UVec2::new(x, y).as_vec2() / cells.as_vec2();
                heights.push(self.height(uv * size));
                uvs.push(uv);
            }
        }
        let lowest = heights.iter().copied().fold(f32::INFINITY, f32::min);
        let positions: Vec<Vec3> = uvs
            .iter()
            .zip(&heights)
            .map(|(uv, height)| ((*uv - 0.5) * Vec2::new(size.x, -size.y)).extend(height - lowest))
            .collect();
        let step = 0.001;
        let normals: Vec<Vec3> = uvs
            .iter()
            .map(|uv| {
                let p = *uv * size;
                let dx = self.height(p + Vec2::X * step) - self.height(p - Vec2::X * step);
                let dy = self.height(p + Vec2::Y * step) - self.height(p - Vec2::Y * step);
                // The sheet's y runs down the page, the mesh's y up.
                Vec3::new(-dx, dy, 2.0 * step).normalize()
            })
            .collect();
        let row = cells.x + 1;
        let indices = (0..cells.y)
            .flat_map(|y| (0..cells.x).map(move |x| y * row + x))
            .flat_map(|corner| {
                let below = corner + row;
                [corner, below, corner + 1, corner + 1, below, below + 1]
            })
            .collect();
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

impl Sheet {
    fn add_material(
        add: On<Add<SheetPrint>>,
        sheets: Query<&SheetPrint>,
        mut materials: ResMut<Assets<PaperMaterial>>,
        mut commands: Commands,
    ) {
        let Ok(sheet) = sheets.get(add.entity) else {
            return;
        };
        let wrinkles = sheet.wrinkles;
        let material = materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: PAPER_SHADE,
                perceptual_roughness: ROUGHNESS,
                reflectance: PAPER_REFLECTANCE,
                ..default()
            },
            extension: Self {
                params: SheetParams {
                    size: wrinkles.size,
                    seed: (wrinkles.seed % SEEDS) as f32,
                    page: 0.0,
                    folds: wrinkles.folds,
                },
                print: sheet.print.clone(),
            },
        });
        commands
            .entity(add.entity)
            .remove::<SheetPrint>()
            .insert(MeshMaterial3d(material));
    }

    /// Stains each page of a sheet in its own way.
    pub(super) fn set_page(&mut self, page: usize) {
        self.params.page = page as f32;
    }

    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("sheet.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl MaterialExtension for Sheet {
    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        Self::shader()
    }
}
