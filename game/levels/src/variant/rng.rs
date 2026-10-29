/// SplitMix64 keyed by a seed and a salt. Equal inputs give equal choices.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64, salt: &str) -> Self {
        let fnv = salt.bytes().fold(0xCBF2_9CE4_8422_2325, |h: u64, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
        });
        Self(seed ^ fnv)
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 up to but not including `len`.
    pub(crate) fn below(&mut self, len: usize) -> usize {
        (self.next_u64() % len as u64) as usize
    }

    pub(crate) fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}
