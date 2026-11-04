use std::f32::consts::{PI, TAU};

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use ld60_geometry::triangles::{End, Triangles};

/// Ribs, each with a gore of cloth to the next.
pub(super) const RIBS: usize = 8;
/// Half the angle the canopy spans on the sphere its ribs bend along.
const SPAN: f32 = 0.73;
/// Quads across a gore and along it.
const ACROSS: usize = 6;
const ALONG: usize = 8;
/// How far the hem curves up between the rib tips, and the cloth sags
/// between the ribs, for a canopy of radius 1.
const SCALLOP: f32 = 0.07;
const SAG: f32 = 0.025;
/// Sizes of the frame for a canopy of radius 1.
const RIB: f32 = 0.007;
const STRETCHER: f32 = 0.006;
const TIP: f32 = 0.014;
/// Where the stretchers meet the ribs, along them.
const STRETCHED_AT: f32 = 0.55;
/// The runner on the shaft, below the top, which holds the stretchers.
const RUNNER: f32 = 0.55;
/// In meters, for the handle and the folded umbrella.
const SHAFT: f32 = 0.006;
const GRIP: f32 = 0.013;
const CROOK: f32 = 0.045;
const GRIP_LENGTH: f32 = 0.12;
const FOLDED_LENGTH: f32 = 0.62;
const FOLDED_GIRTH: f32 = 0.04;
/// Segments around round parts.
const ROUND: u32 = 6;

/// Rods and balls merged into one mesh.
struct Parts {
    mesh: Mesh,
}

impl Default for Parts {
    fn default() -> Self {
        Self {
            mesh: Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new())
            .with_inserted_indices(Indices::U32(Vec::new())),
        }
    }
}

impl Parts {
    fn rod(&mut self, from: Vec3, to: Vec3, radius: f32) {
        let along = to - from;
        let rod = Cylinder::new(radius, along.length())
            .mesh()
            .resolution(ROUND)
            .build()
            .rotated_by(Quat::from_rotation_arc(Vec3::Y, along.normalize()))
            .translated_by((from + to) / 2.0);
        self.add(&rod);
    }

    fn ball(&mut self, at: Vec3, radius: f32) {
        let ball = Sphere::new(radius)
            .mesh()
            .uv(ROUND, ROUND / 2 + 1)
            .translated_by(at);
        self.add(&ball);
    }

    fn add(&mut self, part: &Mesh) {
        let mut part = part.clone();
        part.remove_attribute(Mesh::ATTRIBUTE_UV_0);
        if let Err(err) = self.mesh.merge(&part) {
            error!(error = &err as &dyn std::error::Error, "umbrella part");
        }
    }
}

/// How high the canopy is from the rim to the top, for a radius of 1.
pub(super) fn rise() -> f32 {
    (1.0 - SPAN.cos()) / SPAN.sin()
}

/// A point `along` a rib from the top at 0 to the tip at 1, on a canopy of
/// radius 1 with its top at the origin.
fn rib(angle: f32, along: f32) -> Vec3 {
    let sphere = 1.0 / SPAN.sin();
    let bend = along * SPAN;
    let out = sphere * bend.sin();
    Vec3::new(
        out * angle.cos(),
        sphere * (bend.cos() - 1.0),
        out * angle.sin(),
    )
}

