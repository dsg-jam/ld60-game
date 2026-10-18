//! Service sheets: what the board around the chip does, per board revision.

use bevy::prelude::*;
use ld60_devices::{Annunciator, Board, DeviceKind};

use crate::parts::paper::Paper;

const KEY_WIDTH: f32 = 120.0;
const CODE_WIDTH: f32 = 120.0;

pub(super) fn footer(device: DeviceKind) -> &'static str {
    match device {
        DeviceKind::Tally2 => "DSG ELECTRONICS   TALLY 2",
        DeviceKind::Summa6 => "DSG ELECTRONICS   SUMMA 6",
    }
}

pub(super) fn write(page: &mut ChildSpawnerCommands, board: &Board) {
    let (title, summary) = match board.kind {
        DeviceKind::Tally2 => ("TALLY 2", "2-digit tally counter"),
        DeviceKind::Summa6 => ("SUMMA 6", "6-digit adding machine"),
    };
    page.spawn(Paper::title(title));
    page.spawn(Paper::faded(format!(
        "{summary}. Service sheet for board {}.",
        board.name()
    )));

    page.spawn(Paper::heading("KEY CODES"));
    key_codes(page, board);

    let cells = board.display_cells();
    page.spawn(Paper::heading("DISPLAY"));
    page.spawn(Paper::text(format!(
        "{} digits, M{:X} to M{:X} from left to right.",
        cells.clone().count(),
        cells.start(),
        cells.end()
    )));
    page.spawn(Paper::text("Values above 9 show broken segments."));

    if !board.annunciators.is_empty() {
        page.spawn(Paper::heading("ANNUNCIATORS"));
        for &(annunciator, bit) in board.annunciators {
            let name = match annunciator {
                Annunciator::Minus => "minus sign",
                Annunciator::Error => "error mark",
            };
            page.spawn(Paper::text(format!("ANN bit {bit} lights the {name}.")));
        }
    }

    page.spawn(Paper::heading(format!("BOARD {}", board.name())));
    page.spawn(Paper::text(
        "CalCo chip, firmware in a 256 byte EPROM socket.",
    ));
    page.spawn(Paper::text("RAM holds random values after power on."));

    page.spawn(Paper::heading("KNOWN FAULTS"));
    page.spawn(Paper::text("- Liquid can leave a RAM bit stuck at 0 or 1."));
    page.spawn(Paper::text("- Old EPROMs can lose bits."));
}

/// Digit keys share one row, the other keys get a row each.
fn key_codes(page: &mut ChildSpawnerCommands, board: &Board) {
    let (digits, others): (Vec<_>, Vec<_>) =
        board.keys.iter().partition(|(key, _)| key.is_ascii_digit());
    if let (Some((first, low)), Some((last, high))) = (digits.first(), digits.last()) {
        page.spawn(Paper::row([
            (format!("{first} to {last}"), KEY_WIDTH),
            (format!("{low:02X} to {high:02X}"), CODE_WIDTH),
        ]));
    }
    for (key, code) in others {
        page.spawn(Paper::row([
            (key.to_string(), KEY_WIDTH),
            (format!("{code:02X}"), CODE_WIDTH),
        ]));
    }
}
