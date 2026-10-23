//! The ring mechanism on the spine: two rings, each of two halves that
//! swing apart when the lever at its top end flips down. The tops of the
//! pages are towards -Z.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use super::model::{
    BOARD, HOLE_RADIUS, MECHANISM_TOP, METAL, Parts, RING_RADIUS, RING_WIRE, ring_center,
    ring_holes,
};

/// How far the mechanism reaches past the rings on either side.
const MECHANISM_LIP: f32 = 0.002;
const MECHANISM_LENGTH: f32 = 0.19;
/// How deep the rings reach into the mechanism.
const RING_DIP: f32 = 0.0006;
/// The mechanism is painted, so the lamp above does not glare on it.
const MECHANISM_PAINT: Color = Color::srgb_u8(0x5c, 0x60, 0x66);
/// How far each half of a ring swings out when the rings open, in radians.
const RING_SWING: f32 = 0.55;
/// How far a hole slides past the tip of an open ring to be clear of it.
const CLEAR: f32 = HOLE_RADIUS + RING_WIRE + 0.002;
/// Width, thickness and length of the lever.
const LEVER: Vec3 = Vec3::new(0.012, 0.0012, 0.02);
/// How far the lever stands up from the mechanism with the rings shut, and
/// with them open, in radians.
const LEVER_UP: [f32; 2] = [0.95, 0.1];

/// The moving parts of the mechanism.
#[derive(Debug, Default)]
pub(super) struct Mechanism {
    /// Each half of each ring, with the point in the space of the spine
    /// hinge it swings about and which way it swings.
    halves: Vec<(Entity, Vec3, f32)>,
    lever: Option<Entity>,
}

impl Mechanism {
    /// Spawns the mechanism on the spine hinge, with the rings shut.
    pub(super) fn spawn(
        hinge: &mut ChildSpawnerCommands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        let center = ring_center();
        let foot = foot();
        let width = 2.0 * (RING_RADIUS * foot.cos() + RING_WIRE + MECHANISM_LIP);
        let mechanism = Block::between(
            Vec3::new(
                center.x - width / 2.0,
                -BOARD / 2.0,
                -MECHANISM_LENGTH / 2.0,
            ),
            Vec3::new(
                center.x + width / 2.0,
                MECHANISM_TOP,
                MECHANISM_LENGTH / 2.0,
            ),
        );
        hinge.spawn((Shape::cuboid(&mechanism, MECHANISM_PAINT), Finish::Paint));
        let metal = materials.add(StandardMaterial {
            base_color: METAL,
            metallic: 1.0,
            perceptual_roughness: 0.35,
            ..default()
        });
        let mut half = |range| {
            meshes.add(
                Torus {
                    minor_radius: RING_WIRE,
                    major_radius: RING_RADIUS,
                }
                .mesh()
                .angle_range(range)
                .build(),
            )
        };
        // Each half swings out about where it comes out of the mechanism.
        let sides = [
            (half(foot..=FRAC_PI_2), foot, -1.0),
            (half(FRAC_PI_2..=PI - foot), PI - foot, 1.0),
        ];
        let mut halves = Vec::new();
        for z in ring_holes() {
            for (mesh, at, swing) in &sides {
                let pivot = (center + Vec2::from_angle(*at) * RING_RADIUS).extend(z);
                let entity = hinge
                    .spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(metal.clone()),
                        Self::half_at(pivot, 0.0),
                    ))
                    .id();
                halves.push((entity, pivot, *swing));
            }
        }
        let lever = hinge
            .spawn((
                Self::lever_at(0.0),
                Visibility::Inherited,
                children![(
                    Shape::cuboid(
                        &Block::between(
                            Vec3::new(-LEVER.x / 2.0, 0.0, 0.0),
                            Vec3::new(LEVER.x / 2.0, LEVER.y, LEVER.z),
                        ),
                        METAL,
                    ),
                    Finish::Plated,
                )],
            ))
            .id();
        Self {
            halves,
            lever: Some(lever),
        }
    }

    /// A half of a ring swung `angle` radians about `pivot`.
    fn half_at(pivot: Vec3, angle: f32) -> Transform {
        let mut half = Transform::from_translation(ring_center().extend(pivot.z))
            .with_rotation(Quat::from_rotation_x(-FRAC_PI_2));
        half.rotate_around(pivot, Quat::from_rotation_z(angle));
        half
    }

    /// The lever with the rings `open` from 0 shut to 1. It reaches out
    /// past the top end of the mechanism.
    fn lever_at(open: f32) -> Transform {
        let up = LEVER_UP[0].lerp(LEVER_UP[1], open);
        Transform::from_translation(Parts::lever_spot())
            .with_rotation(Quat::from_rotation_y(PI) * Quat::from_rotation_x(-up))
    }
}

