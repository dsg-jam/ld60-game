use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use crate::geometry::surface::{Finish, Form, SurfaceMaterial};
use crate::parts::liquid::Liquid;

const MUG_RADIUS: f32 = 0.04;
const MUG_HEIGHT: f32 = 0.095;
const MUG_WALL: f32 = 0.004;
/// How far the coffee sits below the rim.
const COFFEE_DROP: f32 = 0.014;
const STRIPE_HEIGHT: f32 = 0.008;
/// From the rim to the top of the stripe.
const STRIPE_DROP: f32 = 0.012;
/// The handle is a ring around this circle, thick enough for a grip.
const HANDLE_RADIUS: f32 = 0.025;
const HANDLE_THICKNESS: f32 = 0.012;
/// The ring's middle sits this far out of the wall, so its ends grow out of
/// the wall and two fingers fit through it.
const HANDLE_OUT: f32 = 0.002;
/// Low enough that the ring stays under the coffee inside the mug.
const HANDLE_HEIGHT: f32 = 0.045;

const GRIP_RADIUS: f32 = 0.012;
const GRIP_LENGTH: f32 = 0.07;
const FERRULE_RADIUS: f32 = 0.007;
const FERRULE_LENGTH: f32 = 0.01;
const SHAFT_RADIUS: f32 = 0.0028;
const SHAFT_LENGTH: f32 = 0.075;
const BLADE_SIZE: Vec3 = Vec3::new(0.008, 0.0055, 0.0018);

const SCREW_HEAD_RADIUS: f32 = 0.004;
const SCREW_HEAD_LENGTH: f32 = 0.0025;
const SCREW_SHANK_RADIUS: f32 = 0.0015;
const SCREW_SHANK_LENGTH: f32 = 0.012;

const CERAMIC: Color = Color::srgb_u8(0xe8, 0xe0, 0xd0);
const STRIPE: Color = Color::srgb_u8(0x3a, 0x55, 0x7a);
const COFFEE: Color = Color::srgb_u8(0x12, 0x09, 0x05);
const GRIP: Color = Color::srgb_u8(0xb0, 0x2e, 0x24);
const STEEL: Color = Color::srgb_u8(0xb8, 0xbc, 0xc0);

/// Materials shared by the props.
pub(super) struct Finishes {
    ceramic: Handle<SurfaceMaterial>,
    stripe: Handle<SurfaceMaterial>,
    grip: Handle<SurfaceMaterial>,
    steel: Handle<SurfaceMaterial>,
}

impl Finishes {
    pub(super) fn new(surfaces: &mut Assets<SurfaceMaterial>) -> Self {
        let mut finish = |finish: Finish, color: Color| {
            surfaces.add(finish.material(color, Form::Freeform, 0.0))
        };
        Self {
            ceramic: finish(Finish::Ceramic, CERAMIC),
            stripe: finish(Finish::Ceramic, STRIPE),
            grip: finish(Finish::Grip, GRIP),
            steel: finish(Finish::Metal, STEEL),
        }
    }
}

/// Spawns props as children, with the bottom of each at the parent origin.
pub(super) struct Props<'a> {
    pub(super) meshes: &'a mut Assets<Mesh>,
    pub(super) finishes: &'a Finishes,
}

