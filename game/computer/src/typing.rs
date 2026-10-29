use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
    window::PrimaryWindow,
};
use ld60_game_core::parts::keypad::PressableKey;

use super::keyboard::KeyboardKey;

/// The printed characters of the keyboard, without and with Shift.
const CHARACTERS: &[(KeyCode, char, char)] = &[
    (KeyCode::Backquote, '`', '~'),
    (KeyCode::Digit1, '1', '!'),
    (KeyCode::Digit2, '2', '@'),
    (KeyCode::Digit3, '3', '#'),
    (KeyCode::Digit4, '4', '$'),
    (KeyCode::Digit5, '5', '%'),
    (KeyCode::Digit6, '6', '^'),
    (KeyCode::Digit7, '7', '&'),
    (KeyCode::Digit8, '8', '*'),
    (KeyCode::Digit9, '9', '('),
    (KeyCode::Digit0, '0', ')'),
    (KeyCode::Minus, '-', '_'),
    (KeyCode::Equal, '=', '+'),
    (KeyCode::BracketLeft, '[', '{'),
    (KeyCode::BracketRight, ']', '}'),
    (KeyCode::Backslash, '\\', '|'),
    (KeyCode::Semicolon, ';', ':'),
    (KeyCode::Quote, '\'', '"'),
    (KeyCode::Comma, ',', '<'),
    (KeyCode::Period, '.', '>'),
    (KeyCode::Slash, '/', '?'),
    (KeyCode::KeyA, 'a', 'A'),
    (KeyCode::KeyB, 'b', 'B'),
    (KeyCode::KeyC, 'c', 'C'),
    (KeyCode::KeyD, 'd', 'D'),
    (KeyCode::KeyE, 'e', 'E'),
    (KeyCode::KeyF, 'f', 'F'),
    (KeyCode::KeyG, 'g', 'G'),
    (KeyCode::KeyH, 'h', 'H'),
    (KeyCode::KeyI, 'i', 'I'),
    (KeyCode::KeyJ, 'j', 'J'),
    (KeyCode::KeyK, 'k', 'K'),
    (KeyCode::KeyL, 'l', 'L'),
    (KeyCode::KeyM, 'm', 'M'),
    (KeyCode::KeyN, 'n', 'N'),
    (KeyCode::KeyO, 'o', 'O'),
    (KeyCode::KeyP, 'p', 'P'),
    (KeyCode::KeyQ, 'q', 'Q'),
    (KeyCode::KeyR, 'r', 'R'),
    (KeyCode::KeyS, 's', 'S'),
    (KeyCode::KeyT, 't', 'T'),
    (KeyCode::KeyU, 'u', 'U'),
    (KeyCode::KeyV, 'v', 'V'),
    (KeyCode::KeyW, 'w', 'W'),
    (KeyCode::KeyX, 'x', 'X'),
    (KeyCode::KeyY, 'y', 'Y'),
    (KeyCode::KeyZ, 'z', 'Z'),
];

const SHIFT: [KeyCode; 2] = [KeyCode::ShiftLeft, KeyCode::ShiftRight];

/// Types with the mouse on the keyboard of the computer, as if the keys were
/// pressed on the real keyboard. The input goes the way of the real keys,
/// so it reaches whatever has the keyboard, like the editor.
///
/// Shift, Ctrl and Alt hold on for the next key, so a click on Shift and
/// then on a letter types it in upper case. Caps Lock stays on until it is
/// clicked again.
#[derive(Resource, Debug, Default)]
pub(super) struct MouseTyping {
    /// The modifiers that hold on for the next key.
    modifiers: Vec<(KeyCode, Key)>,
    caps: bool,
    /// The releases of the keys down under the mouse.
    held: Vec<KeyboardInput>,
    /// Whether the keys went down this frame, so they stay down for at
    /// least a frame.
    fresh: bool,
}

impl MouseTyping {
    pub(super) fn press_on_click(
        press: On<PointerPress>,
        keys: Query<(&KeyboardKey, &PressableKey)>,
        codes: Res<ButtonInput<KeyCode>>,
        window: Single<Entity, With<PrimaryWindow>>,
        mut typing: ResMut<MouseTyping>,
        mut inputs: MessageWriter<KeyboardInput>,
    ) {
        if press.button != PointerButton::Primary {
            return;
        }
        let Ok((&KeyboardKey(code), key)) = keys.get(press.entity) else {
            return;
        };
        if !key.active {
            return;
        }
        inputs.write_batch(typing.held.drain(..));
        let shift = codes.any_pressed(SHIFT);
        let (pressed, released) = typing.press(code, shift, *window);
        inputs.write_batch(pressed);
        typing.held = released;
        typing.fresh = true;
    }

    /// Lets go of the keys once the mouse button comes up.
    pub(super) fn release(
        mouse: Res<ButtonInput<MouseButton>>,
        mut typing: ResMut<MouseTyping>,
        mut inputs: MessageWriter<KeyboardInput>,
    ) {
        if typing.held.is_empty() {
            return;
        }
        if typing.fresh {
            typing.fresh = false;
        } else if !mouse.pressed(MouseButton::Left) {
            inputs.write_batch(typing.held.drain(..));
        }
    }

