use bevy::prelude::*;

use super::console::Console;

const FONT_SIZE: f32 = 14.0;
const PADDING: f32 = 8.0;
const BACKGROUND: Color = Color::srgba(0.02, 0.02, 0.03, 0.95);
const TEXT: Color = Color::srgb(0.8, 0.82, 0.78);
const PROMPT: Color = Color::srgb(0.95, 0.75, 0.35);
/// Lines of output that are drawn. More would not fit.
const DRAWN_LINES: usize = 80;

/// The panel of the console over the top of the screen.
#[derive(Component)]
pub(crate) struct View;

#[derive(Component)]
pub(crate) struct Output;

#[derive(Component)]
pub(crate) struct Prompt;

impl View {
    pub(crate) fn spawn(mut commands: Commands) {
        let font = TextFont {
            font_size: FontSize::Px(FONT_SIZE),
            ..default()
        };
        commands.spawn((
            View,
            Name::new("console"),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(45),
                flex_direction: FlexDirection::Column,
                padding: px(PADDING).all(),
                display: Display::None,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(BACKGROUND),
            GlobalZIndex(i32::MAX),
            children![
                (
                    Node {
                        flex_grow: 1.0,
                        min_height: px(0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexEnd,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    children![(Output, Text::default(), font.clone(), TextColor(TEXT))],
                ),
                (Prompt, Text::default(), font, TextColor(PROMPT)),
            ],
        ));
    }

    pub(crate) fn show(
        console: Res<Console>,
        mut view: Single<&mut Node, With<View>>,
        mut output: Single<&mut Text, (With<Output>, Without<Prompt>)>,
        mut prompt: Single<&mut Text, With<Prompt>>,
    ) {
        if !console.is_changed() {
            return;
        }
        view.display = if console.open {
            Display::Flex
        } else {
            Display::None
        };
        let lines: Vec<&str> = console.output().collect();
        output.0 = lines[lines.len().saturating_sub(DRAWN_LINES)..].join("\n");
        let (before, after) = console.line.split_at(console.caret);
        prompt.0 = format!("> {before}_{after}");
    }
}
