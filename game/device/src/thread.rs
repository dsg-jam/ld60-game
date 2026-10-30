use std::f32::consts::TAU;

use bevy::{asset::uuid_handle, prelude::*};
use ld60_geometry::triangles::Triangles;

/// The shank of a small self-tapping screw: core and outer radius, length
/// and how far the thread climbs per turn.
const CORE: f32 = 0.0008;
const OUTER: f32 = 0.0011;
const LENGTH: f32 = 0.008;
const PITCH: f32 = 0.0009;
/// The share of a turn the flat top of the thread takes.
const CREST: f32 = 0.15;
/// How long the point is, where the shank narrows to its end.
const POINT: f32 = 0.0016;
/// What is left of the radius at the end of the point.
const POINT_END: f32 = 0.2;
const SIDES: usize = 48;
const ROWS_PER_TURN: usize = 12;

const SHANK: Handle<Mesh> = uuid_handle!("0c5f8a5e-6a2d-4c9b-8e43-2f7d1b9e5a30");

pub(super) struct ThreadPlugin;

impl Plugin for ThreadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Thread::add_shank);
    }
}

/// The threaded shank of a screw.
pub(crate) struct Thread;

impl Thread {
    /// The size of the shank, like a cylinder.
    pub(crate) const SIZE: Vec3 = Vec3::new(2.0 * OUTER, LENGTH, 2.0 * OUTER);

    /// The mesh of a shank of size [`Thread::SIZE`] centered on the origin,
    /// with its point at +Y.
    pub(crate) fn shank() -> Handle<Mesh> {
        SHANK
    }

    fn add_shank(mut meshes: ResMut<Assets<Mesh>>) {
        // UUID handles always insert.
        let _ = meshes.insert(&SHANK, Self::mesh());
    }

    /// How far the surface is from the axis at `angle` and height `y`.
    fn radius(angle: f32, y: f32) -> f32 {
        let along = ((y + LENGTH / 2.0) / PITCH - angle / TAU).rem_euclid(1.0);
        let rise = (1.0 - (2.0 * along - 1.0).abs()) / (1.0 - CREST);
        let radius = CORE + (OUTER - CORE) * rise.min(1.0);
        let left = (LENGTH / 2.0 - y) / POINT;
        radius * (POINT_END + (1.0 - POINT_END) * left.clamp(0.0, 1.0))
    }

    fn surface(angle: f32, y: f32) -> Vec3 {
        let radius = Self::radius(angle, y);
        Vec3::new(angle.cos() * radius, y, angle.sin() * radius)
    }

    fn mesh() -> Mesh {
        let rows = (LENGTH / PITCH * ROWS_PER_TURN as f32).ceil() as usize;
        let mut mesh = Triangles::default();
        let (step_angle, step_y) = (TAU / SIDES as f32, LENGTH / rows as f32);
        let rings: Vec<Vec<u32>> = (0..=rows)
            .map(|row| {
                let y = -LENGTH / 2.0 + row as f32 * step_y;
                (0..SIDES)
                    .map(|side| {
                        let angle = side as f32 * step_angle;
                        let around = Self::surface(angle + step_angle / 2.0, y)
                            - Self::surface(angle - step_angle / 2.0, y);
                        let up = Self::surface(angle, y + step_y / 2.0)
                            - Self::surface(angle, y - step_y / 2.0);
                        let mut normal = up.cross(around).normalize_or_zero();
                        if normal.dot(Vec3::new(angle.cos(), 0.0, angle.sin())) < 0.0 {
                            normal = -normal;
                        }
                        mesh.vertex(Self::surface(angle, y), normal)
                    })
                    .collect()
            })
            .collect();
        for pair in rings.windows(2) {
            mesh.strip(&pair[0], &pair[1]);
        }
        for (ring, y) in [(&rings[0], -LENGTH / 2.0), (&rings[rows], LENGTH / 2.0)] {
            let end: Vec<u32> = ring
                .iter()
                .map(|&index| {
                    let at = mesh.positions()[index as usize];
                    mesh.vertex(at, Vec3::Y * y.signum())
                })
                .collect();
            let middle = mesh.vertex(Vec3::Y * y, Vec3::Y * y.signum());
            mesh.fan(middle, &end);
        }
        mesh.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thread_stays_inside_its_size_and_narrows_to_the_point() {
        for row in 0..=200 {
            let y = -LENGTH / 2.0 + LENGTH * row as f32 / 200.0;
            for side in 0..SIDES {
                let radius = Thread::radius(side as f32 / SIDES as f32 * TAU, y);
                assert!(radius <= OUTER + 1e-7, "{radius} at {y}");
            }
        }
        let end = Thread::radius(0.0, LENGTH / 2.0);
        assert!(end <= POINT_END * OUTER + 1e-7, "{end}");
    }
}
