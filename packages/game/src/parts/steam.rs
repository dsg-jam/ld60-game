use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    light::{FogVolume, NotShadowCaster},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use rand::{RngExt, SeedableRng, rngs::SmallRng};

/// Voxels of the density texture across, up and deep.
const GRID: UVec3 = UVec3::new(24, 48, 24);
/// Cells of the repeating noise the wisps come from, along each side.
const NOISE: usize = 16;
const SEED: u64 = 31;
/// The plume widens by this much per meter it rises. Turbulent plumes take
/// in the air around them at a steady rate, after Morton, Taylor and
/// Turner, which makes them cones.
const SPREAD: f32 = 0.12;
/// Meters per second.
const RISE: f32 = 0.06;
/// The steam mixes into the cool air of the shop and vanishes over about
/// this height.
const FADE: f32 = 0.3;
/// How far the plume sways sideways per meter it rises.
const SWAY: f32 = 0.12;
/// Size of the wisps, in meters.
const WISP: f32 = 0.025;
/// How much the wisps twist and curl, in meters.
const CURL: f32 = 0.015;
const DENSITY: f32 = 0.04;
/// Water droplets barely absorb light and scatter most of it forward.
const ABSORPTION: f32 = 0.05;
const SCATTERING: f32 = 0.9;
const FORWARD: f32 = 0.6;
/// Bevy's fog catches little light at this exposure. Makes up for it.
const GLOW: f32 = 60.0;

pub(crate) struct SteamPlugin;

impl Plugin for SteamPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Wisps>()
            .add_systems(Update, Steam::rise)
            .add_observer(Steam::on_add);
    }
}

/// Steam rising from a hot surface, lit by the lights around it.
///
/// The surface is a disc of `radius` around the origin, and the steam rises
/// along +Y to `height`. It can also stand in for smoke.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Steam {
    pub(crate) radius: f32,
    pub(crate) height: f32,
}

/// The density texture of a [`Steam`] plume.
#[derive(Component, Debug)]
struct Plume(Handle<Image>);

/// Repeating noise the wisps of all plumes come from.
#[derive(Resource, Debug)]
struct Wisps(Vec<f32>);

impl FromWorld for Wisps {
    fn from_world(_: &mut World) -> Self {
        let mut rng = SmallRng::seed_from_u64(SEED);
        Self(
            (0..NOISE * NOISE * NOISE)
                .map(|_| rng.random::<f32>())
                .collect(),
        )
    }
}

impl Wisps {
    /// Smooth noise from 0 to 1 at `p`, in cells. It repeats every
    /// [`NOISE`] cells.
    fn at(&self, p: Vec3) -> f32 {
        let cell = p.floor();
        let f = p - cell;
        let f = f * f * (3.0 - 2.0 * f);
        let corner = |dx: usize, dy: usize, dz: usize| {
            let wrap = |v: f32, d: usize| (v as i64 + d as i64).rem_euclid(NOISE as i64) as usize;
            let (x, y, z) = (wrap(cell.x, dx), wrap(cell.y, dy), wrap(cell.z, dz));
            self.0[x + NOISE * (y + NOISE * z)]
        };
        let x00 = corner(0, 0, 0).lerp(corner(1, 0, 0), f.x);
        let x10 = corner(0, 1, 0).lerp(corner(1, 1, 0), f.x);
        let x01 = corner(0, 0, 1).lerp(corner(1, 0, 1), f.x);
        let x11 = corner(0, 1, 1).lerp(corner(1, 1, 1), f.x);
        x00.lerp(x10, f.y).lerp(x01.lerp(x11, f.y), f.z)
    }
}

impl Steam {
    /// Width of the box the plume fits in.
    fn width(&self) -> f32 {
        2.0 * (self.radius + (SPREAD + SWAY) * self.height)
    }

    fn on_add(
        add: On<Add<Steam>>,
        plumes: Query<&Steam>,
        mut images: ResMut<Assets<Image>>,
        mut commands: Commands,
    ) {
        let Ok(steam) = plumes.get(add.entity) else {
            return;
        };
        let mut image = Image::new(
            Extent3d {
                width: GRID.x,
                height: GRID.y,
                depth_or_array_layers: GRID.z,
            },
            TextureDimension::D3,
            vec![0; GRID.element_product() as usize],
            TextureFormat::R8Unorm,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::linear();
        let image = images.add(image);
        let size = Vec3::new(steam.width(), steam.height, steam.width());
        commands.entity(add.entity).with_child((
            Plume(image.clone()),
            FogVolume {
                density_texture: Some(image),
                density_factor: DENSITY,
                absorption: ABSORPTION,
                scattering: SCATTERING,
                scattering_asymmetry: FORWARD,
                light_intensity: GLOW,
                ..default()
            },
            NotShadowCaster,
            Transform::from_translation(Vec3::Y * steam.height / 2.0).with_scale(size),
        ));
    }

    /// Redraws every plume as its wisps rise and it sways in the air.
    fn rise(
        time: Res<Time>,
        wisps: Res<Wisps>,
        plumes: Query<(&Plume, &ChildOf)>,
        sources: Query<&Steam>,
        mut images: ResMut<Assets<Image>>,
    ) {
        let t = time.elapsed_secs();
        for (plume, child_of) in &plumes {
            let Ok(steam) = sources.get(child_of.parent()) else {
                continue;
            };
            let Some(mut image) = images.get_mut(&plume.0) else {
                continue;
            };
            let Some(data) = image.data.as_mut() else {
                continue;
            };
            let width = steam.width();
            let voxel = Vec3::new(width, steam.height, width) / GRID.as_vec3();
            for z in 0..GRID.z {
                for y in 0..GRID.y {
                    let height = (y as f32 + 0.5) * voxel.y;
                    let axis = Vec2::new(
                        (t * 0.5 + height * 6.0).sin() + 0.5 * (t * 1.3 + height * 11.0).sin(),
                        (t * 0.4 + height * 7.0 + 2.0).sin() + 0.5 * (t * 1.1 + height * 9.0).sin(),
                    ) * SWAY
                        * height
                        / 1.5;
                    let reach = steam.radius + SPREAD * height;
                    // Thinner as it spreads and mixes, only forms just above
                    // the surface, and gone by the top of the box.
                    let top = 1.0 - height / steam.height;
                    let thickness = steam.radius / reach
                        * (-height / FADE).exp()
                        * (height / (2.0 * voxel.y)).min(1.0)
                        * (top * 4.0).min(1.0);
                    for x in 0..GRID.x {
                        let p = Vec3::new(
                            (x as f32 + 0.5) * voxel.x - width / 2.0,
                            height,
                            (z as f32 + 0.5) * voxel.z - width / 2.0,
                        );
                        let off_axis = (p.xz() - axis).length() / reach;
                        let profile = (-2.0 * off_axis * off_axis).exp();
                        let moving = (p - Vec3::Y * RISE * t) / WISP;
                        let curl = Vec3::new(
                            wisps.at(moving * 0.5 + 7.0),
                            0.0,
                            wisps.at(moving * 0.5 + 13.0),
                        ) - 0.5;
                        let n = wisps.at(moving + curl * (2.0 * CURL / WISP));
                        let wispy = ((n - 0.45) / 0.2).clamp(0.0, 1.0);
                        let density = thickness * profile * wispy;
                        let index = (x + GRID.x * (y + GRID.y * z)) as usize;
                        data[index] = (density.clamp(0.0, 1.0) * 255.0) as u8;
                    }
                }
            }
        }
    }
}
