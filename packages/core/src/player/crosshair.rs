mod badge;

use bevy::{
    picking::{
        hover::HoverMap,
        pointer::{PointerId, PointerLocation},
    },
    prelude::*,
    window::PrimaryWindow,
};
use ld60_locale::{Localized, Message};

use self::badge::ToolBadge;
use super::{
    cursor,
    focus::{self, ClickTool, Clickable, Focused},
    hints::{Caption, ClickHint, Icon, IconFont},
    look::{Gaze, KeepMouse},
};
use crate::settings::Settings;

pub(super) struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Prompt>()
            .add_systems(Startup, Crosshair::spawn)
            .add_systems(
                Update,
                (Prompt::update, (Crosshair::update, ToolBadge::update)).chain(),
            );
    }
}

/// Marks what a click hits, in the middle of the view while looking around.
///
/// Where a click does something the dot turns into the left mouse button,
/// with what a click does below it. See [`Prompt`].
#[derive(Component, Debug)]
struct Crosshair;

/// The two looks of the [`Crosshair`]. Only one of them is displayed.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Dot,
    Prompt,
}

/// The [`Caption`] of [`Shape::Prompt`].
#[derive(Component, Debug)]
struct PromptCaption;

impl Crosshair {
    fn spawn(mut commands: Commands, font: Res<IconFont>) {
        commands
            .spawn((
                Crosshair,
                Node {
                    width: percent(100),
                    height: percent(100),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                UiTransform::default(),
                Pickable::IGNORE,
                Visibility::Hidden,
            ))
            .with_children(|center| {
                center.spawn((
                    Shape::Dot,
                    Node {
                        width: px(cursor::DOT_SIZE),
                        height: px(cursor::DOT_SIZE),
                        border: UiRect::all(px(cursor::DOT_OUTLINE)),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(cursor::DOT_FILL),
                    BorderColor::all(cursor::DOT_INK),
                    Pickable::IGNORE,
                ));
                center
                    .spawn((
                        Shape::Prompt,
                        Node {
                            display: Display::None,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|prompt| {
                        ToolBadge::spawn(prompt, ClickTool::Screwdriver);
                        ToolBadge::spawn(prompt, ClickTool::Plug);
                        Icon::MouseLeft.spawn(prompt, &font);
                        // Hangs below the icon, so the icon stays in the
                        // middle.
                        prompt.spawn((
                            PromptCaption,
                            Caption,
                            Node {
                                position_type: PositionType::Absolute,
                                top: percent(100),
                                left: percent(50),
                                ..default()
                            },
                            UiTransform::from_translation(Val2 {
                                x: percent(-50),
                                y: Val::ZERO,
                            }),
                        ));
                    });
            });
    }

    /// Shows the dot in the middle while looking around. With a
    /// [`Prompt`] the dot turns into the mouse button, at the free cursor
    /// while focused.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn update(
        gaze: Gaze,
        prompt: Res<Prompt>,
        window: Single<&Window, With<PrimaryWindow>>,
        ui_scale: Res<UiScale>,
        crosshair: Single<(&mut Visibility, &mut UiTransform), With<Crosshair>>,
        mut shapes: Query<(&Shape, &mut Node)>,
        caption: Single<(Entity, Option<&Localized>), With<PromptCaption>>,
        mut commands: Commands,
    ) {
        let (mut visibility, mut transform) = crosshair.into_inner();
        visibility.set_if_neq(if gaze.steering() || prompt.at_cursor.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let offset = prompt.at_cursor.map_or(Vec2::ZERO, |cursor| {
            (cursor - window.size() / 2.0) / ui_scale.0
        });
        let translation = Val2::px(offset.x, offset.y);
        if transform.translation != translation {
            transform.translation = translation;
        }
        for (shape, mut node) in &mut shapes {
            let display = if (*shape == Shape::Prompt) == prompt.message.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
        if let Some(message) = &prompt.message {
            let (entity, current) = *caption;
            let text = Localized::from(message.clone());
            if current != Some(&text) {
                commands.entity(entity).insert(text);
            }
        }
    }
}

/// What a left click does where the player points, if hints are on.
///
/// While looking around that is the middle of the view. While focused with
/// the cursor free it is the cursor, which the [`Crosshair`] then replaces.
#[derive(Resource, Debug, Default, PartialEq)]
pub(crate) struct Prompt {
    message: Option<Message>,
    /// The tool of the hovered part the message is about, if any.
    tool: Option<ClickTool>,
    /// Where the free cursor is, in logical pixels, while the prompt shows
    /// there.
    at_cursor: Option<Vec2>,
}

impl Prompt {
    pub(crate) fn at_cursor(&self) -> bool {
        self.at_cursor.is_some()
    }

    /// In focus views the [`ClickHint`] comes first, then a [`Clickable`]
    /// part of the focused thing.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    pub(super) fn update(
        gaze: Gaze,
        settings: Res<Settings>,
        click: Res<ClickHint>,
        focused: Res<Focused>,
        hovers: Res<HoverMap>,
        parents: Query<&ChildOf>,
        clickables: Query<&Clickable>,
        tools: Query<&ClickTool>,
        window: Single<&Window, With<PrimaryWindow>>,
        keep_mouse: Option<Res<KeepMouse>>,
        pointers: Query<(&PointerId, &PointerLocation)>,
        mut prompt: ResMut<Prompt>,
    ) {
        let hovered = focus::hovered_clickable(&hovers, &parents, &clickables, &focused);
        let from_hover = (settings.hints() && (gaze.steering() || gaze.focused()))
            .then(|| hovered.and_then(|entity| clickables.get(entity).ok()))
            .flatten();
        let (message, tool) = match (settings.hints(), click.0.clone()) {
            (true, Some(click)) if gaze.focused() => (Some(click), None),
            _ => (
                from_hover.map(|clickable| Message::new(clickable.prompt)),
                hovered
                    .filter(|_| from_hover.is_some())
                    .and_then(|entity| tools.get(entity).ok().copied()),
            ),
        };
        // A tool driving the game moves only its pointer, not the cursor.
        let cursor = if keep_mouse.is_some() {
            pointers
                .iter()
                .find(|(id, _)| id.is_mouse())
                .and_then(|(_, location)| location.location())
                .map(|location| location.position)
        } else {
            window.cursor_position()
        };
        let at_cursor = if gaze.focused() && !gaze.free_looking() {
            cursor
        } else {
            None
        };
        prompt.set_if_neq(Prompt {
            at_cursor: at_cursor.filter(|_| message.is_some()),
            message,
            tool,
        });
    }
}
