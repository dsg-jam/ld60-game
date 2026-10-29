use std::io::Write;

use ld60_levels::Level;

use crate::text::LevelText;

pub fn run(out: &mut impl Write) -> crate::Result<()> {
    let text = LevelText::read()?;
    for source in crate::catalog()?.levels() {
        let level = source.load()?;
        let boards: Vec<String> = level
            .allowed_boards()
            .into_iter()
            .map(|rev| level.device.board(rev).name())
            .collect();
        writeln!(out, "{:<28} {}", source.path, level.title)?;
        writeln!(
            out,
            "{:<28} \"{}\"",
            "",
            text.get(&level, Level::COMPLAINT)?
        )?;
        writeln!(out, "{:<28} Boards {}", "", boards.join(" "))?;
    }
    Ok(())
}
