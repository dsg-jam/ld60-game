use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{AcquireFocus, FocusedInput},
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    text::EditableText,
};
use ld60_game_run::job::Job;
use ld60_game_run::{ComputerTool, Origin, Run as ShopRun};
use ld60_locale::{Localized, Message};

use super::Editor;
use super::breakpoints::Breakpoints;
use super::dump::Dump;
use super::errors::Explain;
use super::page::Page;
use super::trace::TraceView;
use ld60_game_device::{Connected, Emulator};

/// A command in the footer below the editor.
///
/// Each one runs by clicking it or with its function key. Macs send function
/// keys to the media controls unless Fn is held, so the mouse always works.
/// The commands of a [`ComputerTool`] only show once the shop owns it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Command {
    /// Reads the firmware off the device, then disassembles it, see
    /// [`Dump`].
    Dump,
    Notes,
    Versions,
    Test,
    Assemble,
    Flash,
    Trace,
    Break,
    Step,
    Go,
}

/// The line above the footer that reports what a command did.
#[derive(Component, Debug)]
pub(super) struct Status;

impl Command {
    pub(super) const ALL: [Self; 10] = [
        Self::Dump,
        Self::Notes,
        Self::Versions,
        Self::Test,
        Self::Assemble,
        Self::Flash,
        Self::Trace,
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
            Self::Dump => KeyCode::F8,
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
            Self::Dump | Self::Notes | Self::Versions | Self::Assemble | Self::Flash => None,
        }
    }

    /// Whether the command can run: [`Command::Dump`] until the source is in
    /// the editor, then the others whose tool the shop has. The test bench
    /// is lent for the jobs of the tutorial.
    pub(super) fn available(self, shop: &ShopRun, dump: Dump, lesson: bool) -> bool {
        if self == Self::Dump {
            return dump != Dump::Source;
        }
        dump == Dump::Source
            && self
                .tool()
                .is_none_or(|tool| shop.has(tool) || (lesson && tool == ComputerTool::TestBench))
    }

    /// Whether the job is one of the tutorial.
    fn lesson(job: Option<&Job>) -> bool {
        job.is_some_and(|job| matches!(job.origin, Origin::Lesson { .. }))
    }

    /// The label of [`Command::Trace`] says whether the trace is on, and the
    /// one of [`Command::Dump`] what it does next.
    pub(super) fn label(self, trace: bool, dump: Dump) -> Message {
        Message::new(match self {
            Self::Dump if dump == Dump::Empty => "editor-command-dump",
            Self::Dump => "editor-command-disassemble",
            Self::Notes => "editor-command-notes",
            Self::Versions => "editor-command-versions",
            Self::Test => "editor-command-test",
            Self::Assemble => "editor-command-assemble",
            Self::Flash => "editor-command-flash",
            Self::Trace if trace => "editor-command-trace-on",
            Self::Trace => "editor-command-trace-off",
            Self::Break => "editor-command-break",
            Self::Step => "editor-command-step",
            Self::Go => "editor-command-go",
        })
    }

    /// Labels new commands and follows the settings they show.
    pub(super) fn show_labels(
        trace: Res<TraceView>,
        dump: Res<Dump>,
        mut labels: Query<(Ref<Command>, &mut Localized)>,
    ) {
        for (command, mut label) in &mut labels {
            if trace.is_changed() || dump.is_changed() || command.is_added() {
                label.set_if_neq(command.label(trace.0, *dump).into());
            }
        }
    }

    /// Shows the commands of the tools the shop owns.
    pub(super) fn show_owned(
        shop: Res<ShopRun>,
        dump: Res<Dump>,
        job: Option<Res<Job>>,
        mut labels: Query<(Ref<Command>, &mut Node)>,
    ) {
        let job_changed = job.as_ref().is_some_and(Res::is_changed);
        let lesson = Self::lesson(job.as_deref());
        for (command, mut node) in &mut labels {
            if !shop.is_changed() && !dump.is_changed() && !job_changed && !command.is_added() {
                continue;
            }
            let display = if command.available(&shop, *dump, lesson) {
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
        dump: Res<Dump>,
        job: Option<Res<Job>>,
        mut commands: Commands,
    ) {
        let lesson = Self::lesson(job.as_deref());
        if !pages.contains(input.focused_entity) {
            return;
        }
        let Some(command) = Self::ALL
            .into_iter()
            .filter(|&command| command.available(&shop, *dump, lesson))
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
        breakpoints: Res<Breakpoints>,
        mut emulators: Query<&mut Emulator, With<Connected>>,
        mut status: Single<&mut Localized, With<Status>>,
        mut commands: Commands,
    ) {
        let source = editor.value().to_string();
        let message = match (run.0, job) {
            (
                Command::Dump
                | Command::Notes
                | Command::Versions
                | Command::Test
                | Command::Trace
                | Command::Break
                | Command::Step
                | Command::Go,
                _,
            ) => return,
            (_, None) => Message::new("editor-no-job"),
            (Command::Assemble, Some(job)) => match ld60_levels::assemble(&job.level, &source) {
                Ok(assembly) => Message::new("editor-assembled").with("bytes", assembly.byte_len()),
                Err(err) => err.message(),
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
            Err(err) => return err.message(),
        };
        let addresses = breakpoints.addresses(&assembly);
        let rom = match ld60_levels::burn(&job.level, emulator.rom(), assembly.image) {
            Ok(rom) => rom,
            Err(err) => return err.message(),
        };
        let device = match ld60_levels::device(&job.level, rom.clone()) {
            Ok(device) => device,
            Err(err) => return err.message(),
        };
        emulator.set_breakpoints(addresses);
        emulator.restart(device);
        commands.trigger(Flashed(source.into()));
        if !job.flashed {
            job.flashed = true;
        }
        Message::new("editor-flashed")
    }
}

#[cfg(test)]
mod tests {
    use ld60_game_run::levels::Levels;
    use ld60_levels::{GAME_SEED, Variant};
    use ld60_locale::Locale;

    use super::*;
    use crate::workshop::computer::editor::page::ActivePage;
    use crate::workshop::computer::editor::undo::Undo;

    /// What the status line says after `command` runs on `source`, with or
    /// without the first job and its device.
    fn status(job: bool, powered: bool, source: &str, command: Command) -> Localized {
        let mut world = World::new();
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
        world.init_resource::<Dump>();
        world.init_resource::<ActivePage>();
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
    fn every_label_formats() {
        let labels = Command::ALL.into_iter().flat_map(|command| {
            [(true, Dump::Empty), (false, Dump::Bits)]
                .map(|(trace, dump)| command.label(trace, dump))
        });
        Locale::on_disk().assert_formats(labels);
    }

    #[test]
    fn shows_only_the_commands_of_owned_tools() {
        let mut world = World::new();
        world.init_resource::<ShopRun>();
        world.init_resource::<Dump>();
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
        assert_eq!(shown(&world), [Command::Dump]);
        world.insert_resource(Dump::Source);
        world.run_system(show).unwrap();
        assert!(shown(&world).iter().all(|command| command.tool().is_none()));
        assert!(!shown(&world).contains(&Command::Dump));
        world.insert_resource(ShopRun::with_all_tools());
        world.run_system(show).unwrap();
        assert_eq!(shown(&world), &Command::ALL[1..]);
    }
}
