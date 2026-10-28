use bevy::prelude::*;

use super::super::cursor::{DOT_FILL, DOT_INK};
use super::super::focus::ClickTool;
use super::Prompt;

const SIZE: f32 = 26.0;
const OUTLINE: f32 = 1.5;
const GRIP: Color = Color::srgb_u8(0xb0, 0x3a, 0x2c);
const METAL: Color = Color::srgb_u8(0xc8, 0xca, 0xce);

/// A small drawing of a [`ClickTool`] left of the mouse button of the
/// prompt.
#[derive(Component, Debug)]
pub(super) struct ToolBadge(pub(super) ClickTool);

impl ToolBadge {
    pub(super) fn spawn(parent: &mut ChildSpawnerCommands, tool: ClickTool) {
        parent
            .spawn((
                ToolBadge(tool),
                Node {
                    display: Display::None,
                    position_type: PositionType::Absolute,
                    right: percent(100),
                    top: percent(50),
                    width: px(SIZE),
                    height: px(SIZE),
                    margin: UiRect::top(px(-SIZE / 2.0)),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::degrees(-45.0)),
                Pickable::IGNORE,
            ))
            .with_children(|badge| match tool {
                ClickTool::Screwdriver => {
                    part(badge, (0.0, 9.0), (11.0, 8.0), GRIP, 3.0);
                    part(badge, (10.0, 11.5), (13.0, 3.0), METAL, 0.5);
                    part(badge, (22.0, 10.5), (4.0, 5.0), METAL, 0.5);
                }
                ClickTool::Plug => {
                    part(badge, (0.0, 11.5), (7.0, 3.0), DOT_INK, 1.0);
                    part(badge, (6.0, 7.0), (11.0, 12.0), DOT_FILL, 2.0);
                    part(badge, (16.0, 8.5), (8.0, 2.5), METAL, 0.5);
                    part(badge, (16.0, 15.0), (8.0, 2.5), METAL, 0.5);
                }
            });
    }

    pub(super) fn update(prompt: Res<Prompt>, mut badges: Query<(&ToolBadge, &mut Node)>) {
        for (badge, mut node) in &mut badges {
            let display = if prompt.tool == Some(badge.0) {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
}

/// A rounded box of the drawing at `(left, top)` with `(width, height)`, in
/// pixels.
fn part(
    badge: &mut ChildSpawnerCommands,
    (left, top): (f32, f32),
    (width, height): (f32, f32),
    color: Color,
    radius: f32,
) {
    badge.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(left),
            top: px(top),
            width: px(width),
            height: px(height),
            border: UiRect::all(px(OUTLINE)),
            border_radius: BorderRadius::all(px(radius)),
            ..default()
        },
        BackgroundColor(color),
        BorderColor::all(DOT_INK),
        Pickable::IGNORE,
    ));
}
