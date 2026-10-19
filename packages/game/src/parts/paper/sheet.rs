use std::f32::consts::{PI, TAU};

use bevy::{
    asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path},
    mesh::{Indices, PrimitiveTopology},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat},
    shader::ShaderRef,
};
use rand::{RngExt, SeedableRng, rngs::SmallRng};

use super::{Binding, PAPER_REFLECTANCE, PAPER_SHADE, Paper};

/// About one vertex per this many meters. Wrinkles are broad, so this is
/// enough. The shader draws the sharp fold lines.
const GRID: f32 = 0.01;
const ROUGHNESS: f32 = 0.95;
/// The shader gets one of this many seeds.
const SEEDS: u64 = 1000;
/// Width of the bulge along a fold, in meters.
const FOLD_WIDTH: f32 = 0.02;
/// Distance from a held edge that stays flat, in meters, like under a clip.
const HELD_FLAT: f32 = 0.012;
/// Distance from a held edge where the wrinkles are back in full.
const HELD_REACH: f32 = 0.04;
/// About the size of a strip of a turning page, for the grain.
const BLANK_SIZE: Vec2 = Vec2::new(0.04, 0.22);

pub(super) struct SheetPlugin;

impl Plugin for SheetPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "sheet.wesl");
        app.add_plugins(MaterialPlugin::<PaperMaterial>::default())
            .add_observer(Sheet::add_material)
            .add_observer(Self::hold_bound_edge);
    }
}

impl SheetPlugin {
    /// Flattens the edge of the sheet that its binding grips.
    fn hold_bound_edge(
        add: On<Add<Binding>>,
        papers: Query<(&Paper, &Binding)>,
        mut sheets: Query<(&Mesh3d, &mut Wrinkles)>,
        mut meshes: ResMut<Assets<Mesh>>,
    ) {
        let Ok((paper, binding)) = papers.get(add.entity) else {
            return;
        };
        let Ok((mesh, mut wrinkles)) = sheets.get_mut(paper.sheet) else {
            return;
        };
        wrinkles.hold(binding);
        if let Some(mut mesh) = meshes.get_mut(mesh) {
            *mesh = wrinkles.mesh();
        }
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
    /// How much grime and stains show, from 0 to 1.
    stains: f32,
    /// Two folds as a point on the line, its angle and its depth, in the
    /// sheet's space: meters from the top left corner, with y down.
    folds: [Vec4; 2],
}

/// Gives a sheet its paper material once spawned.
#[derive(Component, Debug)]
pub(super) struct SheetPrint(pub(super) Handle<Image>);

/// Light a sheet gives off where the paper is bare.
#[derive(Component, Debug)]
pub(super) struct SheetGlow(pub(super) LinearRgba);

/// The shape of a crumpled sheet that was flattened again.
#[derive(Component, Debug, Clone, Copy)]
pub(super) struct Wrinkles {
    size: Vec2,
    seed: u64,
    /// Points from the middle of the sheet to the edge that is held flat, in
    /// the sheet's space.
    held: Option<Vec2>,
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
            held: None,
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

    /// How much the sheet may rise at `p`, from 0 at a held edge to 1.
    fn freedom(&self, p: Vec2) -> f32 {
        let Some(held) = self.held else {
            return 1.0;
        };
        let half = self.size / 2.0;
        let distance = (half * held.abs()).element_sum() - (p - half).dot(held);
        ((distance - HELD_FLAT) / (HELD_REACH - HELD_FLAT)).clamp(0.0, 1.0)
    }

    /// Holds the edge of the sheet the `binding` grips flat, so it stays
    /// under a clip or around the rings.
    pub(super) fn hold(&mut self, binding: &Binding) {
        // The sheet's y runs down the page, the mesh's y up.
        self.held = Some(binding.held().truncate() * Vec2::new(1.0, -1.0));
    }

    /// A grid facing +Z like a [`Rectangle`], displaced by the wrinkles.
    pub(super) fn mesh(&self) -> Mesh {
        let size = self.size;
        let cells = (size / GRID).ceil().as_uvec2().max(UVec2::splat(4));
        let uvs: Vec<Vec2> = (0..=cells.y)
            .flat_map(|y| (0..=cells.x).map(move |x| UVec2::new(x, y).as_vec2() / cells.as_vec2()))
            .collect();
        let lowest = uvs
            .iter()
            .map(|uv| self.height(*uv * size))
            .fold(f32::INFINITY, f32::min);
        let lifted = |p: Vec2| {
            let freedom = self.freedom(p);
            (self.height(p) - lowest) * freedom * freedom * (3.0 - 2.0 * freedom)
        };
        let positions: Vec<Vec3> = uvs
            .iter()
            .map(|uv| ((*uv - 0.5) * Vec2::new(size.x, -size.y)).extend(lifted(*uv * size)))
            .collect();
        let step = 0.001;
        let normals: Vec<Vec3> = uvs
            .iter()
            .map(|uv| {
                let p = *uv * size;
                let dx = lifted(p + Vec2::X * step) - lifted(p - Vec2::X * step);
                let dy = lifted(p + Vec2::Y * step) - lifted(p - Vec2::Y * step);
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
        sheets: Query<(&SheetPrint, &Wrinkles, Option<&SheetGlow>)>,
        mut materials: ResMut<Assets<PaperMaterial>>,
        mut commands: Commands,
    ) {
        let Ok((SheetPrint(print), &wrinkles, glow)) = sheets.get(add.entity) else {
            return;
        };
        let material = materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: PAPER_SHADE,
                emissive: glow.map_or(LinearRgba::BLACK, |glow| glow.0),
                perceptual_roughness: ROUGHNESS,
                reflectance: PAPER_REFLECTANCE,
                ..default()
            },
            extension: Self {
                params: SheetParams {
                    size: wrinkles.size,
                    seed: (wrinkles.seed % SEEDS) as f32,
                    page: 0.0,
                    stains: 1.0,
                    folds: wrinkles.folds,
                },
                print: print.clone(),
            },
        });
        commands
            .entity(add.entity)
            .remove::<SheetPrint>()
            .insert(MeshMaterial3d(material));
    }

    /// Clean paper on both sides, for a page that only shows while it turns.
    pub(super) fn blank(images: &mut Assets<Image>, color: Color) -> PaperMaterial {
        let white = images.add(Image::new_fill(
            Extent3d::default(),
            TextureDimension::D2,
            &[255; 4],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        ));
        ExtendedMaterial {
            base: StandardMaterial {
                base_color: color,
                perceptual_roughness: ROUGHNESS,
                reflectance: PAPER_REFLECTANCE,
                double_sided: true,
                cull_mode: None,
                ..default()
            },
            extension: Self {
                params: SheetParams {
                    size: BLANK_SIZE,
                    seed: 0.0,
                    page: 0.0,
                    stains: 0.0,
                    folds: [Vec4::ZERO; 2],
                },
                print: white,
            },
        }
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
