use bevy::{ecs::relationship::RelatedSpawnerCommands, prelude::*, text::LineHeight};

use super::{cursor, look::Gaze};
use crate::locale::{Localized, Message};
use crate::menu::{AppState, Settings};

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
            .init_resource::<NextStep>()
            .add_systems(Startup, (BackHint::spawn, StepHint::spawn))
            .add_systems(Update, (BackHint::update, StepHint::update));
    }
}

/// A mouse button or key, drawn with Kenney's input prompt font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Icon {
    MouseLeft,
    MouseRight,
    Escape,
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

/// Says that right click or Escape goes back to the desk, while focused.
#[derive(Component, Debug)]
struct BackHint;

/// What the player should do next, if anything. Shown at the bottom of the
/// screen while playing.
#[derive(Resource, Default, Debug, PartialEq)]
pub(crate) struct NextStep(pub(crate) Option<Message>);

/// The line that shows the [`NextStep`].
#[derive(Component, Debug)]
struct StepHint;

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
        }
    }

    /// The outline alone. Drawn over [`Icon::glyph`], it leaves only the
    /// button in the accent color.
    const fn outline(self) -> Option<&'static str> {
        match self {
            Self::MouseLeft | Self::MouseRight => Some("\u{e0ee}"),
            Self::Escape => None,
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
                    bottom: px(CORNER),
                    align_items: AlignItems::Center,
                    column_gap: px(GAP),
                    ..default()
                },
                Pickable::IGNORE,
                Visibility::Hidden,
            ))
            .with_children(|hint| {
                Icon::MouseRight.spawn(hint, &font);
                Icon::Escape.spawn(hint, &font);
                hint.spawn((Caption, Localized::from(Message::new("hint-back"))));
            });
    }

    fn update(
        gaze: Gaze,
        settings: Res<Settings>,
        mut hint: Single<&mut Visibility, With<BackHint>>,
    ) {
        hint.set_if_neq(if gaze.focused() && settings.hints() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

impl StepHint {
    fn spawn(mut commands: Commands) {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                bottom: px(CORNER),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            children![(StepHint, Caption, TextColor(ACCENT), Visibility::Hidden)],
        ));
    }

    fn update(
        next: Res<NextStep>,
        state: Res<State<AppState>>,
        settings: Res<Settings>,
        hint: Single<(Entity, &mut Visibility, Option<&Localized>), With<StepHint>>,
        mut commands: Commands,
    ) {
        let (entity, mut visibility, current) = hint.into_inner();
        let shown = next
            .0
            .as_ref()
            .filter(|_| *state.get() == AppState::Playing && settings.hints());
        visibility.set_if_neq(if shown.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let Some(message) = shown {
            let text = Localized::from(message.clone());
            if current != Some(&text) {
                commands.entity(entity).insert(text);
            }
        }
    }
}
