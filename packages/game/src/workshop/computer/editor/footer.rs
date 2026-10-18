use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{AcquireFocus, FocusedInput},
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    text::EditableText,
};
use ld60_levels::{LevelError, TEST_SEEDS};

use super::Editor;
use crate::audio::KeyClicks;
use crate::device::Emulator;
use crate::workshop::job::Job;

/// A command in the footer below the editor.
///
/// Each one runs by clicking it or with its function key. Macs send function
/// keys to the media controls unless Fn is held, so the mouse always works.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Command {
    Assemble,
    Flash,
    Clicks,
}

/// The line above the footer that reports what a command did.
#[derive(Component, Debug)]
pub(super) struct Status;

impl Command {
    pub(super) const ALL: [Self; 3] = [Self::Assemble, Self::Flash, Self::Clicks];

    fn key(self) -> KeyCode {
        match self {
            Self::Assemble => KeyCode::F5,
            Self::Flash => KeyCode::F6,
            Self::Clicks => KeyCode::F8,
        }
    }

    pub(super) fn label(self, clicks: bool) -> String {
        match self {
            Self::Assemble => "F5 ASSEMBLE".into(),
            Self::Flash => "F6 FLASH".into(),
            Self::Clicks => format!("F8 CLICKS {}", if clicks { "ON" } else { "OFF" }),
        }
    }

    pub(super) fn run_on_key(
        mut input: On<FocusedInput<KeyboardInput>>,
        editors: Query<(), With<Editor>>,
        mut commands: Commands,
    ) {
        if !editors.contains(input.focused_entity) {
            return;
        }
        let Some(command) = Self::ALL
            .into_iter()
            .find(|command| command.key() == input.input.key_code)
        else {
            return;
        };
        input.propagate(false);
        if input.input.state.is_pressed() && !input.input.repeat {
            commands.trigger(Run(command));
        }
    }

    pub(super) fn run_on_click(
        click: On<PointerClick>,
        buttons: Query<&Command>,
        mut commands: Commands,
    ) {
        if click.button == PointerButton::Primary
            && let Ok(&command) = buttons.get(click.entity)
        {
            commands.trigger(Run(command));
        }
    }

    /// Clicking a command would otherwise take the focus from the editor.
    pub(super) fn keep_editor_focus(
        mut acquire: On<AcquireFocus>,
        buttons: Query<(), With<Command>>,
    ) {
        if buttons.contains(acquire.focused_entity) {
            acquire.propagate(false);
        }
    }
}

/// Runs a footer [`Command`].
#[derive(Event, Debug)]
pub(super) struct Run(Command);

impl Run {
    pub(super) fn apply(
        run: On<Run>,
        editor: Single<&EditableText, With<Editor>>,
        job: Option<ResMut<Job>>,
        mut clicks: ResMut<KeyClicks>,
        mut emulators: Query<&mut Emulator>,
        mut status: Single<&mut Text, (With<Status>, Without<Command>)>,
        mut labels: Query<(&Command, &mut Text), Without<Status>>,
    ) {
        let source = editor.value().to_string();
        let message = match (run.0, job) {
            (Command::Clicks, _) => {
                clicks.0 = !clicks.0;
                for (command, mut text) in &mut labels {
                    text.0 = command.label(clicks.0);
                }
                return;
            }
            (_, None) => "NO JOB ON THE BENCH".into(),
            (Command::Assemble, Some(job)) => match ld60_levels::assemble(&job.level, &source) {
                Ok(assembly) => {
                    let bytes: u16 = assembly.lines.iter().map(|line| line.len).sum();
                    format!("ASSEMBLED, {bytes} BYTES")
                }
                Err(err) => Self::describe(&err),
            },
            (Command::Flash, Some(mut job)) => match emulators.single_mut() {
                Ok(mut emulator) => Self::flash(&mut job, &mut emulator, &source),
                Err(_) => "NO DEVICE CONNECTED".into(),
            },
        };
        status.0 = message;
    }

    /// Burns `source` into the device, powers it on and runs the
    /// customer's tests.
    fn flash(job: &mut Job, emulator: &mut Emulator, source: &str) -> String {
        let assembly = match ld60_levels::assemble(&job.level, source) {
            Ok(assembly) => assembly,
            Err(err) => return Self::describe(&err),
        };
        let rom = match ld60_levels::burn(&job.level, emulator.rom(), assembly.image) {
            Ok(rom) => rom,
            Err(err) => return format!("BURN FAILED: {err}"),
        };
        let device = match ld60_levels::device(&job.level, rom.clone()) {
            Ok(device) => device,
            Err(err) => return Self::describe(&err),
        };
        *emulator = Emulator::power_on(device);
        let report = match ld60_levels::run(&job.level, &rom, TEST_SEEDS) {
            Ok(report) => report,
            Err(err) => return Self::describe(&err),
        };
        let passed = report.outcomes.iter().filter(|o| o.passed()).count();
        let total = report.outcomes.len();
        if report.passed() {
            job.repaired = true;
            format!("FLASHED. ALL {total} TESTS PASS")
        } else {
            format!("FLASHED. {passed} OF {total} TESTS PASS")
        }
    }

    /// The first assembler error with its line, or the error itself.
    fn describe(err: &LevelError) -> String {
        match err {
            LevelError::Assemble(errors) => match errors.as_slice() {
                [] => "ASSEMBLY FAILED".into(),
                [only] => only.to_string(),
                [first, rest @ ..] => format!("{first} (+{} MORE)", rest.len()),
            },
            other => other.to_string(),
        }
    }
}
