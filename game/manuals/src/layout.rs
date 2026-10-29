use std::sync::Arc;
use std::{error, fmt};

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    platform::collections::HashMap,
    prelude::*,
};
use ld60_calco::Revision;
use ld60_calco::datasheet::Group;
use ld60_devices::{Board, DeviceKind};
use ld60_game_device::DeviceName;
use ld60_game_parts::paper::{Paper, Press, Stock};
use ld60_locale::Message;
use serde::Deserialize;

use super::datasheet::ChipSheet;
use crate::service::ServiceSheet;

const DATASHEET: &str = "calco";
const ERRATA: &str = "errata";

pub(super) struct LayoutsPlugin;

/// The layouts of all manuals, from the files in `assets/manuals/`, once
/// they are all loaded.
#[derive(Resource, Debug)]
pub struct Layouts {
    manuals: HashMap<DeviceKind, Arc<ManualFile>>,
    datasheet: Arc<ManualFile>,
    errata: Arc<ManualFile>,
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

/// Which page of a [`ManualFile`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum At {
    BoardPage,
    Page(usize),
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Subject {
    Board(&'static Board),
    Device(DeviceKind),
    Chip(Revision),
}

/// A layout file as loaded, before it is checked against what it lays out.
#[derive(Asset, TypePath, Debug)]
struct LayoutFile(Arc<ManualFile>);

/// The layout files, loading or loaded.
#[derive(Resource, Debug)]
struct LayoutFiles {
    manuals: Vec<(DeviceKind, Handle<LayoutFile>)>,
    datasheet: Handle<LayoutFile>,
    errata: Handle<LayoutFile>,
}

#[derive(Default, TypePath)]
struct LayoutLoader;

/// Why the layouts do not load.
#[derive(Debug)]
pub(super) enum LayoutError {
    Parse {
        file: String,
        source: Box<ron::error::SpannedError>,
    },
    /// Pages per board revision in a file that is not a device manual.
    BoardPage { file: String },
    /// A data block on a page whose subject does not have that data.
    Misplaced { file: String, block: Block },
}

impl Plugin for LayoutsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<LayoutFile>()
            .init_asset_loader::<LayoutLoader>()
            .init_resource::<LayoutFiles>()
            .add_systems(Update, Layouts::follow_files);
    }
}

impl Layouts {
    /// Checks each file against what it lays out.
    fn new(
        manuals: impl IntoIterator<Item = (DeviceKind, Arc<ManualFile>)>,
        datasheet: Arc<ManualFile>,
        errata: Arc<ManualFile>,
    ) -> Result<Self, LayoutError> {
        let manuals: HashMap<_, _> = manuals.into_iter().collect();
        for (device, file) in &manuals {
            file.check(device.id(), true)?;
        }
        datasheet.check(DATASHEET, false)?;
        errata.check(ERRATA, false)?;
        Ok(Self {
            manuals,
            datasheet,
            errata,
        })
    }

    pub(super) fn manual(&self, device: DeviceKind) -> &Arc<ManualFile> {
        &self.manuals[&device]
    }

    pub(super) fn datasheet(&self) -> &Arc<ManualFile> {
        &self.datasheet
    }

    pub(super) fn errata(&self) -> &Arc<ManualFile> {
        &self.errata
    }

