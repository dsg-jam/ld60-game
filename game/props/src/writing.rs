//! Writing in the alien script on labels and boxes.
//!
//! All of it comes from one sheet of cards, drawn once. Each card has a
//! mesh of its own that shows it, and all share one material, so the
//! writing on a shelf costs little.

mod canvas;

use std::marker::PhantomData;
use std::ops::Range;

use bevy::asset::uuid::Uuid;
use bevy::asset::uuid_handle;
use bevy::prelude::*;
use ld60_game_parts::script::{GLYPH_GAP, GLYPH_SIZE, Word};
use rand::rngs::SmallRng;
use rand::{Rng, RngExt, SeedableRng};

use self::canvas::Canvas;

const SHEET: Handle<Image> = uuid_handle!("6f0e3a52-9c1d-4b7e-a3f4-2d8b1c5e7a90");
const INK: Handle<StandardMaterial> = uuid_handle!("1b7d4e2a-5c3f-4a81-9e6b-0c2f8d3a4b51");
/// The meshes of the cards follow this one.
const FIRST_CARD: u128 = 0x3c5a_8e1f_7b2d_4f60_a9c4_1e7b_5d3a_0000;
const SHEET_SIZE: UVec2 = UVec2::new(1024, 1408);
const LEVELS: u32 = 6;
const SEED: u64 = 60;
const INK_COLOR: Color = Color::srgb_u8(0x24, 0x20, 0x1c);
/// Space between words, in grid steps.
const WORD_GAP: f32 = 2.8;
/// Space between lines, in grid steps.
const LINE_GAP: f32 = 2.2;

pub(crate) struct WritingPlugin;

impl Plugin for WritingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Card::draw_all);
    }
}

/// What is written on a card, and so its shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Layout {
    /// A title and lines of notes, on a label wider than tall.
    Wide,
    /// A short title and short lines, on a label taller than wide.
    Tall,
    /// A word or two in thick marker.
    Marked,
    /// A single word.
    Tag,
}

/// One card of writing on the sheet.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Card {
    layout: Layout,
    index: u32,
}

impl Layout {
    const ALL: [Self; 4] = [Self::Wide, Self::Tall, Self::Marked, Self::Tag];

    /// The size of a card in pixels.
    fn size(self) -> UVec2 {
        match self {
            Self::Wide | Self::Marked => UVec2::new(256, 128),
            Self::Tall => UVec2::new(128, 256),
            Self::Tag => UVec2::new(128, 64),
        }
    }

    /// The top left corner of the first card and how many there are across
    /// and down.
    fn place(self) -> (UVec2, UVec2) {
        match self {
            Self::Wide => (UVec2::ZERO, UVec2::new(4, 3)),
            Self::Marked => (UVec2::new(0, 384), UVec2::new(4, 1)),
            Self::Tall => (UVec2::new(0, 512), UVec2::new(8, 3)),
            Self::Tag => (UVec2::new(0, 1280), UVec2::new(8, 2)),
        }
    }

    fn count(self) -> u32 {
        let (_, grid) = self.place();
        grid.x * grid.y
    }

    /// The layout for a label of `size`.
    fn fitting(size: Vec2) -> Self {
        if size.x < size.y {
            Self::Tall
        } else if size.x > 2.4 * size.y {
            Self::Tag
        } else {
            Self::Wide
        }
    }
}

impl Card {
    /// A card for a label of `size`, the same one for the same `n`.
    pub(crate) fn fitting(size: Vec2, n: u32) -> Self {
        Self::nth(Layout::fitting(size), n)
    }

    /// One of the cards of `layout`, the same one for the same `n`.
    pub(crate) fn nth(layout: Layout, n: u32) -> Self {
        Self {
            layout,
            index: n % layout.count(),
        }
    }

    /// Spawns the writing as large as fits in `size`, from its top left,
    /// on a face that looks towards +Z. `at` is the middle of that space.
    pub(crate) fn spawn(self, parent: &mut ChildSpawnerCommands, at: Vec3, size: Vec2) {
        let card = self.layout.size().as_vec2();
        let scale = (size / card).min_element();
        let shown = card * scale;
        let offset = Vec2::new(shown.x - size.x, size.y - shown.y) / 2.0;
        parent.spawn((
            Mesh3d(self.mesh()),
            MeshMaterial3d(INK),
            Transform::from_translation(at + offset.extend(0.0)).with_scale(shown.extend(1.0)),
        ));
    }

    fn id(self) -> u128 {
        let before: u32 = Layout::ALL
            .iter()
            .take_while(|&&layout| layout != self.layout)
            .map(|layout| layout.count())
            .sum();
        u128::from(before + self.index)
    }

    fn mesh(self) -> Handle<Mesh> {
        Handle::Uuid(Uuid::from_u128(FIRST_CARD + self.id()), PhantomData)
    }

    /// The top left corner of the card on the sheet, in pixels.
    fn corner(self) -> UVec2 {
        let (first, grid) = self.layout.place();
        first + UVec2::new(self.index % grid.x, self.index / grid.x) * self.layout.size()
    }

