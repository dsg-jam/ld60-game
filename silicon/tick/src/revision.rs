use crate::Op;

/// Chip revision. Later revisions have more ROM and more instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Revision {
    /// Four words of ROM. Steps and waits.
    A,
    /// Eight words of ROM, `HOME`, and a key that sends the program to
    /// [`Revision::KEY_VECTOR`].
    B,
}

impl Revision {
    pub const ALL: [Self; 2] = [Self::A, Self::B];
    /// Where a key press sends the program on chips with a key.
    pub const KEY_VECTOR: u8 = 7;

    /// Decodes like this chip does. Unknown patterns come back as
    /// [`Op::Reserved`].
    pub fn decode(self, word: u8) -> Op {
        match word & 0xF {
            Op::NOP => Op::Nop,
            Op::STEP => Op::Step,
            Op::WAIT => Op::Wait,
            Op::HOME if self.has_key() => Op::Home,
            other => Op::Reserved(other),
        }
    }

    pub fn has_key(self) -> bool {
        self == Self::B
    }

    /// Words of ROM.
    pub fn rom_len(self) -> usize {
        match self {
            Self::A => 4,
            Self::B => 8,
        }
    }
}
