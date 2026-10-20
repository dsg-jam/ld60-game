use bevy::{ecs::relationship::RelatedSpawnerCommands, prelude::*, text::LineHeight};

use super::{
    cursor,
    focus::{Aside, Focused},
    look::Gaze,
};
use crate::locale::{Localized, Message};
use crate::menu::Settings;

const ICON_FONT: &str = "fonts/kenney-input-keyboard-mouse.ttf";
const ICON_SIZE: f32 = 64.0;
/// How high the glyphs reach above the baseline, relative to the icon font
/// size.
const GLYPH_HEIGHT: f32 = 0.46;
/// How high the font's line reaches above the baseline, relative to the icon
/// font size.
const ASCENT: f32 = 0.67;
const TEXT_SIZE: f32 = 18.0;
const GAP: f32 = 6.0;
/// From the bottom left corner to the hint for going back.
const CORNER: f32 = 24.0;
/// Between the hint for going back, the [`KeyHint`] and the [`Hints`].
const KEY_HINT_GAP: f32 = 18.0;
pub(crate) const INK: Color = cursor::DOT_FILL;
/// The button or key to press.
pub(crate) const ACCENT: Color = Color::srgb_u8(0xff, 0xb0, 0x50);
pub(crate) const SHADOW: TextShadow = TextShadow {
    offset: Vec2::new(1.0, 1.0),
    color: Color::srgba(0.0, 0.0, 0.0, 0.8),
};

pub(super) struct HintsPlugin;

impl Plugin for HintsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IconFont>()
            .init_resource::<KeyHint>()
            .init_resource::<ClickHint>()
            .add_systems(Startup, BackHint::spawn)
            .add_systems(Update, (BackHint::update, KeyHint::update, Hints::update));
    }
}

/// A mouse button or key, drawn with Kenney's input prompt font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Icon {
    MouseLeft,
    MouseRight,
    Escape,
    Wheel,
    /// Moving the mouse.
    Move,
    /// The left and right arrow keys.
    ArrowsSideways,
    /// The up and down arrow keys.
    ArrowsUpDown,
}

#[derive(Resource, Debug)]
pub(super) struct IconFont(Handle<Font>);

/// Text next to an [`Icon`].
#[derive(Component, Debug)]
#[require(
    Text,
    TextFont = TextFont::from_font_size(FontSize::Px(TEXT_SIZE)),
    TextColor(INK),
    TextShadow = SHADOW,
    TextLayout = TextLayout::no_wrap(),
    Pickable = Pickable::IGNORE,
)]
pub(super) struct Caption;

/// The line of hints at the bottom. While focused it starts with right
/// click or Escape going back to the desk.
#[derive(Component, Debug)]
struct BackHint;

/// The part of the [`BackHint`] that says how to go back.
#[derive(Component, Debug)]
struct BackPart;

/// A key that the focused thing reacts to and what it does, shown after
/// the hint for going back.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(crate) struct KeyHint(pub(crate) Option<(&'static str, Message)>);

/// What a left click does where the mouse is, shown at the cursor in
/// focus views.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(crate) struct ClickHint(pub(crate) Option<Message>);

/// What the inputs of the focused thing do, after the [`KeyHint`].
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub(crate) struct Hints(pub(crate) Vec<Hint>);

