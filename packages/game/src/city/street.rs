use bevy::{light::VolumetricLight, prelude::*};

use super::wet::{Wet, WetMaterial};
use super::{CURB_HEIGHT, FACADES, FAR_CURB, HALF_LENGTH, Kit, NEAR_CURB};
use super::{glow, matte};
use crate::geometry::block::Block;
use crate::geometry::surface::{Finish, SurfaceMaterial};
use crate::workshop::shop;

/// Thickness of the ground slabs below their top.
const SLAB_DEPTH: f32 = 0.4;
const CURB_WIDTH: f32 = 0.25;
/// The sidewalks reach this far under the buildings, so set back walls and
/// alleys stand on the ground.
const BACKYARD: f32 = 40.0;

const LINE_WIDTH: f32 = 0.15;
const DASH_LENGTH: f32 = 3.0;
const DASH_GAP: f32 = 4.0;
/// Paint sits this far above the road to avoid fighting with it.
const PAINT_HEIGHT: f32 = 0.004;

/// Share of the ground under water, from 0 to 1.
const ROAD_PUDDLES: f32 = 1.0;
const WALK_PUDDLES: f32 = 0.1;

const LAMP_SPACING: f32 = 18.0;
const LAMP_HEIGHT: f32 = 6.5;
const LAMP_REACH: f32 = 1.8;
const LAMP_POST: f32 = 0.14;
const LAMP_PLINTH: Vec3 = Vec3::new(0.3, 0.5, 0.3);
/// The closed housing at the end of the arm, long towards the road.
const LAMP_HOUSING: Vec3 = Vec3::new(0.34, 0.14, 0.8);
/// The housing tips up towards the road, in radians.
const LAMP_TILT: f32 = 0.08;
/// The glass on the underside, set into the housing.
const LAMP_LENS: Vec2 = Vec2::new(0.26, 0.6);
const LAMP_STRUT: f32 = 0.05;
const PHOTOCELL: Vec3 = Vec3::new(0.06, 0.05, 0.06);
const LAMP_INTENSITY: f32 = 400_000.0;
const LAMP_RANGE: f32 = 18.0;
const LAMP_ANGLE: f32 = 1.1;

const ASPHALT: Color = Color::srgb_u8(0x1a, 0x1b, 0x20);
const CONCRETE: Color = Color::srgb_u8(0x4a, 0x48, 0x4c);
const CURB: Color = Color::srgb_u8(0x6a, 0x66, 0x66);
const PAINT: Color = Color::srgb_u8(0xb8, 0xb0, 0x90);
const POST: Color = Color::srgb_u8(0x2c, 0x2e, 0x33);
const HOUSING: Color = Color::srgb_u8(0x3a, 0x3c, 0x40);
const SODIUM: Color = Color::srgb_u8(0xff, 0xa8, 0x5a);

pub(super) struct StreetPlugin;

impl Plugin for StreetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_street);
    }
}

