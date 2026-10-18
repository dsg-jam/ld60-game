use bevy::{light::NotShadowCaster, prelude::*};

use super::switch::LightSwitch;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;

const FLOOR_SIZE: Vec3 = Vec3::new(4.0, 0.02, 4.0);
const WALL_HEIGHT: f32 = 2.6;
const WALL_THICKNESS: f32 = 0.05;
/// Front face of the back wall, just behind the desk.
const BACK_WALL_Z: f32 = -0.42;
/// Inner faces of the side walls.
const SIDE_WALL_X: f32 = 1.4;
const BASEBOARD_SIZE: Vec2 = Vec2::new(0.08, 0.015);

const WINDOW_CENTER: Vec2 = Vec2::new(-0.75, 1.8);
const WINDOW_SIZE: Vec2 = Vec2::new(0.7, 0.5);
const WINDOW_FRAME: f32 = 0.04;
const NIGHT_SKY: LinearRgba = LinearRgba::rgb(0.006, 0.01, 0.025);

const SHELF_CENTER: Vec2 = Vec2::new(0.45, 1.3);
const SHELF_SIZE: Vec3 = Vec3::new(0.75, 0.025, 0.2);
const BRACKET_SIZE: Vec3 = Vec3::new(0.02, 0.1, 0.15);
/// Width and height of each book, from left to right.
const BOOKS: [(Vec2, Color); 6] = [
    (Vec2::new(0.035, 0.2), Color::srgb_u8(0x8a, 0x3b, 0x2e)),
    (Vec2::new(0.05, 0.23), Color::srgb_u8(0x2f, 0x4f, 0x5f)),
    (Vec2::new(0.03, 0.18), Color::srgb_u8(0xb9, 0x8a, 0x3e)),
    (Vec2::new(0.045, 0.21), Color::srgb_u8(0x55, 0x62, 0x3a)),
    (Vec2::new(0.04, 0.19), Color::srgb_u8(0x6e, 0x4a, 0x6b)),
    (Vec2::new(0.06, 0.24), Color::srgb_u8(0x3e, 0x3a, 0x36)),
];
const BOOK_DEPTH: f32 = 0.15;
const BOOKS_START: f32 = -0.33;

/// Pendant lamp hanging from the ceiling above the front of the desk.
const PENDANT_SPOT: Vec3 = Vec3::new(0.0, 2.15, 0.35);
const CORD_SIZE: Vec3 = Vec3::new(0.008, 0.45, 0.008);
const PENDANT_SHADE_SIZE: Vec3 = Vec3::new(0.3, 0.12, 0.3);
const PENDANT_BULB_RADIUS: f32 = 0.03;
const PENDANT_LIGHT: Color = Color::srgb(1.0, 0.82, 0.6);
const PENDANT_INTENSITY: f32 = 25_000.0;
const PENDANT_RANGE: f32 = 8.0;
/// Much smaller than the default, so thin walls cast solid shadows.
const SHADOW_DEPTH_BIAS: f32 = 0.01;
const SHADOW_NORMAL_BIAS: f32 = 0.3;
const PENDANT_GLOW: LinearRgba = LinearRgba::rgb(10.0, 7.5, 5.0);
const PENDANT_SHADE: Color = Color::srgb_u8(0x3f, 0x4a, 0x42);

/// Wall switch for the ceiling lamp, left of the desk.
const SWITCH_CENTER: Vec2 = Vec2::new(-1.1, 1.1);
const SWITCH_PLATE_SIZE: Vec3 = Vec3::new(0.07, 0.11, 0.01);
const SWITCH_TOGGLE_SIZE: Vec3 = Vec3::new(0.02, 0.035, 0.012);
const SWITCH: Color = Color::srgb_u8(0xe6, 0xdf, 0xd0);

const WALL: Color = Color::srgb_u8(0x6b, 0x5a, 0x4c);
const BASEBOARD: Color = Color::srgb_u8(0x4a, 0x3d, 0x33);
const FLOOR: Color = Color::srgb_u8(0x3a, 0x2c, 0x22);
const WOOD_DARK: Color = Color::srgb_u8(0x8e, 0x6a, 0x4a);
const FRAME: Color = Color::srgb_u8(0xd9, 0xd0, 0xc0);

pub(crate) struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Room::on_add);
    }
}

/// Floor, walls, a window, a book shelf and a ceiling lamp around the desk.
///
/// The walls don't cast shadows, so the moonlight still reaches the desk.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Room;

