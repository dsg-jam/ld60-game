use std::io::{BufRead, Write};

use ld60_machine::Step;

use crate::render;
use crate::{Session, Stop};

const HELP: &str = "\
Commands:
  k <keys>     Press keys, for example: k 12+34=
  s [n]        Step n instructions (default 1)
  r            Run until HLT or a breakpoint
  b <addr>     Toggle a breakpoint at an address or label
  l            Show the code around the program counter
  on [seed]    Power on again, with new RAM garbage
  burn <file>  Assemble a file and put it on the chip
  t            Run the level tests on the current chip
  h            Show this help
  q            Quit";

pub fn run(session: &mut Session, input: impl BufRead, out: &mut impl Write) -> crate::Result<()> {
    let level = &session.level;
    writeln!(out, "{} {}", level.id, level.title)?;
    writeln!(
        out,
        "{}: \"{}\"",
        level.customer.name, level.customer.complaint
    )?;
    writeln!(out, "Type h for help.")?;
    render::state(out, session)?;

    let mut lines = input.lines();
    loop {
        write!(out, "> ")?;
        out.flush()?;
        let Some(line) = lines.next().transpose()? else {
            break;
        };
        let line = line.trim();
        let (command, arg) = line.split_once(' ').unwrap_or((line, ""));
        let arg = arg.trim();
        match command {
            "" => {}
            "q" | "quit" => break,
            "h" | "help" => writeln!(out, "{HELP}")?,
            "l" => render::listing(out, session, 6)?,
            "b" => toggle_breakpoint(out, session, arg)?,
            "t" => match session.test() {
                Ok(report) => write!(out, "{report}")?,
                Err(error) => writeln!(out, "error: {error}")?,
            },
            "k" | "s" | "r" | "on" | "burn" => {
                match execute(session, command, arg) {
                    Ok(Some(stop)) => render::stop(out, stop)?,
                    Ok(None) => {}
                    Err(error) => writeln!(out, "error: {error}")?,
                }
                render::state(out, session)?;
            }
            _ => writeln!(out, "Unknown command '{command}'. Type h for help.")?,
        }
    }
    Ok(())
}

fn execute(session: &mut Session, command: &str, arg: &str) -> crate::Result<Option<Stop>> {
    Ok(match command {
        "k" => Some(session.press(arg)?),
        "s" => {
            let count: u32 = if arg.is_empty() { 1 } else { arg.parse()? };
            (0..count)
                .any(|_| session.step() == Step::Halted)
                .then_some(Stop::Halted)
        }
        "r" => Some(session.run()),
        "on" => {
            let seed = if arg.is_empty() {
                crate::seed::random()
            } else {
                arg.parse()?
            };
            Some(session.power_on(seed))
        }
        _ => Some(session.burn(std::fs::read_to_string(arg)?)?),
    })
}

fn toggle_breakpoint(out: &mut impl Write, session: &mut Session, arg: &str) -> crate::Result<()> {
    let Some(addr) = parse_addr(arg).or_else(|| session.label(arg)) else {
        writeln!(out, "Unknown address or label '{arg}'.")?;
        return Ok(());
    };
    if session.breakpoints.insert(addr) {
        writeln!(out, "Breakpoint set at 0x{addr:02X}.")?;
    } else {
        session.breakpoints.remove(&addr);
        writeln!(out, "Breakpoint removed at 0x{addr:02X}.")?;
    }
    Ok(())
}

fn parse_addr(text: &str) -> Option<u16> {
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u16::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}
