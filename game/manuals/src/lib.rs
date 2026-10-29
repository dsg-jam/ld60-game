mod cloth;
mod contents;
mod datasheet;
mod dividers;
mod in_file;
mod index;
mod layout;
mod mechanism;
mod model;
mod moves;
mod pins;
mod pose;
mod punched;
mod service;
mod stack;
mod top_sheet;
mod turning;

use bevy::{platform::hash::fixed_hash_one, prelude::*};
use ld60_game_audio::{Play, Sound};
use ld60_game_parts::paper::{
    Binding, Leaf, LeafShape, PAPER, PAPER_REFLECTANCE, PAPER_ROUGHNESS, Paper, ShowsOwnPages,
    Wrinkles,
};
use ld60_game_parts::screen::{ScreenSurface, UiCanvas};
use ld60_game_player::focus::{Focus, FocusOn, FocusTarget, Focused, View};
use ld60_game_player::hints::{Hint, Hints, Icon, Input, KeyHint};
use ld60_game_run::Run;
use ld60_game_run::job::{Job, JobId};
use ld60_game_run::tutorial::{FirstNote, TutorialNote};
use ld60_geometry::surface::{Finish, Form, SurfaceMaterial};
use ld60_locale::Message;

use self::contents::{Contents, Entry};
use self::dividers::{DividerTab, Tab};
use self::in_file::InFile;
use self::index::{Hover, Links, PageLink};
pub use self::layout::Layouts;
use self::layout::LayoutsPlugin;
use self::model::{Parts, SHEET_SIZE, SPINE_CANVAS};
use self::moves::SheetMoves;
pub use self::moves::{MoveSheet, RailSlot};
pub use self::pins::{Pins, Sheet};
pub use self::pose::Pose;
use self::stack::Stack;
use self::turning::{LeafKit, PageShape};

const CANVAS_SIZE: UVec2 = UVec2::new(640, 880);
const PIN_KEY: KeyCode = KeyCode::KeyP;

pub struct ManualsPlugin;

#[cfg(any(test, feature = "test-support"))]
impl ShopManuals {
    pub const CLOSED: Vec3 = model::CLOSED;
}

impl Plugin for ManualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LayoutsPlugin)
            .init_resource::<Pins>()
            .add_systems(
                Update,
                (
                    Pins::pin_first_note.run_if(resource_added::<FirstNote>),
                    Pins::keep_filed_when_laid.run_if(resource_exists_and_changed::<Layouts>),
                ),
            )
            .add_observer(Pins::clear_for_new_run)
            .add_observer(ShopManuals::on_add)
            .add_observer(ShopManuals::open_at_job)
            .add_observer(PageLink::open_on_click)
            .add_observer(DividerTab::open_on_click)
            .add_observer(SheetMoves::wait)
            .add_observer(Hover::show_enter)
            .add_observer(Hover::show_leave)
            .add_systems(
                Update,
                (
                    SheetMoves::fly.run_if(resource_exists::<Layouts>),
                    ShopManuals::file.run_if(resource_exists::<Layouts>),
                    ShopManuals::switch_on_keys,
                    ShopManuals::pin_on_key,
                    LeafKit::make,
                    Stack::turn,
                    ShopManuals::show_open,
                    Stack::arrange,
                    Stack::show_pages,
                    Stack::relax_top_sheets,
                    ShopManuals::show_pin_hint,
                )
                    .chain()
                    .after(Leaf::animate),
            );
    }
}

/// The ring binder with the human manuals the shop owns.
///
/// It stands closed on the counter, and lies down and opens at `open_at`
/// while the player reads it. The first page is an index of the sections
/// Gadgets, Chips, Errata and Notes. Clicking a line of the index, a divider
/// tab or the INDEX button at the top of a page goes there. The up and down
/// arrows go from entry to entry and Home goes to the index. When the file
/// is opened for a new job, it opens on the page about that job. P or the
/// PIN button at the top of a page pins it to the monitor, see [`Pins`].
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct ShopManuals {
    /// Where the file lies open, in the same space as its [`Transform`].
    pub open_at: Transform,
}

