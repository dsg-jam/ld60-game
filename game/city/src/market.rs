use std::ops::Range;

use bevy::prelude::*;
use ld60_game_core::layout::shop;
use ld60_game_core::parts::script::Word;
use ld60_geometry::bar::Bar;
use ld60_geometry::block::Block;
use ld60_geometry::surface::{Finish, SurfaceMaterial};
use rand::{Rng, RngExt};

use super::drips::DripEdge;
use super::signs::Tube;
use super::{Kit, Pick, glow, turned_to};

/// Height of the market floor, where the building above starts. The same
/// as our own shop.
pub(super) const MARKET_HEIGHT: f32 = shop::SIZE.y;
/// How far the stalls reach into the building.
pub(super) const STALL_DEPTH: f32 = 2.4;
/// The stall floors are a little above the sidewalk.
const STEP: f32 = 0.03;
const BAY_WIDTH: Range<f32> = 1.8..3.6;
/// Leftovers narrower than this become part of the last bay.
const MIN_BAY: f32 = 1.2;
const PILLAR: f32 = 0.3;
const PARTITION: f32 = 0.06;

const COUNTER_HEIGHT: f32 = 0.95;
const COUNTER_DEPTH: f32 = 0.5;
/// Space between the street edge of the bay and the counter.
const COUNTER_SETBACK: f32 = 0.4;
const SHELF_DEPTH: f32 = 0.45;
const SHELF_BOARDS: [f32; 4] = [0.9, 1.5, 2.1, 2.7];
const BOARD: f32 = 0.03;

const AWNING_CHANCE: f64 = 0.65;
const STOOL_CHANCE: f64 = 0.25;
const STOOL_HEIGHT: f32 = 0.75;
/// How far the stools stand out on the sidewalk.
const STOOL_OUT: f32 = 0.45;
const AWNING_REACH: f32 = 1.4;
const AWNING_DROP: f32 = 0.45;
/// How far above the opening the awning is fixed to the facade.
pub(super) const AWNING_ABOVE: f32 = 0.3;
const STRUT: f32 = 0.03;
/// How far below the top of the opening the struts meet the pillars.
const STRUT_DROP: f32 = 0.7;
const SIGN_CHANCE: f64 = 0.6;
const SIGN_HEIGHT: f32 = 0.45;
const SIGN_GLOW: f32 = 4.0;
const SHUTTER_BOX: f32 = 0.3;
const SHUTTER_RIBS: usize = 10;

const LAMP_GLOW: f32 = 25.0;
const LAMP_INTENSITY: f32 = 6_000.0;
const LAMP_RANGE: f32 = 6.0;
/// Only stalls this close to the shop light up their surroundings, to keep
/// the number of lights down.
const LIT_RADIUS: f32 = 22.0;

const CONCRETE: Color = Color::srgb_u8(0x4a, 0x46, 0x44);
const TILES: Color = Color::srgb_u8(0x3c, 0x38, 0x36);
const METAL: Color = Color::srgb_u8(0x50, 0x54, 0x58);
const INK: Color = Color::srgb_u8(0x18, 0x16, 0x1a);
const COUNTERS: [Color; 5] = [
    Color::srgb_u8(0x6a, 0x4e, 0x36),
    Color::srgb_u8(0x3e, 0x5a, 0x58),
    Color::srgb_u8(0x7a, 0x3a, 0x30),
    Color::srgb_u8(0x55, 0x55, 0x5c),
    Color::srgb_u8(0x8a, 0x7a, 0x50),
];
const GOODS: [Color; 8] = [
    Color::srgb_u8(0xc8, 0x40, 0x40),
    Color::srgb_u8(0xe0, 0xb0, 0x40),
    Color::srgb_u8(0x40, 0x90, 0xc0),
    Color::srgb_u8(0x60, 0xb0, 0x60),
    Color::srgb_u8(0xd0, 0xd0, 0xc8),
    Color::srgb_u8(0x8a, 0x60, 0xb0),
    Color::srgb_u8(0x9a, 0x7a, 0x5a),
    Color::srgb_u8(0x30, 0x30, 0x34),
];
const AWNINGS: [Color; 5] = [
    Color::srgb_u8(0xc0, 0x44, 0x3c),
    Color::srgb_u8(0x3a, 0x8a, 0x84),
    Color::srgb_u8(0xd0, 0xa8, 0x40),
    Color::srgb_u8(0x70, 0x50, 0x90),
    Color::srgb_u8(0x8a, 0x90, 0x50),
];
const LAMPS: [Color; 5] = [
    Color::srgb_u8(0xff, 0xd8, 0xa0),
    Color::srgb_u8(0xe0, 0xf0, 0xff),
    Color::srgb_u8(0xff, 0x60, 0xb0),
    Color::srgb_u8(0x60, 0xe0, 0xff),
    Color::srgb_u8(0xa0, 0xff, 0x80),
];
const LIGHTBOXES: [Color; 4] = [
    Color::srgb_u8(0xff, 0xf0, 0xd0),
    Color::srgb_u8(0xff, 0xd0, 0x40),
    Color::srgb_u8(0x80, 0xe0, 0xff),
    Color::srgb_u8(0xff, 0x90, 0xc0),
];

