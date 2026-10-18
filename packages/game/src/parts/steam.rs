mod render;

use bevy::{
    asset::RenderAssetUsages,
    camera::primitives::{Aabb, Frustum},
    image::ImageSampler,
    light::{FogVolume, NotShadowCaster},
    prelude::*,
    render::{
        RenderApp,
        extract_component::ExtractComponent,
        render_resource::{Extent3d, ShaderType, TextureDimension, TextureFormat, TextureUsages},
    },
};

/// Voxels of the density texture across, up and deep.
const GRID: UVec3 = UVec3::new(32, 64, 32);
/// The plume widens by this much per meter it rises. Matches
/// `steam/plume.wesl`.
const SPREAD: f32 = 0.12;
/// How far the plume sways sideways per meter it rises. Matches
/// `steam/plume.wesl`.
const SWAY: f32 = 0.12;
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
        app.add_plugins(render::PlumeRenderPlugin)
            .add_systems(PostUpdate, Plume::follow)
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

/// The fog volume of a [`Steam`] plume. The GPU redraws its density
/// texture every frame while a camera sees it.
#[derive(Component, ExtractComponent, Clone, Debug)]
#[extract_app(RenderApp)]
struct Plume {
    density: Handle<Image>,
    params: PlumeParams,
    seen: bool,
}

/// Matches `Plume` in `steam/plume.wesl`.
#[derive(ShaderType, Clone, Copy, Debug)]
struct PlumeParams {
    radius: f32,
    height: f32,
    width: f32,
    time: f32,
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
        let mut image = Image::new_fill(
            Extent3d {
                width: GRID.x,
                height: GRID.y,
                depth_or_array_layers: GRID.z,
            },
            TextureDimension::D3,
            &[0; 4],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage |= TextureUsages::STORAGE_BINDING;
        image.sampler = ImageSampler::linear();
        let density = images.add(image);
        let size = Vec3::new(steam.width(), steam.height, steam.width());
        commands.entity(add.entity).with_child((
            Plume {
                density: density.clone(),
                params: PlumeParams {
                    radius: steam.radius,
                    height: steam.height,
                    width: steam.width(),
                    time: 0.0,
                },
                seen: false,
            },
            FogVolume {
                density_texture: Some(density),
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
}

impl Plume {
    /// Moves time on for the GPU, and notes whether any camera sees the
    /// plume, so it only gets redrawn then.
    fn follow(
        time: Res<Time>,
        cameras: Query<&Frustum, With<Camera3d>>,
        mut plumes: Query<(&mut Plume, &GlobalTransform)>,
    ) {
        // The fog volume is a unit cube, scaled to size.
        let unit = Aabb::from_min_max(Vec3::splat(-0.5), Vec3::splat(0.5));
        for (mut plume, transform) in &mut plumes {
            let affine = transform.affine();
            plume.seen = cameras
                .iter()
                .any(|frustum| frustum.intersects_obb(&unit, &affine, true, true));
            plume.params.time = time.elapsed_secs();
        }
    }
}
