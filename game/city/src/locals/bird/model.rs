use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{mesh::skinning::SkinnedMeshInverseBindposes, prelude::*};
use ld60_geometry::triangles::{End, Skin, Triangles};
use rand::{Rng, RngExt};

use super::super::plumage::{Cover, Feather, Feathers, Kind};
use super::skeleton::{Bone, HEAD, HEAD_RADII, HIP, HULL, HULL_RADII, KNEES, RUMP, SHOULDER, Side};

/// Different coats to pick from, with and without a crest.
const VARIANTS: usize = 4;
/// The eyes, on the head, relative to its middle.
pub(super) const EYE: Vec3 = Vec3::new(0.066, 0.025, 0.104);
const EYE_RADII: Vec3 = Vec3::new(0.036, 0.041, 0.026);
/// Where the beak starts, relative to the middle of the head, and its
/// length and width.
const BEAK: Vec3 = Vec3::new(0.0, -0.035, 0.14);
const BEAK_LENGTH: f32 = 0.06;
const BEAK_WIDTH: f32 = 0.03;
/// The glint in each eye, a sliver of the light that falls into it.
const GLINT: f32 = 0.0075;
const GLINT_LIGHT: f32 = 1.5;
const IRISES: [Color; 3] = [
    Color::srgb_u8(0x14, 0x10, 0x0c),
    Color::srgb_u8(0x2a, 0x18, 0x0c),
    Color::srgb_u8(0x10, 0x16, 0x1c),
];
const SIDES: u32 = 8;

/// The meshes all birds share, made in the rest pose of [`Bone::rest`].
#[derive(Debug)]
pub(in super::super) struct Kit {
    pub(super) plumages: Vec<Handle<Mesh>>,
    pub(super) skin: Handle<Mesh>,
    pub(super) eye: Handle<Mesh>,
    pub(super) glint: Handle<Mesh>,
    pub(super) irises: Vec<Handle<StandardMaterial>>,
    pub(super) glint_light: Handle<StandardMaterial>,
    pub(super) bindposes: Handle<SkinnedMeshInverseBindposes>,
}

impl Kit {
    pub(in super::super) fn new(
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
        rng: &mut impl Rng,
    ) -> Self {
        let plumages = (0..VARIANTS)
            .map(|variant| meshes.add(skinned(plumage(variant % 2 == 0, rng))))
            .collect();
        let irises = IRISES
            .iter()
            .map(|&color| {
                materials.add(StandardMaterial {
                    base_color: color,
                    perceptual_roughness: 0.08,
                    reflectance: 0.7,
                    ..default()
                })
            })
            .collect();
        let inverse: Vec<Mat4> = Bone::rests()
            .iter()
            .map(|rest| rest.to_matrix().inverse())
            .collect();
        Self {
            plumages,
            skin: meshes.add(skinned(skin())),
            eye: meshes.add(Sphere::new(1.0).mesh().uv(16, 12).scaled_by(EYE_RADII)),
            glint: meshes.add(Sphere::new(GLINT).mesh().uv(12, 8)),
            irises,
            glint_light: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::rgb(GLINT_LIGHT, GLINT_LIGHT, GLINT_LIGHT),
                unlit: true,
                ..default()
            }),
            bindposes: bindposes.add(SkinnedMeshInverseBindposes::from(inverse)),
        }
    }

    /// Where the glint sits on an eye, on the right side.
    pub(super) fn glint_at() -> Vec3 {
        Vec3::new(EYE_RADII.x * 0.35, EYE_RADII.y * 0.45, EYE_RADII.z * 0.85)
    }
}

/// A mesh with the bounds that let it be culled as it moves.
fn skinned(mut mesh: Mesh) -> Mesh {
    if let Err(err) = mesh.generate_skinned_mesh_bounds() {
        error!(error = &err as &dyn std::error::Error, "bird bounds");
    }
    mesh
}

/// How far `point` lies along the segment from `from` to `to`, from 0 to 1.
fn along(point: Vec3, from: Vec3, to: Vec3) -> f32 {
    let axis = to - from;
    ((point - from).dot(axis) / axis.length_squared()).clamp(0.0, 1.0)
}

