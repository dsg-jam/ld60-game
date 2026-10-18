//! Service sheets: what the board around the chip does, per board revision.

use bevy::prelude::*;
use ld60_devices::{Annunciator, Board, DeviceKind};

use crate::locale::{Localized, Message};
use crate::parts::paper::Paper;

const KEY_WIDTH: f32 = 120.0;
const CODE_WIDTH: f32 = 120.0;
const STEPS_WIDTH: f32 = 200.0;
const DISPLAY_WIDTH: f32 = 160.0;

pub(super) fn footer(device: DeviceKind) -> Localized {
    Localized::verbatim(match device {
        DeviceKind::Tally2 => "DSG ELECTRONICS   TALLY 2",
        DeviceKind::Summa6 => "DSG ELECTRONICS   SUMMA 6",
    })
}

pub(super) fn write(page: &mut ChildSpawnerCommands, board: &Board) {
    let (title, summary) = match board.kind {
        DeviceKind::Tally2 => ("TALLY 2", "service-summary-tally2"),
        DeviceKind::Summa6 => ("SUMMA 6", "service-summary-summa6"),
    };
    page.spawn(Paper::title(Localized::verbatim(title)));
    page.spawn(Paper::faded(
        Message::new(summary).with("board", board.name()),
    ));

    page.spawn(Paper::heading(Message::new("service-key-codes")));
    key_codes(page, board);

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
            };
            page.spawn(Paper::text(Message::new(id).with("bit", bit)));
        }
    }

    page.spawn(Paper::heading(
        Message::new("service-board").with("board", board.name()),
    ));
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
pub(super) fn write_line_tests(page: &mut ChildSpawnerCommands, device: DeviceKind) {
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
    for test in device.line_tests() {
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
}

/// Digit keys share one row, the other keys get a row each.
fn key_codes(page: &mut ChildSpawnerCommands, board: &Board) {
    let (digits, others): (Vec<_>, Vec<_>) =
        board.keys.iter().partition(|(key, _)| key.is_ascii_digit());
    if let (Some((first, low)), Some((last, high))) = (digits.first(), digits.last()) {
        page.spawn(Paper::row([
            (range(first.to_string(), last.to_string()), KEY_WIDTH),
            (
                range(format!("{low:02X}"), format!("{high:02X}")),
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
