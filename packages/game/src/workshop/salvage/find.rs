use ld60_calco::{Erratum, Revision};
use ld60_devices::DeviceKind;
use serde::{Deserialize, Serialize};

use crate::locale::Message;
use crate::workshop::run::ComputerTool;

/// Something the scrap trader found, for sale in the salvage crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Find {
    /// A dead unit with its human manual. Buying it hangs its sign in the
    /// window and files the manual in the binder.
    Device(DeviceKind),
    Tool(ComputerTool),
    /// Loose errata pages for one chip revision.
    Errata(Revision),
}

impl Find {
    /// Everything the trader can ever find.
    pub(crate) fn all() -> impl Iterator<Item = Self> {
        let devices = DeviceKind::ALL.into_iter().map(Self::Device);
        let tools = ComputerTool::ALL.into_iter().map(Self::Tool);
        let errata = Revision::ALL
            .into_iter()
            .filter(|&revision| {
                Erratum::ALL
                    .iter()
                    .any(|erratum| erratum.revision() == revision)
            })
            .map(Self::Errata);
        devices.chain(tools).chain(errata)
    }

    pub(crate) const fn price(self) -> u32 {
        match self {
            Self::Device(DeviceKind::Tally2) => 12,
            Self::Device(DeviceKind::Summa6) => 30,
            Self::Device(DeviceKind::Lock4) => 45,
            Self::Tool(ComputerTool::TestBench) => 14,
            Self::Tool(ComputerTool::Debugger) => 22,
            Self::Tool(ComputerTool::Trace) => 16,
            Self::Tool(ComputerTool::Breakpoints) => 10,
            Self::Errata(_) => 6,
        }
    }

    /// What the trader thinks it is, as written on its tag.
    pub(crate) fn guess(self) -> Message {
        Message::new(match self {
            Self::Device(DeviceKind::Tally2) => "salvage-guess-tally2",
            Self::Device(DeviceKind::Summa6) => "salvage-guess-summa6",
            Self::Device(DeviceKind::Lock4) => "salvage-guess-lock4",
            Self::Tool(ComputerTool::TestBench) => "salvage-guess-test-bench",
            Self::Tool(ComputerTool::Debugger) => "salvage-guess-debugger",
            Self::Tool(ComputerTool::Trace) => "salvage-guess-trace",
            Self::Tool(ComputerTool::Breakpoints) => "salvage-guess-breakpoints",
            Self::Errata(_) => "salvage-guess-errata",
        })
    }

    /// What else the tag says about it.
    pub(crate) fn note(self) -> Message {
        match self {
            Self::Device(_) => Message::new("salvage-note-device"),
            Self::Tool(_) => Message::new("salvage-note-tool"),
            Self::Errata(revision) => {
                Message::new("salvage-note-errata").with("revision", revision.to_string())
            }
        }
    }
}
