mod datasheet;
mod tab;

use bevy::{
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};
use ld60_devices::ChipSpec;
use ld60_levels::Level;

use self::tab::{Contents, Tab};
use super::job::{Job, JobId};
use super::run::Run;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;
use crate::parts::paper::{INK, PAPER, Paper};
use crate::parts::screen::UiCanvas;
use crate::player::focus::{Focus, FocusOn, FocusTarget, Focused, View};

const SHEET_SIZE: Vec2 = Vec2::new(0.16, 0.22);
const CANVAS_SIZE: UVec2 = UVec2::new(640, 880);
const COVER_SIZE: Vec3 = Vec3::new(0.185, 0.005, 0.24);
/// Pages under the open one.
const STACK_SIZE: Vec3 = Vec3::new(0.16, 0.012, 0.22);
const RING_SIZE: Vec3 = Vec3::new(0.012, 0.01, 0.012);
/// From the left edge of the pages.
const RING_INSET: f32 = 0.01;
/// From the middle of the left edge, along it.
const RING_SPREAD: f32 = 0.07;
/// Keeps the open page from flickering against the stack.
const SHEET_GAP: f32 = 0.0002;
const TAB_HEIGHT: f32 = 0.0006;
/// How far a tab reaches out of the top edge of the pages.
const TAB_OUT: f32 = 0.014;
/// How much further the tab of the open page reaches out.
const TAB_RAISE: f32 = 0.006;
const TAB_GAP: f32 = 0.002;
/// The tabs share the top edge as if there were at least this many.
const TAB_SLOTS: usize = 5;
/// The label canvas has room for this many tabs.
const MAX_TABS: usize = 8;
const TAB_LABEL: UVec2 = UVec2::new(120, 64);
const TAB_FONT_SIZE: f32 = 26.0;
const COVER: Color = Color::srgb_u8(0x3a, 0x4a, 0x3c);
const RING: Color = Color::srgb_u8(0xa0, 0xa4, 0xa8);
const MANUAL_TAB: Color = Color::srgb_u8(0xe8, 0xcc, 0x6a);
const CHIP_TAB: Color = Color::srgb_u8(0x9a, 0xc0, 0xd8);

pub(crate) struct BinderPlugin;

impl Plugin for BinderPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Binder::on_add)
            .add_observer(Binder::open_at_job)
            .add_observer(BinderTab::open_on_click)
            .add_systems(
                Update,
                (
                    Binder::file
                        .run_if(resource_exists_and_changed::<Run>)
                        .before(Paper::show_current),
                    Binder::switch_on_keys,
                    BinderTab::raise_open,
                )
                    .chain(),
            );
    }
}

/// The ring binder with the human manuals the shop owns.
///
/// Each manual and each chip datasheet has a tab along the top edge. A click
/// on a tab or the up and down arrows go from tab to tab. When the binder is
/// opened for a new job, it opens on the page about that job.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Binder;

/// The canvases of a [`Binder`] and what is filed in it.
#[derive(Component)]
struct Filed {
    pages: UiCanvas,
    labels: UiCanvas,
    contents: Contents,
    /// The UI root of the tab labels.
    label_root: Option<Entity>,
    /// Height of the tabs above the binder's origin.
    tab_y: f32,
    /// The job the binder was last opened for.
    job: Option<JobId>,
}

/// A tab sticking out of a [`Binder`]. Clicking it opens its first page.
#[derive(Component, Debug)]
struct BinderTab {
    index: usize,
    rest: Vec3,
}

