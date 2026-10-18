mod scaffold;
mod services;

use std::collections::HashSet;
use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::buildings::{Facade, FacadeMaterial, WindowGrid};
use super::market::MARKET_HEIGHT;
use super::{Kit, Pick, glow};
use crate::geometry::bar::Bar;
use crate::geometry::block::Block;
use crate::geometry::surface::{Finish, SurfaceMaterial};

/// Additions start above the shop signs.
const LOWEST: f32 = MARKET_HEIGHT + 2.4;
/// Additions get rarer higher up, by a factor of e every this many meters.
const FALLOFF: f32 = 60.0;

/// Chances that a window cell near the bottom gets a pod, a balcony or an
/// air conditioner.
const POD_CHANCE: f64 = 0.12;
const BALCONY_CHANCE: f64 = 0.15;
const AC_CHANCE: f64 = 0.45;
const POD_DEPTH: Range<f32> = 0.9..1.8;
const STRUT: f32 = 0.08;
const BALCONY_DEPTH: f32 = 0.9;
const RAIL_HEIGHT: f32 = 1.0;
const AC_SIZE: Vec3 = Vec3::new(0.8, 0.5, 0.35);
const LED_CHANCE: f64 = 0.4;
const LED_GLOW: f32 = 20.0;
const LAUNDRY_CHANCE: f64 = 0.4;
const LAMP_CHANCE: f64 = 0.3;
/// Towels and rugs hung out of a window to dry.
const TOWEL_CHANCE: f64 = 0.08;
const TOWEL_WIDTH: Range<f32> = 0.35..0.6;
const TOWEL_LENGTH: Range<f32> = 0.4..0.9;

const LAMP_GLOW: f32 = 15.0;

const RAIL: Color = Color::srgb_u8(0x30, 0x32, 0x36);
const DRAIN: Color = Color::srgb_u8(0x5a, 0x5c, 0x5e);
const SCAFFOLD: Color = Color::srgb_u8(0x8a, 0x8c, 0x90);
const PLANKS: Color = Color::srgb_u8(0x6a, 0x54, 0x34);
const NETTING: Color = Color::srgba(0.12, 0.38, 0.24, 0.6);
const RUST: Color = Color::srgb_u8(0x6a, 0x4a, 0x3a);
const UNIT: Color = Color::srgb_u8(0xc8, 0xc6, 0xbc);
const LAUNDRY: [Color; 4] = [
    Color::srgb_u8(0xd8, 0xd4, 0xc8),
    Color::srgb_u8(0x6a, 0x8a, 0xb0),
    Color::srgb_u8(0xb0, 0x50, 0x48),
    Color::srgb_u8(0xd0, 0xb0, 0x50),
];
const LED_GREEN: Color = Color::srgb_u8(0x40, 0xff, 0x60);
const LED_RED: Color = Color::srgb_u8(0xff, 0x30, 0x20);
const PLANT: Color = Color::srgb_u8(0x3a, 0x5a, 0x34);
const WIRE: Color = Color::srgb_u8(0x12, 0x12, 0x14);
const LAMP: Color = Color::srgb_u8(0xff, 0xd0, 0x80);

/// The street side of a building that additions go on.
struct Wall {
    building: Block,
    /// Z of the wall, and the direction it faces along Z.
    front: f32,
    facing: f32,
    /// The window grid, starting at the building's corner.
    cell: Vec2,
    /// Where pipes and cables come out of the ground or the shops below.
    bottom: f32,
    /// Where scaffolding stands: on the street, or above the shop signs.
    footing: f32,
    /// Floors and window columns that may have additions.
    rows: Range<i32>,
    columns: Range<i32>,
}

impl Wall {
    /// Lower left corner of a window cell.
    fn corner(&self, column: i32, row: i32) -> Vec2 {
        self.building.min().xy() + Vec2::new(column as f32, row as f32) * self.cell
    }

