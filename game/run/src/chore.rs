use bevy::prelude::*;
use ld60_game_core::clock::Clock;

use crate::Origin;
use crate::job::Job;

pub struct ChorePlugin;

impl Plugin for ChorePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Busy::pass);
    }
}

/// The shop spends time on `0`. Tutorial jobs take no time.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Busy(pub Chore);

/// Work that takes time. Work at the bench is free, only customers take
/// time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chore {
    /// A customer coming to try a device and take it home.
    HandIn,
    /// A customer coming to try a device and handing it back.
    Return,
}

/// The planet went down, so the rent is due.
#[derive(Event, Debug)]
pub struct PlanetSet;

impl Chore {
    pub(crate) const fn ticks(self) -> u32 {
        match self {
            Self::HandIn => 4,
            Self::Return => 4,
        }
    }
}

impl Busy {
    /// Moves the [`Clock`] on, unless the job is a lesson.
    fn pass(
        busy: On<Self>,
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
}

#[cfg(test)]
mod tests {
    use ld60_game_core::clock::{CYCLE, OPENING};

    use super::*;
    use crate::Run;
    use crate::levels::Levels;

    #[derive(Resource, Default)]
    struct Sets(u32);

    #[test]
    fn each_chore_triggers_the_set_it_reaches() {
        let mut world = World::new();
        world.init_resource::<Clock>();
        world.add_observer(Busy::pass);
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
        world.add_observer(Busy::pass);
        let levels = Levels::on_disk();
        let run = Run::default();
        world.insert_resource(Job::new(run.pick(&levels), &run, &levels).unwrap());
        world.trigger(Busy(Chore::Return));
        world.flush();
        assert_eq!(u32::from(world.resource::<Clock>().clone()), OPENING);
        world.remove_resource::<Job>();
        world.trigger(Busy(Chore::Return));
        world.flush();
        assert_eq!(
            u32::from(world.resource::<Clock>().clone()),
            OPENING + Chore::Return.ticks()
        );
    }
}
