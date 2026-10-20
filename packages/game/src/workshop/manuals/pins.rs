use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::contents::{Contents, Entry, Page};
use crate::parts::paper::{Paper, Print};
use crate::parts::screen::UiCanvas;
use crate::workshop::run::Run;
use crate::workshop::tutorial::TutorialNote;

/// One page of the manuals file, named by its entry so it stays the same
/// page when more is filed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Sheet {
    entry: Entry,
    /// Counted from 0 within the entry.
    page: usize,
}

/// The sheets taken out of the manuals file and pinned to the monitor of
/// the shop computer, at most [`Pins::MAX`] of them, the oldest first. A
/// pinned sheet is not in the file until it is unpinned.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Vec<Sheet>", into = "Vec<Sheet>")]
pub(crate) struct Pins(Vec<Sheet>);

impl Sheet {
    pub(super) fn new(entry: Entry, page: usize) -> Self {
        Self { entry, page }
    }

    pub(super) fn entry(self) -> Entry {
        self.entry
    }

    /// The page, if the entry has it.
    pub(super) fn page(self) -> Option<Page> {
        if let Entry::Note(note) = self.entry {
            TutorialNote::new(note.0)?;
        }
        self.entry.pages().get(self.page).copied()
    }

    /// Spawns the sheet on `canvas`, with the band of its section at the
    /// top.
    pub(crate) fn spawn(self, commands: &mut Commands, canvas: &UiCanvas) -> Option<Entity> {
        let page = self.page()?;
        let spawned = Paper::spawn_single(commands, canvas, page.footer(), |content| {
            content.spawn(self.band());
            page.write(content);
        });
        commands.entity(spawned).insert(self.print());
        Some(spawned)
    }

    pub(crate) fn print(self) -> Print {
        self.entry.print()
    }
}

#[cfg(test)]
impl Sheet {
    /// The first page of the first datasheet.
    pub(crate) fn example() -> Self {
        Self::new(Entry::Chip(ld60_calco::Revision::A), 0)
    }
}

impl Pins {
    pub(crate) const MAX: usize = 2;

    pub(crate) fn sheets(&self) -> &[Sheet] {
        &self.0
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn contains(&self, sheet: Sheet) -> bool {
        self.0.contains(&sheet)
    }

    /// Unpins the sheets whose entry is not in the manuals file of `run`.
    pub(crate) fn keep_filed(&mut self, run: &Run) {
        let contents = Contents::of(run);
        self.0
            .retain(|sheet| contents.entries().contains(&sheet.entry));
    }

    /// Whether the rail holds [`Pins::MAX`] sheets already.
    pub(crate) fn is_full(&self) -> bool {
        self.0.len() >= Self::MAX
    }

    /// Pins `sheet`, unless it is pinned already or the rail is full.
    pub(crate) fn pin(&mut self, sheet: Sheet) {
        if !self.is_full() && !self.contains(sheet) {
            self.0.push(sheet);
        }
    }

    /// Takes `sheet` off the rail, back into the manuals file.
    pub(crate) fn unpin(&mut self, sheet: Sheet) {
        self.0.retain(|&other| other != sheet);
    }
}

impl From<Vec<Sheet>> for Pins {
    /// Keeps the oldest [`Pins::MAX`] sheets that exist, each once.
    fn from(sheets: Vec<Sheet>) -> Self {
        let mut pins = Self::default();
        for sheet in sheets {
            if sheet.page().is_some() {
                pins.pin(sheet);
            }
        }
        pins
    }
}

impl From<Pins> for Vec<Sheet> {
    fn from(pins: Pins) -> Self {
        pins.0
    }
}

#[cfg(test)]
mod tests {
    use ld60_calco::Revision;
    use ld60_devices::DeviceKind;

    use super::*;

    fn chip(page: usize) -> Sheet {
        Sheet::new(Entry::Chip(Revision::A), page)
    }

    #[test]
    fn sheets_that_are_not_filed_come_off() {
        let mut pins = Pins::default();
        let other = Sheet::new(Entry::Chip(Revision::B), 0);
        pins.pin(chip(0));
        pins.pin(other);
        pins.keep_filed(&Run::default());
        assert_eq!(pins.sheets(), [chip(0)]);
    }

    #[test]
    fn a_sheet_is_pinned_once_until_it_is_unpinned() {
        let mut pins = Pins::default();
        pins.pin(chip(0));
        pins.pin(chip(0));
        assert_eq!(pins.sheets(), [chip(0)]);
        pins.unpin(chip(0));
        assert!(pins.is_empty());
    }

    #[test]
    fn a_full_rail_takes_no_more_sheets() {
        let mut pins = Pins::default();
        for page in 0..3 {
            pins.pin(chip(page));
        }
        assert!(pins.is_full());
        assert_eq!(pins.sheets(), [chip(0), chip(1)]);
        pins.unpin(chip(0));
        pins.pin(chip(2));
        assert_eq!(pins.sheets(), [chip(1), chip(2)]);
    }

    #[test]
    fn loading_keeps_the_rules() {
        let missing = Sheet::new(Entry::Manual(DeviceKind::Tally2), 99);
        let note = Sheet::new(Entry::Note(TutorialNote(99)), 0);
        let pins = Pins::from(vec![chip(0), missing, chip(0), note, chip(1), chip(2)]);
        assert_eq!(pins.sheets(), [chip(0), chip(1)]);
    }

    #[test]
    fn pins_load_as_saved() {
        let mut pins = Pins::default();
        pins.pin(chip(1));
        pins.pin(Sheet::new(Entry::Note(TutorialNote(0)), 0));
        let text = ron::to_string(&pins).unwrap();
        assert_eq!(ron::from_str::<Pins>(&text).unwrap(), pins);
    }
}
