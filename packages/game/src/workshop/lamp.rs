use bevy::{light::NotShadowCaster, prelude::*};

use super::switch::LightSwitch;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;

const BASE_SIZE: Vec3 = Vec3::new(0.14, 0.02, 0.14);
const POLE_SIZE: Vec3 = Vec3::new(0.016, 0.46, 0.016);
const ARM_SIZE: Vec3 = Vec3::new(0.3, 0.016, 0.016);
const SHADE_SIZE: Vec3 = Vec3::new(0.15, 0.09, 0.15);
/// How far the bulb hangs below the shade.
const BULB_DROP: f32 = 0.025;

const LIGHT_COLOR: Color = Color::srgb(1.0, 0.85, 0.65);
const LIGHT_INTENSITY: f32 = 6_000.0;
const LIGHT_RANGE: f32 = 4.0;
/// Much smaller than the default, so thin walls cast solid shadows.
const SHADOW_DEPTH_BIAS: f32 = 0.01;
const SHADOW_NORMAL_BIAS: f32 = 0.3;
const BULB_RADIUS: f32 = 0.022;
/// Bright enough to bloom.
const BULB_GLOW: LinearRgba = LinearRgba::rgb(12.0, 9.0, 6.0);

const SHADE: Color = Color::srgb_u8(0xe0, 0x8a, 0x4f);
const METAL: Color = Color::srgb_u8(0x7d, 0x86, 0x8c);

pub(crate) struct LampPlugin;

impl Plugin for LampPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Lamp::on_add);
    }
}

/// A desk lamp with its arm reaching towards +X.
///
/// Clicking it in the desk view switches it on or off.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Lamp;

impl Lamp {
    fn on_add(
        add: On<Add<Lamp>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let base = Block::new(BASE_SIZE);
        let pole = Block::new(POLE_SIZE).on_top_of(&base);
        let arm = Block::new(ARM_SIZE)
            .on_top_of(&pole)
            .translated(Vec3::X * (ARM_SIZE.x - POLE_SIZE.x) / 2.0);
        let shade = Block::new(SHADE_SIZE)
            .below(&arm)
            .translated(Vec3::X * (arm.max().x - SHADE_SIZE.x / 2.0));
        let bulb = shade.bottom() - Vec3::Y * BULB_DROP;

        let bulb_material = materials.add(StandardMaterial {
            base_color: LIGHT_COLOR,
            emissive: BULB_GLOW,
            ..default()
        });
        let mut light = Entity::PLACEHOLDER;
        commands.entity(add.entity).with_children(|lamp| {
            lamp.spawn(Shape::cylinder(&base, SHADE));
            lamp.spawn(Shape::cylinder(&pole, METAL));
            lamp.spawn(Shape::cuboid(&arm, METAL));
            lamp.spawn(Shape::cylinder(&shade, SHADE));
            lamp.spawn((
                Mesh3d(meshes.add(Sphere::new(BULB_RADIUS))),
                MeshMaterial3d(bulb_material.clone()),
                NotShadowCaster,
                Transform::from_translation(bulb),
            ));
            light = lamp
                .spawn((
                    PointLight {
                        color: LIGHT_COLOR,
                        intensity: LIGHT_INTENSITY,
                        range: LIGHT_RANGE,
                        shadow_maps_enabled: true,
                        shadow_depth_bias: SHADOW_DEPTH_BIAS,
                        shadow_normal_bias: SHADOW_NORMAL_BIAS,
                        ..default()
                    },
                    Transform::from_translation(bulb),
                ))
                .id();
        });
        commands.entity(add.entity).insert(LightSwitch::new(
            light,
            LIGHT_INTENSITY,
            bulb_material,
            BULB_GLOW,
        ));
    }
}
