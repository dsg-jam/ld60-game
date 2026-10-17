use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Revision {
    A,
    B,
}

impl Revision {
    /// Value returned by `IN REV`.
    pub fn id(self) -> u8 {
        match self {
            Self::A => 0x10,
            Self::B => 0x20,
        }
    }
}

/// Year and week of manufacture, printed on the chip as `YYWW`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct DateCode(u16);

impl DateCode {
    pub const fn new(year: u8, week: u8) -> Self {
        assert!(year < 100 && week >= 1 && week <= 53, "invalid date code");
        Self(year as u16 * 100 + week as u16)
    }

    pub fn year(self) -> u8 {
        (self.0 / 100) as u8
    }

    pub fn week(self) -> u8 {
        (self.0 % 100) as u8
    }
}

impl fmt::Display for DateCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChipInfo {
    pub revision: Revision,
    pub date_code: DateCode,
}
