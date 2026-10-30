use std::f32::consts::TAU;

/// Seconds for the air on a sheet to build up and die down.
const GUST_TIME: f32 = 0.35;
/// Longer steps than this, in seconds, are split up, so the swing stays
/// stable after a hitch.
const MAX_STEP: f32 = 1.0 / 60.0;

/// A sheet swinging about the edge it is held by, as air blows against it.
#[derive(Clone, Copy, Debug)]
pub struct Swing {
    /// How often it swings back and forth on its own, in hertz.
    rate: f32,
    /// The share of the critical damping, which the drag in the air gives
    /// the swing.
    damping: f32,
    /// Angular acceleration per meter per second of air against the sheet.
    push: f32,
    /// In radians.
    angle: f32,
    speed: f32,
    /// The air against the sheet, smoothed, in meters per second.
    gust: f32,
}

impl Swing {
    pub const fn new(rate: f32, damping: f32, push: f32) -> Self {
        Self {
            rate,
            damping,
            push,
            angle: 0.0,
            speed: 0.0,
            gust: 0.0,
        }
    }

    pub fn angle(&self) -> f32 {
        self.angle
    }

    /// The air against the sheet, smoothed, in meters per second.
    pub fn gust(&self) -> f32 {
        self.gust
    }

    /// Swings on by `dt` with `against` meters per second of air blowing
    /// into the sheet, towards a larger angle.
    pub fn step(&mut self, against: f32, dt: f32) {
        let omega = TAU * self.rate;
        let mut left = dt;
        while left > 0.0 {
            let step = left.min(MAX_STEP);
            self.gust += (against - self.gust) * (step / GUST_TIME).min(1.0);
            let pull = -omega * omega * self.angle - 2.0 * self.damping * omega * self.speed;
            self.speed += (pull + self.push * self.gust) * step;
            self.angle += self.speed * step;
            left -= step;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn swing() -> Swing {
        Swing::new(0.8, 0.35, 9.0)
    }

    #[test]
    fn a_gust_swings_the_sheet_and_it_settles() {
        let mut swing = swing();
        for _ in 0..30 {
            swing.step(0.2, 1.0 / 60.0);
        }
        assert!(swing.angle > 0.01 && swing.angle < 0.1, "{}", swing.angle);
        for _ in 0..600 {
            swing.step(0.0, 1.0 / 60.0);
        }
        assert!(swing.angle.abs() < 1e-3);
    }

    #[test]
    fn long_frames_stay_stable() {
        let mut swing = swing();
        swing.step(1.0, 0.5);
        for _ in 0..20 {
            swing.step(0.0, 0.25);
        }
        assert!(swing.angle.abs() < 0.05);
    }
}