/// Materials shared by all market stalls.
pub(super) struct Stock {
    concrete: Handle<SurfaceMaterial>,
    tiles: Handle<SurfaceMaterial>,
    metal: Handle<SurfaceMaterial>,
    ink: Handle<SurfaceMaterial>,
    counters: Vec<Handle<SurfaceMaterial>>,
    goods: Vec<Handle<SurfaceMaterial>>,
    awnings: Vec<Handle<SurfaceMaterial>>,
    lamps: Vec<(Color, Handle<StandardMaterial>)>,
    lightboxes: Vec<Handle<StandardMaterial>>,
}

/// What a market bay is used for.
#[derive(Clone, Copy, Debug)]
enum Bay {
    Open,
    /// Open, with the shutter partly down.
    HalfShut,
    Shut,
    /// Empty and unlit.
    Dark,
}

impl Stock {
    pub(super) fn new(
        materials: &mut Assets<StandardMaterial>,
        surfaces: &mut Assets<SurfaceMaterial>,
    ) -> Self {
        let mut all = |colors: &[Color], finish: Finish| -> Vec<_> {
            colors
                .iter()
                .map(|&color| surfaces.add(finish.scaled(color)))
                .collect()
        };
        let counters = all(&COUNTERS, Finish::Paint);
        let goods = all(&GOODS, Finish::Plastic);
        let awnings = all(&AWNINGS, Finish::Fabric);
        Self {
            concrete: surfaces.add(Finish::Concrete.scaled(CONCRETE)),
            tiles: surfaces.add(Finish::Ceramic.scaled(TILES)),
            metal: surfaces.add(Finish::Metal.scaled(METAL)),
            ink: surfaces.add(Finish::Paint.scaled(INK)),
            counters,
            goods,
            awnings,
            lamps: LAMPS
                .iter()
                .map(|&color| (color, materials.add(glow(color, LAMP_GLOW))))
                .collect(),
            lightboxes: LIGHTBOXES
                .iter()
                .map(|&color| materials.add(glow(color, SIGN_GLOW)))
                .collect(),
        }
    }

