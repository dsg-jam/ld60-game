use bevy::prelude::*;

use crate::layout::Block;
use crate::shape::Shape;

/// Height of the top surface above the ground.
const HEIGHT: f32 = 0.75;
const TOP_SIZE: Vec3 = Vec3::new(1.6, 0.04, 0.8);
const LEG_WIDTH: f32 = 0.05;
/// How far the legs sit in from the edges of the top.
const LEG_INSET: f32 = 0.025;
const MAT_SIZE: Vec3 = Vec3::new(0.7, 0.004, 0.42);
/// Where the mat lies, see [`Block::top_spot`].
const MAT_SPOT: Vec2 = Vec2::new(0.0, 0.1);

/// Eye height of the seated player above the desk top.
const EYE_HEIGHT: f32 = 0.5;
/// Distance of the player's eyes from the front edge of the desk.
const EYE_DISTANCE: f32 = 0.45;
/// Where the seated player looks, see [`Block::top_spot`].
const LOOK_SPOT: Vec2 = Vec2::new(0.0, -0.5);

const WOOD: Color = Color::srgb_u8(0xc4, 0x9a, 0x6c);
const WOOD_DARK: Color = Color::srgb_u8(0x8e, 0x6a, 0x4a);
const MAT: Color = Color::srgb_u8(0x4f, 0x7a, 0x6b);

pub(crate) struct DeskPlugin;

impl Plugin for DeskPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Desk::on_add);
    }
}

/// A desk resting on the ground.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Desk;

impl Desk {
    fn on_add(add: On<Add<Desk>>, mut commands: Commands) {
        let layout = DeskLayout::new();
        commands.entity(add.entity).with_children(|desk| {
            desk.spawn(Shape::cuboid(&layout.top, WOOD));
            desk.spawn(Shape::cuboid(&layout.mat, MAT));
            for leg in &layout.legs {
                desk.spawn(Shape::cuboid(leg, WOOD_DARK));
            }
        });
    }
}

/// Geometry of a [`Desk`] in its own space.
pub(crate) struct DeskLayout {
    top: Block,
    legs: [Block; 4],
    mat: Block,
}

impl DeskLayout {
    pub(crate) fn new() -> Self {
        let top = Block::new(TOP_SIZE).translated(Vec3::Y * (HEIGHT - TOP_SIZE.y));
        let leg = Block::new(Vec3::new(LEG_WIDTH, HEIGHT - TOP_SIZE.y, LEG_WIDTH));
        let corner = TOP_SIZE.xz() / 2.0 - LEG_INSET - LEG_WIDTH / 2.0;
        let legs = [
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(1.0, 1.0),
        ]
        .map(|side| leg.translated((side * corner).extend(0.0).xzy()));
        let mat = Block::new(MAT_SIZE).resting_on(&top, MAT_SPOT);
        Self { top, legs, mat }
    }

    /// Something resting on the desk top at `spot` (see [`Block::top_spot`]),
    /// turned by `yaw` radians.
    pub(crate) fn place(&self, spot: Vec2, yaw: f32) -> Transform {
        Transform::from_translation(self.top.top_spot(spot))
            .with_rotation(Quat::from_rotation_y(yaw))
    }

    /// Camera pose of the seated player.
    pub(crate) fn seated_view(&self) -> Transform {
        let eye = Vec3::new(
            0.0,
            self.top.max().y + EYE_HEIGHT,
            self.top.max().z + EYE_DISTANCE,
        );
        Transform::from_translation(eye).looking_at(self.top.top_spot(LOOK_SPOT), Vec3::Y)
    }
}
