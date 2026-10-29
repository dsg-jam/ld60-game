use std::fmt;

use crate::Annunciator;

/// What the display shows. Digit values above 9 light invalid segments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Readout {
    pub digits: Vec<u8>,
    pub minus: bool,
    pub error: bool,
    pub open: bool,
    pub shut: bool,
}

impl fmt::Display for Readout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.error {
            return f.write_str(Annunciator::Error.mark());
        }
        if self.minus {
            f.write_str(Annunciator::Minus.mark())?;
        }
        for &d in &self.digits {
            let c = char::from_digit(u32::from(d), 10).unwrap_or('?');
            write!(f, "{c}")?;
        }
        for (lit, annunciator) in [
            (self.open, Annunciator::Open),
            (self.shut, Annunciator::Shut),
        ] {
            if lit {
                write!(f, " {}", annunciator.mark())?;
            }
        }
        Ok(())
    }
}
