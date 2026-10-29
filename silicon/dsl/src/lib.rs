//! Compiler for the level firmware language. See `docs/dsl.md`.
//!
//! One source file describes a firmware. `bug`, `fix` and `change` switches
//! pick the buggy and the fixed version from it. The output is assembly that
//! looks hand written.
//!
//! ```
//! use ld60_calco::{ChipInfo, DateCode, Revision};
//! use ld60_devices::{BoardRev, ChipSpec, DeviceKind};
//! use ld60_dsl::{Flavor, Selection, Target, compile};
//!
//! let chip = ChipSpec::Calco(ChipInfo {
//!     revision: Revision::B,
//!     date_code: DateCode::new(76, 30),
//! });
//! let target = Target::new(chip, DeviceKind::Tally2.board(BoardRev::A));
//! let source = "firmware \"Demo\" for tally2\nentry main {\n    wait\n    goto main\n}\n";
//! let selection = Selection {
//!     flavor: Flavor::Fixed,
//!     switches: &[],
//! };
//! let output = compile(source, &target, &selection).unwrap();
//! assert!(output.asm.contains("main:   WAIT"));
//! ```

mod ast;
mod backend;
mod diagnostic;
mod emit;
mod hir;
mod labels;
mod legalize;
mod lexer;
mod lint;
mod literal;
mod parser;
mod print;
mod resolve;
mod switch;
mod target;

pub use diagnostic::{Diagnostic, Severity};
pub use target::Target;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Flavor {
    Buggy,
    Fixed,
}

/// Which firmware to build and which switches the level turns on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection<'a> {
    pub flavor: Flavor,
    pub switches: &'a [String],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub asm: String,
    pub warnings: Vec<Diagnostic>,
}

/// The buggy and the fixed firmware of one level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    pub buggy: Output,
    pub fixed: Output,
}

/// Compiles one firmware. The output always assembles for the target chip.
pub fn compile(
    source: &str,
    target: &Target,
    selection: &Selection<'_>,
) -> Result<Output, Vec<Diagnostic>> {
    let backend = target.backend.as_ref();
    let file = parser::parse(source).map_err(|e| vec![e])?;
    let file = switch::select(file, selection).map_err(|e| vec![e])?;
    let program = resolve::resolve(&file, target)?;
    lint::recursion(&program)?;
    let program = legalize::legalize(program, backend)?;
    let mut warnings = program.warnings.clone();
    warnings.extend(lint::lint(&program, backend.features(), selection.flavor)?);
    let emitted = emit::emit(&program, backend).map_err(|e| vec![e])?;
    let names = emitted.labels.name(&emitted.lines, backend);
    check_size(&emitted.lines, backend.features().rom_len)?;
    let printed = print::print(&program, &emitted.lines, &names, backend);
    if let Err(errors) = target.chip.isa().assemble(&printed.text) {
        return Err(errors
            .into_iter()
            .map(|e| {
                let pos = printed.positions.get(e.line.wrapping_sub(1));
                Diagnostic::error(
                    pos.copied().unwrap_or_default(),
                    format!("generated code: {}", e.kind),
                )
            })
            .collect());
    }
    let asm = printed.text;
    Ok(Output { asm, warnings })
}

/// Compiles the buggy and the fixed firmware.
///
/// Fails if a switch name does not exist in the source. Error messages start
/// with the firmware that failed.
pub fn compile_pair(
    source: &str,
    target: &Target,
    switches: &[String],
) -> Result<Pair, Vec<Diagnostic>> {
    let build = |flavor| {
        compile(source, target, &Selection { flavor, switches }).map_err(|errors| {
            let name = match flavor {
                Flavor::Buggy => "buggy",
                Flavor::Fixed => "fixed",
            };
            errors
                .into_iter()
                .map(|mut e| {
                    e.message = format!("{name} firmware: {}", e.message);
                    e
                })
                .collect::<Vec<_>>()
        })
    };
    Ok(Pair {
        buggy: build(Flavor::Buggy)?,
        fixed: build(Flavor::Fixed)?,
    })
}

/// Fails with one error at the first instruction that does not fit in the ROM.
fn check_size(lines: &[emit::Line], rom_len: usize) -> Result<(), Vec<Diagnostic>> {
    let instrs = lines.iter().filter_map(|line| match line {
        emit::Line::Instr(instr) => Some(instr),
        _ => None,
    });
    let total: usize = instrs.clone().map(|i| i.size).sum();
    if total <= rom_len {
        return Ok(());
    }
    let mut used = 0;
    let first = instrs
        .clone()
        .find(|i| {
            used += i.size;
            used > rom_len
        })
        .map(|i| i.pos)
        .unwrap_or_default();
    Err(vec![Diagnostic::error(
        first,
        format!("firmware is {total} bytes, the ROM holds {rom_len}"),
    )])
}
