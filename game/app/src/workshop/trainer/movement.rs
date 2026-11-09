use std::f32::consts::TAU;

use bevy::prelude::*;
use ld60_devices::MARKS;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

/// The brass plate of the movement and how high its posts hold it.
const PLATE: Vec2 = Vec2::new(0.038, 0.006);
const POST: Vec2 = Vec2::new(0.003, 0.008);
const DIAL: Vec2 = Vec2::new(0.034, 0.0012);
/// Width, height and length of a mark. The top mark is longer.
const MARK: Vec3 = Vec3::new(0.0014, 0.0003, 0.0045);
const MARK_RADIUS: f32 = 0.028;
/// Width, height and length of the hand, and how far its tail reaches back.
const HAND: Vec3 = Vec3::new(0.0016, 0.0006, 0.029);
const TAIL: f32 = 0.006;
const CAP: Vec2 = Vec2::new(0.0025, 0.0016);
/// A small board sticks out of the plate at mark 0, with the chip on it
/// and the socket for the panel's cable at its end.
const BOARD: Vec3 = Vec3::new(0.02, 0.0016, 0.03);
const CHIP: Vec3 = Vec3::new(0.012, 0.003, 0.008);
const SOCKET: Vec3 = Vec3::new(0.014, 0.004, 0.005);
/// The hand swings to a new mark like a spring this stiff and this damped,
/// a little past it and back.
const STIFFNESS: f32 = 2500.0;
const DAMPING: f32 = 45.0;
const SUBSTEPS: usize = 4;

const BRASS: Color = Color::srgb_u8(0xb0, 0x8a, 0x48);
const ENAMEL: Color = Color::srgb_u8(0xee, 0xe8, 0xd8);
const INK: Color = Color::srgb_u8(0x1a, 0x1a, 0x1c);
const BLUED: Color = Color::srgb_u8(0x1c, 0x24, 0x48);
const CHIP_BLACK: Color = Color::srgb_u8(0x22, 0x22, 0x24);
const BOARD_GREEN: Color = Color::srgb_u8(0x2c, 0x5a, 0x3c);

/// Where the panel's cable plugs in, in the movement's space, and the way
/// it leaves.
pub(super) const CABLE_PLUG: Vec3 = Vec3::new(
    0.0,
    POST.y + PLATE.y / 2.0 + SOCKET.y / 2.0,
    -PLATE.x - BOARD.z + 0.008 - SOCKET.z,
);

/// The hand of the movement, turning about +Y. It points at `-Z` at mark 0
/// and goes clockwise seen from above. It swings over to its mark, always
/// forwards.
#[derive(Component, Debug)]
pub(super) struct Hand {
    mark: u8,
    /// Its turn now and where it swings to, in radians, and how fast it
    /// turns.
    angle: f32,
    goal: f32,
    speed: f32,
}

impl Hand {
    fn new(mark: u8) -> Self {
        let angle = Self::turn(mark);
        Self {
            mark,
            angle,
            goal: angle,
            speed: 0.0,
        }
    }

    fn turn(mark: u8) -> f32 {
        -TAU * f32::from(mark) / f32::from(MARKS)
    }

    fn at(mark: u8) -> Quat {
        Quat::from_rotation_y(Self::turn(mark))
    }

    /// Moves on to `mark`. Returns whether it moved.
    pub(super) fn go_to(&mut self, mark: u8) -> bool {
        if mark == self.mark {
            return false;
        }
        let marks = (mark + MARKS - self.mark) % MARKS;
        self.goal += Self::turn(marks);
        self.mark = mark;
        true
    }

