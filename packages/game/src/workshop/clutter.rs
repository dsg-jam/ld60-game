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
        });
    }
}
