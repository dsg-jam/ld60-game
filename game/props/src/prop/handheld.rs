use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::surface::Finish;
use rand::{Rng, RngExt};

use super::{Board, Prop, part, placed, shade};

const CASES: [Color; 5] = [
    Color::srgb_u8(0x8a, 0x8a, 0x84),
    Color::srgb_u8(0xc4, 0xb8, 0x98),
    Color::srgb_u8(0x3a, 0x3a, 0x3c),
    Color::srgb_u8(0xb8, 0x9a, 0x48),
    Color::srgb_u8(0x5e, 0x66, 0x52),
];
const SCREEN: Color = Color::srgb_u8(0x26, 0x2c, 0x26);
const KEY: Color = Color::srgb_u8(0x30, 0x30, 0x32);
const KEYS: UVec2 = UVec2::new(3, 4);
const KEY_HEIGHT: f32 = 0.003;
const SCREEN_HEIGHT: f32 = 0.001;
/// The board of an open device sits this far below the rim of its case.
const BOARD_DROP: f32 = 0.004;
const CASE_WALL: f32 = 0.003;

/// A handheld device lying face up, brought in for parts. Some have their
/// cover off and show their board.
pub(crate) struct Handheld {
    size: Vec3,
    case: Color,
    face: Face,
}

enum Face {
    Bare,
    Keys,
    Open(Board),
}

impl Handheld {
    /// A device within `most`, or none if it does not fit. Only one on
    /// top of a pile needs its keys, see `keys`.
    pub(crate) fn random(rng: &mut impl Rng, most: Vec3, keys: bool) -> Option<Self> {
        let size = Vec3::new(
            rng.random_range(0.07..0.095f32),
            rng.random_range(0.018..0.03f32),
            rng.random_range(0.12..0.17f32),
        );
        if size.cmpgt(most).any() {
            return None;
        }
        let face = if rng.random_bool(0.25) {
            let inside = size - Vec3::new(2.0, 0.0, 2.0) * CASE_WALL;
            Face::Open(Board::random(rng, inside, keys)?)
        } else if keys {
            Face::Keys
        } else {
            Face::Bare
        };
        Some(Self {
            size,
            case: shade(rng, &CASES),
            face,
        })
    }
}

impl Prop for Handheld {
    fn size(&self) -> Vec3 {
        self.size
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        let size = self.size;
        let Face::Open(board) = &self.face else {
            let case = Block::new(size - Vec3::Y * SCREEN_HEIGHT);
            part(parent, case, self.case, Finish::Plastic);
            let screen = Block::new(Vec3::new(size.x * 0.75, SCREEN_HEIGHT, size.z * 0.3))
                .on_top_of(&case)
                .translated(Vec3::Z * -size.z * 0.22);
            part(parent, screen, SCREEN, Finish::Plastic);
            if matches!(self.face, Face::Keys) {
                let area = Vec2::new(size.x * 0.75, size.z * 0.45);
                let pitch = area / KEYS.as_vec2();
                for index in 0..KEYS.x * KEYS.y {
                    let cell = UVec2::new(index % KEYS.x, index / KEYS.x).as_vec2();
                    let at = Vec2::new(-area.x / 2.0, size.z * 0.02) + (cell + 0.5) * pitch;
                    let key = Block::new((pitch * 0.7).extend(KEY_HEIGHT).xzy())
                        .on_top_of(&case)
                        .translated(Vec3::new(at.x, 0.0, at.y));
                    part(parent, key, KEY, Finish::Plastic);
                }
            }
            return;
        };
        // The bottom shell, with the board lying in it.
        let shell = Block::new(size.with_y(CASE_WALL));
        part(parent, shell, self.case, Finish::Plastic);
        for side in [-1.0, 1.0] {
            let wall = Block::new(Vec3::new(CASE_WALL, size.y, size.z))
                .translated(Vec3::X * side * (size.x - CASE_WALL) / 2.0);
            part(parent, wall, self.case, Finish::Plastic);
            let wall = Block::new(Vec3::new(size.x - 2.0 * CASE_WALL, size.y, CASE_WALL))
                .translated(Vec3::Z * side * (size.z - CASE_WALL) / 2.0);
            part(parent, wall, self.case, Finish::Plastic);
        }
        let lift = (size.y - BOARD_DROP - board.size().y).max(CASE_WALL);
        placed(parent, Transform::from_xyz(0.0, lift, 0.0), |inside| {
            board.spawn(inside);
        });
    }
}