    /// The market in the ground floor of `building`, whose front looks
    /// along Z in the direction of `facing`.
    pub(super) fn spawn_market(
        &self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        building: &Block,
        facing: f32,
        drips: &mut Vec<DripEdge>,
        rng: &mut impl Rng,
    ) {
        let front = if facing > 0.0 {
            building.max().z
        } else {
            building.min().z
        };
        let back = front - facing * STALL_DEPTH;
        let (start, end) = (building.min().x, building.max().x);
        // Walls at both ends, between the pillars and the back of the stalls.
        for x in [start, end - PILLAR] {
            let wall = Block::between(
                Vec3::new(x, 0.0, front),
                Vec3::new(x + PILLAR, MARKET_HEIGHT, back),
            );
            parent.spawn(kit.cuboid(&wall, self.concrete.clone()));
        }

        let mut x = start + PILLAR;
        let last = end - PILLAR;
        while x < last {
            let mut width = rng.random_range(BAY_WIDTH);
            if last - (x + width) < MIN_BAY {
                width = last - x;
            }
            let center = Vec3::new(x + width / 2.0, 0.0, front);
            let rotation = turned_to(facing);
            let lit = center.x.abs() < LIT_RADIUS;
            let bay = Bay::random(rng);
            let place = Transform::from_translation(center).with_rotation(rotation);
            let mut edge = None;
            parent
                .spawn((place, Visibility::default()))
                .with_children(|stall| edge = bay.spawn(stall, kit, self, width, lit, rng));
            if let Some(edge) = edge {
                drips.push(DripEdge {
                    from: place.transform_point(edge.from),
                    to: place.transform_point(edge.to),
                    ..edge
                });
            }
            x += width;
            if x < last {
                // A pillar at the front and a thin wall to the next bay.
                let pillar = Block::between(
                    Vec3::new(x - PILLAR / 2.0, 0.0, front),
                    Vec3::new(x + PILLAR / 2.0, MARKET_HEIGHT, front - facing * PILLAR),
                );
                let partition = Block::between(
                    Vec3::new(x - PARTITION / 2.0, 0.0, front - facing * PILLAR),
                    Vec3::new(x + PARTITION / 2.0, MARKET_HEIGHT, back),
                );
                parent.spawn(kit.cuboid(&pillar, self.concrete.clone()));
                parent.spawn(kit.cuboid(&partition, self.concrete.clone()));
            }
        }
    }

    /// The plain wall at the back of the stalls.
    pub(super) fn wall(&self) -> Handle<SurfaceMaterial> {
        self.concrete.clone()
    }
}

impl Bay {
    fn random(rng: &mut impl Rng) -> Self {
        match rng.random_range(0.0..1.0) {
            x if x < 0.5 => Self::Open,
            x if x < 0.65 => Self::HalfShut,
            x if x < 0.88 => Self::Shut,
            _ => Self::Dark,
        }
    }

    /// The stall in local space: the street lies towards +Z, the bay spans
    /// `width` along X and reaches back to -[`STALL_DEPTH`]. Returns the
    /// dripping edge of its awning, if it has one.
    fn spawn(
        self,
        stall: &mut ChildSpawnerCommands,
        kit: &Kit,
        stock: &Stock,
        width: f32,
        lit: bool,
        rng: &mut impl Rng,
    ) -> Option<DripEdge> {
        let half = width / 2.0;
        let floor = Block::between(
            Vec3::new(-half, -0.1, -STALL_DEPTH),
            Vec3::new(half, STEP, 0.0),
        );
        stall.spawn(kit.cuboid(&floor, stock.tiles.clone()));
        match self {
            Self::Open => Self::spawn_stall(stall, kit, stock, width, lit, rng),
            Self::HalfShut => {
                let edge = Self::spawn_stall(stall, kit, stock, width, lit, rng);
                let down = rng.random_range(0.4..1.2);
                Self::spawn_shutter(stall, kit, stock, width, MARKET_HEIGHT - SHUTTER_BOX - down);
                edge
            }
            Self::Shut => {
                Self::spawn_shutter(stall, kit, stock, width, 0.0);
                None
            }
            Self::Dark => {
                Self::spawn_crates(stall, kit, stock, width, rng);
                None
            }
        }
    }

