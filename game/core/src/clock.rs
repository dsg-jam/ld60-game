use std::f32::consts::PI;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Ticks from planet rise to planet set.
pub const CYCLE: u32 = 40;
/// Ticks into the cycle when a run starts, so the shop opens with the planet
/// well up.
pub const OPENING: u32 = CYCLE / 4;
/// How far the planet swings to each side of straight across the street, in
/// radians. At the ends it is behind the buildings next to the low ones
/// across the street.
const SWING: f32 = 0.7;
/// The planet's height above the horizon at rise and set, in radians. Low
/// enough to hide behind the buildings at the sides.
const LOW: f32 = 0.3;
/// The planet's height above the horizon at its highest, in radians. It
/// passes over the low buildings across the street, below the noren.
const HIGH: f32 = 0.58;
/// Angular radius of the planet, in radians.
pub const PLANET_SIZE: f32 = 0.09;
/// Share of the cycle after rise and before set when the planet is still
/// behind the buildings.
const BEHIND: f32 = 0.1;
/// Seconds the planet takes to glide to the time on the clock.
const GLIDE_TIME: f32 = 1.2;

pub struct ClockPlugin;

impl Plugin for ClockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Clock>()
            .add_systems(Update, Clock::glide);
    }
}

/// The time on the planet, which only moves on when the shop works.
///
/// The ringed planet crosses the sky once in each [`CYCLE`]. The landlord
/// comes for the rent each time it sets.
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "u32", into = "u32")]
pub struct Clock {
    /// Ticks since the planet first rose.
    ticks: u32,
    /// Where the planet is shown, in ticks. It glides after `ticks`.
    shown: f32,
    /// Where the planet was shown when the glide started, in ticks.
    from: f32,
    /// How far the glide has come, from 0 to 1.
    glided: f32,
}

impl Default for Clock {
    fn default() -> Self {
        Self::from(OPENING)
    }
}

impl From<u32> for Clock {
    /// The clock at `ticks`, with the planet shown there.
    fn from(ticks: u32) -> Self {
        Self {
            ticks,
            shown: ticks as f32,
            from: ticks as f32,
            glided: 1.0,
        }
    }
}

impl From<Clock> for u32 {
    fn from(clock: Clock) -> Self {
        clock.ticks
    }
}

impl Clock {
    /// Moves the clock on by `ticks` and returns how often the planet set.
    pub fn advance(&mut self, ticks: u32) -> u32 {
        let before = self.sets();
        self.ticks += ticks;
        self.from = self.shown;
        self.glided = 0.0;
        self.sets() - before
    }

    /// How often the planet has set since it first rose.
    pub fn sets(&self) -> u32 {
        self.ticks / CYCLE
    }

    /// How far the planet as it is shown has come from rise to set, from 0
    /// to 1.
    pub fn phase(&self) -> f32 {
        (self.shown / CYCLE as f32).fract()
    }

    /// Towards the planet as it is shown. It rises behind the buildings to
    /// the right of the shop front, climbs over the low buildings across the
    /// street and sets behind the buildings to the left.
    pub fn planet(&self) -> Vec3 {
        let phase = self.phase();
        let across = SWING * (1.0 - 2.0 * phase);
        let up = LOW + (HIGH - LOW) * (PI * phase).sin();
        Vec3::new(up.cos() * across.sin(), up.sin(), -up.cos() * across.cos())
    }

    /// How much of its light the planet gives, from 0 at rise and set to 1
    /// once it is clear of the buildings.
    pub fn planet_light(&self) -> f32 {
        ((PI * self.phase()).sin() / (PI * BEHIND).sin()).min(1.0)
    }

    /// Moves the shown planet on by `secs` of its glide to the time.
    fn glide_by(&mut self, secs: f32) {
        self.glided = (self.glided + secs / GLIDE_TIME).min(1.0);
        let eased = EaseFunction::SmoothStep.sample_clamped(self.glided);
        self.shown = self.from + (self.ticks as f32 - self.from) * eased;
    }

    fn glide(time: Res<Time>, mut clock: ResMut<Clock>) {
        if clock.glided < 1.0 {
            clock.glide_by(time.delta_secs());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn planet_crosses_the_sky_in_front_of_the_shop() {
        let at = |ticks: f32| {
            let mut clock = Clock::from(0);
            clock.shown = ticks;
            clock.planet()
        };
        let yaw = |planet: Vec3| planet.x.atan2(-planet.z);
        let cycle = CYCLE as f32;
        let (rise, noon, set) = (at(0.0), at(cycle / 2.0), at(cycle - 0.01));
        // Rise and set are behind the buildings next to the low ones across.
        assert!(yaw(rise) > 0.6 && yaw(set) < -0.6, "{rise} {set}");
        assert!(rise.y.asin() < 0.35 && set.y.asin() < 0.35, "{rise} {set}");
        // Noon is about where the planet hung before the clock.
        let before = Vec3::new(-0.09, 0.545, -0.834).normalize();
        assert!(noon.angle_between(before) < 0.2, "{noon}");
        for t in 0..CYCLE {
            let planet = at(t as f32);
            assert!(planet.z < 0.0, "{planet}");
            assert!((planet.length() - 1.0).abs() < 1e-5, "{planet}");
            assert!(planet.y.asin() <= HIGH + 1e-5, "{planet}");
        }
        assert_eq!(Clock::from(0).planet_light(), 0.0);
        assert_eq!(Clock::from(CYCLE / 2).planet_light(), 1.0);
    }

    #[test]
    fn the_shop_opens_with_the_planet_up() {
        let clock = Clock::default();
        assert!(clock.planet().y > 0.45, "{}", clock.planet());
        assert_eq!(clock.planet_light(), 1.0);
    }

    #[test]
    fn planet_glides_to_the_time_in_one_go() {
        let mut clock = Clock::default();
        clock.advance(4);
        clock.glide_by(GLIDE_TIME / 2.0);
        let goal = clock.ticks as f32;
        let halfway = (OPENING as f32 + goal) / 2.0;
        assert!((clock.shown - halfway).abs() < 1e-4, "{}", clock.shown);
        clock.glide_by(GLIDE_TIME / 2.0);
        assert_eq!(clock.shown, goal);
        assert_eq!(clock.glided, 1.0);
    }
}
