//! The people of the city: walkers on the sidewalks, customers on the
//! stools of the stalls and keepers behind their counters, all of them
//! upright herons of sorts.

mod bird;
mod gaze;
mod keeper;
mod limb;
mod plumage;
mod shadows;
mod sound;
mod umbrella;
mod walk;

use std::ops::Range;

use bevy::{mesh::skinning::SkinnedMeshInverseBindposes, prelude::*};
use ld60_geometry::surface::{Finish, Form, SurfaceMaterial};
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

use self::bird::{Bird, Birth, Posture};
use self::gaze::Gaze;
use self::keeper::Keeper;
use self::plumage::{Coat, PlumageMaterial};
use self::walk::Walker;
use super::buildings::Skyline;
use super::{Pick, rng};

const SEED: u64 = 9;
/// Share of the stools near the shop that someone sits on.
const SEATED_CHANCE: f64 = 0.55;
/// Share of the open stalls near the shop with their keeper in.
const KEEPER_CHANCE: f64 = 0.85;
/// Seconds between the phrases of people walking by, sitting and keeping
/// stalls. People sitting together talk the most.
const WALKER_TALK: Range<f32> = 10.0..40.0;
const SITTER_TALK: Range<f32> = 3.0..10.0;
const KEEPER_TALK: Range<f32> = 6.0..20.0;

/// Colors of the scarves and shawls, muted so the plumage stays the
/// main thing.
const CLOTHS: [Color; 6] = [
    Color::srgb_u8(0x7a, 0x2e, 0x2a),
    Color::srgb_u8(0x9a, 0x7a, 0x34),
    Color::srgb_u8(0x2a, 0x56, 0x58),
    Color::srgb_u8(0x2c, 0x32, 0x50),
    Color::srgb_u8(0x56, 0x5e, 0x3a),
    Color::srgb_u8(0x5a, 0x32, 0x4a),
];

pub(super) struct LocalsPlugin;

impl Plugin for LocalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            sound::LocalSoundPlugin,
            shadows::ShadowPlugin,
            plumage::PlumagePlugin,
            umbrella::DropsPlugin,
        ))
        .init_resource::<Places>()
        .insert_resource(Whims(SmallRng::seed_from_u64(SEED)))
        .add_systems(
            Startup,
            (Figures::load, Figures::spawn_all)
                .chain()
                .after(Skyline::spawn),
        )
        .add_systems(
            Update,
            (Walker::walk, Keeper::work_on, Gaze::glance, Bird::pose).chain(),
        )
        .add_systems(Update, (Bird::open_umbrellas, Bird::live));
    }
}

/// Spots near the shop where people stay, found while building the market.
#[derive(Resource, Debug, Default)]
pub(super) struct Places {
    /// Stools, each at its foot, facing the stall.
    pub(super) stools: Vec<Stool>,
    /// The floors behind counters, facing the street, with the half width
    /// of the counter.
    pub(super) counters: Vec<(Transform, f32)>,
    /// Spots on the sidewalks where something stands in the way, like a
    /// stool, seen from above.
    pub(super) in_way: Vec<Vec2>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Stool {
    pub(super) at: Transform,
    pub(super) height: f32,
    /// Whether an awning keeps the rain off.
    pub(super) covered: bool,
}

/// Randomness for what people do from moment to moment.
#[derive(Resource, Debug)]
struct Whims(SmallRng);

/// Shared meshes and materials for all people.
#[derive(Resource, Debug)]
struct Figures {
    /// One for each kind of heron, see [`Coat::skin_colors`].
    skins: Vec<Handle<SurfaceMaterial>>,
    cloths: Vec<Handle<SurfaceMaterial>>,
    umbrellas: umbrella::Kit,
    birds: bird::Kit,
}

impl FromWorld for Figures {
    fn from_world(world: &mut World) -> Self {
        let (umbrellas, birds) = world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
            world.resource_scope(|world, mut materials: Mut<Assets<StandardMaterial>>| {
                let mut bindposes = world.resource_mut::<Assets<SkinnedMeshInverseBindposes>>();
                (
                    umbrella::Kit::new(&mut meshes, &mut materials),
                    bird::Kit::new(
                        &mut meshes,
                        &mut materials,
                        &mut bindposes,
                        &mut rng(SEED + 2),
                    ),
                )
            })
        });
        let mut surfaces = world.resource_mut::<Assets<SurfaceMaterial>>();
        let skins = Coat::skin_colors()
            .map(|color| surfaces.add(Finish::Hide.scaled(color)))
            .collect();
        let cloths = CLOTHS
            .iter()
            .map(|&color| {
                let base = StandardMaterial {
                    base_color: color,
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                };
                surfaces.add(Finish::Fabric.over(base, Form::Scaled, 0.0))
            })
            .collect();
        Self {
            skins,
            cloths,
            umbrellas,
            birds,
        }
    }
}

impl Figures {
    /// Made at startup, once the materials can be.
    fn load(world: &mut World) {
        world.init_resource::<Self>();
    }

    fn spawn_all(
        mut commands: Commands,
        figures: Res<Figures>,
        places: Res<Places>,
        mut plumages: ResMut<Assets<PlumageMaterial>>,
    ) {
        let mut rng = rng(SEED);
        let mut paths = super::rng(SEED + 1);
        for walker in Walker::crowd(&mut paths) {
            let birth = Birth {
                at: walker.place(),
                posture: Posture::Walking,
                may_carry: true,
                sheltered: false,
                coat: Coat::new(&mut plumages, &figures.skins, &mut rng),
            };
            let bird = Bird::spawn(&mut commands, &figures, &mut rng, birth);
            commands.entity(bird).insert(walker);
        }
        for stool in &places.stools {
            if !rng.random_bool(SEATED_CHANCE) {
                continue;
            }
            let turn = Quat::from_rotation_y(rng.random_range(-0.7..0.7));
            let at = stool.at.with_rotation(stool.at.rotation * turn);
            let birth = Birth {
                at,
                posture: Posture::Seated(stool.height),
                may_carry: !stool.covered,
                sheltered: stool.covered,
                coat: Coat::new(&mut plumages, &figures.skins, &mut rng),
            };
            Bird::spawn(&mut commands, &figures, &mut rng, birth);
        }
        for &(at, half_width) in &places.counters {
            if rng.random_bool(KEEPER_CHANCE) {
                Keeper::spawn(
                    &mut commands,
                    &figures,
                    &mut plumages,
                    &mut rng,
                    at,
                    half_width,
                );
            }
        }
    }
}