/// One or more inputs that do the same thing.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Hint {
    pub(crate) inputs: Vec<Input>,
    pub(crate) action: Message,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    Key(&'static str),
    Icon(Icon),
    /// A key held while using the mouse.
    Chord(&'static str, Icon),
    /// A key held while pressing another.
    Shortcut(&'static str, &'static str),
    /// Moving the mouse with the left button held.
    Drag,
}

/// Holds the rows of the [`Hints`].
#[derive(Component, Debug)]
struct HintList;

/// The parts of the [`KeyHint`] line.
#[derive(Component, Debug)]
enum KeyHintPart {
    Row,
    Key,
    Action,
}

impl FromWorld for IconFont {
    fn from_world(world: &mut World) -> Self {
        Self(world.resource::<AssetServer>().load(ICON_FONT))
    }
}

impl Icon {
    /// The outline with the button to press filled in.
    const fn glyph(self) -> &'static str {
        match self {
            Self::MouseLeft => "\u{e0ec}",
            Self::MouseRight => "\u{e0f0}",
            Self::Escape => "\u{e063}",
            Self::Wheel => "\u{e0f8}",
            Self::Move => "\u{e0ed}",
            Self::ArrowsSideways => "\u{e02a}",
            Self::ArrowsUpDown => "\u{e033}",
        }
    }

    /// The outline alone. Drawn over [`Icon::glyph`], it leaves only the
    /// button, the wheel or the arrow keys in the accent color.
    const fn outline(self) -> Option<&'static str> {
        match self {
            Self::MouseLeft | Self::MouseRight | Self::Wheel => Some("\u{e0ee}"),
            Self::ArrowsSideways | Self::ArrowsUpDown => Some("\u{e02d}"),
            Self::Escape | Self::Move => None,
        }
    }

    pub(super) fn spawn(self, parent: &mut RelatedSpawnerCommands<ChildOf>, font: &IconFont) {
        let glyph = |text, color| {
            (
                Text::new(text),
                TextFont {
                    font: font.0.clone().into(),
                    font_size: FontSize::Px(ICON_SIZE),
                    ..default()
                },
                LineHeight::RelativeToFont(GLYPH_HEIGHT),
                TextColor(color),
                Pickable::IGNORE,
            )
        };
        parent
            .spawn((
                Node::default(),
                UiTransform::from_translation(Val2 {
                    x: Val::ZERO,
                    y: px(-ICON_SIZE * (ASCENT - GLYPH_HEIGHT) / 2.0),
                }),
                Pickable::IGNORE,
            ))
            .with_children(|icon| {
                let Some(outline) = self.outline() else {
                    icon.spawn((glyph(self.glyph(), INK), SHADOW));
                    return;
                };
                icon.spawn((glyph(self.glyph(), ACCENT), SHADOW));
                icon.spawn((
                    glyph(outline, INK),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                ));
            });
    }
}

