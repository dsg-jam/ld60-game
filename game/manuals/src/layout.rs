use std::sync::LazyLock;
use std::{error, fmt};

use bevy::prelude::*;
use ld60_calco::Revision;
use ld60_calco::datasheet::Group;
use ld60_devices::{Board, DeviceKind};
use ld60_game_parts::paper::{Paper, Press, Stock};
use ld60_locale::Message;
use serde::Deserialize;

use super::datasheet::ChipSheet;
use crate::service::ServiceSheet;
use ld60_game_device::DeviceName;

/// The layout files in `assets/manuals/`, part of the binary so the pages
/// are there from the start.
macro_rules! layout_file {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!(env!("LD60_ASSETS"), "/manuals/", $name, ".ron")),
        )
    };
}

const FILES: Files = Files {
    manuals: [
        layout_file!("tally2"),
        layout_file!("summa6"),
        layout_file!("lock4"),
        layout_file!("score5"),
        layout_file!("tally1"),
    ],
    datasheet: layout_file!("calco"),
    errata: layout_file!("errata"),
};

static LAYOUTS: LazyLock<Layouts> =
    LazyLock::new(|| Layouts::parse().expect("the manual layouts in the binary are valid"));

/// A layout file by name and contents.
type File = (&'static str, &'static str);

struct Files {
    /// In the order of [`DeviceKind::ALL`].
    manuals: [File; DeviceKind::ALL.len()],
    datasheet: File,
    errata: File,
}

/// The layouts of all manuals.
#[derive(Debug)]
pub(super) struct Layouts {
    /// In the order of [`DeviceKind::ALL`].
    manuals: [ManualFile; DeviceKind::ALL.len()],
    datasheet: ManualFile,
    errata: ManualFile,
}

/// How the pages of one kind of entry are laid out.
#[derive(Deserialize, Debug)]
pub(super) struct ManualFile {
    print: PrintStyle,
    /// The message at the foot of each page.
    footer: String,
    /// Written once per board revision, before `pages`.
    #[serde(default)]
    board_page: Option<PageLayout>,
    #[serde(default)]
    pages: Vec<PageLayout>,
}

#[derive(Deserialize, Clone, Copy, Debug)]
struct PrintStyle {
    press: Press,
    stock: Stock,
}

/// One page, top to bottom.
#[derive(Deserialize, Debug)]
pub(super) struct PageLayout {
    /// What the band at the top says after the entry, and the page's line in
    /// the index.
    #[serde(default)]
    heading: Option<String>,
    blocks: Vec<Block>,
}

/// A piece of a page. Prose names a message. The other blocks write the data
/// of the page's [`Subject`].
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub(super) enum Block {
    Title(String),
    Heading(String),
    Text(String),
    Faded(String),
    /// A line of a list.
    Bullet(String),
    KeyTable,
    DisplayCells,
    Annunciators,
    LineTests,
    Registers,
    Reset,
    Ports,
    Instructions,
    /// The instructions for one purpose, in plain words.
    Group(Group),
    Errata,
}

/// What a page is about. Its messages get its name as arguments.
#[derive(Clone, Copy, Debug)]
pub(super) enum Subject {
    Board(&'static Board),
    Device(DeviceKind),
    Chip(Revision),
}

/// Why the layouts do not load.
#[derive(Debug)]
pub(super) enum LayoutError {
    Parse {
        file: &'static str,
        source: ron::error::SpannedError,
    },
    /// Pages per board revision in a file that is not a device manual.
    BoardPage { file: &'static str },
    /// A data block on a page whose subject does not have that data.
    Misplaced { file: &'static str, block: Block },
}

impl Layouts {
    pub(super) fn get() -> &'static Self {
        &LAYOUTS
    }

    pub(super) fn manual(&self, device: DeviceKind) -> &ManualFile {
        let index = device.index();
        &self.manuals[index]
    }

    pub(super) fn datasheet(&self) -> &ManualFile {
        &self.datasheet
    }

    pub(super) fn errata(&self) -> &ManualFile {
        &self.errata
    }

    fn parse() -> Result<Self, LayoutError> {
        let manuals = FILES.manuals.map(|file| ManualFile::parse(file, true));
        let [tally2, summa6, lock4, score5, tally1] = manuals;
        Ok(Self {
            manuals: [tally2?, summa6?, lock4?, score5?, tally1?],
            datasheet: ManualFile::parse(FILES.datasheet, false)?,
            errata: ManualFile::parse(FILES.errata, false)?,
        })
    }
}

impl ManualFile {
    pub(super) fn press(&self) -> Press {
        self.print.press
    }

    pub(super) fn stock(&self) -> Stock {
        self.print.stock
    }

