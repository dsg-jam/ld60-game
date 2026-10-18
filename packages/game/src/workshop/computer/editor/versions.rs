mod changes;
mod list;

use std::time::Duration;

use self::list::{MAX_VERSIONS, Removal, SaveError, Version, VersionList};
use super::footer::{Flashed, Status};
use super::page::{ActivePage, Page};
use super::undo::Undo;
use super::{Editor, FONT_SIZE, PHOSPHOR, PHOSPHOR_DIM, ROW_GAP};
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;
use crate::locale::{Locale, Localized, Message};
use crate::workshop::job::{Job, JobId};
use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{FocusedInput, tab_navigation::TabIndex},
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    text::EditableText,
};

/// Width of the label column. Longer labels are cut, so the marks line up.
const LABEL_WIDTH: usize = 26;

pub(super) struct VersionsPlugin;

impl Plugin for VersionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Versions>()
            .add_systems(
                Update,
                (
                    Versions::load_job,
                    Versions::disarm.run_if(resource_changed::<ActivePage>),
                    Versions::render
                        .run_if(resource_changed::<Versions>.or_eager(resource_changed::<Locale>)),
                )
                    .chain(),
            )
            .add_observer(Versions::mark_flashed)
            .add_observer(VersionRow::select_on_click)
            .add_observer(Action::run_on_key)
            .add_observer(Action::run_on_click)
            .add_observer(Perform::apply);
        #[cfg(feature = "debug_port")]
        app.add_debug_method("ld60.versions", Versions::debug_state);
    }
}

/// The saved versions of the current job's source. A new job starts a new
/// list.
#[derive(Resource, Debug, Default)]
struct Versions {
    job: Option<JobId>,
    /// When the job came on the bench, in real time.
    started: Duration,
    list: Option<VersionList>,
}

/// The page that lists the [`Versions`].
#[derive(Component, Debug)]
pub(super) struct VersionsPage;

/// A line of the version list, by position.
#[derive(Component, Debug)]
struct VersionRow(usize);

/// The marks after the labels in the version list, in the player's
/// language.
struct Marks {
    auto: String,
    flashed: String,
}

/// Something to do with the version list. Each one runs by clicking it or
/// with its key.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Save,
    Restore,
    Remove,
}

/// Runs an [`Action`].
#[derive(Event, Debug)]
struct Perform(Action);

impl Versions {
    /// Starts a new list when the job changes. Returns whether it did.
    fn sync(&mut self, job: Option<(JobId, &str)>, now: Duration) -> bool {
        let id = job.map(|(id, _)| id);
        if id == self.job {
            return false;
        }
        self.job = id;
        self.started = now;
        self.list = job.map(|(_, factory)| VersionList::new(factory));
        true
    }

    fn load_job(job: Option<Res<Job>>, time: Res<Time<Real>>, mut versions: ResMut<Versions>) {
        let job = job.as_ref().map(|job| (job.id, job.buggy.as_str()));
        if versions.bypass_change_detection().sync(job, time.elapsed()) {
            versions.set_changed();
        }
    }

    /// A removal waits for its second press only while the page stays open.
    fn disarm(mut versions: ResMut<Versions>) {
        if let Some(list) = versions.bypass_change_detection().list.as_mut() {
            list.disarm();
        }
    }

    fn render(
        versions: Res<Versions>,
        locale: Res<Locale>,
        mut rows: Query<(
            &VersionRow,
            &mut Node,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        )>,
    ) {
        let marks = Marks::new(&locale);
        for (version_row, mut node, mut text, mut color, mut background) in &mut rows {
            let Some((list, version)) = versions
                .list
                .as_ref()
                .and_then(|list| Some((list, list.versions().get(version_row.0)?)))
            else {
                node.display = Display::None;
                continue;
            };
            node.display = Display::Flex;
            text.0 = marks.row(version, list.is_flashed(version), &locale);
            let selected = list.selected() == version_row.0;
            color.0 = if selected { Color::BLACK } else { PHOSPHOR };
            background.0 = if selected { PHOSPHOR } else { Color::NONE };
        }
    }

