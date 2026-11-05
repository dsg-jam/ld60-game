mod feathers;

use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use ld60_geometry::surface::{Finish, SurfaceMaterial};
use rand::{Rng, RngExt};

pub(super) use self::feathers::{Cover, Feather, Feathers, Kind, Region, point_along};
use super::Pick;

/// Wet feathers show it in steps this small, so the materials only change
/// now and then.
const WET_STEP: f32 = 0.05;
const ALPHA_CUTOFF: f32 = 0.5;

/// Colors of scarves and cowls, muted so the plumage stays the main thing.
const CLOTHS: [u32; 7] = [
    0x7a2e2a, 0x9a7a34, 0x2a5658, 0x2c3250, 0x56603a, 0x5a324a, 0x8a8478,
];

/// Herons of a few kinds: grey, great blue, great white, purple, black and
/// slate.
const SPECIES: [Species; 6] = [
    Species::new(
        [0x7c838c, 0xdedcd6, 0x26282c],
        [0x8a7a58, 0xc8a848, 0xb0b070],
    ),
    Species::new(
        [0x56657a, 0xc8c4bc, 0x22252c],
        [0x6e6450, 0xc0a050, 0x8a9a70],
    ),
    Species::new(
        [0xe2e0da, 0xf0eee8, 0xc4c0b8],
        [0x2a2826, 0xd0b040, 0xa0b050],
    ),
    Species::new(
        [0x5e5650, 0x9a6040, 0x24201e],
        [0x8a7448, 0xa88a40, 0xb0a050],
    ),
    Species::new(
        [0x34363c, 0x46484e, 0x1c1c20],
        [0x3a3634, 0x2a2826, 0x3a3634],
    ),
    Species::new(
        [0x4a5560, 0xb8bcc0, 0x1e2228],
        [0x7a7458, 0xb8a050, 0xa8a070],
    ),
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
    /// The bare skin of the face.
    lore: Vec4,
    /// Whatever it wears.
    cloth: Vec4,
    wet: f32,
}

/// The colors of one kind of heron: its feathers, and its bare skin.
#[derive(Debug, Clone, Copy)]
struct Species {
    back: u32,
    pale: u32,
    dark: u32,
    /// The legs and the hands.
    skin: u32,
    bill: u32,
    /// The bare skin of the face.
    lore: u32,
}

/// The materials of the bare parts of one kind of heron.
#[derive(Debug, Clone)]
pub(super) struct Bare {
    skin: Handle<SurfaceMaterial>,
    bill: Handle<StandardMaterial>,
}

/// One bird's feathers, and its bare parts to go with them.
#[derive(Debug, Clone)]
pub(super) struct Coat {
    pub(super) feathers: Handle<PlumageMaterial>,
    pub(super) skin: Handle<SurfaceMaterial>,
    pub(super) bill: Handle<StandardMaterial>,
    /// How soaked it is, from 0 to 1, as last shown.
    shown_wet: f32,
}

impl Species {
    const fn new([back, pale, dark]: [u32; 3], [skin, bill, lore]: [u32; 3]) -> Self {
        Self {
            back,
            pale,
            dark,
            skin,
            bill,
            lore,
        }
    }

    fn color(rgb: u32) -> Color {
        Color::srgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }
}

impl Bare {
    /// The bare parts of every kind of heron, in the order [`Coat::new`]
    /// expects them. Legs are leathery, bills have the soft gloss of horn.
    pub(super) fn all(
        surfaces: &mut Assets<SurfaceMaterial>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Vec<Self> {
        SPECIES
            .iter()
            .map(|species| Self {
                skin: surfaces.add(Finish::Hide.scaled(Species::color(species.skin))),
                bill: materials.add(StandardMaterial {
                    base_color: Species::color(species.bill),
                    perceptual_roughness: 0.45,
                    reflectance: 0.4,
                    ..default()
                }),
            })
            .collect()
    }
}

impl Coat {
    /// A coat of one of the kinds of heron, a little different from any
    /// other, with its bare parts from `bare`, made by [`Bare::all`].
    pub(super) fn new(
        plumages: &mut Assets<PlumageMaterial>,
        bare: &[Bare],
        rng: &mut impl Rng,
    ) -> Self {
        let index = rng.random_range(0..Self::SPECIES);
        Self::of(index, plumages, bare, rng)
    }

    /// How many kinds of heron there are.
    pub(super) const SPECIES: usize = SPECIES.len();

    /// A coat of the kind of heron at `index`, below [`Coat::SPECIES`].
    pub(super) fn of(
        index: usize,
        plumages: &mut Assets<PlumageMaterial>,
        bare: &[Bare],
        rng: &mut impl Rng,
    ) -> Self {
        let species = SPECIES[index % SPECIES.len()];
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
                    lore: Species::color(species.lore).to_linear().to_vec4(),
                    cloth: Species::color(*CLOTHS.pick(rng)).to_linear().to_vec4(),
                    wet: 0.0,
                },
            },
        });
        Self {
            feathers,
            skin: bare[index % SPECIES.len()].skin.clone(),
            bill: bare[index % SPECIES.len()].bill.clone(),
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
