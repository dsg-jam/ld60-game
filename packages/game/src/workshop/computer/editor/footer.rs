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
use super::breakpoints::Breakpoints;
use super::page::Page;
use super::trace::TraceView;
use crate::audio::KeyClicks;
use crate::device::{BoardLetter, DeviceName, Emulator};
use crate::locale::{Localized, Message};
use crate::workshop::clock::{Busy, Chore};
use crate::workshop::job::Job;
use crate::workshop::run::{ComputerTool, Run as ShopRun};

/// A command in the footer below the editor.
///
/// Each one runs by clicking it or with its function key. Macs send function
/// keys to the media controls unless Fn is held, so the mouse always works.
/// The commands of a [`ComputerTool`] only show once the shop owns it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Command {
    Notes,
    Versions,
    Test,
    Assemble,
    Flash,
    Trace,
    Clicks,
    Break,
    Step,
    Go,
}

/// The settings that some command labels show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Switches {
    pub(super) clicks: bool,
    pub(super) trace: bool,
}

/// The line above the footer that reports what a command did.
#[derive(Component, Debug)]
pub(super) struct Status;

impl Command {
    pub(super) const ALL: [Self; 10] = [
        Self::Notes,
        Self::Versions,
        Self::Test,
        Self::Assemble,
        Self::Flash,
        Self::Trace,
        Self::Clicks,
        Self::Break,
        Self::Step,
        Self::Go,
    ];

    fn key(self) -> KeyCode {
        match self {
            Self::Notes => KeyCode::F2,
            Self::Versions => KeyCode::F3,
            Self::Test => KeyCode::F4,
            Self::Assemble => KeyCode::F5,
            Self::Flash => KeyCode::F6,
            Self::Trace => KeyCode::F7,
            Self::Clicks => KeyCode::F8,
            Self::Break => KeyCode::F9,
            Self::Step => KeyCode::F10,
            Self::Go => KeyCode::F11,
        }
    }

    /// The tool the command belongs to.
    pub(super) const fn tool(self) -> Option<ComputerTool> {
        match self {
            Self::Test => Some(ComputerTool::TestBench),
            Self::Trace => Some(ComputerTool::Trace),
            Self::Break => Some(ComputerTool::Breakpoints),
            Self::Step | Self::Go => Some(ComputerTool::Debugger),
            Self::Notes | Self::Versions | Self::Assemble | Self::Flash | Self::Clicks => None,
        }
    }

    /// Whether the shop has what the command needs.
    pub(super) fn available(self, shop: &ShopRun) -> bool {
        self.tool().is_none_or(|tool| shop.has(tool))
    }

    pub(super) fn label(self, switches: Switches) -> Message {
        Message::new(match self {
            Self::Notes => "editor-command-notes",
            Self::Versions => "editor-command-versions",
            Self::Test => "editor-command-test",
            Self::Assemble => "editor-command-assemble",
            Self::Flash => "editor-command-flash",
            Self::Trace if switches.trace => "editor-command-trace-on",
            Self::Trace => "editor-command-trace-off",
            Self::Clicks if switches.clicks => "editor-command-clicks-on",
            Self::Clicks => "editor-command-clicks-off",
            Self::Break => "editor-command-break",
            Self::Step => "editor-command-step",
            Self::Go => "editor-command-go",
        })
    }

    /// Labels new commands and follows the settings they show.
    pub(super) fn show_labels(
        clicks: Res<KeyClicks>,
        trace: Res<TraceView>,
        mut labels: Query<(Ref<Command>, &mut Localized)>,
    ) {
        let switches = Switches {
            clicks: clicks.0,
            trace: trace.0,
        };
        for (command, mut label) in &mut labels {
            if clicks.is_changed() || trace.is_changed() || command.is_added() {
                label.set_if_neq(command.label(switches).into());
            }
        }
    }

    /// Shows the commands of the tools the shop owns.
    pub(super) fn show_owned(shop: Res<ShopRun>, mut labels: Query<(Ref<Command>, &mut Node)>) {
        for (command, mut node) in &mut labels {
            if !shop.is_changed() && !command.is_added() {
                continue;
            }
            let display = if command.available(&shop) {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
    }

    pub(super) fn run_on_key(
        mut input: On<FocusedInput<KeyboardInput>>,
        pages: Query<(), With<Page>>,
        shop: Res<ShopRun>,
        mut commands: Commands,
    ) {
        if !pages.contains(input.focused_entity) {
            return;
        }
        let Some(command) = Self::ALL
            .into_iter()
            .filter(|command| command.available(&shop))
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
        breakpoints: Res<Breakpoints>,
        mut emulators: Query<&mut Emulator>,
        mut status: Single<&mut Localized, With<Status>>,
        mut commands: Commands,
    ) {
        let source = editor.value().to_string();
        let message = match (run.0, job) {
            (
                Command::Notes
                | Command::Versions
                | Command::Test
                | Command::Trace
                | Command::Break
                | Command::Step
                | Command::Go,
                _,
            ) => return,
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
                Ok(mut emulator) => Self::flash(
                    &mut job,
                    &mut emulator,
                    &breakpoints,
                    &source,
                    &mut commands,
                ),
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
        breakpoints: &Breakpoints,
        source: &str,
        commands: &mut Commands,
    ) -> Message {
        let assembly = match ld60_levels::assemble(&job.level, source) {
            Ok(assembly) => assembly,
            Err(err) => return Message::from(&err),
        };
        let addresses = breakpoints.addresses(&assembly);
        let rom = match ld60_levels::burn(&job.level, emulator.rom(), assembly.image) {
            Ok(rom) => rom,
            Err(err) => return err.into(),
        };
        let device = match ld60_levels::device(&job.level, rom.clone()) {
            Ok(device) => device,
            Err(err) => return Message::from(&err),
        };
        emulator.set_breakpoints(addresses);
        emulator.restart(device);
        commands.trigger(Flashed(source.into()));
        commands.trigger(Busy(Chore::Flash));
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
        world.init_resource::<Breakpoints>();
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
        let labels = Command::ALL.into_iter().flat_map(|command| {
            [true, false].into_iter().flat_map(move |clicks| {
                [true, false].map(|trace| command.label(Switches { clicks, trace }))
            })
        });
        Locale::on_disk().assert_formats(labels);
    }

    #[test]
    fn shows_only_the_commands_of_owned_tools() {
        let mut world = World::new();
        world.init_resource::<ShopRun>();
        let labels: Vec<_> = Command::ALL
            .into_iter()
            .map(|command| world.spawn((command, Node::default())).id())
            .collect();
        let show = world.register_system(Command::show_owned);
        let shown = |world: &World| -> Vec<Command> {
            labels
                .iter()
                .filter(|&&label| world.get::<Node>(label).unwrap().display != Display::None)
                .map(|&label| *world.get::<Command>(label).unwrap())
                .collect()
        };
        world.run_system(show).unwrap();
        assert!(shown(&world).iter().all(|command| command.tool().is_none()));
        world.insert_resource(ShopRun::with_all_tools());
        world.run_system(show).unwrap();
        assert_eq!(shown(&world), Command::ALL);
    }
}