    fn spawn_stall(
        stall: &mut ChildSpawnerCommands,
        kit: &Kit,
        stock: &Stock,
        width: f32,
        lit: bool,
        rng: &mut impl Rng,
    ) -> Option<DripEdge> {
        let half = width / 2.0;
        let inner = half - 0.2;
        let counter = Block::between(
            Vec3::new(-inner, 0.0, -COUNTER_SETBACK - COUNTER_DEPTH),
            Vec3::new(inner, COUNTER_HEIGHT, -COUNTER_SETBACK),
        );
        let top = Block::new(counter.size.with_y(0.04) + Vec3::new(0.06, 0.0, 0.06))
            .on_top_of(&counter)
            .translated(Vec3::new(0.0, 0.0, counter.center.z));
        stall.spawn(kit.cuboid(&counter, stock.counters.pick(rng).clone()));
        stall.spawn(kit.cuboid(&top, stock.metal.clone()));
        Self::spawn_goods(stall, kit, stock, &top, rng);

        for &height in &SHELF_BOARDS {
            let board = Block::between(
                Vec3::new(-inner, height - BOARD, -STALL_DEPTH),
                Vec3::new(inner, height, -STALL_DEPTH + SHELF_DEPTH),
            );
            stall.spawn(kit.cuboid(&board, stock.metal.clone()));
            Self::spawn_goods(stall, kit, stock, &board, rng);
        }

        // A lamp hanging from the ceiling on its cord.
        let (color, lamp_material) = stock.lamps.pick(rng);
        let at = Vec3::new(rng.random_range(-inner * 0.5..=inner * 0.5), 0.0, -1.3);
        let drop = rng.random_range(0.3..0.6);
        let lamp =
            Block::new(Vec3::new(0.25, 0.08, 0.25)).translated(at.with_y(MARKET_HEIGHT - drop));
        let cord = Block::between(
            at.with_y(lamp.max().y) - Vec3::new(0.01, 0.0, 0.01),
            at.with_y(MARKET_HEIGHT) + Vec3::new(0.01, 0.0, 0.01),
        );
        stall.spawn(kit.cuboid(&lamp, lamp_material.clone()));
        stall.spawn(kit.cuboid(&cord, stock.ink.clone()));
        if lit {
            stall.spawn((
                PointLight {
                    color: *color,
                    intensity: LAMP_INTENSITY,
                    range: LAMP_RANGE,
                    ..default()
                },
                Transform::from_translation(lamp.bottom() - Vec3::Y * 0.05),
            ));
        }

        if rng.random_bool(STOOL_CHANCE) {
            // A couple of stools on the sidewalk, for customers who stay.
            for x in [-inner * 0.5, inner * 0.5] {
                let at = Vec3::new(x, 0.0, STOOL_OUT);
                let leg = Block::new(Vec3::new(0.05, STOOL_HEIGHT - 0.05, 0.05)).translated(at);
                let seat = Block::new(Vec3::new(0.32, 0.05, 0.32))
                    .translated(at)
                    .on_top_of(&leg);
                stall.spawn(kit.cuboid(&leg, stock.metal.clone()));
                stall.spawn(kit.cuboid(&seat, stock.awnings.pick(rng).clone()));
            }
        }

        if rng.random_bool(SIGN_CHANCE) {
            Self::spawn_sign(stall, kit, stock, width, rng);
        }
        if !rng.random_bool(AWNING_CHANCE) {
            return None;
        }
        // Fixed to the facade above the opening, over the sign.
        let top = MARKET_HEIGHT + AWNING_ABOVE;
        let awning = Bar {
            from: Vec3::new(0.0, top, 0.0),
            to: Vec3::new(0.0, top - AWNING_DROP, AWNING_REACH),
            section: Vec2::new(width - 0.1, 0.04),
            up: Vec3::Y,
        };
        stall.spawn(kit.bar(&awning, stock.awnings.pick(rng).clone()));
        let lip = Vec3::new(0.0, top - AWNING_DROP, AWNING_REACH);
        let reach = Vec3::X * (width / 2.0 - 0.1);
        // A light strip under the edge, in the color of the stall's lamp.
        let strip = Block::between(
            lip - reach - Vec3::new(0.0, 0.05, 0.06),
            lip + reach - Vec3::new(0.0, 0.02, 0.02),
        );
        stall.spawn(kit.cuboid(&strip, lamp_material.clone()));
        // Struts from the outer corners back to the pillars.
        for side in [-1.0, 1.0] {
            let on_awning = lip + reach * side * 0.95 - Vec3::Y * 0.03;
            let on_pillar = Vec3::new(side * width / 2.0, MARKET_HEIGHT - STRUT_DROP, 0.0);
            stall.spawn(kit.bar(
                &Bar::square(on_pillar, on_awning, STRUT),
                stock.metal.clone(),
            ));
        }
        Some(DripEdge {
            from: lip - reach,
            to: lip + reach,
            light: *color,
        })
    }