impl Props<'_> {
    /// Glazed coffee mug with a ring handle towards +X.
    pub(super) fn mug(&mut self, parent: &mut ChildSpawnerCommands) {
        let wall = Extrusion::new(Annulus::new(MUG_RADIUS - MUG_WALL, MUG_RADIUS), MUG_HEIGHT);
        let upright = Quat::from_rotation_x(FRAC_PI_2);
        let stripe = Extrusion::new(
            // Only on the outside, so it doesn't fight the inner wall.
            Annulus::new(MUG_RADIUS - 0.0005, MUG_RADIUS + 0.0005),
            STRIPE_HEIGHT,
        );
        let coffee_level = MUG_HEIGHT - COFFEE_DROP;
        let parts = [
            (
                self.meshes.add(wall),
                &self.finishes.ceramic,
                Transform::from_xyz(0.0, MUG_HEIGHT / 2.0, 0.0).with_rotation(upright),
            ),
            (
                self.meshes.add(stripe),
                &self.finishes.stripe,
                Transform::from_xyz(0.0, MUG_HEIGHT - STRIPE_DROP - STRIPE_HEIGHT / 2.0, 0.0)
                    .with_rotation(upright),
            ),
            (
                self.meshes.add(Torus::new(
                    HANDLE_RADIUS - HANDLE_THICKNESS / 2.0,
                    HANDLE_RADIUS + HANDLE_THICKNESS / 2.0,
                )),
                &self.finishes.ceramic,
                Transform::from_xyz(MUG_RADIUS + HANDLE_OUT, HANDLE_HEIGHT, 0.0)
                    .with_rotation(upright),
            ),
        ];
        Self::spawn(parent, parts);
        // A flat surface reaching into the wall, so it meets the wall in one
        // clean line. It also keeps light from shining through the mug.
        parent.spawn((
            Mesh3d(self.meshes.add(Circle::new(MUG_RADIUS - MUG_WALL / 2.0))),
            Liquid::new(MUG_RADIUS - MUG_WALL, coffee_level, COFFEE),
            Transform::from_xyz(0.0, coffee_level, 0.0)
                .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ));
    }

    /// Screwdriver lying along +X, tip first.
    pub(super) fn screwdriver(&mut self, parent: &mut ChildSpawnerCommands) {
        let along_x = Quat::from_rotation_z(-FRAC_PI_2);
        let height = GRIP_RADIUS;
        let grip_end = GRIP_LENGTH / 2.0 + GRIP_RADIUS;
        let ferrule = grip_end + FERRULE_LENGTH / 2.0 - 0.002;
        let shaft = grip_end + FERRULE_LENGTH + SHAFT_LENGTH / 2.0 - 0.002;
        let blade = shaft + SHAFT_LENGTH / 2.0 + BLADE_SIZE.x / 2.0;
        let parts = [
            (
                self.meshes.add(Capsule3d::new(GRIP_RADIUS, GRIP_LENGTH)),
                &self.finishes.grip,
                Transform::from_xyz(0.0, height, 0.0).with_rotation(along_x),
            ),
            (
                self.meshes
                    .add(Cylinder::new(FERRULE_RADIUS, FERRULE_LENGTH)),
                &self.finishes.steel,
                Transform::from_xyz(ferrule, height, 0.0).with_rotation(along_x),
            ),
            (
                self.meshes.add(Cylinder::new(SHAFT_RADIUS, SHAFT_LENGTH)),
                &self.finishes.steel,
                Transform::from_xyz(shaft, height, 0.0).with_rotation(along_x),
            ),
            (
                self.meshes.add(Cuboid::from_size(BLADE_SIZE)),
                &self.finishes.steel,
                Transform::from_xyz(blade, height, 0.0),
            ),
        ];
        Self::spawn(parent, parts);
    }

    /// Screw lying on its side, head towards -X.
    pub(super) fn screw(&mut self, parent: &mut ChildSpawnerCommands) {
        // The shank leans down until its tip touches the ground.
        let lean = ((SCREW_HEAD_RADIUS - SCREW_SHANK_RADIUS) / SCREW_SHANK_LENGTH).atan();
        let axis = Quat::from_rotation_z(-FRAC_PI_2 - lean);
        let shank = axis * Vec3::Y * (SCREW_HEAD_LENGTH + SCREW_SHANK_LENGTH) / 2.0;
        let parts = [
            (
                self.meshes
                    .add(Cylinder::new(SCREW_HEAD_RADIUS, SCREW_HEAD_LENGTH)),
                &self.finishes.steel,
                Transform::from_xyz(0.0, SCREW_HEAD_RADIUS, 0.0).with_rotation(axis),
            ),
            (
                self.meshes
                    .add(Cylinder::new(SCREW_SHANK_RADIUS, SCREW_SHANK_LENGTH)),
                &self.finishes.steel,
                Transform::from_translation(Vec3::Y * SCREW_HEAD_RADIUS + shank)
                    .with_rotation(axis),
            ),
        ];
        Self::spawn(parent, parts);
    }

    fn spawn<const N: usize>(
        parent: &mut ChildSpawnerCommands,
        parts: [(Handle<Mesh>, &Handle<SurfaceMaterial>, Transform); N],
    ) {
        for (mesh, material, transform) in parts {
            parent.spawn((Mesh3d(mesh), MeshMaterial3d(material.clone()), transform));
        }
    }
}
