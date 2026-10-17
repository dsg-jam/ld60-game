use std::io::Write;

use levels::CATALOG;

pub fn run(out: &mut impl Write) -> crate::Result<()> {
    for source in CATALOG {
        let level = source.load()?;
        writeln!(out, "{:<28} {}", source.path, level.title)?;
        writeln!(out, "{:<28} \"{}\"", "", level.customer.complaint)?;
    }
    Ok(())
}
