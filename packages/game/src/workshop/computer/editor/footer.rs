use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{AcquireFocus, FocusedInput},
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    text::EditableText,
};
use ld60_devices::{DeviceError, IsaKind, ProgramError};
use ld60_levels::LevelError;

use super::Editor;
use super::page::Page;
use crate::audio::KeyClicks;
use crate::device::{BoardLetter, DeviceName, Emulator};
use crate::locale::{Localized, Message};
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

    pub(super) fn label(self, clicks: bool) -> Message {
        Message::new(match self {
            Self::Notes => "editor-command-notes",
            Self::Versions => "editor-command-versions",
            Self::Assemble => "editor-command-assemble",
            Self::Flash => "editor-command-flash",
            Self::Clicks if clicks => "editor-command-clicks-on",
            Self::Clicks => "editor-command-clicks-off",
        })
    }

    /// Labels new commands and follows the key clicks setting.
    pub(super) fn show_labels(
        clicks: Res<KeyClicks>,
        mut labels: Query<(Ref<Command>, &mut Localized)>,
    ) {
        for (command, mut label) in &mut labels {
            if clicks.is_changed() || command.is_added() {
                label.set_if_neq(command.label(clicks.0).into());
            }
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
    pub(super) fn apply(
        run: On<Run>,
        editor: Single<&EditableText, With<Editor>>,
        job: Option<ResMut<Job>>,
        mut clicks: ResMut<KeyClicks>,
        mut emulators: Query<&mut Emulator>,
        mut status: Single<&mut Localized, With<Status>>,
        mut commands: Commands,
    ) {
        let source = editor.value().to_string();
        let message = match (run.0, job) {
            (Command::Notes | Command::Versions, _) => return,
            (Command::Clicks, _) => {
                clicks.0 = !clicks.0;
                return;
            }
            (_, None) => Message::new("editor-no-job"),
            (Command::Assemble, Some(job)) => match ld60_levels::assemble(&job.level, &source) {
                Ok(assembly) => Message::new("editor-assembled").with("bytes", assembly.byte_len()),
                Err(err) => Message::from(&err),
            },
            (Command::Flash, Some(mut job)) => match emulators.single_mut() {
                Ok(mut emulator) => Self::flash(&mut job, &mut emulator, &source, &mut commands),
                Err(_) => Message::new("editor-no-device"),
            },
        };
        **status = message.into();
    }

    /// Burns `source` into the device and powers it on. Testing it is up to
    /// the player.
    fn flash(
        job: &mut ResMut<Job>,
        emulator: &mut Emulator,
        source: &str,
        commands: &mut Commands,
    ) -> Message {
        let assembly = match ld60_levels::assemble(&job.level, source) {
            Ok(assembly) => assembly,
            Err(err) => return Message::from(&err),
        };
        let rom = match ld60_levels::burn(&job.level, emulator.rom(), assembly.image) {
            Ok(rom) => rom,
            Err(err) => return err.into(),
        };
        let device = match ld60_levels::device(&job.level, rom.clone()) {
            Ok(device) => device,
            Err(err) => return Message::from(&err),
        };
        emulator.restart(device);
        commands.trigger(Flashed(source.into()));
        if !job.flashed {
            job.flashed = true;
        }
        Message::new("editor-flashed")
    }
}

impl From<&LevelError> for Message {
    /// The first assembler error with its line, or the error itself.
    fn from(err: &LevelError) -> Self {
        match err {
            LevelError::Assemble(errors) => match errors.as_slice() {
                [] => Self::new("editor-assembly-failed"),
                [only] => Self::new("editor-asm-error")
                    .with("line", only.line)
                    .with("message", only.message.as_str()),
                [first, rest @ ..] => Self::new("editor-asm-error-more")
                    .with("line", first.line)
                    .with("message", first.message.as_str())
                    .with("more", rest.len()),
            },
            LevelError::Parse(message) => {
                Self::new("editor-error-parse").with("message", message.as_str())
            }
            LevelError::Write(_) => Self::new("editor-error-write"),
            LevelError::BoardNotAllowed(rev) => Self::new("editor-error-board-not-allowed")
                .with("board", BoardLetter(*rev).to_string()),
            LevelError::NoRewrite(rev) => {
                Self::new("editor-error-no-rewrite").with("board", BoardLetter(*rev).to_string())
            }
            LevelError::Device(DeviceError::ChipMismatch { device, chip }) => {
                let chip = match chip {
                    IsaKind::Calco => "CalCo",
                };
                Self::new("editor-error-chip-mismatch")
                    .with("device", device.name())
                    .with("chip", chip)
            }
            LevelError::Device(DeviceError::RomSize { expected, actual }) => {
                Self::new("editor-error-rom-size")
                    .with("expected", *expected)
                    .with("actual", *actual)
            }
        }
    }
}

impl From<ProgramError> for Message {
    fn from(err: ProgramError) -> Self {
        match err {
            ProgramError::MaskRom => Self::new("editor-burn-mask-rom"),
            ProgramError::SizeMismatch => Self::new("editor-burn-size-mismatch"),
            ProgramError::BitConflict { addr } => {
                Self::new("editor-burn-bit-conflict").with("address", format!("0x{addr:02X}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_devices::{BoardRev, DeviceKind};
    use ld60_levels::{GAME_SEED, Variant};
    use ld60_machine::AsmError;

    use super::*;
    use crate::locale::Locale;
    use crate::workshop::computer::editor::undo::Undo;
    use crate::workshop::levels::Levels;

    /// What the status line says after `command` runs on `source`, with or
    /// without the first job and its device.
    fn status(job: bool, powered: bool, source: &str, command: Command) -> Localized {
        let mut world = World::new();
        world.init_resource::<KeyClicks>();
        if job {
            let job = Job::load(&Levels::on_disk(), 0).unwrap().unwrap();
            if powered {
                let rom = ld60_levels::initial_rom(&job.level, &job.buggy).unwrap();
                let device = ld60_levels::device(&job.level, rom).unwrap();
                world.spawn(Emulator::power_on(device));
            }
            world.insert_resource(job);
        }
        world.spawn((Editor, EditableText::new(source), Undo::default()));
        let status = world.spawn((Status, Localized::verbatim(""))).id();
        world.add_observer(Run::apply);
        world.trigger(Run(command));
        world.flush();
        world.get::<Localized>(status).unwrap().clone()
    }

    #[test]
    fn every_status_formats() {
        let job = Variant::new(&Levels::on_disk().levels()[0], GAME_SEED).unwrap();
        let mut statuses = vec![
            status(false, false, "", Command::Assemble),
            status(true, false, &job.buggy, Command::Assemble),
            status(true, false, "NOPE\nNOPE", Command::Assemble),
            status(true, false, &job.fixed, Command::Flash),
            status(true, true, &job.fixed, Command::Flash),
            status(true, true, "NOPE", Command::Flash),
        ];

        let mut world = World::new();
        world.spawn((Editor, EditableText::default(), Undo::default()));
        let line = world.spawn((Status, Localized::verbatim(""))).id();
        let load_job = world.register_system(Editor::load_job);
        world.insert_resource(Job::load(&Levels::on_disk(), 0).unwrap().unwrap());
        world.run_system(load_job).unwrap();
        world.remove_resource::<Job>();
        world.run_system(load_job).unwrap();
        statuses.push(world.get::<Localized>(line).unwrap().clone());

        for status in &statuses {
            assert!(matches!(status, Localized::Message(_)), "{status:?}");
        }
        Locale::on_disk().assert_formats(statuses);
    }

    #[test]
    fn every_error_formats() {
        let asm = |line| AsmError {
            line,
            span: 0..1,
            message: "unknown mnemonic".into(),
        };
        let errors = [
            LevelError::Assemble(Vec::new()),
            LevelError::Assemble(vec![asm(1)]),
            LevelError::Assemble(vec![asm(1), asm(2)]),
            LevelError::Parse("bad".into()),
            LevelError::Write(ron::Error::Eof),
            LevelError::BoardNotAllowed(BoardRev::B),
            LevelError::NoRewrite(BoardRev::C),
            LevelError::Device(DeviceError::ChipMismatch {
                device: DeviceKind::Summa6,
                chip: IsaKind::Calco,
            }),
            LevelError::Device(DeviceError::RomSize {
                expected: 256,
                actual: 1,
            }),
        ];
        let burns = [
            ProgramError::MaskRom,
            ProgramError::SizeMismatch,
            ProgramError::BitConflict { addr: 0x1F },
        ];
        let locale = Locale::on_disk();
        locale.assert_formats(errors.iter().map(Message::from));
        locale.assert_formats(burns.map(Message::from));
    }

    #[test]
    fn every_label_formats() {
        let labels = Command::ALL
            .into_iter()
            .flat_map(|command| [command.label(true), command.label(false)]);
        Locale::on_disk().assert_formats(labels);
    }
}
