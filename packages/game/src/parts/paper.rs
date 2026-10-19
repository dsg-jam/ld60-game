mod leaf;
mod sheet;

use std::f32::consts::FRAC_PI_2;
use std::hash::Hash;

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::{events::PointerClick, pointer::PointerButton},
    platform::hash::fixed_hash_one,
    prelude::*,
};

pub(crate) use self::leaf::Binding;
use self::leaf::{Leaf, LeafLook};
use self::sheet::{PaperMaterial, SheetPlugin, SheetPrint, Wrinkles};
use super::screen::UiCanvas;
use crate::locale::{Localized, Message};
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
/// Trackpad scrolling this far, in pixels, turns a page.
const SCROLL_PIXELS: f32 = 40.0;
/// A pause this long, in seconds, ends a trackpad swipe.
const SCROLL_PAUSE: f32 = 0.15;
/// Camera height above a sheet while reading it.
const READING_HEIGHT: f32 = 0.3;
/// Matte paper reflects little light off its surface, so the print stays
/// dark under the lamp.
pub(crate) const PAPER_REFLECTANCE: f32 = 0.1;
/// Darkens a sheet under the lamp, so the page is not so bright that the
/// tone mapping washes out its print.
pub(crate) const PAPER_SHADE: Color = Color::linear_rgb(0.5, 0.5, 0.5);

pub(crate) struct PaperPlugin;

impl Plugin for PaperPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SheetPlugin)
            .add_systems(
                Update,
                (
                    Paper::flip_on_keys,
                    Paper::flip_on_scroll,
                    Paper::show_current,
                    Binding::turn_pages,
                    Leaf::animate,
                )
                    .chain(),
            )
            .init_resource::<LeafLook>()
            .add_observer(Paper::flip_on_click);
    }
}

