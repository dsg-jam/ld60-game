mod cladding;

use std::ops::Range;

use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::{ShaderRef, load_shader_library},
};
use rand::{Rng, RngExt};

use self::cladding::Cladding;
use super::additions::Additions;
use super::drips::{DripEdge, DripEdges};
use super::market::{MARKET_HEIGHT, STALL_DEPTH, Stock};
use super::weather::Weather;
use super::{FACADES, HALF_LENGTH, Kit, glow, rng};
use crate::geometry::block::Block;
use crate::geometry::surface::{Finish, SurfaceMaterial};
use crate::workshop::shop;

/// Plots are at least this wide, so everything on them fits.
const MIN_PLOT: f32 = 3.0;
const SEED: u64 = 2;

const ROWS: [Row; 6] = [
    // Our side of the street.
    Row {
        front: shop::FRONT,
        facing: -1.0,
        gap: Some(shop::SIZE.x / 2.0),
        half_length: HALF_LENGTH,
        width: 6.0..18.0,
        depth: 12.0..22.0,
        height: 10.0..65.0,
        skew: 3,
        setback: 0.0,
        alley_chance: 0.2,
        alleys: 1.2..3.0,
        market: true,
        additions: true,
        low_across: None,
        tiers: 0.3,
        crowns: 0.0,
    },
    // Across the street.
    Row {
        front: FACADES,
        facing: 1.0,
        gap: None,
        half_length: HALF_LENGTH,
        width: 6.0..18.0,
        depth: 12.0..22.0,
        height: 10.0..65.0,
        skew: 3,
        setback: 1.5,
        alley_chance: 0.35,
        alleys: 1.2..3.0,
        market: true,
        additions: true,
        low_across: Some(LOW_ACROSS),
        tiers: 0.3,
        crowns: 0.0,
    },
    // The skyline behind: towers of all heights with gaps between them.
    Row {
        front: -48.0,
        facing: 1.0,
        gap: None,
        half_length: 160.0,
        width: 8.0..22.0,
        depth: 15.0..30.0,
        height: 20.0..150.0,
        skew: 3,
        setback: 10.0,
        alley_chance: 0.55,
        alleys: 3.0..12.0,
        market: false,
        additions: true,
        low_across: None,
        tiers: 0.6,
        crowns: 0.15,
    },
    Row {
        front: -120.0,
        facing: 1.0,
        gap: None,
        half_length: 320.0,
        width: 12.0..35.0,
        depth: 25.0..45.0,
        height: 50.0..330.0,
        skew: 3,
        setback: 25.0,
        alley_chance: 0.5,
        alleys: 8.0..30.0,
        market: false,
        additions: false,
        low_across: None,
        tiers: 0.7,
        crowns: 0.25,
    },
    // Skyscrapers far apart, standing out against the low rows.
    Row {
        front: -200.0,
        facing: 1.0,
        gap: None,
        half_length: 300.0,
        width: 16.0..30.0,
        depth: 16.0..30.0,
        height: 180.0..380.0,
        skew: 1,
        setback: 20.0,
        alley_chance: 1.0,
        alleys: 25.0..80.0,
        market: false,
        additions: false,
        low_across: None,
        tiers: 0.8,
        crowns: 0.5,
    },
    // Silhouettes far away in the haze.
    Row {
        front: -260.0,
        facing: 1.0,
        gap: None,
        half_length: 600.0,
        width: 25.0..70.0,
        depth: 40.0..70.0,
        height: 120.0..500.0,
        skew: 3,
        setback: 40.0,
        alley_chance: 0.5,
        alleys: 20.0..60.0,
        market: false,
        additions: false,
        low_across: None,
        tiers: 0.7,
        crowns: 0.3,
    },
];
/// The buildings across from the shop within this distance of it stay low.
const LOW_HALF_WIDTH: f32 = 8.0;
/// Their height: the market floor and one more above it.
const LOW_ACROSS: Range<f32> = 6.5..7.5;
/// Straight across from the shop, the rows behind the street stay below the
/// buildings across, so the sky above them is open. Half the width of that
/// view and its top, as angles from the player's eyes in radians.
const VIEW_CONE: f32 = 0.42;
const VIEW_TOP: f32 = 0.49;
const EYE_HEIGHT: f32 = 1.5;
/// Buildings behind the street this tall may get large signs.
const BACKDROP_HEIGHT: f32 = 20.0;
/// Height of the wall around the roof, above the top floor.
const PARAPET: f32 = 0.8;
/// How far the roof edge sticks out.
const LEDGE_OUT: f32 = 0.12;
const LEDGE_HEIGHT: f32 = 0.3;
const SHOP_FLOOR: f32 = 3.4;

