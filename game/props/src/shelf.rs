use bevy::prelude::*;
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

use super::group::Group;
use super::prop::{
    Binder, Board, Carton, Coil, Drawers, Handheld, Jar, Lying, Prop, Standing, Tin, placed,
};

/// Room kept free under the board above and in front of the shelf.
const CLEAR: Vec2 = Vec2::new(0.015, 0.01);
/// Groups start this far apart, sometimes with a wider gap.
const GAP: std::ops::Range<f32> = 0.01..0.05;
const WIDE_GAP: std::ops::Range<f32> = 0.08..0.2;
/// Narrower room than this at the end of a board stays empty.
const LEAST: f32 = 0.06;
/// Tries for a group that fits before the board counts as full.
const TRIES: usize = 8;
const LEAN: std::ops::Range<f32> = 0.15..0.4;

/// What a shelf board holds in a repair shop for handheld devices.
#[derive(Clone, Copy, Debug)]
pub enum Theme {
    /// Cartons of stock, some open with salvage in them.
    Stock,
    /// Dead devices brought in for parts, their boards and cables.
    Salvage,
    /// Small parts in drawers, jars and tins.
    Parts,
    /// Binders of manuals and records.
    Papers,
}

impl Theme {
    /// Fills a board with props, as children of `parent`.
    ///
    /// `room` is the space above the board in the space of `parent`: X
    /// along the board, +Y up and +Z out to the front, with the origin in
    /// the middle of the top of the board. The same `seed` fills it the
    /// same way.
    pub fn fill(self, seed: u64, room: Vec3, parent: &mut ChildSpawnerCommands) {
        let mut rng = SmallRng::seed_from_u64(seed);
        let most = Vec3::new(room.x, room.y - CLEAR.x, room.z - CLEAR.y);
        let mut x = -room.x / 2.0 + rng.random_range(GAP);
        loop {
            let left = room.x / 2.0 - x;
            if left < LEAST {
                break;
            }
            let most = most.with_x(left);
            let Some(group) = (0..TRIES)
                .map(|_| self.group(&mut rng, most))
                .find(|group| !group.is_empty() && group.size().cmple(most).all())
            else {
                break;
            };
            let size = group.size();
            let back = rng.random_range(0.0..=(most.z - size.z).min(0.03));
            let at = Vec3::new(x + size.x / 2.0, 0.0, -room.z / 2.0 + back + size.z / 2.0);
            placed(parent, Transform::from_translation(at), |spot| {
                group.spawn(spot);
            });
            let gap = if rng.random_bool(0.15) { WIDE_GAP } else { GAP };
            x += size.x + rng.random_range(gap);
        }
    }

    fn group(self, rng: &mut SmallRng, most: Vec3) -> Group {
        let pick = rng.random::<f32>();
        match self {
            Self::Stock => match pick {
                ..0.45 => Self::one(Carton::closed(rng, most)),
                ..0.75 => Self::carton_stack(rng, most),
                _ => Self::one(Carton::open(rng, most, Self::salvage_inside)),
            },
            Self::Salvage => match pick {
                ..0.3 => Self::standing_handhelds(rng, most),
                ..0.45 => Self::handhelds(rng, most),
                ..0.55 => Self::boards(rng, most),
                ..0.65 => Self::coils(rng, most),
                ..0.85 => Self::one(Carton::open(rng, most, Self::salvage_inside)),
                _ => Self::carton_with_pile(rng, most),
            },
            Self::Parts => match pick {
                ..0.35 => Self::one(Drawers::random(rng, most)),
                ..0.6 => Self::jars(rng, most),
                ..0.75 => Self::tins(rng, most),
                ..0.9 => Self::carton_stack(rng, most.min(Vec3::new(0.3, most.y, most.z))),
                _ => Self::coils(rng, most),
            },
            Self::Papers => match pick {
                ..0.6 => Self::binders(rng, most),
                ..0.8 => Self::binder_pile(rng, most),
                _ => Self::one(Carton::closed(rng, most)),
            },
        }
    }

    fn one(prop: Option<impl Prop + 'static>) -> Group {
        Group::row(prop.map(|prop| Box::new(prop) as Box<dyn Prop>), || 0.0)
    }

