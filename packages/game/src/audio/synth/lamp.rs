use super::{Lowpass, Mode, Noise, Resonator, Room, Samples, Vary};
use crate::layout::desk::DeskSpots;

/// The plastic plunger of the switch knocking against its stops.
const PLUNGER: [Mode; 3] = [
    Mode::new(1_250.0, 900.0, 1.0),
    Mode::new(2_100.0, 1_300.0, 0.6),
    Mode::new(3_300.0, 1_800.0, 0.3),
];
/// The brass contact snapping over, small and bright.
const CONTACT: [Mode; 3] = [
    Mode::new(4_600.0, 1_500.0, 1.0),
    Mode::new(6_900.0, 2_200.0, 0.6),
    Mode::new(9_800.0, 3_000.0, 0.3),
];
/// The steel base the switch sits in. Its modes follow those of a disc
/// held at its rim, damped by the weight and the felt under it.
const BASE: [Mode; 4] = [
    Mode::new(520.0, 300.0, 0.5),
    Mode::new(1_080.0, 450.0, 0.4),
    Mode::new(1_780.0, 600.0, 0.3),
    Mode::new(2_640.0, 800.0, 0.2),
];
/// The pole, a steel tube, ringing faintly above the base.
const POLE: [Mode; 2] = [
    Mode::new(760.0, 25.0, 0.015),
    Mode::new(2_090.0, 40.0, 0.01),
];
/// Seconds from pressing the button to letting go of it.
const HOLD: f32 = 0.09;

impl Samples {
    /// A push through switch in the base of the desk lamp, which latches
    /// down on one press and springs back up on the next. Pressing knocks
    /// the plunger against its bottom stop and snaps the contact over.
    /// Letting go of it, the plunger springs up: only a little when it
    /// latches, all the way to its top stop when it comes free. So the lamp
    /// clicks hard and ticks when it goes on, and clicks and clacks when it
    /// goes off. Every hit rings the steel base and the pole.
    pub(in crate::audio) fn lamp_switch(seed: u32, on: bool) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(if on { 70 } else { 80 }));
        let snap = 0.0025 * vary.by(0.3);
        let release = HOLD * vary.by(0.3);
        let (press, back) = if on {
            (1.0, 0.18 * vary.by(0.3))
        } else {
            (0.8, 0.65 * vary.by(0.2))
        };
        let contact = 4.0 * vary.by(0.25);
        let mut plunger = Resonator::even(&PLUNGER, vary.by(0.03));
        let mut contacts = Resonator::even(&CONTACT, vary.by(0.03));
        let mut base = Resonator::even(&BASE, 1.0);
        let mut pole = Resonator::even(&POLE, 1.0);
        for mode in base.iter_mut().chain(&mut pole) {
            mode.gain *= vary.by(0.3);
        }
        let mut noise = Noise::new(seed.wrapping_add(71));
        let mut grit = Lowpass::at(6_000.0);
        Self::generate(release + 0.12, |t| {
            let knock = Self::pulse(t, 0.00015) * press + Self::pulse(t - release, 0.0001) * back;
            let flip = Self::pulse(t - snap, 0.00008) * contact;
            let scrape = grit.next(noise.next())
                * (Self::pulse(t, 0.001) * press + Self::pulse(t - release, 0.0006) * back)
                * 0.08;
            let hit = knock + flip * 0.2;
            plunger.iter_mut().map(|mode| mode.next(knock)).sum::<f32>()
                + contacts.iter_mut().map(|mode| mode.next(flip)).sum::<f32>()
                + base.iter_mut().map(|mode| mode.next(hit)).sum::<f32>()
                + pole.iter_mut().map(|mode| mode.next(hit)).sum::<f32>()
                + scrape
        })
        .in_room(seed, &{
            let spots = DeskSpots::new();
            Room::new(spots.lamp, spots.seat)
        })
        .normalized(0.8)
    }
}
