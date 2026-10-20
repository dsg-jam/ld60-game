mod cubemap;

use bevy::prelude::*;

use self::cubemap::Lobe;
use super::lamp::Lamp;
use crate::player::camera::PlayerCamera;

/// Texels along the side of a face of the reflections.
const SIZE: u32 = 64;
/// Scales the radiance of the lobes to candela per square meter.
const INTENSITY: f32 = 150.0;

/// What shiny things around the counter reflect, as seen from the middle of
/// the desk. The street lies towards -Z.
const SURROUNDINGS: [Lobe; 9] = [
    // The dark awning above and the shop behind.
    Lobe::new(Vec3::Y, 1.5, Vec3::new(0.02, 0.016, 0.014)),
    Lobe::new(Vec3::new(0.0, 0.3, 1.0), 2.0, Vec3::new(0.05, 0.035, 0.025)),
    // The wet street, and the signs across it. Screen space reflections
    // show the real signs in puddles, so these stay as dim as those.
    Lobe::new(Vec3::new(0.0, 0.1, -1.0), 4.0, Vec3::new(0.05, 0.05, 0.1)),
    Lobe::new(
        Vec3::new(0.35, 0.35, -1.0),
        150.0,
        Vec3::new(0.05, 0.3, 0.4),
    ),
    Lobe::new(
        Vec3::new(-0.5, 0.25, -1.0),
        200.0,
        Vec3::new(0.4, 0.08, 0.27),
    ),
    Lobe::new(
        Vec3::new(0.15, 0.15, -1.0),
        200.0,
        Vec3::new(0.35, 0.28, 0.08),
    ),
    Lobe::new(
        Vec3::new(0.0, 0.08, -1.0),
        30.0,
        Vec3::new(0.03, 0.12, 0.05),
    ),
    // The screen of the computer.
    Lobe::new(
        Vec3::new(1.0, 0.15, -0.15),
        40.0,
        Vec3::new(0.12, 0.45, 0.22),
    ),
    // The desk in the dark.
    Lobe::new(Vec3::NEG_Y, 1.5, Vec3::new(0.04, 0.025, 0.015)),
];

/// What the desk lamp adds while it is on.
const LAMP: [Lobe; 2] = [
    // The glowing shade, and the pool of light it throws on the desk.
    Lobe::new(Vec3::new(-0.2, 0.93, -0.3), 60.0, Vec3::new(4.0, 1.8, 0.8)),
    Lobe::new(Vec3::new(-0.1, -1.0, -0.15), 3.0, Vec3::new(0.9, 0.5, 0.25)),
];

pub(super) struct ReflectionsPlugin;

impl Plugin for ReflectionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Reflections>()
            .add_observer(Reflections::dress_camera)
            .add_systems(Update, Reflections::follow_lamp);
    }
}

/// The surroundings of the counter for the [`EnvironmentMapLight`] of the
/// player's camera, with the lamp on and off. Only shiny things reflect
/// them. Matte things keep the light they get from the ambient light.
#[derive(Resource, Debug)]
struct Reflections {
    black: Handle<Image>,
    lit: Handle<Image>,
    dark: Handle<Image>,
}

impl FromWorld for Reflections {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        let lit: Vec<Lobe> = SURROUNDINGS.iter().chain(&LAMP).copied().collect();
        Self {
            black: images.add(cubemap::black()),
            lit: images.add(cubemap::specular(&lit, SIZE)),
            dark: images.add(cubemap::specular(&SURROUNDINGS, SIZE)),
        }
    }
}

impl Reflections {
    fn light(&self, lamp_on: bool) -> EnvironmentMapLight {
        EnvironmentMapLight {
            diffuse_map: self.black.clone(),
            specular_map: if lamp_on {
                self.lit.clone()
            } else {
                self.dark.clone()
            },
            intensity: INTENSITY,
            ..default()
        }
    }

    fn dress_camera(
        add: On<Add<PlayerCamera>>,
        reflections: Res<Reflections>,
        mut commands: Commands,
    ) {
        commands.entity(add.entity).insert(reflections.light(true));
    }

    fn follow_lamp(
        lamps: Query<&Lamp, Changed<Lamp>>,
        reflections: Res<Reflections>,
        mut cameras: Query<&mut EnvironmentMapLight, With<PlayerCamera>>,
    ) {
        for lamp in &lamps {
            for mut light in &mut cameras {
                *light = reflections.light(lamp.is_on());
            }
        }
    }
}
