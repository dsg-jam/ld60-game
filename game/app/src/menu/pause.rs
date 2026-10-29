use bevy::{
    ecs::message::{MessageUpdateSystems, signal_message_update_system},
    input::{
        InputSystems,
        keyboard::{Key, KeyboardInput},
        mouse::MouseWheel,
    },
    input_focus::InputFocusSystems,
    prelude::*,
    window::{PrimaryWindow, WindowFocused},
};
use ld60_game_core::state::KeepMouse;
use ld60_game_player::{focus::Focus, look::CursorLock};
use ld60_locale::Message;

use super::list::{Action, Navigation};
use super::{AppState, Column};

const SHADE: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);

/// The entries of the pause menu.
pub(super) const ENTRIES: &[(Action, &str)] = &[
    (Action::Resume, "menu-resume"),
    (Action::Settings, "menu-settings"),
    (Action::Restart, "menu-restart"),
    (Action::Title, "menu-title"),
    #[cfg(not(target_arch = "wasm32"))]
    (Action::Quit, "menu-quit"),
];

pub(super) struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PreUpdate,
            InputFocusSystems::Dispatch.run_if(in_state(AppState::Playing)),
        )
        .add_systems(
            PreUpdate,
            hold_input
                .after(InputSystems)
                .after(Navigation)
                .run_if(not(in_state(AppState::Playing))),
        )
        .add_systems(
            Update,
            (pause_on_escape, pause_on_blur, pause_on_lost_lock)
                .run_if(in_state(AppState::Playing)),
        )
        // Messages expire after fixed updates, which stop with virtual time.
        // Without this, systems that wait out the pause read its messages
        // after it, like a lost focus that pauses again.
        .add_systems(
            First,
            signal_message_update_system
                .before(MessageUpdateSystems)
                .run_if(|time: Res<Time<Virtual>>| time.is_paused()),
        )
        .add_systems(OnEnter(AppState::Paused), (spawn, stop_time))
        .add_systems(OnExit(AppState::Paused), start_time);
    }
}

/// Escape pauses at the desk. While focused, it leaves the focus instead.
fn pause_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<State<Focus>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) && *focus.get() == Focus::Desk {
        next.set(AppState::Paused);
    }
}

fn pause_on_blur(
    keep_mouse: Option<Res<KeepMouse>>,
    mut events: MessageReader<WindowFocused>,
    primary: Query<(), With<PrimaryWindow>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let blurred = events
        .read()
        .any(|event| !event.focused && primary.contains(event.window));
    if blurred && keep_mouse.is_none() {
        next.set(AppState::Paused);
    }
}

/// Browsers take Escape to release the cursor lock and don't pass the key
/// on, so losing the lock pauses instead.
fn pause_on_lost_lock(lock: Res<CursorLock>, mut next: ResMut<NextState<AppState>>) {
    if lock.lost() {
        next.set(AppState::Paused);
    }
}

/// Keeps the game from seeing input in a menu, up to and including the
/// frame that goes back to the game.
fn hold_input(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut logical: ResMut<ButtonInput<Key>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut keyboard: ResMut<Messages<KeyboardInput>>,
    mut wheel: ResMut<Messages<MouseWheel>>,
) {
    // Held buttons stay held, so key repeats don't count as new presses.
    keys.clear();
    logical.clear();
    mouse.clear();
    // Focused input is dispatched from these. Its reader falls behind while
    // paused and would see them after resuming.
    keyboard.clear();
    wheel.clear();
}

/// Darkens the view behind the pause menu. The shade also catches the
/// pointer, so nothing behind it reacts to clicks.
fn spawn(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(AppState::Paused),
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(SHADE),
    ));
    commands.spawn(Column::screen(
        AppState::Paused,
        Message::new("menu-paused"),
    ));
}

fn stop_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn start_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{
        input::InputPlugin,
        state::app::StatesPlugin,
        time::{TimePlugin, TimeUpdateStrategy},
    };

    use super::*;

    #[test]
    fn a_focus_lost_while_paused_does_not_pause_again() {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            TimePlugin,
            StatesPlugin,
            InputPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            16,
        )))
        .add_message::<WindowFocused>()
        .init_state::<AppState>()
        .init_state::<Focus>()
        .init_resource::<CursorLock>()
        .add_plugins(PausePlugin);
        let window = app.world_mut().spawn(PrimaryWindow).id();
        let focus = |app: &mut App, focused| {
            app.world_mut()
                .write_message(WindowFocused { window, focused });
            for _ in 0..5 {
                app.update();
            }
            *app.world().resource::<State<AppState>>().get()
        };
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        assert_eq!(focus(&mut app, true), AppState::Playing);
        assert_eq!(focus(&mut app, false), AppState::Paused);
        for focused in [true, false, true] {
            focus(&mut app, focused);
        }
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        assert_eq!(focus(&mut app, true), AppState::Playing);
    }
}
