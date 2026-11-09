use bevy::prelude::*;
use ld60_game_parts::paper::{INK, PAPER};
use ld60_game_parts::screen::UiCanvas;
use ld60_locale::{Localized, Message};
use ld60_tick::{Op, WORD_BITS};

const SIZE: Vec2 = Vec2::new(0.105, 0.148);
const CANVAS: UVec2 = UVec2::new(420, 592);
const PADDING: f32 = 22.0;
const TITLE_SIZE: f32 = 24.0;
const TEXT_SIZE: f32 = 14.0;
const WORD_WIDTH: f32 = 72.0;
const NAME_WIDTH: f32 = 52.0;
/// A slide switch in the pictures of words: its slot, its slider, and the
/// gap between switches.
const SLOT: Vec2 = Vec2::new(12.0, 24.0);
const KNOB: f32 = 8.0;
const SWITCH_GAP: f32 = 4.0;

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
                        line.spawn(cell(WORD_WIDTH))
                            .with_children(|cell| Self::picture(cell, op.word()));
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
                page.spawn(text("tick-sheet-switches", TEXT_SIZE));
                page.spawn(text("tick-sheet-other", TEXT_SIZE));
                page.spawn(text("tick-sheet-order", TEXT_SIZE));
            });
        super::sheet::lay(
            &mut commands,
            add.entity,
            (canvas.image, SIZE, "tick-datasheet"),
            &mut meshes,
            &mut materials,
        );
    }

    /// Draws `word` as a row of switches, the highest bit on the left, up
    /// for 1.
    fn picture(parent: &mut ChildSpawnerCommands, word: u8) {
        parent
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(SWITCH_GAP),
                ..default()
            })
            .with_children(|switches| {
                for bit in (0..WORD_BITS).rev() {
                    let up = word >> bit & 1 != 0;
                    switches
                        .spawn((
                            Node {
                                width: px(SLOT.x),
                                height: px(SLOT.y),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: if up {
                                    JustifyContent::FlexStart
                                } else {
                                    JustifyContent::FlexEnd
                                },
                                padding: px(1).all(),
                                border: px(1.5).all(),
                                border_radius: BorderRadius::all(px(3)),
                                ..default()
                            },
                            BorderColor::all(INK),
                        ))
                        .with_child((
                            Node {
                                width: px(KNOB),
                                height: px(KNOB),
                                border_radius: BorderRadius::all(px(1)),
                                ..default()
                            },
                            BackgroundColor(INK),
                        ));
                }
            });
    }
}
