use std::io::Write;
use std::path::Path;

use ld60_dsl::{Target, compile_pair};
use ld60_levels::{CATALOG, Level};

/// Regenerates `buggy.asm` and `fixed.asm` of every level with a firmware
/// source in `data_dir`.
///
/// With `check`, nothing is written. Returns whether all files were current.
pub fn run(out: &mut impl Write, data_dir: &Path, check: bool) -> crate::Result<bool> {
    let mut current = true;
    for source in CATALOG {
        let dir = data_dir.join(source.path);
        let level = Level::from_ron(&std::fs::read_to_string(dir.join("level.ron"))?)?;
        let Some(firmware) = &level.firmware else {
            continue;
        };
        let text = std::fs::read_to_string(data_dir.join(&firmware.source))?;
        let target = Target::new(level.chip, level.device);
        let pair = compile_pair(&text, &target, &firmware.switches).map_err(|errors| {
            let lines: Vec<String> = errors
                .iter()
                .map(|e| format!("{}:{e}", firmware.source))
                .collect();
            lines.join("\n")
        })?;
        for (name, output) in [("buggy.asm", &pair.buggy), ("fixed.asm", &pair.fixed)] {
            let path = dir.join(name);
            for warning in &output.warnings {
                writeln!(out, "{}/{name}: {}:{warning}", source.path, firmware.source)?;
            }
            if std::fs::read_to_string(&path).ok().as_deref() == Some(output.asm.as_str()) {
                continue;
            }
            current = false;
            if check {
                writeln!(out, "{}/{name} is stale", source.path)?;
            } else {
                std::fs::write(&path, &output.asm)?;
                writeln!(out, "{}/{name} written", source.path)?;
            }
        }
    }
    Ok(current)
}
