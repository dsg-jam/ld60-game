use std::f32::consts::TAU;

use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use ld60_game_core::GRAVITY;
use rand::{RngExt, SeedableRng, rngs::SmallRng};

use super::downwash::Downwash;

/// Surface tension over density of water, and so of coffee, in m³/s².
const TENSION: f32 = 7.2e-5;
/// Share of critical damping. A shaken mug rings for a few seconds.
const DAMPING: f32 = 0.03;
/// Foam along the wall damps the waves more, up to this many times, after
/// Sauret et al., "Damping of liquid sloshing by foams".
const FOAM_DAMPING: f32 = 2.0;
/// The liquid turns slowly in the container, in radians per second.
const SWIRL: f32 = 0.03;
/// Sloshing modes of a liquid in an upright round container, after
/// Abramson, "The Dynamic Behavior of Liquids in Moving Containers": how
/// many times the surface rises and falls around the container, and the
/// root of the Bessel function's slope that lets the surface meet the wall
/// square.
const MODES: [(u32, f32); WAVES] = [
    (1, 1.8412),
    (2, 3.0542),
    (0, 3.8317),
    (3, 4.2012),
    (1, 5.3314),
    (0, 7.0156),
    (1, 8.5363),
    (0, 10.1735),
    (1, 11.7060),
    (0, 13.3237),
];
const WAVES: usize = 10;
/// How hard a unit of [`Shake`] pushes the waves.
const RESPONSE: f32 = 0.3;
/// A sideways shake rocks the liquid as a whole, the first mode. A real
/// container is never quite round or level, so it also stirs the others,
/// this much less.
const IMPERFECTION: f32 = 0.5;
const SEED: u64 = 21;
/// How far air across the surface pushes the foam, as meters per meter per
/// second of air, at most this share of the radius away, and how many
/// seconds it takes to get there.
const FOAM_DRIFT: f32 = 0.06;
const FOAM_REACH: f32 = 0.35;
const FOAM_LAG: f32 = 0.5;
/// How much air stirs the surface, like the ground shaking this much per
/// meter per second.
const AIR_SHAKE: f32 = 0.6;
/// Simulation steps stay this short, so the fastest mode stays stable.
const MAX_STEP: f32 = 0.004;
const ROUGHNESS: f32 = 0.08;
/// Like water.
const REFLECTANCE: f32 = 0.35;

pub struct LiquidPlugin;

impl Plugin for LiquidPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "liquid.wesl");
        app.add_plugins(MaterialPlugin::<LiquidMaterial>::default())
            .init_resource::<Shake>()
            .add_systems(First, Shake::settle)
            .add_systems(PostUpdate, Liquid::slosh)
            .add_observer(Liquid::on_add);
    }
}

/// How hard the ground shakes right now, in m/s². Anything that rumbles
/// adds to it every frame, and [`Liquid`]s slosh with it.
#[derive(Resource, Default, Debug)]
pub struct Shake(pub f32);

/// The flat surface of a liquid in a round container, facing +Z. It waves
/// when the ground [`Shake`]s.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct Liquid {
    radius: f32,
    depth: f32,
    color: Color,
    foam: Foam,
    turn: f32,
    /// How far air has pushed the foam across the surface.
    blown: Vec2,
    waves: [Wave; WAVES],
}

/// Bubbles floating on a [`Liquid`].
#[derive(Clone, Copy, Debug)]
pub struct Foam {
    pub color: Color,
    /// How much of the surface it covers, from 0 to 1.
    pub cover: f32,
}

/// One sloshing mode of a [`Liquid`], as two waves turned a quarter turn
/// against each other around the container.
#[derive(Clone, Copy, Debug, Default)]
struct Wave {
    height: Vec2,
    speed: Vec2,
}

pub type LiquidMaterial = ExtendedMaterial<StandardMaterial, LiquidSurface>;

/// The waves on a [`Liquid`].
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct LiquidSurface {
    #[uniform(100)]
    params: LiquidParams,
}

/// Matches `Liquid` in `liquid.wesl`.
#[derive(ShaderType, Reflect, Debug, Clone)]
struct LiquidParams {
    /// Its alpha is how much of the surface the foam covers.
    foam: Vec4,
    radius: f32,
    /// How far the liquid has turned in the container, in radians.
    turn: f32,
    /// How far air has pushed the foam across the surface.
    blown: Vec2,
    /// Per mode: the heights of its two waves, its Bessel root and its
    /// number of waves around.
    waves: [Vec4; WAVES],
}

impl Shake {
    fn settle(mut shake: ResMut<Self>) {
        shake.0 = 0.0;
    }
}