    /// From `x`, `y` and the distance out from the wall to the world.
    fn at(&self, point: Vec3) -> Vec3 {
        point.with_z(self.front + self.facing * point.z)
    }
}

/// Materials shared by the makeshift additions to the buildings.
pub(super) struct Additions {
    rail: Handle<SurfaceMaterial>,
    rust: Handle<SurfaceMaterial>,
    drain: Handle<SurfaceMaterial>,
    scaffold: Handle<SurfaceMaterial>,
    planks: Handle<SurfaceMaterial>,
    netting: Handle<StandardMaterial>,
    unit: Handle<SurfaceMaterial>,
    plant: Handle<SurfaceMaterial>,
    wire: Handle<SurfaceMaterial>,
    laundry: Vec<Handle<SurfaceMaterial>>,
    leds: [Handle<StandardMaterial>; 2],
    lamp: Handle<StandardMaterial>,
}

impl Additions {
    pub(super) fn new(
        materials: &mut Assets<StandardMaterial>,
        surfaces: &mut Assets<SurfaceMaterial>,
    ) -> Self {
        Self {
            rail: surfaces.add(Finish::Paint.scaled(RAIL)),
            rust: surfaces.add(Finish::Rust.scaled(RUST)),
            drain: surfaces.add(Finish::Plastic.scaled(DRAIN)),
            scaffold: surfaces.add(Finish::Metal.scaled(SCAFFOLD)),
            planks: surfaces.add(Finish::Wood.scaled(PLANKS)),
            netting: materials.add(StandardMaterial {
                base_color: NETTING,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 1.0,
                double_sided: true,
                cull_mode: None,
                ..default()
            }),
            unit: surfaces.add(Finish::Paint.scaled(UNIT)),
            plant: surfaces.add(Finish::Fabric.scaled(PLANT)),
            wire: surfaces.add(Finish::Plastic.scaled(WIRE)),
            laundry: LAUNDRY
                .iter()
                .map(|&color| surfaces.add(Finish::Fabric.scaled(color)))
                .collect(),
            leds: [
                materials.add(glow(LED_GREEN, LED_GLOW)),
                materials.add(glow(LED_RED, LED_GLOW)),
            ],
            lamp: materials.add(glow(LAMP, LAMP_GLOW)),
        }
    }

