use std::f32::consts::TAU;

use bevy::{
    asset::RenderAssetUsages,
    light::{FogVolume, VolumetricLight},
    material::OpaqueRendererMethod,
    mesh::{Indices, PrimitiveTopology},
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};

use super::dust::Dust;
use crate::audio::{Play, Sound};
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;
use crate::parts::bulb::Bulb;
use crate::player::focus::{Clickable, Focus};

pub(super) const BASE_SIZE: Vec3 = Vec3::new(0.14, 0.02, 0.14);
const POLE_SIZE: Vec3 = Vec3::new(0.016, 0.46, 0.016);
const ARM_SIZE: Vec3 = Vec3::new(0.3, 0.016, 0.016);
const SHADE_TOP: f32 = 0.04;
const SHADE_BOTTOM: f32 = 0.08;
const SHADE_HEIGHT: f32 = 0.12;
const SHADE_SEGMENTS: u32 = 32;
/// Light that shines through the fabric.
const SHADE_TRANSMISSION: f32 = 0.15;
const SOCKET: Vec3 = Vec3::new(0.026, 0.03, 0.026);
const BULB_RADIUS: f32 = 0.024;
/// The bulb hangs from the socket, inside the shade.
const BULB_DROP: f32 = SOCKET.y + BULB_RADIUS * 0.8;
/// The frosted glass glows a little around the filament.
const GLASS_GLOW: f32 = 6.0;

const LIGHT_COLOR: Color = Color::srgb(1.0, 0.85, 0.65);
const LIGHT_INTENSITY: f32 = 6_000.0;
const LIGHT_RANGE: f32 = 4.0;
/// Dust floats in the light under the shade.
const DUST: Dust = Dust {
    cloud: Block {
        center: Vec3::new(0.2, 0.3, 0.0),
        size: Vec3::new(0.8, 0.6, 0.8),
    },
    motes: 80,
    seed: 11,
};
/// Share of the light cone at full strength, before it fades out at the
/// rim of the shade.
const SOFT_EDGE: f32 = 0.75;
/// Soft light that comes through the shade all around.
const FILL_INTENSITY: f32 = 1_200.0;
/// Dusty air, which the light cone shows up in.
const HAZE: Color = Color::srgb_u8(0xd8, 0xcc, 0xb8);
const HAZE_DENSITY: f32 = 0.04;
const HAZE_GLOW: f32 = 1.0;

const SHADE: Color = Color::srgb_u8(0xe0, 0x8a, 0x4f);
const METAL: Color = Color::srgb_u8(0x7d, 0x86, 0x8c);

pub(crate) struct LampPlugin;

impl Plugin for LampPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Lamp::on_add)
            .add_observer(Lamp::toggle_on_press);
    }
}

/// A desk lamp with its arm reaching towards +X. A click switches it.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Lamp {
    on: bool,
}

impl Default for Lamp {
    fn default() -> Self {
        Self { on: true }
    }
}

/// The light of a [`Lamp`], hidden while it is off.
#[derive(Component, Debug)]
struct LampLight;

impl Lamp {
    const fn clickable(&self) -> Clickable {
        Clickable {
            prompt: if self.on {
                "click-lamp-off"
            } else {
                "click-lamp-on"
            },
        }
    }