    fn mark_flashed(flashed: On<Flashed>, mut versions: ResMut<Versions>) {
        if let Some(list) = versions.list.as_mut() {
            list.mark_flashed(&flashed.0);
        }
    }

    /// `ld60.versions` returns the versions of the current job and the
    /// selected position.
    #[cfg(feature = "debug_port")]
    fn debug_state(
        In(_): In<Option<serde_json::Value>>,
        versions: Res<Versions>,
        locale: Res<Locale>,
    ) -> bevy::remote::BrpResult {
        let Some(list) = &versions.list else {
            return Ok(serde_json::Value::Null);
        };
        let versions: Vec<_> = list
            .versions()
            .iter()
            .map(|version| {
                serde_json::json!({
                    "number": version.number,
                    "label": locale.text(&version.label.clone().into()),
                    "flashed": list.is_flashed(version),
                    "auto": version.auto,
                    "text": version.text,
                })
            })
            .collect();
        Ok(serde_json::json!({
            "versions": versions,
            "selected": list.selected(),
        }))
    }
}

impl VersionsPage {
    /// The versions page, hidden until the player opens it.
    pub(super) fn bundle(font: TextFont) -> impl Bundle {
        let row_font = font.clone();
        (
            VersionsPage,
            Page::Versions,
            Name::new("versions"),
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(ROW_GAP),
                display: Display::None,
                ..default()
            },
            TabIndex(0),
            children![
                (
                    Node {
                        column_gap: px(FONT_SIZE * 2.0),
                        ..default()
                    },
                    Children::spawn(SpawnIter(Action::ALL.into_iter().map(move |action| {
                        (
                            action,
                            Localized::from(action.label()),
                            font.clone(),
                            TextColor(PHOSPHOR_DIM),
                        )
                    }))),
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    Children::spawn(SpawnIter((0..MAX_VERSIONS).map(move |index| {
                        (
                            VersionRow(index),
                            Node {
                                display: Display::None,
                                padding: px(2).horizontal(),
                                ..default()
                            },
                            Text::default(),
                            row_font.clone(),
                            TextColor(PHOSPHOR),
                            BackgroundColor(Color::NONE),
                        )
                    }))),
                ),
            ],
        )
    }
}

impl Marks {
    fn messages() -> [Message; 2] {
        [
            Message::new("editor-version-auto"),
            Message::new("editor-version-flashed"),
        ]
    }

    fn new(locale: &Locale) -> Self {
        let [auto, flashed] = Self::messages().map(|message| locale.text(&message.into()));
        Self { auto, flashed }
    }

    /// The line of `version` in the list.
    fn row(&self, version: &Version, flashed: bool, locale: &Locale) -> String {
        let label: String = locale
            .text(&version.label.clone().into())
            .chars()
            .take(LABEL_WIDTH - 1)
            .collect();
        let auto = if version.auto { self.auto.as_str() } else { "" };
        let auto_width = self.auto.chars().count() + 1;
        let flashed = if flashed { self.flashed.as_str() } else { "" };
        let number = version.number;
        format!("{number:>2}  {label:<LABEL_WIDTH$}{auto:<auto_width$}{flashed}")
            .trim_end()
            .into()
    }
}

impl VersionRow {
    fn select_on_click(
        click: On<PointerClick>,
        rows: Query<&VersionRow>,
        mut versions: ResMut<Versions>,
    ) {
        if click.button == PointerButton::Primary
            && let Ok(row) = rows.get(click.entity)
            && let Some(list) = versions.list.as_mut()
        {
            list.select(row.0);
        }
    }
}

impl Action {
    const ALL: [Self; 3] = [Self::Save, Self::Restore, Self::Remove];

    fn label(self) -> Message {
        Message::new(match self {
            Self::Save => "editor-action-save",
            Self::Restore => "editor-action-restore",
            Self::Remove => "editor-action-remove",
        })
    }

    fn saved(result: Result<u32, SaveError>) -> Message {
        match result {
            Ok(number) => Message::new("editor-saved").with("version", number),
            Err(SaveError::Duplicate(number)) => {
                Message::new("editor-duplicate").with("version", number)
            }
            Err(SaveError::Full) => Message::new("editor-full"),
        }
    }

