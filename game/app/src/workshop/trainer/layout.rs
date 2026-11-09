use bevy::prelude::*;
use ld60_game_parts::drawing::Drawing;
use ld60_locale::{Locale, Message};
use ld60_tick::WORD_BITS;

use super::{Control, Lamp, WORDS};

pub(super) const CASE: Vec3 = Vec3::new(0.17, 0.03, 0.15);
/// The rows of words from word 0 at the back, and the columns of their
/// bits from the highest on the left, from the middle of the top, +Y
/// towards the front.
const ROWS: [f32; WORDS] = [-0.045, -0.02, 0.005, 0.03];
const COLUMNS: [f32; WORD_BITS as usize] = [-0.04, -0.018, 0.004, 0.026];
/// The lamp of each row on its right, its number on its left, and the RUN
/// switch in front.
const LAMP_COLUMN: f32 = 0.056;
const NUMBER_COLUMN: f32 = -0.064;
const RUN: Vec2 = Vec2::new(0.056, 0.058);
/// Labels sit this far in front of what they name.
const LABEL_GAP: f32 = 0.013;
const DOT: f32 = 0.0005;
const LINE: f32 = 0.0006;

pub(super) const PLATE_COLOR: Color = Color::srgb_u8(0x3a, 0x40, 0x48);
const PLATE_INK: Color = Color::srgb_u8(0xe8, 0xe4, 0xd8);

/// Where each control stands on the top of the case.
pub(super) fn controls() -> impl Iterator<Item = (Control, Vec2)> {
    let bits = ROWS.into_iter().enumerate().flat_map(|(word, y)| {
        COLUMNS.into_iter().enumerate().map(move |(column, x)| {
            let bit = WORD_BITS - 1 - column as u8;
            (Control::Bit { word, bit }, Vec2::new(x, y))
        })
    });
    bits.chain([(Control::Run, RUN)])
}

/// Where each lamp stands on the top of the case.
pub(super) fn lamps() -> impl Iterator<Item = (Lamp, Vec2)> {
    ROWS.into_iter()
        .enumerate()
        .map(|(word, y)| (Lamp::Row(word), Vec2::new(LAMP_COLUMN, y)))
}

/// The printed top of the case, with the words in the player's language.
pub(super) fn plate(locale: &Locale) -> Image {
    let word = |id: &'static str| locale.text(&Message::new(id).into());
    let mut plate = Drawing::new(CASE.xz(), PLATE_COLOR, PLATE_INK);
    for (index, &y) in ROWS.iter().enumerate() {
        plate.text(&index.to_string(), Vec2::new(NUMBER_COLUMN, y), DOT);
    }
    // A line under each row ties its switches to its lamp.
    let left = COLUMNS[0] - 0.009;
    let right = LAMP_COLUMN;
    for &y in &ROWS {
        plate.stroke(
            Vec2::new(left, y + 0.0105),
            Vec2::new(right, y + 0.0105),
            LINE,
        );
    }
    plate.text(&word("panel-run"), RUN + Vec2::Y * LABEL_GAP, DOT);
    plate.image()
}