const FLOOR_HEIGHT: Range<f32> = 3.0..3.8;
const WINDOW_PITCH: Range<f32> = 1.8..3.6;
const LIT: Range<f32> = 0.2..0.5;
const WINDOW_GLOW: f32 = 6.0;
const AGE: Range<f32> = 0.4..1.0;
/// Floors above the shop, as high as this.
const SHOP_BUILDING_HEIGHT: f32 = 24.0;
const SHOP_BUILDING_DEPTH: f32 = 14.0;
/// Matches `COVERS` in `facade.wesl`.
const COVERS: usize = 4;

const ROOF_BOXES: Range<usize> = 1..5;
const ANTENNA_CHANCE: f64 = 0.5;
const ANTENNA_HEIGHT: Range<f32> = 3.0..12.0;
const BEACON_SIZE: f32 = 0.35;
const BEACON_PERIOD: f32 = 1.6;

const LEDGE: Color = Color::srgb_u8(0x4e, 0x4a, 0x4c);
const CROWNS: [Color; 4] = [
    Color::srgb_u8(0xff, 0x2a, 0x6d),
    Color::srgb_u8(0x05, 0xd9, 0xe8),
    Color::srgb_u8(0xb1, 0x4a, 0xed),
    Color::srgb_u8(0xff, 0xb3, 0x2e),
];
const CROWN_GLOW: f32 = 12.0;
const CROWN_HEIGHT: f32 = 0.6;
const ROOF_GEAR: Color = Color::srgb_u8(0x50, 0x52, 0x56);
const BEACON: Color = Color::srgb_u8(0xff, 0x20, 0x20);

pub(super) struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "facade.wesl");
        load_shader_library!(app, "walls.wesl");
        app.add_plugins(MaterialPlugin::<FacadeMaterial>::default())
            .add_systems(Startup, Skyline::spawn)
            .add_systems(
                Update,
                (
                    Beacon::blink,
                    Facade::follow_weather.run_if(resource_changed::<Weather>),
                ),
            );
    }
}

pub(super) type FacadeMaterial = ExtendedMaterial<StandardMaterial, Facade>;

/// Front faces of the buildings across the road and behind it, for hanging
/// signs on.
#[derive(Resource, Debug, Default)]
pub(super) struct Skyline {
    pub(super) front: Vec<Frontage>,
    /// Tall buildings in the rows behind the street.
    pub(super) back: Vec<Backdrop>,
    /// Space taken by every building, which flying cars keep out of.
    pub(super) volumes: Vec<Block>,
}

/// A building facing the street.
#[derive(Debug)]
pub(super) struct Frontage {
    pub(super) block: Block,
    pub(super) facade: Handle<FacadeMaterial>,
}

/// A tall building behind the street, seen over the buildings across it.
#[derive(Debug)]
pub(super) struct Backdrop {
    pub(super) front: Frontage,
    /// Its top roof, if there is room on it for a sign that doesn't block
    /// the view of the sky from the shop.
    pub(super) roof: Option<Block>,
    /// Whether the player sees it from the shop, beside the view of the sky.
    pub(super) in_sight: bool,
}

/// Where a building stands, and the height of its floors.
struct Plot {
    block: Block,
    floor: f32,
    /// Kept low for the view from the shop, without anything on top.
    low: bool,
}

/// One row of buildings, parallel to the street.
struct Row {
    /// Z of the front faces.
    front: f32,
    /// Direction along Z that the front faces look.
    facing: f32,
    /// Half width of a gap around X = 0 for the shop.
    gap: Option<f32>,
    half_length: f32,
    width: Range<f32>,
    depth: Range<f32>,
    height: Range<f32>,
    /// Higher makes tall buildings rarer.
    skew: i32,
    /// How far a building may stand back from the row front.
    setback: f32,
    alley_chance: f64,
    alleys: Range<f32>,
    /// Whether the ground floor is a market.
    market: bool,
    /// Whether people built additions onto the front.
    additions: bool,
    /// Chances of a narrower tier on top, and of a glowing crown.
    tiers: f64,
    crowns: f64,
    /// Heights of the buildings across from the shop, kept low so the
    /// skyline shows behind them.
    low_across: Option<Range<f32>>,
}

