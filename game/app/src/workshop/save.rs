use bevy::prelude::*;
use ld60_game_core::clock::Clock;
use ld60_game_run::Run;
use ld60_game_run::job::{Bench, Job};
use ld60_store::{Slot, Store};
use serde::{Deserialize, Serialize};

use super::manuals::Pins;
use super::salvage::SalvageCrate;

pub(crate) struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Store>();
        if let Some(saved) = SavedShop::load(app.world().resource::<Store>()) {
            let SavedShop {
                mut run,
                clock,
                bench,
                salvage,
                pins,
            } = saved.reopened();
            run.file_past_notes();
            run.file_owned_datasheets();
            let mut pins = pins.unwrap_or_default();
            pins.keep_filed(&run);
            app.insert_resource(run)
                .insert_resource(clock)
                .insert_resource(pins);
            if let Some(bench) = bench {
                app.insert_resource(bench);
            }
            if let Some(salvage) = salvage {
                app.add_systems(Startup, move |mut commands: Commands| {
                    commands.spawn(salvage.clone().on_counter());
                });
            }
        }
        app.add_systems(
            Last,
            SavedShop::write.run_if(
                resource_changed::<Run>
                    .or_else(resource_changed::<Pins>)
                    .or_else(resource_changed::<Clock>)
                    .or_else(resource_exists_and_changed::<Job>),
            ),
        );
    }
}

/// The shop as a save keeps it. The shop is saved each time the run, the
/// time or the job on the bench changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SavedShop {
    run: Run,
    clock: Clock,
    bench: Option<Bench>,
    /// What is left in the scrap trader's crate.
    salvage: Option<SalvageCrate>,
    /// The sheets pinned at the monitor. Saves from before kept them in the
    /// run, see [`PinsInRun`].
    #[serde(default)]
    pins: Option<Pins>,
}

/// The pins of a save from before they were saved apart from the run.
#[derive(Deserialize)]
struct PinsInRun {
    run: RunWithPins,
}

#[derive(Deserialize)]
struct RunWithPins {
    #[serde(default)]
    pins: Pins,
}

impl SavedShop {
    /// The saved shop, with the pins of an older save taken out of its run.
    fn load(store: &Store) -> Option<Self> {
        let saved = store.load::<Self>(Slot::Shop)?;
        if saved.pins.is_some() {
            return Some(saved);
        }
        let pins = store.load::<PinsInRun>(Slot::Shop).map(|old| old.run.pins);
        Some(Self { pins, ..saved })
    }

    /// The shop as it opens again. If the landlord came and the till was
    /// short, the run is over and the next one starts.
    fn reopened(self) -> Self {
        if self.clock.sets() <= self.run.stats().cycles {
            let bench = self.bench.filter(|bench| bench.belongs_to(&self.run));
            return Self { bench, ..self };
        }
        Self {
            run: self.run.next(),
            clock: Clock::default(),
            bench: None,
            salvage: None,
            pins: None,
        }
    }

    /// Saves the shop unless it is the same as the last save.
    fn write(
        store: Res<Store>,
        run: Res<Run>,
        pins: Res<Pins>,
        clock: Res<Clock>,
        job: Option<Res<Job>>,
        crates: Query<&SalvageCrate>,
        mut last: Local<Option<Self>>,
    ) {
        let saved = Self {
            run: run.clone(),
            clock: Clock::from(u32::from(clock.clone())),
            bench: job.map(|job| Bench::from(&*job)),
            salvage: crates.iter().next().cloned(),
            pins: Some(pins.clone()),
        };
        if last.as_ref() == Some(&saved) {
            return;
        }
        store.save(Slot::Shop, &saved);
        *last = Some(saved);
    }
}

#[cfg(test)]
mod tests {
    use ld60_calco::Revision;
    use ld60_devices::DeviceKind;
    use ld60_game_core::player::focus::{Focus, Focused};
    use ld60_game_run::NewRun;
    use ld60_game_run::chore::PlanetSet;
    use ld60_game_run::levels::Levels;
    use ld60_game_run::salvage::Find;

    use super::*;
    use crate::workshop::manuals::Sheet;
    use crate::workshop::rent::RentNote;

    fn world(store: Store) -> World {
        let mut world = World::new();
        world.insert_resource(store);
        world.init_resource::<Run>();
        world.init_resource::<Pins>();
        world.init_resource::<Clock>();
        world
    }

