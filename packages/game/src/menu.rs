mod closed;
mod credits;
mod list;
mod pause;
mod restart;
mod settings;
mod title;

use bevy::prelude::*;

use self::closed::StatsPage;
use self::credits::{Credits, CreditsHandle, CreditsPage};
use self::list::{Action, Entries};
use self::restart::RestartPage;
pub(crate) use self::settings::Settings;
use self::settings::SettingsPage;
use crate::audio::Mixer;
use crate::locale::{Localized, Message};
use crate::player::hints::{INK, SHADOW};
use crate::workshop::run::{NewRun, Run};

const HEADING_SIZE: f32 = 56.0;
/// From the left edge of the screen to the column.
const INSET: f32 = 8.0;
const ROW_GAP: f32 = 28.0;
/// The band behind the column, dark at the left edge and clear from here on.
const BAND_END: f32 = 55.0;
const BAND: Color = Color::srgba(0.0, 0.0, 0.0, 0.8);

pub(crate) struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_sub_state::<Page>()
            .init_resource::<Started>()
            .add_plugins((
                closed::ClosedPlugin,
                credits::CreditsPlugin,
                list::ListPlugin,
                pause::PausePlugin,
                settings::SettingsPlugin,
                title::TitlePlugin,
            ))
            .add_systems(
                Update,
                // From the pause menu to the title screen, the page stays the
                // same but the column is new.
                Column::fill
                    .run_if(state_changed::<Page>.or_eager(state_changed::<AppState>))
                    .run_if(resource_exists::<State<Page>>),
            )
            .add_systems(OnEnter(AppState::Playing), Mixer::duck::<false>)
            .add_systems(OnExit(AppState::Playing), Mixer::duck::<true>)
            .add_observer(Action::apply);
    }
}

/// Whether the game runs or waits behind a menu.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum AppState {
    /// The title screen, shown at the start.
    #[default]
    Title,
    Playing,
    Paused,
    /// The landlord closed the shop, so the run is over.
    Closed,
}

/// The page a menu shows.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Title | AppState::Paused | AppState::Closed)]
pub(crate) enum Page {
    #[default]
    Entries,
    Settings,
    Credits,
    /// Asks before a new run replaces the current one.
    Restart,
}

/// Whether the player has started playing.
#[derive(Resource, Debug, Default)]
struct Started(bool);

/// Where a menu shows its pages, on a dark band at the left of the screen.
#[derive(Component, Debug)]
struct Column;

impl Column {
    /// The screen of the menu in `state`, with `heading` above the column.
    fn screen(state: AppState, heading: Message) -> impl Bundle {
        (
            DespawnOnExit(state),
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding: UiRect::left(vw(INSET)),
                row_gap: px(ROW_GAP),
                ..default()
            },
            BackgroundGradient::from(LinearGradient::to_right(vec![
                ColorStop::new(BAND, percent(0)),
                ColorStop::new(BAND.with_alpha(0.0), percent(BAND_END)),
            ])),
            GlobalZIndex(1),
            children![
                (
                    Localized::from(heading),
                    TextFont::from_font_size(FontSize::Px(HEADING_SIZE)),
                    TextColor(INK),
                    SHADOW,
                ),
                (
                    Column,
                    Node {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                ),
            ],
        )
    }

    /// Shows the current page in the column.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn fill(
        app_state: Res<State<AppState>>,
        page: Res<State<Page>>,
        started: Res<Started>,
        run: Res<Run>,
        credits: Res<Assets<Credits>>,
        credits_handle: Res<CreditsHandle>,
        column: Single<Entity, With<Column>>,
        mut commands: Commands,
    ) {
        let mut column = commands.entity(*column);
        match (*app_state.get(), *page.get()) {
            (_, Page::Credits) => {
                column.with_child((
                    DespawnOnExit(Page::Credits),
                    CreditsPage::bundle(credits_handle.get(&credits)),
                ));
            }
            (_, Page::Settings) => {
                column.with_child((DespawnOnExit(Page::Settings), SettingsPage::bundle()));
            }
            (_, Page::Restart) => {
                column.with_child((DespawnOnExit(Page::Restart), RestartPage::bundle()));
            }
            (AppState::Closed, Page::Entries) => {
                column.with_child((
                    DespawnOnExit(Page::Entries),
                    StatsPage::with_entries(run.stats()),
                ));
            }
            (AppState::Paused, Page::Entries) => {
                column.with_child((
                    DespawnOnExit(Page::Entries),
                    Entries::bundle(pause::ENTRIES),
                ));
            }
            (_, Page::Entries) => {
                column.with_child((
                    DespawnOnExit(Page::Entries),
                    Entries::bundle(if started.0 || run.underway() {
                        title::CONTINUE_ENTRIES
                    } else {
                        title::START_ENTRIES
                    }),
                ));
            }
        }
    }
}

