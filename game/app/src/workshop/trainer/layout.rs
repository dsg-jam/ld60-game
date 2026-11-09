use bevy::prelude::*;
use ld60_game_parts::drawing::Drawing;
use ld60_locale::{Locale, Message};

use super::{Control, Lamp};

pub(super) const CASE: Vec3 = Vec3::new(0.24, 0.03, 0.16);
/// Where things stand on the top of the case, from its middle, +Y towards
/// the front.
pub(super) const MOVEMENT: Vec2 = Vec2::new(-0.06, -0.025);
const ADDRESS_ROW: f32 = -0.05;
const DATA_ROW: f32 = -0.015;
const SWITCH_ROW: f32 = 0.015;
const BUTTON_ROW: f32 = 0.05;
/// The data bits from 3 down to 0, left to right.
const BIT_COLUMNS: [f32; 4] = [0.025, 0.045, 0.065, 0.085];
const ADDRESS_COLUMNS: [f32; 3] = [0.025, 0.04, 0.055];
/// Labels sit this far behind what they name, or in front of buttons.
const LABEL_GAP: f32 = 0.012;
const DOT: f32 = 0.0005;
const LINE: f32 = 0.0006;

pub(super) const PLATE_COLOR: Color = Color::srgb_u8(0x3a, 0x40, 0x48);
const PLATE_INK: Color = Color::srgb_u8(0xe8, 0xe4, 0xd8);

/// Where each control stands on the top of the case.
pub(super) fn controls() -> impl Iterator<Item = (Control, Vec2)> {
    let switches = (0..4).map(|index| {
        (
            Control::Data(3 - index as u8),
            Vec2::new(BIT_COLUMNS[index], SWITCH_ROW),
        )
    });
    let buttons = [
        (Control::Reset, -0.095),
        (Control::Erase, -0.06),
        (Control::Next, 0.0),
        (Control::Deposit, 0.03),
        (Control::Step, 0.06),
        (Control::Run, 0.095),
    ]
    .map(|(control, x)| (control, Vec2::new(x, BUTTON_ROW)));
    switches.chain(buttons)
}

/// Where each lamp stands on the top of the case.
pub(super) fn lamps() -> impl Iterator<Item = (Lamp, Vec2)> {
    let address = (0..3).map(|index| {
        (
            Lamp::Address(2 - index as u8),
            Vec2::new(ADDRESS_COLUMNS[index], ADDRESS_ROW),
        )
    });
    let data = (0..4).map(|index| {
        (
            Lamp::Data(3 - index as u8),
            Vec2::new(BIT_COLUMNS[index], DATA_ROW),
        )
    });
    let others = [
        (Lamp::Wait, Vec2::new(0.09, ADDRESS_ROW)),
        (
            Lamp::Erasing,
            Vec2::new(-0.06, BUTTON_ROW - LABEL_GAP - 0.006),
        ),
    ];
    address.chain(data).chain(others)
}

/// The printed top of the case, with the words in the player's language.
pub(super) fn plate(locale: &Locale) -> Image {
    let word = |id: &'static str| locale.text(&Message::new(id).into());
    let mut plate = Drawing::new(CASE.xz(), PLATE_COLOR, PLATE_INK);
    let middle = |columns: &[f32]| (columns[0] + columns[columns.len() - 1]) / 2.0;
    plate.text(
        &word("panel-address"),
        Vec2::new(middle(&ADDRESS_COLUMNS), ADDRESS_ROW - LABEL_GAP),
        DOT,
    );
    plate.text(
        &word("panel-wait"),
        Vec2::new(0.09, ADDRESS_ROW - LABEL_GAP),
        DOT,
    );
    plate.text(
        &word("panel-data"),
        Vec2::new(middle(&BIT_COLUMNS), DATA_ROW - LABEL_GAP),
        DOT,
    );
    for (index, &x) in BIT_COLUMNS.iter().enumerate() {
        plate.text(
            &(3 - index).to_string(),
            Vec2::new(x, SWITCH_ROW + LABEL_GAP),
            DOT,
        );
    }
    let left = BIT_COLUMNS[0] - 0.008;
    let right = BIT_COLUMNS[3] + 0.008;
    plate.stroke(
        Vec2::new(left, SWITCH_ROW - 0.009),
        Vec2::new(right, SWITCH_ROW - 0.009),
        LINE,
    );
    for (control, at) in controls() {
        let id = match control {
            Control::Data(_) => continue,
            Control::Run => "panel-run",
            Control::Reset => "panel-reset",
            Control::Erase => "panel-erase",
            Control::Next => "panel-next",
            Control::Deposit => "panel-deposit",
            Control::Step => "panel-step",
        };
        plate.text(&word(id), at + Vec2::Y * LABEL_GAP * 1.1, DOT);
    }
    plate.image()
}
