use crate::IsaKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ChipSpec {
    Calco(calco::ChipInfo),
}

impl ChipSpec {
    pub fn isa(self) -> IsaKind {
        match self {
            Self::Calco(_) => IsaKind::Calco,
        }
    }
}