fn spawn_street(
    mut commands: Commands,
    kit: Res<Kit>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut wet: ResMut<Assets<WetMaterial>>,
    mut surfaces: ResMut<Assets<SurfaceMaterial>>,
) {
    let asphalt = wet.add(Wet::material(matte(ASPHALT, 0.9), ROAD_PUDDLES, 1.0));
    let concrete = wet.add(Wet::material(matte(CONCRETE, 0.92), WALK_PUDDLES, 2.0));
    let curb = wet.add(Wet::material(matte(CURB, 0.8), 0.0, 3.0));
    let paint = wet.add(Wet::material(matte(PAINT, 0.6), ROAD_PUDDLES, 1.0));
    let slab = |block: &Block, material: &Handle<WetMaterial>| kit.cuboid(block, material.clone());

    let ground = |top: f32, near: f32, far: f32| {
        Block::between(
            Vec3::new(-HALF_LENGTH, top - SLAB_DEPTH, near),
            Vec3::new(HALF_LENGTH, top, far),
        )
    };
    // Edge to edge, so no faces overlap.
    let road = ground(-CURB_HEIGHT, NEAR_CURB, FAR_CURB);
    let near_curb = ground(0.0, NEAR_CURB + CURB_WIDTH, NEAR_CURB);
    let near_walk = ground(0.0, shop::FRONT + BACKYARD, NEAR_CURB + CURB_WIDTH);
    let far_curb = ground(0.0, FAR_CURB, FAR_CURB - CURB_WIDTH);
    let far_walk = ground(0.0, FAR_CURB - CURB_WIDTH, FACADES - BACKYARD);

    commands
        .spawn((
            Name::new("street"),
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|street| {
            street.spawn(slab(&road, &asphalt));
            street.spawn(slab(&near_walk, &concrete));
            street.spawn(slab(&far_walk, &concrete));
            street.spawn(slab(&near_curb, &curb));
            street.spawn(slab(&far_curb, &curb));

            let middle = (NEAR_CURB + FAR_CURB) / 2.0;
            let mut x = -HALF_LENGTH;
            while x < HALF_LENGTH {
                let dash = Block::between(
                    Vec3::new(x, -CURB_HEIGHT, middle - LINE_WIDTH / 2.0),
                    Vec3::new(
                        x + DASH_LENGTH,
                        -CURB_HEIGHT + PAINT_HEIGHT,
                        middle + LINE_WIDTH / 2.0,
                    ),
                );
                street.spawn(slab(&dash, &paint));
                x += DASH_LENGTH + DASH_GAP;
            }
        });

    spawn_lamps(&mut commands, &kit, &mut materials, &mut surfaces);
}

/// Street lamps on both sides, reaching over the road.
fn spawn_lamps(
    commands: &mut Commands,
    kit: &Kit,
    materials: &mut Assets<StandardMaterial>,
    surfaces: &mut Assets<SurfaceMaterial>,
) {
    let post = surfaces.add(Finish::Paint.scaled(POST));
    let housing = surfaces.add(Finish::Metal.scaled(HOUSING));
    let lens = materials.add(glow(SODIUM, 60.0));
    let sides = [
        // Leaves the view from the shop free.
        (NEAR_CURB + CURB_WIDTH * 2.0, -1.0, LAMP_SPACING / 2.0),
        (FAR_CURB - CURB_WIDTH * 2.0, 1.0, LAMP_SPACING / 4.0),
    ];
    for (z, towards_road, offset) in sides {
        let mut x = -HALF_LENGTH + offset;
        while x < HALF_LENGTH {
            let plinth = Block::new(LAMP_PLINTH).translated(Vec3::new(x, 0.0, z));
            let pole = Block::new(Vec3::new(LAMP_POST, LAMP_HEIGHT, LAMP_POST))
                .translated(Vec3::new(x, 0.0, z));
            let arm = Block::between(
                Vec3::new(x - LAMP_POST / 4.0, LAMP_HEIGHT - LAMP_POST / 2.0, z),
                Vec3::new(
                    x + LAMP_POST / 4.0,
                    LAMP_HEIGHT,
                    z + towards_road * LAMP_REACH,
                ),
            );
            // The housing hangs from the end of the arm, tipped towards the
            // road. Its parts are placed in its own space, long along Z.
            let head = Vec3::new(
                x,
                LAMP_HEIGHT - LAMP_HOUSING.y / 2.0,
                z + towards_road * (LAMP_REACH - LAMP_HOUSING.z / 4.0),
            );
            let facing = if towards_road > 0.0 {
                Quat::IDENTITY
            } else {
                Quat::from_rotation_y(std::f32::consts::PI)
            };
            let tilt = facing * Quat::from_rotation_x(-LAMP_TILT);
            let shell = Block {
                center: Vec3::ZERO,
                size: LAMP_HOUSING,
            };
            let glass = Block {
                center: Vec3::new(0.0, -LAMP_HOUSING.y / 2.0, LAMP_HOUSING.z * 0.05),
                size: Vec3::new(LAMP_LENS.x, 0.02, LAMP_LENS.y),
            };
            let cell = Block::new(PHOTOCELL).translated(Vec3::new(
                0.0,
                LAMP_HOUSING.y / 2.0,
                -LAMP_HOUSING.z * 0.3,
            ));
            // A strut from the pole up to the arm, holding it.
            let strut_low = Vec3::new(x, LAMP_HEIGHT - 1.0, z + towards_road * LAMP_POST);
            let strut_high = Vec3::new(
                x,
                LAMP_HEIGHT - LAMP_POST / 2.0,
                z + towards_road * LAMP_REACH * 0.5,
            );
            let light_at = head + tilt * glass.center;
            commands
                .spawn((Transform::default(), Visibility::default()))
                .with_children(|parts| {
                    parts.spawn(kit.cuboid(&plinth, post.clone()));
                    parts.spawn(kit.cuboid(&pole, post.clone()));
                    parts.spawn(kit.cuboid(&arm, post.clone()));
                    parts.spawn(kit.bar(strut_low, strut_high, LAMP_STRUT, post.clone()));
                    parts
                        .spawn((
                            Transform::from_translation(head).with_rotation(tilt),
                            Visibility::default(),
                        ))
                        .with_children(|head| {
                            head.spawn(kit.cuboid(&shell, housing.clone()));
                            head.spawn(kit.cuboid(&glass, lens.clone()));
                            head.spawn(kit.cuboid(&cell, post.clone()));
                        });
                    parts.spawn((
                        VolumetricLight,
                        SpotLight {
                            color: SODIUM,
                            intensity: LAMP_INTENSITY,
                            range: LAMP_RANGE,
                            outer_angle: LAMP_ANGLE,
                            inner_angle: LAMP_ANGLE * 0.6,
                            ..default()
                        },
                        Transform::from_translation(light_at - Vec3::Y * 0.02)
                            .looking_at(light_at.with_y(0.0), Vec3::X),
                    ));
                });
            x += LAMP_SPACING;
        }
    }
}
