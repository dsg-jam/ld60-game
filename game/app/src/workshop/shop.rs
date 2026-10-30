use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use ld60_game_core::layout::shop::{FRONT, PILLAR, PLATFORM, SIZE, ShopLayout, WALL};
use ld60_game_props::Theme;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use super::rug::Rug;

const SHELF_BOARDS: usize = 5;
/// From the bottom board up.
const SHELF_THEMES: [Theme; SHELF_BOARDS] = [
    Theme::Stock,
    Theme::Salvage,
    Theme::Parts,
    Theme::Papers,
    Theme::Stock,
];
/// How high things are piled on top of the shelf.
const ON_TOP: f32 = 0.5;
const BOARD: f32 = 0.025;
const UPRIGHT: f32 = 0.03;

const PLASTER: Color = Color::srgb_u8(0x5e, 0x55, 0x4c);
const BOARDS: Color = Color::srgb_u8(0x5a, 0x40, 0x2c);
/// Matches the pillars of the market stalls.
const CONCRETE: Color = Color::srgb_u8(0x4a, 0x46, 0x44);
const SHELF_WOOD: Color = Color::srgb_u8(0x6a, 0x4e, 0x36);
const CABINET_GREEN: Color = Color::srgb_u8(0x4a, 0x5a, 0x4c);

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
        // Behind the pillars and the shutter, so no face shares their front.
        let (min, max) = (room.min().with_z(FRONT + PILLAR.y), room.max());
        let floor = Block::between(min, max.with_y(PLATFORM));
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

    /// Open shelves, each board filled with what the shop keeps there.
    fn spawn_shelf(shop: &mut ChildSpawnerCommands, shelf: &Block) {
        for end in [-1.0, 1.0] {
            let upright = Block::new(shelf.size.with_z(UPRIGHT)).translated(
                shelf.center.with_y(shelf.min().y) + Vec3::Z * end * (shelf.size.z - UPRIGHT) / 2.0,
            );
            shop.spawn((Shape::cuboid(&upright, SHELF_WOOD), Finish::Wood));
        }
        let pitch = (shelf.size.y - BOARD) / (SHELF_BOARDS - 1) as f32;
        for (index, theme) in SHELF_THEMES.into_iter().enumerate() {
            let board = Block::new(shelf.size.with_y(BOARD))
                .translated(shelf.center.with_y(shelf.min().y + index as f32 * pitch));
            shop.spawn((Shape::cuboid(&board, SHELF_WOOD), Finish::Wood));
            let height = if index + 1 == SHELF_BOARDS {
                ON_TOP
            } else {
                pitch - BOARD
            };
            let room = Vec3::new(shelf.size.z - 2.0 * UPRIGHT, height, shelf.size.x);
            // The front of the board faces the room, towards +X.
            let at = Transform::from_translation(board.center.with_y(board.max().y))
                .with_rotation(Quat::from_rotation_y(FRAC_PI_2));
            shop.spawn((at, Visibility::default()))
                .with_children(|board| theme.fill(index as u64, room, board));
        }
    }
}
