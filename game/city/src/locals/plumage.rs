mod feathers;

use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use ld60_geometry::surface::SurfaceMaterial;
use rand::{Rng, RngExt};

pub(super) use self::feathers::{Cover, Feather, Feathers, Kind, Region, point_along};

/// Wet feathers show it in steps this small, so the materials only change
/// now and then.
const WET_STEP: f32 = 0.05;
const ALPHA_CUTOFF: f32 = 0.5;

/// Herons of a few kinds: grey, great blue, great white, purple, black and
/// slate.
const SPECIES: [Species; 6] = [
    Species::new(0x7c838c, 0xdedcd6, 0x26282c, 0x8a7a58),
    Species::new(0x56657a, 0xc8c4bc, 0x22252c, 0x6e6450),
    Species::new(0xe2e0da, 0xf0eee8, 0xc4c0b8, 0x2a2826),
    Species::new(0x5e5650, 0x9a6040, 0x24201e, 0x8a7448),
    Species::new(0x34363c, 0x46484e, 0x1c1c20, 0x3a3634),
    Species::new(0x4a5560, 0xb8bcc0, 0x1e2228, 0x7a7458),
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
    back: Vec4,
    pale: Vec4,
    dark: Vec4,
    wet: f32,
}

/// The colors of one kind of heron.
#[derive(Debug, Clone, Copy)]
struct Species {
    back: u32,
    pale: u32,
    dark: u32,
    /// The legs, the hands and the beak.
    skin: u32,
}

/// One bird's feathers, and its skin to go with them.
#[derive(Debug, Clone)]
pub(super) struct Coat {
    pub(super) feathers: Handle<PlumageMaterial>,
    pub(super) skin: Handle<SurfaceMaterial>,
    /// How soaked it is, from 0 to 1, as last shown.
    shown_wet: f32,
}

impl Species {
    const fn new(back: u32, pale: u32, dark: u32, skin: u32) -> Self {
        Self {
            back,
            pale,
            dark,
            skin,
        }
    }

    fn color(rgb: u32) -> Color {
        Color::srgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }
}

impl Coat {
    /// The skin colors, one for each kind of heron, in the order
    /// [`Coat::new`] expects them.
    pub(super) fn skin_colors() -> impl Iterator<Item = Color> {
        SPECIES.iter().map(|species| Species::color(species.skin))
    }

    /// A coat of one of the kinds of heron, a little different from any
    /// other, with one of `skins` from [`Coat::skin_colors`].
    pub(super) fn new(
        plumages: &mut Assets<PlumageMaterial>,
        skins: &[Handle<SurfaceMaterial>],
        rng: &mut impl Rng,
    ) -> Self {
        let index = rng.random_range(0..SPECIES.len());
        let species = SPECIES[index];
        let tint = rng.random_range(0.92..1.08f32);
        let color = |rgb| (Species::color(rgb).to_linear() * tint).to_vec4();
        let feathers = plumages.add(ExtendedMaterial {
            base: StandardMaterial {
                alpha_mode: AlphaMode::Mask(ALPHA_CUTOFF),
                double_sided: true,
                cull_mode: None,
                ..default()
            },
            extension: Plumage {
                params: PlumageParams {
                    back: color(species.back),
                    pale: color(species.pale),
                    dark: color(species.dark),
                    wet: 0.0,
                },
            },
        });
        Self {
            feathers,
            skin: skins[index].clone(),
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
