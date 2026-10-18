use std::io::{self, Write};
use std::process::ExitCode;

use ld60_calco::Revision;

const USAGE: &str = "\
Usage:
  cli list                               List all levels
  cli check <level> [file.asm]           Run level tests on the buggy ROM or a file
  cli play <level> [--fixed] [--seed N] [--variant N]
                                         Open a level on the bench
  cli asm <file.asm> [--rev A|B|C]       Print the listing of a CalCo file (default Rev C)
  cli gen [--check]                      Regenerate level firmware from its source";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut out = io::stdout().lock();
    let result = match args.as_slice() {
        ["list"] => ld60_cli::list::run(&mut out).map(|()| true),
        ["check", level] => ld60_cli::check::run(&mut out, level, None),
        ["check", level, file] => ld60_cli::check::run(&mut out, level, Some(file)),
        ["play", level, options @ ..] => play(&mut out, level, options).map(|()| true),
        ["asm", file] => listing(&mut out, file, "C").map(|()| true),
        ["asm", file, "--rev", revision] | ["asm", "--rev", revision, file] => {
            listing(&mut out, file, revision).map(|()| true)
        }
        ["gen"] => generate(&mut out, false),
        ["gen", "--check"] => generate(&mut out, true),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let _ = out.flush();
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) if is_broken_pipe(error.as_ref()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn is_broken_pipe(error: &(dyn std::error::Error + 'static)) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe)
}

fn generate(out: &mut impl Write, check: bool) -> ld60_cli::Result<bool> {
    let data_dir = std::path::Path::new(ld60_levels::DATA_DIR);
    ld60_cli::generate::run(out, data_dir, check)
}

fn listing(out: &mut impl Write, file: &str, revision: &str) -> ld60_cli::Result<()> {
    let revision = Revision::ALL
        .into_iter()
        .find(|r| r.to_string().eq_ignore_ascii_case(revision))
        .ok_or_else(|| format!("unknown revision '{revision}'"))?;
    ld60_cli::listing::run(out, file, revision.isa())
}

fn play(out: &mut impl Write, level: &str, options: &[&str]) -> ld60_cli::Result<()> {
    let mut fixed = false;
    let mut seed = ld60_cli::seed::random();
    let mut variant = None;
    let mut rest = options.iter();
    while let Some(&option) = rest.next() {
        match option {
            "--fixed" => fixed = true,
            "--seed" => {
                let value = rest.next().ok_or("--seed needs a number")?;
                seed = value.parse()?;
            }
            "--variant" => {
                let value = rest.next().ok_or("--variant needs a number")?;
                variant = Some(value.parse()?);
            }
            other => return Err(format!("unknown option '{other}'").into()),
        }
    }
    let source = ld60_cli::find_level(level)?;
    let variant = match variant {
        Some(n) => ld60_levels::Variant::new(source, n)?,
        None => ld60_levels::Variant::original(source)?,
    };
    let mut session = ld60_cli::Session::new(variant, fixed, seed)?;
    ld60_cli::play::run(&mut session, io::stdin().lock(), out)
}