impl Binder {
    fn on_add(
        add: On<Add<Binder>>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let pages = UiCanvas::spawn(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let labels_size = TAB_LABEL * UVec2::new(MAX_TABS as u32, 1);
        let labels = UiCanvas::spawn(&mut commands, &mut images, labels_size, add.entity);
        let cover = Block::new(COVER_SIZE);
        let stack = Block::new(STACK_SIZE).on_top_of(&cover);
        let tab_y = stack.max().y;
        let sheet_center = Vec3::Y * (tab_y + TAB_HEIGHT + SHEET_GAP);

        let mut sheet = Entity::PLACEHOLDER;
        commands.entity(add.entity).with_children(|binder| {
            binder.spawn((Shape::cuboid(&cover, COVER), Finish::Fabric));
            binder.spawn((Shape::cuboid(&stack, PAPER), Finish::Paper));
            sheet = binder
                .spawn(Paper::sheet(
                    &pages,
                    SHEET_SIZE,
                    sheet_center,
                    &mut meshes,
                    &mut materials,
                ))
                .id();
            for side in [-1.0, 1.0] {
                let ring = Block::new(RING_SIZE)
                    .on_top_of(&stack)
                    .translated(Vec3::new(
                        -SHEET_SIZE.x / 2.0 + RING_INSET,
                        0.0,
                        side * RING_SPREAD,
                    ));
                binder.spawn((Shape::cylinder(&ring, RING), Finish::Metal));
            }
        });
        commands.entity(add.entity).insert((
            Paper::new(sheet, SHEET_SIZE, Vec::new()),
            Filed {
                pages,
                labels,
                contents: Contents::default(),
                label_root: None,
                tab_y,
                job: None,
            },
            FocusTarget {
                prompt: "click-binder",
                focus: Focus::Paper,
                view: View::Local(Paper::reading_view(sheet_center)),
            },
        ));
    }

    /// Files the manuals and errata the shop owns, and takes out those of an
    /// earlier run.
    fn file(
        run: Res<Run>,
        mut binders: Query<(Entity, &mut Paper, &mut Filed)>,
        tabs: Query<(Entity, &ChildOf), With<BinderTab>>,
        mut commands: Commands,
    ) {
        let contents = Contents::of(&run);
        for (binder, mut paper, mut filed) in &mut binders {
            if filed.contents == contents {
                continue;
            }
            let (pages, _) = contents.pages();
            let count = pages.len();
            let pages = pages
                .into_iter()
                .enumerate()
                .map(|(index, page)| {
                    Paper::spawn_page(
                        &mut commands,
                        &filed.pages,
                        (index + 1, count),
                        page.footer(),
                        |content| page.write(content),
                    )
                })
                .collect();
            for page in paper.replace_pages(pages) {
                commands.entity(page).despawn();
            }
            for (tab, parent) in &tabs {
                if parent.parent() == binder {
                    commands.entity(tab).despawn();
                }
            }
            if let Some(root) = filed.label_root.take() {
                commands.entity(root).despawn();
            }
            filed.label_root = Some(Self::spawn_tabs(
                &mut commands,
                binder,
                &filed,
                &contents.tabs,
            ));
            filed.contents = contents.clone();
            filed.job = None;
        }
    }

    /// Spawns the tabs and their labels and returns the UI root of the labels.
    fn spawn_tabs(commands: &mut Commands, binder: Entity, filed: &Filed, tabs: &[Tab]) -> Entity {
        let tabs = &tabs[..tabs.len().min(MAX_TABS)];
        let root = commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(PAPER),
                filed.labels.target(),
            ))
            .with_children(|row| {
                for &tab in tabs {
                    row.spawn((
                        Node {
                            width: px(TAB_LABEL.x),
                            height: px(TAB_LABEL.y),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(BinderTab::color(tab)),
                        children![(
                            tab.label(),
                            TextFont {
                                font_size: FontSize::Px(TAB_FONT_SIZE),
                                ..default()
                            },
                            TextColor(INK),
                        )],
                    ));
                }
            })
            .id();
        let slot = SHEET_SIZE.x / tabs.len().max(TAB_SLOTS) as f32;
        let size = Vec3::new(slot - TAB_GAP, TAB_HEIGHT, TAB_OUT);
        commands.entity(binder).with_children(|binder| {
            for index in 0..tabs.len() {
                let block = Block::new(size).translated(Vec3::new(
                    -SHEET_SIZE.x / 2.0 + slot * (index as f32 + 0.5),
                    filed.tab_y,
                    -(SHEET_SIZE.y + TAB_OUT) / 2.0,
                ));
                let region = Rect::new(
                    index as f32 / MAX_TABS as f32,
                    0.0,
                    (index + 1) as f32 / MAX_TABS as f32,
                    1.0,
                );
                binder.spawn((
                    Shape::textured_cuboid(&block, filed.labels.image.clone(), region),
                    Finish::Paper,
                    BinderTab {
                        index,
                        rest: block.center,
                    },
                ));
            }
        });
        root
    }

