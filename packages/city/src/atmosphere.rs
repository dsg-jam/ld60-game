mod height_fog;

use bevy::{
    camera::Exposure,
    ecs::schedule::common_conditions::any_match_filter,
    light::{FogVolume, VolumetricFog},
    pbr::ScreenSpaceReflections,
    post_process::bloom::Bloom,
    prelude::*,
};
use ld60_core::clock::Clock;
use ld60_core::layout::shop;
use ld60_core::player::camera::PlayerCamera;
use ld60_core::player::focus::Focus;
use ld60_core::settings::Settings;
use ld60_geometry::block::Block;

use self::height_fog::HeightFog;
use super::FACADES;
use super::weather::Weather;

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
/// While the player reads a paper up close, the bright page would glow over
/// its own print.
const READING_BLOOM_INTENSITY: f32 = 0.05;
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
            .add_systems(Startup, (Self::spawn_planet_light, StreetHaze::spawn))
            .add_systems(
                Update,
                (
                    (Self::thicken_fog, StreetHaze::thicken).run_if(resource_changed::<Weather>),
                    Self::follow_planet.run_if(resource_changed::<Clock>),
                    Self::dim_bloom_for_reading.run_if(state_changed::<Focus>),
                    Self::match_quality.run_if(
                        resource_changed::<Settings>
                            .or_else(any_match_filter::<Added<PlayerCamera>>),
                    ),
                ),
            )
            .add_observer(Self::dress_camera);
    }
}

/// Marks the [`FogVolume`] over the street, which thickens in the rain.
#[derive(Component, Debug)]
struct StreetHaze;

/// Marks the [`DirectionalLight`] of the planet.
#[derive(Component, Debug)]
struct PlanetLight;

impl AtmospherePlugin {
    fn spawn_planet_light(clock: Res<Clock>, mut commands: Commands) {
        // No shadows: the city lights drown them out, and every building
        // would have to be drawn into them.
        commands.spawn((
            Name::new("planet light"),
            PlanetLight,
            DirectionalLight {
                color: MOON,
                illuminance: MOON_ILLUMINANCE * clock.planet_light(),
                ..default()
            },
            Self::planet_view(&clock),
        ));
    }

    /// Looking from the planet down at the shop.
    fn planet_view(clock: &Clock) -> Transform {
        Transform::from_translation(clock.planet()).looking_at(Vec3::ZERO, Vec3::Y)
    }

    /// The light falls from where the planet is and fades as it sets.
    fn follow_planet(
        clock: Res<Clock>,
        mut planet: Query<(&mut DirectionalLight, &mut Transform), With<PlanetLight>>,
    ) {
        for (mut light, mut transform) in &mut planet {
            light.illuminance = MOON_ILLUMINANCE * clock.planet_light();
            *transform = Self::planet_view(&clock);
        }
    }

    fn dim_bloom_for_reading(
        focus: Res<State<Focus>>,
        mut blooms: Query<&mut Bloom, With<PlayerCamera>>,
    ) {
        let intensity = if *focus.get() == Focus::Paper {
            READING_BLOOM_INTENSITY
        } else {
            BLOOM_INTENSITY
        };
        for mut bloom in &mut blooms {
            bloom.intensity = intensity;
        }
    }

    /// Rain makes the air hazier.
    fn thicken_fog(weather: Res<Weather>, mut fogs: Query<&mut HeightFog, With<PlayerCamera>>) {
        for mut fog in &mut fogs {
            fog.density = FOG_DENSITY + RAIN_FOG_DENSITY * weather.rain;
        }
    }

    /// Drops the costly effects on lower [`Quality`](ld60_core::settings::Settings::quality).
    fn match_quality(
        settings: Res<Settings>,
        cameras: Query<Entity, With<PlayerCamera>>,
        mut commands: Commands,
    ) {
        let quality = settings.quality();
        for camera in &cameras {
            let mut camera = commands.entity(camera);
            if quality.reflections() {
                camera.insert(ScreenSpaceReflections {
                    // Only the puddles reflect. Blurry reflections on damp
                    // ground are noisy without temporal anti-aliasing, which
                    // would smear the rain.
                    min_perceptual_roughness: 0.0..0.01,
                    max_perceptual_roughness: 0.1..0.15,
                    ..default()
                });
            } else {
                camera.remove::<ScreenSpaceReflections>();
            }
            match quality.haze_steps() {
                Some(step_count) => camera.insert(VolumetricFog {
                    ambient_intensity: 0.0,
                    step_count,
                    ..default()
                }),
                None => camera.remove::<VolumetricFog>(),
            };
        }
    }

    /// Night look for the player's view: glowing lights and hazy distance.
    fn dress_camera(add: On<Add<PlayerCamera>>, mut commands: Commands) {
        commands.entity(add.entity).insert((
            Bloom {
                intensity: BLOOM_INTENSITY,
                ..Bloom::NATURAL
            },
            Exposure::INDOOR,
            HeightFog {
                color: FOG.to_linear().to_vec4(),
                density: FOG_DENSITY,
                falloff: 1.0 / FOG_HEIGHT,
            },
        ));
    }
}

impl StreetHaze {
    /// Haze between the buildings, where the lamps and signs light up the air.
    fn spawn(mut commands: Commands) {
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
    fn thicken(weather: Res<Weather>, mut volumes: Query<&mut FogVolume, With<StreetHaze>>) {
        for mut volume in &mut volumes {
            volume.density_factor = HAZE_DENSITY + RAIN_HAZE_DENSITY * weather.rain;
        }
    }
}
