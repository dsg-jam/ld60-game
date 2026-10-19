mod contents;
mod datasheet;
mod index;
mod model;
mod pins;

use bevy::{
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};

use self::contents::{Contents, Section};
use self::index::{Hover, PageLink, PinButton};
use self::model::{Cover, DividerTab, Parts, SPINE_CANVAS, TAB_LABEL};
pub(crate) use self::pins::{Pins, Sheet};
use super::job::{Job, JobId};
use super::run::Run;
use crate::locale::{Localized, Message};
use crate::parts::paper::{Binding, INK, Paper};
use crate::parts::screen::{ScreenSurface, UiCanvas};
use crate::player::focus::{Focus, FocusOn, FocusTarget, Focused, View};
use crate::player::hints::KeyHint;

const CANVAS_SIZE: UVec2 = UVec2::new(640, 880);
const TAB_FONT_SIZE: f32 = 30.0;
const PIN_KEY: KeyCode = KeyCode::KeyP;
/// Shows the stack below through the punched holes.
const HOLE: Color = Color::srgb_u8(0x8a, 0x84, 0x76);

pub(crate) struct ManualsPlugin;

#[cfg(test)]
impl ShopManuals {
    pub(super) const CLOSED: Vec3 = model::CLOSED;
}

impl Plugin for ManualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(ShopManuals::on_add)
            .add_observer(ShopManuals::open_at_job)
            .add_observer(PageLink::open_on_click)
            .add_observer(DividerTab::open_on_click)
            .add_observer(PinButton::toggle_on_click)
            .add_observer(Hover::show_enter)
            .add_observer(Hover::show_leave)
            .add_systems(
                Update,
                (
                    ShopManuals::file
                        .run_if(resource_exists_and_changed::<Run>)
                        .before(Paper::show_current),
                    ShopManuals::switch_on_keys,
                    ShopManuals::pin_on_key,
                    ShopManuals::show_open,
                    ShopManuals::stack_pages,
                    PinButton::show_state,
                    ShopManuals::show_pin_hint,
                )
                    .chain(),
            );
    }
}

/// The lever arch file with the human manuals the shop owns.
///
/// It stands closed on the counter and lies open at `open_at` while the
/// player reads it. The first page is an index of the sections Gadgets,
/// Chips, Errata and Notes. Clicking a line of the index, a divider tab or
/// the INDEX button at the top of a page goes there. The up and down arrows
/// go from entry to entry and Home goes to the index. When the file is
/// opened for a new job, it opens on the page about that job. P or the PIN
/// button at the top of a page pins it to the monitor, see [`Pins`].
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct ShopManuals {
    /// Where the file lies open, in the same space as its [`Transform`].
    pub(crate) open_at: Transform,
}

/// The pages of [`ShopManuals`] and what is filed in it.
#[derive(Component)]
struct Filed {
    pages: UiCanvas,
    labels: UiCanvas,
    contents: Contents,
    /// The job the file was last opened for.
    job: Option<JobId>,
}