    /// Takes the layouts once every file is loaded, and again when one
    /// changes.
    fn follow_files(
        mut events: MessageReader<AssetEvent<LayoutFile>>,
        files: Res<Assets<LayoutFile>>,
        handles: Res<LayoutFiles>,
        mut commands: Commands,
    ) {
        let updated = events.read().any(|event| {
            matches!(
                event,
                AssetEvent::Added { .. } | AssetEvent::Modified { .. }
            )
        });
        if !updated {
            return;
        }
        let file = |handle: &Handle<LayoutFile>| Some(files.get(handle)?.0.clone());
        let Some(manuals) = handles
            .manuals
            .iter()
            .map(|(device, handle)| Some((*device, file(handle)?)))
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        let (Some(datasheet), Some(errata)) = (file(&handles.datasheet), file(&handles.errata))
        else {
            return;
        };
        match Self::new(manuals, datasheet, errata) {
            Ok(layouts) => commands.insert_resource(layouts),
            Err(err) => error!(
                error = &err as &dyn error::Error,
                "manual layouts do not load"
            ),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Layouts {
    /// The layouts in the repository, for tests.
    pub fn on_disk() -> Self {
        let read = |name: &str| {
            let path = format!("{}/manuals/{name}.ron", env!("LD60_ASSETS"));
            Arc::new(ManualFile::parse(name, &std::fs::read_to_string(path).unwrap()).unwrap())
        };
        Self::new(
            DeviceKind::ALL.map(|device| (device, read(device.id()))),
            read(DATASHEET),
            read(ERRATA),
        )
        .unwrap()
    }
}

impl FromWorld for LayoutFiles {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        let load = |name: &str| assets.load(format!("manuals/{name}.ron"));
        Self {
            manuals: DeviceKind::ALL
                .into_iter()
                .map(|device| (device, load(device.id())))
                .collect(),
            datasheet: load(DATASHEET),
            errata: load(ERRATA),
        }
    }
}

impl AssetLoader for LayoutLoader {
    type Asset = LayoutFile;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<LayoutFile, BevyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let name = load_context.path().to_string();
        let file = ManualFile::parse(&name, &String::from_utf8(bytes)?)?;
        Ok(LayoutFile(Arc::new(file)))
    }

    fn extensions(&self) -> &[&str] {
        &[]
    }
}

impl ManualFile {
    pub(super) fn press(&self) -> Press {
        self.print.press
    }

    pub(super) fn stock(&self) -> Stock {
        self.print.stock
    }

    pub(super) fn footer(&self) -> &str {
        &self.footer
    }

    pub(super) fn board_page(&self) -> Option<&PageLayout> {
        self.board_page.as_ref()
    }

    pub(super) fn pages(&self) -> &[PageLayout] {
        &self.pages
    }

    pub(super) fn page(&self, at: At) -> Option<&PageLayout> {
        match at {
            At::BoardPage => self.board_page(),
            At::Page(index) => self.pages.get(index),
        }
    }

    fn parse(name: &str, text: &str) -> Result<Self, LayoutError> {
        ron::from_str(text).map_err(|source| LayoutError::Parse {
            file: name.to_owned(),
            source: Box::new(source),
        })
    }

    /// Checks that each data block is on a page that has its data. Device
    /// manuals have a board on their board page and a device on the others.
    /// Other files are about a chip.
    fn check(&self, name: &str, device: bool) -> Result<(), LayoutError> {
        if !device && self.board_page.is_some() {
            return Err(LayoutError::BoardPage {
                file: name.to_owned(),
            });
        }
        let board = self.board_page.iter().map(|page| (page, Place::Board));
        let others = self.pages.iter().map(|page| {
            let place = if device { Place::Device } else { Place::Chip };
            (page, place)
        });
        for (page, place) in board.chain(others) {
            if let Some(block) = page.blocks.iter().find(|block| !block.fits(place)) {
                return Err(LayoutError::Misplaced {
                    file: name.to_owned(),
                    block: block.clone(),
                });
            }
        }
        Ok(())
    }
}

impl PageLayout {
    pub(super) fn heading(&self) -> Option<&str> {
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

    pub(super) fn write(&self, subject: Subject, page: &mut ChildSpawnerCommands) {
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

    fn write(&self, subject: Subject, page: &mut ChildSpawnerCommands) {
        let message = |id: &String| subject.args(Message::new(id.clone()));
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
            // `ManualFile::check` refuses these.
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
    fn pages_keep_their_numbers() {
        // Saves pin pages by their number in the entry.
        let layouts = Layouts::on_disk();
        for device in DeviceKind::ALL {
            let pages = Entry::Manual(device).pages(&layouts).len();
            assert_eq!(pages, 4, "{device:?}");
        }
        for revision in Revision::ALL {
            let pages = if revision == Revision::A { 4 } else { 5 };
            assert_eq!(Entry::Chip(revision).pages(&layouts).len(), pages);
            assert_eq!(Entry::Errata(revision).pages(&layouts).len(), 1);
        }
    }

    #[test]
    fn data_without_its_subject_is_refused() {
        let file = |text| ManualFile::parse("test", text)?.check("test", false);
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
