use std::f32::consts::TAU;

use bevy::prelude::*;

use super::Wrinkles;
use super::sheet::Pressed;
use crate::downwash::Downwash;

/// Seconds the air on a sheet takes to catch up with the air around it.
const LAG: f32 = 0.15;
/// Edge lift per (meter per second) squared of air running along a sheet.
const LIFT: f32 = 0.012;
const MAX_LIFT: f32 = 0.025;
/// A sheet lifts at most this share of its short side.
const MAX_LIFT_SHARE: f32 = 0.12;
/// How far into the sheet from its upwind edge the lift reaches, and at
/// most this share of the sheet.
const LIFT_REACH: f32 = 0.05;
const LIFT_REACH_SHARE: f32 = 0.25;
/// Share of the lift the downwind edge gets as it flaps.
const TRAILING: f32 = 0.3;
/// Length of the ripples that run down the sheet.
const RIPPLE: f32 = 0.06;
/// Flaps per second in still air, and more per meter per second of air.
const FLAPS: f32 = 4.0;
const FLAPS_PER_SPEED: f32 = 3.0;
/// How much air pressing a lying sheet down smooths out its wrinkles.
const FLATTEN: f32 = 2.0;
/// How far air through a hanging sheet bows it, per (meter per second)
/// squared.
const PUSH: f32 = 0.01;
/// Below this lift and push a sheet lies as it is.
const STILL: f32 = 0.0001;

pub(super) struct FlutterPlugin;

impl Plugin for FlutterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Flutter::blow);
    }
}

/// Lets a loose sheet with [`Wrinkles`] lift and flap in the [`Downwash`].
/// A sheet that is [`Pressed`] stays as it is.
#[derive(Component, Debug, Default)]
pub struct Flutter {
    /// The air on the sheet, in the space of its mesh.
    air: Vec3,
    /// Seconds of flapping, to move the ripples on.
    time: f32,
    shown: bool,
}

impl Flutter {
    fn blow(
        time: Res<Time>,
        downwash: Res<Downwash>,
        mut sheets: Query<
            (Entity, &mut Flutter, &Wrinkles, &GlobalTransform, &Mesh3d),
            Without<Pressed>,
        >,
        parents: Query<&ChildOf>,
        sheltered: Query<(), With<Sheltered>>,
        mut meshes: ResMut<Assets<Mesh>>,
    ) {
        let dt = time.delta_secs();
        let follow = 1.0 - (-dt / LAG).exp();
        for (sheet, mut flutter, wrinkles, at, mesh) in &mut sheets {
            let half = wrinkles.size() / 2.0;
            let corners = [
                Vec2::ZERO,
                half,
                -half,
                half.with_y(-half.y),
                half.with_x(-half.x),
            ];
            let world = corners
                .iter()
                .map(|corner| downwash.at(at.transform_point(corner.extend(0.0))))
                .sum::<Vec3>()
                / corners.len() as f32;
            let calm = sheltered.contains(sheet)
                || parents
                    .iter_ancestors(sheet)
                    .any(|parent| sheltered.contains(parent));
            let air = if calm {
                Vec3::ZERO
            } else {
                at.rotation().inverse() * world
            };
            if air.length() < STILL && flutter.air.length() < STILL && !flutter.shown {
                continue;
            }
            let eased = flutter.air.lerp(air, follow);
            flutter.air = eased;
            flutter.time += dt;
            let lying = (at.rotation() * Vec3::Z).y > 0.7;
            let shape = Gust::new(&flutter, wrinkles.size(), lying);
            let still = shape.lift < STILL && shape.push.abs() < STILL && shape.flatten > 0.99;
            if still && !flutter.shown {
                continue;
            }
            flutter.shown = !still;
            if let Some(mut mesh) = meshes.get_mut(mesh) {
                *mesh = if still {
                    wrinkles.mesh()
                } else {
                    wrinkles.mesh_with(|p, height, freedom| shape.height(p, height, freedom))
                };
            }
        }
    }
}

/// Keeps the [`Flutter`] sheets of this entity and its children out of the
/// air, like a sheet that is still in a printer.
#[derive(Component, Debug, Default)]
pub struct Sheltered;

/// The shape the air gives a sheet at one moment.
struct Gust {
    size: Vec2,
    /// The way the air runs along the sheet, in the sheet's space with y
    /// down the page.
    along: Vec2,
    lift: f32,
    reach: f32,
    /// Waves per meter and the phase of the ripples.
    wave: f32,
    phase: f32,
    /// Share of the wrinkles left under air pressing the sheet down.
    flatten: f32,
    push: f32,
}

impl Gust {
    fn new(flutter: &Flutter, size: Vec2, lying: bool) -> Self {
        let plane = Vec2::new(flutter.air.x, -flutter.air.y);
        let speed = plane.length();
        let normal = flutter.air.z;
        let (flatten, push) = if lying {
            (1.0 / (1.0 + FLATTEN * (-normal).max(0.0)), 0.0)
        } else {
            (1.0, PUSH * normal * normal.abs())
        };
        let flaps = FLAPS + FLAPS_PER_SPEED * speed;
        Self {
            size,
            along: plane.normalize_or_zero(),
            lift: (LIFT * speed * speed)
                .min(MAX_LIFT)
                .min(MAX_LIFT_SHARE * size.min_element()),
            reach: LIFT_REACH.min(LIFT_REACH_SHARE * size.min_element()),
            wave: TAU / RIPPLE,
            phase: TAU * flaps * flutter.time,
            flatten,
            push,
        }
    }

    /// Height of the sheet at `p`, where its wrinkles give `height` and
    /// `freedom` says how far it may rise.
    fn height(&self, p: Vec2, height: f32, freedom: f32) -> f32 {
        let from_middle = p - self.size / 2.0;
        let span = (self.size * self.along.abs()).element_sum();
        let downwind = from_middle.dot(self.along) + span / 2.0;
        let across = from_middle.dot(self.along.perp());
        let edges =
            (-downwind / self.reach).exp() + TRAILING * (-(span - downwind) / self.reach).exp();
        let ripple = 0.75 + 0.25 * (self.wave * downwind - self.phase).sin();
        let ragged = 1.0 + 0.3 * (self.wave * 0.7 * across + self.phase * 0.37).sin();
        let free = freedom * freedom * (3.0 - 2.0 * freedom);
        height * self.flatten + free * (self.lift * edges * ripple * ragged + self.push)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn air_along_a_sheet_lifts_its_upwind_edge_but_not_a_held_edge() {
        let size = Vec2::new(0.2, 0.3);
        let flutter = Flutter {
            air: Vec3::new(-1.5, 0.0, 0.0),
            ..default()
        };
        let gust = Gust::new(&flutter, size, true);
        let middle = size.y / 2.0;
        let upwind = gust.height(Vec2::new(size.x, middle), 0.0, 1.0);
        let downwind = gust.height(Vec2::new(0.0, middle), 0.0, 1.0);
        assert!(
            upwind > 0.005 && upwind > 2.0 * downwind,
            "{upwind} {downwind}"
        );
        assert_eq!(gust.height(Vec2::new(size.x, middle), 0.0, 0.0), 0.0);
        let pressed = Flutter {
            air: Vec3::NEG_Z,
            ..default()
        };
        assert!(Gust::new(&pressed, size, true).height(size / 2.0, 0.002, 1.0) < 0.001);
    }
}
