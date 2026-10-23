use std::f32::consts::PI;

use bevy::math::Vec3;

use super::{Lowpass, Mode, Noise, Resonator, Room, SAMPLE_RATE, Samples, Vary};
use crate::layout::desk::DeskSpots;

const SECONDS: f32 = 0.5;
/// How long the rod swings, in seconds, before it settles with a click.
const SWING: f32 = 0.4;
/// The pin rubbing in its knuckle, small steel parts that ring short.
const PIVOT: [Mode; 3] = [
    Mode::new(1_700.0, 420.0, 1.0),
    Mode::new(2_900.0, 650.0, 0.55),
    Mode::new(4_600.0, 900.0, 0.3),
];
/// Stick and slip of the pin, per second, while the rod swings fastest.
const SLIPS: f32 = 90.0;
/// Where the hinge is, left of the screen and up at its top.
const FROM_SCREEN: f32 = 0.25;

impl Samples {
    /// The hinge of the pin rail turning: the pin sticks and slips in its
    /// knuckle as a soft creak that follows the speed of the swing, and the
    /// rod settles at the end with a quiet tick.
    pub(in crate::audio) fn hinge(seed: u32) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(90));
        let mut bank = Resonator::bank(&PIVOT, vary.by(0.06));
        let mut noise = Noise::new(seed.wrapping_add(91));
        let mut hiss = Lowpass::new(0.3);
        let slips = SLIPS * vary.by(0.2);
        let swing = SWING * vary.by(0.08);
        let mut phase = 0.0;
        let step = 1.0 / SAMPLE_RATE as f32;
        Self::generate(SECONDS, |t| {
            let speed = (PI * (t / swing).min(1.0)).sin();
            phase += slips * (0.5 + speed) * step;
            let slip = if phase >= 1.0 {
                phase -= 1.0;
                speed * (0.6 + 0.4 * noise.next())
            } else {
                0.0
            };
            let tick = Self::pulse(t - swing, 0.0005) * 0.5;
            let rub = hiss.next(noise.next()) * speed * 0.02;
            bank.iter_mut()
                .map(|mode| mode.next(slip + tick))
                .sum::<f32>()
                + rub
        })
        .in_room(seed, &{
            let spots = DeskSpots::new();
            let hinge = spots.computer + Vec3::new(-FROM_SCREEN, FROM_SCREEN, 0.0);
            Room::new(hinge, spots.seat)
        })
        .normalized(0.5)
    }
}