impl Action {
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn apply(
        action: On<Action>,
        app_state: Res<State<AppState>>,
        page: Option<Res<State<Page>>>,
        mut next_state: ResMut<NextState<AppState>>,
        mut next_page: ResMut<NextState<Page>>,
        mut started: ResMut<Started>,
        mut commands: Commands,
        #[cfg(not(target_arch = "wasm32"))] mut exit: MessageWriter<AppExit>,
    ) {
        match *action {
            Self::Start | Self::Resume => {
                started.0 = true;
                next_state.set(AppState::Playing);
            }
            Self::NewRun => {
                started.0 = true;
                commands.trigger(NewRun);
                next_state.set(AppState::Playing);
            }
            Self::Settings => next_page.set(Page::Settings),
            Self::Credits => next_page.set(Page::Credits),
            Self::Restart => next_page.set(Page::Restart),
            Self::Back => match (*app_state.get(), page.map(|page| *page.get())) {
                (_, Some(Page::Settings | Page::Credits | Page::Restart)) => {
                    next_page.set(Page::Entries)
                }
                // Browsers only lock the cursor after a click.
                (AppState::Paused, _) if cfg!(not(target_arch = "wasm32")) => {
                    next_state.set(AppState::Playing);
                }
                _ => {}
            },
            Self::Title => next_state.set(AppState::Title),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Quit => {
                exit.write(AppExit::Success);
            }
            Self::Setting(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::settings::Setting;
    use super::*;
    use crate::locale::Locale;

    #[test]
    fn every_menu_text_exists() {
        let mut texts = Localized::spawned_by(|menu| {
            menu.spawn(Column::screen(AppState::Title, Message::new("menu-game")));
            menu.spawn(Column::screen(
                AppState::Paused,
                Message::new("menu-paused"),
            ));
            menu.spawn(Column::screen(
                AppState::Closed,
                Message::new("menu-closed"),
            ));
            menu.spawn(StatsPage::with_entries(Run::default().stats()));
            for entries in [
                title::START_ENTRIES,
                title::CONTINUE_ENTRIES,
                pause::ENTRIES,
            ] {
                menu.spawn(Entries::bundle(entries));
            }
            menu.spawn(SettingsPage::bundle());
            menu.spawn(RestartPage::bundle());
        });
        let settings = Settings::default();
        let toggled = Settings::toggled_hints();
        for setting in Setting::ALL {
            texts.push(settings.value(setting).into());
            texts.push(toggled.value(setting).into());
        }
        Locale::on_disk().assert_formats(texts);
    }

    #[test]
    fn a_new_run_is_asked_for_first() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .add_sub_state::<Page>()
            .init_resource::<Started>()
            .init_resource::<Run>()
            .add_message::<AppExit>()
            .add_observer(Action::apply)
            .add_observer(|_: On<NewRun>, mut run: ResMut<Run>| *run = run.next());
        let mut pick = |action| {
            app.world_mut().trigger(action);
            app.update();
            let world = app.world();
            (
                *world.resource::<State<AppState>>().get(),
                world.get_resource::<State<Page>>().map(|page| *page.get()),
            )
        };
        assert_eq!(
            pick(Action::Restart),
            (AppState::Title, Some(Page::Restart))
        );
        assert_eq!(pick(Action::Back), (AppState::Title, Some(Page::Entries)));
        pick(Action::Restart);
        assert_eq!(pick(Action::NewRun), (AppState::Playing, None));
        assert!(app.world().resource::<Started>().0);
        assert_eq!(*app.world().resource::<Run>(), Run::default().next());
    }
}