/// Windows drawn onto the walls of a building.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(super) struct Facade {
    #[uniform(100)]
    params: FacadeParams,
}

/// Matches `Facade` in `facade.wesl`.
#[derive(ShaderType, Reflect, Debug, Clone)]
struct FacadeParams {
    /// Corner of the building with the smallest coordinates.
    origin: Vec3,
    seed: f32,
    size: Vec3,
    /// Horizontal window pitch and floor height. Each wall fits a whole
    /// number of windows, so the pitch varies slightly from wall to wall.
    cell: Vec2,
    /// Window size as a share of the cell.
    window: Vec2,
    /// Share of lit windows.
    lit: f32,
    glow: f32,
    /// Direction along Z that the street side looks, 0 for none.
    facing: f32,
    /// Number of floors with windows, from the bottom.
    floors: f32,
    archetype: u32,
    cladding: u32,
    /// How wet the walls are, from 0 to 1.
    wetness: f32,
    /// How weathered the walls are, from 0 to 1.
    age: f32,
    /// Areas of the street side without windows, as min and max corners.
    covers: [Vec4; COVERS],
}

/// Where the windows of a facade are.
#[derive(Clone, Copy, Debug)]
pub(super) struct WindowGrid {
    /// Horizontal window pitch and floor height.
    pub(super) cell: Vec2,
    /// Window size as a share of the cell, centered in it.
    pub(super) window: Vec2,
}

/// What a building is used for, which decides how its windows light up.
///
/// Matches the constants in `facade.wesl`.
#[derive(Clone, Copy, Debug)]
enum Archetype {
    Office = 0,
    Home = 1,
    /// Mostly empty.
    Dead = 2,
}

/// The shared blinking material of the red lights on top of antennas.
#[derive(Resource, Debug)]
struct Beacon(Handle<StandardMaterial>);

