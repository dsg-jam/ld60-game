//! The people of the city: walkers on the sidewalks, customers on the
//! stools of the stalls and keepers behind their counters, all of them
//! upright herons of sorts.

mod bird;
mod gaze;
mod keeper;
mod limb;
#[cfg(feature = "debug_port")]
mod lineup;
mod plumage;
mod shadows;
mod sound;
mod umbrella;
mod walk;

use std::ops::Range;

use bevy::{mesh::skinning::SkinnedMeshInverseBindposes, prelude::*};
use ld60_geometry::surface::SurfaceMaterial;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{RngExt, SeedableRng};

use self::bird::{Bird, Birth, Carry, Dress, Posture};
use self::gaze::Gaze;
use self::keeper::Keeper;
use self::plumage::{Bare, Coat, PlumageMaterial};
use self::walk::{Seats, Walker};
use super::buildings::Skyline;
use super::{Pick, rng};

const SEED: u64 = 9;
/// Share of the stools near the shop that someone sits on at first.
const SEATED_SHARE: f32 = 0.5;
/// Share of the open stalls near the shop with their keeper in.
const KEEPER_CHANCE: f64 = 0.85;
/// Seconds between the phrases of people walking by, sitting and keeping
/// stalls. People sitting together talk the most.
const WALKER_TALK: Range<f32> = 10.0..40.0;
const SITTER_TALK: Range<f32> = 3.0..10.0;
const KEEPER_TALK: Range<f32> = 6.0..20.0;

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
        .init_resource::<Seats>()
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
        #[cfg(feature = "debug_port")]
        app.add_plugins(lineup::LineupPlugin);
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
    /// One for each kind of heron.
    bare: Vec<Bare>,
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
        let bare = world.resource_scope(|world, mut materials: Mut<Assets<StandardMaterial>>| {
            Bare::all(
                &mut world.resource_mut::<Assets<SurfaceMaterial>>(),
                &mut materials,
            )
        });
        Self {
            bare,
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
                posture: Posture::Out,
                carry: Carry::Maybe,
                dress: Dress::pick(&mut rng),
                sheltered: false,
                coat: Coat::new(&mut plumages, &figures.bare, &mut rng),
            };
            let bird = Bird::spawn(&mut commands, &figures, &mut rng, birth);
            commands.entity(bird).insert(walker);
        }
        let mut seats = Seats::new(places.stools.len());
        let mut stools: Vec<usize> = (0..places.stools.len()).collect();
        stools.shuffle(&mut super::rng(SEED + 3));
        let seated = (places.stools.len() as f32 * SEATED_SHARE).round() as usize;
        for index in stools.into_iter().take(seated) {
            let stool = &places.stools[index];
            let Some(walker) = Walker::near(stool.at.translation, &mut paths) else {
                continue;
            };
            let walker = Walker::seated(walker, index, stool, &mut seats, &mut rng);
            let birth = Birth {
                at: walker.place(),
                posture: Posture::Out,
                carry: Carry::Maybe,
                dress: Dress::pick(&mut rng),
                sheltered: false,
                coat: Coat::new(&mut plumages, &figures.bare, &mut rng),
            };
            let bird = Bird::spawn(&mut commands, &figures, &mut rng, birth);
            commands.entity(bird).insert(walker);
        }
        commands.insert_resource(seats);
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