    fn draw_all(
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let mut rng = SmallRng::seed_from_u64(SEED);
        let mut sheet = Canvas::new(SHEET_SIZE);
        for layout in Layout::ALL {
            for index in 0..layout.count() {
                let card = Self { layout, index };
                card.write(&mut sheet, &mut rng);
                let min = card.corner().as_vec2() / SHEET_SIZE.as_vec2();
                let max = min + layout.size().as_vec2() / SHEET_SIZE.as_vec2();
                let mut mesh = Mesh::from(Rectangle::new(1.0, 1.0));
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_UV_0,
                    vec![
                        [max.x, min.y],
                        [min.x, min.y],
                        [min.x, max.y],
                        [max.x, max.y],
                    ],
                );
                // The meshes are made once, at the handles the cards use.
                #[expect(clippy::let_underscore_must_use, reason = "the ids are free")]
                let _ = meshes.insert(&card.mesh(), mesh);
            }
        }
        #[expect(clippy::let_underscore_must_use, reason = "the ids are free")]
        let _ = images.insert(&SHEET, sheet.image(LEVELS));
        #[expect(clippy::let_underscore_must_use, reason = "the ids are free")]
        let _ = materials.insert(
            &INK,
            StandardMaterial {
                base_color: INK_COLOR,
                base_color_texture: Some(SHEET),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.8,
                ..default()
            },
        );
    }

    fn write(self, sheet: &mut Canvas, rng: &mut impl Rng) {
        let corner = self.corner().as_vec2();
        let size = self.layout.size().as_vec2();
        let mut pen = Pen {
            sheet,
            left: corner.x,
            right: corner.x + size.x,
            top: corner.y,
            bottom: corner.y + size.y,
            shake: 0.0,
        };
        match self.layout {
            Layout::Wide => {
                pen.margin(14.0);
                pen.title(rng, 9.0, 3..7);
                pen.notes(rng, 4.5, 1..6);
            }
            Layout::Tall => {
                pen.margin(12.0);
                pen.title(rng, 8.0, 2..5);
                pen.notes(rng, 4.0, 1..4);
            }
            Layout::Marked => {
                pen.margin(10.0);
                pen.shake = 0.18;
                pen.title(rng, 14.0, 2..6);
                if rng.random_bool(0.5) {
                    pen.title(rng, 10.0, 1..4);
                }
            }
            Layout::Tag => {
                pen.margin(8.0);
                pen.title(rng, 7.0, 2..6);
            }
        }
    }
}

/// Writes lines into a box on the sheet, from its top down.
struct Pen<'a> {
    sheet: &'a mut Canvas,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    /// How far the ends of strokes stray, in grid steps.
    shake: f32,
}

impl Pen<'_> {
    fn margin(&mut self, margin: f32) {
        self.left += margin;
        self.right -= margin;
        self.top += margin;
        self.bottom -= margin;
    }

    /// One word of `glyphs` as large as `unit` pixels per grid step, as
    /// long as fits.
    fn title(&mut self, rng: &mut impl Rng, unit: f32, glyphs: Range<usize>) {
        let room = ((self.right - self.left) / unit + GLYPH_GAP) / (GLYPH_SIZE.x + GLYPH_GAP);
        let word = Word::random(rng, glyphs).truncated(room as usize);
        self.word(rng, &word, self.left, unit);
        self.top += unit * (GLYPH_SIZE.y + LINE_GAP * 1.4);
    }

    /// Lines of words of `glyphs` as small as `unit`, down to the bottom.
    /// The last line is often short.
    fn notes(&mut self, rng: &mut impl Rng, unit: f32, glyphs: Range<usize>) {
        while self.top + unit * GLYPH_SIZE.y <= self.bottom {
            let end = if rng.random_bool(0.3) {
                self.left + (self.right - self.left) * rng.random_range(0.3..0.8)
            } else {
                self.right
            };
            let mut x = self.left;
            loop {
                let word = Word::random(rng, glyphs.clone());
                let width = word.size(false).x * unit;
                if x + width > end {
                    break;
                }
                self.word(rng, &word, x, unit);
                x += width + WORD_GAP * unit;
            }
            self.top += unit * (GLYPH_SIZE.y + LINE_GAP);
        }
    }

    fn word(&mut self, rng: &mut impl Rng, word: &Word, left: f32, unit: f32) {
        let half = word.size(false) / 2.0;
        let width = unit * if self.shake > 0.0 { 0.42 } else { 0.32 };
        for (a, b) in word.strokes(false) {
            let [a, b] = [a, b].map(|point| {
                let stray = Vec2::new(rng.random_range(-1.0..=1.0), rng.random_range(-1.0..=1.0))
                    * self.shake;
                let point = point + half * Vec2::new(1.0, -1.0) + stray;
                Vec2::new(left + point.x * unit, self.top - point.y * unit)
            });
            self.sheet.stroke(a, b, width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cards() -> impl Iterator<Item = (Card, URect)> {
        Layout::ALL.into_iter().flat_map(|layout| {
            (0..layout.count()).map(move |index| {
                let card = Card { layout, index };
                let corner = card.corner();
                (card, URect::from_corners(corner, corner + layout.size()))
            })
        })
    }

    #[test]
    fn the_cards_fit_on_the_sheet_apart() {
        assert_eq!(SHEET_SIZE % (1 << (LEVELS - 1)), UVec2::ZERO);
        for (card, rect) in cards() {
            assert!(rect.max.cmple(SHEET_SIZE).all(), "{card:?}");
            for (other, other_rect) in cards() {
                if card.id() != other.id() {
                    assert!(rect.intersect(other_rect).is_empty(), "{card:?} {other:?}");
                }
            }
        }
    }
}
