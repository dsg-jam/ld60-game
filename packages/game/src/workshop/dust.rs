use bevy::{light::NotShadowCaster, prelude::*};
use ld60_core::parts::downwash::Downwash;
use ld60_geometry::block::Block;
use rand::{RngExt, SeedableRng, rngs::SmallRng};

const MOTE_RADIUS: f32 = 0.0009;
/// Meters per second.
const DRIFT: f32 = 0.012;
/// Warm air rises over the counter.
const RISE: f32 = 0.004;
const MOTE: Color = Color::srgb(0.9, 0.88, 0.85);
/// How many seconds a speck takes to go with a change in the air.
const LAG: f32 = 0.3;

pub(super) struct DustPlugin;

impl Plugin for DustPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Mote::drift)
            .add_observer(Dust::on_add);
    }
}

/// Specks of dust floating in the air around an entity. They only show
/// where a light shines on them.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct Dust {
    /// Where the specks float, in the entity's own space.
    pub(super) cloud: Block,
    pub(super) motes: usize,
    pub(super) seed: u64,
}

/// A speck of [`Dust`].
#[derive(Component, Debug)]
struct Mote {
    /// Random phases of its wandering.
    phase: Vec3,
    cloud: Block,
    /// How fast the air moves it, apart from its wandering, in the space
    /// of its cloud.
    blown: Vec3,
}

impl Dust {
    fn on_add(
        add: On<Add<Dust>>,
        clouds: Query<&Dust>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Ok(&dust) = clouds.get(add.entity) else {
            return;
        };
        let mut rng = SmallRng::seed_from_u64(dust.seed);
        let mesh = meshes.add(Sphere::new(MOTE_RADIUS).mesh().uv(6, 4));
        // Adds the light it catches, so it vanishes in the dark.
        let material = materials.add(StandardMaterial {
            base_color: MOTE,
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Add,
            ..default()
        });
        let cloud = dust.cloud;
        commands.entity(add.entity).with_children(|parent| {
            for _ in 0..dust.motes {
                let at =
                    cloud.min() + cloud.size * Vec3::new(rng.random(), rng.random(), rng.random());
                parent.spawn((
                    Mote {
                        phase: Vec3::new(rng.random(), rng.random(), rng.random()) * 100.0,
                        cloud,
                        blown: Vec3::ZERO,
                    },
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(at),
                    NotShadowCaster,
                    Pickable::IGNORE,
                ));
            }
        });
    }
}

impl Mote {
    /// Wanders slowly on the air currents and goes with the [`Downwash`],
    /// and comes back in on the other side when it leaves the cloud.
    fn drift(
        time: Res<Time>,
        downwash: Res<Downwash>,
        clouds: Query<&GlobalTransform>,
        mut motes: Query<(&mut Mote, &mut Transform, &GlobalTransform, &ChildOf)>,
    ) {
        let t = time.elapsed_secs();
        let dt = time.delta_secs();
        let follow = 1.0 - (-dt / LAG).exp();
        for (mut mote, mut transform, at, cloud) in &mut motes {
            let air = clouds.get(cloud.parent()).map_or(Vec3::ZERO, |cloud| {
                cloud
                    .affine()
                    .inverse()
                    .transform_vector3(downwash.at(at.translation()))
            });
            let blown = mote.blown;
            mote.blown += (air - blown) * follow;
            let p = mote.phase;
            let wander = Vec3::new(
                (t * 0.23 + p.x).sin() + 0.5 * (t * 0.61 + p.y).sin(),
                (t * 0.19 + p.y).sin() + 0.5 * (t * 0.53 + p.z).sin(),
                (t * 0.29 + p.z).sin() + 0.5 * (t * 0.47 + p.x).sin(),
            );
            let moved = transform.translation + (wander * DRIFT + Vec3::Y * RISE + mote.blown) * dt;
            let min = mote.cloud.min();
            transform.translation = min + (moved - min).rem_euclid(mote.cloud.size);
        }
    }
}
