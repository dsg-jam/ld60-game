use std::io::{self, Write};
use std::process::ExitCode;

use ld60_calco::Revision;

const USAGE: &str = "\
Usage:
  cli list                               List all levels and their boards
  cli check <level> [file.asm] [--variant N] [--board R]
                                         Run level tests on the buggy ROM or a file
  cli play <level> [--fixed] [--seed N] [--variant N] [--board R]
                                         Open a level on the bench
  cli gen --procedural <device> <seed> <job> [--out DIR]
                                         Generate job number <job> for tally2,
                                         summa6 or lock4 and write its level folder
  cli asm <file.asm> [--rev A|B|C]       Print the listing of a CalCo file (default Rev C)
  cli gen [--check]                      Regenerate level firmware from its source

--board R moves the level to board revision R (A, B or C).
<level> is a level from 'cli list' or a level folder, like one from
'cli gen --procedural'.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut out = io::stdout().lock();
    let result = match args.as_slice() {
        ["list"] => ld60_cli::list::run(&mut out).map(|()| true),
        ["check", level, options @ ..] => check(&mut out, level, options),
        ["play", level, options @ ..] => play(&mut out, level, options).map(|()| true),
        ["asm", file] => listing(&mut out, file, "C").map(|()| true),
        ["asm", file, "--rev", revision] | ["asm", "--rev", revision, file] => {
            listing(&mut out, file, revision).map(|()| true)
        }
        ["gen"] => generate(&mut out, false),
        ["gen", "--check"] => generate(&mut out, true),
        ["gen", "--procedural", device, seed, job, options @ ..] => {
            procedural(&mut out, device, seed, job, options).map(|()| true)
        }
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

fn check(out: &mut impl Write, level: &str, options: &[&str]) -> ld60_cli::Result<bool> {
    let mut file = None;
    let mut variant = None;
    let mut board = None;
    let mut rest = options.iter();
    while let Some(&option) = rest.next() {
        match option {
            "--variant" => variant = Some(value(&mut rest, option)?.parse()?),
            "--board" => board = Some(value(&mut rest, option)?.parse()?),
            path if file.is_none() && !path.starts_with("--") => file = Some(path),
            other => return Err(format!("unknown option '{other}'").into()),
        }
    }
    let variant = ld60_cli::find_variant(level, variant, board)?;
    ld60_cli::check::run(out, &variant, file)
}

fn generate(out: &mut impl Write, check: bool) -> ld60_cli::Result<bool> {
    let data_dir = std::path::Path::new(ld60_cli::LEVELS);
    ld60_cli::generate::run(out, data_dir, check)
}

fn procedural(
    out: &mut impl Write,
    device: &str,
    seed: &str,
    job: &str,
    options: &[&str],
) -> ld60_cli::Result<()> {
    let (device, seed, job) = (device.parse()?, seed.parse()?, job.parse()?);
    let folder = match options {
        [] => ld60_cli::procedural::default_folder(device, seed, job),
        ["--out", folder] => folder.into(),
        _ => return Err(format!("unknown options {options:?}").into()),
    };
    ld60_cli::procedural::run(out, device, seed, job, &folder)?;
    writeln!(out, "Play it with: cli play {}", folder.display())?;
    Ok(())
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
    let mut board = None;
    let mut rest = options.iter();
    while let Some(&option) = rest.next() {
        match option {
            "--fixed" => fixed = true,
            "--seed" => seed = value(&mut rest, option)?.parse()?,
            "--variant" => variant = Some(value(&mut rest, option)?.parse()?),
            "--board" => board = Some(value(&mut rest, option)?.parse()?),
            other => return Err(format!("unknown option '{other}'").into()),
        }
    }
    let variant = ld60_cli::find_variant(level, variant, board)?;
    let mut session = ld60_cli::Session::new(variant, fixed, seed)?;
    ld60_cli::play::run(&mut session, io::stdin().lock(), out)
}

fn value<'a>(rest: &mut std::slice::Iter<'_, &'a str>, option: &str) -> ld60_cli::Result<&'a str> {
    rest.next()
        .copied()
        .ok_or_else(|| format!("{option} needs a value").into())
}
