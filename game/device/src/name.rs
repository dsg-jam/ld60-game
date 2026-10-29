use std::fmt;

use ld60_devices::{BoardRev, DeviceKind};
use ld60_locale::Message;

/// What a device kind is called, in the player's language.
pub trait DeviceName: Sized {
    /// Like `TALLY 2`.
    fn model(self) -> Message {
        self.name().attribute("model")
    }

    /// Like `Tally 2 counter`.
    fn name(self) -> Message;

    /// As the sticker on the device prints it.
    fn plate(self) -> Message {
        self.name().attribute("plate")
    }
}

/// The letter of a board revision, like the `B` in `6B`.
#[derive(Clone, Copy, Debug)]
pub struct BoardLetter(pub BoardRev);

impl DeviceName for DeviceKind {
    fn name(self) -> Message {
        Message::new(match self {
            Self::Tally2 => "device-tally2",
            Self::Summa6 => "device-summa6",
            Self::Lock4 => "device-lock4",
            Self::Score5 => "device-score5",
            Self::Tally1 => "device-tally1",
        })
    }
}

impl fmt::Display for BoardLetter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            BoardRev::A => "A",
            BoardRev::B => "B",
            BoardRev::C => "C",
        })
    }
}