    /// `kept` is the version that saved the changes, if a restore had to.
    fn restored(number: u32, kept: Result<Option<u32>, SaveError>) -> Message {
        match kept {
            Ok(Some(kept)) => Message::new("editor-restored-kept")
                .with("version", number)
                .with("kept", kept),
            Ok(None) => Message::new("editor-restored").with("version", number),
            Err(_) => Message::new("editor-full-keep"),
        }
    }

    /// Arrows move the selection. The page takes all other keys too, so
    /// none of them reach tab navigation.
    fn run_on_key(
        mut input: On<FocusedInput<KeyboardInput>>,
        pages: Query<(), With<VersionsPage>>,
        mut versions: ResMut<Versions>,
        mut commands: Commands,
    ) {
        if !pages.contains(input.focused_entity) {
            return;
        }
        input.propagate(false);
        if !input.input.state.is_pressed() {
            return;
        }
        let action = match input.input.key_code {
            KeyCode::KeyS if !input.input.repeat => Self::Save,
            KeyCode::Enter | KeyCode::NumpadEnter if !input.input.repeat => Self::Restore,
            KeyCode::Delete | KeyCode::Backspace if !input.input.repeat => Self::Remove,
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                let step = if input.input.key_code == KeyCode::ArrowUp {
                    -1
                } else {
                    1
                };
                if let Some(list) = versions.list.as_mut() {
                    list.select_by(step);
                }
                return;
            }
            _ => return,
        };
        commands.trigger(Perform(action));
    }

    fn run_on_click(click: On<PointerClick>, actions: Query<&Action>, mut commands: Commands) {
        if click.button == PointerButton::Primary
            && let Ok(&action) = actions.get(click.entity)
        {
            commands.trigger(Perform(action));
        }
    }
}

impl Perform {
    fn apply(
        perform: On<Perform>,
        job: Option<Res<Job>>,
        time: Res<Time<Real>>,
        mut versions: ResMut<Versions>,
        editor: Single<(&mut EditableText, &mut Undo), With<Editor>>,
        mut status: Single<&mut Localized, With<Status>>,
        mut active: ResMut<ActivePage>,
    ) {
        let (mut editor, mut undo) = editor.into_inner();
        let on_job = time.elapsed().saturating_sub(versions.started);
        let (Some(job), Some(list)) = (job, versions.list.as_mut()) else {
            **status = Message::new("editor-no-job").into();
            return;
        };
        let message = match perform.0 {
            Action::Save => {
                let source = editor.value().to_string();
                let label = Version::label(&job, &list.factory().text, &source, on_job);
                Action::saved(list.save(&source, label))
            }
            Action::Restore => {
                let source = editor.value().to_string();
                let label = Version::label(&job, &list.factory().text, &source, on_job);
                let version = list.selected_version();
                let (number, text) = (version.number, version.text.clone());
                let kept = list.keep(&source, label);
                if kept.is_ok() {
                    editor.viewport.offset.y = 0.0;
                    undo.replace(&mut editor, &text);
                    active.0 = Page::Source;
                }
                Action::restored(number, kept)
            }
            Action::Remove => list.remove_selected().into(),
        };
        **status = message.into();
    }
}

