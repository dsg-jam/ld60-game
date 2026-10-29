use std::f32::consts::TAU;

use super::{Noise, Samples, Vary};

/// The modes of a small coin as ratios of its lowest one, with how loud
/// each rings and how fast it dies down per second.
const COIN: [(f32, f32, f32); 4] = [
    (1.0, 1.0, 14.0),
    (1.52, 0.6, 18.0),
    (2.27, 0.4, 24.0),
    (3.01, 0.25, 30.0),
];

impl Samples {
    /// A coin dropping onto the others in the till: a bright clink and a
    /// quieter bounce. `seed` varies the coin and the bounce.
    pub(crate) fn coin(seed: u32) -> Self {
        let mut vary = Vary::new(seed);
        let mut noise = Noise::new(seed.wrapping_add(31));
        let pitch = 2_900.0 * vary.by(0.15);
        let hits = [(0.0, 1.0), (0.025 * vary.by(0.3), 0.35 * vary.by(0.3))];
        Self::generate(0.3, |t| {
            hits.iter()
                .filter(|(at, _)| t >= *at)
                .map(|&(at, level)| {
                    let t = t - at;
                    let attack = (t * 3_000.0).min(1.0);
                    let ring: f32 = COIN
                        .iter()
                        .map(|&(ratio, gain, decay)| {
                            gain * (TAU * pitch * ratio * t).sin() * (-t * decay).exp()
                        })
                        .sum();
                    let tick = noise.next() * 0.3 * (-t * 4_000.0).exp();
                    (ring * attack + tick) * level
                })
                .sum()
        })
        .normalized(0.6)
    }
}