    pub(super) fn swing(time: Res<Time>, mut hands: Query<(&mut Hand, &mut Transform)>) {
        let dt = time.delta_secs() / SUBSTEPS as f32;
        for (mut hand, mut transform) in &mut hands {
            if hand.speed == 0.0 && hand.angle == hand.goal {
                continue;
            }
            for _ in 0..SUBSTEPS {
                let pull = STIFFNESS * (hand.goal - hand.angle) - DAMPING * hand.speed;
                hand.speed += pull * dt;
                hand.angle += hand.speed * dt;
            }
            if (hand.goal - hand.angle).abs() < 1e-4 && hand.speed.abs() < 1e-3 {
                hand.angle = hand.goal;
                hand.speed = 0.0;
            }
            transform.rotation = Quat::from_rotation_y(hand.angle);
        }
    }
}

/// Spawns a bare watch movement lying dial up on posts, centered on the
/// origin, with its hand at `mark`.
pub(super) fn spawn(mark: u8) -> impl FnOnce(&mut ChildSpawnerCommands) {
    move |parent| spawn_at(parent, mark)
}

fn spawn_at(parent: &mut ChildSpawnerCommands, mark: u8) {
    for index in 0..3 {
        let angle = TAU * index as f32 / 3.0 + 0.5;
        let at = Vec3::new(angle.cos(), 0.0, angle.sin()) * (PLATE.x - 0.006);
        let post = Block::new(Vec3::new(POST.x, POST.y, POST.x)).translated(at);
        parent.spawn((Shape::cylinder(&post, BRASS), Finish::Plated));
    }
    let plate =
        Block::new(Vec3::new(2.0 * PLATE.x, PLATE.y, 2.0 * PLATE.x)).translated(Vec3::Y * POST.y);
    parent.spawn((Shape::cylinder(&plate, BRASS), Finish::Plated));
    let board = Block::new(BOARD).translated(Vec3::new(
        0.0,
        POST.y + PLATE.y / 2.0 - BOARD.y / 2.0,
        -PLATE.x - BOARD.z / 2.0 + 0.008,
    ));
    parent.spawn((Shape::cuboid(&board, BOARD_GREEN), Finish::Plastic));
    let chip = Block::new(CHIP)
        .translated(Vec3::new(0.0, 0.0, -PLATE.x - 0.004))
        .on_top_of(&board);
    parent.spawn((Shape::cuboid(&chip, CHIP_BLACK), Finish::Plastic));
    let socket = Block::new(SOCKET)
        .translated(Vec3::new(0.0, 0.0, CABLE_PLUG.z + SOCKET.z / 2.0))
        .on_top_of(&board);
    parent.spawn((Shape::cuboid(&socket, CHIP_BLACK), Finish::Plastic));
    let face = plate.max().y;
    let dial = Block::new(Vec3::new(2.0 * DIAL.x, DIAL.y, 2.0 * DIAL.x)).translated(Vec3::Y * face);
    parent.spawn((Shape::cylinder(&dial, ENAMEL), Finish::Ceramic));
    let face = dial.max().y;
    for index in 0..MARKS {
        let length = if index == 0 { 2.0 * MARK.z } else { MARK.z };
        let block = Block::new(MARK.with_z(length)).translated(Vec3::new(
            0.0,
            face,
            -MARK_RADIUS + (length - MARK.z) / 2.0,
        ));
        parent
            .spawn((
                Transform::from_rotation(Hand::at(index)),
                Visibility::default(),
            ))
            .with_children(|turned| {
                turned.spawn(Shape::cuboid(&block, INK));
            });
    }
    let hand = Block::new(HAND).translated(Vec3::new(0.0, 0.0, TAIL - HAND.z / 2.0));
    let cap = Block::new(Vec3::new(2.0 * CAP.x, CAP.y, 2.0 * CAP.x));
    parent
        .spawn((
            Hand::new(mark),
            Transform::from_xyz(0.0, face + 0.0004, 0.0).with_rotation(Hand::at(mark)),
            Visibility::default(),
        ))
        .with_children(|pivot| {
            pivot.spawn((Shape::cuboid(&hand, BLUED), Finish::Metal));
            pivot.spawn((Shape::cylinder(&cap, BLUED), Finish::Metal));
        });
}