impl Skyline {
    pub(super) fn spawn(
        mut commands: Commands,
        kit: Res<Kit>,
        mut facades: ResMut<Assets<FacadeMaterial>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let mut rng = rng(SEED);
        let gear = surfaces.add(Finish::Rust.scaled(ROOF_GEAR));
        let ledge = surfaces.add(Finish::Concrete.scaled(LEDGE));
        let crowns: Vec<_> = CROWNS
            .iter()
            .map(|&color| materials.add(glow(color, CROWN_GLOW)))
            .collect();
        let beacon = materials.add(glow(BEACON, 0.0));
        commands.insert_resource(Beacon(beacon.clone()));

        let market = Stock::new(&mut materials, &mut surfaces);
        let additions = Additions::new(&mut materials, &mut surfaces);
        let mut skyline = Skyline::default();
        let mut drips = Vec::new();
        let mut root = commands.spawn((
            Name::new("buildings"),
            Transform::default(),
            Visibility::default(),
        ));
        root.with_children(|parent| {
            for row in &ROWS {
                for Plot { block, floor, low } in row.lay_out(&mut rng) {
                    let facade = if row.market {
                        Self::spawn_market_building(
                            parent,
                            &kit,
                            &mut facades,
                            &market,
                            &additions,
                            &mut drips,
                            &block,
                            floor,
                            row.facing,
                            &mut rng,
                        )
                    } else {
                        let facade =
                            facades.add(Facade::random(&block, row.facing, floor, &mut rng));
                        parent.spawn(kit.cuboid(&block, facade.clone()));
                        facade
                    };
                    parent.spawn(kit.cuboid(&Self::roof_edge(&block), ledge.clone()));
                    skyline.volumes.push(block);
                    if row.additions
                        && !row.market
                        && let Some(grid) = facades.get(&facade).map(|m| m.extension.grid())
                    {
                        additions.spawn_on(
                            parent,
                            &kit,
                            &mut facades,
                            &block,
                            grid,
                            row.facing,
                            &mut rng,
                        );
                    }
                    // Up to two narrower tiers, stepping in towards the top.
                    let mut roof = block;
                    for _ in 0..2 {
                        let floors = ((roof.size.y - PARAPET) / floor) as usize;
                        if low || floors < 3 || !rng.random_bool(row.tiers) {
                            break;
                        }
                        let tier_floors = rng.random_range(1..=floors / 2) as f32;
                        let tier = Block::new(Vec3::new(
                            roof.size.x * rng.random_range(0.5..0.85),
                            tier_floors * floor + PARAPET,
                            roof.size.z * rng.random_range(0.5..0.85),
                        ))
                        .on_top_of(&roof)
                        .translated(
                            Vec3::Z * row.facing * -rng.random_range(0.0..roof.size.z * 0.1),
                        );
                        let facade = Facade::random(&tier, 0.0, floor, &mut rng);
                        parent.spawn(kit.cuboid(&tier, facades.add(facade)));
                        parent.spawn(kit.cuboid(&Self::roof_edge(&tier), ledge.clone()));
                        skyline.volumes.push(tier);
                        roof = tier;
                    }
                    let crowned = !low && rng.random_bool(row.crowns);
                    if crowned {
                        let crown = &crowns[rng.random_range(0..crowns.len())];
                        Self::spawn_crown(parent, &kit, &roof, crown);
                    }
                    Self::spawn_roof_gear(parent, &kit, &roof, &gear, &beacon, &mut rng);
                    if row.market && row.facing > 0.0 {
                        skyline.front.push(Frontage { block, facade });
                    } else if row.facing > 0.0 && !low && block.size.y > BACKDROP_HEIGHT {
                        let free = !crowned && !Self::near_view(&roof, 1.15);
                        skyline.back.push(Backdrop {
                            front: Frontage { block, facade },
                            roof: free.then_some(roof),
                            in_sight: Self::near_view(&block, 3.0),
                        });
                    }
                }
            }
        });
        // The building the shop is in: floors above it and rooms behind it.
        let back = shop::FRONT + SHOP_BUILDING_DEPTH;
        let parts = [
            Block::between(
                Vec3::new(-shop::SIZE.x / 2.0, shop::SIZE.y, shop::FRONT),
                Vec3::new(shop::SIZE.x / 2.0, SHOP_BUILDING_HEIGHT, back),
            ),
            Block::between(
                Vec3::new(-shop::SIZE.x / 2.0, 0.0, shop::FRONT + shop::SIZE.z),
                Vec3::new(shop::SIZE.x / 2.0, shop::SIZE.y, back),
            ),
        ];
        skyline.volumes.extend(parts);
        for part in parts {
            commands.spawn((
                Name::new("shop building"),
                kit.cuboid(
                    &part,
                    facades.add(Facade::random(&part, 0.0, SHOP_FLOOR, &mut rng)),
                ),
            ));
        }
        commands.insert_resource(skyline);
        commands.insert_resource(DripEdges(drips));
    }

