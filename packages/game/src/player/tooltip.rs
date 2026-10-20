use bevy::{picking::hover::HoverMap, prelude::*, window::PrimaryWindow};

use super::{
    focus::{self, Clickable, Focused},
    hints::{Caption, ClickHint, Icon, IconFont},
    look::Gaze,
};
use crate::locale::{Localized, Message};
use crate::menu::Settings;

/// From the cursor to the tooltip, in logical pixels.
const OFFSET: Vec2 = Vec2::new(14.0, 18.0);

pub(super) struct TooltipPlugin;

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Tooltip::spawn)
            .add_systems(Update, Tooltip::update);
    }
}

/// Says what a click does next to the free cursor while focused, like the
/// crosshair does in the desk view: the [`ClickHint`], or else the prompt of
/// a [`Clickable`] under the cursor other than the focused thing.
#[derive(Component, Debug)]
struct Tooltip;

/// The [`Caption`] of the [`Tooltip`].
#[derive(Component, Debug)]
struct TooltipCaption;

impl Tooltip {
    fn spawn(mut commands: Commands, font: Res<IconFont>) {
        commands
            .spawn((
                Tooltip,
                Node {
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                GlobalZIndex(1),
                Pickable::IGNORE,
                Visibility::Hidden,
            ))
            .with_children(|tooltip| {
                Icon::MouseLeft.spawn(tooltip, &font);
                tooltip.spawn((TooltipCaption, Caption, Localized::verbatim("")));
            });
    }

    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn update(
        gaze: Gaze,
        settings: Res<Settings>,
        click: Res<ClickHint>,
        focused: Res<Focused>,
        hovers: Res<HoverMap>,
        parents: Query<&ChildOf>,
        clickables: Query<&Clickable>,
        window: Single<&Window, With<PrimaryWindow>>,
        ui_scale: Res<UiScale>,
        tooltip: Single<(&mut Node, &mut Visibility), With<Tooltip>>,
        mut caption: Single<&mut Localized, With<TooltipCaption>>,
    ) {
        let (mut node, mut visibility) = tooltip.into_inner();
        let message = (gaze.focused() && !gaze.free_looking() && settings.hints())
            .then(|| {
                click.0.clone().or_else(|| {
                    focus::under_mouse(&hovers, &parents)
                        .take_while(|&entity| Some(entity) != focused.get())
                        .find_map(|entity| clickables.get(entity).ok())
                        .map(|clickable| Message::new(clickable.prompt))
                })
            })
            .flatten();
        let shown = message.zip(window.cursor_position());
        visibility.set_if_neq(if shown.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let Some((message, cursor)) = shown else {
            return;
        };
        let at = cursor / ui_scale.0 + OFFSET;
        if node.left != px(at.x) || node.top != px(at.y) {
            node.left = px(at.x);
            node.top = px(at.y);
        }
        let text = Localized::from(message);
        if **caption != text {
            **caption = text;
        }
    }
}
