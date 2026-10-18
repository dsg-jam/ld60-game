use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::{Additions, Wall};
use crate::city::Kit;
use crate::geometry::block::Block;

pub(super) const SCAFFOLD_CHANCE: f64 = 0.15;
const BAYS: Range<usize> = 2..6;
const BAY: f32 = 2.0;
const LIFTS: Range<usize> = 2..7;
const LIFT: f32 = 2.0;
/// From the wall to the inner poles, and between the inner and outer poles.
const GAP: f32 = 0.25;
const DEPTH: f32 = 0.8;
const POLE: f32 = 0.05;
const BOARD: f32 = 0.04;
const GUARD: f32 = 1.0;
const NETTING_CHANCE: f64 = 0.4;

impl Additions {
    /// Scaffolding for work on the wall that never seems to end, standing
    /// on the street or hung from the floors above the shops.
    pub(super) fn spawn_scaffold(
        &self,
        parent: &mut ChildSpawnerCommands,
        kit: &Kit,
        wall: &Wall,
        rng: &mut impl Rng,
    ) {
        let building = &wall.building;
        let bays = rng.random_range(BAYS).min((building.size.x / BAY) as usize);
        let lifts = rng
            .random_range(LIFTS)
            .min(((building.max().y - wall.footing - GUARD) / LIFT) as usize);
        if bays == 0 || lifts == 0 {
            return;
        }
        let width = bays as f32 * BAY;
        let left = building.min().x + rng.random_range(0.0..=building.size.x - width);
        let top = wall.footing + lifts as f32 * LIFT;
        let inner = GAP;
        let outer = GAP + DEPTH;
        // Poles reach past their ends, so the joints close.
        let bar = |parent: &mut ChildSpawnerCommands, a: Vec3, b: Vec3| {
            let (a, b) = (wall.at(a), wall.at(b));
            let past = (b - a).normalize_or_zero() * POLE / 2.0;
            parent.spawn(kit.bar(a - past, b + past, POLE, self.scaffold.clone()));
        };

        for bay in 0..=bays {
            let x = left + bay as f32 * BAY;
            for z in [inner, outer] {
                bar(
                    parent,
                    Vec3::new(x, wall.footing, z),
                    Vec3::new(x, top + GUARD, z),
                );
            }
        }
        for lift in 1..=lifts {
            let y = wall.footing + lift as f32 * LIFT;
            for z in [inner, outer] {
                bar(parent, Vec3::new(left, y, z), Vec3::new(left + width, y, z));
            }
            for bay in 0..=bays {
                let x = left + bay as f32 * BAY;
                bar(parent, Vec3::new(x, y, inner), Vec3::new(x, y, outer));
            }
            let boards = Block::between(
                wall.at(Vec3::new(left, y + POLE / 2.0, inner)),
                wall.at(Vec3::new(left + width, y + POLE / 2.0 + BOARD, outer)),
            );
            parent.spawn(kit.cuboid(&boards, self.planks.clone()));
            // A rail to hold on to, half a lift up.
            let rail = y + GUARD;
            bar(
                parent,
                Vec3::new(left, rail, outer),
                Vec3::new(left + width, rail, outer),
            );
        }
        // Braces across the outside, zigzagging up.
        for lift in 0..lifts {
            let y = wall.footing + lift as f32 * LIFT;
            let bay = lift % bays;
            let (from, to) = if lift % 2 == 0 {
                (0.0, BAY)
            } else {
                (BAY, 0.0)
            };
            let x = left + bay as f32 * BAY;
            bar(
                parent,
                Vec3::new(x + from, y, outer),
                Vec3::new(x + to, y + LIFT, outer),
            );
        }
        if rng.random_bool(NETTING_CHANCE) {
            let net = Block::between(
                wall.at(Vec3::new(left, wall.footing + 0.3, outer + POLE)),
                wall.at(Vec3::new(left + width, top + GUARD, outer + POLE + 0.01)),
            );
            parent.spawn(kit.cuboid(&net, self.netting.clone()));
        }
    }
}