impl From<Removal> for Message {
    fn from(removal: Removal) -> Self {
        match removal {
            Removal::Factory => Self::new("editor-factory-stays"),
            Removal::Armed(number) => Self::new("editor-remove-armed").with("version", number),
            Removal::Removed(number) => Self::new("editor-removed").with("version", number),
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::workshop::levels::Levels;

    /// The rows that [`Versions::render`] shows.
    fn render(versions: Versions, locale: Locale) -> Vec<String> {
        let mut world = World::new();
        let rows: Vec<_> = (0..versions.list.as_ref().unwrap().versions().len())
            .map(|index| {
                world
                    .spawn((
                        VersionRow(index),
                        Node::default(),
                        Text::default(),
                        TextColor::default(),
                        BackgroundColor::default(),
                    ))
                    .id()
            })
            .collect();
        world.insert_resource(versions);
        world.insert_resource(locale);
        world.run_system_once(Versions::render).unwrap();
        rows.into_iter()
            .map(|row| world.get::<Text>(row).unwrap().0.clone())
            .collect()
    }

    /// The factory firmware, a long label and an automatic save, with the
    /// version at `flashed` flashed.
    fn versions(flashed: usize) -> Versions {
        let long = Message::new("editor-version-label")
            .with("time", "123:45")
            .with("changes", "+100 -100")
            .with(
                "result",
                Message::new("editor-version-errors").with("errors", 100_u32),
            );
        let short = Message::new("editor-version-label")
            .with("time", "00:05")
            .with("changes", "+1 -1")
            .with(
                "result",
                Message::new("editor-version-bytes").with("bytes", 42_u32),
            );
        let mut versions = Versions::default();
        versions.sync(Some((JobId::next(), "factory")), Duration::ZERO);
        let list = versions.list.as_mut().unwrap();
        list.save("fix", long).unwrap();
        list.keep("other", short).unwrap();
        let text = list.versions()[flashed].text.clone();
        list.mark_flashed(&text);
        versions
    }

    #[test]
    fn rows_keep_the_marks_in_line() {
        for auto in ["AUTO", "AUTOMATISCH"] {
            let locale = || Locale::on_disk().overridden(&format!("editor-version-auto = {auto}"));
            let plain = render(versions(1), locale());
            let saved = render(versions(2), locale());
            assert_eq!(saved[2].find(auto), Some(4 + LABEL_WIDTH));
            let flashed = Some(4 + LABEL_WIDTH + auto.len() + 1);
            assert_eq!(plain[1].find("FLASHED"), flashed, "{plain:#?}");
            assert_eq!(saved[2].find("FLASHED"), flashed, "{saved:#?}");
        }
    }

    #[test]
    fn every_message_formats() {
        let job = Job::load(&Levels::on_disk(), 0).unwrap().unwrap();
        let label = |source: &str| Version::label(&job, &job.buggy, source, Duration::ZERO);
        let mut messages = vec![
            label(&job.buggy),
            label("NOPE"),
            VersionList::new("").factory().label.clone(),
            Action::saved(Ok(1)),
            Action::saved(Err(SaveError::Duplicate(1))),
            Action::saved(Err(SaveError::Full)),
            Action::restored(1, Ok(Some(2))),
            Action::restored(1, Ok(None)),
            Action::restored(1, Err(SaveError::Full)),
            Removal::Factory.into(),
            Removal::Armed(1).into(),
            Removal::Removed(1).into(),
        ];
        messages.extend(Action::ALL.map(Action::label));
        messages.extend(Marks::messages());
        Locale::on_disk().assert_formats(messages);
    }

    #[test]
    fn without_a_job_the_status_formats() {
        let mut world = World::new();
        world.init_resource::<Versions>();
        world.init_resource::<Time<Real>>();
        world.init_resource::<ActivePage>();
        world.spawn((Editor, EditableText::default(), Undo::default()));
        let status = world.spawn((Status, Localized::verbatim(""))).id();
        world.add_observer(Perform::apply);
        world.trigger(Perform(Action::Save));
        world.flush();
        let status = world.get::<Localized>(status).unwrap().clone();
        assert!(matches!(status, Localized::Message(_)), "{status:?}");
        Locale::on_disk().assert_formats([status]);
    }

    #[test]
    fn a_new_job_starts_a_new_list() {
        let mut versions = Versions::default();
        let first = JobId::next();
        assert!(versions.sync(Some((first, "first")), Duration::ZERO));
        let list = versions.list.as_mut().unwrap();
        list.save("fix", Message::new("A")).unwrap();
        assert!(!versions.sync(Some((first, "first")), Duration::ZERO));
        assert_eq!(versions.list.as_ref().unwrap().versions().len(), 2);

        assert!(versions.sync(Some((JobId::next(), "second")), Duration::from_secs(60)));
        assert_eq!(versions.started, Duration::from_secs(60));
        let list = versions.list.as_ref().unwrap();
        assert_eq!(list.versions().len(), 1);
        assert_eq!(list.selected_version().text, "second");

        assert!(versions.sync(None, Duration::ZERO));
        assert!(versions.list.is_none());
    }
}