    /// The first time the binder is opened for a job, it goes to the page
    /// about it. After that, it stays on the page the player left it at.
    fn open_at_job(
        focus_on: On<FocusOn>,
        job: Option<Res<Job>>,
        mut binders: Query<(&mut Paper, &mut Filed)>,
    ) {
        let (Ok((mut paper, mut filed)), Some(job)) = (binders.get_mut(focus_on.entity), job)
        else {
            return;
        };
        if filed.job == Some(job.id) {
            return;
        }
        filed.job = Some(job.id);
        if let Some(page) = filed.contents.page_for(&job.level) {
            paper.open_page(page);
        }
    }

    /// The up and down arrows go to the tab before or after the open one.
    fn switch_on_keys(
        keys: Res<ButtonInput<KeyCode>>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        mut binders: Query<(&mut Paper, &Filed)>,
    ) {
        if *state.get() != Focus::Paper {
            return;
        }
        let Some((mut paper, filed)) = focused
            .get()
            .and_then(|entity| binders.get_mut(entity).ok())
        else {
            return;
        };
        let forward = if keys.just_pressed(KeyCode::ArrowDown) {
            true
        } else if keys.just_pressed(KeyCode::ArrowUp) {
            false
        } else {
            return;
        };
        if let Some(page) = filed.contents.next_tab(paper.current_page(), forward) {
            paper.open_page(page);
        }
    }
}

impl BinderTab {
    fn color(tab: Tab) -> Color {
        match tab {
            Tab::Manual(_) => MANUAL_TAB,
            Tab::Chip(_) => CHIP_TAB,
        }
    }

    fn open_on_click(
        click: On<PointerClick>,
        tabs: Query<(&BinderTab, &ChildOf)>,
        mut binders: Query<(&mut Paper, &Filed)>,
    ) {
        if click.button != PointerButton::Primary {
            return;
        }
        let Ok((tab, parent)) = tabs.get(click.entity) else {
            return;
        };
        let Ok((mut paper, filed)) = binders.get_mut(parent.parent()) else {
            return;
        };
        if let Some(page) = filed
            .contents
            .tabs
            .get(tab.index)
            .and_then(|&tab| filed.contents.start_of(tab))
        {
            paper.open_page(page);
        }
    }

    /// The tab of the open page reaches out further than the others.
    fn raise_open(
        binders: Query<(Entity, &Paper, &Filed), Changed<Paper>>,
        mut tabs: Query<(&BinderTab, &ChildOf, &mut Transform)>,
    ) {
        for (binder, paper, filed) in &binders {
            let open = filed.contents.tab_index_at(paper.current_page());
            for (tab, parent, mut transform) in &mut tabs {
                if parent.parent() != binder {
                    continue;
                }
                let raise = if Some(tab.index) == open {
                    TAB_RAISE
                } else {
                    0.0
                };
                transform.translation = tab.rest - Vec3::Z * raise;
            }
        }
    }
}

impl Contents {
    /// The first page of `tab`, counted from 0.
    fn start_of(&self, tab: Tab) -> Option<usize> {
        let index = self.tabs.iter().position(|&other| other == tab)?;
        Some(self.starts()[index])
    }

    /// The index of the tab that `page` is in.
    fn tab_index_at(&self, page: usize) -> Option<usize> {
        self.starts().iter().rposition(|&start| start <= page)
    }

    fn starts(&self) -> Vec<usize> {
        self.pages().1
    }

    /// The page about a job on `level`: the page about its board in the
    /// manual, or the datasheet of its chip if the manual is missing.
    fn page_for(&self, level: &Level) -> Option<usize> {
        let ChipSpec::Calco(chip) = level.chip;
        match self.start_of(Tab::Manual(level.device)) {
            Some(start) => Some(start + level.board as usize),
            None => self.start_of(Tab::Chip(chip.revision)),
        }
    }

    /// The first page of the tab after or before the one with `page`.
    fn next_tab(&self, page: usize, forward: bool) -> Option<usize> {
        let index = self.tab_index_at(page)?;
        let next = if forward {
            index + 1
        } else {
            index.checked_sub(1)?
        };
        self.starts().get(next).copied()
    }
}

#[cfg(test)]
mod tests {
    use ld60_calco::Revision;
    use ld60_devices::{BoardRev, DeviceKind};

    use super::*;
    use crate::locale::{Locale, Localized, Message};
    use crate::player::focus::FocusOn;
    use crate::workshop::levels::Levels;
    use crate::workshop::salvage::Find;

