mod demand;
mod per_device;
mod served;
mod tools;

use bevy::prelude::*;
use ld60_calco::{Errata, Revision};
use ld60_core::clock::Clock;
use ld60_core::player::focus::{Focus, Focused};
use ld60_devices::{BoardRev, ChipSpec, DeviceKind};
use ld60_levels::{Catalog, GAME_SEED, LevelSource, Reference};
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

pub(crate) use self::demand::Demand;
use self::per_device::PerDevice;
use self::served::Served;
pub(crate) use self::tools::ComputerTool;
use super::job::{Job, JobItem};
use super::manuals::Pins;
use super::pickup::Pickup;
use super::receipt::Receipt;
use super::regulars::Regulars;
use super::rent::RentNote;
use super::salvage::{Find, SalvageCrate};
use super::tutorial::TutorialNote;

/// What the landlord takes the first time the planet sets.
const RENT: u32 = 6;
/// How much more the landlord takes each time after that.
const RENT_RISE: u32 = 6;
/// The landlord offers the building from this many paid rents on.
const DEED_FROM: u32 = 3;
/// A special job that can come takes one in this many jobs.
const SPECIAL_ODDS: u32 = 2;
/// Keeps the seeds of one run away from those of the next.
const RUN_SEEDS: u64 = 1 << 32;
/// The same board comes at most this many jobs in a row.
const SAME_BOARDS: usize = 2;

/// What is left on the counter when the shop closes.
type Leftovers = Or<(
    With<JobItem>,
    With<Receipt>,
    With<RentNote>,
    With<SalvageCrate>,
)>;

pub(crate) struct RunPlugin;

impl Plugin for RunPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Run>().add_observer(Run::restart);
    }
}

/// One run of the shop, from opening until the landlord closes it.
#[derive(Resource, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Run {
    /// Counts the runs since the game started, from 0.
    number: u32,
    /// The current job, counted from 0.
    job: u32,
    /// Whether the run starts with the tutorial.
    tutorial: bool,
    /// Money in the till.
    till: u32,
    /// The devices with a sign in the shop window. Locals only bring these.
    /// Their manuals are in the manuals file.
    signs: Vec<DeviceKind>,
    /// The chip revisions with a datasheet in the manuals file.
    #[serde(default)]
    datasheets: Vec<Revision>,
    /// The chip revisions with errata pages in the manuals file.
    errata: Vec<Revision>,
    /// The tutorial steps whose notes are filed in the manuals file.
    #[serde(default)]
    notes: Vec<usize>,
    /// The sheets of the manuals file pinned to the monitor.
    #[serde(default)]
    pins: Pins,
    tools: Vec<ComputerTool>,
    demand: Demand,
    served: Served,
    /// The level folders of the special jobs done in this run.
    specials: Vec<String>,
    /// The boards of the last jobs, newest last.
    #[serde(default)]
    boards: Vec<BoardRev>,
    /// How far the stories of the regulars got.
    #[serde(default)]
    regulars: Regulars,
    /// Whether the player bought the building, so no rent is due.
    #[serde(default)]
    owned: bool,
    stats: Stats,
}

/// What the shop did in a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Stats {
    /// Jobs the customer paid for.
    pub(crate) fixed: u32,
    /// Jobs the customer took home broken.
    pub(crate) lost: u32,
    /// All money taken in.
    pub(crate) earned: u32,
    /// All rent paid.
    pub(crate) rent: u32,
    /// All money paid to the scrap trader.
    pub(crate) spent: u32,
    /// How often the planet set with the rent paid.
    pub(crate) cycles: u32,
}

/// Where a job comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    /// Step `step` of the tutorial, counted from 0.
    Lesson { step: usize, source: LevelSource },
    /// A hand-written job between the generated ones.
    Special(LevelSource),
    /// Generated job `number` on `device`, counted from 0 for each device
    /// after the tutorial.
    Generated { device: DeviceKind, number: u32 },
}

