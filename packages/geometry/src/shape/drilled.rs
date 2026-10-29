use bevy::prelude::*;

use crate::triangles::Triangles;

/// Points along each side of the square of solid around a hole.
const SIDE_STEPS: usize = 8;
/// The square around a hole is at most this many radii wide on each side.
const SQUARE: f32 = 3.0;

/// A cuboid of `size` centered on the origin with round holes of `radius`
/// through it from top to bottom, at `holes` on its top.
pub(super) fn drilled(size: Vec3, holes: &[Vec2], radius: f32) -> Mesh {
    let half = size / 2.0;
    let mut mesh = Triangles::default();

    // The solid around each hole is a square, the rest is a grid of plain
    // rectangles between their edges.
    let squares: Vec<(Vec2, f32)> = holes
        .iter()
        .enumerate()
        .map(|(index, &hole)| {
            let near = holes
                .iter()
                .enumerate()
                .filter(|&(other, _)| other != index)
                .map(|(_, other)| (*other - hole).abs().max_element() / 2.0)
                .fold(f32::INFINITY, f32::min);
            let edge = (half.xz() - hole.abs()).min_element();
            (hole, (SQUARE * radius).min(near).min(edge))
        })
        .collect();
    let lines = |axis: fn(Vec2) -> f32, half: f32| {
        let mut lines: Vec<f32> = squares
            .iter()
            .flat_map(|&(hole, side)| [axis(hole) - side, axis(hole) + side])
            .chain([-half, half])
            .collect();
        lines.sort_by(f32::total_cmp);
        lines.dedup_by(|a, b| (*a - *b).abs() < 1e-7);
        lines
    };
    let xs = lines(|p| p.x, half.x);
    let zs = lines(|p| p.y, half.z);
    for x in xs.windows(2) {
        for z in zs.windows(2) {
            let middle = Vec2::new(x[0] + x[1], z[0] + z[1]) / 2.0;
            let inside = squares
                .iter()
                .any(|&(hole, side)| (middle - hole).abs().max_element() < side);
            if inside {
                continue;
            }
            for y in [half.y, -half.y] {
                let corners = [(x[0], z[0]), (x[1], z[0]), (x[1], z[1]), (x[0], z[1])]
                    .map(|(x, z)| mesh.vertex(Vec3::new(x, y, z), Vec3::Y * y.signum()));
                mesh.triangle([corners[0], corners[1], corners[2]]);
                mesh.triangle([corners[0], corners[2], corners[3]]);
            }
        }
    }

    for &(hole, side) in &squares {
        let corners = [
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ];
        let square: Vec<Vec2> = (0..4)
            .flat_map(|index| {
                let [a, b] = [corners[index], corners[(index + 1) % 4]];
                (0..SIDE_STEPS).map(move |step| a.lerp(b, step as f32 / SIDE_STEPS as f32) * side)
            })
            .collect();
        let at = |p: Vec2, y: f32| Vec3::new(hole.x + p.x, y, hole.y + p.y);
        let mut rims = Vec::new();
        for y in [half.y, -half.y] {
            let up = Vec3::Y * y.signum();
            let outer: Vec<u32> = square.iter().map(|&p| mesh.vertex(at(p, y), up)).collect();
            let rim: Vec<u32> = square
                .iter()
                .map(|&p| mesh.vertex(at(p.normalize() * radius, y), up))
                .collect();
            mesh.strip(&rim, &outer);
            rims.push(
                square
                    .iter()
                    .map(|&p| {
                        let out = p.normalize();
                        mesh.vertex(at(out * radius, y), -Vec3::new(out.x, 0.0, out.y))
                    })
                    .collect::<Vec<u32>>(),
            );
        }
        mesh.strip(&rims[0], &rims[1]);
    }

    let corners = [
        Vec3::new(-half.x, 0.0, -half.z),
        Vec3::new(half.x, 0.0, -half.z),
        Vec3::new(half.x, 0.0, half.z),
        Vec3::new(-half.x, 0.0, half.z),
    ];
    for index in 0..4 {
        let [a, b] = [corners[index], corners[(index + 1) % 4]];
        let out = ((a + b) / 2.0).normalize();
        let [top_a, top_b, low_a, low_b] = [
            a.with_y(half.y),
            b.with_y(half.y),
            a.with_y(-half.y),
            b.with_y(-half.y),
        ];
        mesh.flat([top_a, top_b, low_b], out);
        mesh.flat([top_a, low_b, low_a], out);
    }
    mesh.build()
}
