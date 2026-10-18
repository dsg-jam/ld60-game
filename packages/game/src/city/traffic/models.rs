use std::f32::consts::FRAC_PI_2;

use bevy::{light::VolumetricLight, prelude::*, shape::ConvexPolygonError};
use rand::{Rng, RngExt};

use super::super::{Kit, NEON, Pick, glow, matte};
use crate::geometry::block::Block;
use crate::geometry::surface::{Finish, Form, SurfaceMaterial};

/// Side view of a sedan, front towards +X. Hull and windows.
const SEDAN_HULL: [Vec2; 6] = [
    Vec2::new(-2.2, 0.3),
    Vec2::new(2.15, 0.3),
    Vec2::new(2.3, 0.55),
    Vec2::new(2.0, 0.85),
    Vec2::new(-2.1, 0.92),
    Vec2::new(-2.3, 0.6),
];
const SEDAN_CABIN: [Vec2; 4] = [
    Vec2::new(-1.6, 0.85),
    Vec2::new(1.1, 0.85),
    Vec2::new(0.25, 1.38),
    Vec2::new(-1.25, 1.38),
];
/// A delivery van with the cab forward.
const VAN_HULL: [Vec2; 9] = [
    Vec2::new(-2.35, 0.4),
    Vec2::new(2.35, 0.4),
    Vec2::new(2.5, 0.6),
    Vec2::new(2.5, 1.1),
    Vec2::new(1.75, 2.05),
    Vec2::new(1.55, 2.15),
    Vec2::new(-2.35, 2.15),
    Vec2::new(-2.45, 2.05),
    Vec2::new(-2.45, 0.5),
];
/// Ends of the van's windshield on its profile.
const VAN_WINDSHIELD: [Vec2; 2] = [Vec2::new(2.5, 1.1), Vec2::new(1.75, 2.05)];
const HOVER_HULL: [Vec2; 5] = [
    Vec2::new(-2.0, 0.0),
    Vec2::new(1.3, -0.15),
    Vec2::new(2.1, 0.15),
    Vec2::new(1.4, 0.45),
    Vec2::new(-1.9, 0.35),
];
const HOVER_CANOPY: [Vec2; 4] = [
    Vec2::new(-0.7, 0.38),
    Vec2::new(0.95, 0.42),
    Vec2::new(0.3, 0.85),
    Vec2::new(-0.45, 0.8),
];

const CAR_WIDTH: f32 = 1.9;
const CABIN_WIDTH: f32 = 1.6;
const VAN_WIDTH: f32 = 2.0;
const HOVER_WIDTH: f32 = 1.5;
const WHEEL_RADIUS: f32 = 0.33;
const WHEEL_WIDTH: f32 = 0.26;
/// How far the wheels stick out of the body.
const WHEEL_OUT: f32 = 0.04;
pub(super) const WHEELBASE: f32 = 1.45;
/// Distance of a sedan's wheels from its middle.
pub(super) const TRACK: f32 = CAR_WIDTH / 2.0 - WHEEL_WIDTH / 2.0 + WHEEL_OUT;
const POD_RADIUS: f32 = 0.22;
const POD_LENGTH: f32 = 1.6;
const STRIP: f32 = 0.06;
const HEADLIGHT_INTENSITY: f32 = 150_000.0;
const VAN_CHANCE: f64 = 0.3;
const UNDERGLOW_CHANCE: f64 = 0.4;

const PAINTS: [Color; 6] = [
    Color::srgb_u8(0x1f, 0x22, 0x2a),
    Color::srgb_u8(0x5a, 0x1e, 0x2a),
    Color::srgb_u8(0x2a, 0x4a, 0x5a),
    Color::srgb_u8(0xb0, 0xa8, 0x98),
    Color::srgb_u8(0x3a, 0x3a, 0x3e),
    Color::srgb_u8(0xc8, 0x8a, 0x2a),
];
const UNDERGLOW: [Color; 3] = [NEON[1], NEON[0], NEON[2]];
const GLASS: Color = Color::srgb_u8(0x0c, 0x10, 0x16);
const HEADLIGHT: Color = Color::srgb_u8(0xff, 0xf4, 0xe0);
const TAILLIGHT: Color = Color::srgb_u8(0xff, 0x18, 0x10);
const TIRE: Color = Color::srgb_u8(0x10, 0x10, 0x12);