    /// Small boxes standing in a row on top of `surface`.
    fn spawn_goods(
        stall: &mut ChildSpawnerCommands,
        kit: &Kit,
        stock: &Stock,
        surface: &Block,
        rng: &mut impl Rng,
    ) {
        let mut x = surface.min().x + 0.03;
        while x < surface.max().x - 0.1 {
            let size = Vec3::new(
                rng.random_range(0.08..0.3),
                rng.random_range(0.08..0.4),
                rng.random_range(0.1..surface.size.z.min(0.4)),
            );
            if x + size.x > surface.max().x {
                break;
            }
            if rng.random_bool(0.8) {
                let item = Block::new(size).on_top_of(surface).translated(Vec3::new(
                    x + size.x / 2.0,
                    0.0,
                    surface.center.z,
                ));
                stall.spawn(kit.cuboid(&item, stock.goods.pick(rng).clone()));
            }
            x += size.x + rng.random_range(0.01..0.08);
        }
    }

    /// A lightbox with the stall's name, hanging at the top of the opening.
    fn spawn_sign(
        stall: &mut ChildSpawnerCommands,
        kit: &Kit,
        stock: &Stock,
        width: f32,
        rng: &mut impl Rng,
    ) {
        let size = Vec2::new(width * 0.8, SIGN_HEIGHT);
        let lightbox = Block {
            center: Vec3::new(0.0, MARKET_HEIGHT - 0.1 - SIGN_HEIGHT / 2.0, 0.05),
            size: size.extend(0.1),
        };
        stall.spawn(kit.cuboid(&lightbox, stock.lightboxes.pick(rng).clone()));
        let word = Word::random(rng, 2..6);
        for stroke in Tube::lettering(&word, size, false) {
            let local = stroke.transform(lightbox.size.z / 2.0);
            stall.spawn((
                Mesh3d(kit.cube.clone()),
                MeshMaterial3d(stock.ink.clone()),
                Transform::from_translation(lightbox.center) * local,
            ));
        }
    }

    /// A roller shutter down to `bottom`, with its box at the top.
    fn spawn_shutter(
        stall: &mut ChildSpawnerCommands,
        kit: &Kit,
        stock: &Stock,
        width: f32,
        bottom: f32,
    ) {
        let half = width / 2.0;
        let top = MARKET_HEIGHT - SHUTTER_BOX;
        let roll = Block::between(
            Vec3::new(-half, top, -0.35),
            Vec3::new(half, MARKET_HEIGHT, -0.05),
        );
        let panel = Block::between(Vec3::new(-half, bottom, -0.14), Vec3::new(half, top, -0.1));
        stall.spawn(kit.cuboid(&roll, stock.metal.clone()));
        stall.spawn(kit.cuboid(&panel, stock.metal.clone()));
        let pitch = (top - bottom) / SHUTTER_RIBS as f32;
        for index in 0..SHUTTER_RIBS {
            let y = bottom + pitch * (index as f32 + 0.5);
            let rib = Block::between(Vec3::new(-half, y, -0.1), Vec3::new(half, y + 0.03, -0.08));
            stall.spawn(kit.cuboid(&rib, stock.ink.clone()));
        }
    }

    /// Stacked crates in an empty bay.
    fn spawn_crates(
        stall: &mut ChildSpawnerCommands,
        kit: &Kit,
        stock: &Stock,
        width: f32,
        rng: &mut impl Rng,
    ) {
        let half = (width / 2.0 - 0.5).max(0.0);
        for _ in 0..rng.random_range(1..5) {
            let size = Vec3::splat(rng.random_range(0.4..0.7));
            let at = Vec3::new(
                rng.random_range(-half..=half),
                0.0,
                rng.random_range(-STALL_DEPTH + 0.5..-0.5),
            );
            let crate_ = Block::new(size).translated(at);
            stall.spawn(kit.cuboid(&crate_, stock.counters.pick(rng).clone()));
        }
    }
}
