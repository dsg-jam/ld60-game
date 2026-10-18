use bevy::{
    input::{InputSystems, keyboard::Key},
    input_focus::InputFocusSystems,
    prelude::*,
    window::{PrimaryWindow, WindowFocused},
};

use super::AppState;
use crate::player::focus::Focus;

const SHADE: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);

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
                .before(InputFocusSystems::Dispatch)
                .run_if(in_state(AppState::Paused)),
        )
        .add_systems(
            Update,
            (pause_on_escape, pause_on_blur).run_if(in_state(AppState::Playing)),
        )
        .add_systems(OnEnter(AppState::Paused), (spawn_shade, stop_time))
        .add_systems(OnExit(AppState::Paused), start_time);
        #[cfg(target_arch = "wasm32")]
        app.add_systems(
            Update,
            pause_on_lost_lock.run_if(in_state(AppState::Playing)),
        );
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
    mut events: MessageReader<WindowFocused>,
    primary: Query<(), With<PrimaryWindow>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if events
        .read()
        .any(|event| !event.focused && primary.contains(event.window))
    {
        next.set(AppState::Paused);
    }
}

/// Browsers take Escape to release the cursor lock and don't pass the key
/// on, so losing the lock pauses instead.
#[cfg(target_arch = "wasm32")]
fn pause_on_lost_lock(mut had_lock: Local<bool>, mut next: ResMut<NextState<AppState>>) {
    let locked = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.pointer_lock_element())
        .is_some();
    if *had_lock && !locked {
        next.set(AppState::Paused);
    }
    *had_lock = locked;
}

/// Keeps the game from seeing input while paused. A click or Escape goes
/// back to the game.
fn hold_input(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut logical: ResMut<ButtonInput<Key>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if mouse.just_pressed(MouseButton::Left) || keys.just_pressed(KeyCode::Escape) {
        next.set(AppState::Playing);
    }
    keys.reset_all();
    logical.reset_all();
    mouse.reset_all();
}

/// Darkens the view. It also catches the pointer, so nothing behind it
/// reacts to clicks.
fn spawn_shade(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(AppState::Paused),
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(SHADE),
        GlobalZIndex(1),
    ));
}

fn stop_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn start_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}
