use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use ld60_game_parts::paper::{INK, PAPER, PAPER_REFLECTANCE, PAPER_ROUGHNESS, Wrinkles};
use ld60_game_parts::screen::UiCanvas;
use ld60_game_player::focus::{Focus, FocusTarget, View};
use ld60_locale::{Localized, Message};
use ld60_tick::Op;

const SIZE: Vec2 = Vec2::new(0.105, 0.148);
const CANVAS: UVec2 = UVec2::new(420, 592);
const PADDING: f32 = 22.0;
const TITLE_SIZE: f32 = 24.0;
const TEXT_SIZE: f32 = 14.0;
const WORD_WIDTH: f32 = 60.0;
const NAME_WIDTH: f32 = 60.0;
/// Camera height above the card while reading it.
const READING_HEIGHT: f32 = 0.2;
/// Keeps the card off the mat under it.
const LIFT: f32 = 0.0004;

/// The printed datasheet of the Tick chip, lying face up.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Datasheet;

impl Datasheet {
    pub(super) fn on_add(
        add: On<Add<Datasheet>>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let canvas = UiCanvas::spawn_print(&mut commands, &mut images, CANVAS, add.entity);
        let text = |id: &'static str, size: f32| {
            (
                Localized::from(Message::new(id)),
                TextFont {
                    font_size: FontSize::Px(size),
                    ..default()
                },
                TextColor(INK),
            )
        };
        let cell = |width: f32| Node {
            width: px(width),
            flex_shrink: 0.0,
            ..default()
        };
        let row = || Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(8.0),
            ..default()
        };
        let ops = [
            (Op::Nop, "tick-op-nop"),
            (Op::Step, "tick-op-step"),
            (Op::Wait, "tick-op-wait"),
        ];
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12.0),
                    padding: px(PADDING).all(),
                    ..default()
                },
                BackgroundColor(PAPER),
                canvas.target(),
            ))
            .with_children(|page| {
                page.spawn(text("tick-sheet-title", TITLE_SIZE));
                page.spawn(text("tick-sheet-revision", TEXT_SIZE));
                page.spawn(row()).with_children(|head| {
                    head.spawn((cell(WORD_WIDTH), text("tick-sheet-word", TEXT_SIZE)));
                    head.spawn((cell(NAME_WIDTH), text("tick-sheet-name", TEXT_SIZE)));
                    head.spawn(text("tick-sheet-effect", TEXT_SIZE));
                });
                for (op, id) in ops {
                    page.spawn(row()).with_children(|line| {
                        line.spawn((
                            cell(WORD_WIDTH),
                            Text::new(format!("{:04b}", op.word())),
                            TextFont {
                                font_size: FontSize::Px(TEXT_SIZE),
                                ..default()
                            },
                            TextColor(INK),
                        ));
                        line.spawn((
                            cell(NAME_WIDTH),
                            Text::new(format!("{op:?}").to_uppercase()),
                            TextFont {
                                font_size: FontSize::Px(TEXT_SIZE),
                                ..default()
                            },
                            TextColor(INK),
                        ));
                        line.spawn((Node::default(), text(id, TEXT_SIZE)));
                    });
                }
                page.spawn(text("tick-sheet-other", TEXT_SIZE));
                page.spawn(text("tick-sheet-order", TEXT_SIZE));
                page.spawn(text("tick-sheet-erase", TEXT_SIZE));
            });
        let wrinkles = Wrinkles::sticky(SIZE, "tick-datasheet");
        commands
            .entity(add.entity)
            .insert(FocusTarget {
                prompt: "click-datasheet",
                focus: Focus::Paper,
                view: View::Local(
                    Transform::from_xyz(0.0, READING_HEIGHT, 0.001)
                        .looking_at(Vec3::ZERO, -Vec3::Z),
                ),
            })
            .with_child((
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
                Transform::from_xyz(0.0, LIFT, 0.0)
                    .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
            ));
    }
}