impl Liquid {
    /// A liquid in a container of `radius`, filled `depth` deep.
    pub fn new(radius: f32, depth: f32, color: Color) -> Self {
        Self {
            radius,
            depth,
            color,
            foam: Foam { color, cover: 0.0 },
            turn: 0.0,
            blown: Vec2::ZERO,
            waves: [Wave::default(); WAVES],
        }
    }

    pub fn with_foam(self, foam: Foam) -> Self {
        Self { foam, ..self }
    }

    fn on_add(
        add: On<Add<Liquid>>,
        liquids: Query<&Liquid>,
        mut materials: ResMut<Assets<LiquidMaterial>>,
        mut commands: Commands,
    ) {
        let Ok(liquid) = liquids.get(add.entity) else {
            return;
        };
        let material = materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: liquid.color,
                perceptual_roughness: ROUGHNESS,
                reflectance: REFLECTANCE,
                ..default()
            },
            extension: LiquidSurface {
                params: liquid.params(),
            },
        });
        commands.entity(add.entity).insert(MeshMaterial3d(material));
    }

    /// How fast mode `index` swings, in radians per second.
    fn frequency(&self, index: usize) -> f32 {
        let k = MODES[index].1 / self.radius;
        ((GRAVITY * k + TENSION * k * k * k) * (k * self.depth).tanh()).sqrt()
    }

    fn params(&self) -> LiquidParams {
        LiquidParams {
            foam: self
                .foam
                .color
                .to_linear()
                .to_vec3()
                .extend(self.foam.cover),
            radius: self.radius,
            turn: self.turn,
            blown: self.blown,
            waves: std::array::from_fn(|index| {
                let (around, root) = MODES[index];
                let height = self.waves[index].height;
                Vec4::new(height.x, height.y, root, around as f32)
            }),
        }
    }

    /// Moves the waves on, pushed about at random by the shaking ground and
    /// the [`Downwash`]. The air also pushes the foam aside.
    fn slosh(
        time: Res<Time>,
        shake: Res<Shake>,
        downwash: Res<Downwash>,
        mut liquids: Query<(
            &mut Liquid,
            &MeshMaterial3d<LiquidMaterial>,
            &GlobalTransform,
        )>,
        mut materials: ResMut<Assets<LiquidMaterial>>,
        mut noise: Local<Option<SmallRng>>,
    ) {
        let noise = noise.get_or_insert_with(|| SmallRng::seed_from_u64(SEED));
        let dt = time.delta_secs();
        if dt <= 0.0 {
            return;
        }
        let steps = (dt / MAX_STEP).ceil().max(1.0);
        let step = dt / steps;
        // White noise with unit variance per second, whatever the step.
        let spread = 3.0_f32.sqrt() / step.sqrt();
        for (mut liquid, material, at) in &mut liquids {
            liquid.turn = (liquid.turn + SWIRL * dt) % TAU;
            // The surface faces +Z, so air across it is along X and Y.
            let (_, rotation, middle) = at.to_scale_rotation_translation();
            let air = rotation.inverse() * downwash.at(middle);
            let reach = FOAM_REACH * liquid.radius;
            let pushed = (air.truncate() * FOAM_DRIFT).clamp_length_max(reach);
            let blown = liquid.blown;
            liquid.blown += (pushed - blown) * (1.0 - (-dt / FOAM_LAG).exp());
            let shaking = shake.0 + AIR_SHAKE * air.length();
            let damping = DAMPING * (1.0 + FOAM_DAMPING * liquid.foam.cover);
            for (index, &(around, _)) in MODES.iter().enumerate() {
                let omega = liquid.frequency(index);
                let gain = if around == 1 { 1.0 } else { IMPERFECTION };
                let strength = shaking * RESPONSE * gain * spread;
                let wave = &mut liquid.waves[index];
                for _ in 0..steps as usize {
                    let push =
                        Vec2::new(noise.random_range(-1.0..1.0), noise.random_range(-1.0..1.0))
                            * strength;
                    let accel =
                        push - 2.0 * damping * omega * wave.speed - omega * omega * wave.height;
                    wave.speed += accel * step;
                    wave.height += wave.speed * step;
                }
            }
            if let Some(mut material) = materials.get_mut(material) {
                material.extension.params = liquid.params();
            }
        }
    }
}

impl LiquidSurface {
    fn shader() -> ShaderRef {
        AssetPath::from_path_buf(embedded_path!("liquid.wesl"))
            .with_source("embedded")
            .into()
    }
}

impl MaterialExtension for LiquidSurface {
    fn fragment_shader() -> ShaderRef {
        Self::shader()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        Self::shader()
    }
}
