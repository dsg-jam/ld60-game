use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::gpu_readback::{ReadbackComplete, ReadbackOnce},
};

use super::{BLACK_LEVEL, BRIGHTNESS, CrtMaterial, SCANLINE_DARKNESS};

/// Seconds between two looks at what the screen shows.
const INTERVAL: f32 = 0.2;
/// Only every this many pixels across and down count towards the average.
const STRIDE: usize = 4;
/// The picture darkens towards its corners, on average by this much.
const VIGNETTE: f32 = 0.85;
/// Lumens per unit of brightness on the glass. Much more than a real screen,
/// which only lights up a room for eyes used to the dark.
const POWER: f32 = 3000.0;
const RANGE: f32 = 2.0;

pub(super) struct CrtLightPlugin;

impl Plugin for CrtLightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, CrtLight::sample)
            .add_observer(CrtLight::on_add);
    }
}

/// The light a CRT casts in front of it, in the color and brightness of
/// what it shows. Goes on the glass, next to its [`CrtMaterial`], with the
/// picture facing +Z.
#[derive(Component, Debug)]
pub(crate) struct CrtLight {
    /// Width and height of the picture.
    pub(crate) size: Vec2,
}

/// When a [`CrtLight`] next looks at the screen, and its light.
#[derive(Component, Debug)]
struct Sampling {
    timer: Timer,
    light: Entity,
    /// The last readback, until it completes.
    pending: Option<Entity>,
}

impl CrtLight {
    fn on_add(add: On<Add<CrtLight>>, glasses: Query<&CrtLight>, mut commands: Commands) {
        let Ok(glass) = glasses.get(add.entity) else {
            return;
        };
        let light = commands
            .spawn((
                RectLight {
                    color: Color::BLACK,
                    intensity: 0.0,
                    range: RANGE,
                    width: glass.size.x,
                    height: glass.size.y,
                },
                // Lights shine along -Z.
                Transform::from_rotation(Quat::from_rotation_y(PI)),
                ChildOf(add.entity),
            ))
            .id();
        commands.entity(add.entity).insert(Sampling {
            timer: Timer::from_seconds(INTERVAL, TimerMode::Repeating),
            light,
            pending: None,
        });
    }

    /// Reads the picture back from the GPU now and then and lights the
    /// surroundings with it.
    fn sample(
        time: Res<Time>,
        mut glasses: Query<(&mut Sampling, &MeshMaterial3d<CrtMaterial>)>,
        materials: Res<Assets<CrtMaterial>>,
        images: Res<Assets<Image>>,
        mut commands: Commands,
    ) {
        for (mut sampling, material) in &mut glasses {
            if !sampling.timer.tick(time.delta()).just_finished() {
                continue;
            }
            let Some(material) = materials.get(material) else {
                continue;
            };
            let Some(size) = images.get(&material.screen).map(Image::size) else {
                continue;
            };
            // A readback that never completed, for example because the
            // canvas was not on the GPU yet.
            if let Some(stale) = sampling.pending.take() {
                commands.entity(stale).try_despawn();
            }
            let light = sampling.light;
            let readback = commands
                .spawn(ReadbackOnce::texture(material.screen.clone()))
                .observe(
                    move |done: On<ReadbackComplete>,
                          mut lights: Query<&mut RectLight>,
                          mut commands: Commands| {
                        if let Ok(mut rect) = lights.get_mut(light) {
                            Self::shine(&mut rect, Self::average(&done.data, size));
                        }
                        // Bevy only takes the readback off the entity.
                        commands.entity(done.entity).despawn();
                    },
                )
                .id();
            sampling.pending = Some(readback);
        }
    }

    /// Average linear color of the RGBA pixels in `data`, an image of
    /// `size` whose rows may be padded.
    fn average(data: &[u8], size: UVec2) -> Vec3 {
        let row = data.len() / (size.y as usize).max(1);
        let width = (size.x as usize * 4).min(row);
        let mut sum = Vec3::ZERO;
        let mut count = 0;
        for line in data.chunks_exact(row.max(4)).step_by(STRIDE) {
            for pixel in line[..width].as_chunks::<4>().0.iter().step_by(STRIDE) {
                let color = Color::srgb_u8(pixel[0], pixel[1], pixel[2]).to_linear();
                sum += color.to_vec3();
                count += 1;
            }
        }
        sum / count.max(1) as f32
    }

    /// Shines like the glass does when the picture averages to `color`.
    fn shine(light: &mut RectLight, color: Vec3) {
        let glow = (BLACK_LEVEL + color * BRIGHTNESS) * (1.0 - SCANLINE_DARKNESS / 2.0) * VIGNETTE;
        let peak = glow.max_element().max(f32::EPSILON);
        light.color = Color::linear_rgb(glow.x / peak, glow.y / peak, glow.z / peak);
        light.intensity = POWER * peak;
    }
}
