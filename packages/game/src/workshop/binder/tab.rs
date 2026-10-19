use bevy::prelude::*;
use ld60_calco::Revision;
use ld60_devices::{BoardRev, DeviceKind};

use super::datasheet::ChipSheet;
use crate::device::DeviceName;
use crate::locale::{Localized, Message};
use crate::workshop::run::Run;
use crate::workshop::service::ServiceSheet;

/// A section of the binder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tab {
    /// The service manual of a device, one page per board revision and the
    /// line tests.
    Manual(DeviceKind),
    /// The CalCo datasheet of a chip revision.
    Chip(Revision),
}

/// One page of the binder.
#[derive(Clone, Copy, Debug)]
pub(super) enum Page {
    Service(ServiceSheet),
    LineTests(ServiceSheet),
    /// The page at an index of [`ChipSheet::PAGES`].
    Datasheet(ChipSheet, usize),
}

/// What the binder holds: the tabs and the errata filed in them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Contents {
    pub(super) tabs: Vec<Tab>,
    errata: Vec<Revision>,
}

impl Tab {
    pub(super) fn label(self) -> Localized {
        match self {
            Self::Manual(device) => device.model().into(),
            Self::Chip(revision) => Message::new("binder-tab-chip")
                .with("revision", revision.to_string())
                .into(),
        }
    }
}

impl Page {
    pub(super) fn footer(self) -> Message {
        match self {
            Self::Service(sheet) | Self::LineTests(sheet) => sheet.footer(),
            Self::Datasheet(sheet, _) => sheet.footer(),
        }
    }

    pub(super) fn write(self, page: &mut ChildSpawnerCommands) {
        match self {
            Self::Service(sheet) => sheet.write(page),
            Self::LineTests(sheet) => sheet.write_line_tests(page),
            Self::Datasheet(sheet, index) => ChipSheet::PAGES[index](page, sheet),
        }
    }
}

impl Contents {
    /// The manuals of the devices with a sign, in the order of
    /// [`DeviceKind::ALL`], then the datasheets of their chips.
    pub(super) fn of(run: &Run) -> Self {
        let manuals = DeviceKind::ALL
            .into_iter()
            .filter(|device| run.signs().contains(device))
            .map(Tab::Manual);
        let chips = run.chips().into_iter().map(Tab::Chip);
        Self {
            tabs: manuals.chain(chips).collect(),
            errata: run.errata().to_vec(),
        }
    }

    pub(super) fn pages_of(&self, tab: Tab) -> Vec<Page> {
        match tab {
            Tab::Manual(device) => device
                .boards()
                .iter()
                .map(|board| Page::Service(ServiceSheet::new(board)))
                .chain([Page::LineTests(ServiceSheet::new(
                    device.board(BoardRev::A),
                ))])
                .collect(),
            Tab::Chip(revision) => {
                let sheet = ChipSheet::new(revision, self.errata.contains(&revision));
                (0..ChipSheet::PAGES.len())
                    .map(|index| Page::Datasheet(sheet, index))
                    .collect()
            }
        }
    }

    /// All pages, tab after tab, and the first page of each tab.
    pub(super) fn pages(&self) -> (Vec<Page>, Vec<usize>) {
        let mut pages = Vec::new();
        let mut starts = Vec::new();
        for &tab in &self.tabs {
            starts.push(pages.len());
            pages.extend(self.pages_of(tab));
        }
        (pages, starts)
    }
}
