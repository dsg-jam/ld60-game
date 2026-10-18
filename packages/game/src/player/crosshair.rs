use bevy::{picking::hover::HoverMap, prelude::*};

use super::{
    cursor,
    focus::{self, Clickable},
    hints::{Caption, Icon, IconFont},
    look::Gaze,
};
use crate::locale::Localized;

pub(super) struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Crosshair::spawn)
            .add_systems(Update, Crosshair::update);
    }
}

/// Marks what a click hits, in the middle of the view while looking around.
///
/// On a [`Clickable`] the dot turns into the left mouse button, with what a
/// click does below it.
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

    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn update(
        gaze: Gaze,
        hovers: Res<HoverMap>,
        parents: Query<&ChildOf>,
        clickables: Query<&Clickable>,
        mut crosshair: Single<&mut Visibility, With<Crosshair>>,
        mut shapes: Query<(&Shape, &mut Node)>,
        caption: Single<(Entity, Option<&Localized>), With<PromptCaption>>,
        mut commands: Commands,
    ) {
        crosshair.set_if_neq(if gaze.looking_around() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let hovered =
            focus::under_mouse(&hovers, &parents).find_map(|entity| clickables.get(entity).ok());
        for (shape, mut node) in &mut shapes {
            let display = if (*shape == Shape::Prompt) == hovered.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
        if let Some(clickable) = hovered {
            let (entity, current) = *caption;
            if current != Some(&Localized(clickable.prompt)) {
                commands.entity(entity).insert(Localized(clickable.prompt));
            }
        }
    }
}
