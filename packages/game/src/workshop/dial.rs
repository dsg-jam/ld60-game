use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use super::clock::Clock;

const BOARD: Vec3 = Vec3::new(0.3, 0.02, 0.3);
const FACE: Vec3 = Vec3::new(0.26, 0.004, 0.26);
/// Marks from rise to set, the last one red.
const MARKS: usize = 7;
const MARK: Vec3 = Vec3::new(0.005, 0.002, 0.018);
const SET_MARK: Vec3 = Vec3::new(0.014, 0.002, 0.03);
/// The marks sit this far from the pivot.
const MARK_RADIUS: f32 = 0.105;
const HORIZON: Vec3 = Vec3::new(0.24, 0.002, 0.008);
/// Painted rooftops below the horizon, as x, width and height.
const ROOFS: [(f32, f32, f32); 5] = [
    (-0.07, 0.04, 0.05),
    (-0.03, 0.03, 0.08),
    (0.01, 0.05, 0.03),
    (0.055, 0.03, 0.065),
    (0.085, 0.025, 0.04),
];
const STICK: Vec3 = Vec3::new(0.006, 0.003, 0.085);
const PLANET: Vec3 = Vec3::new(0.026, 0.004, 0.026);
const RING: Vec3 = Vec3::new(0.05, 0.003, 0.006);
const RING_TILT: f32 = 0.35;
const PIN: Vec3 = Vec3::new(0.014, 0.012, 0.014);
/// Space between the marks and the hand.
const HAND_GAP: f32 = 0.002;

const WOOD: Color = Color::srgb_u8(0x6a, 0x4e, 0x36);
const CREAM: Color = Color::srgb_u8(0xd8, 0xc8, 0xa0);
const INK: Color = Color::srgb_u8(0x2a, 0x26, 0x20);
const RED: Color = Color::srgb_u8(0xd0, 0x2a, 0x1a);
const OCHRE: Color = Color::srgb_u8(0xc8, 0x9a, 0x5a);
const METAL: Color = Color::srgb_u8(0x7d, 0x86, 0x8c);

pub(crate) struct DialPlugin;

impl Plugin for DialPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Dial::on_add)
            .add_systems(Update, Dial::follow.run_if(resource_changed::<Clock>));
    }
}

/// A wooden dial that shows where the planet is between rise and set. A red
/// mark shows where it sets and the rent is due. Its back is at the origin
/// and its face looks towards +Z.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Dial;

/// The hand of a [`Dial`], with a little ringed planet on its end.
#[derive(Component, Debug)]
struct Hand;

impl Dial {
    /// Turn of the hand at `phase`, see [`Clock::phase`]. It points right
    /// at rise, up at noon and left at set, counterclockwise as seen from
    /// the front.
    fn angle(phase: f32) -> f32 {
        PI * (phase - 0.5)
    }

    fn hand_rotation(phase: f32) -> Quat {
        Quat::from_rotation_y(Self::angle(phase))
    }

    /// Built lying face up, where -Z is up on the wall.
    fn on_add(add: On<Add<Dial>>, clock: Res<Clock>, mut commands: Commands) {
        let board = Block::new(BOARD);
        let face = Block::new(FACE).on_top_of(&board);
        let paint = Block::new(HORIZON).on_top_of(&face);
        let marks = (0..MARKS).map(|index| {
            let phase = index as f32 / (MARKS - 1) as f32;
            let (size, color) = if index + 1 == MARKS {
                (SET_MARK, RED)
            } else {
                (MARK, INK)
            };
            let turn = Self::hand_rotation(phase);
            let at = paint.center.with_y(paint.max().y) + turn * Vec3::NEG_Z * MARK_RADIUS;
            (Block::new(size), turn, at, color)
        });
        let hand_at = paint.max().y + SET_MARK.y.max(MARK.y) + HAND_GAP;
        let pin = Block::new(PIN).on_top_of(&face);
        let stick = Block::new(STICK).translated(Vec3::NEG_Z * STICK.z / 2.0);
        let planet = Block::new(PLANET).translated(Vec3::NEG_Z * STICK.z);
        let ring = Block::new(RING).translated(planet.center.with_y(STICK.y));

        commands.entity(add.entity).with_children(|dial| {
            dial.spawn((
                Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                Visibility::default(),
            ))
            .with_children(|dial| {
                dial.spawn((Shape::cylinder(&board, WOOD), Finish::Wood));
                dial.spawn((Shape::cylinder(&face, CREAM), Finish::Paint));
                dial.spawn(Shape::cuboid(&paint, INK));
                for (x, width, height) in ROOFS {
                    let roof = Block::new(Vec3::new(width, HORIZON.y, height))
                        .on_top_of(&face)
                        .translated(Vec3::new(x, 0.0, (HORIZON.z + height) / 2.0));
                    dial.spawn(Shape::cuboid(&roof, INK));
                }
                for (mark, turn, at, color) in marks {
                    dial.spawn(Shape::cuboid(&mark, color))
                        .insert(Transform::from_translation(at + mark.center).with_rotation(turn));
                }
                dial.spawn((Shape::cylinder(&pin, METAL), Finish::Metal));
                dial.spawn((
                    Hand,
                    Transform::from_translation(Vec3::Y * hand_at)
                        .with_rotation(Self::hand_rotation(clock.phase())),
                    Visibility::default(),
                ))
                .with_children(|hand| {
                    hand.spawn((Shape::cuboid(&stick, INK), Finish::Metal));
                    hand.spawn(Shape::cylinder(&planet, OCHRE));
                    hand.spawn(Shape::cuboid(&ring, OCHRE)).insert(
                        ring.transform()
                            .with_rotation(Quat::from_rotation_y(RING_TILT)),
                    );
                });
            });
        });
    }

    fn follow(clock: Res<Clock>, mut hands: Query<&mut Transform, With<Hand>>) {
        for mut hand in &mut hands {
            hand.rotation = Self::hand_rotation(clock.phase());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_points_right_at_rise_up_at_noon_and_left_at_set() {
        let tip = |phase: f32| (Dial::hand_rotation(phase) * Vec3::NEG_Z).xz();
        assert!(tip(0.0).distance(Vec2::X) < 1e-5);
        assert!(tip(0.5).distance(Vec2::NEG_Y) < 1e-5);
        assert!(tip(1.0).distance(Vec2::NEG_X) < 1e-5);
    }
}
