use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{mesh::skinning::SkinnedMeshInverseBindposes, prelude::*};
use ld60_geometry::triangles::{End, Skin, Triangles};
use rand::{Rng, RngExt};

use super::super::plumage::{Cover, Feather, Feathers, Kind, Region, point_along};
use super::dress;
use super::skeleton::{
    Bone, HEAD, HEAD_RADII, HIP, HULL, HULL_RADII, HULL_TILT, NECK, RUMP, SHOULDER, Side,
};

/// Different coats to pick from.
const VARIANTS: usize = 4;
/// The eyes, on the head, relative to its middle: small, low and close to
/// the beak.
pub(super) const EYE: Vec3 = Vec3::new(0.0165, 0.006, 0.012);
const EYE_RADIUS: f32 = 0.011;
/// The back of the head, relative to its middle, and how the head is
/// narrower than tall.
const HEAD_BACK: Vec3 = Vec3::new(0.0, 0.004, -0.05);
const HEAD_SCALE: Vec3 = Vec3::new(0.9, 1.05, 1.0);
/// The upper lid covers this much of the eye, from its top, in radians.
const LID: f32 = 1.05;
/// Where the beak starts, relative to the middle of the head, its length,
/// its width at the root and how far it points down.
const BEAK: Vec3 = Vec3::new(0.0, -0.006, 0.042);
const BEAK_LENGTH: f32 = 0.175;
const BEAK_WIDTH: f32 = 0.034;
const BEAK_DROOP: f32 = 0.12;
/// The neck in the rest pose, from the hull to under the head.
const NECK_SPINE: [Vec3; 5] = [
    NECK,
    Vec3::new(0.0, 0.94, 0.155),
    Vec3::new(0.0, 1.03, 0.115),
    Vec3::new(0.0, 1.12, 0.09),
    Vec3::new(0.0, 1.185, 0.095),
];
/// Warm, calm irises.
const IRISES: [Color; 3] = [
    Color::srgb_u8(0xc8, 0x90, 0x38),
    Color::srgb_u8(0xb0, 0x88, 0x48),
    Color::srgb_u8(0xd0, 0xa8, 0x58),
];
const SIDES: u32 = 8;