/// Bound to the hull near the top of a limb and to its bones below,
/// blending across the joints.
fn limb_skin(point: Vec3, side: Side, arm: bool) -> Skin {
    let (upper, lower, end) = if arm {
        (Bone::UpperArm(side), Bone::Forearm(side), Bone::Hand(side))
    } else {
        (Bone::Thigh(side), Bone::Shin(side), Bone::Foot(side))
    };
    let [root, joint, tip] = [upper, lower, end].map(|bone| bone.rest().translation);
    // How far down the limb, from 0 at its root over 1 at the joint to 2
    // at its tip.
    let second = along(point, joint, tip);
    let down = if second > 0.0 {
        1.0 + second
    } else {
        along(point, root, joint)
    };
    if down < 0.15 {
        Skin::between(Bone::Hull.joint(), upper.joint(), down / 0.15)
    } else if down < 0.85 {
        Skin::one(upper.joint())
    } else if down < 1.15 {
        Skin::between(upper.joint(), lower.joint(), (down - 0.85) / 0.3)
    } else if down < 1.85 {
        Skin::one(lower.joint())
    } else {
        Skin::between(lower.joint(), end.joint(), (down - 1.85) / 0.3)
    }
}

/// The feathers and down of a bird, with a crest if it has `crest`.
fn plumage(crest: bool, rng: &mut impl Rng) -> Mesh {
    let mut mesh = Triangles::default();
    let mut feathers = Feathers(&mut mesh);

    let mark = feathers.mark();
    feathers.down(HULL, HULL_RADII * 0.9);
    feathers.coat(
        Cover {
            center: HULL,
            radii: HULL_RADII,
            count: 300,
            flow: Vec3::new(0.0, -0.45, -1.0),
            length: 0.11,
            kind: Kind::Contour,
        },
        |_| false,
        rng,
    );
    feathers.bind_since(mark, |_| Skin::one(Bone::Hull.joint()));

    // The short neck, from the hull into the head.
    let mark = feathers.mark();
    let neck = HULL.lerp(HEAD, 0.6) + Vec3::Z * 0.04;
    feathers.coat(
        Cover {
            center: neck,
            radii: Vec3::new(0.13, 0.11, 0.12),
            count: 90,
            flow: Vec3::new(0.0, -1.0, -0.3),
            length: 0.075,
            kind: Kind::Shimmer,
        },
        |_| false,
        rng,
    );
    feathers.bind_since(mark, |point| {
        let t = (point.y - (HEAD.y - 0.14)) / 0.1;
        Skin::between(Bone::Hull.joint(), Bone::Head.joint(), t)
    });

    let mark = feathers.mark();
    feathers.down(HEAD, HEAD_RADII * 0.93);
    let eyes = Side::BOTH.map(|side| HEAD + side.mirror(EYE));
    let beak = HEAD + BEAK;
    feathers.coat(
        Cover {
            center: HEAD,
            radii: HEAD_RADII,
            count: 320,
            flow: Vec3::new(0.0, -0.15, -1.0),
            length: 0.05,
            kind: Kind::Contour,
        },
        |root| {
            eyes.iter()
                .any(|eye| root.distance(*eye) < EYE_RADII.y * 1.3)
                || root.distance(beak) < BEAK_WIDTH * 1.2
        },
        rng,
    );
    if crest {
        let plumes = rng.random_range(3..6);
        for index in 0..plumes {
            let spread = (index as f32 / (plumes - 1) as f32 - 0.5) * 0.6;
            feathers.push(
                Feather {
                    root: HEAD + Vec3::new(spread * 0.04, HEAD_RADII.y * 0.85, 0.03),
                    along: Vec3::new(spread, rng.random_range(0.8..1.2), -0.7).normalize(),
                    out: Vec3::new(0.0, 0.3, 1.0).normalize(),
                    length: rng.random_range(0.08..0.14),
                    width: 0.035,
                    curl: -0.7,
                    kind: Kind::Plume,
                },
                rng,
            );
        }
    }
    feathers.bind_since(mark, |_| Skin::one(Bone::Head.joint()));

    let mark = feathers.mark();
    let plumes = rng.random_range(5..8);
    for index in 0..plumes {
        let spread = index as f32 / (plumes - 1) as f32 - 0.5;
        feathers.push(
            Feather {
                root: RUMP + Vec3::new(spread * 0.08, 0.0, 0.04),
                along: Vec3::new(spread * 0.8, rng.random_range(0.2..0.5), -1.0).normalize(),
                out: Vec3::new(spread * 0.3, 1.0, 0.2).normalize(),
                length: rng.random_range(0.16..0.26) * (1.0 - 0.3 * spread.abs()),
                width: 0.06,
                curl: -0.3,
                kind: Kind::Plume,
            },
            rng,
        );
    }
    feathers.bind_since(mark, |_| Skin::one(Bone::Tail.joint()));

    for side in Side::BOTH {
        let mark = feathers.mark();
        let hip = side.mirror(HIP);
        let knee = Bone::Shin(side).rest().translation;
        sleeve(
            &mut feathers,
            hip,
            hip.lerp(knee, 0.75),
            0.06,
            0.08,
            -KNEES,
            rng,
        );
        feathers.bind_since(mark, |point| limb_skin(point, side, false));

        let mark = feathers.mark();
        let shoulder = side.mirror(SHOULDER);
        let elbow = Bone::Forearm(side).rest().translation;
        let wrist = Bone::Hand(side).rest().translation;
        sleeve(&mut feathers, shoulder, elbow, 0.04, 0.06, Vec3::Y, rng);
        sleeve(&mut feathers, elbow, wrist, 0.032, 0.05, Vec3::Y, rng);
        // A fringe of longer feathers along the back of the arm, like a
        // small wing.
        for step in 0..6 {
            let t = step as f32 / 5.0;
            let root = if t < 0.5 {
                shoulder.lerp(elbow, t * 2.0)
            } else {
                elbow.lerp(wrist, t * 2.0 - 1.0)
            };
            feathers.push(
                Feather {
                    root,
                    along: Vec3::new(side.sign() * 0.2, -0.6, -1.0).normalize(),
                    out: side.mirror(Vec3::new(1.0, 0.2, 0.0)).normalize(),
                    length: 0.07 + 0.05 * (PI * t).sin(),
                    width: 0.045,
                    curl: 0.4,
                    kind: Kind::Contour,
                },
                rng,
            );
        }
        feathers.bind_since(mark, |point| limb_skin(point, side, true));
    }
    mesh.build()
}

