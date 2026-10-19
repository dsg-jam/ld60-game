use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::{ShaderRef, load_shader_library},
};

/// How used every surface looks, from 0 to 1.
const WEAR: f32 = 0.8;

pub(crate) struct SurfacePlugin;

impl Plugin for SurfacePlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "floor.wesl");
        load_shader_library!(app, "wear.wesl");
        load_shader_library!(app, "wood.wesl");
        embedded_asset!(app, "surface.wesl");
        app.add_plugins(MaterialPlugin::<SurfaceMaterial>::default());
    }
}

pub(crate) type SurfaceMaterial = ExtendedMaterial<StandardMaterial, Surface>;

/// What a surface is made of, which decides its detail and how it wears.
///
/// Matches the constants in `surface.wesl`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) enum Finish {
    #[default]
    Paint = 0,
    Plastic = 1,
    Metal = 2,
    Wood = 3,
    /// Painted plaster, for walls.
    Plaster = 4,
    Fabric = 5,
    Paper = 6,
    Ceramic = 7,
    /// Boards running along Z, for floors.
    Planks = 8,
    /// Plastic with ribs around its local Y axis, for handles.
    Grip = 9,
    /// Metal sheet with rows of round holes, like a speaker grille.
    Perforated = 10,
    /// Bare concrete, for pillars, curbs and sidewalks.
    Concrete = 11,
    /// Steel left out in the rain, rusted in patches.
    Rust = 12,
    /// A knotted rug lying flat, long along Z.
    Rug = 13,
    /// Plated steel, like a clip. It wears like [`Finish::Metal`] but does
    /// not rust.
    Plated = 14,
}

/// The shape a [`Finish`] covers, which decides where its edges wear.
///
/// Matches the constants in `surface.wesl`.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Form {
    /// A box of this size.
    Box(Vec3),
    /// An upright cylinder filling a box of this size.
    Cylinder(Vec3),
    /// Any other mesh. It has no edges to wear.
    Freeform,
    /// Any number of unit cubes, each scaled to the size of a box. The
    /// material can be shared between them.
    Scaled,
}

/// Detail and signs of use on a surface: grain, bumps, worn edges, grime,
/// dust and scratches.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(crate) struct Surface {
    #[uniform(100)]
    params: SurfaceParams,
}

/// Matches `Surface` in `surface.wesl`.
#[derive(ShaderType, Reflect, Debug, Clone)]
struct SurfaceParams {
    half_size: Vec3,
    seed: f32,
    finish: u32,
    form: u32,
    wear: f32,
}

impl Finish {
    /// A material in `color` with this finish, for a shape of `form`. The
    /// `seed` keeps similar shapes from looking the same.
    pub(crate) fn material(self, color: Color, form: Form, seed: f32) -> SurfaceMaterial {
        self.textured(color, None, form, seed)
    }

    /// Like [`Self::material`], with `texture` printed on.
    pub(crate) fn textured(
        self,
        color: Color,
        texture: Option<Handle<Image>>,
        form: Form,
        seed: f32,
    ) -> SurfaceMaterial {
        let base = StandardMaterial {
            base_color: color,
            base_color_texture: texture,
            perceptual_roughness: self.roughness(),
            metallic: self.metallic(),
            ..default()
        };
        self.over(base, form, seed)
    }

    /// A material in `color` with this finish for any number of unit cubes,
    /// each scaled to the size of a box.
    pub(crate) fn scaled(self, color: Color) -> SurfaceMaterial {
        self.material(color, Form::Scaled, 0.0)
    }

    /// Like [`Self::material`], over a `base` that sets more than the color,
    /// like the gloss of car paint.
    pub(crate) fn over(self, base: StandardMaterial, form: Form, seed: f32) -> SurfaceMaterial {
        let half_size = match form {
            Form::Box(size) | Form::Cylinder(size) => size / 2.0,
            Form::Freeform | Form::Scaled => Vec3::ZERO,
        };
        ExtendedMaterial {
            base,
            extension: Surface {
                params: SurfaceParams {
                    half_size,
                    seed,
                    finish: self as u32,
                    form: form.code(),
                    wear: WEAR,
                },
            },
        }
    }

    fn roughness(self) -> f32 {
        match self {
            Self::Ceramic => 0.25,
            Self::Metal | Self::Plated | Self::Perforated => 0.35,
            Self::Plastic => 0.5,
            Self::Paint | Self::Grip => 0.6,
            Self::Wood | Self::Planks => 0.65,
            Self::Paper => 0.9,
            Self::Plaster | Self::Concrete | Self::Rust => 0.95,
            Self::Fabric | Self::Rug => 1.0,
        }
    }

    fn metallic(self) -> f32 {
        match self {
            Self::Metal | Self::Plated | Self::Perforated | Self::Rust => 1.0,
            _ => 0.0,
        }
    }
}

impl Form {
    /// Matches the constants in `surface.wesl`.
    fn code(self) -> u32 {
        match self {
            Self::Box(_) => 0,
            Self::Cylinder(_) => 1,
            Self::Freeform => 2,
            Self::Scaled => 3,
        }
    }
}

impl Surface {
    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("surface.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl MaterialExtension for Surface {
    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        Self::shader()
    }
}
