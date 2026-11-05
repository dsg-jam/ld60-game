use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use ld60_geometry::triangles::{End, Skin, Triangles};
use rand::{Rng, RngExt};

use super::model::neck_skin;
use super::skeleton::{Bone, NECK};

/// Rows and columns of the shawl.
const SHAWL_ROWS: usize = 10;
const SHAWL_COLUMNS: usize = 28;
/// The shawl hangs from its collar around the neck to its hem, leaving
/// this much open at the front, in radians.
const SHAWL_OPEN: f32 = 1.1;
const COLLAR: Vec3 = Vec3::new(0.0, 0.87, 0.03);
const HEM: Vec3 = Vec3::new(0.0, 0.5, -0.09);
/// Folds around the shawl.
const FOLDS: f32 = 7.0;

/// What a bird wears over its feathers, if anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Dress {
    Bare,
    /// A knitted scarf around the root of the neck, its ends hanging over
    /// the breast.
    Scarf,
    /// A cloth over the shoulders and the back, open at the front.
    Shawl,
}

impl Dress {
    pub(super) fn pick(rng: &mut impl Rng) -> Self {
        match rng.random_range(0..3) {
            0 => Self::Bare,
            1 => Self::Scarf,
            _ => Self::Shawl,
        }
    }
}

/// A scarf, bound to the neck and the hull.
pub(super) fn scarf() -> Mesh {
    let mut mesh = Triangles::default();
    let axis = Vec3::new(0.0, 1.0, 0.45).normalize();
    let side = Vec3::X;
    let front = side.cross(axis);
    let center = NECK + axis * 0.015;
    let ring: Vec<Vec3> = (0..=24)
        .map(|step| {
            let angle = TAU * step as f32 / 24.0;
            center + (side * angle.cos() + front * angle.sin()) * 0.046
        })
        .collect();
    let mark = mesh.mark();
    mesh.tube(&ring, |_| 0.012, 10, [End::Open, End::Open]);
    mesh.bind_since(mark, neck_skin);

    let mark = mesh.mark();
    let knot = center + front * 0.05 + side * 0.025;
    for (lean, length) in [(0.25, 0.2), (0.05, 0.16)] {
        let spine: Vec<Vec3> = (0..=6)
            .map(|step| {
                let t = step as f32 / 6.0;
                knot + Vec3::new(lean * t * 0.1, -length * t, 0.03 * (PI * t).sin())
            })
            .collect();
        ribbon(&mut mesh, &spine, 0.03, front);
    }
    mesh.bind_since(mark, |_| Skin::one(Bone::Hull.joint()));
    mesh.build()
}

/// A flat strip along `spine`, `width` wide, facing `facing`.
fn ribbon(mesh: &mut Triangles, spine: &[Vec3], width: f32, facing: Vec3) {
    let mut last: Option<[u32; 2]> = None;
    for (index, &at) in spine.iter().enumerate() {
        let next = spine[(index + 1).min(spine.len() - 1)];
        let before = spine[index.saturating_sub(1)];
        let along = (next - before).normalize_or(Vec3::NEG_Y);
        let across = along.cross(facing).normalize_or(Vec3::X);
        let normal = across.cross(along).normalize_or(facing);
        let t = index as f32 / (spine.len() - 1) as f32;
        let pair = [-0.5, 0.5].map(|side| {
            mesh.vertex_at(at + across * side * width, normal, Vec2::new(side + 0.5, t))
        });
        if let Some([a, b]) = last {
            mesh.triangle([a, b, pair[1]]);
            mesh.triangle([a, pair[1], pair[0]]);
        }
        last = Some(pair);
    }
}

/// A shawl, bound to the hull.
pub(super) fn shawl() -> Mesh {
    let mut mesh = Triangles::default();
    let mut rows: Vec<Vec<u32>> = Vec::with_capacity(SHAWL_ROWS + 1);
    for row in 0..=SHAWL_ROWS {
        let t = row as f32 / SHAWL_ROWS as f32;
        let middle = COLLAR.lerp(HEM, t);
        let radius = 0.075 + 0.13 * t.sqrt();
        let ring = (0..=SHAWL_COLUMNS)
            .map(|column| {
                let u = column as f32 / SHAWL_COLUMNS as f32;
                // From the front left round the back to the front right.
                let angle = -PI + SHAWL_OPEN / 2.0 + u * (TAU - SHAWL_OPEN);
                let fold = 1.0 + 0.06 * t * (angle * FOLDS).sin();
                let hem = 0.015 * t * (angle * 3.0 + 1.0).sin();
                let out = Vec3::new(-angle.sin(), 0.0, -angle.cos());
                let at = middle + out * Vec3::new(1.0, 0.0, 0.85) * radius * fold + Vec3::Y * hem;
                let normal = (out + Vec3::Y * 0.4 * (1.0 - t)).normalize();
                mesh.vertex_at(at, normal, Vec2::new(u, t))
            })
            .collect();
        rows.push(ring);
    }
    for pair in rows.windows(2) {
        for column in 0..SHAWL_COLUMNS {
            let [a, b] = [pair[0][column], pair[0][column + 1]];
            let [c, d] = [pair[1][column], pair[1][column + 1]];
            mesh.triangle([a, b, d]);
            mesh.triangle([a, d, c]);
        }
    }
    mesh.bind_since(0, |_| Skin::one(Bone::Hull.joint()));
    mesh.build()
}
