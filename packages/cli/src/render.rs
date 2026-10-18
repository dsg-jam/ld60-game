use std::io::{self, Write};

use ld60_devices::Device;
use ld60_machine::DisasmLine;

use crate::{Session, Stop};

pub(crate) fn hex_words(words: &[u16], word_bits: u8) -> String {
    let width = usize::from(word_bits).div_ceil(4);
    let parts: Vec<String> = words.iter().map(|w| format!("{w:0width$X}")).collect();
    parts.join(" ")
}

pub(crate) fn stop(out: &mut impl Write, stop: Stop) -> io::Result<()> {
    match stop {
        Stop::Halted => writeln!(out, "Waiting for a key."),
        Stop::Breakpoint(addr) => writeln!(out, "Breakpoint at 0x{addr:02X}."),
        Stop::Timeout => writeln!(out, "No HLT reached. The firmware may be stuck in a loop."),
    }
}

pub(crate) fn state(out: &mut impl Write, session: &Session) -> io::Result<()> {
    let device = &session.device;
    let cpu = device.cpu();
    writeln!(out, "\n  [ {} ]\n", device.readout())?;

    let registers: Vec<String> = cpu
        .registers()
        .iter()
        .map(|r| {
            let width = usize::from(r.bits).div_ceil(4);
            format!("{}={:0width$X}", r.name, r.value)
        })
        .collect();
    writeln!(out, "  {}", registers.join(" "))?;

    let cells = cpu.ram().cells();
    let names: Vec<String> = (0..cells.len()).map(|i| format!("M{i:X}")).collect();
    let values: Vec<String> = cells.iter().map(|c| format!("{c:02X}")).collect();
    writeln!(out, "       {}", names.join(" "))?;
    writeln!(out, "  RAM  {}", values.join(" "))?;

    let lines = disassembly(session);
    if let Some(index) = line_index(&lines, cpu.pc()) {
        writeln!(out)?;
        instruction(out, session, &lines[index], true)?;
    }
    Ok(())
}

/// Disassembly around the program counter, with source lines and breakpoints.
pub(crate) fn listing(out: &mut impl Write, session: &Session, radius: usize) -> io::Result<()> {
    let lines = disassembly(session);
    let pc = session.device.cpu().pc();
    let center = line_index(&lines, pc).unwrap_or_default();
    let start = center.saturating_sub(radius);
    let end = (center + radius + 1).min(lines.len());
    for line in &lines[start..end] {
        instruction(out, session, line, line.addr == pc)?;
    }
    Ok(())
}

fn instruction(
    out: &mut impl Write,
    session: &Session,
    line: &DisasmLine,
    current: bool,
) -> io::Result<()> {
    let marker = match (current, session.breakpoints.contains(&line.addr)) {
        (true, _) => "->",
        (false, true) => " *",
        (false, false) => "  ",
    };
    let bits = session.device.rom().image().word_bits();
    let source = session
        .source_line(line.addr)
        .map(|(n, text)| format!("{n:>4}: {text}"))
        .unwrap_or_default();
    writeln!(
        out,
        "{marker} {:02X}  {:<6} {:<16} {source}",
        line.addr,
        hex_words(&line.words, bits),
        line.text
    )
}

fn disassembly(session: &Session) -> Vec<DisasmLine> {
    let device = &session.device;
    device.chip().isa().disassemble(device.rom().image())
}

fn line_index(lines: &[DisasmLine], addr: u16) -> Option<usize> {
    lines
        .iter()
        .position(|l| (l.addr..l.addr + l.words.len() as u16).contains(&addr))
}