    pub(super) fn footer(&'static self) -> &'static str {
        &self.footer
    }

    pub(super) fn board_page(&'static self) -> Option<&'static PageLayout> {
        self.board_page.as_ref()
    }

    pub(super) fn pages(&'static self) -> &'static [PageLayout] {
        &self.pages
    }

    /// Reads `file` and checks that each data block is on a page that has
    /// its data. Device manuals have a board on their board page and a
    /// device on the others. Other files are about a chip.
    fn parse((name, text): File, device: bool) -> Result<Self, LayoutError> {
        let parsed: Self =
            ron::from_str(text).map_err(|source| LayoutError::Parse { file: name, source })?;
        if !device && parsed.board_page.is_some() {
            return Err(LayoutError::BoardPage { file: name });
        }
        let board = parsed.board_page.iter().map(|page| (page, Place::Board));
        let others = parsed.pages.iter().map(|page| {
            let place = if device { Place::Device } else { Place::Chip };
            (page, place)
        });
        for (page, place) in board.chain(others) {
            if let Some(block) = page.blocks.iter().find(|block| !block.fits(place)) {
                return Err(LayoutError::Misplaced {
                    file: name,
                    block: block.clone(),
                });
            }
        }
        Ok(parsed)
    }
}

impl PageLayout {
    pub(super) fn heading(&'static self) -> Option<&'static str> {
        self.heading.as_deref()
    }

    /// Whether the page shows anything for `revision`. A page of only
    /// instruction groups the chip does not have stays out.
    pub(super) fn shows(&self, revision: Revision) -> bool {
        self.blocks.iter().any(|block| match *block {
            Block::Group(group) => ChipSheet::new(revision).has(group),
            _ => true,
        })
    }

    pub(super) fn write(&'static self, subject: Subject, page: &mut ChildSpawnerCommands) {
        for block in &self.blocks {
            block.write(subject, page);
        }
    }
}

/// What kind of subject a page has.
#[derive(Clone, Copy, Debug)]
enum Place {
    Board,
    Device,
    Chip,
}

impl Block {
    fn fits(&self, place: Place) -> bool {
        match self {
            Self::Title(_)
            | Self::Heading(_)
            | Self::Text(_)
            | Self::Faded(_)
            | Self::Bullet(_) => true,
            Self::KeyTable | Self::DisplayCells | Self::Annunciators => {
                matches!(place, Place::Board)
            }
            Self::LineTests => matches!(place, Place::Device),
            Self::Registers
            | Self::Reset
            | Self::Ports
            | Self::Instructions
            | Self::Group(_)
            | Self::Errata => matches!(place, Place::Chip),
        }
    }

    fn write(&'static self, subject: Subject, page: &mut ChildSpawnerCommands) {
        let message = |id: &'static String| subject.args(Message::new(id.as_str()));
        match (self, subject) {
            (Self::Title(id), _) => {
                page.spawn(Paper::title(message(id)));
            }
            (Self::Heading(id), _) => {
                page.spawn(Paper::heading(message(id)));
            }
            (Self::Text(id), _) => {
                page.spawn(Paper::text(message(id)));
            }
            (Self::Faded(id), _) => {
                page.spawn(Paper::faded(message(id)));
            }
            (Self::Bullet(id), _) => {
                page.spawn(Paper::text(
                    Message::new("paper-bullet").with("text", message(id)),
                ));
            }
            (Self::KeyTable, Subject::Board(board)) => ServiceSheet::new(board).key_codes(page),
            (Self::DisplayCells, Subject::Board(board)) => {
                ServiceSheet::new(board).display_cells(page);
            }
            (Self::Annunciators, Subject::Board(board)) => {
                ServiceSheet::new(board).annunciators(page);
            }
            (Self::LineTests, Subject::Device(kind)) => ServiceSheet::line_tests(kind, page),
            (Self::Registers, Subject::Chip(revision)) => ChipSheet::new(revision).registers(page),
            (Self::Reset, Subject::Chip(revision)) => ChipSheet::new(revision).reset(page),
            (Self::Ports, Subject::Chip(revision)) => ChipSheet::new(revision).ports(page),
            (Self::Instructions, Subject::Chip(revision)) => {
                ChipSheet::new(revision).instructions(page);
            }
            (&Self::Group(group), Subject::Chip(revision)) => {
                ChipSheet::new(revision).group(group, page);
            }
            (Self::Errata, Subject::Chip(revision)) => ChipSheet::new(revision).errata(page),
            // `ManualFile::parse` refuses these.
            _ => {}
        }
    }
}

impl Subject {
    /// `message` with the names of the subject as its arguments.
    pub(super) fn args(self, message: Message) -> Message {
        match self {
            Self::Board(board) => message
                .with("device", board.kind.model())
                .with("board", board.name()),
            Self::Device(kind) => message.with("device", kind.model()),
            Self::Chip(revision) => message.with("revision", revision.to_string()),
        }
    }
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { file, .. } => write!(f, "the manual layout {file} does not parse"),
            Self::BoardPage { file } => {
                write!(f, "the manual layout {file} has board pages but no device")
            }
            Self::Misplaced { file, block } => {
                write!(
                    f,
                    "the manual layout {file} has {block:?} on a page without its data"
                )
            }
        }
    }
}

impl error::Error for LayoutError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Parse { source, .. } => Some(source),
            Self::BoardPage { .. } | Self::Misplaced { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contents::Entry;

    #[test]
    fn every_layout_loads() {
        Layouts::parse().unwrap();
    }

    #[test]
    fn pages_keep_their_numbers() {
        // Saves pin pages by their number in the entry.
        for device in DeviceKind::ALL {
            assert_eq!(Entry::Manual(device).pages().len(), 4, "{device:?}");
        }
        for revision in Revision::ALL {
            let pages = if revision == Revision::A { 4 } else { 5 };
            assert_eq!(Entry::Chip(revision).pages().len(), pages, "{revision}");
            assert_eq!(Entry::Errata(revision).pages().len(), 1);
        }
    }

    #[test]
    fn data_without_its_subject_is_refused() {
        let file = |text| ManualFile::parse(("test", text), false);
        let chip = r#"(print: (press: Offset, stock: Bright), footer: "x", pages: [(blocks: [KeyTable])])"#;
        assert!(matches!(
            file(chip),
            Err(LayoutError::Misplaced {
                block: Block::KeyTable,
                ..
            })
        ));
        let boards = r#"(print: (press: Offset, stock: Bright), footer: "x", board_page: Some((blocks: [])))"#;
        assert!(matches!(file(boards), Err(LayoutError::BoardPage { .. })));
        assert!(matches!(file("("), Err(LayoutError::Parse { .. })));
    }
}
