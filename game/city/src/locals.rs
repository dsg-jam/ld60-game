//! The people of the city: walkers on the sidewalks, customers on the
//! stools of the stalls and keepers behind their counters.

mod gaze;
mod keeper;
mod limb;
mod plumage;
mod shadows;
mod sound;
mod strider;
mod umbrella;
mod walk;

use std::ops::Range;

use bevy::prelude::*;
use ld60_geometry::surface::{Finish, SurfaceMaterial};
use rand::rngs::SmallRng;
use rand::{Rng, RngExt, SeedableRng};

use self::gaze::Gaze;
use self::keeper::Keeper;
use self::plumage::{Coat, PlumageMaterial};
use self::strider::{Birth, Posture, Strider};
use self::walk::Walker;
use super::buildings::Skyline;
use super::{Pick, glow, rng};

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

const SKINS: [Color; 6] = [
    Color::srgb_u8(0x7a, 0x8a, 0x6e),
    Color::srgb_u8(0x9a, 0x7e, 0x5a),
    Color::srgb_u8(0x8a, 0x7a, 0x92),
    Color::srgb_u8(0x5e, 0x74, 0x86),
    Color::srgb_u8(0xa0, 0x8a, 0x7a),
    Color::srgb_u8(0x6a, 0x64, 0x58),
];
const SHELLS: [Color; 5] = [
    Color::srgb_u8(0x6a, 0x4a, 0x34),
    Color::srgb_u8(0x5a, 0x6a, 0x4a),
    Color::srgb_u8(0x7a, 0x5a, 0x4a),
    Color::srgb_u8(0x4a, 0x4a, 0x52),
    Color::srgb_u8(0x8a, 0x6a, 0x3a),
];
const EYES: [Color; 4] = [
    Color::srgb_u8(0xff, 0xd0, 0x70),
    Color::srgb_u8(0x70, 0xf0, 0xff),
    Color::srgb_u8(0xb0, 0xff, 0x80),
    Color::srgb_u8(0xff, 0x80, 0xc0),
];
const EYE_GLOW: f32 = 8.0;

pub(super) struct LocalsPlugin;

impl Plugin for LocalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            sound::LocalSoundPlugin,
            shadows::ShadowPlugin,
            plumage::PlumagePlugin,
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
            (
                Walker::walk,
                Keeper::work,
                Gaze::glance,
                (Strider::pose, Keeper::pose),
            )
                .chain(),
        )
        .add_systems(Update, (Strider::open_umbrellas, Strider::soak));
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
    /// A sphere one unit across.
    ball: Handle<Mesh>,
    /// A cylinder one unit across and long, along Y.
    rod: Handle<Mesh>,
    skins: Vec<Handle<SurfaceMaterial>>,
    umbrellas: umbrella::Kit,
    striders: strider::Kit,
    shells: Vec<Handle<SurfaceMaterial>>,
    eyes: Vec<Handle<StandardMaterial>>,
}

/// The colors of one person.
#[derive(Debug, Clone)]
struct Colors {
    skin: Handle<SurfaceMaterial>,
    /// The skull, skin or feathered.
    face: Handle<SurfaceMaterial>,
    eyes: Handle<StandardMaterial>,
}

impl Colors {
    fn with_face(self, face: &Handle<SurfaceMaterial>) -> Self {
        Self {
            face: face.clone(),
            ..self
        }
    }
}

impl FromWorld for Figures {
    fn from_world(world: &mut World) -> Self {
        let umbrellas = world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
            let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
            umbrella::Kit::new(&mut meshes, &mut materials)
        });
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let striders = strider::Kit::new(&mut meshes, &mut rng(SEED + 2));
        let ball = meshes.add(Sphere::new(0.5).mesh().uv(16, 10));
        let rod = meshes.add(Cylinder::new(0.5, 1.0).mesh().resolution(8));
        let mut surfaces = world.resource_mut::<Assets<SurfaceMaterial>>();
        let mut all = |colors: &[Color], finish: Finish| -> Vec<_> {
            colors
                .iter()
                .map(|&color| surfaces.add(finish.scaled(color)))
                .collect()
        };
        let skins = all(&SKINS, Finish::Hide);
        let shells = all(&SHELLS, Finish::Shell);
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let eyes = EYES
            .iter()
            .map(|&color| materials.add(glow(color, EYE_GLOW)))
            .collect();
        Self {
            ball,
            rod,
            skins,
            umbrellas,
            striders,
            shells,
            eyes,
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
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let mut rng = rng(SEED);
        let mut paths = super::rng(SEED + 1);
        for walker in Walker::crowd(&mut paths) {
            let birth = Birth {
                at: walker.place(),
                posture: Posture::Walking,
                may_carry: true,
                sheltered: false,
                coat: Coat::new(&mut plumages, &mut surfaces, &mut rng),
            };
            let strider = Strider::spawn(&mut commands, &figures, &mut rng, birth);
            commands.entity(strider).insert(walker);
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
                coat: Coat::new(&mut plumages, &mut surfaces, &mut rng),
            };
            Strider::spawn(&mut commands, &figures, &mut rng, birth);
        }
        for &(at, half_width) in &places.counters {
            if rng.random_bool(KEEPER_CHANCE) {
                Keeper::spawn(&mut commands, &figures, &mut rng, at, half_width);
            }
        }
    }

    fn colors(&self, rng: &mut impl Rng) -> Colors {
        let skin = self.skins.pick(rng).clone();
        Colors {
            face: skin.clone(),
            skin,
            eyes: self.eyes.pick(rng).clone(),
        }
    }

    fn ball<M: Material>(&self, material: &Handle<M>, transform: Transform) -> impl Bundle {
        (
            Mesh3d(self.ball.clone()),
            MeshMaterial3d(material.clone()),
            transform,
        )
    }

    fn rod<M: Material>(&self, material: &Handle<M>, transform: Transform) -> impl Bundle {
        (
            Mesh3d(self.rod.clone()),
            MeshMaterial3d(material.clone()),
            transform,
        )
    }

    /// A head of `size` on `parent`, its face towards +Z: a long skull with
    /// two or three glowing eyes, and feelers if it has `feelers`.
    fn spawn_head(
        &self,
        parent: &mut ChildSpawnerCommands,
        colors: &Colors,
        size: f32,
        feelers: bool,
        rng: &mut impl Rng,
    ) {
        let skull = Vec3::new(0.2, 0.18, 0.28) * size;
        parent.spawn(self.ball(
            &colors.face,
            Transform::from_xyz(0.0, 0.0, 0.04 * size).with_scale(skull),
        ));
        let eye = Vec3::splat(0.035 * size);
        let mut eyes = vec![Vec3::new(-0.055, 0.03, 0.15), Vec3::new(0.055, 0.03, 0.15)];
        if rng.random_bool(0.3) {
            eyes.push(Vec3::new(0.0, 0.08, 0.13));
        }
        for at in eyes {
            parent.spawn(self.ball(
                &colors.eyes,
                Transform::from_translation(at * size).with_scale(eye),
            ));
        }
        if feelers {
            let length = rng.random_range(0.1..0.2);
            for side in [-1.0, 1.0] {
                let root = Vec3::new(side * 0.05, 0.07, -0.04) * size;
                let tip = root + Vec3::new(side * 0.05, length, -length * 0.8) * size;
                parent.spawn(self.rod(&colors.skin, limb::rod(root, tip, 0.016 * size)));
            }
        }
    }
}
