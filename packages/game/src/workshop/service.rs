//! Service sheets: what the board around the chip does, per board revision.

use bevy::prelude::*;
use ld60_devices::{Annunciator, Board, DeviceKind};

use crate::device::DeviceName;
use crate::locale::{Localized, Message};
use crate::parts::paper::Paper;

const KEY_WIDTH: f32 = 120.0;
const CODE_WIDTH: f32 = 120.0;
const STEPS_WIDTH: f32 = 200.0;
const DISPLAY_WIDTH: f32 = 160.0;

/// The service sheet of one board revision.
#[derive(Clone, Copy, Debug)]
pub(super) struct ServiceSheet {
    board: &'static Board,
}

impl ServiceSheet {
    pub(super) fn new(board: &'static Board) -> Self {
        Self { board }
    }

    pub(super) fn footer(self) -> Message {
        Message::new("service-footer").with("device", self.board.kind.model())
    }

    /// Like `2B`.
    pub(super) fn board_name(self) -> Localized {
        Localized::verbatim(self.board.name())
    }

    /// Like `BOARD 2B`.
    pub(super) fn board_heading(self) -> Message {
        Message::new("service-board").with("board", self.board.name())
    }

    pub(super) fn write(self, page: &mut ChildSpawnerCommands) {
        let board = self.board;
        let summary = match board.kind {
            DeviceKind::Tally2 => "service-summary-tally2",
            DeviceKind::Summa6 => "service-summary-summa6",
            DeviceKind::Lock4 => "service-summary-lock4",
        };
        page.spawn(Paper::title(
            Message::new("service-title")
                .with("device", board.kind.model())
                .with("board", board.name()),
        ));
        page.spawn(Paper::faded(
            Message::new(summary).with("board", board.name()),
        ));

        page.spawn(Paper::heading(Message::new("service-key-codes")));
        self.key_codes(page);

        let cells = board.display_cells();
        page.spawn(Paper::heading(Message::new("service-display")));
        page.spawn(Paper::text(
            Message::new("service-display-cells")
                .with("digits", cells.clone().count())
                .with("first", format!("M{:X}", cells.start()))
                .with("last", format!("M{:X}", cells.end())),
        ));
        page.spawn(Paper::text(Message::new("service-display-broken")));

        if !board.annunciators.is_empty() {
            page.spawn(Paper::heading(Message::new("service-annunciators")));
            for &(annunciator, bit) in board.annunciators {
                let id = match annunciator {
                    Annunciator::Minus => "service-annunciator-minus",
                    Annunciator::Error => "service-annunciator-error",
                    Annunciator::Open => "service-annunciator-open",
                    Annunciator::Shut => "service-annunciator-shut",
                };
                page.spawn(Paper::text(Message::new(id).with("bit", bit)));
            }
        }

        page.spawn(Paper::heading(self.board_heading()));
        page.spawn(Paper::text(Message::new("service-board-chip")));
        page.spawn(Paper::text(Message::new("service-board-ram")));

        page.spawn(Paper::heading(Message::new("service-known-faults")));
        for fault in ["service-fault-liquid", "service-fault-eprom"] {
            page.spawn(Paper::text(
                Message::new("paper-bullet").with("text", Message::new(fault)),
            ));
        }
    }

    /// The factory checklist, on its own page.
    pub(super) fn write_line_tests(self, page: &mut ChildSpawnerCommands) {
        page.spawn(Paper::title(Message::new("service-line-tests")));
        page.spawn(Paper::faded(Message::new("service-line-tests-factory")));
        page.spawn(Paper::text(Message::new("service-line-tests-how")));
        page.spawn(Paper::text(Message::new("service-line-tests-spaces")));
        page.spawn(Paper::row([
            (Message::new("service-line-tests-keys").into(), STEPS_WIDTH),
            (
                Message::new("service-line-tests-display").into(),
                DISPLAY_WIDTH,
            ),
        ]));
        for test in self.board.kind.line_tests() {
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
        page.spawn(Paper::faded(Message::new("service-line-tests-flash")));

        let usage = self.usage();
        if !usage.is_empty() {
            page.spawn(Paper::heading(Message::new("service-use")));
            for &id in usage {
                page.spawn(Paper::text(Message::new(id)));
            }
        }
    }

    /// How to use a model whose keys do not explain themselves.
    fn usage(self) -> &'static [&'static str] {
        match self.board.kind {
            DeviceKind::Tally2 | DeviceKind::Summa6 => &[],
            DeviceKind::Lock4 => &[
                "service-use-lock4-open",
                "service-use-lock4-entry",
                "service-use-lock4-wrong",
                "service-use-lock4-clear",
                "service-use-lock4-change",
            ],
        }
    }

    /// Digit keys share one row, the other keys get a row each.
    fn key_codes(self, page: &mut ChildSpawnerCommands) {
        let (digits, others): (Vec<_>, Vec<_>) = self
            .board
            .keys
            .iter()
            .partition(|(key, _)| key.is_ascii_digit());
        if let (Some((first, low)), Some((last, high))) = (digits.first(), digits.last()) {
            page.spawn(Paper::row([
                (Self::range(first.to_string(), last.to_string()), KEY_WIDTH),
                (
                    Self::range(format!("{low:02X}"), format!("{high:02X}")),
                    CODE_WIDTH,
                ),
            ]));
        }
        for (key, code) in others {
            page.spawn(Paper::row([
                (Localized::verbatim(key.to_string()), KEY_WIDTH),
                (Localized::verbatim(format!("{code:02X}")), CODE_WIDTH),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::Locale;

    #[test]
    fn every_sheet_formats() {
        let locale = Locale::on_disk();
        for kind in DeviceKind::ALL {
            for board in kind.boards() {
                let sheet = ServiceSheet::new(board);
                locale.assert_formats([sheet.footer()]);
                locale.assert_formats(Localized::spawned_by(|page| sheet.write(page)));
                locale.assert_formats(Localized::spawned_by(|page| sheet.write_line_tests(page)));
            }
        }
    }
}
