use std::io::Write;
use std::path::{Path, PathBuf};

use ld60_dsl::{Target, compile_pair};
use ld60_levels::{Catalog, Level, LevelSource};

/// Regenerates `buggy.asm` and `fixed.asm` of every level with a firmware
/// source in `data_dir`.
///
/// Every level compiles before any file is written, so a failure leaves all
/// files as they were. With `check`, nothing is written. Returns whether all
/// files were current.
pub fn run(out: &mut impl Write, data_dir: &Path, check: bool) -> crate::Result<bool> {
    let mut files: Vec<(String, PathBuf, String)> = Vec::new();
    let mut errors = Vec::new();
    let order = Catalog::order(&std::fs::read_to_string(data_dir.join(Catalog::FILE))?)?;
    for path in order {
        let file = |name| data_dir.join(LevelSource::file(&path, name));
        let level = Level::from_ron(&std::fs::read_to_string(file(LevelSource::LEVEL_FILE))?)?;
        let Some(firmware) = &level.firmware else {
            continue;
        };
        let text = std::fs::read_to_string(data_dir.join(&firmware.source))?;
        let target = Target::new(level.chip, level.board());
        let pair = match compile_pair(&text, &target, &firmware.switches) {
            Ok(pair) => pair,
            Err(failures) => {
                errors.extend(
                    failures
                        .iter()
                        .map(|e| format!("{path}: {}:{e}", firmware.source)),
                );
                continue;
            }
        };
        for (name, output) in [
            (LevelSource::BUGGY_FILE, pair.buggy),
            (LevelSource::FIXED_FILE, pair.fixed),
        ] {
            let shown = LevelSource::file(&path, name);
            for warning in &output.warnings {
                writeln!(out, "{shown}: {}:{warning}", firmware.source)?;
            }
            files.push((shown, file(name), output.asm));
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("\n").into());
    }
    let mut current = true;
    for (shown, path, asm) in files {
        if std::fs::read_to_string(&path).ok().as_deref() == Some(asm.as_str()) {
            continue;
        }
        current = false;
        if check {
            writeln!(out, "{shown} is stale")?;
        } else {
            std::fs::write(&path, &asm)?;
            writeln!(out, "{shown} written")?;
        }
    }
    Ok(current)
}
