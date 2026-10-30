mod binder;
mod board;
mod carton;
mod coil;
mod drawers;
mod handheld;
mod jar;
mod turned;

use bevy::prelude::*;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use rand::{Rng, RngExt};

pub(crate) use self::binder::Binder;
pub(crate) use self::board::Board;
pub(crate) use self::carton::Carton;
pub(crate) use self::coil::Coil;
pub(crate) use self::drawers::Drawers;
pub(crate) use self::handheld::Handheld;
pub(crate) use self::jar::{Jar, Tin};
pub(crate) use self::turned::{Lying, Standing};

const LABEL: Color = Color::srgb_u8(0xe0, 0xd8, 0xc4);
const INK: Color = Color::srgb_u8(0x2a, 0x26, 0x22);
/// Writing stands this far out of a label.
const INK_DEPTH: f32 = 0.0004;
/// A label stands this far out of the face it is stuck on.
const LABEL_DEPTH: f32 = 0.0006;

/// A thing that stands on a shelf.
///
/// It is built in its own space: the middle of its bottom at the origin,
/// +Y up and its front towards +Z.
pub(crate) trait Prop {
    /// The box it fills.
    fn size(&self) -> Vec3;

    fn spawn(&self, parent: &mut ChildSpawnerCommands);
}

/// Spawns a box of `color` with `finish` filling `block`.
pub(crate) fn part(parent: &mut ChildSpawnerCommands, block: Block, color: Color, finish: Finish) {
    parent.spawn((Shape::cuboid(&block, color), finish));
}

/// A paper label of `size` with `lines` of writing, stuck on a face that
/// looks towards +Z. `at` is the middle of the label on that face.
pub(crate) fn label(parent: &mut ChildSpawnerCommands, at: Vec3, size: Vec2, lines: usize) {
    let paper = Block::new(size.extend(LABEL_DEPTH))
        .translated(at - Vec3::Y * size.y / 2.0 + Vec3::Z * LABEL_DEPTH / 2.0);
    part(parent, paper, LABEL, Finish::Paper);
    let pitch = size.y / (lines + 1) as f32;
    for line in 1..=lines {
        // Shorter lines below the first, like a title and notes.
        let width = size.x * if line == 1 { 0.75 } else { 0.5 };
        let ink = Block::new(Vec3::new(width, pitch * 0.35, INK_DEPTH)).translated(Vec3::new(
            paper.min().x + size.x * 0.12 + width / 2.0,
            paper.max().y - pitch * line as f32 - pitch * 0.175,
            paper.max().z + INK_DEPTH / 2.0,
        ));
        part(parent, ink, INK, Finish::Paint);
    }
}

/// One of `colors`, a little lighter or darker.
pub(crate) fn shade(rng: &mut impl Rng, colors: &[Color]) -> Color {
    let color = colors[rng.random_range(..colors.len())];
    let factor = rng.random_range(0.85..1.12);
    let linear = color.to_linear();
    Color::linear_rgb(
        linear.red * factor,
        linear.green * factor,
        linear.blue * factor,
    )
}

/// Spawns what `spawn` makes as children of an entity at `at`.
pub(crate) fn placed(
    parent: &mut ChildSpawnerCommands,
    at: Transform,
    spawn: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((at, Visibility::default()))
        .with_children(spawn);
}
