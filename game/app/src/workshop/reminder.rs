use bevy::prelude::*;
use ld60_game_computer::ComputerLayout;
use ld60_game_parts::paper::{Flutter, PAPER_REFLECTANCE, PAPER_ROUGHNESS, Wrinkles};
use ld60_game_parts::screen::UiCanvas;
use ld60_game_run::Run;
use ld60_locale::{Locale, Localized, Message};

const SIZE: Vec2 = Vec2::splat(0.075);
/// How far the middle of the note sits in from the left end of the bezel.
const IN_FROM_LEFT: f32 = 0.06;
/// How far the top of the note sits below the top of the bezel bar.
const BELOW_TOP: f32 = 0.006;
/// Turns the note a little, as stuck on by hand.
const ROLL: f32 = 0.06;
/// Keeps the note off the monitor under it.
const LIFT: f32 = 0.0003;
const CANVAS: UVec2 = UVec2::new(300, 300);
const PAPER: Color = Color::srgb_u8(0xe8, 0xd4, 0x6a);
/// A blue ballpoint pen.
const INK: Color = Color::srgb_u8(0x2a, 0x3a, 0x6a);
/// The size of each line of the note, and the paper left bare around them.
const SIZES: [f32; 3] = [30.0, 72.0, 20.0];
const PADDING: f32 = 24.0;

pub(crate) struct ReminderPlugin;

impl Plugin for ReminderPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Reminder::on_add)
            .add_systems(Update, Reminder::write);
    }
}

/// A sticky note on the monitor that says what the next rent is.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Reminder;

/// A line of the [`Reminder`], counted from the top.
#[derive(Component, Clone, Copy, Debug)]
struct Line(usize);

impl Reminder {
    /// Stuck to the bezel below the glass, at the left, hanging over the
    /// bottom edge. `computer` is where the computer stands.
    pub(super) fn place(computer: Transform) -> Transform {
        let bezel = ComputerLayout::new().bezel_bottom();
        let top = Vec3::new(
            bezel.min().x + IN_FROM_LEFT,
            bezel.max().y - BELOW_TOP,
            bezel.max().z,
        );
        let roll = Quat::from_rotation_z(ROLL);
        computer
            * Transform::from_translation(top + roll * Vec3::NEG_Y * SIZE.y / 2.0)
                .with_rotation(roll)
    }

    fn on_add(
        add: On<Add<Reminder>>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let canvas = UiCanvas::spawn(&mut commands, &mut images, CANVAS, add.entity);
        commands.spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: px(PADDING).all(),
                ..default()
            },
            BackgroundColor(PAPER),
            canvas.target(),
            Children::spawn(SpawnIter(SIZES.into_iter().enumerate().map(
                |(index, size)| {
                    (
                        Line(index),
                        Node {
                            width: percent(100),
                            ..default()
                        },
                        Localized::verbatim(""),
                        TextFont {
                            font_size: FontSize::Px(size),
                            ..default()
                        },
                        TextLayout::justify(Justify::Center),
                        TextColor(INK),
                    )
                },
            ))),
        ));
        let wrinkles = Wrinkles::sticky(SIZE, "reminder");
        commands.entity(add.entity).with_child((
            Mesh3d(meshes.add(wrinkles.mesh())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(canvas.image),
                perceptual_roughness: PAPER_ROUGHNESS,
                reflectance: PAPER_REFLECTANCE,
                double_sided: true,
                cull_mode: None,
                ..default()
            })),
            wrinkles,
            Flutter::default(),
            Transform::from_xyz(0.0, 0.0, LIFT),
        ));
    }

    /// Follows the rent of the run. Once the player owns the building, it
    /// says so. The note is one message, so it reads as one sentence in any
    /// language, and each of its lines is written in its own size.
    fn write(run: Res<Run>, locale: Res<Locale>, mut lines: Query<(&Line, &mut Localized)>) {
        let message = if run.owned() {
            Message::new("reminder-owned")
        } else {
            Message::new("reminder").with("amount", run.rent())
        };
        let text = locale.text(&message.into());
        let mut written: Vec<&str> = text.lines().collect();
        // A short note sits on the big middle line.
        if written.len() == 1 {
            written.insert(0, "");
        }
        for (&Line(index), mut line) in &mut lines {
            let wanted = Localized::verbatim(written.get(index).copied().unwrap_or_default());
            line.set_if_neq(wanted);
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
            Message::new("reminder").with("amount", 30),
            Message::new("reminder-owned"),
        ]);
    }
}
