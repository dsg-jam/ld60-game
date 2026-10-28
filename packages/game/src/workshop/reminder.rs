use bevy::prelude::*;
use ld60_core::parts::screen::UiCanvas;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use ld60_locale::{Localized, Message};

use super::run::Run;

/// In front of the till, where the player sees it, see
/// [`CounterLayout::place`].
///
/// [`CounterLayout::place`]: ld60_core::layout::counter::CounterLayout::place
pub(super) const SPOT: Vec2 = Vec2::new(-0.02, -0.5);
pub(super) const YAW: f32 = 0.2;
pub(super) const SIZE: Vec3 = Vec3::new(0.075, 0.002, 0.075);
const CANVAS: UVec2 = UVec2::new(300, 300);
const PAPER: Color = Color::srgb_u8(0xe8, 0xd4, 0x6a);
/// A blue ballpoint pen.
const INK: Color = Color::srgb_u8(0x2a, 0x3a, 0x6a);
const TITLE_SIZE: f32 = 42.0;
const AMOUNT_SIZE: f32 = 88.0;
const WHEN_SIZE: f32 = 21.0;

pub(crate) struct ReminderPlugin;

impl Plugin for ReminderPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Reminder::on_add)
            .add_systems(Update, Reminder::write);
    }
}

/// A sticky note in front of the till that says what the next rent is.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Reminder;

/// A line of the [`Reminder`].
#[derive(Component, Clone, Copy, Debug)]
enum Line {
    Title,
    Amount,
    When,
}

impl Reminder {
    fn on_add(add: On<Add<Reminder>>, mut commands: Commands, mut images: ResMut<Assets<Image>>) {
        let canvas = UiCanvas::spawn(&mut commands, &mut images, CANVAS, add.entity);
        let line = |line: Line, size: f32| {
            (
                line,
                Localized::verbatim(""),
                TextFont {
                    font_size: FontSize::Px(size),
                    ..default()
                },
                TextColor(INK),
            )
        };
        commands.spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(PAPER),
            canvas.target(),
            children![
                line(Line::Title, TITLE_SIZE),
                line(Line::Amount, AMOUNT_SIZE),
                line(Line::When, WHEN_SIZE),
            ],
        ));
        commands.entity(add.entity).with_child((
            Shape::textured_cuboid(
                &Block::new(SIZE),
                canvas.image,
                Rect::new(0.0, 0.0, 1.0, 1.0),
            ),
            Finish::Paper,
        ));
    }

    /// Follows the rent of the run. Once the player owns the building, it
    /// says so.
    fn write(run: Res<Run>, mut lines: Query<(&Line, &mut Localized)>) {
        for (&line, mut text) in &mut lines {
            let message = match (line, run.owned()) {
                (Line::Title, false) => Message::new("reminder-title"),
                (Line::Amount, false) => Message::new("reminder-amount").with("amount", run.rent()),
                (Line::When, false) => Message::new("reminder-when"),
                (Line::Title, true) => Message::new("reminder-owned"),
                (Line::Amount | Line::When, true) => {
                    text.set_if_neq(Localized::verbatim(""));
                    continue;
                }
            };
            text.set_if_neq(message.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_locale::Locale;

    use super::*;

    #[test]
    fn every_line_formats() {
        Locale::on_disk().assert_formats([
            Message::new("reminder-title"),
            Message::new("reminder-amount").with("amount", 30),
            Message::new("reminder-when"),
            Message::new("reminder-owned"),
        ]);
    }
}
