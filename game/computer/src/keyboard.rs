use bevy::{input::keyboard::NativeKeyCode, prelude::*};
use ld60_game_parts::keypad::KeyBinding::Code;
use ld60_game_parts::keypad::{KeyDef, Keypad, KeypadStyle};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use super::{CASE, CASE_DARK};

/// Base border around the keys.
const MARGIN: f32 = 0.012;
const BASE_HEIGHT: f32 = 0.018;
/// How far the floor the keys stand on lies below the cover around them.
const WELL: f32 = 0.004;
/// The floor between the keys, darker in the shade of the cover.
const FLOOR: Color = Color::srgb_u8(0x3a, 0x37, 0x32);

const STYLE: KeypadStyle = KeypadStyle {
    pitch: 0.0215,
    key_size: Vec3::new(0.018, 0.008, 0.017),
    travel: 0.003,
    label_cell: 96.0,
    font_size: 40.0,
    word_font_size: 22.0,
    ink: Color::srgb_u8(0x4a, 0x44, 0x3a),
    cap: CASE,
    shape: (0.0016, 0.0005),
    // The function keys sit apart from the main block.
    row_gaps: &[0.0, 0.5],
};

/// Rows from the back to the front.
const ROWS: &[&[KeyDef<()>]] = &[
    &[
        KeyDef::unit("ESC", (), &[Code(KeyCode::Escape)]),
        KeyDef::gap((), 1.0),
        KeyDef::unit("F1", (), &[Code(KeyCode::F1)]),
        KeyDef::unit("F2", (), &[Code(KeyCode::F2)]),
        KeyDef::unit("F3", (), &[Code(KeyCode::F3)]),
        KeyDef::unit("F4", (), &[Code(KeyCode::F4)]),
        KeyDef::gap((), 0.5),
        KeyDef::unit("F5", (), &[Code(KeyCode::F5)]),
        KeyDef::unit("F6", (), &[Code(KeyCode::F6)]),
        KeyDef::unit("F7", (), &[Code(KeyCode::F7)]),
        KeyDef::unit("F8", (), &[Code(KeyCode::F8)]),
        KeyDef::gap((), 0.5),
        KeyDef::unit("F9", (), &[Code(KeyCode::F9)]),
        KeyDef::unit("F10", (), &[Code(KeyCode::F10)]),
        KeyDef::unit("F11", (), &[Code(KeyCode::F11)]),
        KeyDef::unit("F12", (), &[Code(KeyCode::F12)]),
    ],
    &[
        KeyDef::unit("`", (), &[Code(KeyCode::Backquote)]),
        KeyDef::unit("1", (), &[Code(KeyCode::Digit1)]),
        KeyDef::unit("2", (), &[Code(KeyCode::Digit2)]),
        KeyDef::unit("3", (), &[Code(KeyCode::Digit3)]),
        KeyDef::unit("4", (), &[Code(KeyCode::Digit4)]),
        KeyDef::unit("5", (), &[Code(KeyCode::Digit5)]),
        KeyDef::unit("6", (), &[Code(KeyCode::Digit6)]),
        KeyDef::unit("7", (), &[Code(KeyCode::Digit7)]),
        KeyDef::unit("8", (), &[Code(KeyCode::Digit8)]),
        KeyDef::unit("9", (), &[Code(KeyCode::Digit9)]),
        KeyDef::unit("0", (), &[Code(KeyCode::Digit0)]),
        KeyDef::unit("-", (), &[Code(KeyCode::Minus)]),
        KeyDef::unit("=", (), &[Code(KeyCode::Equal)]),
        KeyDef::wide("BKSP", (), &[Code(KeyCode::Backspace)], 2.0),
    ],
    &[
        KeyDef::wide("TAB", (), &[Code(KeyCode::Tab)], 1.5),
        KeyDef::unit("Q", (), &[Code(KeyCode::KeyQ)]),
        KeyDef::unit("W", (), &[Code(KeyCode::KeyW)]),
        KeyDef::unit("E", (), &[Code(KeyCode::KeyE)]),
        KeyDef::unit("R", (), &[Code(KeyCode::KeyR)]),
        KeyDef::unit("T", (), &[Code(KeyCode::KeyT)]),
        KeyDef::unit("Y", (), &[Code(KeyCode::KeyY)]),
        KeyDef::unit("U", (), &[Code(KeyCode::KeyU)]),
        KeyDef::unit("I", (), &[Code(KeyCode::KeyI)]),
        KeyDef::unit("O", (), &[Code(KeyCode::KeyO)]),
        KeyDef::unit("P", (), &[Code(KeyCode::KeyP)]),
        KeyDef::unit("[", (), &[Code(KeyCode::BracketLeft)]),
        KeyDef::unit("]", (), &[Code(KeyCode::BracketRight)]),
        KeyDef::wide("\\", (), &[Code(KeyCode::Backslash)], 1.5),
    ],
    &[
        KeyDef::wide("CAPS", (), &[Code(KeyCode::CapsLock)], 1.75),
        KeyDef::unit("A", (), &[Code(KeyCode::KeyA)]),
        KeyDef::unit("S", (), &[Code(KeyCode::KeyS)]),
        KeyDef::unit("D", (), &[Code(KeyCode::KeyD)]),
        KeyDef::unit("F", (), &[Code(KeyCode::KeyF)]),
        KeyDef::unit("G", (), &[Code(KeyCode::KeyG)]),
        KeyDef::unit("H", (), &[Code(KeyCode::KeyH)]),
        KeyDef::unit("J", (), &[Code(KeyCode::KeyJ)]),
        KeyDef::unit("K", (), &[Code(KeyCode::KeyK)]),
        KeyDef::unit("L", (), &[Code(KeyCode::KeyL)]),
        KeyDef::unit(";", (), &[Code(KeyCode::Semicolon)]),
        KeyDef::unit("'", (), &[Code(KeyCode::Quote)]),
        KeyDef::wide("ENTER", (), &[Code(KeyCode::Enter)], 2.25),
    ],
    &[
        KeyDef::wide("SHIFT", (), &[Code(KeyCode::ShiftLeft)], 2.25),
        KeyDef::unit("Z", (), &[Code(KeyCode::KeyZ)]),
        KeyDef::unit("X", (), &[Code(KeyCode::KeyX)]),
        KeyDef::unit("C", (), &[Code(KeyCode::KeyC)]),
        KeyDef::unit("V", (), &[Code(KeyCode::KeyV)]),
        KeyDef::unit("B", (), &[Code(KeyCode::KeyB)]),
        KeyDef::unit("N", (), &[Code(KeyCode::KeyN)]),
        KeyDef::unit("M", (), &[Code(KeyCode::KeyM)]),
        KeyDef::unit(",", (), &[Code(KeyCode::Comma)]),
        KeyDef::unit(".", (), &[Code(KeyCode::Period)]),
        KeyDef::unit("/", (), &[Code(KeyCode::Slash)]),
        KeyDef::wide("SHIFT", (), &[Code(KeyCode::ShiftRight)], 2.75),
    ],
    &[
        KeyDef::wide("CTRL", (), &[Code(KeyCode::ControlLeft)], 1.5),
        KeyDef::wide("ALT", (), &[Code(KeyCode::AltLeft)], 1.5),
        KeyDef::wide("", (), &[Code(KeyCode::Space)], 9.0),
        KeyDef::wide("ALT", (), &[Code(KeyCode::AltRight)], 1.5),
        KeyDef::wide("CTRL", (), &[Code(KeyCode::ControlRight)], 1.5),
    ],
];