/// The meshes all birds share, made in the rest pose of [`Bone::rest`].
#[derive(Debug)]
pub(in super::super) struct Kit {
    pub(super) plumages: Vec<Handle<Mesh>>,
    pub(super) skin: Handle<Mesh>,
    pub(super) scarf: Handle<Mesh>,
    pub(super) shawl: Handle<Mesh>,
    pub(super) eye: Handle<Mesh>,
    pub(super) pupil: Handle<Mesh>,
    pub(super) irises: Vec<Handle<StandardMaterial>>,
    pub(super) pupil_black: Handle<StandardMaterial>,
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
            .map(|_| meshes.add(skinned(plumage(rng))))
            .collect();
        let irises = IRISES
            .iter()
            .map(|&color| {
                materials.add(StandardMaterial {
                    base_color: color,
                    perceptual_roughness: 0.25,
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
            scarf: meshes.add(skinned(dress::scarf())),
            shawl: meshes.add(skinned(dress::shawl())),
            eye: meshes.add(Sphere::new(EYE_RADIUS).mesh().uv(16, 12)),
            pupil: meshes.add(Sphere::new(EYE_RADIUS * 0.5).mesh().uv(12, 8)),
            irises,
            pupil_black: materials.add(StandardMaterial {
                base_color: Color::srgb_u8(0x08, 0x08, 0x08),
                perceptual_roughness: 0.2,
                ..default()
            }),
            bindposes: bindposes.add(SkinnedMeshInverseBindposes::from(inverse)),
        }
    }

    /// Where the pupil sits in the right eye: on its outer side.
    pub(super) fn pupil_at() -> Vec3 {
        Vec3::X * EYE_RADIUS * 0.62
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
pub(super) fn limb_skin(point: Vec3, side: Side, arm: bool) -> Skin {
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

/// Bound along the neck: to the hull at its root, the neck in the middle
/// and the head at the top.
pub(super) fn neck_skin(point: Vec3) -> Skin {
    let t = (point.y - NECK.y) / (HEAD.y - NECK.y);
    if t < 0.2 {
        Skin::between(Bone::Hull.joint(), Bone::Neck.joint(), t / 0.2)
    } else if t < 0.6 {
        Skin::one(Bone::Neck.joint())
    } else {
        Skin::between(Bone::Neck.joint(), Bone::Head.joint(), (t - 0.6) / 0.35)
    }
}

/// The hull, turned to lean forward.
pub(super) fn hull_place() -> Transform {
    Transform::from_translation(HULL).with_rotation(Quat::from_rotation_x(HULL_TILT))
}

/// Pale where it faces forward, like a heron's breast.
fn breast(_: Vec3, normal: Vec3) -> Region {
    if normal.z > 0.6 {
        Region::Pale
    } else {
        Region::Back
    }
}

/// The black stripe from above the eye back to the crest, and pale
/// elsewhere on the head.
fn face(point: Vec3) -> Region {
    let at = point - HEAD;
    let stripe = at.y > EYE.y + EYE_RADIUS * 0.4
        && at.y < EYE.y + EYE_RADIUS * 1.8
        && at.z < EYE.z + EYE_RADIUS
        && at.x.abs() > 0.008;
    if stripe { Region::Dark } else { Region::Pale }
}

/// The head's thickness from its back at 0 to the root of the beak at 1:
/// a flat wedge that runs on into the beak.
fn head_radius(t: f32) -> f32 {
    let smooth = |from: f32, to: f32, t: f32| {
        let t = ((t - from) / (to - from)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let swell = 0.024 + 0.01 * smooth(0.0, 0.35, t);
    swell - 0.016 * smooth(0.35, 1.0, t)
}

/// The neck's thickness from its root at 0 to the head at 1.
fn neck_radius(t: f32) -> f32 {
    0.034 - 0.013 * t
}

/// The feathers and down of a bird.
fn plumage(rng: &mut impl Rng) -> Mesh {
    let mut mesh = Triangles::default();
    let mut feathers = Feathers(&mut mesh);
    let hull = hull_place();

    let mark = feathers.mark();
    feathers.down(HULL_RADII * 0.97, &hull, Vec2::new(26.0, 14.0), breast);
    // Feathers only fringe the lower hull, over the tops of the legs.
    feathers.coat(
        Cover {
            center: HULL,
            radii: HULL_RADII,
            turn: hull.rotation,
            count: 130,
            flow: hull.rotation * Vec3::new(0.0, -1.0, -0.2),
            length: 0.085,
        },
        |point, normal| {
            let local = hull.rotation.inverse() * (point - HULL);
            (local.y < -HULL_RADII.y * 0.35).then(|| breast(point, normal))
        },
        rng,
    );
    // Long plumes over the back, lying like a shawl.
    let plumes = rng.random_range(7..10);
    for index in 0..plumes {
        let spread = index as f32 / (plumes - 1) as f32 - 0.5;
        let angle = PI + spread * 2.0;
        let around = Vec3::new(angle.sin(), 0.0, angle.cos());
        feathers.push(
            Feather {
                root: hull.transform_point(Vec3::new(0.0, 0.12, 0.0) + around * 0.085),
                along: (Vec3::new(0.0, -1.0, -0.35) + around * 0.15).normalize(),
                out: around,
                length: rng.random_range(0.2..0.28),
                width: 0.05,
                curl: 0.25,
                kind: Kind::Plume,
                region: Region::Back,
            },
            rng,
        );
    }
    feathers.bind_since(mark, |_| Skin::one(Bone::Hull.joint()));

    let mark = feathers.mark();
    feathers.down_tube(
        &smooth(&NECK_SPINE, 5),
        neck_radius,
        Vec2::new(9.0, 22.0),
        |point, normal| {
            if point.y > HEAD.y - HEAD_RADII.y * 1.2 {
                face(point)
            } else {
                breast(point, normal)
            }
        },
    );
    feathers.bind_since(mark, neck_skin);

    // Fine plumes hanging from the lower neck over the breast.
    let mark = feathers.mark();
    for _ in 0..rng.random_range(4..6) {
        let t = rng.random_range(0.0..0.3);
        let root = NECK_SPINE[1].lerp(NECK_SPINE[2], t) + Vec3::Z * 0.028;
        feathers.push(
            Feather {
                root: root + Vec3::X * rng.random_range(-0.012..0.012),
                along: Vec3::new(rng.random_range(-0.1..0.1), -1.0, 0.25).normalize(),
                out: Vec3::Z,
                length: rng.random_range(0.1..0.14),
                width: 0.04,
                curl: 0.2,
                kind: Kind::Plume,
                region: Region::Pale,
            },
            rng,
        );
    }
    feathers.bind_since(mark, |_| {
        Skin::between(Bone::Hull.joint(), Bone::Neck.joint(), 0.6)
    });

    let mark = feathers.mark();
    let head_spine: Vec<Vec3> = (0..=10)
        .map(|step| HEAD + HEAD_BACK.lerp(BEAK, step as f32 / 10.0))
        .collect();
    feathers.down_tube(&head_spine, head_radius, Vec2::new(8.0, 5.0), |point, _| {
        face(point)
    });
    let narrow = Transform::from_translation(HEAD).with_scale(HEAD_SCALE)
        * Transform::from_translation(-HEAD);
    feathers.0.transform_since(mark, &narrow);
    for side in Side::BOTH {
        lid(&mut feathers, HEAD + side.mirror(EYE), side);
    }
    let crest = rng.random_range(2..4);
    for index in 0..crest {
        let spread = (index as f32 / (crest - 1) as f32 - 0.5) * 0.3;
        feathers.push(
            Feather {
                root: HEAD + HEAD_BACK * 0.8 + Vec3::new(spread * 0.02, 0.016, 0.0),
                along: Vec3::new(spread, -0.5, -1.0).normalize(),
                out: Vec3::Y,
                length: rng.random_range(0.08..0.11),
                width: 0.014,
                curl: 0.6,
                kind: Kind::Plume,
                region: Region::Dark,
            },
            rng,
        );
    }
    feathers.bind_since(mark, |_| Skin::one(Bone::Head.joint()));

    let mark = feathers.mark();
    for index in 0..7 {
        let spread = index as f32 / 6.0 - 0.5;
        feathers.push(
            Feather {
                root: RUMP + Vec3::new(spread * 0.08, 0.0, 0.02),
                along: Vec3::new(spread * 0.5, -0.75, -0.7).normalize(),
                out: Vec3::new(spread * 0.3, 0.7, -0.7).normalize(),
                length: rng.random_range(0.1..0.13),
                width: 0.05,
                curl: 0.2,
                kind: Kind::Contour,
                region: Region::Primary,
            },
            rng,
        );
    }
    feathers.bind_since(mark, |_| Skin::one(Bone::Tail.joint()));

    for side in Side::BOTH {
        let mark = feathers.mark();
        let hip = side.mirror(HIP);
        let knee = Bone::Shin(side).rest().translation;
        feathers.sleeve(
            &[hip, hip.lerp(knee, 0.5)],
            |t| 0.045 - 0.015 * t,
            0.06,
            |_, _| Region::Back,
            rng,
        );
        feathers.bind_since(mark, |point| limb_skin(point, side, false));

        let mark = feathers.mark();
        wing(&mut feathers, side, rng);
        feathers.bind_since(mark, |point| limb_skin(point, side, true));
    }
    mesh.build()
}

/// The upper lid over an eye: a cap of down over its top.
fn lid(feathers: &mut Feathers, eye: Vec3, side: Side) {
    let radius = EYE_RADIUS * 1.12;
    let profile: Vec<Vec2> = (0..=5)
        .map(|step| {
            let angle = LID * step as f32 / 5.0;
            Vec2::new(angle.sin(), angle.cos()) * radius
        })
        .collect();
    let mark = feathers.mark();
    feathers.0.lathe(&profile, 12);
    let place = Transform::from_translation(eye).with_rotation(
        Quat::from_rotation_y(0.4 * side.sign()) * Quat::from_rotation_z(-0.15 * side.sign()),
    );
    feathers.0.transform_since(mark, &place);
    feathers.paint_down(mark, Vec2::new(6.0, 2.0), |point, _| face(point));
}

/// The arm as a folded wing: short feathers along it, and long ones
/// hanging from it down the side of the hull.
fn wing(feathers: &mut Feathers, side: Side, rng: &mut impl Rng) {
    let shoulder = side.mirror(SHOULDER);
    let elbow = Bone::Forearm(side).rest().translation;
    let wrist = Bone::Hand(side).rest().translation;
    feathers.down_tube(
        &smooth(&[shoulder, elbow, wrist], 3),
        |t| 0.03 - 0.012 * t,
        Vec2::new(7.0, 9.0),
        |_, _| Region::Back,
    );
    // Coverts down the outside of the arm, so it reads as a folded wing.
    let arm = smooth(&[shoulder, elbow, wrist], 3);
    let total: f32 = arm.windows(2).map(|pair| pair[0].distance(pair[1])).sum();
    for step in 0..9 {
        let t = step as f32 / 9.0;
        let (root, along) = point_along(&arm, t * total);
        let outside = side.mirror(Vec3::new(1.0, 0.0, 0.25)).normalize();
        let out = (outside - along * outside.dot(along)).normalize();
        feathers.push(
            Feather {
                root: root + out * (0.028 - 0.01 * t),
                along,
                out,
                length: 0.1 - 0.02 * t,
                width: 0.085 - 0.025 * t,
                curl: 0.35,
                kind: Kind::Contour,
                region: Region::Back,
            },
            rng,
        );
    }
    let out = side.mirror(Vec3::new(0.8, 0.0, -0.6)).normalize();
    for step in 0..10 {
        let t = step as f32 / 9.0;
        let root = if t < 0.4 {
            shoulder.lerp(elbow, 0.5 + t * 1.25)
        } else {
            elbow.lerp(wrist, (t - 0.4) / 0.6)
        };
        feathers.push(
            Feather {
                root: root + out * 0.01,
                along: Vec3::new(side.sign() * 0.05, -0.75, -0.65).normalize(),
                out,
                length: 0.2 + 0.03 * t,
                width: 0.075,
                curl: 0.12,
                kind: Kind::Contour,
                region: Region::Back,
            },
            rng,
        );
    }
    for step in 0..5 {
        let t = step as f32 / 4.0;
        feathers.push(
            Feather {
                root: elbow.lerp(wrist, 0.6 + 0.4 * t) + out * 0.015,
                along: Vec3::new(side.sign() * 0.04, -0.55, -0.85).normalize(),
                out,
                length: 0.28 - 0.03 * t,
                width: 0.06,
                curl: 0.1,
                kind: Kind::Contour,
                region: Region::Primary,
            },
            rng,
        );
    }
}

/// `spine` made smooth, with `between` more points between each two, on
/// a curve through all of them.
fn smooth(spine: &[Vec3], between: usize) -> Vec<Vec3> {
    let last = spine.len() - 1;
    let mut points = Vec::with_capacity(last * (between + 1) + 1);
    for index in 0..last {
        let [a, b, c, d] = [
            spine[index.saturating_sub(1)],
            spine[index],
            spine[index + 1],
            spine[(index + 2).min(last)],
        ];
        for step in 0..=between {
            let t = step as f32 / (between + 1) as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            points.push(
                0.5 * (2.0 * b
                    + (c - a) * t
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (3.0 * b - a - 3.0 * c + d) * t3),
            );
        }
    }
    points.push(spine[last]);
    points
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
            |t| 0.024 - 0.012 * t,
            SIDES,
            [End::Round, End::Round],
        );
        // The joint that bends back shows as a small knob.
        mesh.tube(
            &[knee, knee.lerp(ankle, 0.5), ankle],
            |t| 0.0105 + 0.004 * (1.0 - t * 6.0).max(0.0),
            SIDES,
            [End::Round, End::Round],
        );
        mesh.bind_since(mark, |point| limb_skin(point, side, false));

        let mark = mesh.mark();
        for (angle, length) in [(-0.45, 0.1), (0.0, 0.115), (0.45, 0.1), (PI, 0.045)] {
            let ahead = Quat::from_rotation_y(angle * side.sign()) * Vec3::Z;
            let tip = ankle.with_y(0.005) + ahead * length;
            let middle = ankle.with_y(0.012) + ahead * length * 0.5;
            mesh.tube(
                &[ankle - Vec3::Y * 0.015, middle, tip],
                |t| 0.0065 - 0.0035 * t,
                6,
                [End::Open, End::Round],
            );
        }
        mesh.bind_since(mark, |_| Skin::one(Bone::Foot(side).joint()));

        let elbow = Bone::Forearm(side).rest().translation;
        let wrist = Bone::Hand(side).rest().translation;
        let mark = mesh.mark();
        mesh.tube(
            &[elbow.lerp(wrist, 0.5), wrist],
            |t| 0.018 - 0.004 * t,
            SIDES,
            [End::Open, End::Round],
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
            Vec2::new(BEAK_WIDTH * 0.5, -0.02),
            Vec2::new(BEAK_WIDTH * 0.5, 0.0),
            Vec2::new(BEAK_WIDTH * 0.42, BEAK_LENGTH * 0.3),
            Vec2::new(BEAK_WIDTH * 0.27, BEAK_LENGTH * 0.62),
            Vec2::new(BEAK_WIDTH * 0.1, BEAK_LENGTH * 0.9),
            Vec2::new(0.0, BEAK_LENGTH),
        ],
        12,
    );
    let beak = Transform::from_translation(HEAD + BEAK)
        .with_rotation(Quat::from_rotation_x(FRAC_PI_2 + BEAK_DROOP))
        .with_scale(Vec3::new(0.7, 1.0, 1.0));
    mesh.transform_since(mark, &beak);
    mesh.bind_since(mark, |_| Skin::one(Bone::Head.joint()));
    mesh.build()
}

/// A slender hand in its bone's frame: the palm along +Y from the wrist
/// and three long fingers curling towards +Z.
fn hand(mesh: &mut Triangles) {
    mesh.ellipsoid(
        Vec3::new(0.016, 0.024, 0.01),
        &Transform::from_xyz(0.0, 0.018, 0.0),
        8,
        10,
    );
    for (index, length) in [0.045, 0.052, 0.045].into_iter().enumerate() {
        let x = (index as f32 - 1.0) * 0.009;
        let base = Vec3::new(x, 0.036, 0.003);
        let spine: Vec<Vec3> = (0..5)
            .map(|step| {
                let t = step as f32 / 4.0;
                let bend = t * 1.3;
                base + Vec3::new(
                    x * t * 0.5,
                    bend.sin() * length / 1.3,
                    (1.0 - bend.cos()) * length / 1.3,
                )
            })
            .collect();
        mesh.tube(&spine, |t| 0.0055 - 0.002 * t, 6, [End::Round, End::Round]);
    }
    mesh.tube(
        &[
            Vec3::new(0.012, 0.016, 0.005),
            Vec3::new(0.022, 0.034, 0.018),
        ],
        |t| 0.005 - 0.0015 * t,
        6,
        [End::Round, End::Round],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eye_sits_on_the_head() {
        let t = (EYE.z - HEAD_BACK.z) / (BEAK.z - HEAD_BACK.z);
        let surface = head_radius(t) * HEAD_SCALE.x;
        assert!(EYE.x < surface && EYE.x + EYE_RADIUS > surface);
    }

    #[test]
    fn hull_reaches_the_neck() {
        let top = hull_place().transform_point(Vec3::Y * HULL_RADII.y);
        assert!(top.distance(NECK) < 0.12);
    }

    #[test]
    fn leg_reaches_the_ground() {
        let ankle = Bone::Foot(Side::Right).rest().translation;
        assert!(ankle.y < 0.05);
        let knee = Bone::Shin(Side::Right).rest().translation;
        assert!(knee.y > 0.2 && knee.y < 0.35);
    }

    #[test]
    fn neck_binds_the_head_end_to_the_head() {
        let top = neck_skin(NECK_SPINE[4]);
        assert_eq!(top.joints[1], Bone::Head.joint());
        assert!(top.weights[1] > 0.5);
    }
}
