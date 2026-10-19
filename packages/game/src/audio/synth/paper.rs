use std::f32::consts::PI;

use super::{Band, Lowpass, Mode, Noise, Resonator, SAMPLE_RATE, Samples, Vary};

/// How loud the shop answers a sheet.
const ROOM: f32 = 0.08;
/// Seconds of sound after the sheet lands.
const SETTLE: f32 = 0.15;
/// How far a hit moves the body of a sheet, which is light and soft.
const GIVE: f32 = 0.0015;
/// The steel rings of the binder, which the holes of a sheet scrape along.
const RINGS: [Mode; 3] = [
    Mode::new(2_900.0, 60.0, 1.0),
    Mode::new(4_700.0, 80.0, 0.7),
    Mode::new(7_300.0, 110.0, 0.4),
];

/// A page of thin paper: many small crackles and a bright rustle, and a
/// soft flap as it lands.
const PAGE: Sheet = Sheet {
    turn: 0.3,
    crackles: 350.0,
    crackle_floor: 0.04,
    crackle_band: (1_500.0, 5_000.0),
    rustle: 0.12,
    rustle_band: (900.0, 4_000.0),
    air: 0.09,
    pop: 0.15,
    flap: 0.21,
    slap: 0.35,
    body: &[Mode::new(170.0, 110.0, 1.0), Mode::new(330.0, 180.0, 0.5)],
    rings: 0.0013,
};

/// A divider of card: too stiff to crackle much, so it pops and flexes
/// with a few louder ticks, moves more air and lands with a firm slap.
const DIVIDER: Sheet = Sheet {
    turn: 0.38,
    crackles: 25.0,
    crackle_floor: 0.1,
    crackle_band: (900.0, 4_500.0),
    rustle: 0.035,
    rustle_band: (600.0, 2_500.0),
    air: 0.22,
    pop: 0.7,
    flap: 0.6,
    slap: 1.0,
    body: &[
        Mode::new(210.0, 70.0, 1.0),
        Mode::new(460.0, 130.0, 0.7),
        Mode::new(940.0, 260.0, 0.4),
    ],
    rings: 0.003,
};

/// How a sheet sounds as it turns over the rings of the binder.
struct Sheet {
    /// Seconds from the start of the turn until the sheet lands, as long as
    /// the animation.
    turn: f32,
    /// Crackles per second where the sheet bends fastest.
    crackles: f32,
    /// The quietest crackle. Most crackles are this quiet, a few are much
    /// louder.
    crackle_floor: f32,
    /// The band in hertz of the crackles.
    crackle_band: (f32, f32),
    /// How loud the surface rustles while the sheet sweeps over.
    rustle: f32,
    /// The band in hertz of the rustle.
    rustle_band: (f32, f32),
    /// How loud the air is that the sheet pushes away.
    air: f32,
    /// How loud the sheet pops as it comes off the pile.
    pop: f32,
    /// How loud the air is that the sheet pushes out as it lands.
    flap: f32,
    /// How hard the sheet lands on the pile.
    slap: f32,
    /// How the sheet rings when it lands.
    body: &'static [Mode],
    /// How loud the holes scrape the rings.
    rings: f32,
}

impl Samples {
    /// A page turning: it peels off the pile, sweeps over with a rustle and
    /// crackles where it bends, and flaps down on the pages under it.
    pub(in crate::audio) fn page_turn(seed: u32) -> Self {
        Self::turn(seed, &PAGE)
    }

    /// A divider of card turning, stiffer and slower than a page.
    pub(in crate::audio) fn divider_turn(seed: u32) -> Self {
        Self::turn(seed.wrapping_add(500), &DIVIDER)
    }

    /// After the crumpling noise of Houle and Sethna, "Acoustic emission
    /// from crumpling paper": a crackle comes each time the sheet buckles,
    /// with power law sizes, in bursts. The turn follows the smooth step of
    /// the animation, so it moves fastest halfway and bends fastest at both
    /// ends.
    fn turn(seed: u32, sheet: &Sheet) -> Self {
        let mut vary = Vary::new(seed.wrapping_add(60));
        let land = sheet.turn * vary.by(0.04);
        let pop_at = 0.02 * vary.by(0.5);
        let pop = sheet.pop * vary.by(0.5);
        let mut noise = Noise::new(seed.wrapping_add(61));
        let mut dice = Noise::new(seed.wrapping_add(62));
        let mut gusts = Noise::new(seed.wrapping_add(63));
        let mut gust = Lowpass::at(12.0);
        let mut crackle_band = Band::new(
            sheet.crackle_band.0 * vary.by(0.15),
            sheet.crackle_band.1 * vary.by(0.15),
        );
        let mut rustle_band = Band::new(sheet.rustle_band.0, sheet.rustle_band.1 * vary.by(0.1));
        let mut air_band = Band::new(100.0, 600.0);
        let mut puff_band = Band::new(200.0, 2_500.0);
        let mut body = Resonator::even(sheet.body, vary.by(0.08));
        let mut rings = Resonator::even(&RINGS, vary.by(0.05));
        let mut crackle = 0.0_f32;
        let mut onset = Lowpass::at(1_500.0);
        // About half a millisecond per crackle.
        let fade = (-1.0 / (0.0005 * SAMPLE_RATE as f32)).exp();
        Self::generate(land + SETTLE, |t| {
            let white = noise.next();
            let along = (t / land).min(1.0);
            let speed = 4.0 * along * (1.0 - along);
            let bending = if t < land {
                (PI * along).cos().abs()
            } else {
                0.0
            };
            let landed = t - land;
            let settling = if landed < 0.0 {
                0.0
            } else {
                0.4 * (-landed / 0.015).exp()
            };
            let burst = (gust.next(gusts.next()) * 25.0).abs().min(1.0);
            let rate = sheet.crackles * (0.6 * bending + 0.4 * speed + settling) * (0.3 + burst);
            if (dice.next() + 1.0) / 2.0 < rate / SAMPLE_RATE as f32 {
                let size = ((dice.next() + 1.0) / 2.0).max(0.001);
                crackle += (sheet.crackle_floor * size.powf(-0.7)).min(0.6);
            }
            crackle *= fade;
            let crackles = crackle_band.next(white) * onset.next(crackle);
            let rustle = rustle_band.next(white) * sheet.rustle * speed.powf(1.5) * (0.6 + burst);
            let air = air_band.next(white) * sheet.air * speed * speed;
            let puff = if landed < 0.0 {
                0.0
            } else {
                puff_band.next(white)
                    * sheet.flap
                    * (landed / 0.002).min(1.0)
                    * (-landed / 0.012).exp()
            };
            let hit =
                Self::pulse(landed, 0.004) * sheet.slap + Self::pulse(t - pop_at, 0.0006) * pop;
            let thump = body.iter_mut().map(|mode| mode.next(hit)).sum::<f32>();
            let scrape = Self::pulse(t - pop_at, 0.003) + Self::pulse(landed, 0.003);
            let ring = rings
                .iter_mut()
                .map(|mode| mode.next(white * scrape * sheet.rings))
                .sum::<f32>();
            crackles + rustle + air + puff + thump * GIVE + ring
        })
        .in_room(seed, ROOM)
        .normalized(0.7)
    }
}