/// The landlord's visit when the planet sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Rent {
    /// The landlord took `amount` from the till and comes for `next` the
    /// next time. He sells the building for `deed`, if he offers it.
    Paid {
        amount: u32,
        next: u32,
        deed: Option<u32>,
    },
    /// The till held only `till` of `amount`, so the shop closes.
    Short { amount: u32, till: u32 },
    /// The player owns the building, so the landlord takes nothing.
    Owned,
}

/// Starts a new run.
#[derive(Event, Debug)]
pub(crate) enum NewRun {
    /// After the shop closed. The player knows the shop by then.
    Reopen,
    /// Asked for in the menu. Starts over with the tutorial.
    StartOver,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            number: 0,
            job: 0,
            tutorial: true,
            till: 0,
            signs: vec![DeviceKind::Tally2],
            datasheets: vec![Self::shipped_chip(DeviceKind::Tally2)],
            errata: Vec::new(),
            notes: Vec::new(),
            pins: Pins::default(),
            tools: Vec::new(),
            demand: Demand::default(),
            served: Served::default(),
            specials: Vec::new(),
            boards: Vec::new(),
            regulars: Regulars::default(),
            owned: false,
            stats: Stats::default(),
        }
    }
}

impl Run {
    /// The run after this one. The player knows the shop by then, so it
    /// skips the tutorial. The notes stay filed and the pinned sheets go
    /// back into the file.
    pub(crate) fn next(&self) -> Self {
        Self {
            number: self.number + 1,
            tutorial: false,
            notes: self.notes.clone(),
            ..Self::default()
        }
    }

    /// A run from the start, with the tutorial and an empty manuals file.
    pub(crate) fn start_over(&self) -> Self {
        Self {
            number: self.number + 1,
            ..Self::default()
        }
    }

    /// Whether the shop has served anyone yet, so the title screen offers
    /// to continue.
    pub(crate) fn underway(&self) -> bool {
        self.job > 0 || self.number > 0
    }

    pub(crate) fn till(&self) -> u32 {
        self.till
    }

    pub(crate) fn stats(&self) -> Stats {
        self.stats
    }

    pub(crate) fn regulars(&self) -> &Regulars {
        &self.regulars
    }

    /// The current job, counted from 0.
    pub(crate) fn job(&self) -> u32 {
        self.job
    }

    pub(crate) fn demand(&self) -> &Demand {
        &self.demand
    }

    /// Counts the runs since the game started, from 0.
    pub(crate) fn number(&self) -> u32 {
        self.number
    }

    pub(crate) fn signs(&self) -> &[DeviceKind] {
        &self.signs
    }

    pub(crate) fn errata(&self) -> &[Revision] {
        &self.errata
    }

    /// The tutorial steps whose notes are filed, in the order they were
    /// filed.
    pub(crate) fn notes(&self) -> &[usize] {
        &self.notes
    }

    /// Files the note of tutorial step `step` in the manuals file.
    pub(crate) fn file_note(&mut self, step: usize) {
        if !self.notes.contains(&step) {
            self.notes.push(step);
        }
    }

    /// Files the notes of the tutorial steps the shop is past. Saves from
    /// before the manuals file kept no notes.
    pub(crate) fn file_past_notes(&mut self) {
        let past = if self.tutorial {
            self.job as usize
        } else {
            usize::MAX
        };
        for note in TutorialNote::before(past) {
            self.file_note(note.0);
        }
    }

    /// Files the datasheets that come with the signs and the errata pages.
    /// Saves from before datasheets were sold kept none.
    pub(crate) fn file_owned_datasheets(&mut self) {
        let owned = self
            .signs
            .iter()
            .map(|&device| Self::shipped_chip(device))
            .chain(self.errata.iter().copied());
        for revision in owned.collect::<Vec<_>>() {
            self.file_datasheet(revision);
        }
    }

    /// Unpins the sheets that are not in the manuals file. Saves from before
    /// datasheets were sold can pin datasheets the shop no longer owns.
    pub(crate) fn unpin_unfiled(&mut self) {
        let mut pins = std::mem::take(&mut self.pins);
        pins.keep_filed(self);
        self.pins = pins;
    }

    fn file_datasheet(&mut self, revision: Revision) {
        if !self.datasheets.contains(&revision) {
            self.datasheets.push(revision);
        }
    }

