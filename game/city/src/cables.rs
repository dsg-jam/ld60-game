use std::ops::Range;

use bevy::{
    prelude::*,
    shape::{Aabb3d, BoundingVolume},
    transform::TransformSystems,
};
use ld60_game_parts::bulb::Bulb;
use ld60_geometry::bar::Bar;
use ld60_geometry::block::Block;
use ld60_geometry::surface::{Finish, SurfaceMaterial};
use rand::{Rng, RngExt};

use super::buildings::{FacadeMaterial, Frontage, Skyline};
use super::signs::SignBounds;
use super::{FACADES, Kit, Pick, Segment, rng};
use ld60_game_core::layout::shop::FRONT;

const SEED: u64 = 8;
/// Z of the middle of the street, between the two rows of buildings.
const STREET_MIDDLE: f32 = (FRONT + FACADES) / 2.0;
const CABLES: usize = 14;
const SPAN: f32 = 30.0;
/// Below the lowest flying cars.
const HEIGHT: Range<f32> = 4.8..7.0;
const THICKNESS: f32 = 0.025;
const SAG: Range<f32> = 0.3..1.0;
const SEGMENTS: usize = 8;
/// Cables keep this far from signs and from the rest of the city, and
/// are fixed this far from windows and from the ends of a building.
const CLEARANCE: f32 = 0.2;
const PART_CLEARANCE: f32 = 0.05;
const WINDOW_CLEARANCE: f32 = 0.25;
const END_CLEARANCE: f32 = 0.4;
/// Along this much of a cable at each end, it only meets its wall.
const FIXED: f32 = 0.3;
/// Parts of the city this far beyond the cables' reach can't be in the way.
const BEYOND: f32 = 6.0;
const TRIES: usize = 30;
const STRING_LIGHT_CHANCE: f64 = 0.5;
/// Bulb strings are at least this many cables apart.
const STRING_SPACING: usize = 3;
const BULB_SPACING: f32 = 0.9;
const BULB_RADIUS: f32 = 0.035;
/// Bright enough to light up the rain from across the street.
const BULB_GLOW: f32 = 60.0;
/// The socket each bulb hangs from, under the cable.
const SOCKET: Vec3 = Vec3::new(0.02, 0.03, 0.02);

const WIRE: Color = Color::srgb_u8(0x12, 0x12, 0x14);
const SOCKET_COLOR: Color = Color::srgb_u8(0x1a, 0x1a, 0x1c);
const BULBS: [Color; 3] = [
    Color::srgb_u8(0xff, 0xd0, 0x80),
    Color::srgb_u8(0xff, 0x70, 0xc0),
    Color::srgb_u8(0x70, 0xd0, 0xff),
];

pub(super) struct CablesPlugin;

