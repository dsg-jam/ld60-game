//! The alien script used on signs and labels around the city.

use bevy::prelude::*;

/// Size of the point grid a glyph is drawn on, in grid steps.
pub(crate) const GLYPH_SIZE: Vec2 = Vec2::new(2.0, 3.0);

/// The letters of the script.
pub(crate) const ALPHABET: [Glyph; 20] = [
    Glyph(&[((1, 0), (1, 3)), ((1, 3), (2, 2))]),
    Glyph(&[((0, 0), (2, 0)), ((1, 0), (1, 3)), ((0, 2), (1, 3))]),
    Glyph(&[((0, 0), (0, 3)), ((0, 3), (2, 3)), ((2, 3), (2, 1))]),
    Glyph(&[((0, 1), (2, 1)), ((2, 1), (0, 3))]),
    Glyph(&[((1, 0), (2, 1)), ((2, 1), (1, 2)), ((1, 2), (1, 3))]),
    Glyph(&[((0, 0), (2, 0)), ((2, 0), (2, 3)), ((0, 2), (2, 2))]),
    Glyph(&[((0, 3), (1, 0)), ((1, 0), (2, 3)), ((0, 1), (2, 1))]),
    Glyph(&[
        ((0, 0), (0, 2)),
        ((0, 2), (2, 2)),
        ((2, 2), (2, 0)),
        ((1, 2), (1, 3)),
    ]),
    Glyph(&[((0, 1), (1, 0)), ((1, 0), (2, 1)), ((1, 0), (1, 3))]),
    Glyph(&[((0, 3), (2, 3)), ((1, 3), (1, 1)), ((0, 0), (2, 1))]),
    Glyph(&[
        ((0, 0), (1, 1)),
        ((1, 1), (2, 0)),
        ((1, 1), (1, 3)),
        ((0, 3), (2, 3)),
    ]),
    Glyph(&[((2, 0), (0, 1)), ((0, 1), (2, 2)), ((2, 2), (0, 3))]),
    Glyph(&[((0, 0), (0, 3)), ((2, 0), (2, 3)), ((0, 1), (2, 2))]),
    Glyph(&[((0, 2), (1, 3)), ((1, 3), (2, 2)), ((1, 0), (1, 3))]),
    Glyph(&[
        ((0, 0), (2, 0)),
        ((0, 0), (0, 1)),
        ((2, 3), (0, 3)),
        ((2, 3), (2, 2)),
    ]),
    Glyph(&[((1, 0), (1, 2)), ((0, 3), (2, 3))]),
    Glyph(&[((0, 1), (2, 1)), ((0, 2), (2, 2)), ((1, 0), (1, 1))]),
    Glyph(&[((0, 0), (2, 3)), ((2, 0), (1, 1))]),
    Glyph(&[((0, 0), (1, 0)), ((1, 0), (1, 3)), ((1, 3), (2, 3))]),
    Glyph(&[
        ((0, 1), (1, 0)),
        ((1, 0), (2, 1)),
        ((2, 1), (1, 2)),
        ((1, 2), (0, 1)),
        ((1, 2), (1, 3)),
    ]),
];

/// A letter made of straight strokes between points of a 3 by 4 grid.
///
/// Grid points count from the top left corner.
#[derive(Debug)]
pub(crate) struct Glyph(&'static [Stroke]);

/// The two grid points a stroke connects.
type Stroke = ((u8, u8), (u8, u8));

impl Glyph {
    /// Strokes as pairs of points, with +Y up and the origin at the top
    /// left corner.
    pub(crate) fn strokes(&self) -> impl Iterator<Item = (Vec2, Vec2)> {
        let point = |(x, y): (u8, u8)| Vec2::new(f32::from(x), -f32::from(y));
        self.0.iter().map(move |&(a, b)| (point(a), point(b)))
    }
}
