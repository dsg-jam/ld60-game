use crate::{ChipInfo, DateCode, Revision};

/// Known chip bugs. The in-game errata sheet is generated from this list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Erratum {
    DsClearsCarryOnZero,
}

impl Erratum {
    pub const ALL: &[Erratum] = &[Erratum::DsClearsCarryOnZero];

    /// A stable name of the bug, like `ds-clears-carry-on-zero`.
    pub fn id(self) -> &'static str {
        match self {
            Self::DsClearsCarryOnZero => "ds-clears-carry-on-zero",
        }
    }

    pub fn affects(self, chip: ChipInfo) -> bool {
        match self {
            Self::DsClearsCarryOnZero => {
                chip.revision == self.revision() && chip.date_code < DateCode::new(76, 24)
            }
        }
    }

    /// The revision with the bug. Only some of its chips may be affected.
    pub fn revision(self) -> Revision {
        match self {
            Self::DsClearsCarryOnZero => Revision::B,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Errata(Vec<Erratum>);

impl Errata {
    pub fn for_chip(chip: ChipInfo) -> Self {
        Self(
            Erratum::ALL
                .iter()
                .copied()
                .filter(|e| e.affects(chip))
                .collect(),
        )
    }

    pub fn has(&self, erratum: Erratum) -> bool {
        self.0.contains(&erratum)
    }

    pub fn iter(&self) -> impl Iterator<Item = Erratum> + '_ {
        self.0.iter().copied()
    }
}
