use bevy::prelude::*;
use ld60_geometry::block::Block;

use super::shop::{FRONT, PILLAR, PLATFORM, SIZE};

/// Height of the counter top above the shop floor.
const HEIGHT: f32 = 0.75;
const TOP_THICKNESS: f32 = 0.05;
const TOP_DEPTH: f32 = 0.85;
/// How far the top reaches over the counter wall towards the street.
const OVERHANG: f32 = 0.06;
const WALL: f32 = 0.06;
/// The part of the top where the player works and puts things.
const WORK_AREA: Vec2 = Vec2::new(1.6, 0.8);
const MAT_SIZE: Vec3 = Vec3::new(0.48, 0.004, 0.42);
/// Where the mat lies, see [`Block::top_spot`].
const MAT_SPOT: Vec2 = Vec2::new(0.0, 0.1);
/// A shelf under the top, on the player's side.
const SHELF_HEIGHT: f32 = 0.3;
const SHELF_DEPTH: f32 = 0.4;
/// Eye height of the seated player above the counter top.
const EYE_HEIGHT: f32 = 0.5;
/// Distance of the player's eyes from the edge of the work area.
const EYE_DISTANCE: f32 = 0.45;
/// Where the seated player looks, see [`Block::top_spot`].
const LOOK_SPOT: Vec2 = Vec2::new(0.0, -0.5);

/// Geometry of the counter in its own space.
pub(crate) struct CounterLayout {
    pub(crate) top: Block,
    pub(crate) wall: Block,
    work: Block,
    pub(crate) mat: Block,
    pub(crate) shelf: Block,
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
