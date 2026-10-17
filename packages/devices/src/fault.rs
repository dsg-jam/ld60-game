use machine::StuckBit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Fault {
    StuckRamBit(StuckBit),
    FadedRomBits { addr: u16, mask: u16 },
}
