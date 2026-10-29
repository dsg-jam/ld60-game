use bevy::prelude::*;
use ld60_calco::Revision;
use ld60_devices::{ChipSpec, DeviceKind};
use ld60_game_core::parts::paper::Print;
use ld60_levels::Level;
use ld60_locale::{Localized, Message};
use serde::{Deserialize, Serialize};

use super::layout::{Layouts, ManualFile, PageLayout, Subject};
use super::pins::Sheet;
use crate::workshop::run::Run;
use crate::workshop::tutorial::TutorialNote;
use ld60_game_device::DeviceName;

const GADGETS: Color = Color::srgb_u8(0xe8, 0xcc, 0x6a);
const CHIPS: Color = Color::srgb_u8(0x9a, 0xc0, 0xd8);
const ERRATA: Color = Color::srgb_u8(0xe0, 0x9a, 0x8a);
const NOTES: Color = Color::srgb_u8(0xa8, 0xc8, 0x8a);

/// A section of the manuals file, behind a divider of its own color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Section {
    Gadgets,
    Chips,
    Errata,
    Notes,
}

/// One thing filed in a [`Section`], with a line in the index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) enum Entry {
    /// The service manual of a device: one page per board revision and the
    /// line tests.
    Manual(DeviceKind),
    /// The CalCo datasheet of a chip revision.
    Chip(Revision),
    /// The errata page of a chip revision.
    Errata(Revision),
    Note(TutorialNote),
}

/// One page of the manuals file.
#[derive(Clone, Copy, Debug)]
pub(super) enum Page {
    Index,
    /// A page from the layout files in `assets/manuals/`.
    Laid {
        file: &'static ManualFile,
        layout: &'static PageLayout,
        subject: Subject,
    },
    Note(TutorialNote),
}

/// What the manuals file holds: the index, then the entries section after
/// section.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Contents {
    entries: Vec<Entry>,
}

impl Section {
    pub(super) const ALL: [Self; 4] = [Self::Gadgets, Self::Chips, Self::Errata, Self::Notes];

    pub(super) fn label(self) -> Message {
        Message::new(match self {
            Self::Gadgets => "manuals-gadgets",
            Self::Chips => "manuals-chips",
            Self::Errata => "manuals-errata",
            Self::Notes => "manuals-notes",
        })
    }

    /// The color of its divider.
    pub(super) fn color(self) -> Color {
        match self {
            Self::Gadgets => GADGETS,
            Self::Chips => CHIPS,
            Self::Errata => ERRATA,
            Self::Notes => NOTES,
        }
    }

    /// What the index says while nothing is filed in the section.
    pub(super) fn empty(self) -> Option<Message> {
        match self {
            Self::Gadgets | Self::Chips => None,
            Self::Errata => Some(Message::new("manuals-empty-errata")),
            Self::Notes => Some(Message::new("manuals-empty-notes")),
        }
    }
}

impl Entry {
    pub(super) fn section(self) -> Section {
        match self {
            Self::Manual(_) => Section::Gadgets,
            Self::Chip(_) => Section::Chips,
            Self::Errata(_) => Section::Errata,
            Self::Note(_) => Section::Notes,
        }
    }

    pub(super) fn label(self) -> Localized {
        match self {
            Self::Manual(device) => device.model().into(),
            Self::Chip(revision) | Self::Errata(revision) => Message::new("manuals-chip")
                .with("revision", revision.to_string())
                .into(),
            Self::Note(note) => note.label().into(),
        }
    }

    /// How the shop's copy was printed, and on what paper. Each manual is a
    /// print run of its own.
    pub(super) fn print(self) -> Print {
        let layouts = Layouts::get();
        let (file, run) = match self {
            Self::Manual(device) => {
                let file = layouts.manual(device);
                (file, file.press().run(device))
            }
            Self::Chip(revision) => {
                let file = layouts.datasheet();
                (file, file.press().run(("datasheet", revision)))
            }
            Self::Errata(revision) => {
                let file = layouts.errata();
                (file, file.press().run(("errata", revision)))
            }
            Self::Note(note) => return note.print(),
        };
        run.on(file.stock())
    }

    pub(super) fn pages(self) -> Vec<Page> {
        let layouts = Layouts::get();
        let laid = |file: &'static ManualFile, subject| {
            file.pages().iter().map(move |layout| Page::Laid {
                file,
                layout,
                subject,
            })
        };
        match self {
            Self::Manual(device) => {
                let file = layouts.manual(device);
                let boards = file.board_page().into_iter().flat_map(|layout| {
                    device.boards().iter().map(move |board| Page::Laid {
                        file,
                        layout,
                        subject: Subject::Board(board),
                    })
                });
                boards.chain(laid(file, Subject::Device(device))).collect()
            }
            Self::Chip(revision) => {
                let file = layouts.datasheet();
                file.pages()
                    .iter()
                    .filter(|layout| layout.shows(revision))
                    .map(|layout| Page::Laid {
                        file,
                        layout,
                        subject: Subject::Chip(revision),
                    })
                    .collect()
            }
            Self::Errata(revision) => laid(layouts.errata(), Subject::Chip(revision)).collect(),
            Self::Note(note) => vec![Page::Note(note)],
        }
    }
}

