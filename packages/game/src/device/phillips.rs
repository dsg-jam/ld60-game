use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, TAU};

use bevy::{
    asset::{RenderAssetUsages, uuid_handle},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

/// Pan head of a screw: radius, height and the round over its outer edge.
const HEAD_RADIUS: f32 = 0.00275;
const HEAD_HEIGHT: f32 = 0.0014;
const HEAD_ROUND: f32 = 0.0005;
/// The cross cut into the head: how far each arm reaches from the middle,
/// half its width, and how deep it is in the middle and at the end of an
/// arm.
const REACH: f32 = 0.0014;
const HALF_WIDTH: f32 = 0.0003;
const DEPTH: f32 = 0.0009;
const END_DEPTH: f32 = 0.0003;
/// Light lost at the bottom of the cross, which little light reaches.
const RECESS_SHADE: f32 = 0.65;
/// How far short of the bottom of the cross the blunt point of a tip stops.
const POINT_GAP: f32 = 0.0001;
/// The wings of a tip are a little thinner than the arms they go in.
const WING_FIT: f32 = 0.85;
/// Points along each side of the cross and around the round edge.
const EDGE_STEPS: usize = 4;
const ROUND_STEPS: usize = 6;
/// Points around a tip.
const TIP_SIDES: usize = 128;

const HEAD: Handle<Mesh> = uuid_handle!("6f1d2c64-3b0e-4f43-9a51-0b8e4d2a7c11");

pub(super) struct PhillipsPlugin;

impl Plugin for PhillipsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Phillips::add_head);
    }
}

/// A Phillips screw head and the screwdriver tip that fits it.
pub(crate) struct Phillips;

impl Phillips {
    /// The size of the head, like a cylinder.
    pub(crate) const HEAD: Vec3 = Vec3::new(2.0 * HEAD_RADIUS, HEAD_HEIGHT, 2.0 * HEAD_RADIUS);
    /// How deep the point of a tip sits in the head.
    pub(crate) const SEAT: f32 = DEPTH - POINT_GAP;

    /// The mesh of a head of size [`Phillips::HEAD`] centered on the origin,
    /// with the cross in its face at -Y. The arms run along X and Z.
    pub(crate) fn head() -> Handle<Mesh> {
        HEAD
    }

    fn add_head(mut meshes: ResMut<Assets<Mesh>>) {
        // UUID handles always insert.
        let _ = meshes.insert(&HEAD, Self::head_mesh());
    }