/// The cloth, radius 1, its top at the origin and its rim below. Each
/// gore is pulled straight between its ribs, a little slack in the
/// middle, and its hem curves up between the rib tips.
pub(super) fn canopy() -> Mesh {
    let mut points = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for gore in 0..RIBS {
        let from = gore as f32 * TAU / RIBS as f32;
        let to = from + TAU / RIBS as f32;
        let base = points.len() as u32;
        for step in 0..=ALONG {
            for side in 0..=ACROSS {
                let across = side as f32 / ACROSS as f32;
                let arch = (PI * across).sin();
                let along = step as f32 / ALONG as f32 * (1.0 - SCALLOP * arch);
                let point =
                    rib(from, along).lerp(rib(to, along), across) - Vec3::Y * SAG * arch * along;
                points.push(point.to_array());
                uvs.push([across, along]);
            }
        }
        let row = ACROSS as u32 + 1;
        for step in 0..ALONG as u32 {
            for side in 0..ACROSS as u32 {
                let corner = base + step * row + side;
                indices.extend([
                    corner,
                    corner + 1,
                    corner + row,
                    corner + 1,
                    corner + row + 1,
                    corner + row,
                ]);
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    mesh
}

/// The ribs under the cloth with their tips, the stretchers, the runner
/// and the ferrule on top, for a canopy of radius 1.
pub(super) fn frame() -> Mesh {
    let mut parts = Parts::default();
    let runner = Vec3::NEG_Y * RUNNER * rise();
    for index in 0..RIBS {
        let angle = index as f32 * TAU / RIBS as f32;
        let under = |along: f32| rib(angle, along) * 0.995 - Vec3::Y * RIB;
        for step in 0..ALONG {
            let along = step as f32 / ALONG as f32;
            parts.rod(under(along), under(along + 1.0 / ALONG as f32), RIB);
        }
        parts.ball(rib(angle, 1.0), TIP);
        parts.rod(runner, under(STRETCHED_AT), STRETCHER);
    }
    parts.rod(runner - Vec3::Y * 0.04, runner + Vec3::Y * 0.04, 0.02);
    parts.rod(Vec3::NEG_Y * 0.03, Vec3::Y * 0.02, 0.018);
    parts.rod(Vec3::Y * 0.02, Vec3::Y * 0.09, 0.006);
    parts.mesh
}

/// The handle below the hand: a short grip and its crook in one bent tube,
/// in meters, the hand at the origin and the shaft along +Y.
pub(super) fn handle() -> Mesh {
    let bottom = Vec3::NEG_Y * GRIP_LENGTH;
    let center = bottom + Vec3::Z * CROOK;
    let spine: Vec<Vec3> = [Vec3::Y * 0.04, bottom + Vec3::Y * 0.02]
        .into_iter()
        .chain((0..=ROUND * 2).map(|step| {
            let angle = PI * step as f32 / (ROUND * 2) as f32;
            center - Vec3::new(0.0, angle.sin(), angle.cos()) * CROOK
        }))
        .collect();
    let mut mesh = Triangles::default();
    mesh.tube(&spine, |_| GRIP, ROUND * 2, [End::Round, End::Round]);
    mesh.build()
}

/// The shaft from the hand up, one meter long, to be scaled along Y.
pub(super) fn shaft() -> Mesh {
    Cylinder::new(SHAFT, 1.0)
        .mesh()
        .resolution(ROUND)
        .build()
        .translated_by(Vec3::Y * 0.5)
}

/// The cloth of a closed umbrella, wound around the shaft in folds along
/// +Y from the hand, thickest a third of the way, and the tip.
pub(super) fn folded() -> Mesh {
    const RINGS: usize = 10;
    const AROUND: usize = 4 * RIBS;
    let mut points = Vec::new();
    let mut indices = Vec::new();
    for ring in 0..=RINGS {
        let along = ring as f32 / RINGS as f32;
        let girth = FOLDED_GIRTH * (PI * along.powf(0.6)).sin().max(0.15);
        for step in 0..=AROUND {
            let angle = TAU * step as f32 / AROUND as f32 + along * 0.6;
            let fold = 1.0 + 0.2 * (RIBS as f32 * angle).cos();
            let out = girth * fold;
            points.push([
                out * angle.cos(),
                0.05 + along * FOLDED_LENGTH,
                out * angle.sin(),
            ]);
        }
    }
    let row = AROUND as u32 + 1;
    for ring in 0..RINGS as u32 {
        for step in 0..AROUND as u32 {
            let corner = ring * row + step;
            indices.extend([
                corner,
                corner + row,
                corner + 1,
                corner + 1,
                corner + row,
                corner + row + 1,
            ]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points)
    .with_inserted_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    let mut parts = Parts { mesh };
    let tip = Vec3::Y * (0.05 + FOLDED_LENGTH);
    parts.rod(tip, tip + Vec3::Y * 0.06, SHAFT);
    parts.mesh
}