/// Meshes and materials shared by all vehicles.
pub(super) struct Fleet {
    sedan_hull: Handle<Mesh>,
    sedan_cabin: Handle<Mesh>,
    van_hull: Handle<Mesh>,
    van_windshield: Handle<Mesh>,
    hover_hull: Handle<Mesh>,
    hover_canopy: Handle<Mesh>,
    wheel: Handle<Mesh>,
    pod: Handle<Mesh>,
    paints: Vec<Handle<SurfaceMaterial>>,
    glows: Vec<Handle<StandardMaterial>>,
    glass: Handle<StandardMaterial>,
    head: Handle<StandardMaterial>,
    tail: Handle<StandardMaterial>,
    tire: Handle<SurfaceMaterial>,
}

impl Fleet {
    pub(super) fn new(
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        surfaces: &mut Assets<SurfaceMaterial>,
    ) -> Result<Self, ConvexPolygonError> {
        let mut extrude = |profile: &[Vec2], width: f32| -> Result<_, ConvexPolygonError> {
            let polygon = ConvexPolygon::new(profile.iter().copied())?;
            Ok(meshes.add(Extrusion::new(polygon, width)))
        };
        Ok(Self {
            sedan_hull: extrude(&SEDAN_HULL, CAR_WIDTH)?,
            sedan_cabin: extrude(&SEDAN_CABIN, CABIN_WIDTH)?,
            van_hull: extrude(&VAN_HULL, VAN_WIDTH)?,
            van_windshield: extrude(&Self::pane(VAN_WINDSHIELD), VAN_WIDTH - 0.3)?,
            hover_hull: extrude(&HOVER_HULL, HOVER_WIDTH)?,
            hover_canopy: extrude(&HOVER_CANOPY, HOVER_WIDTH * 0.7)?,
            wheel: meshes.add(Cylinder::new(WHEEL_RADIUS, WHEEL_WIDTH)),
            pod: meshes.add(Capsule3d::new(POD_RADIUS, POD_LENGTH - 2.0 * POD_RADIUS)),
            paints: PAINTS
                .iter()
                .map(|&color| {
                    surfaces.add(Finish::Paint.over(
                        StandardMaterial {
                            base_color: color,
                            perceptual_roughness: 0.3,
                            metallic: 0.6,
                            ..default()
                        },
                        Form::Freeform,
                        0.0,
                    ))
                })
                .collect(),
            glows: UNDERGLOW
                .iter()
                .map(|&color| materials.add(glow(color, 20.0)))
                .collect(),
            glass: materials.add(matte(GLASS, 0.1)),
            head: materials.add(glow(HEADLIGHT, 50.0)),
            tail: materials.add(glow(TAILLIGHT, 30.0)),
            tire: surfaces.add(Finish::Plastic.over(matte(TIRE, 0.9), Form::Freeform, 0.0)),
        })
    }

    pub(super) fn car(&self, parts: &mut ChildSpawnerCommands, kit: &Kit, rng: &mut impl Rng) {
        let paint = self.paints.pick(rng).clone();
        let (width, front, rear) = if rng.random_bool(VAN_CHANCE) {
            self.van_details(parts, kit, &paint, rng);
            (VAN_WIDTH, 2.5, -2.45)
        } else {
            parts.spawn(Self::part(&self.sedan_hull, &paint));
            parts.spawn(Self::part(&self.sedan_cabin, &self.glass));
            (CAR_WIDTH, 2.3, -2.3)
        };
        for x in [-WHEELBASE, WHEELBASE] {
            for side in [-1.0, 1.0] {
                let z = side * (width / 2.0 - WHEEL_WIDTH / 2.0 + WHEEL_OUT);
                parts.spawn((
                    Mesh3d(self.wheel.clone()),
                    MeshMaterial3d(self.tire.clone()),
                    Transform::from_xyz(x, WHEEL_RADIUS, z)
                        .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                ));
            }
        }
        self.lights(parts, kit, front, rear, 0.6, width);
        if rng.random_bool(UNDERGLOW_CHANCE) {
            let strip = Block::between(
                Vec3::new(rear * 0.8, 0.22, -width * 0.42),
                Vec3::new(front * 0.8, 0.22 + STRIP, width * 0.42),
            );
            parts.spawn(kit.cuboid(&strip, self.glows.pick(rng).clone()));
        }
    }