impl Parts {
    /// Opens the rings from 0 shut to 1 with the lever.
    pub(super) fn open_rings(&self, open: f32, transforms: &mut Query<&mut Transform>) {
        let mut set = |entity: Entity, transform: Transform| {
            if let Ok(mut current) = transforms.get_mut(entity) {
                *current = transform;
            }
        };
        let swing = RING_SWING * open;
        for &(half, pivot, side) in &self.mechanism.halves {
            set(half, Mechanism::half_at(pivot, side * swing));
        }
        if let Some(lever) = self.mechanism.lever {
            set(lever, Mechanism::lever_at(open));
        }
    }

    /// Where the lever is, in the space of the spine hinge, so it can sound
    /// from there.
    pub(super) fn lever_spot() -> Vec3 {
        Vec3::new(ring_center().x, MECHANISM_TOP, -MECHANISM_LENGTH / 2.0)
    }
}

/// The angle on a ring where its right half comes out of the mechanism.
fn foot() -> f32 {
    ((MECHANISM_TOP - RING_DIP - ring_center().y) / RING_RADIUS).asin()
}

/// Where a hole that hangs on a ring at `hole`, in the space of the spine
/// hinge, is as its page slides `along` from 0 there to 1 off the open
/// rings. The hole follows the half of the ring it hangs on up to its tip
/// and on until it is clear of the wire.
pub(super) fn sliding_off(hole: Vec2, along: f32) -> Vec2 {
    let center = ring_center();
    // Works on the right half, and mirrors holes on the left half.
    let side = Vec2::new(if hole.x < center.x { -1.0 } else { 1.0 }, 1.0);
    let local = (hole - center) * side;
    let pivot = Vec2::from_angle(foot()) * RING_RADIUS;
    let swing = Vec2::from_angle(-RING_SWING);
    let wire = |angle: f32| pivot + swing.rotate(Vec2::from_angle(angle) * RING_RADIUS - pivot);
    let start = local.to_angle().clamp(foot(), FRAC_PI_2);
    let arc = RING_RADIUS * (FRAC_PI_2 - start);
    let travel = along.clamp(0.0, 1.0) * (arc + CLEAR);
    let on_wire = if travel < arc {
        wire(start + travel / RING_RADIUS)
    } else {
        wire(FRAC_PI_2) + swing.rotate(Vec2::NEG_X) * (travel - arc)
    };
    // The hole is a little off the open wire where it started.
    let off = (local - wire(start)) * (1.0 - along.clamp(0.0, 1.0));
    center + (on_wire + off) * side
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hole_slides_off_the_open_ring_and_clears_it() {
        let center = ring_center();
        let pivot = Vec2::from_angle(foot()) * RING_RADIUS;
        let swing = Vec2::from_angle(-RING_SWING);
        for side in [-1.0, 1.0] {
            for angle in [0.1, 0.8, 1.4] {
                let hole = center + Vec2::from_angle(angle) * RING_RADIUS * Vec2::new(side, 1.0);
                assert!(sliding_off(hole, 0.0).abs_diff_eq(hole, 1e-6));
                let off = (sliding_off(hole, 1.0) - center) * Vec2::new(side, 1.0);
                let nearest = (0..=50)
                    .map(|step| {
                        let angle = foot() + (FRAC_PI_2 - foot()) * step as f32 / 50.0;
                        let wire =
                            pivot + swing.rotate(Vec2::from_angle(angle) * RING_RADIUS - pivot);
                        off.distance(wire)
                    })
                    .fold(f32::INFINITY, f32::min);
                assert!(
                    nearest > HOLE_RADIUS + RING_WIRE,
                    "{side} {angle} {nearest}"
                );
            }
        }
    }
}
