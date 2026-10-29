use bevy::{
    input::{InputSystems, keyboard::Key},
    prelude::*,
    winit::RawWinitWindowEvent,
};
use winit::{
    event::WindowEvent,
    keyboard::{KeyCode as WinitKeyCode, PhysicalKey},
};

use super::free_look::MODIFIERS;

pub(super) struct CtrlPlugin;

impl Plugin for CtrlPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeldCtrl>()
            .add_systems(PreUpdate, HeldCtrl::restore.after(InputSystems));
    }
}

/// Which of [`MODIFIERS`] are down, from key events alone.
///
/// Bevy also reads the modifiers of mouse events and releases Ctrl when they
/// leave it out. Firefox leaves it out of every mouse move while the pointer
/// is locked, so Ctrl would come up as soon as the mouse moves.
#[derive(Resource, Debug, Default)]
struct HeldCtrl([bool; 2]);

impl HeldCtrl {
    /// Presses Ctrl again where Bevy released it without a key event, as if
    /// it never came up.
    fn restore(
        mut events: MessageReader<RawWinitWindowEvent>,
        mut held: ResMut<HeldCtrl>,
        mut keys: ResMut<ButtonInput<KeyCode>>,
        mut logical: ResMut<ButtonInput<Key>>,
    ) {
        for RawWinitWindowEvent { event, .. } in events.read() {
            match event {
                WindowEvent::KeyboardInput { event, .. } => {
                    let side = match event.physical_key {
                        PhysicalKey::Code(WinitKeyCode::ControlLeft) => 0,
                        PhysicalKey::Code(WinitKeyCode::ControlRight) => 1,
                        _ => continue,
                    };
                    held.0[side] = event.state.is_pressed();
                }
                WindowEvent::Focused(false) => held.0 = [false; 2],
                _ => {}
            }
        }
        for (key, down) in MODIFIERS.into_iter().zip(held.0) {
            if down {
                Self::hold(&mut keys, key);
            }
        }
        if held.0.contains(&true) {
            Self::hold(&mut logical, Key::Control);
        }
    }

    fn hold<T: Clone + Eq + std::hash::Hash + Send + Sync + 'static>(
        input: &mut ButtonInput<T>,
        key: T,
    ) {
        if input.pressed(key.clone()) {
            return;
        }
        input.press(key.clone());
        if input.clear_just_released(key.clone()) {
            input.clear_just_pressed(key);
        }
    }
}
