use std::ops::Range;

use bevy::prelude::*;
use rand::{RngExt, SeedableRng, rngs::SmallRng};

/// A candle in still air barely changes. What little it does is slow.
const WANDER: f32 = 0.03;
const WANDER_RATE: f32 = 0.7;
/// The flame springs back after a draft knocks it, with a wobble that dies
/// out quickly.
const SPRING_HZ: f32 = 5.0;
const DAMPING: f32 = 0.35;
/// Drafts per second in calm air and when the breeze is up.
const CALM_DRAFTS: f32 = 0.08;
const BREEZY_DRAFTS: f32 = 1.2;
/// How fast the breeze comes and goes.
const BREEZE_RATE: f32 = 0.04;
/// How hard a draft knocks the flame down, as a change in speed.
const KNOCK: Range<f32> = 2.5..8.0;
/// The spring is stiff, so it steps in small steps of time.
const STEP: f32 = 0.004;
/// How far the flame sways per unit of dimming, in meters.
const SWAY: f32 = 0.06;
/// Color of a dim flame, which burns redder.
const EMBER: Color = Color::srgb_u8(0xff, 0x50, 0x1a);

pub(super) struct CandlePlugin;

impl Plugin for CandlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Candle::burn, Candle::light, Candle::glow).chain());
    }
}

/// A candle burning in a paper lantern, on the lantern's light. It dims and
/// sways when drafts catch it.
#[derive(Component, Debug)]
pub(super) struct Candle {
    /// Light of the steady flame.
    color: Color,
    intensity: f32,
    /// Where the flame rests.
    rest: Vec3,
    /// Brightness relative to the steady flame, and how fast it changes.
    level: f32,
    speed: f32,
    /// The way the last draft blew.
    blown: Vec3,
    seed: f32,
    time: f32,
    rng: SmallRng,
}

/// The paper of a lantern, glowing with the [`Candle`] on this entity.
#[derive(Component, Debug)]
pub(super) struct Paper {
    pub(super) candle: Entity,
    pub(super) glow: f32,
}

impl Candle {
    pub(super) fn new(color: Color, intensity: f32, rest: Vec3, seed: u64) -> Self {
        Self {
            color,
            intensity,
            rest,
            level: 1.0,
            speed: 0.0,
            blown: Vec3::X,
            seed: (seed % 100) as f32,
            time: 0.0,
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    fn burn(time: Res<Time>, mut candles: Query<&mut Candle>) {
        for mut candle in &mut candles {
            candle.advance(time.delta_secs());
        }
    }

    /// Lets the flame burn on for `seconds`.
    fn advance(&mut self, seconds: f32) {
        let omega = std::f32::consts::TAU * SPRING_HZ;
        // Counts down, so it ends even once the clock is too large to tell
        // a step apart.
        let mut left = seconds;
        while left > 0.0 {
            let dt = STEP.min(left);
            left -= dt;
            self.time += dt;
            let t = self.time;
            let breeze = smoothstep(0.4, 0.8, noise(t * BREEZE_RATE, self.seed));
            let drafts = CALM_DRAFTS + (BREEZY_DRAFTS - CALM_DRAFTS) * breeze;
            if self.rng.random::<f32>() < drafts * dt {
                self.speed -= self.rng.random_range(KNOCK) * (0.5 + breeze);
                let angle = self.rng.random_range(0.0..std::f32::consts::TAU);
                self.blown = Vec3::new(angle.cos(), 0.0, angle.sin());
            }
            let rest = 1.0
                + WANDER
                    * (0.7 * noise(t * WANDER_RATE, self.seed + 1.0)
                        + 0.3 * noise(t * WANDER_RATE * 3.1, self.seed + 2.0));
            let pull = -omega * omega * (self.level - rest) - 2.0 * DAMPING * omega * self.speed;
            self.speed += pull * dt;
            self.level = (self.level + self.speed * dt).clamp(0.3, 1.3);
        }
    }

    fn color(&self) -> Color {
        self.color
            .mix(&EMBER, ((1.0 - self.level) * 2.0).clamp(0.0, 1.0))
    }

    fn light(mut lights: Query<(&Candle, &mut PointLight, &mut Transform)>) {
        for (candle, mut light, mut transform) in &mut lights {
            light.intensity = candle.intensity * candle.level;
            light.color = candle.color();
            // A dimmed flame is one that is bent over.
            let bend = (1.0 - candle.level).max(0.0);
            transform.translation = candle.rest + candle.blown * bend * SWAY;
        }
    }

    fn glow(
        candles: Query<&Candle>,
        papers: Query<(&Paper, &MeshMaterial3d<StandardMaterial>)>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for (paper, material) in &papers {
            let (Ok(candle), Some(mut glowing)) =
                (candles.get(paper.candle), materials.get_mut(material))
            else {
                continue;
            };
            glowing.emissive = candle.color().to_linear() * paper.glow * candle.level;
        }
    }
}

/// Smooth noise from -1 to 1, changing about once per unit of `x`.
fn noise(x: f32, seed: f32) -> f32 {
    let cell = x.floor();
    let f = x - cell;
    let blend = f * f * (3.0 - 2.0 * f);
    let a = hash(cell, seed);
    let b = hash(cell + 1.0, seed);
    (a + (b - a) * blend) * 2.0 - 1.0
}

fn hash(x: f32, seed: f32) -> f32 {
    ((x * 12.9898 + seed * 78.233).sin() * 43_758.547)
        .fract()
        .abs()
}

fn smoothstep(from: f32, to: f32, x: f32) -> f32 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flames_are_mostly_steady() {
        let mut candle = Candle::new(Color::WHITE, 1.0, Vec3::ZERO, 3);
        let mut dim = 0;
        let frames = 60 * 600;
        for _ in 0..frames {
            candle.advance(1.0 / 60.0);
            assert!((0.3..=1.3).contains(&candle.level));
            if candle.level < 0.9 {
                dim += 1;
            }
        }
        // Drafts dim it now and then, but only briefly.
        assert!(dim > 0);
        assert!(dim < frames / 10, "{dim}");
    }
}