    fn on_add(
        add: On<Add<Lamp>>,
        lamps: Query<&Lamp>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let base = Block::new(BASE_SIZE);
        let pole = Block::new(POLE_SIZE).on_top_of(&base);
        let arm = Block::new(ARM_SIZE)
            .on_top_of(&pole)
            .translated(Vec3::X * (ARM_SIZE.x - POLE_SIZE.x) / 2.0);
        let top = Vec3::new(arm.max().x - SHADE_BOTTOM, arm.min().y, 0.0);
        let socket = Block::new(SOCKET).standing_at(top - Vec3::Y * SOCKET.y);
        let bulb = top - Vec3::Y * BULB_DROP;
        let Ok(lamp) = lamps.get(add.entity) else {
            return;
        };
        let (on, clickable) = (lamp.on, lamp.clickable());
        let light = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        commands
            .entity(add.entity)
            .insert((DUST, clickable))
            .with_children(|lamp| {
                lamp.spawn(Shape::cylinder(&base, SHADE));
                lamp.spawn((Shape::cylinder(&pole, METAL), Finish::Metal));
                lamp.spawn((Shape::cuboid(&arm, METAL), Finish::Metal));
                lamp.spawn((
                    Mesh3d(meshes.add(Self::shade())),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: SHADE,
                        perceptual_roughness: 0.9,
                        diffuse_transmission: SHADE_TRANSMISSION,
                        double_sided: true,
                        cull_mode: None,
                        // Deferred rendering has no room for transmission.
                        opaque_render_method: OpaqueRendererMethod::Forward,
                        ..default()
                    })),
                    Transform::from_translation(top - Vec3::Y * SHADE_HEIGHT),
                ));
                lamp.spawn((Shape::cylinder(&socket, METAL), Finish::Metal));
                lamp.spawn((
                    Bulb {
                        radius: BULB_RADIUS,
                        color: LIGHT_COLOR,
                        glow: GLASS_GLOW,
                        on,
                    },
                    Transform::from_translation(bulb),
                ));
                lamp.spawn((
                    LampLight,
                    light,
                    PointLight {
                        color: SHADE.mix(&LIGHT_COLOR, 0.5),
                        intensity: FILL_INTENSITY,
                        range: LIGHT_RANGE,
                        ..default()
                    },
                    Transform::from_translation(bulb),
                ));
                // Out of the bottom of the shade, as far as its rim lets it.
                let rim = SHADE_HEIGHT - BULB_DROP;
                let cone = (SHADE_BOTTOM / rim).atan();
                lamp.spawn((
                    LampLight,
                    light,
                    SpotLight {
                        color: LIGHT_COLOR,
                        intensity: LIGHT_INTENSITY,
                        range: LIGHT_RANGE,
                        radius: BULB_RADIUS,
                        shadow_maps_enabled: true,
                        // The shade is close to the bulb.
                        shadow_map_near_z: SOCKET.y / 2.0,
                        outer_angle: cone,
                        inner_angle: cone * SOFT_EDGE,
                        ..default()
                    },
                    VolumetricLight,
                    Transform::from_translation(bulb).looking_to(Vec3::NEG_Y, Vec3::X),
                ));
                // Only around the cone down to the desk, as the haze costs
                // for every pixel it covers.
                let spread = bulb.y * cone.tan();
                let haze = Block::between(
                    bulb.with_y(0.0) - Vec3::new(spread, 0.0, spread),
                    bulb + Vec3::new(spread, 0.0, spread),
                );
                lamp.spawn((
                    Name::new("lamp haze"),
                    FogVolume {
                        fog_color: HAZE,
                        density_factor: HAZE_DENSITY,
                        light_intensity: HAZE_GLOW,
                        ..default()
                    },
                    haze.transform().with_scale(haze.size),
                ));
            });
    }

    /// The fabric of the shade: a cone open at the bottom and closed at the
    /// top, from Y = 0 up to its height.
    fn shade() -> Mesh {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        // Outwards and slightly up, along the slanted side.
        let slope = (SHADE_BOTTOM - SHADE_TOP) / SHADE_HEIGHT;
        for index in 0..=SHADE_SEGMENTS {
            let angle = index as f32 / SHADE_SEGMENTS as f32 * TAU;
            let around = Vec3::new(angle.cos(), 0.0, angle.sin());
            let normal = (around + Vec3::Y * slope).normalize();
            positions.push(around * SHADE_BOTTOM);
            positions.push(around * SHADE_TOP + Vec3::Y * SHADE_HEIGHT);
            normals.extend([normal, normal]);
        }
        for index in 0..SHADE_SEGMENTS {
            let i = index * 2;
            indices.extend([i, i + 1, i + 2, i + 2, i + 1, i + 3]);
        }
        let center = positions.len() as u32;
        positions.push(Vec3::Y * SHADE_HEIGHT);
        normals.push(Vec3::Y);
        for index in 0..=SHADE_SEGMENTS {
            let angle = index as f32 / SHADE_SEGMENTS as f32 * TAU;
            positions.push(
                Vec3::new(angle.cos(), 0.0, angle.sin()) * SHADE_TOP + Vec3::Y * SHADE_HEIGHT,
            );
            normals.push(Vec3::Y);
        }
        for index in 0..SHADE_SEGMENTS {
            indices.extend([center, center + index + 2, center + index + 1]);
        }
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
    }

    fn toggle_on_press(
        mut press: On<PointerPress>,
        state: Res<State<Focus>>,
        mut lamps: Query<(&mut Lamp, &mut Clickable, &Children)>,
        mut lights: Query<&mut Visibility, With<LampLight>>,
        mut bulbs: Query<&mut Bulb>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || *state.get() != Focus::Desk {
            return;
        }
        let Ok((mut lamp, mut clickable, children)) = lamps.get_mut(press.entity) else {
            return;
        };
        press.propagate(false);
        lamp.on = !lamp.on;
        *clickable = lamp.clickable();
        for &child in children {
            if let Ok(mut visibility) = lights.get_mut(child) {
                visibility.toggle_inherited_hidden();
            }
            if let Ok(mut bulb) = bulbs.get_mut(child) {
                bulb.on = lamp.on;
            }
        }
        commands.trigger(Play(Sound::Click));
    }
}
