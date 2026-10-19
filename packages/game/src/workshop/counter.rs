use bevy::prelude::*;

use super::shop::{FRONT, PILLAR, PLATFORM, SIZE};
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;

/// Height of the counter top above the shop floor.
const HEIGHT: f32 = 0.75;
const TOP_THICKNESS: f32 = 0.05;
const TOP_DEPTH: f32 = 0.85;
/// How far the top reaches over the counter wall towards the street.
const OVERHANG: f32 = 0.06;
const WALL: f32 = 0.06;
const SLAT: f32 = 0.12;
const SLAT_GAP: f32 = 0.008;
/// The part of the top where the player works and puts things.
const WORK_AREA: Vec2 = Vec2::new(1.6, 0.8);
const MAT_SIZE: Vec3 = Vec3::new(0.48, 0.004, 0.42);
/// Where the mat lies, see [`Block::top_spot`].
const MAT_SPOT: Vec2 = Vec2::new(0.0, 0.1);
/// A shelf under the top, on the player's side.
const SHELF_HEIGHT: f32 = 0.3;
const SHELF_DEPTH: f32 = 0.4;

const STOOL_HEIGHT: f32 = 0.75;
const STOOL_SEAT: Vec3 = Vec3::new(0.34, 0.05, 0.34);
const STOOL_LEG: f32 = 0.05;
/// Stools stand this far in front of the counter, at these X positions.
const STOOL_DISTANCE: f32 = 0.35;
const STOOLS: [f32; 2] = [-0.55, 0.55];

/// Eye height of the seated player above the counter top.
const EYE_HEIGHT: f32 = 0.5;
/// Distance of the player's eyes from the edge of the work area.
const EYE_DISTANCE: f32 = 0.45;
/// Where the seated player looks, see [`Block::top_spot`].
const LOOK_SPOT: Vec2 = Vec2::new(0.0, -0.5);

const TOP_WOOD: Color = Color::srgb_u8(0x8e, 0x6c, 0x4c);
const SLAT_WOOD: Color = Color::srgb_u8(0x5a, 0x40, 0x2c);
const MAT: Color = Color::srgb_u8(0x4f, 0x7a, 0x6b);
const STOOL_METAL: Color = Color::srgb_u8(0x50, 0x54, 0x58);
const STOOL_SEAT_RED: Color = Color::srgb_u8(0x8a, 0x2e, 0x2a);

pub(crate) struct CounterPlugin;

impl Plugin for CounterPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Counter::on_add);
    }
}

/// The counter across the front of the shop: a wall towards the street with
/// the work surface on top, and stools for waiting customers.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Counter;

impl Counter {
    fn on_add(add: On<Add<Counter>>, mut commands: Commands) {
        let layout = CounterLayout::new();
        commands.entity(add.entity).with_children(|counter| {
            counter.spawn((Shape::cuboid(&layout.top, TOP_WOOD), Finish::Wood));
            counter.spawn((Shape::cuboid(&layout.mat, MAT), Finish::Plastic));
            counter.spawn((Shape::cuboid(&layout.shelf, TOP_WOOD), Finish::Wood));
            let wall = &layout.wall;
            let mut x = wall.min().x;
            while x < wall.max().x - SLAT_GAP {
                let width = SLAT.min(wall.max().x - x) - SLAT_GAP;
                let slat = Block::between(wall.min().with_x(x), wall.max().with_x(x + width));
                counter.spawn((Shape::cuboid(&slat, SLAT_WOOD), Finish::Wood));
                x += SLAT;
            }
            for x in STOOLS {
                let at = Vec3::new(x, 0.0, wall.min().z - STOOL_DISTANCE);
                let leg = Block::new(Vec3::new(STOOL_LEG, STOOL_HEIGHT - STOOL_SEAT.y, STOOL_LEG))
                    .translated(at);
                let seat = Block::new(STOOL_SEAT).translated(at).on_top_of(&leg);
                let foot = Block::new(Vec3::new(0.3, 0.03, 0.3)).translated(at);
                counter.spawn((Shape::cylinder(&leg, STOOL_METAL), Finish::Metal));
                counter.spawn((Shape::cylinder(&foot, STOOL_METAL), Finish::Metal));
                counter.spawn((Shape::cylinder(&seat, STOOL_SEAT_RED), Finish::Plastic));
            }
        });
    }
}

/// Geometry of the [`Counter`] in its own space.
pub(crate) struct CounterLayout {
    top: Block,
    wall: Block,
    work: Block,
    pub(super) mat: Block,
    shelf: Block,
}

impl CounterLayout {
    pub(crate) fn new() -> Self {
        let half = SIZE.x / 2.0 - PILLAR.x;
        let wall_z = FRONT + PILLAR.y / 2.0;
        let top_y = PLATFORM + HEIGHT;
        let wall = Block::between(
            Vec3::new(-half, 0.0, wall_z - WALL / 2.0),
            Vec3::new(half, top_y - TOP_THICKNESS, wall_z + WALL / 2.0),
        );
        let top = Block::between(
            Vec3::new(-half, top_y - TOP_THICKNESS, wall.min().z - OVERHANG),
            Vec3::new(half, top_y, wall.min().z - OVERHANG + TOP_DEPTH),
        );
        let work = Block::new(WORK_AREA.extend(TOP_THICKNESS).xzy()).translated(Vec3::new(
            0.0,
            top.min().y,
            top.center.z,
        ));
        let mat = Block::new(MAT_SIZE).resting_on(&work, MAT_SPOT);
        let shelf = Block::between(
            Vec3::new(-half, PLATFORM + SHELF_HEIGHT - 0.02, wall.max().z),
            Vec3::new(half, PLATFORM + SHELF_HEIGHT, wall.max().z + SHELF_DEPTH),
        );
        Self {
            top,
            wall,
            work,
            mat,
            shelf,
        }
    }

    /// Something resting on the work area at `spot` (see
    /// [`Block::top_spot`]), turned by `yaw` radians.
    pub(crate) fn place(&self, spot: Vec2, yaw: f32) -> Transform {
        Transform::from_translation(self.work.top_spot(spot))
            .with_rotation(Quat::from_rotation_y(yaw))
    }

    /// Like [`CounterLayout::place`], but on top of the cutting mat, with
    /// `spot` relative to the mat.
    pub(crate) fn place_on_mat(&self, spot: Vec2, yaw: f32) -> Transform {
        Transform::from_translation(self.mat.top_spot(spot))
            .with_rotation(Quat::from_rotation_y(yaw))
    }

    /// Camera pose of the seated player.
    pub(crate) fn seated_view(&self) -> Transform {
        let eye = Vec3::new(
            0.0,
            self.work.max().y + EYE_HEIGHT,
            self.work.max().z + EYE_DISTANCE,
        );
        Transform::from_translation(eye).looking_at(self.work.top_spot(LOOK_SPOT), Vec3::Y)
    }
}
