use std::f32::consts::FRAC_PI_2;

use bevy::{
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};

use super::screen::UiCanvas;
use crate::player::focus::{Focus, Focused};

pub(crate) const PAPER: Color = Color::srgb_u8(0xf2, 0xec, 0xdc);
pub(crate) const INK: Color = Color::srgb_u8(0x2a, 0x26, 0x22);
pub(crate) const INK_FADED: Color = Color::srgb_u8(0x6a, 0x62, 0x58);
const TITLE_SIZE: f32 = 34.0;
const HEADING_SIZE: f32 = 21.0;
const TEXT_SIZE: f32 = 17.0;
const TABLE_SIZE: f32 = 15.0;
/// Side of the `<` and `>` marks, in canvas pixels.
const CORNER_MARK: f32 = 34.0;
/// Clicks within this fraction of the sheet from a bottom corner turn the
/// page. It is larger than the mark, so aiming is easy.
const CORNER_ZONE: f32 = 0.2;
/// Keeps text clear of the edges of the sheet.
const MARGIN: f32 = 48.0;
/// Camera height above a sheet while reading it.
const READING_HEIGHT: f32 = 0.3;

pub(crate) struct PaperPlugin;

impl Plugin for PaperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Paper::flip_on_keys, Paper::show_current).chain())
            .add_observer(Paper::flip_on_click);
    }
}

/// A document shown one page at a time on a sheet lying face up.
///
/// While the paper is focused, clicking the `>` in the bottom right corner
/// or pressing the right arrow turns to the next page. The `<` in the bottom
/// left corner and the left arrow go back.
#[derive(Component, Debug)]
pub(crate) struct Paper {
    sheet: Entity,
    size: Vec2,
    pages: Vec<Entity>,
    current: usize,
}

impl Paper {
    /// `sheet` is the mesh from [`Paper::sheet`] and `size` its size.
    pub(crate) fn new(sheet: Entity, size: Vec2, pages: Vec<Entity>) -> Self {
        Self {
            sheet,
            size,
            pages,
            current: 0,
        }
    }

    /// Spawns page `number` of `count` on `canvas`, with `footer` and the
    /// corner marks for turning at the bottom.
    pub(crate) fn spawn_page(
        commands: &mut Commands,
        canvas: &UiCanvas,
        (number, count): (usize, usize),
        footer: &str,
        content: impl FnOnce(&mut ChildSpawnerCommands),
    ) -> Entity {
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    padding: px(MARGIN).all(),
                    row_gap: px(10),
                    ..default()
                },
                BackgroundColor(PAPER),
                canvas.target(),
            ))
            .with_children(|page| {
                content(page);
                page.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                page.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: px(16),
                    ..default()
                })
                .with_children(|bottom| {
                    bottom.spawn(Self::corner_mark("<", number > 1));
                    bottom.spawn((
                        Node {
                            flex_grow: 1.0,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        children![Self::faded(format!("{footer}   PAGE {number} OF {count}"))],
                    ));
                    bottom.spawn(Self::corner_mark(">", number < count));
                });
            })
            .id()
    }

    /// Hidden on the first and last page, but still taking up its space.
    fn corner_mark(arrow: &str, shown: bool) -> impl Bundle {
        (
            Node {
                width: px(CORNER_MARK),
                height: px(CORNER_MARK),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: px(1).all(),
                ..default()
            },
            BorderColor::all(INK_FADED),
            if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            children![Self::print(arrow, HEADING_SIZE, INK)],
        )
    }

    /// A sheet of `size` showing `canvas`, centered on `at`. It faces up
    /// with the top of the page away from the player.
    pub(crate) fn sheet(
        canvas: &UiCanvas,
        size: Vec2,
        at: Vec3,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> impl Bundle {
        (
            Mesh3d(meshes.add(Rectangle::from_size(size))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(canvas.image.clone()),
                perceptual_roughness: 0.95,
                ..default()
            })),
            Transform::from_translation(at).with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        )
    }

    pub(crate) fn title(text: impl Into<String>) -> impl Bundle {
        Self::print(text, TITLE_SIZE, INK)
    }

    pub(crate) fn heading(text: impl Into<String>) -> impl Bundle {
        Self::print(text, HEADING_SIZE, INK)
    }

    pub(crate) fn text(text: impl Into<String>) -> impl Bundle {
        Self::print(text, TEXT_SIZE, INK)
    }

    pub(crate) fn faded(text: impl Into<String>) -> impl Bundle {
        Self::print(text, TEXT_SIZE, INK_FADED)
    }

    /// A table row with each cell `width` pixels wide.
    pub(crate) fn row<const N: usize>(cells: [(String, f32); N]) -> impl Bundle {
        (
            Node::default(),
            Children::spawn(SpawnIter(cells.into_iter().map(|(text, width)| {
                (
                    Node {
                        width: px(width),
                        ..default()
                    },
                    children![Self::print(text, TABLE_SIZE, INK)],
                )
            }))),
        )
    }

    fn print(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
        (
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(color),
        )
    }

    /// Camera pose for reading a sheet centered on `at`.
    pub(crate) fn reading_view(at: Vec3) -> Transform {
        Transform::from_translation(at + Vec3::Y * READING_HEIGHT).looking_at(at, Vec3::NEG_Z)
    }

    pub(crate) fn open_first_page(&mut self) {
        self.current = 0;
    }

    fn turn(&mut self, forward: bool) {
        let last = self.pages.len().saturating_sub(1);
        self.current = if forward {
            (self.current + 1).min(last)
        } else {
            self.current.saturating_sub(1)
        };
    }

    fn flip_on_click(
        click: On<PointerClick>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        mut papers: Query<&mut Paper>,
        sheets: Query<&GlobalTransform>,
    ) {
        if click.button != PointerButton::Primary
            || *state.get() != Focus::Paper
            || focused.get() != Some(click.entity)
        {
            return;
        }
        let Ok(mut paper) = papers.get_mut(click.entity) else {
            return;
        };
        let (Some(hit), Ok(sheet)) = (click.hit.position, sheets.get(paper.sheet)) else {
            return;
        };
        // The sheet lies in its local XY plane with the top of the page at +Y.
        let local = sheet.affine().inverse().transform_point3(hit).truncate() / paper.size;
        if local.y > CORNER_ZONE - 0.5 {
            return;
        }
        if local.x > 0.5 - CORNER_ZONE {
            paper.turn(true);
        } else if local.x < CORNER_ZONE - 0.5 {
            paper.turn(false);
        }
    }

    fn flip_on_keys(
        keys: Res<ButtonInput<KeyCode>>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        mut papers: Query<&mut Paper>,
    ) {
        if *state.get() != Focus::Paper {
            return;
        }
        let Some(mut paper) = focused.get().and_then(|entity| papers.get_mut(entity).ok()) else {
            return;
        };
        if keys.just_pressed(KeyCode::ArrowRight) {
            paper.turn(true);
        } else if keys.just_pressed(KeyCode::ArrowLeft) {
            paper.turn(false);
        }
    }

    fn show_current(papers: Query<&Paper, Changed<Paper>>, mut nodes: Query<&mut Node>) {
        for paper in &papers {
            for (index, &page) in paper.pages.iter().enumerate() {
                if let Ok(mut node) = nodes.get_mut(page) {
                    node.display = if index == paper.current {
                        Display::Flex
                    } else {
                        Display::None
                    };
                }
            }
        }
    }
}
