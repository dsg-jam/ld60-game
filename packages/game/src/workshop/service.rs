//! Service sheets: what the board around the chip does, per device model.

use bevy::prelude::*;
use ld60_devices::DeviceKind;

use crate::parts::paper::Paper;

const KEY_WIDTH: f32 = 120.0;
const CODE_WIDTH: f32 = 120.0;

pub(super) fn footer(device: DeviceKind) -> &'static str {
    match device {
        DeviceKind::Tally2 => "DSG ELECTRONICS   TALLY 2",
        DeviceKind::Summa6 => "DSG ELECTRONICS   SUMMA 6",
    }
}

pub(super) fn write(page: &mut ChildSpawnerCommands, device: DeviceKind) {
    let (title, summary) = match device {
        DeviceKind::Tally2 => ("TALLY 2", "2-digit tally counter. Service sheet."),
        DeviceKind::Summa6 => ("SUMMA 6", "6-digit adding machine. Service sheet."),
    };
    page.spawn(Paper::title(title));
    page.spawn(Paper::faded(summary));

    page.spawn(Paper::heading("KEY CODES"));
    key_codes(page, device);

    let cells = device.display_cells();
    page.spawn(Paper::heading("DISPLAY"));
    page.spawn(Paper::text(format!(
        "{} digits, M{} to M{} from left to right.",
        cells.clone().count(),
        cells.start(),
        cells.end()
    )));
    page.spawn(Paper::text("Values above 9 show broken segments."));

    if device == DeviceKind::Summa6 {
        page.spawn(Paper::heading("ANNUNCIATORS"));
        page.spawn(Paper::text("ANN bit 0 lights the minus sign."));
        page.spawn(Paper::text("ANN bit 1 lights the error mark."));
    }

    page.spawn(Paper::heading("BOARD"));
    page.spawn(Paper::text(
        "CalCo chip, firmware in a 256 byte EPROM socket.",
    ));
    page.spawn(Paper::text("RAM holds random values after power on."));

    page.spawn(Paper::heading("KNOWN FAULTS"));
    page.spawn(Paper::text("- Liquid can leave a RAM bit stuck at 0 or 1."));
    page.spawn(Paper::text("- Old EPROMs can lose bits."));
}

/// Digit keys share one row, the other keys get a row each.
fn key_codes(page: &mut ChildSpawnerCommands, device: DeviceKind) {
    let (digits, others): (Vec<_>, Vec<_>) = device
        .key_codes()
        .iter()
        .partition(|(key, _)| key.is_ascii_digit());
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