/// Rings of feathers around a limb from `from` to `to`, `radius` thick,
/// lying down the limb, the rings turned so a seam faces `seam`.
fn sleeve(
    feathers: &mut Feathers,
    from: Vec3,
    to: Vec3,
    radius: f32,
    length: f32,
    seam: Vec3,
    rng: &mut impl Rng,
) {
    const AROUND: usize = 7;
    let axis = (to - from).normalize();
    let side = (seam - axis * seam.dot(axis)).normalize_or(axis.any_orthonormal_vector());
    let rings = ((to - from).length() / (length * 0.45)).ceil() as usize;
    for ring in 0..rings {
        let center = from.lerp(to, ring as f32 / rings as f32);
        for step in 0..AROUND {
            let angle = (step as f32 + 0.5 * (ring % 2) as f32) * PI * 2.0 / AROUND as f32;
            let out = Quat::from_axis_angle(axis, angle) * side;
            feathers.push(
                Feather {
                    root: center + out * radius * 0.8,
                    along: axis,
                    out,
                    length: length * rng.random_range(0.85..1.15),
                    width: length * 0.8,
                    curl: 0.4,
                    kind: Kind::Contour,
                },
                rng,
            );
        }
    }
}

/// The bare skin: legs, toes, hands and the beak.
fn skin() -> Mesh {
    let mut mesh = Triangles::default();
    for side in Side::BOTH {
        let hip = side.mirror(HIP);
        let knee = Bone::Shin(side).rest().translation;
        let ankle = Bone::Foot(side).rest().translation;
        let mark = mesh.mark();
        mesh.tube(
            &[hip, knee],
            |t| 0.04 - 0.012 * t,
            SIDES,
            [End::Round, End::Round],
        );
        mesh.tube(
            &[knee, knee.lerp(ankle, 0.5), ankle],
            |t| 0.026 - 0.008 * t,
            SIDES,
            [End::Round, End::Round],
        );
        mesh.bind_since(mark, |point| limb_skin(point, side, false));

        let mark = mesh.mark();
        for (angle, length) in [(-0.5, 0.075), (0.0, 0.085), (0.5, 0.075), (PI, 0.035)] {
            let ahead = Quat::from_rotation_y(angle * side.sign()) * Vec3::Z;
            let tip = ankle.with_y(0.008) + ahead * length;
            let middle = ankle.with_y(0.016) + ahead * length * 0.5;
            mesh.tube(
                &[ankle - Vec3::Y * 0.02, middle, tip],
                |t| 0.012 - 0.006 * t,
                6,
                [End::Open, End::Round],
            );
        }
        mesh.bind_since(mark, |_| Skin::one(Bone::Foot(side).joint()));

        let shoulder = side.mirror(SHOULDER);
        let elbow = Bone::Forearm(side).rest().translation;
        let wrist = Bone::Hand(side).rest().translation;
        let mark = mesh.mark();
        mesh.tube(
            &[shoulder, elbow, wrist],
            |t| 0.03 - 0.012 * t,
            SIDES,
            [End::Round, End::Round],
        );
        mesh.bind_since(mark, |point| limb_skin(point, side, true));

        let mark = mesh.mark();
        hand(&mut mesh);
        mesh.transform_since(mark, &Bone::Hand(side).rest());
        mesh.bind_since(mark, |_| Skin::one(Bone::Hand(side).joint()));
    }

    let mark = mesh.mark();
    mesh.lathe(
        &[
            Vec2::new(BEAK_WIDTH * 0.5, -0.01),
            Vec2::new(BEAK_WIDTH * 0.5, 0.0),
            Vec2::new(BEAK_WIDTH * 0.42, BEAK_LENGTH * 0.4),
            Vec2::new(BEAK_WIDTH * 0.22, BEAK_LENGTH * 0.8),
            Vec2::new(0.0, BEAK_LENGTH),
        ],
        12,
    );
    let beak = Transform::from_translation(HEAD + BEAK)
        .with_rotation(Quat::from_rotation_x(FRAC_PI_2 + 0.25))
        .with_scale(Vec3::new(1.0, 1.0, 0.75));
    mesh.transform_since(mark, &beak);
    mesh.bind_since(mark, |_| Skin::one(Bone::Head.joint()));
    mesh.build()
}

