use std::ops::Range;

use bevy::{
    prelude::*,
    shape::{Aabb3d, RayCast3d},
};
use rand::{Rng, RngExt};

use super::signs::{Sign, SignBounds};
use super::{FACADES, Kit, glow, rng};
use crate::geometry::surface::{Finish, SurfaceMaterial};
use crate::workshop::shop;

const SEED: u64 = 8;
const CABLES: usize = 14;
const SPAN: f32 = 30.0;
/// Below the lowest flight lane.
const HEIGHT: Range<f32> = 4.8..7.0;
const THICKNESS: f32 = 0.025;
const SAG: Range<f32> = 0.3..1.0;
const SEGMENTS: usize = 8;
/// Cables keep this far from signs.
const CLEARANCE: f32 = 0.2;
const TRIES: usize = 20;
const STRING_LIGHT_CHANCE: f64 = 0.5;
/// Bulb strings are at least this many cables apart.
const STRING_SPACING: usize = 3;
const BULB_SPACING: f32 = 0.9;
const BULB: f32 = 0.07;
const BULB_GLOW: f32 = 15.0;

const WIRE: Color = Color::srgb_u8(0x12, 0x12, 0x14);
const BULBS: [Color; 3] = [
    Color::srgb_u8(0xff, 0xd0, 0x80),
    Color::srgb_u8(0xff, 0x70, 0xc0),
    Color::srgb_u8(0x70, 0xd0, 0xff),
];

pub(super) struct CablesPlugin;

impl Plugin for CablesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Cable::spawn_all.after(Sign::spawn_all));
    }
}

/// A power cable across the street near the shop, sagging between the
/// buildings.
struct Cable {
    near: Vec3,
    far: Vec3,
    sag: f32,
}

impl Cable {
    /// Spread evenly along the street, clear of the signs, and bulb strings
    /// apart from each other.
    fn spawn_all(
        mut commands: Commands,
        kit: Res<Kit>,
        signs: Res<SignBounds>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let mut rng = rng(SEED);
        let wire = surfaces.add(Finish::Plastic.scaled(WIRE));
        let bulbs: Vec<_> = BULBS
            .iter()
            .map(|&color| materials.add(glow(color, BULB_GLOW)))
            .collect();
        let slot = 2.0 * SPAN / CABLES as f32;
        let mut last_string = None;
        let mut root = commands.spawn((
            Name::new("cables"),
            Transform::default(),
            Visibility::default(),
        ));
        root.with_children(|parent| {
            for index in 0..CABLES {
                let Some(cable) = (0..TRIES)
                    .map(|_| Self::random(index, slot, &mut rng))
                    .find(|cable| cable.clear_of(&signs.0))
                else {
                    continue;
                };
                for segment in 0..SEGMENTS {
                    let a = cable.point(segment as f32 / SEGMENTS as f32);
                    let b = cable.point((segment + 1) as f32 / SEGMENTS as f32);
                    parent.spawn((
                        Mesh3d(kit.cube.clone()),
                        MeshMaterial3d(wire.clone()),
                        Transform::from_translation((a + b) / 2.0)
                            .looking_to(b - a, Vec3::Y)
                            .with_scale(Vec3::new(THICKNESS, THICKNESS, a.distance(b))),
                    ));
                }
                let apart = last_string.is_none_or(|last| index - last >= STRING_SPACING);
                if apart && rng.random_bool(STRING_LIGHT_CHANCE) {
                    last_string = Some(index);
                    let bulb = bulbs[rng.random_range(0..bulbs.len())].clone();
                    let count = (cable.near.distance(cable.far) / BULB_SPACING) as usize;
                    for index in 1..count {
                        let at = cable.point(index as f32 / count as f32) - Vec3::Y * BULB;
                        parent.spawn((
                            Mesh3d(kit.cube.clone()),
                            MeshMaterial3d(bulb.clone()),
                            Transform::from_translation(at).with_scale(Vec3::splat(BULB)),
                        ));
                    }
                }
            }
        });
    }

    fn random(index: usize, slot: f32, rng: &mut impl Rng) -> Self {
        let x = -SPAN + (index as f32 + rng.random_range(0.2..0.8)) * slot;
        Self {
            near: Vec3::new(x, rng.random_range(HEIGHT), shop::FRONT),
            far: Vec3::new(
                x + rng.random_range(-4.0..4.0),
                rng.random_range(HEIGHT),
                FACADES,
            ),
            sag: rng.random_range(SAG),
        }
    }

    fn point(&self, t: f32) -> Vec3 {
        self.near.lerp(self.far, t) - Vec3::Y * self.sag * 4.0 * t * (1.0 - t)
    }

    fn clear_of(&self, signs: &[Aabb3d]) -> bool {
        (0..SEGMENTS).all(|segment| {
            let a = self.point(segment as f32 / SEGMENTS as f32);
            let b = self.point((segment + 1) as f32 / SEGMENTS as f32);
            let Ok(direction) = Dir3::new(b - a) else {
                return true;
            };
            let ray = RayCast3d::new(a, direction, a.distance(b));
            signs.iter().all(|sign| {
                let bounds = Aabb3d {
                    min: sign.min - CLEARANCE,
                    max: sign.max + CLEARANCE,
                };
                ray.aabb_intersection_at(&bounds).is_none()
            })
        })
    }
}
