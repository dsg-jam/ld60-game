use std::io::Write;
use std::path::{Path, PathBuf};

use ld60_devices::{ChipSpec, DeviceKind};
use ld60_levels::{Difficulty, Generated, Level, LevelSource, Reference};

use crate::LEVELS;
use crate::text::LevelText;

/// Where `cli gen --procedural` writes a job if no folder is given.
pub fn default_folder(device: DeviceKind, seed: u64, job: u32) -> PathBuf {
    let name = format!("{device:?}-{seed}-{job}").to_lowercase();
    std::env::temp_dir().join("ld60-procedural").join(name)
}

/// Generates job number `job` for `device` from `seed` and writes its level
/// folder to `folder`. Prints what the customer says and where the bugs are.
pub fn run(
    out: &mut impl Write,
    device: DeviceKind,
    seed: u64,
    job: u32,
    folder: &Path,
) -> crate::Result<Generated> {
    let difficulty = Difficulty::for_job(job, device);
    let source = std::fs::read_to_string(Path::new(LEVELS).join(Reference::source(device)))?;
    let reference = Reference::compile(device, difficulty.chip(), &source)?;
    let generated = Generated::new(&reference, seed, &difficulty)?;
    let variant = &generated.variant;
    let level = &variant.level;
    std::fs::create_dir_all(folder)?;
    for (name, text) in [
        (LevelSource::LEVEL_FILE, level.to_ron()?),
        (LevelSource::BUGGY_FILE, variant.buggy.clone()),
        (LevelSource::FIXED_FILE, variant.fixed.clone()),
    ] {
        std::fs::write(folder.join(name), text)?;
    }

    let text = LevelText::read()?;
    writeln!(
        out,
        "{}: \"{}\"",
        text.get(level, Level::CUSTOMER)?,
        text.get(level, Level::COMPLAINT)?
    )?;
    for remark in &level.remarks {
        writeln!(out, "  - {}", text.get(level, &Level::remark(remark))?)?;
    }
    let date_code = match level.chip {
        ChipSpec::Calco(info) => info.date_code,
    };
    writeln!(
        out,
        "Board {}, {} date code {date_code}, reward ${}",
        level.board().name(),
        level.chip.isa().name(),
        level.reward
    )?;
    for bug in &generated.bugs {
        writeln!(
            out,
            "Bug: {:?} on line {}, {:?} after '{}'",
            bug.operator,
            bug.line + 1,
            bug.symptom,
            level.tests[bug.test].keys
        )?;
    }
    writeln!(out, "Written to {}", folder.display())?;
    Ok(generated)
}