/// A small hand in its bone's frame: the palm along +Y from the wrist and
/// three soft fingers curling towards +Z.
fn hand(mesh: &mut Triangles) {
    mesh.ellipsoid(
        Vec3::new(0.024, 0.03, 0.016),
        &Transform::from_xyz(0.0, 0.022, 0.0),
        8,
        10,
    );
    for (index, length) in [0.04, 0.046, 0.04].into_iter().enumerate() {
        let x = (index as f32 - 1.0) * 0.015;
        let base = Vec3::new(x, 0.045, 0.004);
        let spine: Vec<Vec3> = (0..5)
            .map(|step| {
                let t = step as f32 / 4.0;
                let bend = t * 1.6;
                base + Vec3::new(
                    x * t * 0.4,
                    bend.sin() * length / 1.6,
                    (1.0 - bend.cos()) * length / 1.6,
                )
            })
            .collect();
        mesh.tube(&spine, |t| 0.009 - 0.003 * t, 6, [End::Round, End::Round]);
    }
    // The thumb, short and to the side.
    mesh.tube(
        &[Vec3::new(0.02, 0.02, 0.006), Vec3::new(0.034, 0.038, 0.02)],
        |t| 0.008 - 0.002 * t,
        6,
        [End::Round, End::Round],
    );
}
