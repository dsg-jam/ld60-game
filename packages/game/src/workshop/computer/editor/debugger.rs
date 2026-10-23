use bevy::prelude::*;
use ld60_locale::{Localized, Message};
use ld60_machine::{Cpu, Step};

use super::PHOSPHOR;
use super::footer::{Command, Run, Status};
use super::listing::Listing;
use crate::device::{Emulator, Stop};
use crate::workshop::run::{ComputerTool, Run as ShopRun};

/// RAM cells the panel shows.
const RAM_CELLS: usize = 16;

/// The registers and RAM of the held chip, above the status line.
#[derive(Component, Debug)]
pub(super) struct DebugPanel;

/// The panel line with the registers.
#[derive(Component, Debug)]
pub(super) struct RegisterLine;

/// The panel line with the RAM cells.
#[derive(Component, Debug)]
pub(super) struct RamLine;

impl DebugPanel {
    /// The panel, hidden until the debugger holds the chip.
    pub(super) fn bundle(font: TextFont) -> impl Bundle {
        (
            DebugPanel,
            Node {
                flex_direction: FlexDirection::Column,
                display: Display::None,
                ..default()
            },
            children![
                (
                    RegisterLine,
                    Localized::verbatim(""),
                    font.clone(),
                    TextColor(PHOSPHOR)
                ),
                (RamLine, Localized::verbatim(""), font, TextColor(PHOSPHOR)),
            ],
        )
    }

    /// Each register as its name and hex value, like `A 0F  Z 1`.
    pub(super) fn registers(cpu: &dyn Cpu) -> String {
        cpu.registers()
            .iter()
            .map(|register| {
                let digits = usize::from(register.bits).div_ceil(4);
                format!("{} {:0digits$X}", register.name, register.value)
            })
            .collect::<Vec<_>>()
            .join("  ")
    }

    /// The first [`RAM_CELLS`] cells in hex.
    pub(super) fn cells(cpu: &dyn Cpu) -> String {
        cpu.ram()
            .cells()
            .iter()
            .take(RAM_CELLS)
            .map(|cell| format!("{cell:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(super) fn update(
        shop: Res<ShopRun>,
        emulators: Query<&Emulator>,
        mut panel: Single<&mut Node, With<DebugPanel>>,
        mut registers: Single<&mut Localized, With<RegisterLine>>,
        mut ram: Single<&mut Localized, (With<RamLine>, Without<RegisterLine>)>,
    ) {
        let held = emulators
            .single()
            .ok()
            .filter(|emulator| emulator.held() && shop.has(ComputerTool::Debugger));
        let Some(emulator) = held else {
            if panel.display != Display::None {
                panel.display = Display::None;
            }
            return;
        };
        if panel.display != Display::Flex {
            panel.display = Display::Flex;
        }
        let cpu = emulator.cpu();
        registers.set_if_neq(Localized::verbatim(Self::registers(cpu)));
        let cells = Message::new("editor-debug-ram").with("cells", Self::cells(cpu));
        ram.set_if_neq(cells.into());
    }

    /// Steps and releases the chip from the footer.
    pub(super) fn run_on_command(
        run: On<Run>,
        listing: Res<Listing>,
        mut emulators: Query<&mut Emulator>,
        mut status: Single<&mut Localized, With<Status>>,
    ) {
        if !matches!(run.0, Command::Step | Command::Go) {
            return;
        }
        let message = match emulators.single_mut() {
            Ok(mut emulator) if run.0 == Command::Step => Self::step(&mut emulator, &listing),
            Ok(mut emulator) => Self::go(&mut emulator, &listing),
            Err(_) => Message::new("editor-no-device"),
        };
        **status = message.into();
    }

    /// Holds a running chip, or runs one instruction of a held one.
    pub(super) fn step(emulator: &mut Emulator, listing: &Listing) -> Message {
        if !emulator.held() {
            emulator.hold();
            return Message::new("editor-debug-held").with("place", Self::place(emulator, listing));
        }
        match emulator.step() {
            Step::Executed => {
                Message::new("editor-debug-stepped").with("place", Self::place(emulator, listing))
            }
            Step::Halted => {
                Message::new("editor-debug-waiting").with("place", Self::place(emulator, listing))
            }
        }
    }

    /// Lets the held chip run on.
    pub(super) fn go(emulator: &mut Emulator, listing: &Listing) -> Message {
        if !emulator.held() {
            return Message::new("editor-debug-not-held");
        }
        match emulator.release() {
            Stop::Halted => Message::new("editor-debug-running"),
            Stop::Break => {
                Message::new("editor-debug-break").with("place", Self::place(emulator, listing))
            }
            Stop::Timeout => Message::new("editor-debug-timeout"),
        }
    }

    /// The source line the chip runs next, or its address if no line has it.
    fn place(emulator: &Emulator, listing: &Listing) -> Message {
        let pc = emulator.cpu().pc();
        match listing.line_at(pc) {
            Some(line) => Message::new("editor-debug-line").with("line", line + 1),
            None => Message::new("editor-debug-address").with("address", format!("0x{pc:02X}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_devices::AnyDevice;
    use ld60_levels::{Level, device, initial_rom};
    use ld60_locale::Locale;

    use super::*;
    use crate::workshop::levels::Levels;

    fn tally() -> (Level, String, AnyDevice) {
        let levels = Levels::on_disk();
        let source = &levels.levels()[0];
        let level = source.load().unwrap();
        let rom = initial_rom(&level, &source.buggy).unwrap();
        let device = device(&level, rom).unwrap();
        (level, source.buggy.clone(), device)
    }

    #[test]
    fn steps_through_the_firmware() {
        let (level, buggy, device) = tally();
        let listing = Listing::new(ld60_levels::assemble(&level, &buggy).unwrap());
        let mut emulator = Emulator::power_on(device);
        let mut messages = vec![
            DebugPanel::go(&mut emulator, &listing),
            DebugPanel::step(&mut emulator, &listing),
        ];
        assert!(emulator.held());
        let pc = emulator.cpu().pc();
        messages.push(DebugPanel::step(&mut emulator, &listing));
        assert_eq!(emulator.cpu().pc(), pc, "a halted chip waits for a key");
        messages.push(DebugPanel::go(&mut emulator, &listing));
        assert!(!emulator.held());
        messages.push(Message::new("editor-debug-break").with(
            "place",
            Message::new("editor-debug-address").with("address", "0x1F"),
        ));
        messages.push(Message::new("editor-debug-timeout"));
        messages.push(
            Message::new("editor-debug-ram").with("cells", DebugPanel::cells(emulator.cpu())),
        );
        Locale::on_disk().assert_formats(messages);
    }

    #[test]
    fn shows_registers_in_hex() {
        let (_, _, device) = tally();
        let emulator = Emulator::power_on(device);
        let registers = DebugPanel::registers(emulator.cpu());
        assert!(registers.starts_with("A "), "{registers}");
        assert!(registers.contains("  Z "), "{registers}");
        assert_eq!(
            DebugPanel::cells(emulator.cpu()).split(' ').count(),
            RAM_CELLS
        );
    }
}
