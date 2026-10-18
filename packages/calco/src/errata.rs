use crate::{ChipInfo, DateCode, Revision};

/// Known chip bugs. The in-game errata sheet is generated from this list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Erratum {
    DsClearsCarryOnZero,
}

impl Erratum {
    pub const ALL: &[Erratum] = &[Erratum::DsClearsCarryOnZero];

    pub fn title(self) -> &'static str {
        match self {
            Self::DsClearsCarryOnZero => "DS clears C when the result is 0",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::DsClearsCarryOnZero => {
                "When DS adds 10 and the result is 0, C is cleared instead of kept. \
                 The borrow to the next digit is lost."
            }
        }
    }

    /// Which chips have the bug, as printed on the errata sheet.
    pub fn affected_chips(self) -> &'static str {
        match self {
            Self::DsClearsCarryOnZero => "Rev A, date codes before 7624",
        }
    }

    pub fn affects(self, chip: ChipInfo) -> bool {
        match self {
            Self::DsClearsCarryOnZero => {
                chip.revision == Revision::A && chip.date_code < DateCode::new(76, 24)
            }
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
