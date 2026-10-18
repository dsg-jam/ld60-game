use bevy::{
    camera::Exposure,
    light::{FogVolume, VolumetricFog},
    pbr::ScreenSpaceReflections,
    post_process::bloom::Bloom,
    prelude::*,
};

use super::FACADES;
use super::sky::PLANET;
use super::weather::Weather;
use crate::geometry::block::Block;
use crate::player::camera::PlayerCamera;
use crate::workshop::shop;

/// Night sky, also the color everything fades into far away.
pub(super) const HAZE: Color = Color::srgb_u8(0x3a, 0x2c, 0x58);
const FOG_DENSITY: f32 = 0.007;
/// Extra fog in full rain.
const RAIN_FOG_DENSITY: f32 = 0.004;
const AMBIENT: Color = Color::srgb_u8(0x6a, 0x60, 0xa0);
const AMBIENT_BRIGHTNESS: f32 = 20.0;
/// Light reflected by the planet in the sky.
const MOON: Color = Color::srgb_u8(0x9a, 0xa8, 0xd0);
const MOON_ILLUMINANCE: f32 = 15.0;
const BLOOM_INTENSITY: f32 = 0.35;
/// Haze over the street that lights shine through.
const STREET_HAZE: Color = Color::srgb_u8(0xc8, 0xc0, 0xd8);
const HAZE_DENSITY: f32 = 0.02;
/// Extra haze in full rain.
const RAIN_HAZE_DENSITY: f32 = 0.03;
/// Bevy dims the light in a fog volume by the size of the whole volume, so
/// the haze only covers the stretch of street around the shop.
const HAZE_HALF_LENGTH: f32 = 25.0;
/// Makes up for the thin haze.
const HAZE_GLOW: f32 = 8.0;
const HAZE_STEPS: u32 = 64;
const HAZE_HEIGHT: f32 = 12.0;

pub(super) struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(HAZE))
            .insert_resource(GlobalAmbientLight {
                color: AMBIENT,
                brightness: AMBIENT_BRIGHTNESS,
                ..default()
            })
            .add_systems(Startup, (spawn_planet_light, spawn_street_haze))
            .add_systems(Update, thicken_fog.run_if(resource_changed::<Weather>))
            .add_observer(dress_camera);
    }
}

fn spawn_planet_light(mut commands: Commands) {
    commands.spawn((
        Name::new("planet light"),
        DirectionalLight {
            color: MOON,
            illuminance: MOON_ILLUMINANCE,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_translation(PLANET).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Haze between the buildings, where the lamps and signs light up the air.
fn spawn_street_haze(mut commands: Commands) {
    let street = Block::between(
        Vec3::new(-HAZE_HALF_LENGTH, -1.0, FACADES),
        Vec3::new(HAZE_HALF_LENGTH, HAZE_HEIGHT, shop::FRONT),
    );
    commands.spawn((
        Name::new("street haze"),
        FogVolume {
            fog_color: STREET_HAZE,
            density_factor: HAZE_DENSITY,
            light_intensity: HAZE_GLOW,
            ..default()
        },
        street.transform().with_scale(street.size),
    ));
}

/// Rain makes the air hazier.
fn thicken_fog(
    weather: Res<Weather>,
    mut fogs: Query<&mut DistanceFog, With<PlayerCamera>>,
    mut volumes: Query<&mut FogVolume>,
) {
    for mut fog in &mut fogs {
        fog.falloff = FogFalloff::Exponential {
            density: FOG_DENSITY + RAIN_FOG_DENSITY * weather.rain,
        };
    }
    for mut volume in &mut volumes {
        volume.density_factor = HAZE_DENSITY + RAIN_HAZE_DENSITY * weather.rain;
    }
}

/// Night look for the player's view: glowing lights and hazy distance.
fn dress_camera(add: On<Add<PlayerCamera>>, mut commands: Commands) {
    commands.entity(add.entity).insert((
        ScreenSpaceReflections {
            // Only the puddles reflect. Blurry reflections on damp ground
            // are noisy without temporal anti-aliasing, which would smear
            // the rain.
            min_perceptual_roughness: 0.0..0.01,
            max_perceptual_roughness: 0.1..0.15,
            ..default()
        },
        Bloom {
            intensity: BLOOM_INTENSITY,
            ..Bloom::NATURAL
        },
        VolumetricFog {
            ambient_intensity: 0.0,
            step_count: HAZE_STEPS,
            ..default()
        },
        Exposure::INDOOR,
        DistanceFog {
            color: HAZE,
            falloff: FogFalloff::Exponential {
                density: FOG_DENSITY,
            },
            ..default()
        },
    ));
}
