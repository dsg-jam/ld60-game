use std::io::{self, Write};
use std::process::ExitCode;

const USAGE: &str = "\
Usage:
  cli list                               List all levels
  cli check <level> [file.asm]           Run level tests on the buggy ROM or a file
  cli play <level> [--fixed] [--seed N] [--variant N]
                                         Open a level on the bench
  cli asm <file.asm>                     Print the listing of a CalCo file";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut out = io::stdout().lock();
    let result = match args.as_slice() {
        ["list"] => ld60_cli::list::run(&mut out).map(|()| true),
        ["check", level] => ld60_cli::check::run(&mut out, level, None),
        ["check", level, file] => ld60_cli::check::run(&mut out, level, Some(file)),
        ["play", level, options @ ..] => play(&mut out, level, options).map(|()| true),
        ["asm", file] => ld60_cli::listing::run(&mut out, file).map(|()| true),
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
