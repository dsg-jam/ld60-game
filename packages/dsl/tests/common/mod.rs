#![allow(dead_code)]

use ld60_calco::{ChipInfo, DateCode, Revision};
use ld60_devices::{BoardRev, ChipSpec, DeviceKind};
use ld60_dsl::{Diagnostic, Flavor, Output, Selection, Target, compile};

pub(crate) const TALLY_0_9: &str = include_str!(concat!(
    env!("LD60_ASSETS"),
    "/levels/tally2/firmware-0.9.fw"
));
pub(crate) const TALLY_1_0: &str = include_str!(concat!(
    env!("LD60_ASSETS"),
    "/levels/tally2/firmware-1.0.fw"
));
pub(crate) const SUMMA: &str =
    include_str!(concat!(env!("LD60_ASSETS"), "/levels/summa6/firmware.fw"));

pub(crate) fn target(revision: Revision, device: DeviceKind) -> Target {
    let chip = ChipSpec::Calco(ChipInfo {
        revision,
        date_code: DateCode::new(76, 30),
    });
    Target::new(chip, device.board(BoardRev::A))
}

pub(crate) fn build(
    source: &str,
    revision: Revision,
    device: DeviceKind,
    flavor: Flavor,
    switches: &[&str],
) -> Result<Output, Vec<Diagnostic>> {
    let switches: Vec<String> = switches.iter().map(|s| (*s).into()).collect();
    let selection = Selection {
        flavor,
        switches: &switches,
    };
    compile(source, &target(revision, device), &selection)
}

/// Compiles a Tally 2 firmware for Rev B with no switches.
pub(crate) fn tally(source: &str) -> Result<Output, Vec<Diagnostic>> {
    build(source, Revision::B, DeviceKind::Tally2, Flavor::Fixed, &[])
}

/// The first error message.
pub(crate) fn error(result: Result<Output, Vec<Diagnostic>>) -> Diagnostic {
    match result {
        Ok(output) => panic!("compiled:\n{}", output.asm),
        Err(errors) => errors[0].clone(),
    }
}

/// The code lines after the header.
pub(crate) fn code(output: &Output) -> String {
    let start = output.asm.find("\n\n").map_or(0, |i| i + 2);
    output.asm[start..].to_string()
}