    /// A building with a market in its ground floor and windows above.
    /// Returns the facade of the upper floors.
    #[expect(
        clippy::too_many_arguments,
        reason = "all of it goes into one building"
    )]
    fn spawn_market_building(
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        facades: &mut Assets<FacadeMaterial>,
        market: &Stock,
        additions: &Additions,
        drips: &mut Vec<DripEdge>,
        block: &Block,
        floor: f32,
        facing: f32,
        rng: &mut impl Rng,
    ) -> Handle<FacadeMaterial> {
        let upper = Block::between(block.min().with_y(MARKET_HEIGHT), block.max());
        // The base stands back behind the stalls.
        let base = if facing > 0.0 {
            Block::between(
                block.min(),
                block.max().with_y(MARKET_HEIGHT) - Vec3::Z * STALL_DEPTH,
            )
        } else {
            Block::between(
                block.min() + Vec3::Z * STALL_DEPTH,
                block.max().with_y(MARKET_HEIGHT),
            )
        };
        let facade = facades.add(Facade::random(&upper, facing, floor, rng));
        parent.spawn(kit.cuboid(&upper, facade.clone()));
        parent.spawn(kit.cuboid(&base, market.wall()));
        market.spawn_market(parent, kit, block, facing, drips, rng);
        if let Some(grid) = facades
            .get(&facade)
            .map(|material| material.extension.grid())
        {
            additions.spawn_on(parent, kit, facades, &upper, grid, facing, rng);
        }
        facade
    }

    /// Whether `block` is within `width` times the view of the sky from the
    /// shop.
    fn near_view(block: &Block, width: f32) -> bool {
        let distance = shop::FRONT - block.max().z;
        let view = distance * VIEW_CONE.tan() * width;
        block.min().x < view && block.max().x > -view
    }

    /// A band of light around the top of a tower, seen from far away.
    fn spawn_crown(
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        roof: &Block,
        glow: &Handle<StandardMaterial>,
    ) {
        let out = LEDGE_OUT + 0.05;
        let (min, max) = (roof.min(), roof.max());
        let y = max.y - LEDGE_HEIGHT - CROWN_HEIGHT;
        let band = |a: Vec3, b: Vec3| Block::between(a.with_y(y), b.with_y(y + CROWN_HEIGHT));
        let sides = [
            band(
                Vec3::new(min.x - out, 0.0, max.z),
                Vec3::new(max.x + out, 0.0, max.z + out),
            ),
            band(
                Vec3::new(min.x - out, 0.0, min.z - out),
                Vec3::new(max.x + out, 0.0, min.z),
            ),
            band(
                Vec3::new(min.x - out, 0.0, min.z),
                Vec3::new(min.x, 0.0, max.z),
            ),
            band(
                Vec3::new(max.x, 0.0, min.z),
                Vec3::new(max.x + out, 0.0, max.z),
            ),
        ];
        for side in &sides {
            parent.spawn(kit.cuboid(side, glow.clone()));
        }
    }

    /// A ledge around the top of `block`, sticking out a little.
    fn roof_edge(block: &Block) -> Block {
        Block::between(
            block.min().with_y(block.max().y - LEDGE_HEIGHT) - Vec3::new(LEDGE_OUT, 0.0, LEDGE_OUT),
            block.max() + Vec3::new(LEDGE_OUT, 0.05, LEDGE_OUT),
        )
    }

    /// Air conditioners and antennas on a roof.
    fn spawn_roof_gear(
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        roof: &Block,
        gear: &Handle<SurfaceMaterial>,
        beacon: &Handle<StandardMaterial>,
        rng: &mut impl Rng,
    ) {
        for _ in 0..rng.random_range(ROOF_BOXES) {
            let size = Vec3::new(
                rng.random_range(1.0..3.0),
                rng.random_range(0.8..2.0),
                rng.random_range(1.0..3.0),
            );
            let spot = Vec2::new(rng.random_range(-0.8..0.8), rng.random_range(-0.8..0.8));
            let unit = Block::new(size).resting_on(roof, spot);
            parent.spawn(kit.cuboid(&unit, gear.clone()));
        }
        if rng.random_bool(ANTENNA_CHANCE) {
            let spot = Vec2::new(rng.random_range(-0.7..0.7), rng.random_range(-0.7..0.7));
            let mast = Block::new(Vec3::new(0.12, rng.random_range(ANTENNA_HEIGHT), 0.12))
                .resting_on(roof, spot);
            let light = Block::new(Vec3::splat(BEACON_SIZE)).on_top_of(&mast);
            parent.spawn(kit.cuboid(&mast, gear.clone()));
            parent.spawn(kit.cuboid(&light, beacon.clone()));
        }
    }
}