/// A keyboard with separate keys on a base, in its own local space. The
/// keys stand on a floor sunk into the base, and the cover of the base
/// frames the function keys and the main block.
pub(super) struct Keyboard {
    pub(super) base: Block,
    floor: Block,
    cover: Vec<Block>,
    keypad: Keypad<()>,
}

impl Keyboard {
    /// Lays out the keyboard with its base resting on the ground.
    pub(super) fn new() -> Self {
        let area = Keypad::area(ROWS, &STYLE);
        let size = area + 2.0 * MARGIN;
        let base = Block::new(size.extend(BASE_HEIGHT).xzy());
        let floor = Block::new(size.extend(BASE_HEIGHT - WELL).xzy());
        let keypad = Keypad::new(ROWS, &STYLE, &floor, Vec2::ZERO);
        let bar = |size: Vec2, center: Vec2| {
            Block::new(size.extend(WELL).xzy())
                .on_top_of(&floor)
                .translated(center.extend(0.0).xzy())
        };
        let side = Vec2::new(MARGIN, size.y);
        let end = Vec2::new(area.x, MARGIN);
        let edge = (area + Vec2::splat(MARGIN)) / 2.0;
        // The strip between the function keys and the main block.
        let gap = STYLE.row_gaps[1] * STYLE.pitch;
        let split = -area.y / 2.0 + STYLE.pitch + gap / 2.0;
        let cover = vec![
            bar(side, Vec2::new(-edge.x, 0.0)),
            bar(side, Vec2::new(edge.x, 0.0)),
            bar(end, Vec2::new(0.0, -edge.y)),
            bar(end, Vec2::new(0.0, edge.y)),
            bar(Vec2::new(area.x, gap), Vec2::new(0.0, split)),
        ];
        Self {
            base,
            floor,
            cover,
            keypad,
        }
    }

    /// Spawns the label canvas on `owner` and returns its texture.
    pub(super) fn spawn_labels(
        &self,
        commands: &mut Commands,
        images: &mut Assets<Image>,
        owner: Entity,
    ) -> Handle<Image> {
        self.keypad.spawn_labels(commands, images, owner)
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
                keyboard.spawn((Shape::cuboid(&self.floor, FLOOR), Finish::Plastic));
                for bar in &self.cover {
                    keyboard.spawn((Shape::cuboid(bar, CASE_DARK), Finish::Plastic));
                }
                self.keypad.spawn_keys(keyboard, &labels, false, |key| {
                    KeyboardKey(
                        key.code()
                            .unwrap_or(KeyCode::Unidentified(NativeKeyCode::Unidentified)),
                    )
                });
            });
    }
}

/// A key of the computer keyboard, at its position on a real keyboard.
#[derive(Component)]
pub(super) struct KeyboardKey(pub(super) KeyCode);
