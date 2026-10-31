use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::surface::Finish;
use rand::{Rng, RngExt};

use super::super::writing::{Card, Layout};
use super::{INK_DEPTH, Prop, label, part, placed, shade};

const CARDBOARD: [Color; 3] = [
    Color::srgb_u8(0xa8, 0x86, 0x5c),
    Color::srgb_u8(0x94, 0x74, 0x4e),
    Color::srgb_u8(0xb8, 0x9c, 0x72),
];
const TAPE: [Color; 2] = [
    Color::srgb_u8(0xc8, 0xa4, 0x68),
    Color::srgb_u8(0x8a, 0x8c, 0x8e),
];
const LEAST: Vec3 = Vec3::new(0.16, 0.1, 0.16);
const MOST: Vec3 = Vec3::new(0.42, 0.38, 0.4);
const WALL: f32 = 0.004;
const TAPE_WIDTH: f32 = 0.048;
const TAPE_DEPTH: f32 = 0.0008;
/// How far the tape reaches down the front and back.
const TAPE_DROP: f32 = 0.06;
/// The long flaps of an open carton lean out by this much, in radians.
const FLAP_LEAN: std::ops::Range<f32> = 0.15..0.5;
/// The short flaps fold in over the edge by this much past upright.
const FOLD: f32 = 0.5;
/// What stands in an open carton reaches up to this fraction of its height.
const FILLED: f32 = 0.9;

/// A cardboard box, taped shut or open with its flaps up and something in
/// it.
pub(crate) struct Carton {
    body: Vec3,
    color: Color,
    look: Look,
    writing: u32,
}

enum Look {
    Taped {
        tape: Color,
        label: Option<f32>,
    },
    /// Writing in marker on the front.
    Marked,
    Open {
        lean: f32,
        contents: Option<Box<dyn Prop>>,
    },
}

impl Carton {
    /// A closed carton within `most`, or none if no carton fits.
    pub(crate) fn closed(rng: &mut impl Rng, most: Vec3) -> Option<Self> {
        let body = Self::body(rng, most)?;
        let look = if rng.random_bool(0.75) {
            Look::Taped {
                tape: shade(rng, &TAPE),
                label: rng
                    .random_bool(0.6)
                    .then(|| rng.random_range(-0.2..0.2) * body.x),
            }
        } else {
            Look::Marked
        };
        Some(Self {
            body,
            color: shade(rng, &CARDBOARD),
            look,
            writing: rng.random(),
        })
    }

    /// An open carton within `most`, with what `fill` makes for the room
    /// inside standing in it.
    pub(crate) fn open<R: Rng>(
        rng: &mut R,
        most: Vec3,
        fill: impl FnOnce(&mut R, Vec3) -> Option<Box<dyn Prop>>,
    ) -> Option<Self> {
        let lean = rng.random_range(FLAP_LEAN);
        // The flaps are half as long as the carton is deep.
        let deep = most.z / (1.0 + lean.sin());
        let most = Vec3::new(most.x, most.y - deep / 2.0 * lean.cos(), deep);
        let body = Self::body(rng, most)?;
        let contents = fill(rng, body - 2.0 * WALL);
        Some(Self {
            body,
            color: shade(rng, &CARDBOARD),
            look: Look::Open { lean, contents },
            writing: rng.random(),
        })
    }

    fn body(rng: &mut impl Rng, most: Vec3) -> Option<Vec3> {
        let most = most.min(MOST);
        LEAST.cmple(most).all().then(|| {
            Vec3::new(
                rng.random_range(LEAST.x..=most.x),
                rng.random_range(LEAST.y..=most.y),
                rng.random_range(LEAST.z.max(most.z * 0.75)..=most.z),
            )
        })
    }

    fn flap(&self) -> f32 {
        self.body.z / 2.0
    }

