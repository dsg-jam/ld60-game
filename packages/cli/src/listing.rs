use std::io::Write;

use devices::IsaKind;

use crate::render::hex_words;

pub fn run(out: &mut impl Write, path: &str) -> crate::Result<()> {
    let source = std::fs::read_to_string(path)?;
    let assembly = IsaKind::Calco.isa().assemble(&source).map_err(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| format!("{path}: {e}")).collect();
        lines.join("\n")
    })?;
    let text: Vec<&str> = source.lines().collect();
    writeln!(out, "ADDR  BYTES  LINE  SOURCE")?;
    for line in &assembly.lines {
        let start = usize::from(line.addr);
        let words = &assembly.image.words()[start..start + usize::from(line.len)];
        writeln!(
            out,
            "{:04X}  {:<5}  {:>4}  {}",
            line.addr,
            hex_words(words, assembly.image.word_bits()),
            line.line,
            text[line.line - 1].trim_end()
        )?;
    }
    let used: u16 = assembly.lines.iter().map(|l| l.len).sum();
    writeln!(out, "\n{used} of {} words used", assembly.image.len())?;
    Ok(())
}