impl Room {
    fn on_add(
        add: On<Add<Room>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let floor = Block::new(FLOOR_SIZE).translated(Vec3::NEG_Y * FLOOR_SIZE.y);
        let back = Block::new(Vec3::new(2.0 * SIDE_WALL_X, WALL_HEIGHT, WALL_THICKNESS))
            .translated(Vec3::Z * (BACK_WALL_Z - WALL_THICKNESS / 2.0));
        let side = Block::new(Vec3::new(WALL_THICKNESS, WALL_HEIGHT, FLOOR_SIZE.z))
            .translated(Vec3::Z * (BACK_WALL_Z + FLOOR_SIZE.z / 2.0));
        let left = side.translated(Vec3::NEG_X * (SIDE_WALL_X + WALL_THICKNESS / 2.0));
        let right = side.translated(Vec3::X * (SIDE_WALL_X + WALL_THICKNESS / 2.0));
        let baseboard = Block::new(Vec3::new(back.size.x, BASEBOARD_SIZE.x, BASEBOARD_SIZE.y))
            .in_front_of(&back);

        let pane = Block::new(WINDOW_SIZE.extend(0.005))
            .translated(WINDOW_CENTER.extend(0.0) - Vec3::Y * WINDOW_SIZE.y / 2.0)
            .in_front_of(&back);
        let frame_depth = 0.03;
        let frame_bars = [
            Vec3::new(WINDOW_SIZE.x + WINDOW_FRAME, WINDOW_FRAME, frame_depth),
            Vec3::new(WINDOW_FRAME, WINDOW_SIZE.y, frame_depth),
        ];
        let frame = |size: Vec3, offset: Vec2| {
            Block::new(size)
                .centered_on(&pane)
                .translated(offset.extend(0.0))
                .in_front_of(&back)
        };
        let half = WINDOW_SIZE / 2.0;
        let frames = [
            frame(frame_bars[0], Vec2::Y * half.y),
            frame(frame_bars[0], Vec2::NEG_Y * half.y),
            frame(frame_bars[0], Vec2::ZERO),
            frame(frame_bars[1], Vec2::X * half.x),
            frame(frame_bars[1], Vec2::NEG_X * half.x),
            frame(frame_bars[1], Vec2::ZERO),
        ];

        let shelf = Block::new(SHELF_SIZE)
            .translated(SHELF_CENTER.extend(0.0))
            .in_front_of(&back);
        let brackets = [-0.3, 0.3].map(|x| {
            Block::new(BRACKET_SIZE)
                .below(&shelf)
                .translated(Vec3::X * x * SHELF_SIZE.x)
                .in_front_of(&back)
                .translated(Vec3::X * shelf.center.x)
        });
        let mut x = shelf.center.x + BOOKS_START;
        let books = BOOKS.map(|(size, color)| {
            let book = Block::new(size.extend(BOOK_DEPTH))
                .resting_on(&shelf, Vec2::ZERO)
                .translated(Vec3::X * (x + size.x / 2.0 - shelf.center.x));
            x += size.x;
            (book, color)
        });

        let shade = Block::new(PENDANT_SHADE_SIZE).standing_at(PENDANT_SPOT);
        let cord = Block::new(CORD_SIZE).on_top_of(&shade);
        let bulb = shade.bottom();
        let plate = Block::new(SWITCH_PLATE_SIZE)
            .translated(SWITCH_CENTER.extend(0.0) - Vec3::Y * SWITCH_PLATE_SIZE.y / 2.0)
            .in_front_of(&back);
        let toggle = Block::new(SWITCH_TOGGLE_SIZE)
            .translated(Vec3::NEG_Y * SWITCH_TOGGLE_SIZE.y / 2.0)
            .translated(Vec3::Z * (SWITCH_PLATE_SIZE.z + SWITCH_TOGGLE_SIZE.z) / 2.0);
        let pendant_bulb = materials.add(StandardMaterial {
            base_color: PENDANT_LIGHT,
            emissive: PENDANT_GLOW,
            ..default()
        });

        let mut pendant_light = Entity::PLACEHOLDER;
        commands.entity(add.entity).with_children(|room| {
            room.spawn(Shape::cylinder(&cord, PENDANT_SHADE));
            // A cloth shade lets light through, so it doesn't block the walls.
            room.spawn((Shape::cylinder(&shade, PENDANT_SHADE), NotShadowCaster));
            room.spawn((
                Mesh3d(meshes.add(Sphere::new(PENDANT_BULB_RADIUS))),
                MeshMaterial3d(pendant_bulb.clone()),
                Transform::from_translation(bulb),
                NotShadowCaster,
            ));
            pendant_light = room
                .spawn((
                    PointLight {
                        color: PENDANT_LIGHT,
                        intensity: PENDANT_INTENSITY,
                        range: PENDANT_RANGE,
                        shadow_maps_enabled: true,
                        shadow_depth_bias: SHADOW_DEPTH_BIAS,
                        shadow_normal_bias: SHADOW_NORMAL_BIAS,
                        ..default()
                    },
                    Transform::from_translation(bulb - Vec3::Y * 2.0 * PENDANT_BULB_RADIUS),
                ))
                .id();
            room.spawn((Shape::cuboid(&floor, FLOOR), Finish::Planks));
            for wall in [back, left, right] {
                room.spawn((Shape::cuboid(&wall, WALL), Finish::Plaster, NotShadowCaster));
            }
            room.spawn((
                Shape::cuboid(&baseboard, BASEBOARD),
                Finish::Wood,
                NotShadowCaster,
            ));
            room.spawn((
                Mesh3d(meshes.add(Cuboid::from_size(pane.size))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::BLACK,
                    emissive: NIGHT_SKY,
                    ..default()
                })),
                pane.transform(),
                NotShadowCaster,
            ));
            for bar in frames {
                room.spawn((Shape::cuboid(&bar, FRAME), Finish::Wood, NotShadowCaster));
            }
            room.spawn((Shape::cuboid(&shelf, WOOD_DARK), Finish::Wood));
            for bracket in brackets {
                room.spawn((Shape::cuboid(&bracket, WOOD_DARK), Finish::Wood));
            }
            for (book, color) in books {
                room.spawn((Shape::cuboid(&book, color), Finish::Paper));
            }
            room.spawn((
                Shape::cuboid(&plate, SWITCH),
                NotShadowCaster,
                Name::new("light switch"),
            ))
            .insert(LightSwitch::new(
                pendant_light,
                PENDANT_INTENSITY,
                pendant_bulb,
                PENDANT_GLOW,
            ))
            .with_children(|plate| {
                plate.spawn(Shape::cuboid(&toggle, SWITCH));
            });
        });
    }
}
