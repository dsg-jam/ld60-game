use bevy::prelude::*;
use bevy::remote::{BrpError, BrpResult, error_codes};
use ld60_game_core::clock::Clock;
use ld60_game_core::debug_port::{DebugMethod, DebugMethods, ParamKind};
use ld60_game_device::Emulator;
use ld60_game_run::chore::{Busy, Chore, PlanetSet};
use ld60_game_run::job::{Job, JobOnBench};
use ld60_game_run::levels::Levels;
use ld60_game_run::{Origin, Run};
use serde_json::{Value, json};

use super::{Arrive, HandIn, JobItem, NEXT_JOB};

/// Registers `ld60.job` and `ld60.clock`.
pub(super) fn register(app: &mut App) {
    app.add_debug_method(
        DebugMethod::new(
            "ld60.job",
            "Reports and works the job on the bench. skip and level replace it.",
        )
        .param("skip", ParamKind::Numbers, "settles this many jobs as paid")
        .param(
            "level",
            ParamKind::Text,
            "puts this level folder on the bench",
        )
        .param("levels", ParamKind::Flag, "lists the level folders")
        .param("fix", ParamKind::Flag, "flashes the factory fix")
        .param("stamp", ParamKind::Flag, "stamps the ticket")
        .param("hand_in", ParamKind::Flag, "the customer tries the device"),
        job,
    )
    .add_debug_method(
        DebugMethod::new(
            "ld60.clock",
            "Moves the planet on. The landlord comes when it sets.",
        )
        .param("ticks", ParamKind::Numbers, "how far, 40 is a whole cycle"),
        clock,
    );
}

fn invalid(message: impl Into<String>) -> BrpError {
    BrpError {
        code: error_codes::INVALID_PARAMS,
        message: message.into(),
        data: None,
    }
}

fn failed(error: impl std::fmt::Display) -> BrpError {
    BrpError {
        code: error_codes::INTERNAL_ERROR,
        message: error.to_string(),
        data: None,
    }
}

/// `ld60.job { "skip": 2, "level": "...", "levels": true, "fix": true,
/// "stamp": true, "hand_in": true }`. `skip` settles jobs as if they were
/// fixed and paid, and the drone brings the next. `level` puts a level
/// folder on the bench out of turn. The others work on the job on the
/// bench, in that order. Returns the job on the bench, or the level folders
/// with `levels`.
fn job(
    In(params): In<Option<Value>>,
    mut run: ResMut<Run>,
    levels: Option<Res<Levels>>,
    job: Option<ResMut<Job>>,
    mut emulators: Query<&mut Emulator, With<JobItem>>,
    mut commands: Commands,
) -> BrpResult {
    let params = params.unwrap_or_default();
    let flag = |name: &str| params.get(name).and_then(Value::as_bool) == Some(true);
    let levels = levels.ok_or_else(|| invalid("the levels are not loaded yet"))?;
    if flag("levels") {
        return Ok(levels
            .levels()
            .iter()
            .map(|source| json!(source.path))
            .collect());
    }
    if let Some(skip) = params.get("skip").and_then(Value::as_u64) {
        for _ in 0..skip {
            let mut next = Job::for_run(&run, &levels).map_err(failed)?;
            let rom = fixed_rom(&next, None)?;
            let pickup = next.pick_up(next.try_out(&rom));
            if !pickup.ends_job() {
                return Err(failed(format!(
                    "the factory fix of {} fails",
                    next.level.id
                )));
            }
            run.settle(&next, &pickup);
            commands.trigger(Busy(Chore::HandIn));
        }
        commands.insert_resource(Arrive(NEXT_JOB));
        commands.run_system_cached(Job::start);
        return Ok(json!({ "skipped": skip, "till": run.till() }));
    }
    if let Some(path) = params.get("level").and_then(Value::as_str) {
        let source = levels
            .find(path)
            .cloned()
            .ok_or_else(|| invalid(format!("no level folder {path}, see levels")))?;
        let next = Job::from_level(source, &run, &levels).map_err(failed)?;
        let status = status(&next);
        commands.insert_resource(next);
        commands.insert_resource(Arrive(NEXT_JOB));
        commands.trigger(JobOnBench);
        return Ok(status);
    }
    let mut job = job.ok_or_else(|| invalid("no job is on the bench"))?;
    if flag("fix") {
        let mut emulator = emulators
            .single_mut()
            .map_err(|_| invalid("the device is not on the counter"))?;
        let rom = fixed_rom(&job, Some(&*emulator))?;
        let device = ld60_levels::device(&job.level, rom).map_err(failed)?;
        emulator.restart(device);
        job.flashed = true;
    }
    if flag("stamp") || flag("hand_in") {
        job.stamped = true;
    }
    if flag("hand_in") {
        commands.trigger(HandIn);
    }
    Ok(status(&job))
}

/// The firmware of `job` with the factory fix burned in, into the ROM of
/// `emulator` or the one the device came with.
fn fixed_rom(job: &Job, emulator: Option<&Emulator>) -> Result<ld60_devices::Rom, BrpError> {
    let rom = match emulator {
        Some(emulator) => emulator.rom().clone(),
        None => ld60_levels::initial_rom(&job.level, &job.buggy).map_err(failed)?,
    };
    let assembly = ld60_levels::assemble(&job.level, job.fixed()).map_err(failed)?;
    ld60_levels::burn(&job.level, &rom, assembly.image).map_err(failed)
}

fn status(job: &Job) -> Value {
    json!({
        "level": job.level.id,
        "device": format!("{:?}", job.level.device),
        "origin": match job.origin {
            Origin::Lesson { .. } => "lesson",
            Origin::Special(_) => "special",
            Origin::Generated { .. } => "generated",
        },
        "flashed": job.flashed,
        "stamped": job.stamped,
        "tries_left": job.tries_left(),
        "reward": job.reward(),
    })
}

/// `ld60.clock { "ticks": 4 }` moves the planet on, as work at the counter
/// would. Each set brings the landlord. Returns the ticks since it first
/// rose.
fn clock(
    In(params): In<Option<Value>>,
    mut clock: ResMut<Clock>,
    mut commands: Commands,
) -> BrpResult {
    let ticks = params
        .as_ref()
        .and_then(|params| params.get("ticks"))
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let ticks = u32::try_from(ticks).map_err(|_| invalid("too many ticks"))?;
    for _ in 0..clock.advance(ticks) {
        commands.trigger(PlanetSet);
    }
    Ok(json!(u32::from(clock.clone())))
}
