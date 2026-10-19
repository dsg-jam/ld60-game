use bevy::prelude::*;

use super::rug::{self, Rug};
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;

/// Z of the open front towards the street, which lies towards -Z.
pub(crate) const FRONT: f32 = -0.6;
/// Inner width along X, height and depth along +Z. As high as the market
/// floors of the other buildings.
pub(crate) const SIZE: Vec3 = Vec3::new(3.0, 3.6, 3.0);
/// The shop floor is raised above the street.
pub(crate) const PLATFORM: f32 = 0.3;
/// Width and depth of the pillars at both sides of the opening.
pub(crate) const PILLAR: Vec2 = Vec2::new(0.3, 0.3);

const WALL: f32 = 0.05;
/// The shelves and the cabinet start behind the counter.
const BEHIND_COUNTER: f32 = 0.95;

const SHELF_DEPTH: f32 = 0.32;
const SHELF_LENGTH: f32 = 1.6;
const SHELF_HEIGHT: f32 = 2.1;
const SHELF_BOARDS: usize = 5;
const BOARD: f32 = 0.025;
const UPRIGHT: f32 = 0.03;
const CABINET: Vec3 = Vec3::new(0.45, 1.3, 0.7);

const PLASTER: Color = Color::srgb_u8(0x5e, 0x55, 0x4c);
const BOARDS: Color = Color::srgb_u8(0x5a, 0x40, 0x2c);
/// Middle of the rug, under the player's seat.
const RUG_Z: f32 = 1.0;
/// Matches the pillars of the market stalls.
const CONCRETE: Color = Color::srgb_u8(0x4a, 0x46, 0x44);
const SHELF_WOOD: Color = Color::srgb_u8(0x6a, 0x4e, 0x36);
const CABINET_GREEN: Color = Color::srgb_u8(0x4a, 0x5a, 0x4c);
const BOXES: [Color; 4] = [
    Color::srgb_u8(0x8a, 0x70, 0x50),
    Color::srgb_u8(0x6a, 0x6e, 0x70),
    Color::srgb_u8(0x9a, 0x86, 0x66),
    Color::srgb_u8(0x50, 0x58, 0x60),
];

pub(crate) struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Shop::on_add);
    }
}

/// The cramped room behind the counter, open towards the street.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Shop;

impl Shop {
    fn on_add(add: On<Add<Shop>>, mut commands: Commands) {
        let ShopLayout {
            room,
            shelf,
            cabinet,
            rug,
        } = ShopLayout::new();
        let (min, max) = (room.min(), room.max());
        let floor = Block::between(min.with_z(FRONT + PILLAR.y), max.with_y(PLATFORM));
        let ceiling = Block::between(min.with_y(max.y - WALL), max);
        let left = Block::between(min, max.with_x(min.x + WALL));
        let right = Block::between(min.with_x(max.x - WALL), max);
        let back = Block::between(min.with_z(max.z - WALL), max);

        let pillar = Block::new(PILLAR.extend(SIZE.y).xzy());
        let pillars = [-1.0, 1.0].map(|side| {
            pillar.translated(Vec3::new(
                side * (SIZE.x - PILLAR.x) / 2.0,
                0.0,
                FRONT + PILLAR.y / 2.0,
            ))
        });

        commands.entity(add.entity).with_children(|shop| {
            shop.spawn((Shape::cuboid(&floor, BOARDS), Finish::Planks));
            shop.spawn((Rug, Transform::from_translation(rug.bottom())));
            for wall in [ceiling, left, right, back] {
                shop.spawn((Shape::cuboid(&wall, PLASTER), Finish::Plaster));
            }
            for pillar in &pillars {
                shop.spawn((Shape::cuboid(pillar, CONCRETE), Finish::Concrete));
            }
            Self::spawn_shelf(shop, &shelf);
            shop.spawn(Shape::cuboid(&cabinet, CABINET_GREEN));
        });
    }

    /// Open shelves with boxes on each board.
    fn spawn_shelf(shop: &mut ChildSpawnerCommands, shelf: &Block) {
        for end in [-1.0, 1.0] {
            let upright = Block::new(shelf.size.with_z(UPRIGHT)).translated(
                shelf.center.with_y(shelf.min().y) + Vec3::Z * end * (shelf.size.z - UPRIGHT) / 2.0,
            );
            shop.spawn((Shape::cuboid(&upright, SHELF_WOOD), Finish::Wood));
        }
        let pitch = (shelf.size.y - BOARD) / (SHELF_BOARDS - 1) as f32;
        for index in 0..SHELF_BOARDS {
            let board = Block::new(shelf.size.with_y(BOARD))
                .translated(shelf.center.with_y(shelf.min().y + index as f32 * pitch));
            shop.spawn((Shape::cuboid(&board, SHELF_WOOD), Finish::Wood));
            if index + 1 == SHELF_BOARDS {
                continue;
            }
            // A few boxes of different sizes, filling the board from one end.
            let mut z = board.min().z + UPRIGHT;
            for (n, color) in BOXES.iter().cycle().skip(index).take(4).enumerate() {
                let size = Vec3::new(
                    shelf.size.x * 0.8,
                    pitch * (0.45 + 0.1 * ((index + n) % 4) as f32),
                    0.18 + 0.08 * ((index * 3 + n) % 3) as f32,
                );
                if z + size.z > board.max().z - UPRIGHT {
                    break;
                }
                let item = Block::new(size).on_top_of(&board).translated(Vec3::new(
                    board.center.x,
                    0.0,
                    z + size.z / 2.0,
                ));
                shop.spawn((Shape::cuboid(&item, *color), Finish::Paper));
                z += size.z + 0.03;
            }
        }
    }
}

/// Geometry of the [`Shop`] in its own space, which is the world's.
pub(crate) struct ShopLayout {
    /// The whole room including its walls, from the street level up.
    pub(crate) room: Block,
    /// Along the left wall.
    pub(crate) shelf: Block,
    /// Against the right wall.
    pub(crate) cabinet: Block,
    /// On the floor, under the player's seat.
    pub(crate) rug: Block,
}

impl ShopLayout {
    pub(crate) fn new() -> Self {
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
            rug: Block::new(rug::SIZE).translated(Vec3::new(0.0, PLATFORM, RUG_Z)),
        }
    }

    /// The air inside the walls, above the floor and under the ceiling.
    /// Its -Z side is the open front.
    pub(crate) fn air(&self) -> Block {
        let (min, max) = (self.room.min(), self.room.max());
        Block::between(
            Vec3::new(min.x + WALL, PLATFORM, min.z),
            Vec3::new(max.x - WALL, max.y - WALL, max.z - WALL),
        )
    }
}
