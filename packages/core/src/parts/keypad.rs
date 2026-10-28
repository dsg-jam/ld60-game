mod stroke;

use bevy::{
    input::keyboard::Key,
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use self::stroke::Stroke;
use crate::parts::screen::UiCanvas;

pub struct KeypadPlugin;

impl Plugin for KeypadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (PressableKey::release_pointer, PressableKey::update).chain(),
        )
        .add_observer(PressableKey::press_with_pointer);
    }
}

/// A key with a printed label, pressed by the pointer or by `bindings` on
/// the keyboard.
pub struct KeyDef<K> {
    label: &'static str,
    key: K,
    bindings: &'static [KeyBinding],
    width: f32,
    /// Leaves the space empty instead of placing a key.
    gap: bool,
}

/// A keyboard key that presses a [`PressableKey`].
#[derive(Clone, Debug)]
pub enum KeyBinding {
    /// A key by its position, whatever the layout prints on it.
    Code(KeyCode),
    /// Whatever key types this text in the current layout.
    Typed(&'static str),
    /// A named key like Enter, in the current layout.
    Named(Key),
}

impl KeyBinding {
    fn is_pressed(&self, codes: &ButtonInput<KeyCode>, keys: &ButtonInput<Key>) -> bool {
        // A tap can press and release within one frame. `just_pressed` still
        // shows it for that frame.
        match self {
            Self::Code(code) => codes.pressed(*code) || codes.just_pressed(*code),
            Self::Typed(text) => {
                let key = Key::Character((*text).into());
                keys.pressed(key.clone()) || keys.just_pressed(key)
            }
            Self::Named(key) => keys.pressed(key.clone()) || keys.just_pressed(key.clone()),
        }
    }
}

impl<K> KeyDef<K> {
    /// A key that is one unit wide.
    pub const fn unit(label: &'static str, key: K, bindings: &'static [KeyBinding]) -> Self {
        Self::wide(label, key, bindings, 1.0)
    }

    pub const fn wide(
        label: &'static str,
        key: K,
        bindings: &'static [KeyBinding],
        width: f32,
    ) -> Self {
        Self {
            label,
            key,
            bindings,
            width,
            gap: false,
        }
    }

    /// Empty space as wide as `width` keys. `key` is never used.
    pub const fn gap(key: K, width: f32) -> Self {
        Self {
            label: "",
            key,
            bindings: &[],
            width,
            gap: true,
        }
    }

    fn label_text(&self, style: &KeypadStyle) -> impl Bundle {
        let font_size = if self.label.chars().count() > 1 {
            style.word_font_size
        } else {
            style.font_size
        };
        (
            Text::new(self.label),
            TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            },
            TextColor(style.ink),
        )
    }
}

/// Size and look of the keys of a [`Keypad`].
pub struct KeypadStyle {
    /// Distance between the centers of two neighboring keys.
    pub pitch: f32,
    /// Size of a key that is one unit wide.
    pub key_size: Vec3,
    /// How far a key moves down while pressed.
    pub travel: f32,
    /// Label texture pixels per key unit.
    pub label_cell: f32,
    pub font_size: f32,
    /// For labels longer than one character.
    pub word_font_size: f32,
    pub ink: Color,
    pub cap: Color,
}

/// Rows of keys with printed labels, centered in each row.
///
/// The labels are rendered into one texture laid out like the keys, so each
/// key top shows its own region of it.
pub struct Keypad<K: 'static> {
    rows: &'static [&'static [KeyDef<K>]],
    style: &'static KeypadStyle,
    keys: Vec<PlacedKey<K>>,
}

impl<K> PlacedKey<K> {
    /// The position on a real keyboard that presses the key, if any.
    pub fn code(&self) -> Option<KeyCode> {
        self.bindings.iter().find_map(|binding| match binding {
            KeyBinding::Code(code) => Some(*code),
            _ => None,
        })
    }
}

/// A key of a [`Keypad`], resting on its surface.
pub struct PlacedKey<K> {
    pub block: Block,
    pub key: K,
    bindings: &'static [KeyBinding],
    /// Part of the label texture shown on the key top.
    label_region: Rect,
}

impl<K: Copy> Keypad<K> {
    /// Area covered by the keys, in x and z.
    pub fn area(rows: &[&[KeyDef<K>]], style: &KeypadStyle) -> Vec2 {
        Vec2::new(Self::max_units(rows), rows.len() as f32) * style.pitch
    }

