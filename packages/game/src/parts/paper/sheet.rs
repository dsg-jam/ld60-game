use std::f32::consts::{PI, TAU};
use std::hash::Hash;

use bevy::{
    asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path},
    mesh::{Indices, PrimitiveTopology},
    pbr::{ExtendedMaterial, MaterialExtension},
    platform::hash::fixed_hash_one,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::{ShaderRef, load_shader_library},
};
use rand::{RngExt, SeedableRng, rngs::SmallRng};

use super::leaf::{relaxed, relaxing};
use super::press::Print;
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

pub(super) struct SheetPlugin;

impl Plugin for SheetPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "punch.wesl");
        load_shader_library!(app, "press.wesl");
        load_shader_library!(app, "stock.wesl");
        embedded_asset!(app, "sheet.wesl");
        app.add_plugins(MaterialPlugin::<PaperMaterial>::default())
            .add_systems(Update, (Sheet::follow_print, Pressed::press))
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
    /// The [`Press`](super::press::Press) the page was printed on.
    press: u32,
    run: f32,
    /// The paper's tint, and in `w` its age, see [`Stock`](super::Stock).
    stock: Vec4,
    /// Two folds as a point on the line, its angle and its depth, in the
    /// sheet's space: meters from the top left corner, with y down.
    folds: [Vec4; 2],
    /// Center and radius of each punched hole, in the same space.
    holes: [Vec4; 2],
}

/// Gives a sheet its paper material once spawned.
#[derive(Component, Debug)]
pub(super) struct SheetPrint(pub(super) Handle<Image>);

/// Light a sheet gives off where the paper is bare.
#[derive(Component, Debug)]
pub(super) struct SheetGlow(pub(super) LinearRgba);

/// Presses a sheet with [`Wrinkles`] flat, or lets it relax again.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub(crate) struct Pressed {
    /// Seconds of relaxing it has had since it was pressed flat, see
    /// [`relaxing`].
    pub(crate) relaxed: f32,
    /// Whether it relaxes on its own as time goes on.
    pub(crate) relaxing: bool,
}

impl Pressed {
    /// Shows the share of the wrinkles each pressed sheet has relaxed to.
    fn press(
        time: Res<Time>,
        mut sheets: Query<(&mut Pressed, &Wrinkles, &Mesh3d)>,
        mut meshes: ResMut<Assets<Mesh>>,
    ) {
        for (mut pressed, wrinkles, mesh) in &mut sheets {
            let share = relaxing(pressed.relaxed);
            if pressed.relaxing && share < 1.0 {
                pressed.relaxed += time.delta_secs();
            } else if !pressed.is_changed() {
                continue;
            }
            if let Some(mut mesh) = meshes.get_mut(mesh) {
                *mesh = relaxed(&wrinkles.mesh(), relaxing(pressed.relaxed));
            }
        }
    }
}

/// The shape of a crumpled sheet that was flattened again.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct Wrinkles {
    size: Vec2,
    seed: u64,
    /// Points from the middle of the sheet to the edge that is held flat, in
    /// the sheet's space.
    held: Option<Vec2>,
    folds: [Vec4; 2],
    /// Direction and length in waves per meter, phase and height.
    waves: [(Vec2, f32, f32); 4],
    curls: [Curl; 2],
    /// The direction the whole sheet bends along, and how far its ends rise
    /// above its middle.
    bend: (Vec2, f32),
    /// How far a held sheet bows away from what it is held against.
    bow: f32,
}

/// A corner of a sheet lifting off, bent along a straight line across it
/// like real paper.
#[derive(Debug, Clone, Copy)]
struct Curl {
    corner: Vec2,
    /// Points from the corner into the sheet, across the line it bends
    /// along.
    inward: Vec2,
    /// From the corner to the line it bends along.
    reach: f32,
    lift: f32,
}

impl Curl {
    fn random(size: Vec2, lift: f32, rng: &mut SmallRng) -> Self {
        let corner = Vec2::new(
            if rng.random_bool(0.5) { 0.0 } else { size.x },
            if rng.random_bool(0.6) { size.y } else { 0.0 },
        );
        let diagonal = (size / 2.0 - corner).normalize();
        Self {
            corner,
            inward: Vec2::from_angle(rng.random_range(-0.35..0.35)).rotate(diagonal),
            reach: size.min_element() * rng.random_range(0.3..0.55),
            lift,
        }
    }

    fn height(&self, p: Vec2) -> f32 {
        let beyond = (1.0 - (p - self.corner).dot(self.inward) / self.reach).max(0.0);
        self.lift * beyond * beyond
    }

