use bevy::{
    input::InputSystems,
    picking::{
        events::{PointerClick, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};

use super::AppState;
use super::settings::{Adjust, Setting};
use crate::locale::{Localized, Message};
use crate::player::hints::{ACCENT, INK, SHADOW};

const ENTRY_SIZE: f32 = 28.0;
const ENTRY_GAP: f32 = 10.0;
/// The bar beside the selected entry.
const MARK: Vec2 = Vec2::new(4.0, 26.0);
const MARK_GAP: f32 = 14.0;

pub(super) struct ListPlugin;

impl Plugin for ListPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            Entries::navigate
                .in_set(Navigation)
                .after(InputSystems)
                .run_if(not(in_state(AppState::Playing))),
        )
        .add_systems(Update, Entry::show)
        .add_observer(Entry::select_on_hover)
        .add_observer(Entry::pick_on_click);
    }
}

/// Reads the keys for the menu, before the game's input is held back.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Navigation;

/// A column of entries. The selected one is the one the mouse rests on, or
/// the one the arrow keys moved to. A click or Enter picks it.
#[derive(Component, Debug)]
#[require(Node = Entries::node())]
pub(super) struct Entries;

/// One entry in [`Entries`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Entry(pub(super) Action);

/// Marks the selected [`Entry`].
#[derive(Component, Debug)]
struct Selected;

/// The bar beside an [`Entry`] that shows it is selected.
#[derive(Component, Debug)]
struct Mark;

/// What picking an [`Entry`] does.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Start,
    Resume,
    Settings,
    Credits,
    /// Goes back from a page, or out of the pause menu.
    Back,
    /// Goes to the title screen.
    Title,
    /// Asks before a new run starts.
    Restart,
    /// Starts a new run.
    NewRun,
    /// Gives up the current run for one from the start.
    StartOver,
    /// Browsers have no way to close the game, so only native builds quit.
    #[cfg(not(target_arch = "wasm32"))]
    Quit,
    /// Steps a setting, and wraps around at the end.
    Setting(Setting),
}

impl Entries {
    /// Entries that each do an action, labeled with a message.
    pub(super) fn bundle(entries: &[(Action, &'static str)]) -> impl Bundle {
        let entries: Vec<_> = entries
            .iter()
            .map(|&(action, label)| Entry::bundle(action, Self::label(Message::new(label))))
            .collect();
        (Entries, Children::spawn(SpawnIter(entries.into_iter())))
    }

    fn node() -> Node {
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            row_gap: px(ENTRY_GAP),
            ..default()
        }
    }

    /// The text of an entry.
    pub(super) fn label(text: impl Into<Localized>) -> impl Bundle {
        (
            text.into(),
            TextFont::from_font_size(FontSize::Px(ENTRY_SIZE)),
            TextColor(INK),
            SHADOW,
            Pickable::IGNORE,
        )
    }

    /// Moves the selection with the arrow keys, picks with Enter and goes
    /// back with Escape.
    fn navigate(
        keys: Res<ButtonInput<KeyCode>>,
        lists: Query<&Children, With<Entries>>,
        entries: Query<(&Entry, Has<Selected>)>,
        mut commands: Commands,
    ) {
        if keys.just_pressed(KeyCode::Escape) {
            commands.trigger(Action::Back);
        }
        let Some(children) = lists.iter().next() else {
            return;
        };
        let list: Vec<_> = children
            .iter()
            .filter_map(|child| Some((child, entries.get(child).ok()?)))
            .collect();
        let Some(selected) = list.iter().position(|(_, (_, selected))| *selected) else {
            return;
        };
        let (entity, (&Entry(action), _)) = list[selected];
        let step = |key| i32::from(keys.just_pressed(key));
        let moved = step(KeyCode::ArrowDown) - step(KeyCode::ArrowUp);
        if moved != 0 {
            let next = (selected as i32 + moved).rem_euclid(list.len() as i32) as usize;
            Entry::select(list[next].0, &mut commands);
        }
        if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space]) {
            Entry::pick(entity, action, &mut commands);
        }
        let adjust = step(KeyCode::ArrowRight) - step(KeyCode::ArrowLeft);
        if let (Action::Setting(setting), true) = (action, adjust != 0) {
            commands.trigger(Adjust {
                setting,
                step: adjust,
                wrap: false,
            });
        }
    }
}

impl Entry {
    /// An entry that does `action`, with `label` and anything else in
    /// `content` beside the mark.
    pub(super) fn bundle(action: Action, content: impl Bundle) -> impl Bundle {
        (
            Entry(action),
            Node {
                align_items: AlignItems::Center,
                column_gap: px(MARK_GAP),
                ..default()
            },
            children![
                (
                    Mark,
                    Node {
                        width: px(MARK.x),
                        height: px(MARK.y),
                        ..default()
                    },
                    BackgroundColor(ACCENT),
                    Visibility::Hidden,
                    Pickable::IGNORE,
                ),
                content,
            ],
        )
    }

    fn select(entity: Entity, commands: &mut Commands) {
        commands.queue(move |world: &mut World| {
            let selected: Vec<_> = world
                .query_filtered::<Entity, With<Selected>>()
                .iter(world)
                .collect();
            for other in selected {
                world.entity_mut(other).remove::<Selected>();
            }
            world.entity_mut(entity).insert(Selected);
        });
    }

    fn pick(entity: Entity, action: Action, commands: &mut Commands) {
        match action {
            Action::Setting(setting) => commands.trigger(Adjust {
                setting,
                step: 1,
                wrap: true,
            }),
            _ => commands.trigger(action),
        }
        Self::select(entity, commands);
    }

    fn select_on_hover(
        over: On<PointerOver>,
        entries: Query<(), With<Entry>>,
        mut commands: Commands,
    ) {
        if entries.contains(over.entity) {
            Self::select(over.entity, &mut commands);
        }
    }

    fn pick_on_click(click: On<PointerClick>, entries: Query<&Entry>, mut commands: Commands) {
        if click.button == PointerButton::Primary
            && let Ok(&Entry(action)) = entries.get(click.entity)
        {
            Self::pick(click.entity, action, &mut commands);
        }
    }

    /// Selects the first entry of a new list, and shows which entry is
    /// selected.
    fn show(
        lists: Query<&Children, With<Entries>>,
        entries: Query<(Entity, Has<Selected>), With<Entry>>,
        tree: Query<&Children>,
        mut marks: Query<&mut Visibility, With<Mark>>,
        mut texts: Query<&mut TextColor>,
        mut commands: Commands,
    ) {
        for children in &lists {
            let mut list = children.iter().filter_map(|child| entries.get(child).ok());
            let first = list.clone().next();
            if let (Some((first, _)), false) = (first, list.any(|(_, selected)| selected)) {
                Self::select(first, &mut commands);
            }
        }
        for (entry, selected) in &entries {
            let color = if selected { ACCENT } else { INK };
            let shown = if selected {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            for part in tree.iter_descendants(entry) {
                if let Ok(mut mark) = marks.get_mut(part) {
                    mark.set_if_neq(shown);
                }
                if let Ok(mut text) = texts.get_mut(part) {
                    text.set_if_neq(TextColor(color));
                }
            }
        }
    }
}
