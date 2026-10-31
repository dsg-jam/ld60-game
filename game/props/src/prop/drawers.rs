use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::surface::Finish;
use rand::{Rng, RngExt};

use super::super::writing::{Card, Layout};
use super::{INK_DEPTH, LABEL, LABEL_DEPTH, Prop, part, shade};

const FRAMES: [Color; 3] = [
    Color::srgb_u8(0xc4, 0xbc, 0xa8),
    Color::srgb_u8(0x5a, 0x5c, 0x60),
    Color::srgb_u8(0x6a, 0x7e, 0x96),
];
/// Milky plastic, tinted.
const FRONTS: [Color; 4] = [
    Color::srgb_u8(0xd4, 0xd4, 0xcc),
    Color::srgb_u8(0x9c, 0xb4, 0xc8),
    Color::srgb_u8(0xc8, 0x5a, 0x40),
    Color::srgb_u8(0xd0, 0xb0, 0x50),
];
const DRAWER: Vec2 = Vec2::new(0.06, 0.04);
const GAP: f32 = 0.004;
const FRONT: f32 = 0.003;
const LEAST_DEPTH: f32 = 0.12;
const MOST_DEPTH: f32 = 0.2;
/// Some drawers stand open by up to this much.
const MOST_OPEN: f32 = 0.04;

/// A cabinet of small plastic drawers for parts, each with a label.
pub(crate) struct Drawers {
    size: Vec3,
    grid: UVec2,
    frame: Color,
    front: Color,
    /// How far each drawer stands out, row by row.
    open: Vec<f32>,
    writing: u32,
}

impl Drawers {
    /// A cabinet within `most`, or none if not even one drawer fits.
    pub(crate) fn random(rng: &mut impl Rng, most: Vec3) -> Option<Self> {
        let depth = most.z.min(MOST_DEPTH) - MOST_OPEN;
        if depth < LEAST_DEPTH {
            return None;
        }
        let pitch = DRAWER + GAP;
        let fits = ((most.xy() - GAP) / pitch).floor().as_uvec2();
        if fits.cmpeq(UVec2::ZERO).any() {
            return None;
        }
        let grid = UVec2::new(
            rng.random_range(fits.x.min(2)..=fits.x.min(5)),
            rng.random_range(fits.y.min(3)..=fits.y.min(7)),
        );
        let open = (0..grid.x * grid.y)
            .map(|_| {
                if rng.random_bool(0.12) {
                    rng.random_range(0.005..MOST_OPEN)
                } else {
                    0.0
                }
            })
            .collect();
        Some(Self {
            size: (grid.as_vec2() * pitch + GAP).extend(depth),
            grid,
            frame: shade(rng, &FRAMES),
            front: shade(rng, &FRONTS),
            open,
            writing: rng.random(),
        })
    }
}

impl Prop for Drawers {
    fn size(&self) -> Vec3 {
        self.size + Vec3::Z * MOST_OPEN
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        let back = Vec3::Z * -MOST_OPEN / 2.0;
        let frame = Block::new(self.size).translated(back);
        part(parent, frame, self.frame, Finish::Plastic);
        let pitch = DRAWER + GAP;
        for (index, open) in self.open.iter().enumerate() {
            let cell = UVec2::new(index as u32 % self.grid.x, index as u32 / self.grid.x);
            let corner = Vec2::new(-self.size.x / 2.0, 0.0) + GAP + cell.as_vec2() * pitch;
            let front = Block::new(DRAWER.extend(FRONT + open)).translated(
                corner.extend(frame.max().z + (FRONT + open) / 2.0) + Vec3::X * DRAWER.x / 2.0,
            );
            part(parent, front, self.front, Finish::Plastic);
            let label = Block::new(Vec3::new(DRAWER.x * 0.6, DRAWER.y * 0.35, LABEL_DEPTH))
                .translated(Vec3::new(
                    front.center.x,
                    front.min().y + DRAWER.y * 0.5,
                    front.max().z + LABEL_DEPTH / 2.0,
                ));
            part(parent, label, LABEL, Finish::Paper);
            Card::nth(Layout::Tag, self.writing.wrapping_add(index as u32)).spawn(
                parent,
                label.center.with_z(label.max().z + INK_DEPTH),
                label.size.xy() * 0.85,
            );
        }
    }
}
