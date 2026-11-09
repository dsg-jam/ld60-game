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
/// The chip on the side of the plate.
const CHIP: Vec3 = Vec3::new(0.012, 0.003, 0.008);

const BRASS: Color = Color::srgb_u8(0xb0, 0x8a, 0x48);
const ENAMEL: Color = Color::srgb_u8(0xee, 0xe8, 0xd8);
const INK: Color = Color::srgb_u8(0x1a, 0x1a, 0x1c);
const BLUED: Color = Color::srgb_u8(0x1c, 0x24, 0x48);
const CHIP_BLACK: Color = Color::srgb_u8(0x22, 0x22, 0x24);

/// The hand of the movement, turning about +Y. It points at `-Z` at mark 0
/// and goes clockwise seen from above.
#[derive(Component, Debug)]
pub(super) struct Hand;

impl Hand {
    pub(super) fn at(mark: u8) -> Quat {
        Quat::from_rotation_y(-TAU * f32::from(mark) / f32::from(MARKS))
    }
}

/// Spawns a bare watch movement lying dial up on posts, centered on the
/// origin.
pub(super) fn spawn(parent: &mut ChildSpawnerCommands) {
    for index in 0..3 {
        let angle = TAU * index as f32 / 3.0 + 0.5;
        let at = Vec3::new(angle.cos(), 0.0, angle.sin()) * (PLATE.x - 0.006);
        let post = Block::new(Vec3::new(POST.x, POST.y, POST.x)).translated(at);
        parent.spawn((Shape::cylinder(&post, BRASS), Finish::Plated));
    }
    let plate =
        Block::new(Vec3::new(2.0 * PLATE.x, PLATE.y, 2.0 * PLATE.x)).translated(Vec3::Y * POST.y);
    parent.spawn((Shape::cylinder(&plate, BRASS), Finish::Plated));
    let chip = Block::new(CHIP).translated(Vec3::new(PLATE.x * 0.55, POST.y - CHIP.y, PLATE.x));
    parent.spawn((Shape::cuboid(&chip, CHIP_BLACK), Finish::Plastic));
    let face = plate.max().y;
    let dial = Block::new(Vec3::new(2.0 * DIAL.x, DIAL.y, 2.0 * DIAL.x)).translated(Vec3::Y * face);
    parent.spawn((Shape::cylinder(&dial, ENAMEL), Finish::Ceramic));
    let face = dial.max().y;
    for mark in 0..MARKS {
        let length = if mark == 0 { 2.0 * MARK.z } else { MARK.z };
        let block = Block::new(MARK.with_z(length)).translated(Vec3::new(
            0.0,
            face,
            -MARK_RADIUS + (length - MARK.z) / 2.0,
        ));
        parent
            .spawn((
                Transform::from_rotation(Hand::at(mark)),
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
            Hand,
            Transform::from_xyz(0.0, face + 0.0004, 0.0),
            Visibility::default(),
        ))
        .with_children(|pivot| {
            pivot.spawn((Shape::cuboid(&hand, BLUED), Finish::Metal));
            pivot.spawn((Shape::cylinder(&cap, BLUED), Finish::Metal));
        });
}
