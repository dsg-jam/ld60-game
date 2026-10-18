mod height_fog;

use bevy::{
    camera::Exposure,
    light::{FogVolume, VolumetricFog},
    pbr::ScreenSpaceReflections,
    post_process::bloom::Bloom,
    prelude::*,
};

use self::height_fog::HeightFog;
use super::FACADES;
use super::sky::PLANET;
use super::weather::Weather;
use crate::geometry::block::Block;
use crate::player::camera::PlayerCamera;
use crate::workshop::shop;

/// Air lit by the city, which everything fades into far away.
const FOG: Color = Color::srgb_u8(0x42, 0x36, 0x3e);
/// The clear night sky above the haze.
pub(super) const SKY: Color = Color::srgb_u8(0x14, 0x18, 0x30);
/// How much a meter of air hides at street level.
const FOG_DENSITY: f32 = 0.016;
/// Extra fog in full rain.
const RAIN_FOG_DENSITY: f32 = 0.008;
/// The haze thins out to a third every this many meters up.
const FOG_HEIGHT: f32 = 12.0;
/// Faint light from the city that reaches everywhere.
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
/// Samples along each view ray through the haze.
const HAZE_STEPS: u32 = 64;
/// The street haze reaches this high, in meters.
const HAZE_HEIGHT: f32 = 12.0;

pub(super) struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(height_fog::HeightFogPlugin)
            .insert_resource(ClearColor(SKY))
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

/// Marks the [`FogVolume`] over the street, which thickens in the rain.
#[derive(Component, Debug)]
struct StreetHaze;

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
        StreetHaze,
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
    mut fogs: Query<&mut HeightFog, With<PlayerCamera>>,
    mut volumes: Query<&mut FogVolume, With<StreetHaze>>,
) {
    for mut fog in &mut fogs {
        fog.density = FOG_DENSITY + RAIN_FOG_DENSITY * weather.rain;
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
        HeightFog {
            color: FOG.to_linear().to_vec4(),
            density: FOG_DENSITY,
            falloff: 1.0 / FOG_HEIGHT,
            ground: 0.0,
        },
    ));
}
