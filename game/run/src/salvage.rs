use ld60_calco::{Erratum, Revision};
use ld60_devices::DeviceKind;
use ld60_locale::Message;
use serde::{Deserialize, Serialize};

use crate::{ComputerTool, Run};

/// Something the scrap trader found, for sale in the salvage crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Find {
    /// A dead unit with its human manual and the datasheet of the chip it
    /// shipped with. Buying it hangs its sign in the window and files both
    /// in the manuals file.
    Device(DeviceKind),
    Tool(ComputerTool),
    /// Loose errata pages for one chip revision.
    Errata(Revision),
    /// A loose datasheet for one chip revision.
    Datasheet(Revision),
    /// The paper that makes the shop's building the player's. The landlord
    /// left it with the trader.
    Deed,
}

impl Find {
    /// Everything the trader can ever find.
    pub fn all() -> impl Iterator<Item = Self> {
        let devices = DeviceKind::SERVICED.into_iter().map(Self::Device);
        let tools = ComputerTool::ALL.into_iter().map(Self::Tool);
        let errata = Revision::ALL
            .into_iter()
            .filter(|&revision| {
                Erratum::ALL
                    .iter()
                    .any(|erratum| erratum.revision() == revision)
            })
            .map(Self::Errata);
        let datasheets = Revision::ALL.into_iter().map(Self::Datasheet);
        devices
            .chain(tools)
            .chain(errata)
            .chain(datasheets)
            .chain([Self::Deed])
    }

    /// The chip revision whose datasheet buying this files.
    pub fn datasheet(self) -> Option<Revision> {
        match self {
            Self::Device(device) => Some(Run::shipped_chip(device)),
            Self::Datasheet(revision) => Some(revision),
            Self::Tool(_) | Self::Errata(_) | Self::Deed => None,
        }
    }

    pub const fn price(self) -> u32 {
        match self {
            Self::Device(DeviceKind::Tally1) => 8,
            Self::Device(DeviceKind::Tally2) => 12,
            Self::Device(DeviceKind::Summa6) => 20,
            Self::Device(DeviceKind::Lock4) => 36,
            Self::Device(DeviceKind::Score5) => 28,
            Self::Tool(ComputerTool::TestBench) => 14,
            Self::Tool(ComputerTool::Debugger) => 22,
            Self::Tool(ComputerTool::Trace) => 16,
            Self::Tool(ComputerTool::Breakpoints) => 10,
            Self::Errata(_) => 6,
            Self::Datasheet(_) => 8,
            Self::Deed => 150,
        }
    }

    /// What the trader thinks it is, as written on its tag.
    pub fn guess(self) -> Message {
        Message::new(match self {
            Self::Device(DeviceKind::Tally1) => "salvage-guess-tally1",
            Self::Device(DeviceKind::Tally2) => "salvage-guess-tally2",
            Self::Device(DeviceKind::Summa6) => "salvage-guess-summa6",
            Self::Device(DeviceKind::Lock4) => "salvage-guess-lock4",
            Self::Device(DeviceKind::Score5) => "salvage-guess-score5",
            Self::Tool(ComputerTool::TestBench) => "salvage-guess-test-bench",
            Self::Tool(ComputerTool::Debugger) => "salvage-guess-debugger",
            Self::Tool(ComputerTool::Trace) => "salvage-guess-trace",
            Self::Tool(ComputerTool::Breakpoints) => "salvage-guess-breakpoints",
            Self::Errata(_) => "salvage-guess-errata",
            Self::Datasheet(_) => "salvage-guess-datasheet",
            Self::Deed => "salvage-guess-deed",
        })
    }

    /// What else the tag says about it.
    pub fn note(self) -> Message {
        match self {
            Self::Device(_) => Message::new("salvage-note-device"),
            Self::Tool(_) => Message::new("salvage-note-tool"),
            Self::Deed => Message::new("salvage-note-deed"),
            Self::Errata(revision) => {
                Message::new("salvage-note-errata").with("revision", revision.to_string())
            }
            Self::Datasheet(revision) => {
                Message::new("salvage-note-datasheet").with("revision", revision.to_string())
            }
        }
    }
}
