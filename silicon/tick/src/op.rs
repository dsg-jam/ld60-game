/// One instruction. Every word decodes to one, see [`Revision::decode`].
///
/// [`Revision::decode`]: crate::Revision::decode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    /// Does nothing. An erased word reads as this.
    Nop,
    /// Moves the hand one mark on.
    Step,
    /// Sleeps until the next tick.
    Wait,
    /// Moves the hand back to the top.
    Home,
    /// A pattern this chip does not know. It does nothing.
    Reserved(u8),
}

impl Op {
    pub const NOP: u8 = 0b1111;
    pub const STEP: u8 = 0b0001;
    pub const WAIT: u8 = 0b0010;
    pub const HOME: u8 = 0b0100;

    /// The word that encodes it.
    pub fn word(self) -> u8 {
        match self {
            Self::Nop => Self::NOP,
            Self::Step => Self::STEP,
            Self::Wait => Self::WAIT,
            Self::Home => Self::HOME,
            Self::Reserved(word) => word,
        }
    }
}
