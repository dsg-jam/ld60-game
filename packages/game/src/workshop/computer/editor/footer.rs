use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{AcquireFocus, FocusedInput},
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    text::EditableText,
};
use ld60_levels::LevelError;

use super::Editor;
use super::page::Page;
use crate::audio::KeyClicks;
use crate::device::Emulator;
use crate::workshop::job::Job;

/// A command in the footer below the editor.
///
/// Each one runs by clicking it or with its function key. Macs send function
/// keys to the media controls unless Fn is held, so the mouse always works.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Command {
    Notes,
    Versions,
    Assemble,
    Flash,
    Clicks,
}

/// The line above the footer that reports what a command did.
#[derive(Component, Debug)]
pub(super) struct Status;

impl Command {
    pub(super) const ALL: [Self; 5] = [
        Self::Notes,
        Self::Versions,
        Self::Assemble,
        Self::Flash,
        Self::Clicks,
    ];

    fn key(self) -> KeyCode {
        match self {
            Self::Notes => KeyCode::F2,
            Self::Versions => KeyCode::F3,
            Self::Assemble => KeyCode::F5,
            Self::Flash => KeyCode::F6,
            Self::Clicks => KeyCode::F8,
        }
    }

    pub(super) fn label(self, clicks: bool) -> String {
        match self {
            Self::Notes => "F2 NOTES".into(),
            Self::Versions => "F3 VERSIONS".into(),
            Self::Assemble => "F5 ASSEMBLE".into(),
            Self::Flash => "F6 FLASH".into(),
            Self::Clicks => format!("F8 CLICKS {}", if clicks { "ON" } else { "OFF" }),
        }
    }

    pub(super) fn run_on_key(
        mut input: On<FocusedInput<KeyboardInput>>,
        pages: Query<(), With<Page>>,
        mut commands: Commands,
    ) {
        if !pages.contains(input.focused_entity) {
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
pub(super) struct Run(pub(super) Command);

/// The device runs new firmware built from this source.
#[derive(Event, Debug)]
pub(super) struct Flashed(pub(super) String);

impl Run {
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    pub(super) fn apply(
        run: On<Run>,
        editor: Single<&EditableText, With<Editor>>,
        job: Option<ResMut<Job>>,
        mut clicks: ResMut<KeyClicks>,
        mut emulators: Query<&mut Emulator>,
        mut status: Single<&mut Text, (With<Status>, Without<Command>)>,
        mut labels: Query<(&Command, &mut Text), Without<Status>>,
        mut commands: Commands,
    ) {
        let source = editor.value().to_string();
        let message = match (run.0, job) {
            (Command::Notes | Command::Versions, _) => return,
            (Command::Clicks, _) => {
                clicks.0 = !clicks.0;
                for (command, mut text) in &mut labels {
                    text.0 = command.label(clicks.0);
                }
                return;
            }
            (_, None) => "NO JOB ON THE BENCH".into(),
            (Command::Assemble, Some(job)) => match ld60_levels::assemble(&job.level, &source) {
                Ok(assembly) => format!("ASSEMBLED, {} BYTES", assembly.byte_len()),
                Err(err) => Self::describe(&err),
            },
            (Command::Flash, Some(mut job)) => match emulators.single_mut() {
                Ok(mut emulator) => Self::flash(&mut job, &mut emulator, &source, &mut commands),
                Err(_) => "NO DEVICE CONNECTED".into(),
            },
        };
        status.0 = message;
    }

    /// Burns `source` into the device and powers it on. Testing it is up to
    /// the player.
    fn flash(
        job: &mut ResMut<Job>,
        emulator: &mut Emulator,
        source: &str,
        commands: &mut Commands,
    ) -> String {
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
        emulator.restart(device);
        commands.trigger(Flashed(source.into()));
        if !job.flashed {
            job.flashed = true;
        }
        "FLASHED. DEVICE RESTARTED.".into()
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