impl Plugin for CablesPlugin {
    fn build(&self, app: &mut App) {
        // After the transforms are propagated, to find what is in the way.
        app.add_systems(
            PostStartup,
            Cable::spawn_all.after(TransformSystems::Propagate),
        );
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
    /// Spread evenly along the street, fixed to the walls between the
    /// windows, clear of everything else, and bulb strings apart from each
    /// other.
    fn spawn_all(
        mut commands: Commands,
        kit: Res<Kit>,
        signs: Res<SignBounds>,
        skyline: Res<Skyline>,
        facades: Res<Assets<FacadeMaterial>>,
        parts: Query<(&Mesh3d, &GlobalTransform)>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let mut rng = rng(SEED);
        let wire = surfaces.add(Finish::Plastic.scaled(WIRE));
        let socket = surfaces.add(Finish::Plastic.scaled(SOCKET_COLOR));
        let slot = 2.0 * SPAN / CABLES as f32;
        let reach = Aabb3d::new(
            Vec3::ZERO,
            Vec3::new(SPAN + BEYOND, HEIGHT.end, BEYOND * 4.0),
        );
        let in_way: Vec<Aabb3d> = parts
            .iter()
            .filter(|(mesh, _)| mesh.0 == kit.cube)
            .map(|(_, at)| Self::bounds(at))
            .filter(|bounds| bounds.intersects(&reach))
            .collect();
        let signs: Vec<Aabb3d> = signs
            .0
            .iter()
            .map(|sign| sign.grow(Vec3::splat(CLEARANCE)))
            .collect();
        let mut last_string = None;
        let mut root = commands.spawn((
            Name::new("cables"),
            Transform::default(),
            Visibility::default(),
        ));
        root.with_children(|parent| {
            for index in 0..CABLES {
                let Some(cable) = (0..TRIES)
                    .filter_map(|_| Self::random(index, slot, &skyline, &facades, &mut rng))
                    .find(|cable| cable.clear_of(&signs) && cable.clear_of(&in_way))
                else {
                    continue;
                };
                for Segment { from, to } in cable.segments() {
                    parent.spawn(kit.bar(&Bar::square(from, to, THICKNESS), wire.clone()));
                }
                let apart = last_string.is_none_or(|last| index - last >= STRING_SPACING);
                if apart && rng.random_bool(STRING_LIGHT_CHANCE) {
                    last_string = Some(index);
                    let color = *BULBS.pick(&mut rng);
                    let count = (cable.near.distance(cable.far) / BULB_SPACING) as usize;
                    for index in 1..count {
                        let at = cable.point(index as f32 / count as f32);
                        let holder = Block::new(SOCKET).standing_at(at - Vec3::Y * SOCKET.y);
                        parent.spawn(kit.cuboid(&holder, socket.clone()));
                        parent.spawn((
                            Bulb {
                                radius: BULB_RADIUS,
                                color,
                                glow: BULB_GLOW,
                                on: true,
                            },
                            Transform::from_translation(
                                holder.bottom() - Vec3::Y * BULB_RADIUS * 0.9,
                            ),
                        ));
                    }
                }
            }
        });
    }

    /// A cable in the `index`th of the slots `slot` wide along the street,
    /// if both its ends land on a wall.
    fn random(
        index: usize,
        slot: f32,
        skyline: &Skyline,
        facades: &Assets<FacadeMaterial>,
        rng: &mut impl Rng,
    ) -> Option<Self> {
        let x = -SPAN + (index as f32 + rng.random_range(0.2..0.8)) * slot;
        let near = Self::fixed_to(&skyline.near, x, rng.random_range(HEIGHT), facades)?;
        let far_x = x + rng.random_range(-4.0..4.0);
        let far = Self::fixed_to(&skyline.front, far_x, rng.random_range(HEIGHT), facades)?;
        Some(Self {
            near,
            far,
            sag: rng.random_range(SAG),
        })
    }

    /// Where on the street side of one of `fronts` a cable at `x` and `y`
    /// is fixed, if it lands on plain wall there.
    fn fixed_to(
        fronts: &[Frontage],
        x: f32,
        y: f32,
        facades: &Assets<FacadeMaterial>,
    ) -> Option<Vec3> {
        let front = fronts.iter().find(|front| {
            let block = &front.block;
            block.min().x + END_CLEARANCE < x
                && x < block.max().x - END_CLEARANCE
                && y < block.max().y - END_CLEARANCE
        })?;
        let block = &front.block;
        // The street side is the one nearer the middle of the street.
        let z = if block.center.z > STREET_MIDDLE {
            block.min().z
        } else {
            block.max().z
        };
        let at = Vec3::new(x, y, z);
        let facade = &facades.get(&front.facade)?.extension;
        (!facade.has_window_near(at, WINDOW_CLEARANCE)).then_some(at)
    }

    /// The box around a unit cube placed at `at`.
    fn bounds(at: &GlobalTransform) -> Aabb3d {
        let affine = at.affine();
        let half = affine.matrix3.abs() * Vec3A::splat(0.5);
        Aabb3d {
            min: affine.translation - half,
            max: affine.translation + half,
        }
    }

    fn point(&self, t: f32) -> Vec3 {
        self.near.lerp(self.far, t) - Vec3::Y * self.sag * 4.0 * t * (1.0 - t)
    }

    /// Whether it keeps clear of all `bounds`, but for where its ends are
    /// fixed to the walls.
    fn clear_of(&self, bounds: &[Aabb3d]) -> bool {
        let off = FIXED / self.near.distance(self.far);
        let free = |t: f32| off + (1.0 - 2.0 * off) * t;
        let pieces = (0..SEGMENTS).map(|segment| Segment {
            from: self.point(free(segment as f32 / SEGMENTS as f32)),
            to: self.point(free((segment + 1) as f32 / SEGMENTS as f32)),
        });
        pieces.into_iter().all(|segment| {
            bounds
                .iter()
                .all(|bounds| segment.misses(&bounds.grow(Vec3::splat(PART_CLEARANCE))))
        })
    }

    /// The straight pieces the sagging cable is made of.
    fn segments(&self) -> impl Iterator<Item = Segment> {
        (0..SEGMENTS).map(|segment| Segment {
            from: self.point(segment as f32 / SEGMENTS as f32),
            to: self.point((segment + 1) as f32 / SEGMENTS as f32),
        })
    }
}
