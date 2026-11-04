mod feathers;

use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use rand::{Rng, RngExt};

pub(super) use self::feathers::{Cover, Feather, Feathers, Kind};
use super::Pick;

/// Wet feathers show it in steps this small, so the materials only change
/// now and then.
const WET_STEP: f32 = 0.05;
const ALPHA_CUTOFF: f32 = 0.5;

/// Feathers of every kind.
const PALETTES: [Palette; 8] = [
    Palette::new(0x5a6068, 0x3a6a4a, Pattern::Plain, 0.9),
    Palette::new(0x6a4a30, 0xd8c8a0, Pattern::Barred, 0.0),
    Palette::new(0x2e2c30, 0xb0502a, Pattern::Tipped, 0.3),
    Palette::new(0x5a6230, 0xd0b040, Pattern::Tipped, 0.2),
    Palette::new(0x3a5a64, 0xe07a30, Pattern::Spotted, 0.4),
    Palette::new(0xc8bca8, 0x2a2420, Pattern::Tipped, 0.0),
    Palette::new(0x2c2e50, 0xb08040, Pattern::Plain, 1.0),
    Palette::new(0x7a5a48, 0x3a2a24, Pattern::Spotted, 0.1),
];

pub(super) struct PlumagePlugin;

impl Plugin for PlumagePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "plumage.wesl");
        app.add_plugins(MaterialPlugin::<PlumageMaterial>::default());
    }
}

pub(super) type PlumageMaterial = ExtendedMaterial<StandardMaterial, Plumage>;

/// The look of a bird's feathers. See `plumage.wesl`.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(super) struct Plumage {
    #[uniform(100)]
    params: PlumageParams,
}

/// Matches `Plumage` in `plumage.wesl`.
#[derive(ShaderType, Reflect, Debug, Clone)]
struct PlumageParams {
    accent: Vec4,
    pattern: u32,
    wet: f32,
    sheen: f32,
    seed: f32,
}

/// Matches the constants in `plumage.wesl`.
#[derive(Debug, Clone, Copy)]
enum Pattern {
    Plain = 0,
    /// Bands across each feather.
    Barred = 1,
    /// A tip of another color.
    Tipped = 2,
    /// A spot near the tip.
    Spotted = 3,
}

#[derive(Debug, Clone, Copy)]
struct Palette {
    base: u32,
    accent: u32,
    pattern: Pattern,
    /// How much the neck and head shimmer, from 0 to 1.
    sheen: f32,
}

/// One bird's feathers.
#[derive(Debug, Clone)]
pub(super) struct Coat {
    pub(super) feathers: Handle<PlumageMaterial>,
    /// How soaked it is, from 0 to 1, as last shown.
    shown_wet: f32,
}

impl Palette {
    const fn new(base: u32, accent: u32, pattern: Pattern, sheen: f32) -> Self {
        Self {
            base,
            accent,
            pattern,
            sheen,
        }
    }

    fn color(rgb: u32) -> Color {
        Color::srgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }
}

impl Coat {
    /// A coat of one of the palettes, a little different from any other.
    pub(super) fn new(plumages: &mut Assets<PlumageMaterial>, rng: &mut impl Rng) -> Self {
        let palette = PALETTES.pick(rng);
        let tint = rng.random_range(0.85..1.15f32);
        let base = Palette::color(palette.base).to_linear() * tint;
        let feathers = plumages.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: base.into(),
                alpha_mode: AlphaMode::Mask(ALPHA_CUTOFF),
                double_sided: true,
                cull_mode: None,
                ..default()
            },
            extension: Plumage {
                params: PlumageParams {
                    accent: Palette::color(palette.accent).to_linear().to_vec4(),
                    pattern: palette.pattern as u32,
                    wet: 0.0,
                    sheen: palette.sheen,
                    seed: rng.random_range(0.0..10.0),
                },
            },
        });
        Self {
            feathers,
            shown_wet: 0.0,
        }
    }

    /// Shows how soaked the feathers are, from 0 to 1.
    pub(super) fn soak(&mut self, wet: f32, plumages: &mut Assets<PlumageMaterial>) {
        let stepped = (wet / WET_STEP).round() * WET_STEP;
        if stepped == self.shown_wet {
            return;
        }
        self.shown_wet = stepped;
        if let Some(mut material) = plumages.get_mut(&self.feathers) {
            material.extension.params.wet = stepped;
        }
    }
}

impl Plumage {
    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("plumage.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl MaterialExtension for Plumage {
    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        Self::shader()
    }
}
