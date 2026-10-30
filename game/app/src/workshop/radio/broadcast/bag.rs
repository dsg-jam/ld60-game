use rand::{Rng, RngExt};

/// A pool of items where each item is out with one holder at a time.
///
/// [`ShuffleBag::draw`] keeps one item back, so [`ShuffleBag::exchange`]
/// always has another item to give for the one it takes back.
#[derive(Debug)]
pub(super) struct ShuffleBag<T> {
    items: Vec<T>,
}

/// An item taken out of a [`ShuffleBag`]. Only the bag hands them out.
#[derive(Debug)]
pub(super) struct Drawn<T>(T);

impl<T> ShuffleBag<T> {
    pub(super) fn new(items: impl IntoIterator<Item = T>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    /// Takes out a random item, unless only one is left.
    pub(super) fn draw(&mut self, rng: &mut impl Rng) -> Option<Drawn<T>> {
        (self.items.len() > 1).then(|| {
            let index = rng.random_range(..self.items.len());
            Drawn(self.items.swap_remove(index))
        })
    }

    /// Swaps `drawn` for a random item still in the bag. The new item is
    /// picked before `drawn` goes back, so it is never the same one.
    pub(super) fn exchange(&mut self, rng: &mut impl Rng, drawn: &mut Drawn<T>) {
        if self.items.is_empty() {
            return;
        }
        let index = rng.random_range(..self.items.len());
        std::mem::swap(&mut self.items[index], &mut drawn.0);
    }
}

impl<T> Drawn<T> {
    pub(super) fn get(&self) -> &T {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::SmallRng};

    use super::*;

    #[test]
    fn draw_keeps_one_back() {
        let mut rng = SmallRng::seed_from_u64(1);
        let mut bag = ShuffleBag::new(0..5);
        let drawn: Vec<_> = std::iter::from_fn(|| bag.draw(&mut rng)).collect();
        assert_eq!(drawn.len(), 4);
        assert_eq!(bag.items.len(), 1);
    }

    #[test]
    fn an_exchange_gives_another_item_and_no_item_is_out_twice() {
        let mut rng = SmallRng::seed_from_u64(2);
        let mut bag = ShuffleBag::new(0..4);
        let mut out: Vec<_> = std::iter::from_fn(|| bag.draw(&mut rng)).collect();
        for round in 0..1000 {
            let holder = &mut out[round % 3];
            let before = *holder.get();
            bag.exchange(&mut rng, holder);
            assert_ne!(*holder.get(), before);
            let mut held: Vec<_> = out.iter().map(|drawn| *drawn.get()).collect();
            held.extend(&bag.items);
            held.sort_unstable();
            assert_eq!(held, [0, 1, 2, 3]);
        }
    }
}