/// The pages of [`ShopManuals`] and what is filed in it.
#[derive(Component)]
struct Filed {
    pages: UiCanvas,
    /// Draws the page on a turning leaf.
    leaf_pages: UiCanvas,
    /// Draws the page on its way to or from the rail.
    flight: UiCanvas,
    labels: UiCanvas,
    contents: Contents,
    /// The pins the file was filed for. Pinned pages are not in it.
    pins: Pins,
    in_file: InFile,
    /// The shape of each page in the file.
    shapes: Vec<PageShape>,
    /// The job the file was last opened for.
    job: Option<JobId>,
}

impl ShopManuals {
    /// The file lying open, as a box around both covers and the spine.
    pub const OPEN: Vec3 = Vec3::new(2.0 * model::COVER.x + model::SPINE, 0.012, model::COVER.y);

    fn on_add(
        add: On<Add<ShopManuals>>,
        files: Query<(&ShopManuals, &Transform)>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let Ok((file, stand)) = files.get(add.entity) else {
            return;
        };
        let lying =
            GlobalTransform::from(file.open_at).reparented_to(&GlobalTransform::from(*stand));
        let pages = UiCanvas::spawn_print(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let leaf_pages = UiCanvas::spawn_print(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let flight = UiCanvas::spawn_print(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let spine = UiCanvas::spawn_smooth(&mut commands, &mut images, SPINE_CANVAS, add.entity);
        let labels = UiCanvas::spawn_smooth(
            &mut commands,
            &mut images,
            DividerTab::labels_size(),
            add.entity,
        );
        model::write_spine(&mut commands, &spine);
        DividerTab::write_labels(&mut commands, &labels);
        let edges = surfaces.add(Finish::Paper.over(
            StandardMaterial {
                base_color: PAPER,
                perceptual_roughness: PAPER_ROUGHNESS,
                reflectance: PAPER_REFLECTANCE,
                ..default()
            },
            Form::Freeform,
            0.0,
        ));
        let mut parts = None;
        commands.entity(add.entity).with_children(|file| {
            parts = Some(Parts::spawn(
                file,
                (&pages, &spine),
                edges,
                &mut meshes,
                (&mut materials, &mut surfaces),
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
            ShowsOwnPages,
            Binding::rings(),
            Pose::default(),
            Stack::default(),
            SheetMoves::default(),
            FocusTarget {
                prompt: "click-manuals",
                focus: Focus::Paper,
                view: View::Local(lying.mul_transform(Parts::reading_view())),
            },
            Lying(lying),
            Hints(vec![
                Hint {
                    inputs: vec![Input::Icon(Icon::Wheel), Input::Icon(Icon::ArrowsSideways)],
                    action: Message::new("hint-turn-pages"),
                },
                Hint {
                    inputs: vec![Input::Icon(Icon::ArrowsUpDown)],
                    action: Message::new("hint-next-entry"),
                },
                Hint {
                    inputs: vec![Input::Key("Home")],
                    action: Message::new("hint-open-index"),
                },
            ]),
            parts,
            Filed {
                pages,
                leaf_pages,
                flight,
                labels,
                contents: Contents::default(),
                pins: Pins::default(),
                in_file: InFile::default(),
                shapes: Vec::new(),
                job: None,
            },
        ));
    }

    /// Files the manuals, errata and notes the shop owns, and takes out
    /// those of an earlier run. Pinned pages stay out until they are
    /// unpinned, and so does a page on its way to the rail. The file stays
    /// at the page it is open at, or the one after it if that page is out.
    fn file(
        (run, layouts): (Res<Run>, Res<Layouts>),
        pinned: Res<Pins>,
        mut files: Query<(
            Entity,
            &mut Paper,
            &mut Filed,
            &mut Stack,
            &Parts,
            &SheetMoves,
        )>,
        tabs: Query<(Entity, &DividerTab)>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
        mut commands: Commands,
    ) {
        let mut shop = None;
        for (file, mut paper, mut filed, mut stack, parts, moves) in &mut files {
            let pins = moves.filed_pins(&pinned);
            if !run.is_changed() && !layouts.is_changed() && filed.pins == pins {
                continue;
            }
            let contents: &Contents = shop.get_or_insert_with(|| Contents::of(&run, &layouts));
            if filed.contents == *contents && filed.pins == pins {
                continue;
            }
            let refiled = filed.contents != *contents;
            let open = filed.in_file.number(paper.current_page());
            let in_file = InFile::new(contents, &pins);
            let all = contents.pages();
            let count = all.len();
            let links = Links {
                file,
                in_file: &in_file,
            };
            let mut seeds = Vec::new();
            let mut pages = Vec::new();
            filed.shapes.clear();
            for &number in in_file.numbers() {
                let page = &all[number];
                let seed = fixed_hash_one(("manuals", page.1));
                let wrinkles = Wrinkles::held(SHEET_SIZE, seed, &Binding::rings());
                filed.shapes.push(PageShape {
                    leaf: LeafShape::new(&wrinkles),
                    wrinkles,
                });
                seeds.push(seed);
                let spawned = Paper::spawn_page(
                    &mut commands,
                    &filed.pages,
                    (number + 1, count),
                    page.0.footer(),
                    |content| contents.write(page, &links, content),
                );
                if let Some(sheet) = page.1 {
                    commands.entity(spawned).insert(sheet.print(&layouts));
                }
                pages.push(spawned);
            }
            for page in paper.replace_pages(pages) {
                commands.entity(page).despawn();
            }
            if !refiled && let Some(page) = open.and_then(|number| in_file.from(number)) {
                paper.open_page(page);
            }
            for (tab, divider) in &tabs {
                if divider.file == file {
                    commands.entity(tab).despawn();
                }
            }
            if let Some(turning) = stack.turning.take() {
                commands.entity(turning.leaf).despawn();
            }
            let mut cards = Vec::new();
            commands.entity(parts.spine).with_children(|spine| {
                let starts = Tab::ALL
                    .into_iter()
                    .filter_map(|tab| Some((tab, tab.start(contents)?)));
                for (position, (tab, start)) in starts.enumerate() {
                    // A divider stays in front of the first page after it
                    // that is in the file.
                    let start = in_file.from(start).unwrap_or(in_file.numbers().len());
                    let card = DividerTab::spawn(
                        spine,
                        file,
                        (tab, position, start),
                        &filed.labels,
                        (&mut meshes, &mut surfaces),
                    );
                    cards.push((start, card, tab.seed()));
                }
            });
            *stack = Stack::new(&seeds, &cards);
            stack.jump_to(paper.current_page());
            filed.contents = contents.clone();
            filed.pins = pins;
            filed.in_file = in_file;
            if refiled {
                filed.job = None;
            }
        }
    }

    /// The first time the file is opened for a job, it goes to the page
    /// about it: the note about a job of the tutorial, or the page about its
    /// board. After that, it stays on the page the player left it at.
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
        let note = TutorialNote::for_job(&job).and_then(|note| {
            let start = filed.contents.start_of(Entry::Note(note))?;
            filed.in_file.page(start)
        });
        if let Some(page) = note.or_else(|| {
            filed
                .contents
                .page_for(&job.level)
                .and_then(|number| filed.in_file.from(number))
        }) {
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
        let Some(open) = filed.in_file.number(paper.current_page()) else {
            return;
        };
        let forward = if keys.just_pressed(KeyCode::Home) {
            paper.open_page(0);
            return;
        } else if keys.just_pressed(KeyCode::ArrowDown) {
            true
        } else if keys.just_pressed(KeyCode::ArrowUp) {
            false
        } else {
            return;
        };
        // Skips entries with all their pages pinned.
        let mut number = open;
        while let Some(next) = filed.contents.next_entry(number, forward) {
            if let Some(page) = filed.in_file.from(next)
                && filed
                    .in_file
                    .number(page)
                    .and_then(|number| filed.contents.entry_at(number))
                    == filed.contents.entry_at(next)
            {
                paper.open_page(page);
                return;
            }
            number = next;
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
        filed
            .contents
            .sheet_at(filed.in_file.number(paper.current_page())?)
    }

    fn pin_on_key(
        keys: Res<ButtonInput<KeyCode>>,
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        files: Query<(&Paper, &Filed)>,
        pins: Res<Pins>,
        mut commands: Commands,
    ) {
        if keys.just_pressed(PIN_KEY)
            && let Some(sheet) = Self::open_sheet(&state, &focused, &files)
        {
            if pins.is_full() {
                commands.trigger(Play(Sound::Refuse));
            } else {
                commands.trigger(MoveSheet::Pin(sheet));
            }
        }
    }

    fn show_pin_hint(
        state: Res<State<Focus>>,
        focused: Res<Focused>,
        files: Query<(&Paper, &Filed)>,
        pins: Res<Pins>,
        mut hint: ResMut<KeyHint>,
    ) {
        let action = Self::open_sheet(&state, &focused, &files).map(|_| {
            let id = if pins.is_full() {
                "hint-pin-full"
            } else {
                "hint-pin"
            };
            ("P", Message::new(id))
        });
        hint.set_if_neq(KeyHint(action));
    }

    /// The file lies open while the player reads it and stands closed
    /// otherwise.
    fn show_open(
        time: Res<Time>,
        (state, focused): (Res<State<Focus>>, Res<Focused>),
        mut files: Query<(Entity, &Parts, &Lying, &mut Pose, Ref<Stack>)>,
        mut transforms: Query<&mut Transform>,
        mut visibilities: Query<&mut Visibility>,
        mut surfaces: Query<&mut ScreenSurface>,
        mut meshes: ResMut<Assets<Mesh>>,
    ) {
        for (file, parts, lying, mut pose, stack) in &mut files {
            let open = *state.get() == Focus::Paper && focused.get() == Some(file);
            let moved = pose.bypass_change_detection().step(open, time.delta_secs());
            let leaned = stack.is_changed() && pose.bypass_change_detection().lean_to(stack.lean());
            if moved || leaned || pose.is_added() {
                pose.set_changed();
                parts.pose(
                    &pose,
                    lying.0,
                    &mut transforms,
                    (&mut visibilities, &mut meshes),
                );
            }
            if let Ok(mut surface) = surfaces.get_mut(parts.sheet)
                && surface.active != open
            {
                surface.active = open;
            }
        }
    }
}

/// Where the file lies open, in its own space.
#[derive(Component, Debug)]
struct Lying(Transform);

#[cfg(test)]
mod tests {
    use ld60_calco::Revision;
    use ld60_devices::{BoardRev, DeviceKind};
    use ld60_game_run::levels::Levels;
    use ld60_game_run::salvage::Find;
    use ld60_game_run::tutorial::TutorialNote;
    use ld60_locale::{Locale, Localized, Message};

    use super::contents::{Entry, Section};
    use super::*;

    fn rich_run() -> Run {
        let mut run = Run::default();
        run.add_till(1000);
        assert!(run.buy(Find::Device(DeviceKind::Summa6)));
        assert!(run.buy(Find::Device(DeviceKind::Lock4)));
        assert!(run.buy(Find::Device(DeviceKind::Score5)));
        assert!(run.buy(Find::Errata(Revision::B)));
        run.file_note(1);
        run.file_note(0);
        run
    }

    fn world_with_file() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<Run>();
        world.init_resource::<Pins>();
        world.insert_resource(Layouts::on_disk());
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<Assets<SurfaceMaterial>>();
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
        let contents = Contents::of(&Run::default(), &Layouts::on_disk());
        assert_eq!(
            contents.entries(),
            [Entry::Manual(DeviceKind::Tally2), Entry::Chip(Revision::A)]
        );
    }

    #[test]
    fn sections_come_in_order() {
        let contents = Contents::of(&rich_run(), &Layouts::on_disk());
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
        let contents = Contents::of(&rich_run(), &Layouts::on_disk());
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
            let contents = Contents::of(&run, &Layouts::on_disk());
            for page in contents.pages() {
                locale.assert_formats([page.0.footer()]);
                let in_file = InFile::new(&contents, &Pins::default());
                let links = Links {
                    file: Entity::PLACEHOLDER,
                    in_file: &in_file,
                };
                locale.assert_formats(Localized::spawned_by(|content| {
                    contents.write(&page, &links, content);
                }));
            }
        }
        locale.assert_formats(Section::ALL.map(Section::label));
        locale.assert_formats([Message::new("click-manuals"), Message::new("manuals-spine")]);
        locale.assert_formats(
            [
                "hint-pin",
                "hint-pin-full",
                "hint-pinned",
                "hint-open-index",
                "hint-turn-pages",
            ]
            .map(Message::new),
        );
        locale.assert_formats([Message::new("hint-open").with("target", "2A")]);
        locale.assert_formats(Tab::ALL.map(Tab::hint));
    }

    #[test]
    fn every_page_but_the_index_is_a_sheet() {
        let layouts = Layouts::on_disk();
        let contents = Contents::of(&rich_run(), &layouts);
        for (index, (page, sheet)) in contents.pages().into_iter().enumerate() {
            assert_eq!(contents.sheet_at(index), sheet);
            if let Some(sheet) = sheet {
                assert_eq!(sheet.page(&layouts), Some(page));
            }
        }
        assert_eq!(contents.sheet_at(0), None);
    }

    #[test]
    fn the_pin_key_asks_to_pin_the_open_page() {
        let (mut world, file) = world_with_file();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.insert_resource(State::new(Focus::Paper));
        world.init_resource::<Focused>();
        world.init_resource::<NextState<Focus>>();
        world.add_observer(FocusOn::apply);
        world.add_observer(SheetMoves::wait);
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
        world.flush();
        assert!(world.get::<SheetMoves>(file).unwrap().waiting().is_empty());
        world.get_mut::<Paper>(file).unwrap().open_page(2);
        press(&mut world);
        press(&mut world);
        world.flush();
        let pinned = Sheet::new(Entry::Manual(DeviceKind::Tally2), 1);
        assert_eq!(
            world.get::<SheetMoves>(file).unwrap().waiting(),
            [MoveSheet::Pin(pinned)]
        );
        world.resource_mut::<Pins>().pin(pinned);
        world.run_system_cached(ShopManuals::file).unwrap();
        world.flush();
        let paper = world.get::<Paper>(file).unwrap();
        assert_eq!(paper.current_page(), 2);
        assert_eq!(
            paper.page_count(),
            Contents::of(&Run::default(), &Layouts::on_disk())
                .pages()
                .len()
                - 1
        );
        let open = world.get::<Filed>(file).unwrap().in_file.number(2);
        assert_eq!(open, Some(3));
    }

    #[test]
    fn unpinning_puts_the_page_back_in_its_place() {
        let (mut world, file) = world_with_file();
        let pinned = Sheet::new(Entry::Manual(DeviceKind::Tally2), 1);
        world.resource_mut::<Pins>().pin(pinned);
        world.run_system_cached(ShopManuals::file).unwrap();
        let count = world.get::<Paper>(file).unwrap().page_count();
        world.resource_mut::<Pins>().unpin(pinned);
        world.run_system_cached(ShopManuals::file).unwrap();
        assert_eq!(world.get::<Paper>(file).unwrap().page_count(), count + 1);
        let filed = world.get::<Filed>(file).unwrap();
        assert_eq!(filed.in_file.page(2), Some(2));
    }

    #[test]
    fn a_job_opens_its_board() {
        let contents = Contents::of(&rich_run(), &Layouts::on_disk());
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
        let contents = Contents::of(&Run::default(), &Layouts::on_disk());
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
    fn the_index_and_sections_with_entries_get_a_tab() {
        let (mut world, file) = world_with_file();
        let tabs = |world: &mut World| {
            world
                .query::<&DividerTab>()
                .iter(world)
                .filter(|tab| tab.file == file)
                .count()
        };
        assert_eq!(tabs(&mut world), 3);
        world.insert_resource(rich_run());
        world.run_system_cached(ShopManuals::file).unwrap();
        world.flush();
        assert_eq!(tabs(&mut world), Tab::ALL.len());
    }

    #[test]
    fn the_index_tab_lies_in_front_of_the_first_page() {
        let (world, file) = world_with_file();
        let stack = world.get::<Stack>(file).unwrap();
        assert!(matches!(stack.items()[0], stack::Item::Card(_)));
        assert_eq!(stack.index_of(0), Some(1));
    }
}
