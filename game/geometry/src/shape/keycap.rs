use bevy::prelude::*;

use crate::triangles::Triangles;

/// Points across and along the dish in the top of a keycap.
const STEPS: usize = 8;

/// A keycap of `size` centered on the origin. Its sides lean in by `taper`
/// towards the top, and its top dips by `dish` in the middle from front to
/// back, where a finger rests. The top has UVs from 0 to 1 like the top of
/// a [`Cuboid`]. It has no bottom.
pub(super) fn keycap(size: Vec3, taper: f32, dish: f32) -> Mesh {
    let half = size / 2.0;
    let top = (half.xz() - taper).max(Vec2::splat(0.0005));
    let mut mesh = Triangles::default();
    let height = |z: f32| half.y - dish * (1.0 - (z / top.y).powi(2));
    let grid: Vec<Vec<u32>> = (0..=STEPS)
        .map(|j| {
            let v = j as f32 / STEPS as f32;
            let z = (v * 2.0 - 1.0) * top.y;
            // The dish slopes up from front and back towards the edges.
            let slope = 2.0 * dish * z / (top.y * top.y);
            let normal = Vec3::new(0.0, 1.0, -slope).normalize();
            (0..=STEPS)
                .map(|i| {
                    let u = i as f32 / STEPS as f32;
                    let x = (u * 2.0 - 1.0) * top.x;
                    mesh.vertex_at(Vec3::new(x, height(z), z), normal, Vec2::new(u, v))
                })
                .collect()
        })
        .collect();
    for j in 0..STEPS {
        for i in 0..STEPS {
            let [a, b, c, d] = [
                grid[j][i],
                grid[j][i + 1],
                grid[j + 1][i + 1],
                grid[j + 1][i],
            ];
            mesh.triangle([a, b, c]);
            mesh.triangle([a, c, d]);
        }
    }

    // Each side runs from the edge of the top down to the edge of the
    // bottom.
    let edge = |t: f32, side: usize| -> (Vec3, Vec3, Vec3) {
        let s = t * 2.0 - 1.0;
        let (top_xz, bottom_xz, out) = match side {
            0 => (
                Vec2::new(s * top.x, -top.y),
                Vec2::new(s * half.x, -half.z),
                Vec3::NEG_Z,
            ),
            1 => (
                Vec2::new(top.x, s * top.y),
                Vec2::new(half.x, s * half.z),
                Vec3::X,
            ),
            2 => (
                Vec2::new(-s * top.x, top.y),
                Vec2::new(-s * half.x, half.z),
                Vec3::Z,
            ),
            _ => (
                Vec2::new(-top.x, -s * top.y),
                Vec2::new(-half.x, -s * half.z),
                Vec3::NEG_X,
            ),
        };
        (
            Vec3::new(top_xz.x, height(top_xz.y), top_xz.y),
            Vec3::new(bottom_xz.x, -half.y, bottom_xz.y),
            out,
        )
    };
    for side in 0..4 {
        for step in 0..STEPS {
            let (top_a, bottom_a, out) = edge(step as f32 / STEPS as f32, side);
            let (top_b, bottom_b, _) = edge((step + 1) as f32 / STEPS as f32, side);
            mesh.flat([top_a, top_b, bottom_b], out);
            mesh.flat([top_a, bottom_b, bottom_a], out);
        }
    }
    mesh.build()
}
