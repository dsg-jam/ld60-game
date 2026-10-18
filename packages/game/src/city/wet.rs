use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

use super::weather::Weather;
use super::{FAR_CURB, NEAR_CURB};

/// How dark and glossy the ground gets when fully wet.
const WETNESS: f32 = 0.6;

pub(super) struct WetPlugin;

impl Plugin for WetPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "wet.wesl");
        app.add_plugins(MaterialPlugin::<WetMaterial>::default())
            .add_systems(
                Update,
                Wet::follow_weather.run_if(resource_changed::<Weather>),
            );
    }
}

pub(super) type WetMaterial = ExtendedMaterial<StandardMaterial, Wet>;

/// Rain water on the ground: darker and glossier, with puddles that ripple.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(super) struct Wet {
    #[uniform(100)]
    params: WetParams,
    /// Share of the surface under water when soaked.
    puddles: f32,
}

/// Matches `Wet` in `wet.wesl`.
#[derive(ShaderType, Reflect, Debug, Clone)]
struct WetParams {
    wetness: f32,
    puddles: f32,
    ripples: f32,
    seed: f32,
    curbs: Vec2,
}

impl Wet {
    /// A `surface` that gets wet in the rain, with up to `puddles` of it
    /// under water.
    pub(super) fn material(surface: StandardMaterial, puddles: f32, seed: f32) -> WetMaterial {
        ExtendedMaterial {
            base: surface,
            extension: Self {
                params: WetParams {
                    wetness: 0.0,
                    puddles: 0.0,
                    ripples: 0.0,
                    seed,
                    curbs: Vec2::new(NEAR_CURB, FAR_CURB),
                },
                puddles,
            },
        }
    }

    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("wet.wesl"))
            .with_source("embedded")
            .into()
    }

    fn follow_weather(weather: Res<Weather>, mut materials: ResMut<Assets<WetMaterial>>) {
        for (_, material) in materials.iter_mut() {
            let wet = &mut material.extension;
            wet.params.wetness = weather.wetness * WETNESS;
            wet.params.puddles = weather.wetness * wet.puddles;
            wet.params.ripples = weather.rain;
        }
    }
}

impl MaterialExtension for Wet {
    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        Self::shader()
    }
}