    /// The chip revision whose datasheet comes with the manual of `device`.
    /// It is the oldest one the firmware of `device` runs on.
    pub(crate) fn shipped_chip(device: DeviceKind) -> Revision {
        Reference::revisions(device)[0]
    }

    pub(crate) fn pins(&self) -> &Pins {
        &self.pins
    }

    pub(crate) fn pins_mut(&mut self) -> &mut Pins {
        &mut self.pins
    }

    /// Whether the manuals file has the errata pages for every known bug of
    /// `chip`.
    pub(crate) fn explains(&self, chip: ChipSpec) -> bool {
        let ChipSpec::Calco(chip) = chip;
        Errata::for_chip(chip)
            .iter()
            .all(|erratum| self.errata.contains(&erratum.revision()))
    }

    /// The chip revisions with a datasheet in the manuals file, in the order
    /// of [`Revision::ALL`].
    pub(crate) fn chips(&self) -> Vec<Revision> {
        Revision::ALL
            .into_iter()
            .filter(|revision| self.datasheets.contains(revision))
            .collect()
    }

    /// Whether the shop computer has `tool`.
    pub(crate) fn has(&self, tool: ComputerTool) -> bool {
        self.tools.contains(&tool)
    }

    pub(crate) fn owns(&self, find: Find) -> bool {
        match find {
            Find::Device(device) => self.signs.contains(&device),
            Find::Tool(tool) => self.has(tool),
            Find::Errata(revision) => self.errata.contains(&revision),
            Find::Datasheet(revision) => self.datasheets.contains(&revision),
            Find::Deed => self.owned,
        }
    }

    /// Whether the landlord sells the building.
    fn sells_deed(&self) -> bool {
        self.stats.cycles >= DEED_FROM
    }

    /// Whether the scrap trader can offer `find`. The shop must not own it
    /// yet and must be able to use it.
    pub(crate) fn wants(&self, find: Find) -> bool {
        let usable = match find {
            Find::Device(_) | Find::Datasheet(_) => true,
            Find::Tool(tool) => tool
                .needs()
                .is_none_or(|needed| self.tools.contains(&needed)),
            Find::Errata(revision) => self.datasheets.contains(&revision),
            Find::Deed => self.sells_deed(),
        };
        usable && !self.owns(find)
    }

    /// Pays for `find` from the till and puts it in the shop. Returns
    /// whether the till held enough.
    pub(crate) fn buy(&mut self, find: Find) -> bool {
        let price = find.price();
        if self.till < price || self.owns(find) {
            return false;
        }
        self.till -= price;
        self.stats.spent += price;
        match find {
            Find::Device(device) => {
                self.signs.push(device);
                self.file_datasheet(Self::shipped_chip(device));
            }
            Find::Tool(tool) => self.tools.push(tool),
            Find::Errata(revision) => self.errata.push(revision),
            Find::Datasheet(revision) => self.datasheets.push(revision),
            Find::Deed => self.owned = true,
        }
        true
    }

    /// The seed of the current job.
    pub(crate) fn seed(&self) -> u64 {
        GAME_SEED + RUN_SEEDS * u64::from(self.number) + u64::from(self.job)
    }

    /// Where the current job comes from.
    ///
    /// The tutorial comes first. After it, a special job can come once its
    /// sign hangs in the window, the shop took [`Special::from`] jobs on its
    /// device and the manuals file explains the bugs of its chip. Otherwise the
    /// job is generated for one of the signs, picked by [`Demand`].
    ///
    /// [`Special::from`]: ld60_levels::Special::from
    pub(crate) fn pick(&self, catalog: &Catalog) -> Origin {
        let tutorial = if self.tutorial {
            catalog.tutorial()
        } else {
            &[]
        };
        let job = self.job as usize;
        if let Some(source) = tutorial.get(job) {
            return Origin::Lesson {
                step: job,
                source: source.clone(),
            };
        }
        let mut rng = SmallRng::seed_from_u64(self.seed());
        let special = catalog.specials().find(|(source, from)| {
            !self.specials.contains(&source.path)
                && source.load().is_ok_and(|level| {
                    self.signs.contains(&level.device)
                        && self.served.jobs(level.device) >= *from
                        && self.explains(level.chip)
                })
        });
        // The intro jobs of a device, from its first job on, come first and
        // always. Later special jobs come now and then.
        if let Some((source, from)) = special
            && (from == 0 || rng.random_ratio(1, SPECIAL_ODDS))
        {
            return Origin::Special(source.clone());
        }
        let device = self
            .demand
            .pick(&self.signs, &mut rng)
            .unwrap_or(DeviceKind::Tally2);
        Origin::Generated {
            device,
            number: self.served.jobs(device),
        }
    }

