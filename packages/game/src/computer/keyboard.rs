use bevy::prelude::*;

use super::{CASE, CASE_DARK};
use crate::keypad::KeyBinding::Code;
use crate::keypad::{KeyDef, Keypad, KeypadStyle};
use crate::layout::Block;
use crate::shape::Shape;

/// Base border around the keys.
const MARGIN: f32 = 0.012;
const BASE_HEIGHT: f32 = 0.018;

const STYLE: KeypadStyle = KeypadStyle {
    pitch: 0.0215,
    key_size: Vec3::new(0.018, 0.008, 0.017),
    travel: 0.003,
    label_cell: 48.0,
    font_size: 20.0,
    word_font_size: 11.0,
    ink: Color::srgb_u8(0x4a, 0x44, 0x3a),
    cap: CASE,
};

/// Rows from the back to the front.
const ROWS: &[&[KeyDef]] = &[
    &[
        KeyDef::unit("`", &[Code(KeyCode::Backquote)]),
        KeyDef::unit("1", &[Code(KeyCode::Digit1)]),
        KeyDef::unit("2", &[Code(KeyCode::Digit2)]),
        KeyDef::unit("3", &[Code(KeyCode::Digit3)]),
        KeyDef::unit("4", &[Code(KeyCode::Digit4)]),
        KeyDef::unit("5", &[Code(KeyCode::Digit5)]),
        KeyDef::unit("6", &[Code(KeyCode::Digit6)]),
        KeyDef::unit("7", &[Code(KeyCode::Digit7)]),
        KeyDef::unit("8", &[Code(KeyCode::Digit8)]),
        KeyDef::unit("9", &[Code(KeyCode::Digit9)]),
        KeyDef::unit("0", &[Code(KeyCode::Digit0)]),
        KeyDef::unit("-", &[Code(KeyCode::Minus)]),
        KeyDef::unit("=", &[Code(KeyCode::Equal)]),
        KeyDef::wide("BKSP", &[Code(KeyCode::Backspace)], 2.0),
    ],
    &[
        KeyDef::wide("TAB", &[Code(KeyCode::Tab)], 1.5),
        KeyDef::unit("Q", &[Code(KeyCode::KeyQ)]),
        KeyDef::unit("W", &[Code(KeyCode::KeyW)]),
        KeyDef::unit("E", &[Code(KeyCode::KeyE)]),
        KeyDef::unit("R", &[Code(KeyCode::KeyR)]),
        KeyDef::unit("T", &[Code(KeyCode::KeyT)]),
        KeyDef::unit("Y", &[Code(KeyCode::KeyY)]),
        KeyDef::unit("U", &[Code(KeyCode::KeyU)]),
        KeyDef::unit("I", &[Code(KeyCode::KeyI)]),
        KeyDef::unit("O", &[Code(KeyCode::KeyO)]),
        KeyDef::unit("P", &[Code(KeyCode::KeyP)]),
        KeyDef::unit("[", &[Code(KeyCode::BracketLeft)]),
        KeyDef::unit("]", &[Code(KeyCode::BracketRight)]),
        KeyDef::wide("\\", &[Code(KeyCode::Backslash)], 1.5),
    ],
    &[
        KeyDef::wide("CAPS", &[Code(KeyCode::CapsLock)], 1.75),
        KeyDef::unit("A", &[Code(KeyCode::KeyA)]),
        KeyDef::unit("S", &[Code(KeyCode::KeyS)]),
        KeyDef::unit("D", &[Code(KeyCode::KeyD)]),
        KeyDef::unit("F", &[Code(KeyCode::KeyF)]),
        KeyDef::unit("G", &[Code(KeyCode::KeyG)]),
        KeyDef::unit("H", &[Code(KeyCode::KeyH)]),
        KeyDef::unit("J", &[Code(KeyCode::KeyJ)]),
        KeyDef::unit("K", &[Code(KeyCode::KeyK)]),
        KeyDef::unit("L", &[Code(KeyCode::KeyL)]),
        KeyDef::unit(";", &[Code(KeyCode::Semicolon)]),
        KeyDef::unit("'", &[Code(KeyCode::Quote)]),
        KeyDef::wide("ENTER", &[Code(KeyCode::Enter)], 2.25),
    ],
    &[
        KeyDef::wide("SHIFT", &[Code(KeyCode::ShiftLeft)], 2.25),
        KeyDef::unit("Z", &[Code(KeyCode::KeyZ)]),
        KeyDef::unit("X", &[Code(KeyCode::KeyX)]),
        KeyDef::unit("C", &[Code(KeyCode::KeyC)]),
        KeyDef::unit("V", &[Code(KeyCode::KeyV)]),
        KeyDef::unit("B", &[Code(KeyCode::KeyB)]),
        KeyDef::unit("N", &[Code(KeyCode::KeyN)]),
        KeyDef::unit("M", &[Code(KeyCode::KeyM)]),
        KeyDef::unit(",", &[Code(KeyCode::Comma)]),
        KeyDef::unit(".", &[Code(KeyCode::Period)]),
        KeyDef::unit("/", &[Code(KeyCode::Slash)]),
        KeyDef::wide("SHIFT", &[Code(KeyCode::ShiftRight)], 2.75),
    ],
    &[
        KeyDef::wide("CTRL", &[Code(KeyCode::ControlLeft)], 1.5),
        KeyDef::wide("ALT", &[Code(KeyCode::AltLeft)], 1.5),
        KeyDef::wide("", &[Code(KeyCode::Space)], 9.0),
        KeyDef::wide("ALT", &[Code(KeyCode::AltRight)], 1.5),
        KeyDef::wide("CTRL", &[Code(KeyCode::ControlRight)], 1.5),
    ],
];

/// A keyboard with separate keys on a base, in its own local space.
pub(super) struct Keyboard {
    pub(super) base: Block,
    keypad: Keypad,
}

impl Keyboard {
    /// Lays out the keyboard with its base resting on the ground.
    pub(super) fn new() -> Self {
        let area = Keypad::area(ROWS, &STYLE);
        let base = Block::new((area + 2.0 * MARGIN).extend(BASE_HEIGHT).xzy());
        let keypad = Keypad::new(ROWS, &STYLE, &base, Vec2::ZERO);
        Self { base, keypad }
    }

    /// Spawns the label canvas and returns its texture.
    pub(super) fn spawn_labels(
        &self,
        commands: &mut Commands,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        self.keypad.spawn_labels(commands, images)
    }

    pub(super) fn spawn(
        &self,
        parent: &mut ChildSpawnerCommands,
        transform: Transform,
        labels: Handle<Image>,
    ) {
        parent
            .spawn((Name::new("keyboard"), transform, Visibility::default()))
            .with_children(|keyboard| {
                keyboard.spawn(Shape::cuboid(&self.base, CASE_DARK));
                self.keypad
                    .spawn_keys(keyboard, &labels, false, |_| KeyboardKey);
            });
    }
}

/// A key of the computer keyboard.
#[derive(Component)]
pub(super) struct KeyboardKey;