    /// Lays out the keys resting on `surface`, with their area centered on
    /// `spot` (see [`Block::top_spot`]).
    pub fn new(
        rows: &'static [&'static [KeyDef<K>]],
        style: &'static KeypadStyle,
        surface: &Block,
        spot: Vec2,
    ) -> Self {
        let area = Self::area(rows, style);
        let origin = surface.top_spot(spot) - (area / 2.0).extend(0.0).xzy();
        let max_units = Self::max_units(rows);
        let row_count = rows.len() as f32;
        let mut keys = Vec::new();
        for (row, defs) in rows.iter().enumerate() {
            let row = row as f32;
            let units = Self::units(defs);
            // Left edge of the current key, in units from the left of the area.
            let mut left = (max_units - units) / 2.0;
            for def in *defs {
                if def.gap {
                    left += def.width;
                    continue;
                }
                let span = Vec2::new(def.width, 1.0) * style.pitch;
                let size = style
                    .key_size
                    .with_x(span.x - (style.pitch - style.key_size.x));
                let center = Vec2::new(left + def.width / 2.0, row + 0.5) * style.pitch;
                let block = Block::new(size).standing_at(origin + center.extend(0.0).xzy());

                // The key top is smaller than its cell by the gap to its
                // neighbors.
                let cell = Rect::new(
                    left / max_units,
                    row / row_count,
                    (left + def.width) / max_units,
                    (row + 1.0) / row_count,
                );
                let label_region =
                    Rect::from_center_size(cell.center(), cell.size() * size.xz() / span);

                keys.push(PlacedKey {
                    block,
                    key: def.key,
                    bindings: def.bindings,
                    label_region,
                });
                left += def.width;
            }
        }
        Self { rows, style, keys }
    }

    pub fn keys(&self) -> &[PlacedKey<K>] {
        &self.keys
    }

    /// Spawns the keys under `parent`, each with the bundle `extra` returns
    /// for it.
    pub fn spawn_keys<B: Bundle>(
        &self,
        parent: &mut ChildSpawnerCommands,
        labels: &Handle<Image>,
        active: bool,
        extra: impl Fn(&PlacedKey<K>) -> B,
    ) {
        for key in &self.keys {
            parent.spawn((
                Shape::textured_cuboid(&key.block, labels.clone(), key.label_region),
                Finish::Plastic,
                PressableKey {
                    rest: key.block.center,
                    width: key.block.size.x,
                    travel: self.style.travel,
                    bindings: key.bindings,
                    active,
                    held_by_pointer: false,
                    tapped_by_pointer: false,
                    pressed: false,
                    stroke: Stroke::default(),
                },
                extra(key),
            ));
        }
    }

    /// Spawns the label canvas for the keypad on `owner` and returns its
    /// texture.
    pub fn spawn_labels(
        &self,
        commands: &mut Commands,
        images: &mut Assets<Image>,
        owner: Entity,
    ) -> Handle<Image> {
        let cell = self.style.label_cell;
        let size = (Self::area(self.rows, self.style) / self.style.pitch * cell).as_uvec2();
        let canvas = UiCanvas::spawn(commands, images, size, owner);
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(self.style.cap),
                canvas.target(),
            ))
            .with_children(|root| {
                for defs in self.rows {
                    root.spawn(Node {
                        height: px(cell),
                        ..default()
                    })
                    .with_children(|row| {
                        for def in *defs {
                            row.spawn((
                                Node {
                                    width: px(def.width * cell),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                children![def.label_text(self.style)],
                            ));
                        }
                    });
                }
            });
        canvas.image
    }

    fn units(row: &[KeyDef<K>]) -> f32 {
        row.iter().map(|def| def.width).sum()
    }

    fn max_units(rows: &[&[KeyDef<K>]]) -> f32 {
        rows.iter().map(|row| Self::units(row)).fold(0.0, f32::max)
    }
}

/// A key that moves down while held by the pointer or by one of its key
/// bindings. Inactive keys ignore both.
///
/// The key is pressed as soon as the input says so. It moves down and back
/// up over a short [`Stroke`].
#[derive(Component, Debug)]
pub struct PressableKey {
    rest: Vec3,
    width: f32,
    travel: f32,
    bindings: &'static [KeyBinding],
    pub active: bool,
    held_by_pointer: bool,
    /// Set on a pointer press until the next update, so a click that
    /// presses and releases within one frame still shows.
    tapped_by_pointer: bool,
    pressed: bool,
    stroke: Stroke,
}

impl PressableKey {
    pub fn is_pressed(&self) -> bool {
        self.pressed
    }

    /// In meters.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// The press keeps bubbling, so a click on a key still reaches its
    /// device or computer.
    fn press_with_pointer(press: On<PointerPress>, mut keys: Query<&mut PressableKey>) {
        if press.button == PointerButton::Primary
            && let Ok(mut key) = keys.get_mut(press.entity)
            && key.active
        {
            key.held_by_pointer = true;
            key.tapped_by_pointer = true;
        }
    }

    /// Lets go of every key when the mouse button goes up, wherever the
    /// pointer is.
    fn release_pointer(mouse: Res<ButtonInput<MouseButton>>, mut keys: Query<&mut PressableKey>) {
        if mouse.just_released(MouseButton::Left) {
            for mut key in &mut keys {
                key.held_by_pointer = false;
            }
        }
    }

    pub fn update(
        time: Res<Time>,
        codes: Res<ButtonInput<KeyCode>>,
        keys: Res<ButtonInput<Key>>,
        mut caps: Query<(&mut PressableKey, &mut Transform)>,
    ) {
        for (mut cap, mut transform) in &mut caps {
            let pressed = cap.held_by_pointer
                || cap.tapped_by_pointer
                || (cap.active
                    && cap
                        .bindings
                        .iter()
                        .any(|binding| binding.is_pressed(&codes, &keys)));
            if cap.tapped_by_pointer {
                cap.tapped_by_pointer = false;
            }
            if pressed != cap.pressed {
                cap.pressed = pressed;
            }
            let stroke = cap.stroke.step(pressed, time.delta_secs());
            if stroke != cap.stroke {
                cap.stroke = stroke;
                transform.translation = cap.rest - Vec3::Y * cap.travel * stroke.depth();
            }
        }
    }
}