    /// Body, glass, a stripe, door seams and wheel arches of a van.
    fn van_details(
        &self,
        parts: &mut ChildSpawnerCommands,
        kit: &Kit,
        paint: &Handle<SurfaceMaterial>,
        rng: &mut impl Rng,
    ) {
        parts.spawn(Self::part(&self.van_hull, paint));
        parts.spawn(Self::part(&self.van_windshield, &self.glass));
        // Things on the sides stick out a little, so they don't fight with
        // the body.
        let side = |from: Vec2, to: Vec2, out: f32| {
            Block::between(
                from.extend(-VAN_WIDTH / 2.0 - out),
                to.extend(VAN_WIDTH / 2.0 + out),
            )
        };
        let windows = side(Vec2::new(1.15, 1.3), Vec2::new(1.85, 1.85), 0.01);
        let stripe = side(Vec2::new(-2.4, 0.95), Vec2::new(2.45, 1.05), 0.012);
        parts.spawn(kit.cuboid(&windows, self.glass.clone()));
        parts.spawn(kit.cuboid(&stripe, self.paints.pick(rng).clone()));
        for x in [0.95, -0.3] {
            let seam = side(Vec2::new(x, 0.5), Vec2::new(x + 0.03, 2.0), 0.014);
            parts.spawn(kit.cuboid(&seam, self.tire.clone()));
        }
        for x in [-WHEELBASE, WHEELBASE] {
            let arch = side(Vec2::new(x - 0.45, 0.4), Vec2::new(x + 0.45, 0.8), 0.006);
            parts.spawn(kit.cuboid(&arch, self.tire.clone()));
        }
    }

    /// A thin pane of glass along the edge from `a` to `b` of a profile.
    fn pane([a, b]: [Vec2; 2]) -> [Vec2; 4] {
        let along = (b - a).normalize();
        let out = -along.perp() * 0.02;
        let (a, b) = (a + along * 0.08, b - along * 0.08);
        [a - out, a + out, b + out, b - out]
    }

    pub(super) fn hover(&self, parts: &mut ChildSpawnerCommands, kit: &Kit, rng: &mut impl Rng) {
        let paint = self.paints.pick(rng).clone();
        let glow = self.glows.pick(rng).clone();
        parts.spawn(Self::part(&self.hover_hull, &paint));
        parts.spawn(Self::part(&self.hover_canopy, &self.glass));
        for side in [-1.0, 1.0] {
            let center = Vec3::new(-0.6, 0.15, side * (HOVER_WIDTH / 2.0 + POD_RADIUS * 0.8));
            parts.spawn((
                Mesh3d(self.pod.clone()),
                MeshMaterial3d(paint.clone()),
                Transform::from_translation(center).with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
            ));
            // Exhaust glow at the back of the pod.
            let exhaust = Block::between(
                center - Vec3::new(POD_LENGTH / 2.0 + 0.02, POD_RADIUS * 0.6, POD_RADIUS * 0.6),
                center
                    - Vec3::new(
                        POD_LENGTH / 2.0 - 0.15,
                        -POD_RADIUS * 0.6,
                        -POD_RADIUS * 0.6,
                    ),
            );
            parts.spawn(kit.cuboid(&exhaust, glow.clone()));
        }
        let strip = Block::between(
            Vec3::new(-1.6, 0.2, -HOVER_WIDTH / 2.0 - 0.01),
            Vec3::new(1.2, 0.2 + STRIP, HOVER_WIDTH / 2.0 + 0.01),
        );
        parts.spawn(kit.cuboid(&strip, glow));
        self.lights(parts, kit, 2.1, -2.0, 0.15, HOVER_WIDTH);
    }

    /// Head lights, a tail light bar, and a real light shining ahead.
    fn lights(
        &self,
        parts: &mut ChildSpawnerCommands,
        kit: &Kit,
        front: f32,
        rear: f32,
        height: f32,
        width: f32,
    ) {
        for side in [-1.0, 1.0] {
            let head = Block::between(
                Vec3::new(front - 0.12, height - 0.05, side * width * 0.42),
                Vec3::new(front + 0.02, height + 0.05, side * width * 0.22),
            );
            parts.spawn(kit.cuboid(&head, self.head.clone()));
        }
        let tail = Block::between(
            Vec3::new(rear - 0.02, height + 0.05, -width * 0.45),
            Vec3::new(rear + 0.1, height + 0.12, width * 0.45),
        );
        parts.spawn(kit.cuboid(&tail, self.tail.clone()));
        parts.spawn((
            VolumetricLight,
            SpotLight {
                color: HEADLIGHT,
                intensity: HEADLIGHT_INTENSITY,
                range: 25.0,
                outer_angle: 0.5,
                inner_angle: 0.3,
                ..default()
            },
            Transform::from_xyz(front + 0.2, height, 0.0)
                .looking_to(Vec3::new(1.0, -0.15, 0.0), Vec3::Y),
        ));
    }

    fn part<M: Material>(mesh: &Handle<Mesh>, material: &Handle<M>) -> impl Bundle {
        (Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()))
    }
}
