use std::ops::Range;

use rand::{Rng, RngExt};

use super::super::bird::{Food, Meal};

/// Seated, people wait this long to be served, reach over the counter for
/// this long, and take a bite or sip this often, for this long.
const SERVED_AFTER: Range<f32> = 2.0..6.0;
const REACH_TIME: f32 = 1.2;
const BITE_EVERY: Range<f32> = 3.0..11.0;
const BITE_TIME: f32 = 2.2;
/// Shares of people who have tea or soup at a stall. The others just sit.
const TEA: f64 = 0.45;
const SOUP: f64 = 0.6;

/// A meal at a stall, from ordering it to putting the dish back.
#[derive(Debug, Clone, Copy)]
pub(super) struct Dining {
    food: Option<Food>,
    step: Step,
    /// Seconds into the step.
    time: f32,
    /// Seconds until the next bite.
    until_bite: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Ordering,
    Taking,
    Eating,
    Biting,
    PuttingBack,
    Done,
}

impl Dining {
    /// A meal for someone just sat down, a bowl rather than a cup if its
    /// right hand holds an umbrella.
    pub(super) fn order(umbrella: bool, rng: &mut impl Rng) -> Self {
        let roll = rng.random_bool(TEA);
        let food = if roll && !umbrella {
            Some(Food::Cup)
        } else if rng.random_bool(SOUP) {
            Some(Food::Bowl)
        } else {
            None
        };
        Self {
            food,
            step: if food.is_some() {
                Step::Ordering
            } else {
                Step::Done
            },
            time: 0.0,
            until_bite: rng.random_range(SERVED_AFTER),
        }
    }

    /// A meal someone is already eating.
    pub(super) fn served(umbrella: bool, rng: &mut impl Rng) -> Self {
        let mut dining = Self::order(umbrella, rng);
        if dining.food.is_some() {
            dining.step = Step::Eating;
            dining.until_bite = rng.random_range(BITE_EVERY);
        }
        dining
    }

    /// Soup instead of tea, once its right hand holds an umbrella.
    pub(super) fn fit_umbrella(&mut self, umbrella: bool) {
        if umbrella && self.food == Some(Food::Cup) {
            self.food = Some(Food::Bowl);
        }
    }

    /// Whether the dish is back on the counter, if there was one.
    pub(super) fn is_done(&self) -> bool {
        self.step == Step::Done
    }

    /// Moves on by `dt` seconds, and finishes up once it is `leaving`.
    /// Returns what the bird does with its food now.
    pub(super) fn go_on(&mut self, dt: f32, leaving: bool, rng: &mut impl Rng) -> Meal {
        self.time += dt;
        match self.step {
            Step::Ordering if leaving => self.step = Step::Done,
            Step::Ordering => {
                self.until_bite -= dt;
                if self.until_bite <= 0.0 {
                    self.next(Step::Taking);
                }
            }
            Step::Taking | Step::PuttingBack if self.time >= REACH_TIME => {
                let after = if self.step == Step::Taking {
                    self.until_bite = rng.random_range(BITE_EVERY);
                    Step::Eating
                } else {
                    Step::Done
                };
                self.next(after);
            }
            Step::Eating if leaving => self.next(Step::PuttingBack),
            Step::Eating => {
                self.until_bite -= dt;
                if self.until_bite <= 0.0 {
                    self.next(Step::Biting);
                }
            }
            Step::Biting if self.time >= BITE_TIME => {
                self.until_bite = rng.random_range(BITE_EVERY);
                self.next(Step::Eating);
            }
            _ => {}
        }
        self.meal()
    }

    fn next(&mut self, step: Step) {
        self.step = step;
        self.time = 0.0;
    }

    /// What the bird does with its food in the current step.
    fn meal(&self) -> Meal {
        let t = self.time / REACH_TIME;
        // Out over the counter and back, the dish changing hands halfway.
        let reach = (std::f32::consts::PI * t.clamp(0.0, 1.0)).sin();
        let held = match self.step {
            Step::Taking => t > 0.5,
            Step::PuttingBack => t < 0.5,
            Step::Eating | Step::Biting => true,
            Step::Ordering | Step::Done => false,
        };
        let reaching = matches!(self.step, Step::Taking | Step::PuttingBack);
        Meal {
            food: self.food.filter(|_| self.step != Step::Done),
            held,
            reach: if reaching { reach } else { 0.0 },
            bite: if self.step == Step::Biting {
                self.time / BITE_TIME
            } else {
                0.0
            },
        }
    }
}