    fn head_mesh() -> Mesh {
        let outline = Self::outline();
        let face = -HEAD_HEIGHT / 2.0;
        let mut mesh = Builder::default();

        // The round edge and the side, from the face out and up to the base.
        let mut profile: Vec<(f32, f32, Vec2)> = (0..=ROUND_STEPS)
            .map(|step| {
                let angle = FRAC_PI_2 * (1.0 - step as f32 / ROUND_STEPS as f32);
                let normal = Vec2::new(angle.cos(), -angle.sin());
                let at = Vec2::new(HEAD_RADIUS - HEAD_ROUND, face + HEAD_ROUND)
                    + Vec2::new(normal.x, normal.y) * HEAD_ROUND;
                (at.x, at.y, normal)
            })
            .collect();
        profile.push((HEAD_RADIUS, HEAD_HEIGHT / 2.0, Vec2::X));
        let rings: Vec<Vec<u32>> = profile
            .iter()
            .map(|&(radius, y, normal)| {
                outline
                    .iter()
                    .map(|point| {
                        let out = point.normalize();
                        mesh.vertex(
                            Vec3::new(out.x * radius, y, out.y * radius),
                            Vec3::new(out.x * normal.x, normal.y, out.y * normal.x),
                        )
                    })
                    .collect()
            })
            .collect();
        for pair in rings.windows(2) {
            mesh.strip(&pair[0], &pair[1]);
        }
        let base_ring: Vec<u32> = outline
            .iter()
            .map(|point| {
                let out = point.normalize() * HEAD_RADIUS;
                mesh.vertex(Vec3::new(out.x, HEAD_HEIGHT / 2.0, out.y), Vec3::Y)
            })
            .collect();
        let middle = mesh.vertex(Vec3::Y * HEAD_HEIGHT / 2.0, Vec3::Y);
        mesh.fan(middle, &base_ring);

        // The flat face from the round edge in to the cross.
        let inner: Vec<u32> = outline
            .iter()
            .map(|point| mesh.vertex(Vec3::new(point.x, face, point.y), Vec3::NEG_Y))
            .collect();
        let edge: Vec<u32> = outline
            .iter()
            .map(|point| {
                let out = point.normalize() * (HEAD_RADIUS - HEAD_ROUND);
                mesh.vertex(Vec3::new(out.x, face, out.y), Vec3::NEG_Y)
            })
            .collect();
        mesh.strip(&inner, &edge);

        // The walls of the cross and its floor, which falls towards the
        // middle.
        let floor = |point: Vec2| {
            let along = point.x.abs().max(point.y.abs());
            let depth = END_DEPTH + (DEPTH - END_DEPTH) * (1.0 - along / REACH);
            Vec3::new(point.x, face + depth, point.y)
        };
        let bottom = Vec3::Y * (face + DEPTH);
        for (index, &a) in outline.iter().enumerate() {
            let b = outline[(index + 1) % outline.len()];
            let along = b - a;
            let inward = Vec3::new(-along.y, 0.0, along.x);
            let [top_a, top_b] = [a, b].map(|point| Vec3::new(point.x, face, point.y));
            mesh.flat([top_a, top_b, floor(b)], inward);
            mesh.flat([top_a, floor(b), floor(a)], inward);
            mesh.flat([bottom, floor(a), floor(b)], Vec3::NEG_Y);
        }
        let shade = mesh
            .positions
            .iter()
            .map(|point| {
                let inside = point.xz().length() <= REACH * 1.01;
                let depth = if inside {
                    (point.y - face) / DEPTH
                } else {
                    0.0
                };
                let shade = 1.0 - RECESS_SHADE * depth.clamp(0.0, 1.0);
                [shade, shade, shade, 1.0]
            })
            .collect::<Vec<_>>();
        mesh.build()
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, shade)
    }

    /// The edge of the cross on the face, going around it one way, with
    /// [`EDGE_STEPS`] points along each of its sides.
    fn outline() -> Vec<Vec2> {
        let corners: Vec<Vec2> = (0..4)
            .flat_map(|arm| {
                let turn = Vec2::from_angle(arm as f32 * FRAC_PI_2);
                [
                    Vec2::new(HALF_WIDTH, -HALF_WIDTH),
                    Vec2::new(REACH, -HALF_WIDTH),
                    Vec2::new(REACH, HALF_WIDTH),
                ]
                .map(|corner| turn.rotate(corner))
            })
            .collect();
        (0..corners.len())
            .flat_map(|index| {
                let [a, b] = [corners[index], corners[(index + 1) % corners.len()]];
                (0..EDGE_STEPS).map(move |step| a.lerp(b, step as f32 / EDGE_STEPS as f32))
            })
            .collect()
    }

    /// A tip `length` long that fits the head, on a shaft of `radius`. It
    /// is centered on the origin with its point at +Y, and its wings run
    /// along X and Z. Each wing slopes back like the floor of the cross, and
    /// the grooves between them close up towards the shaft.
    pub(crate) fn tip(radius: f32, length: f32) -> Mesh {
        let wing = HALF_WIDTH * WING_FIT;
        // Seated, the edge of a wing meets the end of an arm of the cross.
        let spread = REACH / (DEPTH - END_DEPTH);
        let core_spread = (radius - wing) / (0.8 * length);
        let section = |back: f32, angle: f32| {
            let reach = (spread * (back + POINT_GAP)).min(radius);
            let core = wing + core_spread * back;
            let off = (angle + FRAC_PI_4).rem_euclid(FRAC_PI_2) - FRAC_PI_4;
            let (sin, cos) = off.abs().sin_cos();
            let plus = (reach / cos).min(if sin > 0.0 { wing / sin } else { f32::MAX });
            let distance = plus.max(core).min(radius);
            Vec3::new(
                angle.cos() * distance,
                length / 2.0 - back,
                angle.sin() * distance,
            )
        };
        let backs = [
            0.0, 0.05, 0.1, 0.2, 0.3, 0.45, 0.6, 0.8, 1.0, 1.3, 1.7, 2.2, 2.8, 3.5, 4.3, 5.2, 6.0,
        ]
        .map(|back: f32| (back * 0.001).min(length));
        let mut mesh = Builder::default();
        for pair in backs.windows(2) {
            for side in 0..TIP_SIDES {
                let [a, b] = [side, side + 1].map(|side| side as f32 / TIP_SIDES as f32 * TAU);
                let [near_a, near_b, far_a, far_b] = [
                    section(pair[0], a),
                    section(pair[0], b),
                    section(pair[1], a),
                    section(pair[1], b),
                ];
                let out = ((near_a + far_b) / 2.0).with_y(0.0);
                mesh.flat([near_a, far_a, far_b], out);
                mesh.flat([near_a, far_b, near_b], out);
            }
        }
        let point = Vec3::Y * length / 2.0;
        for side in 0..TIP_SIDES {
            let [a, b] =
                [side, side + 1].map(|side| section(0.0, side as f32 / TIP_SIDES as f32 * TAU));
            mesh.flat([point, a, b], Vec3::Y);
        }
        mesh.build()
    }
}

