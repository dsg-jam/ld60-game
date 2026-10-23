mod props;
mod sides;

use bevy::prelude::*;
use ld60_core::layout::counter::CounterLayout;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::{Finish, SurfaceMaterial};

use self::props::{Finishes, Props};
#[cfg(test)]
use super::footprint::Footprint;

const MUG_SPOT: Vec2 = Vec2::new(-0.62, 0.45);
const SCREWDRIVER_SPOT: Vec2 = Vec2::new(-0.38, 0.62);
const SCREWDRIVER_YAW: f32 = 0.4;
/// Spots on the mat, see [`CounterLayout::place_on_mat`], and yaws.
const SCREWS: [(Vec2, f32); 3] = [
    (Vec2::new(0.47, 0.62), 0.4),
    (Vec2::new(0.58, 0.74), 2.1),
    (Vec2::new(0.39, 0.82), -0.9),
];

const NOTES: [(Vec2, f32, Color); 2] = [
    (
        Vec2::new(-0.04, -0.55),
        0.2,
        Color::srgb_u8(0xe8, 0xd4, 0x6a),
    ),
    (
        Vec2::new(-0.03, -0.8),
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

/// Small things lying around on the counter.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Clutter;

#[cfg(test)]
impl Clutter {
    pub(super) fn footprints(counter: &CounterLayout) -> Vec<Footprint> {
        let mut footprints = vec![
            Footprint::new("mug", counter.place(MUG_SPOT, 0.0), Props::mug_area()),
            Footprint::new(
                "screwdriver",
                counter.place(SCREWDRIVER_SPOT, SCREWDRIVER_YAW),
                Props::screwdriver_area(),
            ),
        ];
        for (spot, yaw, _) in NOTES {
            footprints.push(Footprint::new(
                "note",
                counter.place(spot, yaw),
                Block::new(NOTE_SIZE),
            ));
        }
        footprints.extend(sides::footprints(counter));
        footprints
    }
}

impl Clutter {
    fn on_add(
        add: On<Add<Clutter>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let counter = CounterLayout::new();
        let finishes = Finishes::new(&mut surfaces);
        let mut props = Props {
            meshes: &mut meshes,
            finishes: &finishes,
        };
        commands.entity(add.entity).with_children(|clutter| {
            clutter
                .spawn((counter.place(MUG_SPOT, 0.0), Visibility::default()))
                .with_children(|mug| props.mug(mug));
            clutter
                .spawn((
                    counter.place(SCREWDRIVER_SPOT, SCREWDRIVER_YAW),
                    Visibility::default(),
                ))
                .with_children(|screwdriver| props.screwdriver(screwdriver));
            for (spot, yaw) in SCREWS {
                clutter
                    .spawn((counter.place_on_mat(spot, yaw), Visibility::default()))
                    .with_children(|screw| props.screw(screw));
            }
            for (spot, yaw, color) in NOTES {
                clutter
                    .spawn((counter.place(spot, yaw), Visibility::default()))
                    .with_children(|note| {
                        note.spawn((Shape::cuboid(&Block::new(NOTE_SIZE), color), Finish::Paper));
                    });
            }
            sides::spawn(clutter, props.meshes, &mut materials);
        });
    }
}
