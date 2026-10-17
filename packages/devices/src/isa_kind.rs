use machine::Isa;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IsaKind {
    Calco,
}

impl IsaKind {
    pub fn isa(self) -> &'static dyn Isa {
        match self {
            Self::Calco => &calco::Calco,
        }
    }
}
