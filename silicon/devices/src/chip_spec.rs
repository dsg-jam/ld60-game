use ld60_machine::Isa;

use crate::IsaKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ChipSpec {
    Calco(ld60_calco::ChipInfo),
}

impl ChipSpec {
    /// The instruction set as this chip implements it. Its mnemonics are the
    /// ones the chip supports.
    pub fn isa(self) -> &'static dyn Isa {
        match self {
            Self::Calco(info) => info.revision.isa(),
        }
    }

    /// The chip family, whatever the revision.
    pub fn isa_kind(self) -> IsaKind {
        match self {
            Self::Calco(_) => IsaKind::Calco,
        }
    }
}