    fn rich_run() -> Run {
        let mut run = Run::default();
        run.add_till(1000);
        assert!(run.buy(Find::Device(DeviceKind::Summa6)));
        assert!(run.buy(Find::Errata(Revision::B)));
        run
    }

    #[test]
    fn the_first_run_files_the_tally2_and_its_chips() {
        let contents = Contents::of(&Run::default());
        assert_eq!(
            contents.tabs,
            [
                Tab::Manual(DeviceKind::Tally2),
                Tab::Chip(Revision::A),
                Tab::Chip(Revision::B),
                Tab::Chip(Revision::C),
            ]
        );
    }

    #[test]
    fn buying_files_the_manual_and_the_errata() {
        let before = Contents::of(&Run::default());
        let after = Contents::of(&rich_run());
        assert_ne!(before, after);
        assert_eq!(after.tabs[1], Tab::Manual(DeviceKind::Summa6));
        assert!(after.tabs.len() <= MAX_TABS);
    }

    #[test]
    fn every_page_formats() {
        let locale = Locale::on_disk();
        for run in [Run::default(), rich_run()] {
            let contents = Contents::of(&run);
            for tab in &contents.tabs {
                locale.assert_formats([tab.label()]);
            }
            for page in contents.pages().0 {
                locale.assert_formats([page.footer()]);
                locale.assert_formats(Localized::spawned_by(|content| page.write(content)));
            }
        }
        locale.assert_formats([Message::new("click-binder")]);
    }

    #[test]
    fn a_job_opens_its_board() {
        let contents = Contents::of(&rich_run());
        let mut level = Levels::on_disk().levels()[0].load().unwrap();
        level.device = DeviceKind::Summa6;
        level.board = BoardRev::C;
        let manual = contents.start_of(Tab::Manual(DeviceKind::Summa6)).unwrap();
        assert_eq!(contents.page_for(&level), Some(manual + 2));
    }

    #[test]
    fn each_job_opens_on_its_own_board() {
        let mut world = World::new();
        world.init_resource::<Run>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.add_observer(Binder::on_add);
        world.add_observer(Binder::open_at_job);
        let binder = world.spawn(Binder).id();
        world.flush();
        world.run_system_cached(Binder::file).unwrap();
        let current = |world: &mut World| world.get::<Paper>(binder).unwrap().current_page();
        assert_eq!(current(&mut world), 0);
        for board in [BoardRev::B, BoardRev::C, BoardRev::A] {
            let mut job = Job::load(&Levels::on_disk(), 0).unwrap().unwrap();
            job.level.device = DeviceKind::Tally2;
            job.level.board = board;
            world.insert_resource(job);
            world.trigger(FocusOn { entity: binder });
            world.flush();
            assert_eq!(current(&mut world), board as usize);
        }
        world.get_mut::<Paper>(binder).unwrap().open_page(4);
        world.trigger(FocusOn { entity: binder });
        world.flush();
        assert_eq!(current(&mut world), 4);
    }

    #[test]
    fn arrows_go_from_tab_to_tab() {
        let contents = Contents::of(&Run::default());
        let starts = contents.starts();
        assert_eq!(contents.next_tab(1, true), Some(starts[1]));
        assert_eq!(contents.next_tab(starts[1] + 1, false), Some(0));
        assert_eq!(contents.next_tab(0, false), None);
        let last = contents.pages().0.len() - 1;
        assert_eq!(contents.next_tab(last, true), None);
    }

    #[test]
    fn tabs_fill_their_label_canvas_cells() {
        let mut world = World::new();
        world.init_resource::<Run>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.add_observer(Binder::on_add);
        let binder = world.spawn(Binder).id();
        world.flush();
        world.run_system_cached(Binder::file).unwrap();
        let tabs = world.query::<&BinderTab>().iter(&world).count();
        assert_eq!(tabs, Contents::of(&Run::default()).tabs.len());
        world.insert_resource(rich_run());
        world.run_system_cached(Binder::file).unwrap();
        let tabs: Vec<_> = world
            .query::<(&BinderTab, &ChildOf)>()
            .iter(&world)
            .filter(|(_, parent)| parent.parent() == binder)
            .map(|(tab, _)| tab.index)
            .collect();
        assert_eq!(tabs.len(), Contents::of(&rich_run()).tabs.len());
    }
}
