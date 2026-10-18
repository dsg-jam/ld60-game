use super::rng::Rng;

const CELLS: usize = 16;

/// A permutation of the RAM cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RamMap([u8; CELLS]);

impl RamMap {
    /// Moves every cell `by` cells up, wrapping like `[B+n]` does.
    pub(crate) fn rotate(by: u8) -> Self {
        Self(std::array::from_fn(|c| {
            (c as u8).wrapping_add(by) % CELLS as u8
        }))
    }

    /// Shuffles every cell except the `pinned` ones.
    pub(crate) fn shuffle(pinned: impl IntoIterator<Item = usize>, rng: &mut Rng) -> Self {
        let mut free = [true; CELLS];
        for cell in pinned {
            free[cell % CELLS] = false;
        }
        let cells: Vec<u8> = (0..CELLS as u8).filter(|&c| free[usize::from(c)]).collect();
        let mut targets = cells.clone();
        rng.shuffle(&mut targets);
        let mut map = std::array::from_fn(|c| c as u8);
        for (from, to) in cells.into_iter().zip(targets) {
            map[usize::from(from)] = to;
        }
        Self(map)
    }

    pub(crate) fn get(&self, cell: u8) -> u8 {
        self.0[usize::from(cell) % CELLS]
    }
}
