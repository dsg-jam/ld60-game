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

use super::writing::Card;

pub(crate) use self::binder::Binder;
pub(crate) use self::board::Board;
pub(crate) use self::carton::Carton;
pub(crate) use self::coil::Coil;
pub(crate) use self::drawers::Drawers;
pub(crate) use self::handheld::Handheld;
pub(crate) use self::jar::{Jar, Tin};
pub(crate) use self::turned::{Lying, Standing};

const LABEL: Color = Color::srgb_u8(0xe0, 0xd8, 0xc4);
/// Writing stands this far out of the face it is on.
const INK_DEPTH: f32 = 0.0003;
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

/// A paper label of `size` with writing on it, stuck on a face that looks
/// towards +Z. `at` is the middle of the label on that face. The same
/// `writing` writes the same.
pub(crate) fn label(parent: &mut ChildSpawnerCommands, at: Vec3, size: Vec2, writing: u32) {
    let paper = Block::new(size.extend(LABEL_DEPTH))
        .translated(at - Vec3::Y * size.y / 2.0 + Vec3::Z * LABEL_DEPTH / 2.0);
    part(parent, paper, LABEL, Finish::Paper);
    let margin = size.min_element() * 0.12;
    Card::fitting(size, writing).spawn(
        parent,
        at + Vec3::Z * (LABEL_DEPTH + INK_DEPTH),
        size - margin,
    );
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
