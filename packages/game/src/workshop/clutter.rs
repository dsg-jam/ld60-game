mod props;

use bevy::prelude::*;

use self::props::{Finishes, Props};
use super::desk::DeskLayout;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::{Finish, SurfaceMaterial};

const MUG_SPOT: Vec2 = Vec2::new(-0.62, 0.45);
const SCREWDRIVER_SPOT: Vec2 = Vec2::new(-0.38, 0.62);
const SCREWDRIVER_YAW: f32 = 0.4;
/// Spots on the mat, see [`DeskLayout::place_on_mat`], and yaws.
const SCREWS: [(Vec2, f32); 3] = [
    (Vec2::new(0.32, 0.62), 0.4),
    (Vec2::new(0.4, 0.74), 2.1),
    (Vec2::new(0.27, 0.82), -0.9),
];

const NOTES: [(Vec2, f32, Color); 2] = [
    (
        Vec2::new(-0.2, -0.55),
        0.2,
        Color::srgb_u8(0xe8, 0xd4, 0x6a),
    ),
    (
        Vec2::new(-0.1, -0.68),
        -0.3,
        Color::srgb_u8(0xe6, 0x9a, 0xa0),
    ),
];
const NOTE_SIZE: Vec3 = Vec3::new(0.075, 0.002, 0.075);

const BOX_SPOT: Vec2 = Vec2::new(-0.5, -0.62);
const BOX_YAW: f32 = 0.25;
const BOX_SIZE: Vec3 = Vec3::new(0.26, 0.11, 0.18);
/// Thick enough that light doesn't leak through its shadow.
const BOX_WALL: f32 = 0.012;
/// Broken devices peeking out of the box: size, offset and tilt.
const BROKEN: [(Vec3, Vec3, f32); 2] = [
    (
        Vec3::new(0.085, 0.14, 0.02),
        Vec3::new(-0.05, 0.0, 0.01),
        0.25,
    ),
    (
        Vec3::new(0.1, 0.12, 0.022),
        Vec3::new(0.05, 0.0, -0.03),
        -0.2,
    ),
];

const CARDBOARD: Color = Color::srgb_u8(0xa8, 0x82, 0x58);
const PLASTIC: [Color; 2] = [
    Color::srgb_u8(0x3e, 0x3a, 0x36),
    Color::srgb_u8(0x6a, 0x6e, 0x70),
];

pub(crate) struct ClutterPlugin;

impl Plugin for ClutterPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Clutter::on_add);
    }
}

/// Small things lying around on the desk.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Clutter;

impl Clutter {
    fn on_add(
        add: On<Add<Clutter>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let desk = DeskLayout::new();
        let finishes = Finishes::new(&mut materials, &mut surfaces);
        let mut props = Props {
            meshes: &mut meshes,
            finishes: &finishes,
        };
        commands.entity(add.entity).with_children(|clutter| {
            clutter
                .spawn((desk.place(MUG_SPOT, 0.0), Visibility::default()))
                .with_children(|mug| props.mug(mug));
            clutter
                .spawn((
                    desk.place(SCREWDRIVER_SPOT, SCREWDRIVER_YAW),
                    Visibility::default(),
                ))
                .with_children(|screwdriver| props.screwdriver(screwdriver));
            for (spot, yaw) in SCREWS {
                clutter
                    .spawn((desk.place_on_mat(spot, yaw), Visibility::default()))
                    .with_children(|screw| props.screw(screw));
            }
            for (spot, yaw, color) in NOTES {
                clutter
                    .spawn((desk.place(spot, yaw), Visibility::default()))
                    .with_children(|note| {
                        note.spawn((Shape::cuboid(&Block::new(NOTE_SIZE), color), Finish::Paper));
                    });
            }
            clutter
                .spawn((desk.place(BOX_SPOT, BOX_YAW), Visibility::default()))
                .with_children(Self::repair_box);
        });
    }

    /// Open cardboard box with two broken devices in it.
    fn repair_box(parts: &mut ChildSpawnerCommands) {
        let outer = Block::new(BOX_SIZE);
        let bottom = Block::new(BOX_SIZE.with_y(BOX_WALL));
        let side = Block::new(Vec3::new(BOX_WALL, BOX_SIZE.y, BOX_SIZE.z));
        let end = Block::new(Vec3::new(BOX_SIZE.x, BOX_SIZE.y, BOX_WALL));
        let walls = [
            side.translated(Vec3::X * (outer.min().x + BOX_WALL / 2.0)),
            side.translated(Vec3::X * (outer.max().x - BOX_WALL / 2.0)),
            end.translated(Vec3::Z * (outer.min().z + BOX_WALL / 2.0)),
            end.translated(Vec3::Z * (outer.max().z - BOX_WALL / 2.0)),
        ];
        parts.spawn((Shape::cuboid(&bottom, CARDBOARD), Finish::Paper));
        for wall in walls {
            parts.spawn((Shape::cuboid(&wall, CARDBOARD), Finish::Paper));
        }
        for ((size, offset, tilt), color) in BROKEN.into_iter().zip(PLASTIC) {
            parts
                .spawn((
                    Transform::from_translation(offset + Vec3::Y * BOX_WALL)
                        .with_rotation(Quat::from_rotation_z(tilt)),
                    Visibility::default(),
                ))
                .with_children(|device| {
                    device.spawn((Shape::cuboid(&Block::new(size), color), Finish::Plastic));
                });
        }
    }
}