impl ShopManuals {
    fn on_add(
        add: On<Add<ShopManuals>>,
        files: Query<(&ShopManuals, &Transform)>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Ok((file, stand)) = files.get(add.entity) else {
            return;
        };
        let open =
            GlobalTransform::from(file.open_at).reparented_to(&GlobalTransform::from(*stand));
        let pages = UiCanvas::spawn_print(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let spine = UiCanvas::spawn_smooth(&mut commands, &mut images, SPINE_CANVAS, add.entity);
        let labels = UiCanvas::spawn_smooth(
            &mut commands,
            &mut images,
            DividerTab::labels_size(),
            add.entity,
        );
        model::write_spine(&mut commands, &spine);
        Self::write_tab_labels(&mut commands, &labels);
        let mut parts = None;
        commands.entity(add.entity).with_children(|file| {
            parts = Some(Parts::spawn(
                file,
                open,
                &pages,
                &spine,
                &mut meshes,
                &mut materials,
            ));
        });
        let Some(parts) = parts else {
            return;
        };
        commands
            .entity(parts.sheet)
            .insert(ScreenSurface::new(&pages));
        commands.entity(add.entity).insert((
            Paper::new(parts.sheet, model::SHEET_SIZE, Vec::new()),
            Binding::rings(model::SHEET_SIZE, model::ring_gap()),
            Cover::default(),
            FocusTarget {
                prompt: "click-manuals",
                focus: Focus::Paper,
                view: View::Local(open.mul_transform(Parts::reading_view())),
            },
            parts,
            Filed {
                pages,
                labels,
                contents: Contents::default(),
                job: None,
            },
        ));
    }

    fn write_tab_labels(commands: &mut Commands, labels: &UiCanvas) {
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                labels.target(),
            ))
            .with_children(|row| {
                for section in Section::ALL {
                    row.spawn((
                        Node {
                            width: px(TAB_LABEL.x),
                            height: px(TAB_LABEL.y),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(section.color()),
                        children![(
                            Localized::from(section.label()),
                            TextFont {
                                font_size: FontSize::Px(TAB_FONT_SIZE),
                                ..default()
                            },
                            TextColor(INK),
                        )],
                    ));
                }
            });
    }

