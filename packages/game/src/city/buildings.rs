mod cladding;
mod facade;
mod rows;

use std::ops::Range;

use bevy::{prelude::*, shader::load_shader_library};
use ld60_geometry::block::Block;
use ld60_geometry::surface::{Finish, SurfaceMaterial};
use rand::{Rng, RngExt};

pub(super) use self::facade::{Facade, FacadeMaterial, WindowGrid};
use self::rows::{Plot, ROWS, Row, VIEW_CONE};
use super::additions::Additions;
use super::drips::{DripEdge, DripEdges};
use super::market::{MARKET_HEIGHT, STALL_DEPTH, Stock};
use super::{Kit, NEON, Pick, glow, rng};
use crate::workshop::shop;

const SEED: u64 = 2;

/// Buildings behind the street this tall may get large signs.
const BACKDROP_HEIGHT: f32 = 20.0;
/// Height of the wall around the roof, above the top floor.
const PARAPET: f32 = 0.8;
/// How far the roof edge sticks out.
const LEDGE_OUT: f32 = 0.12;
const LEDGE_HEIGHT: f32 = 0.3;
const SHOP_FLOOR: f32 = 3.4;

/// Floors above the shop, as high as this.
const SHOP_BUILDING_HEIGHT: f32 = 24.0;
const SHOP_BUILDING_DEPTH: f32 = 14.0;

const ROOF_BOXES: Range<usize> = 1..5;
const ANTENNA_CHANCE: f64 = 0.5;
const ANTENNA_HEIGHT: Range<f32> = 3.0..12.0;
const BEACON_SIZE: f32 = 0.35;
const BEACON_PERIOD: f32 = 1.6;

const LEDGE: Color = Color::srgb_u8(0x4e, 0x4a, 0x4c);
const CROWNS: [Color; 4] = [NEON[0], NEON[1], NEON[2], NEON[4]];
const CROWN_GLOW: f32 = 12.0;
const CROWN_HEIGHT: f32 = 0.6;
const ROOF_GEAR: Color = Color::srgb_u8(0x50, 0x52, 0x56);
const BEACON: Color = Color::srgb_u8(0xff, 0x20, 0x20);

pub(super) struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "walls.wesl");
        app.add_plugins(facade::FacadePlugin)
            .add_systems(Startup, Skyline::spawn)
            .add_systems(Update, Beacon::blink);
    }
}

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
                for plot in row.lay_out(&mut rng) {
                    let Plot { block, floor, low } = plot;
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
                    let roof = skyline.spawn_tiers(
                        parent,
                        &kit,
                        &mut facades,
                        &ledge,
                        &plot,
                        row,
                        &mut rng,
                    );
                    let crowned = !low && rng.random_bool(row.crowns);
                    if crowned {
                        let crown = crowns.pick(&mut rng);
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
        skyline.spawn_shop_building(&mut commands, &kit, &mut facades, &mut rng);
        commands.insert_resource(skyline);
        commands.insert_resource(DripEdges(drips));
    }

    /// Up to two narrower tiers on top of `plot`, stepping in towards the
    /// top. Returns the roof of the highest.
    #[expect(
        clippy::too_many_arguments,
        reason = "all of it goes into one building"
    )]
    fn spawn_tiers(
        &mut self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        facades: &mut Assets<FacadeMaterial>,
        ledge: &Handle<SurfaceMaterial>,
        plot: &Plot,
        row: &Row,
        rng: &mut impl Rng,
    ) -> Block {
        let floor = plot.floor;
        let mut roof = plot.block;
        for _ in 0..2 {
            let floors = ((roof.size.y - PARAPET) / floor) as usize;
            if plot.low || floors < 3 || !rng.random_bool(row.tiers) {
                break;
            }
            let tier_floors = rng.random_range(1..=floors / 2) as f32;
            let tier = Block::new(Vec3::new(
                roof.size.x * rng.random_range(0.5..0.85),
                tier_floors * floor + PARAPET,
                roof.size.z * rng.random_range(0.5..0.85),
            ))
            .on_top_of(&roof)
            .translated(Vec3::Z * row.facing * -rng.random_range(0.0..roof.size.z * 0.1));
            let facade = Facade::random(&tier, 0.0, floor, rng);
            parent.spawn(kit.cuboid(&tier, facades.add(facade)));
            parent.spawn(kit.cuboid(&Self::roof_edge(&tier), ledge.clone()));
            self.volumes.push(tier);
            roof = tier;
        }
        roof
    }

    /// The building the shop is in: floors above it and rooms behind it.
    fn spawn_shop_building(
        &mut self,
        commands: &mut Commands,
        kit: &Kit,
        facades: &mut Assets<FacadeMaterial>,
        rng: &mut impl Rng,
    ) {
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
        self.volumes.extend(parts);
        for part in parts {
            commands.spawn((
                Name::new("shop building"),
                kit.cuboid(
                    &part,
                    facades.add(Facade::random(&part, 0.0, SHOP_FLOOR, rng)),
                ),
            ));
        }
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