    /// Takes in what the customer paid for `job`. Once the device leaves
    /// the shop, the run moves on to the next job.
    pub(crate) fn settle(&mut self, job: &Job, pickup: &Pickup) {
        if let Pickup::Paid(payment) = pickup {
            self.till += payment.amount();
            self.stats.earned += payment.amount();
        }
        if !pickup.ends_job() {
            return;
        }
        if let Some(visit) = job.visit {
            self.regulars.record(visit, self.job);
        }
        self.job += 1;
        self.boards.push(job.level.board);
        if self.boards.len() > SAME_BOARDS {
            self.boards.remove(0);
        }
        match job.origin {
            Origin::Lesson { step, .. } => self.file_note(step),
            _ => self.served.add(job.level.device),
        }
        match pickup {
            Pickup::Paid(_) => {
                self.stats.fixed += 1;
                if !matches!(job.origin, Origin::Lesson { .. }) {
                    self.demand.fixed(job.level.device);
                }
            }
            Pickup::HandedBack { .. } | Pickup::Left(_) => self.stats.lost += 1,
        }
        if let Origin::Special(source) = &job.origin {
            self.specials.push(source.path.clone());
        }
    }

    /// The board the next job should not be on, because the last jobs all
    /// were.
    pub(crate) fn tired_board(&self) -> Option<BoardRev> {
        match self.boards.as_slice() {
            [first, rest @ ..]
                if rest.len() + 1 == SAME_BOARDS && rest.iter().all(|b| b == first) =>
            {
                Some(*first)
            }
            _ => None,
        }
    }

    /// Whether the player bought the building, so no rent is due.
    pub(crate) fn owned(&self) -> bool {
        self.owned
    }

    /// What the landlord takes the next time the planet sets.
    pub(crate) fn rent(&self) -> u32 {
        RENT + RENT_RISE * self.stats.cycles
    }

    /// The landlord takes the rent from the till, if it holds enough.
    pub(crate) fn pay_rent(&mut self) -> Rent {
        if self.owned {
            self.stats.cycles += 1;
            self.demand.recover();
            return Rent::Owned;
        }
        let amount = self.rent();
        if self.till < amount {
            return Rent::Short {
                amount,
                till: self.till,
            };
        }
        self.till -= amount;
        self.stats.rent += amount;
        self.stats.cycles += 1;
        self.demand.recover();
        Rent::Paid {
            amount,
            next: self.rent(),
            deed: self.sells_deed().then_some(Find::Deed.price()),
        }
    }

    /// Clears the counter and opens the shop again with an empty till.
    pub(super) fn restart(
        new_run: On<NewRun>,
        mut run: ResMut<Run>,
        mut clock: ResMut<Clock>,
        mut focused: ResMut<Focused>,
        mut focus: ResMut<NextState<Focus>>,
        leftovers: Query<Entity, Leftovers>,
        mut commands: Commands,
    ) {
        *run = match *new_run {
            NewRun::Reopen => run.next(),
            NewRun::StartOver => run.start_over(),
        };
        *clock = Clock::default();
        focused.leave(&mut focus);
        for entity in &leftovers {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<Job>();
        commands.run_system_cached(Job::start);
    }
}

#[cfg(test)]
impl Run {
    pub(crate) fn add_till(&mut self, amount: u32) {
        self.till += amount;
    }

    /// A run whose landlord sells the building.
    pub(crate) fn offer_deed(&mut self) {
        self.stats.cycles = DEED_FROM;
    }

