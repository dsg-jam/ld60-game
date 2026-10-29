use std::ops::Range;

use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use ld60_geometry::block::Block;
use rand::{Rng, RngExt};

use super::super::weather::Weather;
use super::cladding::Cladding;

const WINDOW_PITCH: Range<f32> = 1.8..3.6;
const LIT: Range<f32> = 0.2..0.5;
const WINDOW_GLOW: f32 = 6.0;
const AGE: Range<f32> = 0.4..1.0;

/// Matches `COVERS` in `facade.wesl`.
const COVERS: usize = 4;

pub(super) struct FacadePlugin;

impl Plugin for FacadePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "facade.wesl");
        app.add_plugins(MaterialPlugin::<FacadeMaterial>::default())
            .add_systems(
                Update,
                Facade::follow_weather.run_if(resource_changed::<Weather>),
            );
    }
}

pub(crate) type FacadeMaterial = ExtendedMaterial<StandardMaterial, Facade>;

/// Windows drawn onto the walls of a building.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(crate) struct Facade {
    #[uniform(100)]
    params: FacadeParams,
}

/// Matches `Facade` in `facade.wesl`.
#[derive(ShaderType, Reflect, Debug, Clone)]
struct FacadeParams {
    /// Corner of the building with the smallest coordinates.
    origin: Vec3,
    seed: f32,
    size: Vec3,
    /// Horizontal window pitch and floor height. Each wall fits a whole
    /// number of windows, so the pitch varies slightly from wall to wall.
    cell: Vec2,
    /// Window size as a share of the cell.
    window: Vec2,
    /// Share of lit windows.
    lit: f32,
    glow: f32,
    /// Direction along Z that the street side looks, 0 for none.
    facing: f32,
    /// Number of floors with windows, from the bottom.
    floors: f32,
    archetype: u32,
    cladding: u32,
    /// How wet the walls are, from 0 to 1.
    wetness: f32,
    /// How weathered the walls are, from 0 to 1.
    age: f32,
    /// Areas of the street side without windows, as min and max corners.
    covers: [Vec4; COVERS],
}

/// Where the windows of a facade are.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WindowGrid {
    /// Horizontal window pitch and floor height.
    pub(crate) cell: Vec2,
    /// Window size as a share of the cell, centered in it.
    pub(crate) window: Vec2,
}

/// What a building is used for, which decides how its windows light up.
///
/// Matches the constants in `facade.wesl`.
#[derive(Clone, Copy, Debug)]
enum Archetype {
    Office = 0,
    Home = 1,
    /// Mostly empty.
    Dead = 2,
}

impl Facade {
    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("facade.wesl"))
            .with_source("embedded")
            .into()
    }

    pub(crate) fn grid(&self) -> WindowGrid {
        WindowGrid {
            cell: self.params.cell,
            window: self.params.window,
        }
    }

    fn follow_weather(weather: Res<Weather>, mut facades: ResMut<Assets<FacadeMaterial>>) {
        for (_, facade) in facades.iter_mut() {
            facade.extension.params.wetness = weather.wetness;
        }
    }

    /// Hides the windows behind `area` of the shop side, in world X and Y.
    pub(crate) fn cover(&mut self, area: Rect) {
        let origin = self.params.origin.xy();
        let free = self
            .params
            .covers
            .iter_mut()
            .find(|cover| cover.z <= cover.x);
        if let Some(cover) = free {
            *cover = (area.min - origin)
                .extend(area.max.x - origin.x)
                .extend(area.max.y - origin.y);
        }
    }

    /// Windows for `block` with floors of height `floor`. The block faces
    /// the street along `facing`.
    pub(crate) fn random(
        block: &Block,
        facing: f32,
        floor: f32,
        rng: &mut impl Rng,
    ) -> FacadeMaterial {
        let cladding = Cladding::random(rng);
        let wall = cladding.color(rng);
        let columns = (block.size.x / rng.random_range(WINDOW_PITCH))
            .round()
            .max(1.0);
        ExtendedMaterial {
            base: StandardMaterial {
                base_color: wall,
                perceptual_roughness: 0.85,
                ..default()
            },
            extension: Self {
                params: FacadeParams {
                    origin: block.min(),
                    seed: rng.random_range(0.0..1000.0),
                    size: block.size,
                    cell: Vec2::new(block.size.x / columns, floor),
                    window: Vec2::new(rng.random_range(0.3..0.6), rng.random_range(0.4..0.6)),
                    lit: rng.random_range(LIT),
                    glow: WINDOW_GLOW,
                    facing,
                    floors: (block.size.y / floor).floor(),
                    archetype: Archetype::random(rng) as u32,
                    cladding: cladding as u32,
                    wetness: 0.0,
                    age: rng.random_range(AGE),
                    covers: [Vec4::ZERO; COVERS],
                },
            },
        }
    }
}

impl Archetype {
    fn random(rng: &mut impl Rng) -> Self {
        match rng.random_range(0.0..1.0) {
            x if x < 0.35 => Self::Office,
            x if x < 0.9 => Self::Home,
            _ => Self::Dead,
        }
    }
}

impl MaterialExtension for Facade {
    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        Self::shader()
    }
}
