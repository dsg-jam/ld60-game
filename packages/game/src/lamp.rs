use bevy::prelude::*;

use crate::layout::Block;
use crate::shape::Shape;

const BASE_SIZE: Vec3 = Vec3::new(0.14, 0.02, 0.14);
const POLE_SIZE: Vec3 = Vec3::new(0.016, 0.46, 0.016);
const ARM_SIZE: Vec3 = Vec3::new(0.3, 0.016, 0.016);
const SHADE_SIZE: Vec3 = Vec3::new(0.15, 0.09, 0.15);
/// How far the bulb hangs below the shade.
const BULB_DROP: f32 = 0.025;

const LIGHT_COLOR: Color = Color::srgb(1.0, 0.85, 0.65);
const LIGHT_INTENSITY: f32 = 4_000.0;
const LIGHT_RANGE: f32 = 4.0;

const SHADE: Color = Color::srgb_u8(0xe0, 0x8a, 0x4f);
const METAL: Color = Color::srgb_u8(0x7d, 0x86, 0x8c);

pub(crate) struct LampPlugin;

impl Plugin for LampPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Lamp::on_add);
    }
}

/// A desk lamp with its arm reaching towards +X.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Lamp;

impl Lamp {
    fn on_add(add: On<Add<Lamp>>, mut commands: Commands) {
        let base = Block::new(BASE_SIZE);
        let pole = Block::new(POLE_SIZE).on_top_of(&base);
        let arm = Block::new(ARM_SIZE)
            .on_top_of(&pole)
            .translated(Vec3::X * (ARM_SIZE.x - POLE_SIZE.x) / 2.0);
        let shade = Block::new(SHADE_SIZE)
            .below(&arm)
            .translated(Vec3::X * (arm.max().x - SHADE_SIZE.x / 2.0));
        let bulb = shade.bottom() - Vec3::Y * BULB_DROP;

        commands.entity(add.entity).with_children(|lamp| {
            lamp.spawn(Shape::cylinder(&base, SHADE));
            lamp.spawn(Shape::cylinder(&pole, METAL));
            lamp.spawn(Shape::cuboid(&arm, METAL));
            lamp.spawn(Shape::cylinder(&shade, SHADE));
            lamp.spawn((
                PointLight {
                    color: LIGHT_COLOR,
                    intensity: LIGHT_INTENSITY,
                    range: LIGHT_RANGE,
                    shadow_maps_enabled: true,
                    ..default()
                },
                Transform::from_translation(bulb),
            ));
        });
    }
}
