use std::f32::consts::PI;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::job::Job;
use super::run::Origin;

/// Ticks from planet rise to planet set.
const CYCLE: u32 = 40;
/// Ticks into the cycle when a run starts, so the shop opens with the planet
/// well up.
const OPENING: u32 = CYCLE / 4;
/// How far the planet swings to each side of straight across the street, in
/// radians.
const SWING: f32 = 1.0;
/// The planet's height above the horizon at rise and set, in radians.
const LOW: f32 = -0.1;
/// The planet's height above the horizon at its highest, in radians.
const HIGH: f32 = 0.65;
/// How quickly the planet glides to the time on the clock.
const GLIDE_RATE: f32 = 2.0;
/// Closer than this, the planet is on time, in ticks.
const ON_TIME: f32 = 0.001;

pub(crate) struct ClockPlugin;

impl Plugin for ClockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Clock>()
            .add_systems(Update, Clock::glide)
            .add_observer(Clock::pass);
    }
}

/// The time on the planet, which only moves on when the shop works.
///
/// The ringed planet crosses the sky once in each [`CYCLE`]. The landlord
/// comes for the rent each time it sets.
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "u32", into = "u32")]
pub(crate) struct Clock {
    /// Ticks since the planet first rose.
    ticks: u32,
    /// Where the planet is shown, in ticks. It glides after `ticks`.
    shown: f32,
}

/// The shop spends time on `0`. Tutorial jobs take no time.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Busy(pub(crate) Chore);

/// Work that takes time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Chore {
    /// Burning firmware into a device.
    Flash,
    /// A customer coming to try a device and take it home.
    HandIn,
    /// A customer coming to try a device and handing it back.
    Return,
}

/// The planet went down, so the rent is due.
#[derive(Event, Debug)]
pub(crate) struct PlanetSet;

impl Default for Clock {
    fn default() -> Self {
        Self {
            ticks: OPENING,
            shown: OPENING as f32,
        }
    }
}

impl From<u32> for Clock {
    /// The clock at `ticks`, with the planet shown there.
    fn from(ticks: u32) -> Self {
        Self {
            ticks,
            shown: ticks as f32,
        }
    }
}

impl From<Clock> for u32 {
    fn from(clock: Clock) -> Self {
        clock.ticks
    }
}

impl Chore {
    pub(crate) const fn ticks(self) -> u32 {
        match self {
            Self::Flash => 2,
            Self::HandIn => 4,
            Self::Return => 6,
        }
    }
}

impl Clock {
    /// Moves the clock on by `ticks` and returns how often the planet set.
    pub(crate) fn advance(&mut self, ticks: u32) -> u32 {
        let before = self.sets();
        self.ticks += ticks;
        self.sets() - before
    }

    /// How often the planet has set since it first rose.
    pub(crate) fn sets(&self) -> u32 {
        self.ticks / CYCLE
    }

    /// Towards the planet as it is shown. It rises to the right of the shop
    /// front, climbs across the street and sets to the left.
    pub(crate) fn planet(&self) -> Vec3 {
        let phase = (self.shown / CYCLE as f32).fract();
        let across = SWING * (1.0 - 2.0 * phase);
        let up = LOW + (HIGH - LOW) * (PI * phase).sin();
        Vec3::new(up.cos() * across.sin(), up.sin(), -up.cos() * across.cos())
    }

    /// How much of its light the planet gives at its height, from 0 below
    /// the horizon to 1 well above it.
    pub(crate) fn planet_light(&self) -> f32 {
        (self.planet().y * 4.0).clamp(0.0, 1.0)
    }

    fn pass(
        busy: On<Busy>,
        job: Option<Res<Job>>,
        mut clock: ResMut<Clock>,
        mut commands: Commands,
    ) {
        if job.is_some_and(|job| matches!(job.origin, Origin::Lesson { .. })) {
            return;
        }
        for _ in 0..clock.advance(busy.0.ticks()) {
            commands.trigger(PlanetSet);
        }
    }

    fn glide(time: Res<Time>, mut clock: ResMut<Clock>) {
        let goal = clock.ticks as f32;
        if (goal - clock.shown).abs() < ON_TIME {
            return;
        }
        clock
            .shown
            .smooth_nudge(&goal, GLIDE_RATE, time.delta_secs());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workshop::levels::Levels;
    use crate::workshop::run::Run;

    #[derive(Resource, Default)]
    struct Sets(u32);

    #[test]
    fn planet_sets_once_per_cycle() {
        let mut clock = Clock::default();
        assert_eq!(clock.advance(CYCLE - OPENING - 1), 0);
        assert_eq!(clock.advance(1), 1);
        assert_eq!(clock.ticks, CYCLE);
        assert_eq!(clock.advance(2 * CYCLE + 3), 2);
        assert_eq!(clock.ticks % CYCLE, 3);
    }

    #[test]
    fn each_chore_triggers_the_set_it_reaches() {
        let mut world = World::new();
        world.init_resource::<Clock>();
        world.add_observer(Clock::pass);
        world.init_resource::<Sets>();
        world.add_observer(|_: On<PlanetSet>, mut sets: ResMut<Sets>| sets.0 += 1);
        let chores = (CYCLE - OPENING).div_ceil(Chore::HandIn.ticks());
        for _ in 0..chores - 1 {
            world.trigger(Busy(Chore::HandIn));
        }
        world.flush();
        assert_eq!(world.resource::<Sets>().0, 0);
        world.trigger(Busy(Chore::HandIn));
        world.flush();
        assert_eq!(world.resource::<Sets>().0, 1);
    }

    #[test]
    fn tutorial_jobs_take_no_time() {
        let mut world = World::new();
        world.init_resource::<Clock>();
        world.add_observer(Clock::pass);
        let levels = Levels::on_disk();
        let run = Run::default();
        world.insert_resource(Job::new(run.pick(&levels), &run, &levels).unwrap());
        world.trigger(Busy(Chore::Flash));
        world.flush();
        assert_eq!(world.resource::<Clock>().ticks, OPENING);
        world.remove_resource::<Job>();
        world.trigger(Busy(Chore::Flash));
        world.flush();
        assert_eq!(
            world.resource::<Clock>().ticks,
            OPENING + Chore::Flash.ticks()
        );
    }

    #[test]
    fn planet_crosses_the_sky_in_front_of_the_shop() {
        let at = |ticks: f32| {
            Clock {
                ticks: 0,
                shown: ticks,
            }
            .planet()
        };
        let cycle = CYCLE as f32;
        let (rise, noon, set) = (at(0.0), at(cycle / 2.0), at(cycle - 0.01));
        assert!(rise.y < 0.0 && set.y < 0.0);
        assert!(rise.x > 0.0 && set.x < 0.0);
        assert!(noon.y > 0.5 && noon.z < -0.7 && noon.x.abs() < 0.01);
        for t in 0..CYCLE {
            let planet = at(t as f32);
            assert!(planet.z < 0.0, "{planet}");
            assert!((planet.length() - 1.0).abs() < 1e-5, "{planet}");
        }
        assert_eq!(
            Clock {
                ticks: 0,
                shown: 0.0
            }
            .planet_light(),
            0.0
        );
    }

    #[test]
    fn the_shop_opens_with_the_planet_up() {
        let clock = Clock::default();
        assert!(clock.planet().y > 0.3, "{}", clock.planet());
        assert_eq!(clock.planet_light(), 1.0);
    }
}
