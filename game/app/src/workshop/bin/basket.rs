use std::f32::consts::TAU;

use bevy::prelude::*;
use ld60_geometry::triangles::Triangles;

/// Points around a wire.
const WIRE_SIDES: usize = 6;
/// Points around a ring.
const RING_STEPS: usize = 96;
/// The plate in the bottom.
const PLATE: f32 = 0.003;

/// A round wire basket of `size`, centered on the origin like a cylinder,
/// narrower at the bottom by `narrow` on each side. `bars` wires of
/// `thickness` run from a ring at the bottom up to a rolled rim of `rim`,
/// with another ring a third of the way up.
pub(super) fn basket(size: Vec3, narrow: f32, bars: usize, thickness: f32, rim: f32) -> Mesh {
    let half = size.y / 2.0;
    let top = size.x / 2.0 - rim;
    let bottom = top - narrow;
    let radius = |y: f32| bottom.lerp(top, (y + half) / size.y);
    let mut mesh = Triangles::default();
    let low = -half + thickness;
    let high = half - rim;
    for bar in 0..bars {
        let out = Vec2::from_angle(bar as f32 / bars as f32 * TAU);
        let at = |y: f32| (out * radius(y)).extend(y).xzy();
        wire(&mut mesh, at(low), at(high), thickness / 2.0);
    }
    ring(&mut mesh, low, radius(low), thickness);
    ring(
        &mut mesh,
        -half + size.y / 3.0,
        radius(-half + size.y / 3.0),
        thickness * 0.8,
    );
    ring(&mut mesh, high, radius(high), rim);
    plate(&mut mesh, -half + PLATE / 2.0, radius(low));
    mesh.build()
}

/// A round wire of `radius` from `a` to `b`, with open ends.
fn wire(mesh: &mut Triangles, a: Vec3, b: Vec3, radius: f32) {
    let along = (b - a).normalize();
    let side = along.any_orthonormal_vector();
    let loops: Vec<Vec<u32>> = [a, b]
        .iter()
        .map(|&end| {
            (0..WIRE_SIDES)
                .map(|step| {
                    let turn = Quat::from_axis_angle(along, step as f32 / WIRE_SIDES as f32 * TAU);
                    let out = turn * side;
                    mesh.vertex(end + out * radius, out)
                })
                .collect()
        })
        .collect();
    mesh.strip(&loops[0], &loops[1]);
}

/// A horizontal ring of wire `thickness` thick at height `y`, `radius` from
/// the middle.
fn ring(mesh: &mut Triangles, y: f32, radius: f32, thickness: f32) {
    let loops: Vec<Vec<u32>> = (0..WIRE_SIDES)
        .map(|step| {
            let around = Vec2::from_angle(step as f32 / WIRE_SIDES as f32 * TAU);
            (0..RING_STEPS)
                .map(|index| {
                    let out = Vec2::from_angle(index as f32 / RING_STEPS as f32 * TAU);
                    let normal = (out * around.x).extend(around.y).xzy();
                    let center = (out * radius).extend(y).xzy();
                    mesh.vertex(center + normal * thickness / 2.0, normal)
                })
                .collect()
        })
        .collect();
    for step in 0..WIRE_SIDES {
        mesh.strip(&loops[step], &loops[(step + 1) % WIRE_SIDES]);
    }
}

/// The round plate in the bottom, at height `y`.
fn plate(mesh: &mut Triangles, y: f32, radius: f32) {
    for (y, up) in [(y + PLATE / 2.0, Vec3::Y), (y - PLATE / 2.0, Vec3::NEG_Y)] {
        let edge: Vec<u32> = (0..RING_STEPS)
            .map(|index| {
                let out = Vec2::from_angle(index as f32 / RING_STEPS as f32 * TAU) * radius;
                mesh.vertex(out.extend(y).xzy(), up)
            })
            .collect();
        let middle = mesh.vertex(Vec3::Y * y, up);
        mesh.fan(middle, &edge);
    }
}
