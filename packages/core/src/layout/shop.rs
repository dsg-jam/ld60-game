use bevy::prelude::*;
use ld60_geometry::block::Block;

/// Z of the open front towards the street, which lies towards -Z.
pub const FRONT: f32 = -0.6;
/// Inner width along X, height and depth along +Z. As high as the market
/// floors of the other buildings.
pub const SIZE: Vec3 = Vec3::new(3.0, 3.6, 3.0);
/// The shop floor is raised above the street.
pub const PLATFORM: f32 = 0.3;
/// Width and depth of the pillars at both sides of the opening.
pub const PILLAR: Vec2 = Vec2::new(0.3, 0.3);

pub const WALL: f32 = 0.05;
/// The shelves and the cabinet start behind the counter.
const BEHIND_COUNTER: f32 = 0.95;

const SHELF_DEPTH: f32 = 0.32;
const SHELF_LENGTH: f32 = 1.6;
const SHELF_HEIGHT: f32 = 2.1;
const CABINET: Vec3 = Vec3::new(0.45, 1.3, 0.7);

/// Middle of the rug, under the player's seat.
const RUG_Z: f32 = 1.0;
/// The knotted part of the rug, long along Z.
pub const RUG_SIZE: Vec3 = Vec3::new(1.3, 0.008, 1.8);

/// Geometry of the shop in its own space, which is the world's.
pub struct ShopLayout {
    /// The whole room including its walls, from the street level up.
    pub room: Block,
    /// Along the left wall.
    pub shelf: Block,
    /// Against the right wall.
    pub cabinet: Block,
    /// On the floor, under the player's seat.
    pub rug: Block,
}

impl Default for ShopLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl ShopLayout {
    pub fn new() -> Self {
        let room = Block::between(
            Vec3::new(-SIZE.x / 2.0, 0.0, FRONT),
            Vec3::new(SIZE.x / 2.0, SIZE.y, FRONT + SIZE.z),
        );
        let shelf_x = room.min().x + WALL + SHELF_DEPTH / 2.0;
        let shelf_z = FRONT + BEHIND_COUNTER + SHELF_LENGTH / 2.0;
        let shelf = Block::new(Vec3::new(SHELF_DEPTH, SHELF_HEIGHT, SHELF_LENGTH))
            .translated(Vec3::new(shelf_x, PLATFORM, shelf_z));
        let cabinet = Block::new(CABINET).translated(Vec3::new(
            room.max().x - WALL - CABINET.x / 2.0,
            PLATFORM,
            FRONT + BEHIND_COUNTER + CABINET.z / 2.0,
        ));
        Self {
            room,
            shelf,
            cabinet,
            rug: Block::new(RUG_SIZE).translated(Vec3::new(0.0, PLATFORM, RUG_Z)),
        }
    }

    /// The air inside the walls, above the floor and under the ceiling.
    /// Its -Z side is the open front.
    pub fn air(&self) -> Block {
        let (min, max) = (self.room.min(), self.room.max());
        Block::between(
            Vec3::new(min.x + WALL, PLATFORM, min.z),
            Vec3::new(max.x - WALL, max.y - WALL, max.z - WALL),
        )
    }
}