impl Page {
    pub(super) fn footer(self) -> Message {
        match self {
            Self::Index => Message::new("manuals-footer"),
            Self::Laid { file, subject, .. } => subject.args(Message::new(file.footer())),
            Self::Note(note) => note.footer(),
        }
    }

    /// What the band at the top of the page says after the entry, for the
    /// pages with a heading in their layout.
    pub(super) fn heading(self) -> Option<Localized> {
        match self {
            Self::Laid {
                layout, subject, ..
            } => Some(subject.args(Message::new(layout.heading()?)).into()),
            Self::Index | Self::Note(_) => None,
        }
    }

    /// How the index names the page under its entry: the board of a board
    /// page, otherwise the heading.
    pub(super) fn short_name(self) -> Option<Localized> {
        match self {
            Self::Laid {
                subject: Subject::Board(board),
                ..
            } => Some(Localized::verbatim(board.name())),
            Self::Laid { .. } => self.heading(),
            Self::Index | Self::Note(_) => None,
        }
    }

    /// Writes everything but the index, which needs the [`Contents`].
    pub(super) fn write(self, page: &mut ChildSpawnerCommands) {
        match self {
            Self::Index => {}
            Self::Laid {
                layout, subject, ..
            } => layout.write(subject, page),
            Self::Note(note) => note.write(page),
        }
    }
}

impl Contents {
    /// The manuals the shop has in the order of [`DeviceKind::ALL`], the
    /// datasheets of their chips, the errata the shop bought and the notes
    /// it filed.
    pub(super) fn of(run: &Run) -> Self {
        let manuals = DeviceKind::ALL
            .into_iter()
            .filter(|&device| run.has_manual(device))
            .map(Entry::Manual);
        let chips = run.chips().into_iter().map(Entry::Chip);
        let errata = Revision::ALL
            .into_iter()
            .filter(|revision| run.errata().contains(revision))
            .map(Entry::Errata);
        let mut steps = run.notes().to_vec();
        steps.sort_unstable();
        let notes = steps
            .into_iter()
            .filter_map(TutorialNote::new)
            .map(Entry::Note);
        Self {
            entries: manuals.chain(chips).chain(errata).chain(notes).collect(),
        }
    }

    pub(super) fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// All pages, the index first, each as a [`Sheet`] of its entry.
    pub(super) fn pages(&self) -> Vec<(Page, Option<Sheet>)> {
        let entries = self.entries.iter().flat_map(|&entry| {
            entry
                .pages()
                .into_iter()
                .enumerate()
                .map(move |(index, page)| (page, Some(Sheet::new(entry, index))))
        });
        [(Page::Index, None)].into_iter().chain(entries).collect()
    }

    /// The first page of `entry`, counted from 0.
    pub(super) fn start_of(&self, entry: Entry) -> Option<usize> {
        let index = self.entries.iter().position(|&other| other == entry)?;
        Some(self.starts()[index + 1])
    }

    /// The first page of the first entry in `section`.
    pub(super) fn start_of_section(&self, section: Section) -> Option<usize> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.section() == section)?;
        self.start_of(*entry)
    }

    /// Page `page` as a [`Sheet`] of its entry, or `None` for the index.
    pub(super) fn sheet_at(&self, page: usize) -> Option<Sheet> {
        let entry = self.entry_at(page)?;
        let start = self.start_of(entry)?;
        Some(Sheet::new(entry, page - start))
    }

    /// The entry that `page` belongs to, or `None` for the index.
    pub(super) fn entry_at(&self, page: usize) -> Option<Entry> {
        let index = self.starts().iter().rposition(|&start| start <= page)?;
        index.checked_sub(1).map(|index| self.entries[index])
    }

    /// The page about a job on `level`: the page about its board in the
    /// manual, or the datasheet of its chip if the manual is missing.
    pub(super) fn page_for(&self, level: &Level) -> Option<usize> {
        let ChipSpec::Calco(chip) = level.chip;
        match self.start_of(Entry::Manual(level.device)) {
            Some(start) => Some(start + level.board as usize),
            None => self.start_of(Entry::Chip(chip.revision)),
        }
    }

    /// The first page of the entry after or before the one with `page`. The
    /// index comes before the first entry.
    pub(super) fn next_entry(&self, page: usize, forward: bool) -> Option<usize> {
        let starts = self.starts();
        let index = starts.iter().rposition(|&start| start <= page)?;
        let next = if forward {
            index + 1
        } else {
            index.checked_sub(1)?
        };
        starts.get(next).copied()
    }

    /// The first page of the index and of each entry.
    fn starts(&self) -> Vec<usize> {
        let mut starts = vec![0];
        let mut next = 1;
        for entry in &self.entries {
            starts.push(next);
            next += entry.pages().len();
        }
        starts
    }
}