    /// Living space people added to the front of `building` over the
    /// years, fitted to its window `grid`. Most of it near the bottom, where
    /// it was easy to reach.
    #[expect(
        clippy::too_many_arguments,
        reason = "all of it goes into one building"
    )]
    pub(super) fn spawn_on(
        &self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        facades: &mut Assets<FacadeMaterial>,
        building: &Block,
        grid: WindowGrid,
        facing: f32,
        rng: &mut impl Rng,
    ) {
        let front = if facing > 0.0 {
            building.max().z
        } else {
            building.min().z
        };
        let cell = grid.cell;
        let columns = (building.size.x / cell.x) as i32;
        let rows = (building.size.y / cell.y) as i32;
        let first_row = ((LOWEST - building.min().y) / cell.y).ceil().max(0.0) as i32;
        // Leave the top floor and the roof edge free.
        let last_row = rows - 1;
        if columns <= 0 || last_row <= first_row {
            return;
        }
        let wall = Wall {
            building: *building,
            front,
            facing,
            cell,
            bottom: building.min().y,
            footing: if building.min().y > 0.0 { LOWEST } else { 0.0 },
            rows: first_row..last_row,
            columns: 0..columns,
        };
        let out = |block: Block| block.translated(Vec3::Z * facing * block.size.z / 2.0);
        let mut taken = HashSet::new();
        // Each building has its own habit of where the air conditioners go.
        let under_sill = rng.random_bool(0.5);
        let sill = cell.y * (1.0 - grid.window.y) / 2.0;
        let pier = cell.x * (1.0 - grid.window.x);

        for row in first_row..last_row {
            let above = wall.corner(0, row).y - LOWEST;
            let chance = (-above / FALLOFF).exp();
            for column in 0..columns {
                if taken.contains(&(column, row)) {
                    continue;
                }
                let span = if column + 1 < columns
                    && !taken.contains(&(column + 1, row))
                    && rng.random_bool(0.4)
                {
                    2
                } else {
                    1
                };
                let roll = rng.random::<f64>() / f64::from(chance);
                let at = wall.corner(column, row);
                if roll < POD_CHANCE {
                    for next in column..column + span {
                        taken.insert((next, row));
                    }
                    let size = Vec2::new(span as f32 * cell.x - 0.1, cell.y - 0.15);
                    let pod = out(Block {
                        center: (at + Vec2::new(span as f32 * cell.x, cell.y) / 2.0).extend(front),
                        size: size.extend(rng.random_range(POD_DEPTH)),
                    });
                    let facade = facades.add(Facade::random(&pod, 0.0, pod.size.y, rng));
                    parent.spawn(kit.cuboid(&pod, facade));
                    self.spawn_struts(parent, kit, &pod, front, facing);
                } else if roll < POD_CHANCE + BALCONY_CHANCE {
                    for next in column..column + span {
                        taken.insert((next, row));
                    }
                    let width = span as f32 * cell.x - 0.3;
                    let corner = Vec3::new(at.x + 0.15, at.y, front);
                    self.spawn_balcony(parent, kit, corner, width, facing, rng);
                } else if roll < POD_CHANCE + BALCONY_CHANCE + AC_CHANCE {
                    let center = if under_sill && sill > AC_SIZE.y + 0.15 {
                        Vec2::new(at.x + cell.x / 2.0, at.y + sill / 2.0)
                    } else if pier > AC_SIZE.x + 0.1 && column + 1 < columns {
                        Vec2::new(at.x + cell.x, at.y + cell.y / 2.0)
                    } else {
                        continue;
                    };
                    self.spawn_air_conditioner(parent, kit, center.extend(front), facing, rng);
                } else if roll < POD_CHANCE + BALCONY_CHANCE + AC_CHANCE + TOWEL_CHANCE {
                    let sill = Vec3::new(at.x + cell.x / 2.0, at.y + sill, front);
                    self.spawn_towel(parent, kit, sill, facing, rng);
                }
            }
        }

        self.spawn_services(parent, kit, &wall, rng);
        self.spawn_scaffold(parent, kit, &wall, rng);
    }

    /// A towel or a small rug hanging over a window sill.
    fn spawn_towel(
        &self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        sill: Vec3,
        facing: f32,
        rng: &mut impl Rng,
    ) {
        let width = rng.random_range(TOWEL_WIDTH);
        let length = rng.random_range(TOWEL_LENGTH);
        let x = sill.x + rng.random_range(-0.3..0.3);
        let towel = Block::between(
            Vec3::new(x - width / 2.0, sill.y - length, sill.z + facing * 0.02),
            Vec3::new(x + width / 2.0, sill.y + 0.03, sill.z + facing * 0.04),
        );
        parent.spawn(kit.cuboid(&towel, self.laundry.pick(rng).clone()));
    }

    /// Two diagonal struts holding a pod up from the wall below it.
    fn spawn_struts(
        &self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        pod: &Block,
        front: f32,
        facing: f32,
    ) {
        let reach = pod.size.z;
        for x in [pod.min().x + 0.3, pod.max().x - 0.3] {
            let on_pod = Vec3::new(x, pod.min().y, front + facing * reach * 0.9);
            let on_wall = Vec3::new(x, pod.min().y - reach, front);
            parent.spawn(kit.bar(&Bar::square(on_wall, on_pod, STRUT), self.rail.clone()));
        }
    }

    /// A box with a fan grille, some with a status light.
    fn spawn_air_conditioner(
        &self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        on_wall: Vec3,
        facing: f32,
        rng: &mut impl Rng,
    ) {
        let unit = Block {
            center: on_wall + Vec3::Z * facing * AC_SIZE.z / 2.0,
            size: AC_SIZE,
        };
        let face = unit.center.z + facing * AC_SIZE.z / 2.0;
        let grille = Block {
            center: unit.center.with_z(face) + Vec3::X * AC_SIZE.x * 0.15,
            size: Vec3::new(AC_SIZE.y * 0.8, AC_SIZE.y * 0.8, 0.02),
        };
        parent.spawn(kit.cuboid(&unit, self.unit.clone()));
        parent.spawn(kit.cuboid(&grille, self.wire.clone()));
        if rng.random_bool(LED_CHANCE) {
            let led = Block {
                center: unit.center.with_z(face)
                    + Vec3::new(-AC_SIZE.x * 0.38, AC_SIZE.y * 0.3, 0.0),
                size: Vec3::splat(0.04),
            };
            let color = if rng.random_bool(0.5) {
                &self.leds[0]
            } else {
                &self.leds[1]
            };
            parent.spawn(kit.cuboid(&led, color.clone()));
        }
    }

    fn spawn_balcony(
        &self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        corner: Vec3,
        width: f32,
        facing: f32,
        rng: &mut impl Rng,
    ) {
        let edge = corner.z + facing * BALCONY_DEPTH;
        let slab = Block::between(
            corner - Vec3::Y * 0.1,
            Vec3::new(corner.x + width, corner.y, edge),
        );
        let rail = Block::between(
            Vec3::new(corner.x, corner.y + RAIL_HEIGHT - 0.05, edge),
            Vec3::new(
                corner.x + width,
                corner.y + RAIL_HEIGHT,
                edge - facing * 0.05,
            ),
        );
        parent.spawn(kit.cuboid(&slab, self.rail.clone()));
        parent.spawn(kit.cuboid(&rail, self.rail.clone()));
        let posts = (width / 0.5) as usize;
        for index in 0..=posts {
            let x = corner.x + index as f32 * width / posts as f32;
            let post = Block::between(
                Vec3::new(x - 0.02, corner.y, edge),
                Vec3::new(x + 0.02, corner.y + RAIL_HEIGHT, edge - facing * 0.04),
            );
            parent.spawn(kit.cuboid(&post, self.rail.clone()));
        }
        if rng.random_bool(0.5) {
            let plant =
                Block::new(Vec3::new(0.4, rng.random_range(0.4..0.9), 0.4)).translated(Vec3::new(
                    corner.x + rng.random_range(0.3..width - 0.3),
                    corner.y,
                    corner.z + facing * 0.4,
                ));
            parent.spawn(kit.cuboid(&plant, self.plant.clone()));
        }
        if rng.random_bool(LAUNDRY_CHANCE) {
            // Clothes drying on a line under the top of the railing.
            let mut x = corner.x + 0.15;
            while x < corner.x + width - 0.5 {
                let size = Vec3::new(rng.random_range(0.3..0.5), rng.random_range(0.4..0.7), 0.02);
                let cloth = Block::between(
                    Vec3::new(x, corner.y + RAIL_HEIGHT - size.y, edge - facing * 0.15),
                    Vec3::new(x + size.x, corner.y + RAIL_HEIGHT, edge - facing * 0.17),
                );
                parent.spawn(kit.cuboid(&cloth, self.laundry.pick(rng).clone()));
                x += size.x + rng.random_range(0.05..0.2);
            }
        }
        if rng.random_bool(LAMP_CHANCE) {
            let lamp = Block::between(
                Vec3::new(corner.x + 0.1, corner.y + 2.0, corner.z),
                Vec3::new(corner.x + 0.3, corner.y + 2.12, corner.z + facing * 0.1),
            );
            parent.spawn(kit.cuboid(&lamp, self.lamp.clone()));
        }
    }
}
