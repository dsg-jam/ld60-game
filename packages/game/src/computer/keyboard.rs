use bevy::prelude::*;

use super::{CASE, CASE_DARK};
use crate::layout::Block;
use crate::shape::Shape;

/// Distance between the centers of two neighboring keys.
const KEY_PITCH: f32 = 0.0215;
/// Size of a key that is one unit wide.
const KEY_SIZE: Vec3 = Vec3::new(0.018, 0.008, 0.017);
/// How far a key moves down while pressed.
const KEY_TRAVEL: f32 = 0.003;
/// Base border around the keys.
const MARGIN: f32 = 0.012;
const BASE_HEIGHT: f32 = 0.018;

/// Label texture pixels per key unit.
const LABEL_CELL: f32 = 48.0;
const LABEL_FONT_SIZE: f32 = 20.0;
const LABEL_FONT_SIZE_WORD: f32 = 11.0;
const LABEL_INK: Color = Color::srgb_u8(0x4a, 0x44, 0x3a);

/// Rows from the back to the front.
const ROWS: [&[KeyDef]; 5] = [
    &[
        KeyDef::unit("`", KeyCode::Backquote),
        KeyDef::unit("1", KeyCode::Digit1),
        KeyDef::unit("2", KeyCode::Digit2),
        KeyDef::unit("3", KeyCode::Digit3),
        KeyDef::unit("4", KeyCode::Digit4),
        KeyDef::unit("5", KeyCode::Digit5),
        KeyDef::unit("6", KeyCode::Digit6),
        KeyDef::unit("7", KeyCode::Digit7),
        KeyDef::unit("8", KeyCode::Digit8),
        KeyDef::unit("9", KeyCode::Digit9),
        KeyDef::unit("0", KeyCode::Digit0),
        KeyDef::unit("-", KeyCode::Minus),
        KeyDef::unit("=", KeyCode::Equal),
        KeyDef::wide("BKSP", KeyCode::Backspace, 2.0),
    ],
    &[
        KeyDef::wide("TAB", KeyCode::Tab, 1.5),
        KeyDef::unit("Q", KeyCode::KeyQ),
        KeyDef::unit("W", KeyCode::KeyW),
        KeyDef::unit("E", KeyCode::KeyE),
        KeyDef::unit("R", KeyCode::KeyR),
        KeyDef::unit("T", KeyCode::KeyT),
        KeyDef::unit("Y", KeyCode::KeyY),
        KeyDef::unit("U", KeyCode::KeyU),
        KeyDef::unit("I", KeyCode::KeyI),
        KeyDef::unit("O", KeyCode::KeyO),
        KeyDef::unit("P", KeyCode::KeyP),
        KeyDef::unit("[", KeyCode::BracketLeft),
        KeyDef::unit("]", KeyCode::BracketRight),
        KeyDef::wide("\\", KeyCode::Backslash, 1.5),
    ],
    &[
        KeyDef::wide("CAPS", KeyCode::CapsLock, 1.75),
        KeyDef::unit("A", KeyCode::KeyA),
        KeyDef::unit("S", KeyCode::KeyS),
        KeyDef::unit("D", KeyCode::KeyD),
        KeyDef::unit("F", KeyCode::KeyF),
        KeyDef::unit("G", KeyCode::KeyG),
        KeyDef::unit("H", KeyCode::KeyH),
        KeyDef::unit("J", KeyCode::KeyJ),
        KeyDef::unit("K", KeyCode::KeyK),
        KeyDef::unit("L", KeyCode::KeyL),
        KeyDef::unit(";", KeyCode::Semicolon),
        KeyDef::unit("'", KeyCode::Quote),
        KeyDef::wide("ENTER", KeyCode::Enter, 2.25),
    ],
    &[
        KeyDef::wide("SHIFT", KeyCode::ShiftLeft, 2.25),
        KeyDef::unit("Z", KeyCode::KeyZ),
        KeyDef::unit("X", KeyCode::KeyX),
        KeyDef::unit("C", KeyCode::KeyC),
        KeyDef::unit("V", KeyCode::KeyV),
        KeyDef::unit("B", KeyCode::KeyB),
        KeyDef::unit("N", KeyCode::KeyN),
        KeyDef::unit("M", KeyCode::KeyM),
        KeyDef::unit(",", KeyCode::Comma),
        KeyDef::unit(".", KeyCode::Period),
        KeyDef::unit("/", KeyCode::Slash),
        KeyDef::wide("SHIFT", KeyCode::ShiftRight, 2.75),
    ],
    &[
        KeyDef::wide("CTRL", KeyCode::ControlLeft, 1.5),
        KeyDef::wide("ALT", KeyCode::AltLeft, 1.5),
        KeyDef::wide("", KeyCode::Space, 9.0),
        KeyDef::wide("ALT", KeyCode::AltRight, 1.5),
        KeyDef::wide("CTRL", KeyCode::ControlRight, 1.5),
    ],
];

/// Total width of each row in key units.
const ROW_UNITS: [f32; ROWS.len()] = {
    let mut units = [0.0; ROWS.len()];
    let mut row = 0;
    while row < ROWS.len() {
        let mut key = 0;
        while key < ROWS[row].len() {
            units[row] += ROWS[row][key].width;
            key += 1;
        }
        row += 1;
    }
    units
};