impl BackHint {
    fn spawn(mut commands: Commands, font: Res<IconFont>) {
        commands
            .spawn((
                BackHint,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(CORNER),
                    right: px(CORNER),
                    bottom: px(CORNER),
                    align_items: AlignItems::Center,
                    column_gap: px(GAP),
                    ..default()
                },
                Pickable::IGNORE,
                Visibility::Hidden,
            ))
            .with_children(|hint| {
                hint.spawn((
                    BackPart,
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(GAP),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|back| {
                    Icon::MouseRight.spawn(back, &font);
                    Icon::Escape.spawn(back, &font);
                    back.spawn((Caption, Localized::from(Message::new("hint-back"))));
                });
                hint.spawn((
                    KeyHintPart::Row,
                    Node {
                        display: Display::None,
                        align_items: AlignItems::Center,
                        column_gap: px(GAP),
                        margin: UiRect::left(px(KEY_HINT_GAP)),
                        ..default()
                    },
                    Pickable::IGNORE,
                    children![
                        key_cap((KeyHintPart::Key, Localized::verbatim(""))),
                        (KeyHintPart::Action, Caption, Localized::verbatim("")),
                    ],
                ));
                // Wraps onto more lines when the window is too narrow.
                hint.spawn((
                    HintList,
                    Node {
                        align_items: AlignItems::Center,
                        flex_wrap: FlexWrap::Wrap,
                        min_width: px(0),
                        row_gap: px(GAP),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            });
    }

    fn update(
        gaze: Gaze,
        settings: Res<Settings>,
        mut hint: Single<&mut Visibility, With<BackHint>>,
        mut back: Single<&mut Node, With<BackPart>>,
    ) {
        hint.set_if_neq(
            if (gaze.focused() || gaze.looking_around()) && settings.hints() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        );
        let display = if gaze.focused() {
            Display::Flex
        } else {
            Display::None
        };
        if back.display != display {
            back.display = display;
        }
    }
}

impl KeyHint {
    fn update(
        hint: Res<KeyHint>,
        mut parts: Query<(&KeyHintPart, &mut Node, Option<&mut Localized>)>,
    ) {
        if !hint.is_changed() {
            return;
        }
        for (part, mut node, text) in &mut parts {
            match (part, &hint.0, text) {
                (KeyHintPart::Row, action, _) => {
                    node.display = if action.is_some() {
                        Display::Flex
                    } else {
                        Display::None
                    };
                }
                (KeyHintPart::Key, Some((key, _)), Some(mut text)) => {
                    *text = Localized::verbatim(*key);
                }
                (KeyHintPart::Action, Some((_, action)), Some(mut text)) => {
                    *text = action.clone().into();
                }
                _ => {}
            }
        }
    }
}

impl Hints {
    /// The hints of the focused thing, then free look and zoom, which work
    /// in every view but the pinned sheets.
    fn update(
        gaze: Gaze,
        focused: Res<Focused>,
        hints: Query<(Option<&Hints>, Option<&Aside>)>,
        list: Single<Entity, With<HintList>>,
        font: Res<IconFont>,
        mut shown: Local<Hints>,
        mut commands: Commands,
    ) {
        let hints = focused
            .get()
            .and_then(|entity| hints.get(entity).ok())
            .map(|(hints, aside)| {
                let mut hints = hints.cloned().unwrap_or_default();
                hints.0.push(Hint {
                    inputs: vec![Input::Key("Ctrl")],
                    action: Message::new("hint-look-around"),
                });
                if !aside.is_some_and(|aside| aside.open) {
                    hints.0.push(Hint {
                        inputs: vec![Input::Chord("Ctrl", Icon::Wheel)],
                        action: Message::new("hint-zoom"),
                    });
                }
                hints
            })
            .unwrap_or_else(|| {
                Hints(if gaze.looking_around() {
                    vec![Hint {
                        inputs: vec![Input::Chord("Ctrl", Icon::Wheel)],
                        action: Message::new("hint-zoom"),
                    }]
                } else {
                    Vec::new()
                })
            });
        if *shown == hints {
            return;
        }
        commands
            .entity(*list)
            .despawn_children()
            .with_children(|list| {
                // At the desk nothing comes before the first hint.
                let lead = focused.get().is_none();
                for (index, hint) in hints.0.iter().enumerate() {
                    hint.spawn(list, &font, lead && index == 0);
                }
            });
        *shown = hints;
    }
}

impl Hint {
    fn spawn(&self, parent: &mut RelatedSpawnerCommands<ChildOf>, font: &IconFont, lead: bool) {
        parent
            .spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(GAP),
                    margin: UiRect::left(px(if lead { 0.0 } else { KEY_HINT_GAP })),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                for input in &self.inputs {
                    match *input {
                        Input::Key(key) => {
                            row.spawn(key_cap(Localized::verbatim(key)));
                        }
                        Input::Icon(icon) => icon.spawn(row, font),
                        Input::Chord(key, icon) => {
                            row.spawn(key_cap(Localized::verbatim(key)));
                            row.spawn((Caption, Localized::verbatim("+")));
                            icon.spawn(row, font);
                        }
                        Input::Shortcut(held, key) => {
                            row.spawn(key_cap(Localized::verbatim(held)));
                            row.spawn((Caption, Localized::verbatim("+")));
                            row.spawn(key_cap(Localized::verbatim(key)));
                        }
                        Input::Drag => {
                            Icon::MouseLeft.spawn(row, font);
                            row.spawn((Caption, Localized::verbatim("+")));
                            Icon::Move.spawn(row, font);
                        }
                    }
                }
                row.spawn((Caption, Localized::from(self.action.clone())));
            });
    }
}

/// A key to press, with its `label` in a box.
fn key_cap(label: impl Bundle) -> impl Bundle {
    (
        Node {
            padding: UiRect::axes(px(6), px(1)),
            border: px(2).all(),
            ..default()
        },
        BorderColor::all(INK),
        Pickable::IGNORE,
        children![(Caption, TextColor(ACCENT), label)],
    )
}
