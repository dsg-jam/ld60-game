use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::{Additions, Wall};
use crate::city::Kit;
use crate::geometry::block::Block;

/// Thickness of the pipes running along a wall.
const PIPE: f32 = 0.12;
const DRAIN: f32 = 0.1;
const DRAIN_CHANCE: f64 = 0.7;
const BRACKET_SPACING: f32 = 1.8;
/// How far pipes and cables stand off the wall.
const OFF_WALL: f32 = 0.05;
const PIPE_RUN_CHANCE: f64 = 0.5;
const PIPE_RUN_CELLS: Range<i32> = 2..6;
const BUNDLES: Range<usize> = 0..4;
const BUNDLE_WIRES: Range<usize> = 1..5;
const WIRE: f32 = 0.018;
const WIRE_GAP: f32 = 0.03;

impl Additions {
    /// Drain pipes, water pipes and cables that were run along the outside
    /// of the wall, as that was easier than through it.
    pub(super) fn spawn_services(
        &self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        wall: &Wall,
        rng: &mut impl Rng,
    ) {
        if rng.random_bool(DRAIN_CHANCE) {
            self.spawn_drain(parent, kit, wall, rng);
        }
        if rng.random_bool(PIPE_RUN_CHANCE) {
            self.spawn_pipe_run(parent, kit, wall, rng);
        }
        for _ in 0..rng.random_range(BUNDLES) {
            self.spawn_bundle(parent, kit, wall, rng);
        }
    }

    /// From the roof down one corner of the wall, held by brackets.
    fn spawn_drain(
        &self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        wall: &Wall,
        rng: &mut impl Rng,
    ) {
        let building = &wall.building;
        let x = if rng.random_bool(0.5) {
            building.min().x + 0.2
        } else {
            building.max().x - 0.2 - DRAIN
        };
        let top = building.max().y - 0.3;
        let pipe = Block::between(
            wall.at(Vec3::new(x, wall.bottom, OFF_WALL)),
            wall.at(Vec3::new(x + DRAIN, top, OFF_WALL + DRAIN)),
        );
        // The outlet from the gutter, bending back into the wall.
        let outlet = Block::between(
            wall.at(Vec3::new(x, top - DRAIN, 0.0)),
            wall.at(Vec3::new(x + DRAIN, top, OFF_WALL)),
        );
        parent.spawn(kit.cuboid(&pipe, self.drain.clone()));
        parent.spawn(kit.cuboid(&outlet, self.drain.clone()));
        let mut y = wall.bottom + BRACKET_SPACING / 2.0;
        while y < top {
            let bracket = Block::between(
                wall.at(Vec3::new(x - 0.02, y, 0.0)),
                wall.at(Vec3::new(
                    x + DRAIN + 0.02,
                    y + 0.04,
                    OFF_WALL + DRAIN + 0.01,
                )),
            );
            parent.spawn(kit.cuboid(&bracket, self.rail.clone()));
            y += BRACKET_SPACING;
        }
    }

    /// Along a floor for a few windows, then down to the street.
    fn spawn_pipe_run(
        &self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        wall: &Wall,
        rng: &mut impl Rng,
    ) {
        if wall.rows.is_empty() || wall.columns.is_empty() {
            return;
        }
        let row = rng.random_range(wall.rows.clone());
        let cells = rng.random_range(PIPE_RUN_CELLS);
        let first = rng.random_range(wall.columns.clone());
        let last = (first + cells).min(wall.columns.end);
        let y = wall.corner(0, row).y + 0.15;
        let from = wall.corner(first, row).x;
        let to = wall.corner(last, row).x;
        let size = PIPE;
        let run = Block::between(
            wall.at(Vec3::new(from, y, OFF_WALL)),
            wall.at(Vec3::new(to, y + size, OFF_WALL + size)),
        );
        let down = Block::between(
            wall.at(Vec3::new(to - size, wall.bottom, OFF_WALL)),
            wall.at(Vec3::new(to, y, OFF_WALL + size)),
        );
        parent.spawn(kit.cuboid(&run, self.rust.clone()));
        parent.spawn(kit.cuboid(&down, self.rust.clone()));
    }

    /// A few cables up a pier and along to a window, for a dish or a shop
    /// sign someone put up.
    fn spawn_bundle(
        &self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        wall: &Wall,
        rng: &mut impl Rng,
    ) {
        if wall.rows.is_empty() || wall.columns.is_empty() {
            return;
        }
        let column = rng.random_range(wall.columns.clone());
        let row = rng.random_range(wall.rows.clone());
        let pier = wall.corner(column, row).x;
        let top = wall.corner(column, row).y + wall.cell.y * rng.random_range(0.3..0.9);
        let across = wall.cell.x * rng.random_range(-0.6..0.6);
        for index in 0..rng.random_range(BUNDLE_WIRES) {
            let offset = index as f32 * WIRE_GAP;
            let out = OFF_WALL * 0.5 + rng.random_range(0.0..0.02);
            let x = pier + offset;
            let y = top - offset;
            let up = Block::between(
                wall.at(Vec3::new(x, wall.bottom, out)),
                wall.at(Vec3::new(x + WIRE, y, out + WIRE)),
            );
            let along = Block::between(
                wall.at(Vec3::new(x.min(x + across), y, out)),
                wall.at(Vec3::new(x.max(x + across) + WIRE, y + WIRE, out + WIRE)),
            );
            parent.spawn(kit.cuboid(&up, self.wire.clone()));
            parent.spawn(kit.cuboid(&along, self.wire.clone()));
        }
    }
}