    fn spawn_taped(&self, parent: &mut ChildSpawnerCommands, tape: Color, spot: Option<f32>) {
        let body = Block::new(self.body);
        part(parent, body, self.color, Finish::Paper);
        let top = Block::new(Vec3::new(TAPE_WIDTH, TAPE_DEPTH, self.body.z)).on_top_of(&body);
        part(parent, top, tape, Finish::Plastic);
        for side in [-1.0, 1.0] {
            let drop =
                Block::new(Vec3::new(TAPE_WIDTH, TAPE_DROP, TAPE_DEPTH)).translated(Vec3::new(
                    0.0,
                    self.body.y - TAPE_DROP,
                    side * (self.body.z + TAPE_DEPTH) / 2.0,
                ));
            part(parent, drop, tape, Finish::Plastic);
        }
        if let Some(x) = spot {
            let size = Vec2::new(0.09, 0.06).min(self.body.xy() * 0.4);
            let at = Vec3::new(x, self.body.y * 0.5, self.body.z / 2.0);
            label(parent, at, size, self.writing);
        }
    }

    fn spawn_marked(&self, parent: &mut ChildSpawnerCommands) {
        part(parent, Block::new(self.body), self.color, Finish::Paper);
        let size = self.body.xy() * Vec2::new(0.7, 0.4);
        Card::nth(Layout::Marked, self.writing).spawn(
            parent,
            Vec3::new(
                -self.body.x * 0.05,
                self.body.y * 0.65,
                self.body.z / 2.0 + INK_DEPTH,
            ),
            size,
        );
    }

    fn spawn_open(
        &self,
        parent: &mut ChildSpawnerCommands,
        lean: f32,
        contents: Option<&dyn Prop>,
    ) {
        let size = self.body;
        part(
            parent,
            Block::new(size.with_y(WALL)),
            self.color,
            Finish::Paper,
        );
        for side in [-1.0, 1.0] {
            let wall = Block::new(Vec3::new(WALL, size.y, size.z))
                .translated(Vec3::X * side * (size.x - WALL) / 2.0);
            part(parent, wall, self.color, Finish::Paper);
            let wall = Block::new(Vec3::new(size.x - 2.0 * WALL, size.y, WALL))
                .translated(Vec3::Z * side * (size.z - WALL) / 2.0);
            part(parent, wall, self.color, Finish::Paper);
        }
        let flap = self.flap();
        for side in [-1.0, 1.0] {
            // Hinged on the top edge of the front or back.
            let hinge = Vec3::new(0.0, size.y, side * (size.z - WALL) / 2.0);
            let at = Transform::from_translation(hinge)
                .with_rotation(Quat::from_rotation_x(side * lean));
            placed(parent, at, |hinged| {
                let sheet = Block::new(Vec3::new(size.x, flap, WALL));
                part(hinged, sheet, self.color, Finish::Paper);
            });
        }
        for side in [-1.0, 1.0] {
            let hinge = Vec3::new(side * (size.x - WALL) / 2.0, size.y, 0.0);
            let at = Transform::from_translation(hinge)
                .with_rotation(Quat::from_rotation_z(side * (FRAC_PI_2 + FOLD)));
            placed(parent, at, |hinged| {
                let sheet = Block::new(Vec3::new(WALL, (size.x / 4.0).min(0.06), size.z * 0.9));
                part(hinged, sheet, self.color, Finish::Paper);
            });
        }
        if let Some(contents) = contents {
            let lift = (size.y * FILLED - contents.size().y).max(WALL);
            placed(parent, Transform::from_xyz(0.0, lift, 0.0), |inside| {
                contents.spawn(inside);
            });
        }
    }
}

impl Prop for Carton {
    fn size(&self) -> Vec3 {
        match &self.look {
            Look::Open { lean, .. } => {
                let flap = self.flap();
                Vec3::new(
                    self.body.x,
                    self.body.y + flap * lean.cos(),
                    self.body.z + 2.0 * flap * lean.sin(),
                )
            }
            _ => self.body,
        }
    }

    fn spawn(&self, parent: &mut ChildSpawnerCommands) {
        match &self.look {
            Look::Taped { tape, label } => self.spawn_taped(parent, *tape, *label),
            Look::Marked => self.spawn_marked(parent),
            Look::Open { lean, contents } => self.spawn_open(parent, *lean, contents.as_deref()),
        }
    }
}