    fn pile<P: Prop + 'static>(
        rng: &mut SmallRng,
        count: std::ops::RangeInclusive<usize>,
        mut make: impl FnMut(&mut SmallRng, bool) -> Option<P>,
    ) -> Group {
        let count = rng.random_range(count);
        let props: Vec<_> = (0..count)
            .map_while(|index| make(rng, index + 1 == count))
            .map(|prop| Box::new(prop) as Box<dyn Prop>)
            .collect();
        Group::pile(rng, props)
    }

    /// A pile on its own, as the contents of an open carton.
    fn salvage_inside(rng: &mut SmallRng, room: Vec3) -> Option<Box<dyn Prop>> {
        let group = if rng.random_bool(0.6) {
            Self::handhelds(rng, room)
        } else {
            Self::boards(rng, room)
        };
        (!group.is_empty() && group.size().cmple(room).all())
            .then(|| Box::new(group) as Box<dyn Prop>)
    }

    fn carton_stack(rng: &mut SmallRng, most: Vec3) -> Group {
        let Some(bottom) = Carton::closed(rng, most.with_y(most.y * 0.6)) else {
            return Group::default();
        };
        let size = bottom.size();
        let top = Carton::closed(rng, size.with_y(most.y - size.y) * 0.95);
        Group::pile(
            rng,
            std::iter::once(Box::new(bottom) as Box<dyn Prop>)
                .chain(top.map(|top| Box::new(top) as Box<dyn Prop>)),
        )
    }

    fn handhelds(rng: &mut SmallRng, most: Vec3) -> Group {
        let most = most * 0.9;
        Self::pile(rng, 1..=4, |rng, top| Handheld::random(rng, most, top))
    }

    fn boards(rng: &mut SmallRng, most: Vec3) -> Group {
        let most = most * 0.9;
        Self::pile(rng, 2..=6, |rng, top| Board::random(rng, most, top))
    }

    fn coils(rng: &mut SmallRng, most: Vec3) -> Group {
        let most = most * 0.9;
        Self::pile(rng, 1..=3, |rng, _| Coil::random(rng, most))
    }

    fn jars(rng: &mut SmallRng, most: Vec3) -> Group {
        let count = rng.random_range(2..=5);
        let props: Vec<_> = (0..count)
            .filter_map(|_| Jar::random(rng, most))
            .map(|jar| Box::new(jar) as Box<dyn Prop>)
            .collect();
        Group::row(props, || 0.004)
    }

    fn tins(rng: &mut SmallRng, most: Vec3) -> Group {
        if rng.random_bool(0.5) {
            Self::pile(rng, 2..=3, |rng, _| {
                Tin::random(rng, most.with_y(most.y / 3.0))
            })
        } else {
            let props: Vec<_> = (0..rng.random_range(2..=4))
                .filter_map(|_| Tin::random(rng, most))
                .map(|tin| Box::new(tin) as Box<dyn Prop>)
                .collect();
            Group::row(props, || 0.006)
        }
    }

    /// Binders of one height side by side, the last one sometimes leaning
    /// on the others.
    fn binders(rng: &mut SmallRng, most: Vec3) -> Group {
        let Some(height) = Binder::height(rng, most.y) else {
            return Group::default();
        };
        Self::run(rng, 3..=10, |rng| Binder::random(rng, height, most))
    }

    /// Devices standing on end side by side, facing out.
    fn standing_handhelds(rng: &mut SmallRng, most: Vec3) -> Group {
        let lying = Vec3::new(most.x, most.z, most.y);
        Self::run(rng, 2..=6, |rng| {
            Handheld::random(rng, lying, true).map(Standing)
        })
    }

    /// Props side by side, the last one sometimes leaning on the others.
    fn run<P: Prop + 'static>(
        rng: &mut SmallRng,
        count: std::ops::RangeInclusive<usize>,
        mut make: impl FnMut(&mut SmallRng) -> Option<P>,
    ) -> Group {
        let count = rng.random_range(count);
        let props: Vec<_> = (0..count)
            .filter_map(|_| make(rng))
            .map(|prop| Box::new(prop) as Box<dyn Prop>)
            .collect();
        let gaps: Vec<f32> = (0..props.len())
            .map(|_| rng.random_range(0.0..0.003))
            .collect();
        let mut gaps = gaps.into_iter();
        let mut group = Group::row(props, || gaps.next().unwrap_or_default());
        if !group.is_empty()
            && rng.random_bool(0.5)
            && let Some(prop) = make(rng)
        {
            group.lean(Box::new(prop), rng.random_range(LEAN));
        }
        group
    }

    /// A closed carton with a pile of salvage on it.
    fn carton_with_pile(rng: &mut SmallRng, most: Vec3) -> Group {
        let Some(carton) = Carton::closed(rng, most.with_y(most.y * 0.6)) else {
            return Group::default();
        };
        let size = carton.size();
        let room = size.with_y(most.y - size.y);
        let pile = if rng.random_bool(0.5) {
            Self::handhelds(rng, room)
        } else {
            Self::coils(rng, room)
        };
        Group::pile(
            rng,
            [
                Box::new(carton) as Box<dyn Prop>,
                Box::new(pile) as Box<dyn Prop>,
            ],
        )
    }

    /// Binders lying flat on top of each other.
    fn binder_pile(rng: &mut SmallRng, most: Vec3) -> Group {
        let Some(height) = Binder::height(rng, most.x) else {
            return Group::default();
        };
        let upright = Vec3::new(most.y, most.x, most.z);
        let props: Vec<_> = (0..rng.random_range(2..=4))
            .filter_map(|_| Binder::random(rng, height, upright))
            .map(|binder| Box::new(Lying(binder)) as Box<dyn Prop>)
            .collect();
        Group::pile(rng, props)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEMES: [Theme; 4] = [Theme::Stock, Theme::Salvage, Theme::Parts, Theme::Papers];

    #[test]
    fn any_room_makes_a_group_without_panicking() {
        let mut rng = SmallRng::seed_from_u64(3);
        for _ in 0..20_000 {
            let most = Vec3::new(
                rng.random_range(0.0..1.6),
                rng.random_range(0.0..0.6),
                rng.random_range(0.0..0.4),
            );
            for theme in THEMES {
                let group = theme.group(&mut rng, most);
                let size = group.size();
                assert!(size.is_finite(), "{theme:?} in {most}: {size}");
            }
        }
    }
}
