use bevy::prelude::*;

/// Seconds for a key to go all the way down.
const DOWN_TIME: f32 = 0.022;
/// Seconds for a key to spring back up, a little past its rest and back.
const UP_TIME: f32 = 0.05;

/// Where a key is on its way down or back up. A key always reaches the
/// bottom before it comes back, however short the press.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Stroke {
    /// Going down from `from`, `t` seconds ago.
    Down { from: f32, t: f32 },
    /// Coming back up from the bottom, `t` seconds ago.
    Up { t: f32 },
}

impl Default for Stroke {
    fn default() -> Self {
        Self::Up { t: UP_TIME }
    }
}

impl Stroke {
    /// How far down the key is, from 0 at rest to 1 at the bottom.
    pub(super) fn depth(self) -> f32 {
        match self {
            Self::Down { from, t } => {
                from + (1.0 - from) * EaseFunction::QuadraticOut.sample_clamped(t / DOWN_TIME)
            }
            Self::Up { t } => 1.0 - EaseFunction::BackOut.sample_clamped(t / UP_TIME),
        }
    }

    /// The stroke `dt` seconds later, while the key is `held` or not.
    pub(super) fn step(self, held: bool, dt: f32) -> Self {
        match self {
            Self::Down { t, .. } if t >= DOWN_TIME && !held => Self::Up { t: dt.min(UP_TIME) },
            Self::Down { from, t } => Self::Down {
                from,
                t: (t + dt).min(DOWN_TIME),
            },
            Self::Up { .. } if held => Self::Down {
                from: self.depth(),
                t: dt.min(DOWN_TIME),
            },
            Self::Up { t } => Self::Up {
                t: (t + dt).min(UP_TIME),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// The depths of `frames` steps, each with the key held or not.
    fn depths(frames: &[bool], dt: f32) -> Vec<f32> {
        let mut stroke = Stroke::default();
        frames
            .iter()
            .map(|&held| {
                stroke = stroke.step(held, dt);
                stroke.depth()
            })
            .collect()
    }

    #[test]
    fn a_tap_reaches_the_bottom_before_it_comes_back() {
        for dt in [DT, 1.0 / 144.0, 0.1] {
            let mut frames = vec![true];
            frames.extend([false; 30]);
            let depths = depths(&frames, dt);
            let bottom = depths.iter().position(|&depth| depth == 1.0);
            assert!(bottom.is_some(), "{dt} {depths:?}");
            assert!(
                depths[..bottom.unwrap()].windows(2).all(|w| w[0] < w[1]),
                "{depths:?}"
            );
            assert_eq!(*depths.last().unwrap(), 0.0);
        }
    }

    #[test]
    fn a_held_key_stays_down_and_springs_back_on_release() {
        let mut frames = vec![true; 10];
        frames.extend([false; 10]);
        let depths = depths(&frames, DT);
        assert!(depths[0] > 0.5 && depths[0] < 1.0, "{depths:?}");
        assert!(depths[2..10].iter().all(|&depth| depth == 1.0));
        let overshoot = depths[10..].iter().copied().fold(f32::INFINITY, f32::min);
        assert!(overshoot < 0.0 && overshoot > -0.15, "{depths:?}");
        assert_eq!(depths[19], 0.0);
    }

    #[test]
    fn pressing_again_goes_down_from_where_the_key_is() {
        let stroke = Stroke::Up { t: UP_TIME / 3.0 };
        let depth = stroke.depth();
        assert!(depth > 0.0 && depth < 1.0);
        let again = stroke.step(true, 0.0);
        assert!((again.depth() - depth).abs() < 1e-6);
    }
}