/// A document shown one page at a time on a sheet lying face up.
///
/// While the paper is focused, clicking the `>` in the bottom right corner
/// or pressing the right arrow turns to the next page. The `<` in the bottom
/// left corner and the left arrow go back. Scrolling down turns to the next
/// page and scrolling up goes back.
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
        footer: impl Into<Localized>,
        content: impl FnOnce(&mut ChildSpawnerCommands),
    ) -> Entity {
        let footer = Self::footer(footer.into(), (number, count));
        Self::spawn_layout(commands, canvas, content, |bottom| {
            bottom.spawn(Self::corner_mark("<", number > 1));
            bottom.spawn(Self::footer_line(footer));
            bottom.spawn(Self::corner_mark(">", number < count));
        })
    }

    /// Spawns a loose page on `canvas`, with only `footer` at the bottom.
    pub(crate) fn spawn_single(
        commands: &mut Commands,
        canvas: &UiCanvas,
        footer: impl Into<Localized>,
        content: impl FnOnce(&mut ChildSpawnerCommands),
    ) -> Entity {
        let footer = footer.into();
        Self::spawn_layout(commands, canvas, content, |bottom| {
            bottom.spawn(Self::footer_line(footer));
        })
    }

    fn spawn_layout(
        commands: &mut Commands,
        canvas: &UiCanvas,
        content: impl FnOnce(&mut ChildSpawnerCommands),
        bottom: impl FnOnce(&mut ChildSpawnerCommands),
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
                .with_children(bottom);
            })
            .id()
    }

    fn footer_line(footer: impl Into<Localized>) -> impl Bundle {
        (
            Node {
                flex_grow: 1.0,
                justify_content: JustifyContent::Center,
                ..default()
            },
            children![Self::faded(footer)],
        )
    }

    fn footer(footer: Localized, (number, count): (usize, usize)) -> Message {
        Message::new("paper-footer")
            .with("footer", footer)
            .with("page", number)
            .with("pages", count)
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
            children![Self::print(Localized::verbatim(arrow), HEADING_SIZE, INK)],
        )
    }

    /// A sheet of `size` showing `canvas`, centered on `at`. It faces up
    /// with the top of the page away from the player.
    ///
    /// `document` picks the wrinkles and stains, so the same document looks
    /// the same every time.
    pub(crate) fn sheet(
        canvas: &UiCanvas,
        document: impl Hash,
        size: Vec2,
        at: Vec3,
        meshes: &mut Assets<Mesh>,
    ) -> impl Bundle {
        let wrinkles = Wrinkles::new(size, fixed_hash_one(document));
        (
            Mesh3d(meshes.add(wrinkles.mesh())),
            wrinkles,
            SheetPrint(canvas.image.clone()),
            Transform::from_translation(at).with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        )
    }

    pub(crate) fn title(text: impl Into<Localized>) -> impl Bundle {
        Self::print(text, TITLE_SIZE, INK)
    }

    pub(crate) fn heading(text: impl Into<Localized>) -> impl Bundle {
        Self::print(text, HEADING_SIZE, INK)
    }

    pub(crate) fn text(text: impl Into<Localized>) -> impl Bundle {
        Self::print(text, TEXT_SIZE, INK)
    }

    pub(crate) fn faded(text: impl Into<Localized>) -> impl Bundle {
        Self::print(text, TEXT_SIZE, INK_FADED)
    }

    /// A table row with each cell `width` pixels wide.
    pub(crate) fn row<const N: usize>(cells: [(Localized, f32); N]) -> impl Bundle {
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

    fn print(text: impl Into<Localized>, size: f32, color: Color) -> impl Bundle {
        (
            text.into(),
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

    pub(crate) fn current_page(&self) -> usize {
        self.current
    }

    pub(crate) fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Opens page `index`, counted from 0, or the last page if there are
    /// fewer.
    pub(crate) fn open_page(&mut self, index: usize) {
        self.current = index.min(self.pages.len().saturating_sub(1));
    }

    /// Where `hit` lies on the sheet, from -0.5 to 0.5 on both axes with the
    /// top of the page at +0.5.
    pub(crate) fn sheet_point(&self, hit: Vec3, sheets: &Query<&GlobalTransform>) -> Option<Vec2> {
        let sheet = sheets.get(self.sheet).ok()?;
        // The sheet lies in its local XY plane with the top of the page at +Y.
        Some(sheet.affine().inverse().transform_point3(hit).truncate() / self.size)
    }

    /// Opens the first of `pages` and returns the old pages.
    pub(crate) fn replace_pages(&mut self, pages: Vec<Entity>) -> Vec<Entity> {
        self.current = 0;
        std::mem::replace(&mut self.pages, pages)
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
        let Some(local) = click
            .hit
            .position
            .and_then(|hit| paper.sheet_point(hit, &sheets))
        else {
            return;
        };
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

    /// Each notch of a wheel turns one page. A trackpad swipe turns one page
    /// once it has gone [`SCROLL_PIXELS`], however long it goes on.
    fn flip_on_scroll(
        mut wheel: MessageReader<MouseWheel>,
        time: Res<Time>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        mut papers: Query<&mut Paper>,
        mut swipe: Local<Swipe>,
    ) {
        let now = time.elapsed_secs();
        let mut turns = 0;
        for event in wheel.read() {
            match event.unit {
                MouseScrollUnit::Line => turns -= event.y.signum() as i32,
                MouseScrollUnit::Pixel => {
                    if now - swipe.last > SCROLL_PAUSE {
                        *swipe = Swipe::default();
                    }
                    swipe.last = now;
                    swipe.travel += event.y;
                    if !swipe.spent && swipe.travel.abs() >= SCROLL_PIXELS {
                        swipe.spent = true;
                        turns -= swipe.travel.signum() as i32;
                    }
                }
            }
        }
        if turns == 0 || *state.get() != Focus::Paper {
            return;
        }
        let Some(mut paper) = focused.get().and_then(|entity| papers.get_mut(entity).ok()) else {
            return;
        };
        for _ in 0..turns.unsigned_abs() {
            paper.turn(turns > 0);
        }
    }

    pub(crate) fn show_current(
        papers: Query<&Paper, Changed<Paper>>,
        mut nodes: Query<&mut Node>,
        sheets: Query<&MeshMaterial3d<PaperMaterial>>,
        mut materials: ResMut<Assets<PaperMaterial>>,
    ) {
        for paper in &papers {
            if let Some(mut material) = sheets
                .get(paper.sheet)
                .ok()
                .and_then(|sheet| materials.get_mut(sheet))
            {
                material.extension.set_page(paper.current);
            }
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

/// The trackpad scrolling since the last pause.
#[derive(Debug, Default)]
struct Swipe {
    travel: f32,
    /// Whether the swipe has turned its page.
    spent: bool,
    /// When the last scroll came in.
    last: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::Locale;

    #[test]
    fn footers_format() {
        let footer = Paper::footer(Localized::verbatim("DSG"), (1, 2));
        assert_eq!(
            Locale::on_disk().check(&footer.into()),
            Ok("DSG  PAGE 1 OF 2".into())
        );
    }
}