    #[test]
    fn an_older_save_keeps_the_pins_it_kept_in_the_run() {
        let mut world = world(Store::in_temp("pins-in-run"));
        world.resource_mut::<Pins>().pin(Sheet::example());
        let saved = written(&mut world);
        // An older save keeps the pins in the run instead.
        let pins = ron::to_string(&saved.pins).unwrap();
        let text = ron::to_string(&saved)
            .unwrap()
            .replace(&format!(",pins:{pins}"), "")
            .replacen(
                "(run:(",
                &format!(
                    "(run:(pins:{}, ",
                    ron::to_string(&saved.pins.clone().unwrap()).unwrap()
                ),
                1,
            );
        assert!(!text.contains(&format!(",pins:{pins}")), "{text}");
        let older: SavedShop = ron::from_str(&text).unwrap();
        assert_eq!(older.pins, None);
        let kept = ron::from_str::<PinsInRun>(&text).unwrap().run.pins;
        assert!(kept.contains(Sheet::example()));
    }

    fn written(world: &mut World) -> SavedShop {
        world.run_system_cached(SavedShop::write).unwrap();
        world.resource::<Store>().load(Slot::Shop).unwrap()
    }

    #[test]
    fn the_shop_loads_as_it_was_saved() {
        let mut world = world(Store::in_temp("shop-loads"));
        let mut run = Run::default().next();
        run.add_till(40);
        assert!(run.buy(Find::Device(DeviceKind::Summa6)));
        world.insert_resource(run.clone());
        world.insert_resource(Clock::from(23));
        let salvage = SalvageCrate::with_finds(vec![Find::Errata(Revision::B)]);
        world.spawn(salvage.clone());
        let saved = written(&mut world);
        assert_eq!(
            saved,
            SavedShop {
                run,
                clock: Clock::from(23),
                bench: None,
                salvage: Some(salvage),
                pins: Some(Pins::default()),
            }
        );
        assert_eq!(saved.clone().reopened(), saved);
    }

    #[test]
    fn the_job_on_the_bench_comes_back_as_it_was() {
        let levels = Levels::on_disk();
        let mut world = world(Store::in_temp("bench"));
        let mut run = Run::default().next();
        let mut job = Job::for_run(&run, &levels).unwrap();
        job.returns = 2;
        world.insert_resource(job);
        run.add_till(40);
        assert!(run.buy(Find::Device(DeviceKind::Summa6)));
        world.insert_resource(run);
        let bench = written(&mut world).reopened().bench.unwrap();
        let reopened = bench.reopen(&levels).unwrap();
        let job = world.resource::<Job>();
        assert_eq!(reopened.origin, job.origin);
        assert_eq!(reopened.level.reward, job.level.reward);
        assert_eq!(reopened.buggy, job.buggy);
        assert_eq!(reopened.returns, 2);
    }

    #[test]
    fn a_finished_job_is_not_reopened() {
        let levels = Levels::on_disk();
        let mut world = world(Store::in_temp("finished-job"));
        let run = Run::default().next();
        world.insert_resource(Job::for_run(&run, &levels).unwrap());
        let mut saved = written(&mut world);
        saved.run = run.next();
        assert_eq!(saved.reopened().bench, None);
    }

    #[test]
    fn an_unpaid_rent_closes_the_saved_run() {
        let mut world = world(Store::in_temp("unpaid-rent"));
        world.init_resource::<Focused>();
        world.init_resource::<NextState<Focus>>();
        world.add_observer(RentNote::collect);
        world.insert_resource(Clock::from(40));
        world.trigger(PlanetSet);
        world.flush();
        let saved = written(&mut world);
        let reopened = saved.clone().reopened();
        assert_eq!(reopened.run, saved.run.next());
        assert_eq!(reopened.clock, Clock::default());
    }

    #[test]
    fn a_new_run_replaces_the_save() {
        let mut world = world(Store::in_temp("new-run"));
        world.init_resource::<Focused>();
        world.init_resource::<NextState<Focus>>();
        world.add_observer(Run::restart);
        let mut run = Run::default().next();
        run.add_till(40);
        world.insert_resource(run.clone());
        world.insert_resource(Clock::from(23));
        written(&mut world);
        world.trigger(NewRun::Reopen);
        world.flush();
        let saved = written(&mut world);
        assert_eq!(saved.run, run.next());
        assert_eq!(saved.clock, Clock::default());
        assert_eq!(saved.clone().reopened(), saved);
    }

    #[test]
    fn a_paid_rent_keeps_the_run() {
        let mut world = world(Store::in_temp("paid-rent"));
        world.init_resource::<Focused>();
        world.init_resource::<NextState<Focus>>();
        world.add_observer(RentNote::collect);
        world.resource_mut::<Run>().add_till(100);
        world.insert_resource(Clock::from(40));
        world.trigger(PlanetSet);
        world.flush();
        let saved = written(&mut world);
        assert_eq!(saved.run.stats().cycles, 1);
        assert_eq!(saved.clone().reopened(), saved);
    }
}