    /// Moves the curl to the other side of the sheet, if it is on the side
    /// of `held`.
    fn away_from(&mut self, held: Vec2, size: Vec2) {
        let from_middle = (self.corner - size / 2.0).dot(held);
        if from_middle > 0.0 {
            self.corner -= 2.0 * from_middle * held;
            self.inward = self.inward.reflect(held);
        }
    }
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
        // Mostly one corner lifts clearly, sometimes a second a little.
        let lift = if rng.random_bool(0.85) {
            rng.random_range(0.0025..0.006)
        } else {
            0.0
        };
        let first = Curl::random(size, lift, &mut rng);
        let lift = if rng.random_bool(0.4) {
            rng.random_range(0.001..0.003)
        } else {
            0.0
        };
        let second = Curl::random(size, lift, &mut rng);
        let along = if rng.random_bool(0.5) {
            Vec2::X
        } else {
            Vec2::Y
        };
        let bend = (
            Vec2::from_angle(rng.random_range(-0.3..0.3)).rotate(along),
            rng.random_range(0.0008..0.0025) * if rng.random_bool(0.75) { 1.0 } else { -1.0 },
        );
        Self {
            size,
            seed,
            held: None,
            folds,
            waves,
            curls: [first, second],
            bend,
            bow: rng.random_range(0.0015..0.003),
        }
    }

    /// The wrinkles of `document` on a sheet of `size`, held by `binding`.
    pub(crate) fn held(size: Vec2, document: impl Hash, binding: &Binding) -> Self {
        let mut wrinkles = Self::new(size, fixed_hash_one(document));
        wrinkles.hold(binding);
        wrinkles
    }

    pub(crate) fn size(&self) -> Vec2 {
        self.size
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
        for curl in &self.curls {
            height += curl.height(p);
        }
        let (along, rise) = self.bend;
        let half = (self.size * along.abs()).element_sum() / 2.0;
        let from_middle = (p - self.size / 2.0).dot(along) / half;
        height += rise * from_middle * from_middle;
        if let Some(held) = self.held {
            // Pressed down under the clip, the sheet bulges out a third of
            // the way down and lies back towards its free end.
            let length = (self.size * held.abs()).element_sum();
            let t = (length / 2.0 - (p - self.size / 2.0).dot(held)) / length;
            height += self.bow * 6.75 * t * (1.0 - t) * (1.0 - t);
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
        let held = binding.held() * Vec2::new(1.0, -1.0);
        for curl in &mut self.curls {
            curl.away_from(held, self.size);
        }
        self.held = Some(held);
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
        sheets: Query<(&SheetPrint, &Wrinkles, Option<&SheetGlow>, Option<&Print>)>,
        mut materials: ResMut<Assets<PaperMaterial>>,
        mut commands: Commands,
    ) {
        let Ok((SheetPrint(print), &wrinkles, glow, printed)) = sheets.get(add.entity) else {
            return;
        };
        let printed = printed.copied().unwrap_or_default();
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
                    press: printed.press as u32,
                    run: printed.run,
                    stock: printed.stock.params(),
                    folds: wrinkles.folds,
                    holes: [Vec4::ZERO; 2],
                },
                print: print.clone(),
            },
        });
        commands
            .entity(add.entity)
            .remove::<SheetPrint>()
            .insert(MeshMaterial3d(material));
    }

    /// The same sheet with `print` on it, like its blank back.
    pub(super) fn reprinted(&self, print: Handle<Image>) -> Self {
        Self {
            params: self.params,
            print,
        }
    }

    /// Shows the sheet with `wrinkles`, punched with `holes`, each a center
    /// in the sheet's space and a radius.
    pub(crate) fn set_sheet(&mut self, wrinkles: &Wrinkles, holes: [Vec3; 2]) {
        self.params.seed = (wrinkles.seed % SEEDS) as f32;
        self.params.page = 0.0;
        self.params.folds = wrinkles.folds;
        self.params.holes = holes.map(|hole| hole.extend(0.0));
    }

    /// Prints the page a sheet shows the way its [`Print`] says.
    fn follow_print(
        sheets: Query<(&Print, &MeshMaterial3d<PaperMaterial>), Changed<Print>>,
        mut materials: ResMut<Assets<PaperMaterial>>,
    ) {
        for (print, material) in &sheets {
            if let Some(mut material) = materials.get_mut(material) {
                material.extension.set_print(*print);
            }
        }
    }

    pub(crate) fn set_print(&mut self, print: Print) {
        self.params.press = print.press as u32;
        self.params.run = print.run;
        self.params.stock = print.stock.params();
    }

    /// Stains each page of a sheet in its own way.
    pub(crate) fn set_page(&mut self, page: usize) {
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
