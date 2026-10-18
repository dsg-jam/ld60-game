use std::io::Write;

use ld60_levels::CATALOG;

pub fn run(out: &mut impl Write) -> crate::Result<()> {
    for source in CATALOG {
        let level = source.load()?;
        let boards: Vec<String> = level
            .allowed_boards()
            .into_iter()
            .map(|rev| level.device.board(rev).name())
            .collect();
        writeln!(out, "{:<28} {}", source.path, level.title)?;
        writeln!(out, "{:<28} \"{}\"", "", level.customer.complaint)?;
        writeln!(out, "{:<28} Boards {}", "", boards.join(" "))?;
    }
    Ok(())
}