    /// Files the manuals, errata and notes the shop owns, and takes out
    /// those of an earlier run.
    fn file(
        run: Res<Run>,
        mut files: Query<(Entity, &mut Paper, &mut Filed, &Parts)>,
        tabs: Query<(Entity, &DividerTab)>,
        mut commands: Commands,
    ) {
        let contents = Contents::of(&run);
        for (file, mut paper, mut filed, parts) in &mut files {
            if filed.contents == contents {
                continue;
            }
            let pages = contents.pages();
            let count = pages.len();
            let pages = pages
                .into_iter()
                .enumerate()
                .map(|(index, page)| {
                    Paper::spawn_page(
                        &mut commands,
                        &filed.pages,
                        (index + 1, count),
                        page.0.footer(),
                        |content| {
                            contents.write(file, page, content);
                            Self::punch(content);
                        },
                    )
                })
                .collect();
            for page in paper.replace_pages(pages) {
                commands.entity(page).despawn();
            }
            for (tab, divider) in &tabs {
                if divider.file == file {
                    commands.entity(tab).despawn();
                }
            }
            commands.entity(parts.open).with_children(|open| {
                let starts = Section::ALL
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, &section)| {
                        Some((slot, contents.start_of_section(section)?))
                    });
                for (position, (slot, start)) in starts.enumerate() {
                    DividerTab::spawn(open, file, (slot, position, start), &filed.labels);
                }
            });
            filed.contents = contents.clone();
            filed.job = None;
        }
    }

    /// The first time the file is opened for a job, it goes to the page
    /// about it. After that, it stays on the page the player left it at.
    fn open_at_job(
        focus_on: On<FocusOn>,
        job: Option<Res<Job>>,
        mut files: Query<(&mut Paper, &mut Filed)>,
    ) {
        let (Ok((mut paper, mut filed)), Some(job)) = (files.get_mut(focus_on.entity), job) else {
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

    /// The up and down arrows go to the entry before or after the open one.
    /// Home goes to the index.
    fn switch_on_keys(
        keys: Res<ButtonInput<KeyCode>>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        mut files: Query<(&mut Paper, &Filed)>,
    ) {
        if *state.get() != Focus::Paper {
            return;
        }
        let Some((mut paper, filed)) = focused.get().and_then(|entity| files.get_mut(entity).ok())
        else {
            return;
        };
        let page = if keys.just_pressed(KeyCode::Home) {
            Some(0)
        } else if keys.just_pressed(KeyCode::ArrowDown) {
            filed.contents.next_entry(paper.current_page(), true)
        } else if keys.just_pressed(KeyCode::ArrowUp) {
            filed.contents.next_entry(paper.current_page(), false)
        } else {
            return;
        };
        if let Some(page) = page {
            paper.open_page(page);
        }
    }

    /// The page open in the file the player reads, unless it is the index.
    fn open_sheet(
        state: &State<Focus>,
        focused: &Focused,
        files: &Query<(&Paper, &Filed)>,
    ) -> Option<Sheet> {
        if *state.get() != Focus::Paper {
            return None;
        }
        let (paper, filed) = files.get(focused.get()?).ok()?;
        filed.contents.sheet_at(paper.current_page())
    }

    fn pin_on_key(
        keys: Res<ButtonInput<KeyCode>>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        files: Query<(&Paper, &Filed)>,
        mut run: ResMut<Run>,
    ) {
        if keys.just_pressed(PIN_KEY)
            && let Some(sheet) = Self::open_sheet(&state, &focused, &files)
        {
            run.pins_mut().toggle(sheet);
        }
    }

    fn show_pin_hint(
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        files: Query<(&Paper, &Filed)>,
        run: Res<Run>,
        mut hint: ResMut<KeyHint>,
    ) {
        let action = Self::open_sheet(&state, &focused, &files).map(|sheet| {
            let id = if run.pins().contains(sheet) {
                "hint-unpin"
            } else {
                "hint-pin"
            };
            ("P", Message::new(id))
        });
        hint.set_if_neq(KeyHint(action));
    }

    /// The holes the rings go through, near the spine edge of the page.
    fn punch(page: &mut ChildSpawnerCommands) {
        let scale = CANVAS_SIZE.x as f32 / model::SHEET_SIZE.x;
        let radius = model::HOLE_RADIUS * scale;
        for hole in model::holes() {
            page.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(model::HOLE_INSET * scale - radius),
                    top: px(hole * scale - radius),
                    width: px(2.0 * radius),
                    height: px(2.0 * radius),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(HOLE),
            ));
        }
    }

    /// The file lies open while the player reads it and stands closed
    /// otherwise. Its cover swings open once it lies on the mat and swings
    /// shut before it stands up again.
    fn show_open(
        time: Res<Time>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        mut files: Query<(Entity, &Parts, &mut Cover)>,
        mut transforms: Query<&mut Transform>,
        mut visibilities: Query<&mut Visibility>,
        mut surfaces: Query<&mut ScreenSurface>,
    ) {
        for (file, parts, mut cover) in &mut files {
            let open = *state.get() == Focus::Paper && focused.get() == Some(file);
            let lying =
                parts.swing_cover(&mut cover, open, time.delta_secs(), &mut transforms) || open;
            let shown = |shown: bool| {
                if shown {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                }
            };
            if let Ok(mut visibility) = visibilities.get_mut(parts.open) {
                visibility.set_if_neq(shown(lying));
            }
            if let Ok(mut visibility) = visibilities.get_mut(parts.closed) {
                visibility.set_if_neq(shown(!lying));
            }
            if let Ok(mut surface) = surfaces.get_mut(parts.sheet)
                && surface.active != open
            {
                surface.active = open;
            }
        }
    }

    /// Moves the turned pages and dividers to the front cover and the rest
    /// to the back cover.
    fn stack_pages(
        files: Query<(Entity, &Paper, &Parts), Changed<Paper>>,
        dividers: Query<(Entity, &DividerTab, &ChildOf)>,
        mut transforms: Query<&mut Transform>,
        mut visibilities: Query<&mut Visibility>,
        mut commands: Commands,
    ) {
        for (file, paper, parts) in &files {
            let pages = (paper.current_page(), paper.page_count());
            parts.stack_pages(pages, &mut transforms, &mut visibilities);
            for (entity, divider, parent) in &dividers {
                if divider.file != file {
                    continue;
                }
                let holder = parts.holder(divider.start <= pages.0);
                if parent.parent() != holder {
                    commands.entity(entity).insert(ChildOf(holder));
                }
                divider.place(pages, entity, &mut transforms);
            }
        }
    }
}

