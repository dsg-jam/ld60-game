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
/// Width and height of the LED strips under the boards.
const STRIP: Vec2 = Vec2::new(0.012, 0.006);
/// The strips sit this far back from the front of the boards.
const STRIP_BACK: f32 = 0.025;
const STRIP_GLOW: f32 = 12.0;
/// Lights along each strip.
const STRIP_LIGHTS: usize = 4;
const STRIP_INTENSITY: f32 = 900.0;
/// About as far as the board below, so the light does not reach through it.
const STRIP_RANGE: f32 = 0.6;

const PLASTER: Color = Color::srgb_u8(0x5e, 0x55, 0x4c);
const BOARDS: Color = Color::srgb_u8(0x5a, 0x40, 0x2c);
/// Matches the pillars of the market stalls.
const CONCRETE: Color = Color::srgb_u8(0x4a, 0x46, 0x44);
const SHELF_WOOD: Color = Color::srgb_u8(0x6a, 0x4e, 0x36);
const CABINET_GREEN: Color = Color::srgb_u8(0x4a, 0x5a, 0x4c);
const ALUMINIUM: Color = Color::srgb_u8(0xa8, 0xaa, 0xac);
const LED: Color = Color::srgb(0.9, 0.95, 1.0);

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
    fn on_add(
        add: On<Add<Shop>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
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
            Self::spawn_shelf(shop, &shelf, (&mut meshes, &mut materials));
            shop.spawn(Shape::cuboid(&cabinet, CABINET_GREEN));
        });
    }

    /// Open shelves, each board filled with what the shop keeps there.
    fn spawn_shelf(
        shop: &mut ChildSpawnerCommands,
        shelf: &Block,
        (meshes, materials): (&mut Assets<Mesh>, &mut Assets<StandardMaterial>),
    ) {
        let glow = (
            meshes.add(Cuboid::from_size(Vec3::ONE)),
            materials.add(StandardMaterial {
                base_color: Color::BLACK,
                emissive: LED.to_linear() * STRIP_GLOW,
                ..default()
            }),
        );
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
            if index > 0 {
                Self::spawn_strip(shop, &board, glow.clone());
            }
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

    /// An LED strip under the front of `board`, lighting the board below.
    /// The shelf runs along Z with its front towards +X.
    fn spawn_strip(
        shop: &mut ChildSpawnerCommands,
        board: &Block,
        glow: (Handle<Mesh>, Handle<StandardMaterial>),
    ) {
        let length = board.size.z - 2.0 * UPRIGHT;
        let channel = Block::new(Vec3::new(STRIP.x, STRIP.y, length)).translated(Vec3::new(
            board.max().x - STRIP_BACK,
            board.min().y - STRIP.y,
            board.center.z,
        ));
        shop.spawn((Shape::cuboid(&channel, ALUMINIUM), Finish::Metal));
        let diffuser = Block::new(Vec3::new(STRIP.x * 0.8, 0.003, length - 0.01))
            .translated(channel.center.with_y(channel.min().y - 0.0031));
        shop.spawn((
            Mesh3d(glow.0),
            MeshMaterial3d(glow.1),
            diffuser.transform().with_scale(diffuser.size),
        ));
        let pitch = length / STRIP_LIGHTS as f32;
        for light in 0..STRIP_LIGHTS {
            let z = board.center.z - length / 2.0 + pitch * (light as f32 + 0.5);
            let at = channel.center.with_y(channel.min().y - 0.002).with_z(z);
            shop.spawn((
                SpotLight {
                    color: LED,
                    intensity: STRIP_INTENSITY,
                    range: STRIP_RANGE,
                    radius: STRIP.x / 2.0,
                    inner_angle: 0.6,
                    outer_angle: 1.45,
                    ..default()
                },
                Transform::from_translation(at).looking_to(Vec3::NEG_Y, Vec3::X),
            ));
        }
    }
}