/// Collects the triangles of a mesh, each facing the way it is told.
#[derive(Default)]
struct Builder {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    indices: Vec<u32>,
}

impl Builder {
    fn vertex(&mut self, position: Vec3, normal: Vec3) -> u32 {
        self.positions.push(position);
        self.normals.push(normal);
        (self.positions.len() - 1) as u32
    }

    /// A triangle of shared vertices, facing along their normals.
    fn triangle(&mut self, [a, b, c]: [u32; 3]) {
        let [pa, pb, pc] = [a, b, c].map(|index| self.positions[index as usize]);
        let normal: Vec3 = [a, b, c]
            .map(|index| self.normals[index as usize])
            .iter()
            .sum();
        let facing = (pb - pa).cross(pc - pa).dot(normal) >= 0.0;
        self.indices
            .extend(if facing { [a, b, c] } else { [a, c, b] });
    }

    /// A triangle of its own, lit flat and facing about `towards`.
    fn flat(&mut self, [a, b, c]: [Vec3; 3], towards: Vec3) {
        let mut normal = (b - a).cross(c - a).normalize_or_zero();
        if normal.dot(towards) < 0.0 {
            normal = -normal;
        }
        let indices = [a, b, c].map(|point| self.vertex(point, normal));
        self.triangle(indices);
    }

    /// Quads between two loops of the same length.
    fn strip(&mut self, near: &[u32], far: &[u32]) {
        for index in 0..near.len() {
            let next = (index + 1) % near.len();
            self.triangle([near[index], near[next], far[next]]);
            self.triangle([near[index], far[next], far[index]]);
        }
    }

    /// Triangles from `middle` to each side of a loop.
    fn fan(&mut self, middle: u32, ring: &[u32]) {
        for index in 0..ring.len() {
            self.triangle([middle, ring[index], ring[(index + 1) % ring.len()]]);
        }
    }

    fn build(self) -> Mesh {
        let count = self.positions.len();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![Vec2::ZERO; count])
        .with_inserted_indices(Indices::U32(self.indices))
    }
}
