//! The alien script used on signs and labels around the city.

use std::ops::Range;

use bevy::prelude::*;
use rand::{Rng, RngExt};

/// Size of the point grid a glyph is drawn on, in grid steps.
pub const GLYPH_SIZE: Vec2 = Vec2::new(2.0, 3.0);

/// The letters of the script.
pub const ALPHABET: [Glyph; 20] = [
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

/// Space between glyphs of a word, in grid steps.
pub const GLYPH_GAP: f32 = 1.3;

/// A letter made of straight strokes between points of a 3 by 4 grid.
///
/// Grid points count from the top left corner.
#[derive(Debug)]
pub struct Glyph(&'static [Stroke]);

/// The two grid points a stroke connects.
type Stroke = ((u8, u8), (u8, u8));

/// A sequence of glyphs.
#[derive(Clone, Debug)]
pub struct Word(Vec<&'static Glyph>);

impl Glyph {
    /// Strokes as pairs of points, with +Y up and the origin at the top
    /// left corner.
    pub fn strokes(&self) -> impl Iterator<Item = (Vec2, Vec2)> {
        let point = |(x, y): (u8, u8)| Vec2::new(f32::from(x), -f32::from(y));
        self.0.iter().map(move |&(a, b)| (point(a), point(b)))
    }

    /// Like [`Glyph::strokes`], with the origin in the middle of the glyph.
    pub fn centered_strokes(&self) -> impl Iterator<Item = (Vec2, Vec2)> {
        let middle = GLYPH_SIZE * Vec2::new(0.5, -0.5);
        self.strokes().map(move |(a, b)| (a - middle, b - middle))
    }
}

impl Word {
    pub fn random(rng: &mut impl Rng, lengths: Range<usize>) -> Self {
        let length = rng.random_range(lengths);
        Self(
            (0..length)
                .map(|_| &ALPHABET[rng.random_range(0..ALPHABET.len())])
                .collect(),
        )
    }

    #[expect(clippy::len_without_is_empty, reason = "a word always has letters")]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// The first `length` glyphs, at most.
    pub fn truncated(mut self, length: usize) -> Self {
        self.0.truncate(length);
        self
    }

    /// Strokes of all glyphs, written left to right or top to bottom.
    ///
    /// The word is centered on the origin, in grid steps.
    pub fn strokes(&self, vertical: bool) -> Vec<(Vec2, Vec2)> {
        let size = self.size(vertical);
        let advance = if vertical {
            Vec2::new(0.0, -(GLYPH_SIZE.y + GLYPH_GAP))
        } else {
            Vec2::new(GLYPH_SIZE.x + GLYPH_GAP, 0.0)
        };
        let start = Vec2::new(-size.x / 2.0, size.y / 2.0);
        self.0
            .iter()
            .enumerate()
            .flat_map(|(index, glyph)| {
                let origin = start + advance * index as f32;
                glyph.strokes().map(move |(a, b)| (origin + a, origin + b))
            })
            .collect()
    }

    /// Width and height in grid steps.
    pub fn size(&self, vertical: bool) -> Vec2 {
        let count = self.0.len() as f32;
        let gaps = (count - 1.0).max(0.0) * GLYPH_GAP;
        if vertical {
            Vec2::new(GLYPH_SIZE.x, GLYPH_SIZE.y * count + gaps)
        } else {
            Vec2::new(GLYPH_SIZE.x * count + gaps, GLYPH_SIZE.y)
        }
    }
}