    /// A run whose shop computer has every tool.
    pub(crate) fn with_all_tools() -> Self {
        Self {
            tools: ComputerTool::ALL.to_vec(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_core::player::focus::{FocusOn, FocusTarget, View};
    use ld60_levels::Difficulty;

    use super::*;
    use crate::workshop::levels::Levels;
    use crate::workshop::manuals::Sheet;
    use crate::workshop::pickup::{Complaint, Payment};
    use crate::workshop::regulars::Regular;
    use crate::workshop::salvage::Find;

    fn job(run: &Run) -> Job {
        Job::new(run.pick(&Levels::on_disk()), run, &Levels::on_disk()).unwrap()
    }

    #[test]
    fn the_tutorial_comes_first() {
        let levels = Levels::on_disk();
        let mut run = Run::default();
        for (step, source) in levels.tutorial().iter().enumerate() {
            assert_eq!(
                run.pick(&levels),
                Origin::Lesson {
                    step,
                    source: source.clone()
                }
            );
            run.job += 1;
        }
        assert_eq!(
            run.pick(&levels),
            Origin::Generated {
                device: DeviceKind::Tally2,
                number: 0
            }
        );
    }

    #[test]
    fn tutorial_notes_stay_filed() {
        let mut run = Run::default();
        run.settle(&job(&run), &Pickup::Paid(Payment::example(4)));
        assert_eq!(run.notes(), [0]);
        assert_eq!(run.next().notes(), [0]);
    }

    #[test]
    fn old_saves_file_the_notes_of_past_steps() {
        let mut run = Run::default();
        run.file_past_notes();
        assert!(run.notes().is_empty());
        run.job = 1;
        run.file_past_notes();
        assert_eq!(run.notes(), [0]);
        let mut later = Run::default().next();
        later.file_past_notes();
        assert_eq!(later.notes(), (0..7).collect::<Vec<_>>());
    }

    #[test]
    fn pinned_sheets_go_back_for_the_next_run() {
        let mut run = Run::default();
        run.pins_mut().pin(Sheet::example());
        assert!(!run.pins().is_empty());
        assert!(run.next().pins().is_empty());
    }

    #[test]
    fn pins_are_saved_with_the_run() {
        let mut run = Run::default();
        run.pins_mut().pin(Sheet::example());
        let text = ron::to_string(&run).unwrap();
        assert_eq!(ron::from_str::<Run>(&text).unwrap(), run);
    }

    #[test]
    fn a_run_is_underway_after_the_first_job() {
        let mut run = Run::default();
        assert!(!run.underway());
        run.settle(&job(&run), &Pickup::Paid(Payment::example(4)));
        assert!(run.underway());
        assert!(Run::default().next().underway());
    }

    #[test]
    fn a_new_run_skips_the_tutorial() {
        let run = Run {
            job: 5,
            till: 30,
            ..Run::default()
        };
        let next = run.next();
        assert_eq!((next.job, next.till), (0, 0));
        assert_ne!(next.seed(), Run::default().seed());
        assert_eq!(
            next.pick(&Levels::on_disk()),
            Origin::Generated {
                device: DeviceKind::Tally2,
                number: 0
            }
        );
    }

    #[test]
    fn specials_need_their_sign() {
        let levels = Levels::on_disk();
        let mut run = Run::default().next();
        let picks = |run: &mut Run| {
            (0..40)
                .map(|job| {
                    run.job = job;
                    run.served = Served::default();
                    for _ in 0..job {
                        run.served.add(DeviceKind::Summa6);
                    }
                    run.pick(&levels)
                })
                .collect::<Vec<_>>()
        };
        assert!(
            picks(&mut run)
                .iter()
                .all(|origin| matches!(origin, Origin::Generated { .. }))
        );
        run.signs.push(DeviceKind::Summa6);
        let specials: Vec<_> = picks(&mut run)
            .into_iter()
            .enumerate()
            .filter_map(|(job, origin)| match origin {
                Origin::Special(source) => Some((job as u32, source.path)),
                _ => None,
            })
            .collect();
        assert!(!specials.is_empty());
        for (job, path) in specials {
            let (_, from) = levels
                .specials()
                .find(|(source, _)| source.path == path)
                .unwrap();
            assert!(job >= from, "{path} at job {job}");
        }
    }

    #[test]
    fn a_special_comes_once() {
        let levels = Levels::on_disk();
        let mut run = Run::default().next();
        run.signs = DeviceKind::SERVICED.to_vec();
        run.errata = vec![Revision::B];
        for device in DeviceKind::SERVICED {
            for _ in 0..100 {
                run.served.add(device);
            }
        }
        let mut seen = Vec::new();
        while run.job < 200 && seen.len() < levels.specials().count() {
            let Origin::Special(source) = run.pick(&levels) else {
                run.job += 1;
                continue;
            };
            assert!(!seen.contains(&source.path), "{}", source.path);
            seen.push(source.path.clone());
            run.settle(&job(&run), &Pickup::Paid(Payment::example(1)));
        }
        assert_eq!(seen.len(), levels.specials().count());
    }

    #[test]
    fn errata_jobs_need_the_errata_pages() {
        const ERRATA: &str = "summa6/05-errata";
        let levels = Levels::on_disk();
        let mut run = Run::default().next();
        run.signs.push(DeviceKind::Summa6);
        for _ in 0..100 {
            run.served.add(DeviceKind::Summa6);
        }
        run.specials = levels
            .specials()
            .map(|(source, _)| source.path.clone())
            .take_while(|path| path != ERRATA)
            .collect();
        let dealt = |run: &mut Run| {
            (0..200).any(|job| {
                run.job = job;
                matches!(run.pick(&levels), Origin::Special(source) if source.path == ERRATA)
            })
        };
        assert!(!dealt(&mut run));
        run.errata.push(Revision::B);
        assert!(dealt(&mut run));
    }

    #[test]
    fn generated_jobs_skip_chips_the_manuals_cannot_explain() {
        let run = Run::default().next();
        let chips =
            |difficulty: fn(u32) -> Difficulty| (8..40).map(move |job| difficulty(job).chip());
        assert!(
            chips(|job| Difficulty::for_job(job, DeviceKind::Summa6))
                .any(|chip| !run.explains(chip))
        );
        assert!(
            chips(|job| Difficulty::for_shop(job, DeviceKind::Summa6, &Revision::ALL, &[]))
                .all(|chip| run.explains(chip))
        );
    }

    #[test]
    fn devices_bring_the_datasheet_of_their_chip() {
        let mut run = Run::default().next();
        assert_eq!(run.chips(), [Revision::A]);
        run.add_till(100);
        assert!(run.buy(Find::Device(DeviceKind::Lock4)));
        assert_eq!(run.chips(), [Revision::A, Revision::C]);
        assert!(run.buy(Find::Device(DeviceKind::Summa6)));
        assert_eq!(run.chips(), [Revision::A, Revision::B, Revision::C]);
        assert!(!run.wants(Find::Datasheet(Revision::B)));
    }

    #[test]
    fn a_loose_datasheet_is_sold_once() {
        let mut run = Run::default().next();
        let datasheet = Find::Datasheet(Revision::C);
        assert!(run.wants(datasheet));
        run.add_till(100);
        assert!(run.buy(datasheet));
        assert_eq!(run.chips(), [Revision::A, Revision::C]);
        assert!(!run.wants(datasheet));
        assert!(!run.buy(datasheet));
    }

    #[test]
    fn errata_pages_need_their_datasheet() {
        let mut run = Run::default().next();
        let errata = Find::Errata(Revision::B);
        assert!(!run.wants(errata));
        run.add_till(100);
        assert!(run.buy(Find::Datasheet(Revision::B)));
        assert!(run.wants(errata));
    }

    #[test]
    fn generated_jobs_run_on_chips_with_a_datasheet() {
        let levels = Levels::on_disk();
        let mut run = Run::default().next();
        for number in 0..20 {
            run.job = number;
            let job = Job::new(
                Origin::Generated {
                    device: DeviceKind::Tally2,
                    number,
                },
                &run,
                &levels,
            )
            .unwrap();
            let ChipSpec::Calco(chip) = job.level.chip;
            assert_eq!(chip.revision, Revision::A);
        }
    }

    #[test]
    fn old_saves_file_the_datasheets_they_own() {
        let text = ron::to_string(&Run {
            signs: vec![DeviceKind::Tally2, DeviceKind::Summa6],
            errata: vec![Revision::B],
            ..Run::default().next()
        })
        .unwrap();
        let old = text.replace("datasheets:[A],", "");
        assert_ne!(old, text);
        let mut run: Run = ron::from_str(&old).unwrap();
        assert!(run.chips().is_empty());
        run.file_owned_datasheets();
        assert_eq!(run.chips(), [Revision::A, Revision::B]);
    }

    #[test]
    fn a_new_sign_starts_with_its_intro_jobs() {
        let mut run = Run::default().next();
        for _ in 0..30 {
            run.served.add(DeviceKind::Tally2);
        }
        run.signs = vec![DeviceKind::Summa6];
        let Origin::Special(source) = run.pick(&Levels::on_disk()) else {
            panic!("no intro job");
        };
        assert_eq!(source.path, "summa6/i1-wakes-in-squiggles");
        run.specials.push(source.path);
        let Origin::Special(source) = run.pick(&Levels::on_disk()) else {
            panic!("no second intro job");
        };
        assert_eq!(source.path, "summa6/i2-right-digit-squiggle");
    }

    #[test]
    fn settling_counts_the_job() {
        let mut run = Run::default().next();
        let job = job(&run);
        let handed_back = Pickup::HandedBack {
            complaint: Complaint::Dead,
            tries_left: 2,
        };
        run.settle(&job, &handed_back);
        assert_eq!((run.job, run.till), (0, 0));
        run.settle(&job, &Pickup::Paid(Payment::example(4)));
        assert_eq!((run.job, run.till), (1, 4));
        run.settle(&job, &Pickup::Left(Complaint::Dead));
        assert_eq!(
            run.stats,
            Stats {
                fixed: 1,
                lost: 1,
                earned: 4,
                ..Stats::default()
            }
        );
        assert!(run.demand.percent(DeviceKind::Tally2) < 100);
        assert_eq!(run.served.jobs(DeviceKind::Tally2), 2);
    }

    #[test]
    fn the_tutorial_leaves_demand_alone() {
        let mut run = Run::default();
        run.settle(&job(&run), &Pickup::Paid(Payment::example(4)));
        assert_eq!((run.job, run.stats.fixed), (1, 1));
        assert_eq!(run.demand, Demand::default());
        assert_eq!(run.served, Served::default());
    }

    #[test]
    fn a_new_run_leaves_the_focus() {
        let mut world = World::new();
        world.init_resource::<Run>();
        world.init_resource::<Clock>();
        world.init_resource::<Focused>();
        world.init_resource::<NextState<Focus>>();
        world.add_observer(Run::restart);
        world.add_observer(FocusOn::apply);
        let note = world
            .spawn(FocusTarget {
                prompt: "click-rent",
                focus: Focus::Paper,
                view: View::World(Transform::IDENTITY),
            })
            .id();
        world.trigger(FocusOn { entity: note });
        assert_eq!(world.resource::<Focused>().get(), Some(note));
        world.trigger(NewRun::Reopen);
        world.flush();
        assert_eq!(world.resource::<Focused>().get(), None);
        assert!(matches!(
            *world.resource::<NextState<Focus>>(),
            NextState::Pending(Focus::Desk)
        ));
    }

    #[test]
    fn regulars_come_back_until_their_story_is_told() {
        let levels = Levels::on_disk();
        let mut run = Run::default().next();
        let mut visits = Vec::new();
        for _ in 0..60 {
            let job = Job::for_run(&run, &levels).unwrap();
            if let Some(visit) = job.visit {
                let customer = job.level.text.as_ref().map(|text| text.customer.as_str());
                assert_eq!(customer, Some(visit.regular.id()));
                visits.push(visit);
            }
            run.settle(&job, &Pickup::Paid(Payment::example(job.level.reward)));
        }
        let last = visits.iter().filter(|visit| visit.gift() > 0).count();
        assert!(last >= 2, "{visits:?}");
        assert!(visits.iter().all(|visit| visit.number <= Regular::VISITS));
    }

    #[test]
    fn a_board_comes_at_most_twice_in_a_row() {
        let levels = Levels::on_disk();
        let mut run = Run::default().next();
        let mut boards = Vec::new();
        for _ in 0..16 {
            let job = Job::for_run(&run, &levels).unwrap();
            boards.push(job.level.board);
            run.settle(&job, &Pickup::Paid(Payment::example(job.level.reward)));
        }
        assert!(
            boards
                .windows(SAME_BOARDS + 1)
                .all(|w| w.iter().any(|b| *b != w[0])),
            "{boards:?}"
        );
    }

    #[test]
    fn starting_over_brings_back_the_tutorial() {
        let mut run = Run::default().next();
        run.notes.push(0);
        let over = run.start_over();
        assert!(over.tutorial);
        assert!(over.notes.is_empty());
        assert_eq!(over.number, run.number + 1);
        assert!(matches!(
            over.pick(&Levels::on_disk()),
            Origin::Lesson { step: 0, .. }
        ));
    }

    #[test]
    fn worn_demand_lowers_the_reward() {
        let levels = Levels::on_disk();
        let run = Run::default().next();
        let mut worn = run.clone();
        for _ in 0..10 {
            worn.demand.fixed(DeviceKind::Tally2);
        }
        let origin = run.pick(&levels);
        let fresh = Job::new(origin.clone(), &run, &levels).unwrap();
        let tired = Job::new(origin, &worn, &levels).unwrap();
        assert!(tired.level.reward < fresh.level.reward);
    }

    #[test]
    fn tutorial_jobs_pay_in_full() {
        let levels = Levels::on_disk();
        let mut run = Run::default();
        for _ in 0..10 {
            run.demand.fixed(DeviceKind::Tally2);
        }
        let job = job(&run);
        let full = levels.tutorial()[0].load().unwrap().reward;
        assert_eq!(job.level.reward, full);
    }

    #[test]
    fn rent_grows_each_cycle() {
        let mut run = Run {
            till: 100,
            ..Run::default()
        };
        assert_eq!(
            run.pay_rent(),
            Rent::Paid {
                amount: RENT,
                next: RENT + RENT_RISE,
                deed: None,
            }
        );
        assert_eq!(
            run.pay_rent(),
            Rent::Paid {
                amount: RENT + RENT_RISE,
                next: RENT + 2 * RENT_RISE,
                deed: None,
            }
        );
        assert_eq!(run.till, 100 - 2 * RENT - RENT_RISE);
        assert_eq!(
            (run.stats.cycles, run.stats.rent),
            (2, 2 * RENT + RENT_RISE)
        );
    }

    #[test]
    fn the_landlord_sells_the_building_and_then_takes_no_rent() {
        let mut run = Run {
            till: 1000,
            ..Run::default()
        };
        assert!(!run.wants(Find::Deed));
        for _ in 0..DEED_FROM {
            run.pay_rent();
        }
        assert!(matches!(
            run.pay_rent(),
            Rent::Paid {
                deed: Some(price),
                ..
            } if price == Find::Deed.price()
        ));
        assert!(run.wants(Find::Deed));
        assert!(run.buy(Find::Deed));
        assert!(run.owned() && !run.wants(Find::Deed));
        let (till, cycles) = (run.till, run.stats.cycles);
        assert_eq!(run.pay_rent(), Rent::Owned);
        assert_eq!((run.till, run.stats.cycles), (till, cycles + 1));
    }

    #[test]
    fn short_rent_leaves_the_till() {
        let mut run = Run {
            till: RENT - 1,
            ..Run::default()
        };
        assert_eq!(
            run.pay_rent(),
            Rent::Short {
                amount: RENT,
                till: RENT - 1
            }
        );
        assert_eq!((run.till, run.stats.cycles), (RENT - 1, 0));
    }
}
