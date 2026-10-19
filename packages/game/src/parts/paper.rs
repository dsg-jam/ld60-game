mod leaf;
mod sheet;

use std::f32::consts::FRAC_PI_2;
use std::hash::Hash;
use std::iter;

use bevy::{
    ecs::system::SystemParam,
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::{
        events::PointerClick,
        hover::HoverMap,
        pointer::{PointerButton, PointerId},
    },
    platform::hash::fixed_hash_one,
    prelude::*,
};

pub(crate) use self::leaf::{
    Binding, Leaf, LeafShape, LeafSpec, Stiffness, blank_print, reprinted,
};
pub(crate) use self::sheet::{PaperMaterial, Wrinkles};
use self::sheet::{SheetGlow, SheetPlugin, SheetPrint};
use super::screen::UiCanvas;
use crate::locale::{Localized, Message};
use crate::player::focus::{self, Focus, Focused};
use crate::player::hints::ClickHint;

pub(crate) const PAPER: Color = Color::srgb_u8(0xf2, 0xec, 0xdc);
pub(crate) const INK: Color = Color::srgb_u8(0x2a, 0x26, 0x22);
pub(crate) const INK_FADED: Color = Color::srgb_u8(0x6a, 0x62, 0x58);
const TITLE_SIZE: f32 = 34.0;
const HEADING_SIZE: f32 = 21.0;
const TEXT_SIZE: f32 = 17.0;
const TABLE_SIZE: f32 = 15.0;
/// Clicks within this fraction of the sheet from its bottom right corner
/// turn to the next page.
const CORNER_ZONE: f32 = 0.3;
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
                    Paper::show_turn_hint,
                    Leaf::animate,
                )
                    .chain(),
            )
            .add_observer(Paper::flip_on_click);
    }
}

