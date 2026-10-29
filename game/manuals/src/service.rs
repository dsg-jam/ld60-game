//! Service sheets: what the board around the chip does, per board revision.

use bevy::prelude::*;
use ld60_devices::{Annunciator, Board, DeviceKind};
use ld60_game_parts::paper::Paper;
use ld60_locale::{Localized, Message};

const KEY_WIDTH: f32 = 120.0;
const CODE_WIDTH: f32 = 120.0;
const STEPS_WIDTH: f32 = 200.0;
const DISPLAY_WIDTH: f32 = 160.0;

/// The tables of a service sheet, from the data of one board revision.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ServiceSheet {
    board: &'static Board,
}

impl ServiceSheet {
    pub(super) fn new(board: &'static Board) -> Self {
        Self { board }
    }

    /// Digit keys share one row, the other keys get a row each.
    pub(super) fn key_codes(self, page: &mut ChildSpawnerCommands) {
        let (digits, others): (Vec<_>, Vec<_>) = self
            .board
            .keys
            .iter()
            .partition(|(key, _)| key.is_ascii_digit());
        if let (Some((first, low)), Some((last, high))) = (digits.first(), digits.last()) {
            page.spawn(Paper::row([
                (Self::range(first.to_string(), last.to_string()), KEY_WIDTH),
                (Self::range(low.to_string(), high.to_string()), CODE_WIDTH),
            ]));
        }
        for (key, code) in others {
            page.spawn(Paper::row([
                (Localized::verbatim(key.to_string()), KEY_WIDTH),
                (Localized::verbatim(code.to_string()), CODE_WIDTH),
            ]));
        }
    }

    /// Which RAM cells the display shows.
    pub(super) fn display_cells(self, page: &mut ChildSpawnerCommands) {
        let cells = self.board.display_cells();
        page.spawn(Paper::text(
            Message::new("service-display-cells")
                .with("digits", cells.clone().count())
                .with("first", format!("M{:X}", cells.start()))
                .with("last", format!("M{:X}", cells.end())),
        ));
    }

    /// The bits that light the marks of the display, under a heading, if the
    /// board has any.
    pub(super) fn annunciators(self, page: &mut ChildSpawnerCommands) {
        if self.board.annunciators.is_empty() {
            return;
        }
        page.spawn(Paper::heading(Message::new("service-annunciators")));
        for &(annunciator, bit) in self.board.annunciators {
            let id = match annunciator {
                Annunciator::Minus => "service-annunciator-minus",
                Annunciator::Error => "service-annunciator-error",
                Annunciator::Open => "service-annunciator-open",
                Annunciator::Shut => "service-annunciator-shut",
            };
            page.spawn(Paper::text(Message::new(id).with("bit", bit)));
        }
    }

    /// The factory checklist of `kind`: keys to press and what the display
    /// then shows.
    pub(super) fn line_tests(kind: DeviceKind, page: &mut ChildSpawnerCommands) {
        page.spawn(Paper::row([
            (Message::new("service-line-tests-keys").into(), STEPS_WIDTH),
            (
                Message::new("service-line-tests-display").into(),
                DISPLAY_WIDTH,
            ),
        ]));
        for test in kind.line_tests() {
            let steps = if test.keys.is_empty() {
                Message::new("service-line-tests-power-on").into()
            } else {
                Localized::verbatim(test.keys)
            };
            page.spawn(Paper::row([
                (steps, STEPS_WIDTH),
                (Localized::verbatim(test.display), DISPLAY_WIDTH),
            ]));
        }
    }

    fn range(first: String, last: String) -> Localized {
        Message::new("service-range")
            .with("first", first)
            .with("last", last)
            .into()
    }
}