/// Width of the widest row in key units.
const MAX_ROW_UNITS: f32 = {
    let mut max = 0.0;
    let mut row = 0;
    while row < ROW_UNITS.len() {
        if ROW_UNITS[row] > max {
            max = ROW_UNITS[row];
        }
        row += 1;
    }
    max
};

pub(super) const BASE_SIZE: Vec3 = Vec3::new(
    MAX_ROW_UNITS * KEY_PITCH + 2.0 * MARGIN,
    BASE_HEIGHT,
    ROWS.len() as f32 * KEY_PITCH + 2.0 * MARGIN,
);

/// Size of the label texture. It is laid out like the keys, one cell per
/// key.
pub(super) const LABELS_SIZE: UVec2 = UVec2::new(
    (MAX_ROW_UNITS * LABEL_CELL) as u32,
    (ROWS.len() as f32 * LABEL_CELL) as u32,
);

/// One key: its label, the physical key it mirrors and its width in units.
struct KeyDef {
    label: &'static str,
    code: KeyCode,
    width: f32,
}

/// A keyboard with separate keys on a base, in its own local space.
pub(super) struct Keyboard {
    base: Block,
    keys: Vec<KeyLayout>,
}

struct KeyLayout {
    block: Block,
    code: KeyCode,
    /// Part of the label texture shown on the key top.
    label_region: Rect,
}

impl Keyboard {
    /// Lays out the keyboard with its base resting on the ground.
    pub(super) fn new() -> Self {
        let base = Block::new(BASE_SIZE);
        let back = base.min().z + MARGIN;
        let rows = ROWS.len() as f32;
        let mut keys = Vec::new();
        for (row, (defs, units)) in ROWS.iter().zip(ROW_UNITS).enumerate() {
            let row = row as f32;
            // Left edge of the current key, in units from the center.
            let mut left = -units / 2.0;
            for def in *defs {
                let span = Vec2::new(def.width, 1.0) * KEY_PITCH;
                let size = KEY_SIZE.with_x(span.x - (KEY_PITCH - KEY_SIZE.x));
                let center = Vec2::new(left + def.width / 2.0, row + 0.5) * KEY_PITCH;
                let block = Block::new(size).on_top_of(&base).translated(Vec3::new(
                    center.x,
                    0.0,
                    back + center.y,
                ));

                // Cells are laid out like the keys, so the label sits at the
                // same relative spot. The key top is smaller than its cell
                // by the gap to its neighbors.
                let cell = Rect::new(
                    (left + MAX_ROW_UNITS / 2.0) / MAX_ROW_UNITS,
                    row / rows,
                    (left + def.width + MAX_ROW_UNITS / 2.0) / MAX_ROW_UNITS,
                    (row + 1.0) / rows,
                );
                let label_region =
                    Rect::from_center_size(cell.center(), cell.size() * size.xz() / span);

                keys.push(KeyLayout {
                    block,
                    code: def.code,
                    label_region,
                });
                left += def.width;
            }
        }
        Self { base, keys }
    }

    pub(super) fn spawn(
        &self,
        parent: &mut ChildSpawnerCommands,
        transform: Transform,
        labels: Handle<Image>,
    ) {
        parent
            .spawn((transform, Visibility::default()))
            .with_children(|keyboard| {
                keyboard.spawn(Shape::cuboid(&self.base, CASE_DARK));
                for key in &self.keys {
                    keyboard.spawn((
                        Shape::textured_cuboid(&key.block, labels.clone(), key.label_region),
                        KeyCap {
                            code: key.code,
                            rest: key.block.center,
                        },
                    ));
                }
            });
    }

    /// Spawns the label texture UI into the canvas rendered by `camera`.
    pub(super) fn spawn_labels(commands: &mut Commands, camera: Entity) {
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(CASE),
                UiTargetCamera(camera),
            ))
            .with_children(|root| {
                for defs in ROWS {
                    root.spawn(Node {
                        height: px(LABEL_CELL),
                        ..default()
                    })
                    .with_children(|row| {
                        for def in defs {
                            row.spawn((
                                Node {
                                    width: px(def.width * LABEL_CELL),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                children![def.label_text()],
                            ));
                        }
                    });
                }
            });
    }
}

impl KeyDef {
    /// A key that is one unit wide.
    const fn unit(label: &'static str, code: KeyCode) -> Self {
        Self::wide(label, code, 1.0)
    }

    const fn wide(label: &'static str, code: KeyCode, width: f32) -> Self {
        Self { label, code, width }
    }

    fn label_text(&self) -> impl Bundle {
        let font_size = if self.label.chars().count() > 1 {
            LABEL_FONT_SIZE_WORD
        } else {
            LABEL_FONT_SIZE
        };
        (
            Text::new(self.label),
            TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            },
            TextColor(LABEL_INK),
        )
    }
}

/// A keyboard key that moves down while the matching real key is held.
#[derive(Component, Debug)]
pub(super) struct KeyCap {
    code: KeyCode,
    rest: Vec3,
}

impl KeyCap {
    pub(super) fn follow_input(
        keys: Res<ButtonInput<KeyCode>>,
        mut caps: Query<(&KeyCap, &mut Transform)>,
    ) {
        for (cap, mut transform) in &mut caps {
            let travel = if keys.pressed(cap.code) {
                KEY_TRAVEL
            } else {
                0.0
            };
            transform.translation = cap.rest - Vec3::Y * travel;
        }
    }
}
