/// A RAM bit that always reads as `high`, whatever was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StuckBit {
    pub cell: u8,
    pub bit: u8,
    pub high: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ram {
    cells: Vec<u8>,
    stuck: Vec<StuckBit>,
}

impl Ram {
    pub fn new(len: usize) -> Self {
        Self {
            cells: vec![0; len],
            stuck: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn cells(&self) -> &[u8] {
        &self.cells
    }

    pub fn read(&self, addr: usize) -> u8 {
        self.cells[addr]
    }

    pub fn write(&mut self, addr: usize, value: u8) {
        self.cells[addr] = self.apply_faults(addr, value);
    }

    pub fn fill(&mut self, mut next: impl FnMut() -> u8) {
        for addr in 0..self.cells.len() {
            self.write(addr, next());
        }
    }

    pub fn add_stuck_bit(&mut self, stuck: StuckBit) {
        self.stuck.push(stuck);
        let addr = usize::from(stuck.cell);
        self.write(addr, self.cells[addr]);
    }

    fn apply_faults(&self, addr: usize, value: u8) -> u8 {
        self.stuck
            .iter()
            .filter(|s| usize::from(s.cell) == addr)
            .fold(value, |v, s| {
                let bit = 1 << s.bit;
                if s.high { v | bit } else { v & !bit }
            })
    }
}
