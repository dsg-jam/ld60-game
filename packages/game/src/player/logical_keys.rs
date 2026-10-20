use bevy::{
    input::{
        ButtonState, InputSystems,
        keyboard::{Key, KeyboardFocusLost, KeyboardInput},
    },
    platform::collections::HashMap,
    prelude::*,
};

pub(super) struct LogicalKeysPlugin;

impl Plugin for LogicalKeysPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LogicalKeys>()
            .add_systems(PreUpdate, LogicalKeys::release.after(InputSystems));
    }
}

/// The logical key each physical key typed when it went down.
///
/// Bevy releases the logical key a key types when it comes up. Shift can
/// change that in between, so a key pressed as `c` and released as `C`
/// would stay down. Bevy also keeps logical keys down when the window loses
/// focus.
#[derive(Resource, Debug, Default)]
struct LogicalKeys(HashMap<KeyCode, Key>);

impl LogicalKeys {
    fn release(
        mut events: MessageReader<KeyboardInput>,
        mut focus_lost: MessageReader<KeyboardFocusLost>,
        mut pressed: ResMut<LogicalKeys>,
        mut keys: ResMut<ButtonInput<Key>>,
    ) {
        for event in events.read() {
            match event.state {
                ButtonState::Pressed => {
                    pressed.0.insert(event.key_code, event.logical_key.clone());
                }
                ButtonState::Released => {
                    if let Some(key) = pressed.0.remove(&event.key_code)
                        && key != event.logical_key
                    {
                        keys.release(key);
                    }
                }
            }
        }
        if focus_lost.read().count() > 0 {
            pressed.0.clear();
            keys.release_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::input::InputPlugin;

    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((InputPlugin, LogicalKeysPlugin));
        app
    }

    fn key(app: &mut App, text: &str, state: ButtonState) {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyC,
            logical_key: Key::Character(text.into()),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }

    fn c_is_down(app: &App) -> bool {
        let keys = app.world().resource::<ButtonInput<Key>>();
        keys.pressed(Key::Character("c".into())) || keys.pressed(Key::Character("C".into()))
    }

    #[test]
    fn a_key_released_with_shift_comes_up() {
        let mut app = app();
        key(&mut app, "c", ButtonState::Pressed);
        assert!(c_is_down(&app));
        key(&mut app, "C", ButtonState::Released);
        assert!(!c_is_down(&app));
    }

    #[test]
    fn a_focus_lost_releases_every_key() {
        let mut app = app();
        key(&mut app, "c", ButtonState::Pressed);
        app.world_mut().write_message(KeyboardFocusLost);
        app.update();
        assert!(!c_is_down(&app));
    }
}