impl DividerTab {
    fn open_on_click(
        click: On<PointerClick>,
        tabs: Query<&DividerTab>,
        mut files: Query<(&mut Paper, &Filed)>,
    ) {
        if click.button != PointerButton::Primary {
            return;
        }
        let Ok(tab) = tabs.get(click.entity) else {
            return;
        };
        let Ok((mut paper, filed)) = files.get_mut(tab.file) else {
            return;
        };
        if let Some(page) = filed.contents.start_of_section(Section::ALL[tab.slot]) {
            paper.open_page(page);
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_calco::Revision;
    use ld60_devices::{BoardRev, DeviceKind};

    use super::contents::Entry;
    use super::*;
    use crate::locale::{Locale, Localized, Message};
    use crate::workshop::levels::Levels;
    use crate::workshop::salvage::Find;
    use crate::workshop::tutorial::TutorialNote;

    fn rich_run() -> Run {
        let mut run = Run::default();
        run.add_till(1000);
        assert!(run.buy(Find::Device(DeviceKind::Summa6)));
        assert!(run.buy(Find::Errata(Revision::B)));
        run.file_note(1);
        run.file_note(0);
        run
    }

    fn world_with_file() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<Run>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.add_observer(ShopManuals::on_add);
        world.add_observer(ShopManuals::open_at_job);
        let file = world
            .spawn(ShopManuals {
                open_at: Transform::from_xyz(0.3, 0.0, 0.2),
            })
            .id();
        world.flush();
        world.run_system_cached(ShopManuals::file).unwrap();
        (world, file)
    }

    #[test]
    fn the_first_run_files_the_tally2_and_the_rev_a_datasheet() {
        let contents = Contents::of(&Run::default());
        assert_eq!(
            contents.entries(),
            [Entry::Manual(DeviceKind::Tally2), Entry::Chip(Revision::A)]
        );
    }

    #[test]
    fn sections_come_in_order() {
        let contents = Contents::of(&rich_run());
        let sections: Vec<_> = contents
            .entries()
            .iter()
            .map(|entry| entry.section())
            .collect();
        let mut sorted = sections.clone();
        sorted.sort_by_key(|section| Section::ALL.iter().position(|other| other == section));
        assert_eq!(sections, sorted);
        assert!(
            contents
                .entries()
                .contains(&Entry::Manual(DeviceKind::Summa6))
        );
        assert!(contents.entries().contains(&Entry::Errata(Revision::B)));
        let notes: Vec<_> = contents
            .entries()
            .iter()
            .filter(|entry| entry.section() == Section::Notes)
            .collect();
        assert_eq!(
            notes,
            [
                &Entry::Note(TutorialNote::new(0).unwrap()),
                &Entry::Note(TutorialNote::new(1).unwrap())
            ]
        );
    }

    #[test]
    fn the_index_comes_first_and_lists_every_entry() {
        let contents = Contents::of(&rich_run());
        let pages = contents.pages();
        assert!(pages[0].1.is_none());
        for &entry in contents.entries() {
            let start = contents.start_of(entry).unwrap();
            assert_eq!(pages[start].1, Some(Sheet::new(entry, 0)));
            assert_eq!(contents.entry_at(start), Some(entry));
        }
        assert_eq!(contents.entry_at(0), None);
        let notes = contents.start_of_section(Section::Notes).unwrap();
        assert_eq!(notes, pages.len() - 2);
    }

    #[test]
    fn every_page_formats() {
        let locale = Locale::on_disk();
        for run in [Run::default(), rich_run()] {
            let contents = Contents::of(&run);
            for page in contents.pages() {
                locale.assert_formats([page.0.footer()]);
                locale.assert_formats(Localized::spawned_by(|content| {
                    contents.write(Entity::PLACEHOLDER, page, content);
                }));
            }
        }
        locale.assert_formats(Section::ALL.map(Section::label));
        locale.assert_formats([Message::new("click-manuals"), Message::new("manuals-spine")]);
        locale.assert_formats([true, false].map(PinButton::label));
        locale.assert_formats([Message::new("hint-pin"), Message::new("hint-unpin")]);
    }

    #[test]
    fn every_page_but_the_index_is_a_sheet() {
        let contents = Contents::of(&rich_run());
        for (index, (page, sheet)) in contents.pages().into_iter().enumerate() {
            assert_eq!(contents.sheet_at(index), sheet);
            if let Some(sheet) = sheet {
                assert_eq!(format!("{:?}", sheet.page()), format!("{:?}", Some(page)));
            }
        }
        assert_eq!(contents.sheet_at(0), None);
    }

    #[test]
    fn the_pin_key_pins_the_open_page() {
        let (mut world, file) = world_with_file();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.insert_resource(State::new(Focus::Paper));
        world.init_resource::<Focused>();
        world.init_resource::<NextState<Focus>>();
        world.add_observer(FocusOn::apply);
        world.trigger(FocusOn { entity: file });
        world.flush();
        let press = |world: &mut World| {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.release(PIN_KEY);
            keys.clear();
            keys.press(PIN_KEY);
            world.run_system_cached(ShopManuals::pin_on_key).unwrap();
        };
        press(&mut world);
        assert!(world.resource::<Run>().pins().is_empty());
        world.get_mut::<Paper>(file).unwrap().open_page(2);
        press(&mut world);
        let pinned = Sheet::new(Entry::Manual(DeviceKind::Tally2), 1);
        assert_eq!(world.resource::<Run>().pins().sheets(), [pinned]);
        press(&mut world);
        assert!(world.resource::<Run>().pins().is_empty());
    }

    #[test]
    fn a_job_opens_its_board() {
        let contents = Contents::of(&rich_run());
        let mut level = Levels::on_disk().levels()[0].load().unwrap();
        level.device = DeviceKind::Summa6;
        level.board = BoardRev::C;
        let manual = contents
            .start_of(Entry::Manual(DeviceKind::Summa6))
            .unwrap();
        assert_eq!(contents.page_for(&level), Some(manual + 2));
    }

    #[test]
    fn each_job_opens_on_its_own_board() {
        let (mut world, file) = world_with_file();
        let current = |world: &mut World| world.get::<Paper>(file).unwrap().current_page();
        assert_eq!(current(&mut world), 0);
        for board in [BoardRev::B, BoardRev::C, BoardRev::A] {
            let mut job = Job::load(&Levels::on_disk(), 0).unwrap().unwrap();
            job.level.device = DeviceKind::Tally2;
            job.level.board = board;
            world.insert_resource(job);
            world.trigger(FocusOn { entity: file });
            world.flush();
            assert_eq!(current(&mut world), 1 + board as usize);
        }
        world.get_mut::<Paper>(file).unwrap().open_page(4);
        world.trigger(FocusOn { entity: file });
        world.flush();
        assert_eq!(current(&mut world), 4);
    }

    #[test]
    fn arrows_go_from_entry_to_entry() {
        let contents = Contents::of(&Run::default());
        let chips = contents.start_of(Entry::Chip(Revision::A)).unwrap();
        assert_eq!(contents.next_entry(0, true), Some(1));
        assert_eq!(contents.next_entry(2, true), Some(chips));
        assert_eq!(contents.next_entry(chips + 1, false), Some(1));
        assert_eq!(contents.next_entry(1, false), Some(0));
        assert_eq!(contents.next_entry(0, false), None);
        let last = contents.pages().len() - 1;
        assert_eq!(contents.next_entry(last, true), None);
    }

    #[test]
    fn only_sections_with_entries_get_a_tab() {
        let (mut world, file) = world_with_file();
        let slots = |world: &mut World| -> Vec<usize> {
            world
                .query::<&DividerTab>()
                .iter(world)
                .filter(|tab| tab.file == file)
                .map(|tab| tab.slot)
                .collect()
        };
        assert_eq!(slots(&mut world), [0, 1]);
        world.insert_resource(rich_run());
        world.run_system_cached(ShopManuals::file).unwrap();
        world.flush();
        let mut after = slots(&mut world);
        after.sort_unstable();
        assert_eq!(after, [0, 1, 2, 3]);
    }
}