impl Row {
    fn lay_out(&self, rng: &mut impl Rng) -> Vec<Plot> {
        let mut plots = Vec::new();
        let mut x = -self.half_length;
        while x < self.half_length {
            let mut width = rng.random_range(self.width.clone());
            if let Some(gap) = self.gap
                && x < gap
                && x + width > -gap
            {
                // The building ends at the gap, the next one starts after.
                // A sliver left before the gap stays empty.
                if x < -gap - MIN_PLOT {
                    width = -gap - x;
                } else {
                    x = gap;
                    continue;
                }
            }
            let depth = rng.random_range(self.depth.clone());
            let front = self.front - self.facing * rng.random_range(0.0..=self.setback);
            let mut height = self.height.start
                + (self.height.end - self.height.start) * rng.random::<f32>().powi(self.skew);
            if let Some(low) = &self.low_across
                && x < LOW_HALF_WIDTH
                && x + width > -LOW_HALF_WIDTH
            {
                height = rng.random_range(low.clone());
            }
            let distance = shop::FRONT - front;
            let view = distance * VIEW_CONE.tan();
            let low = !self.market && x < view && x + width > -view;
            if low {
                height = height.min(EYE_HEIGHT + distance * VIEW_TOP.tan());
            }
            // Whole floors, and a parapet on top.
            let floor = rng.random_range(FLOOR_HEIGHT);
            let base = if self.market { MARKET_HEIGHT } else { 0.0 };
            let floors = if low {
                ((height - base - PARAPET) / floor).floor().max(1.0)
            } else {
                ((height - base) / floor).round().max(1.0)
            };
            let height = base + floors * floor + PARAPET;
            plots.push(Plot {
                block: Block::between(
                    Vec3::new(x, 0.0, front - self.facing * depth),
                    Vec3::new(x + width, height, front),
                ),
                floor,
                low,
            });
            x += width;
            if rng.random_bool(self.alley_chance) {
                x += rng.random_range(self.alleys.clone());
            }
        }
        plots
    }
}

impl Facade {
    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("facade.wesl"))
            .with_source("embedded")
            .into()
    }

    pub(super) fn grid(&self) -> WindowGrid {
        WindowGrid {
            cell: self.params.cell,
            window: self.params.window,
        }
    }

    fn follow_weather(weather: Res<Weather>, mut facades: ResMut<Assets<FacadeMaterial>>) {
        for (_, facade) in facades.iter_mut() {
            facade.extension.params.wetness = weather.wetness;
        }
    }

    /// Hides the windows behind `area` of the shop side, in world X and Y.
    pub(super) fn cover(&mut self, area: Rect) {
        let origin = self.params.origin.xy();
        let free = self
            .params
            .covers
            .iter_mut()
            .find(|cover| cover.z <= cover.x);
        if let Some(cover) = free {
            *cover = (area.min - origin)
                .extend(area.max.x - origin.x)
                .extend(area.max.y - origin.y);
        }
    }

    /// Windows for `block` with floors of height `floor`. The block faces
    /// the street along `facing`.
    pub(super) fn random(
        block: &Block,
        facing: f32,
        floor: f32,
        rng: &mut impl Rng,
    ) -> FacadeMaterial {
        let cladding = Cladding::random(rng);
        let wall = cladding.color(rng);
        let columns = (block.size.x / rng.random_range(WINDOW_PITCH))
            .round()
            .max(1.0);
        ExtendedMaterial {
            base: StandardMaterial {
                base_color: wall,
                perceptual_roughness: 0.85,
                ..default()
            },
            extension: Self {
                params: FacadeParams {
                    origin: block.min(),
                    seed: rng.random_range(0.0..1000.0),
                    size: block.size,
                    cell: Vec2::new(block.size.x / columns, floor),
                    window: Vec2::new(rng.random_range(0.3..0.6), rng.random_range(0.4..0.6)),
                    lit: rng.random_range(LIT),
                    glow: WINDOW_GLOW,
                    facing,
                    floors: (block.size.y / floor).floor(),
                    archetype: Archetype::random(rng) as u32,
                    cladding: cladding as u32,
                    wetness: 0.0,
                    age: rng.random_range(AGE),
                    covers: [Vec4::ZERO; COVERS],
                },
            },
        }
    }
}

impl Archetype {
    fn random(rng: &mut impl Rng) -> Self {
        match rng.random_range(0.0..1.0) {
            x if x < 0.35 => Self::Office,
            x if x < 0.9 => Self::Home,
            _ => Self::Dead,
        }
    }
}

impl MaterialExtension for Facade {
    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        Self::shader()
    }
}

impl Beacon {
    fn blink(
        time: Res<Time>,
        beacon: Res<Beacon>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Some(mut material) = materials.get_mut(&beacon.0) else {
            return;
        };
        let on = (time.elapsed_secs() / BEACON_PERIOD).fract() < 0.15;
        let strength = if on { 40.0 } else { 0.5 };
        material.emissive = BEACON.to_linear() * strength;
    }
}