/// A document shown one page at a time on a sheet lying face up.
///
/// While the paper is focused, clicking the bottom right corner of the
/// sheet or pressing the right arrow turns to the next page. Clicking the
/// turned pages past its [`Binding`], or the bottom left corner of a loose
/// sheet, and the left arrow go back. Scrolling down turns to the next page
/// and scrolling up goes back.
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
    /// page number at the bottom.
    pub(crate) fn spawn_page(
        commands: &mut Commands,
        canvas: &UiCanvas,
        (number, count): (usize, usize),
        footer: impl Into<Localized>,
        content: impl FnOnce(&mut ChildSpawnerCommands),
    ) -> Entity {
        let footer = Self::footer(footer.into(), (number, count));
        Self::spawn_single(commands, canvas, footer, content)
    }

    /// Spawns a loose page on `canvas`, with only `footer` at the bottom.
    pub(crate) fn spawn_single(
        commands: &mut Commands,
        canvas: &UiCanvas,
        footer: impl Into<Localized>,
        content: impl FnOnce(&mut ChildSpawnerCommands),
    ) -> Entity {
        let footer = footer.into();
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
                page.spawn((
                    Node {
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    children![Self::faded(footer)],
                ));
            })
            .id()
    }

    fn footer(footer: Localized, (number, count): (usize, usize)) -> Message {
        Message::new("paper-footer")
            .with("footer", footer)
            .with("page", number)
            .with("pages", count)
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

    /// Like [`Paper::sheet`], but upright and hanging from a clip on its top
    /// edge. It gives off `glow` where it is bare, so it stays readable in
    /// the shade.
    pub(crate) fn hanging_sheet(
        canvas: &UiCanvas,
        document: impl Hash,
        size: Vec2,
        at: Vec3,
        glow: LinearRgba,
        meshes: &mut Assets<Mesh>,
    ) -> impl Bundle {
        let mut wrinkles = Wrinkles::new(size, fixed_hash_one(document));
        wrinkles.hold(&Binding::clip());
        (
            Mesh3d(meshes.add(wrinkles.mesh())),
            wrinkles,
            SheetPrint(canvas.image.clone()),
            SheetGlow(glow),
            Transform::from_translation(at),
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

    /// The UI root of page `index`, counted from 0.
    pub(crate) fn page(&self, index: usize) -> Option<Entity> {
        self.pages.get(index).copied()
    }

    /// Where `hit` lies on the sheet, from -0.5 to 0.5 on both axes with the
    /// top of the page at +0.5.
    fn sheet_point(&self, hit: Vec3, sheets: &Query<&GlobalTransform>) -> Option<Vec2> {
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

    /// Whether a click at `hit` turns forward or back, if it turns at all.
    /// The bottom right corner of the sheet turns forward. The turned pages
    /// past the binding turn back, or the bottom left corner without one.
    fn turn_at(
        &self,
        hit: Vec3,
        binding: Option<&Binding>,
        sheets: &Query<&GlobalTransform>,
    ) -> Option<bool> {
        let point = self.sheet_point(hit, sheets)?;
        let low = point.y < CORNER_ZONE - 0.5;
        let back = match binding {
            Some(binding) => binding.beyond(point),
            None => low && point.x < CORNER_ZONE - 0.5,
        };
        let forward = !back && low && point.x > 0.5 - CORNER_ZONE;
        if back && self.current > 0 {
            Some(false)
        } else if forward && self.current + 1 < self.pages.len() {
            Some(true)
        } else {
            None
        }
    }

    fn flip_on_click(click: On<PointerClick>, mut mouse: MouseOnPaper) {
        if click.button != PointerButton::Primary || mouse.focused.get() != Some(click.entity) {
            return;
        }
        if let Some(forward) = mouse.turn()
            && let Ok((mut paper, _)) = mouse.papers.get_mut(click.entity)
        {
            paper.turn(forward);
        }
    }

    /// Says what a click does on the focused paper, where the mouse is.
    fn show_turn_hint(mouse: MouseOnPaper, mut hint: ResMut<ClickHint>) {
        hint.set_if_neq(ClickHint(mouse.hint()));
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
        papers: Query<&Paper, (Changed<Paper>, Without<ShowsOwnPages>)>,
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

/// Marks a [`Paper`] whose owner shows its pages, like a binder that shows
/// two pages while a leaf turns.
#[derive(Component, Debug, Default)]
pub(crate) struct ShowsOwnPages;

/// Something on or next to a paper with a click of its own, like a link
/// printed on a page or a divider tab. A click on it doesn't turn the page.
/// Hovering it shows its hint, if it has one.
#[derive(Component, Debug, Default)]
pub(crate) struct PaperButton(pub(crate) Option<Message>);

/// The mouse over the focused [`Paper`].
#[derive(SystemParam)]
struct MouseOnPaper<'w, 's> {
    state: Res<'w, State<Focus>>,
    focused: Res<'w, Focused>,
    hovers: Res<'w, HoverMap>,
    parents: Query<'w, 's, &'static ChildOf>,
    buttons: Query<'w, 's, &'static PaperButton>,
    papers: Query<'w, 's, (&'static mut Paper, Option<&'static Binding>)>,
    pointers: Query<'w, 's, &'static PointerId>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
}

impl MouseOnPaper<'_, '_> {
    /// Whether a click turns forward or back, if it turns at all. Nothing
    /// turns on a [`PaperButton`].
    fn turn(&self) -> Option<bool> {
        if *self.state.get() != Focus::Paper {
            return None;
        }
        let entity = self.focused.get()?;
        let (paper, binding) = self.papers.get(entity).ok()?;
        let (&hit, data) = self
            .hovers
            .get(&PointerId::Mouse)?
            .iter()
            .min_by(|a, b| a.1.depth.total_cmp(&b.1.depth))?;
        let path: Vec<Entity> = iter::once(hit)
            .chain(self.parents.iter_ancestors(hit))
            .collect();
        if !path.contains(&entity)
            || path.iter().any(|&entity| self.buttons.contains(entity))
            || self.on_printed_button(paper.sheet)
        {
            return None;
        }
        paper.turn_at(data.position?, binding, &self.transforms)
    }

    /// Whether the pointer that `sheet` drives in its canvas is on a button.
    fn on_printed_button(&self, sheet: Entity) -> bool {
        self.pointers
            .get(sheet)
            .ok()
            .and_then(|pointer| self.hovers.get(pointer))
            .is_some_and(|hits| {
                hits.keys().any(|&hit| {
                    iter::once(hit)
                        .chain(self.parents.iter_ancestors(hit))
                        .any(|entity| self.buttons.contains(entity))
                })
            })
    }

    /// What a click does: the hint of a [`PaperButton`], or turning a page.
    fn hint(&self) -> Option<Message> {
        if *self.state.get() != Focus::Paper {
            return None;
        }
        let button = focus::under_mouse(&self.hovers, &self.parents)
            .find_map(|entity| self.buttons.get(entity).ok());
        if let Some(button) = button {
            return button.0.clone();
        }
        let forward = self.turn()?;
        Some(Message::new(if forward {
            "hint-next-page"
        } else {
            "hint-previous-page"
        }))
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