    /// The inputs for pressing the key at `code`, with Shift held on the
    /// real keyboard if `shift`, and those for releasing it afterwards.
    fn press(
        &mut self,
        code: KeyCode,
        shift: bool,
        window: Entity,
    ) -> (Vec<KeyboardInput>, Vec<KeyboardInput>) {
        let input = |code, logical_key, text: Option<char>| KeyboardInput {
            key_code: code,
            logical_key,
            state: ButtonState::Pressed,
            text: text.map(|c| c.to_string().into()),
            repeat: false,
            window,
        };
        if let Some(modifier) = modifier(code) {
            match self
                .modifiers
                .iter()
                .position(|(_, held)| *held == modifier)
            {
                Some(index) => {
                    self.modifiers.remove(index);
                }
                None => self.modifiers.push((code, modifier)),
            }
            return (Vec::new(), Vec::new());
        }
        if code == KeyCode::CapsLock {
            self.caps = !self.caps;
        }
        let shift = shift || self.modifiers.iter().any(|(_, held)| *held == Key::Shift);
        let mut pressed: Vec<KeyboardInput> = self
            .modifiers
            .drain(..)
            .map(|(held, logical_key)| input(held, logical_key, None))
            .collect();
        let (logical_key, text) = match code {
            KeyCode::Backspace => (Key::Backspace, None),
            KeyCode::Tab => (Key::Tab, None),
            KeyCode::Enter => (Key::Enter, None),
            KeyCode::CapsLock => (Key::CapsLock, None),
            KeyCode::Space => (Key::Space, Some(' ')),
            _ => match CHARACTERS.iter().find(|(key, ..)| *key == code) {
                Some(&(_, plain, shifted)) => {
                    let upper = if plain.is_ascii_alphabetic() {
                        shift != self.caps
                    } else {
                        shift
                    };
                    let c = if upper { shifted } else { plain };
                    (Key::Character(c.to_string().into()), Some(c))
                }
                None => return (Vec::new(), Vec::new()),
            },
        };
        pressed.push(input(code, logical_key, text));
        let released = pressed
            .iter()
            .rev()
            .map(|press| KeyboardInput {
                state: ButtonState::Released,
                text: None,
                ..press.clone()
            })
            .collect();
        (pressed, released)
    }
}

/// The logical key of a modifier key, or `None` for other keys.
fn modifier(code: KeyCode) -> Option<Key> {
    match code {
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Some(Key::Shift),
        KeyCode::ControlLeft | KeyCode::ControlRight => Some(Key::Control),
        KeyCode::AltLeft | KeyCode::AltRight => Some(Key::Alt),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(typing: &mut MouseTyping, code: KeyCode) -> Vec<(Key, Option<String>)> {
        let (pressed, released) = typing.press(code, false, Entity::PLACEHOLDER);
        assert_eq!(pressed.len(), released.len());
        assert!(
            pressed
                .iter()
                .all(|input| input.state == ButtonState::Pressed)
        );
        assert!(
            released
                .iter()
                .all(|input| input.state == ButtonState::Released)
        );
        let order =
            |inputs: &[KeyboardInput]| inputs.iter().map(|i| i.key_code).collect::<Vec<_>>();
        let mut back = order(&released);
        back.reverse();
        assert_eq!(order(&pressed), back);
        pressed
            .into_iter()
            .map(|input| (input.logical_key, input.text.map(|text| text.to_string())))
            .collect()
    }

    fn character(c: &str) -> (Key, Option<String>) {
        (Key::Character(c.into()), Some(c.into()))
    }

    #[test]
    fn keys_type_their_characters() {
        let mut typing = MouseTyping::default();
        assert_eq!(typed(&mut typing, KeyCode::KeyQ), [character("q")]);
        assert_eq!(typed(&mut typing, KeyCode::Digit1), [character("1")]);
        assert_eq!(
            typed(&mut typing, KeyCode::Space),
            [(Key::Space, Some(" ".into()))]
        );
        assert_eq!(
            typed(&mut typing, KeyCode::Backspace),
            [(Key::Backspace, None)]
        );
        assert_eq!(typed(&mut typing, KeyCode::Enter), [(Key::Enter, None)]);
    }

    #[test]
    fn shift_holds_on_for_one_key() {
        let mut typing = MouseTyping::default();
        assert!(typed(&mut typing, KeyCode::ShiftRight).is_empty());
        assert_eq!(
            typed(&mut typing, KeyCode::Digit2),
            [(Key::Shift, None), character("@")]
        );
        assert_eq!(typed(&mut typing, KeyCode::KeyA), [character("a")]);
        let (pressed, _) = typing.press(KeyCode::KeyA, true, Entity::PLACEHOLDER);
        assert_eq!(pressed[0].text.as_deref(), Some("A"));
    }

    #[test]
    fn a_second_click_lets_go_of_a_modifier() {
        let mut typing = MouseTyping::default();
        typed(&mut typing, KeyCode::ControlLeft);
        typed(&mut typing, KeyCode::ControlRight);
        assert_eq!(typed(&mut typing, KeyCode::KeyZ), [character("z")]);
        typed(&mut typing, KeyCode::ControlLeft);
        typed(&mut typing, KeyCode::ShiftLeft);
        assert_eq!(
            typed(&mut typing, KeyCode::KeyZ),
            [(Key::Control, None), (Key::Shift, None), character("Z")]
        );
    }

    #[test]
    fn caps_lock_shifts_letters_until_clicked_again() {
        let mut typing = MouseTyping::default();
        assert_eq!(
            typed(&mut typing, KeyCode::CapsLock),
            [(Key::CapsLock, None)]
        );
        assert_eq!(typed(&mut typing, KeyCode::KeyB), [character("B")]);
        assert_eq!(typed(&mut typing, KeyCode::Digit3), [character("3")]);
        typed(&mut typing, KeyCode::ShiftLeft);
        assert_eq!(
            typed(&mut typing, KeyCode::KeyB),
            [(Key::Shift, None), character("b")]
        );
        typed(&mut typing, KeyCode::CapsLock);
        assert_eq!(typed(&mut typing, KeyCode::KeyB), [character("b")]);
    }
}
