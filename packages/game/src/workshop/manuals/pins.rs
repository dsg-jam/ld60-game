use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::contents::{Entry, Page};
use crate::parts::paper::Paper;
use crate::parts::screen::UiCanvas;
use crate::workshop::tutorial::TutorialNote;

/// One page of the manuals file, named by its entry so it stays the same
/// page when more is filed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Sheet {
    entry: Entry,
    /// Counted from 0 within the entry.
    page: usize,
}

/// The sheets taken out of the manuals file and pinned to the monitor of
/// the shop computer, at most [`Pins::MAX`] of them, the oldest first.
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
        Some(Paper::spawn_single(
            commands,
            canvas,
            page.footer(),
            |content| {
                content.spawn(self.band());
                page.write(content);
            },
        ))
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

    /// Unpins `sheet` if it is pinned, and pins it otherwise. With
    /// [`Pins::MAX`] sheets pinned already, the oldest one comes off.
    pub(crate) fn toggle(&mut self, sheet: Sheet) {
        if let Some(index) = self.0.iter().position(|&other| other == sheet) {
            self.0.remove(index);
            return;
        }
        if self.0.len() == Self::MAX {
            self.0.remove(0);
        }
        self.0.push(sheet);
    }
}

impl From<Vec<Sheet>> for Pins {
    /// Keeps the newest [`Pins::MAX`] sheets that exist, each once.
    fn from(sheets: Vec<Sheet>) -> Self {
        let mut pins = Self::default();
        for sheet in sheets {
            if sheet.page().is_some() && !pins.contains(sheet) {
                pins.toggle(sheet);
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
    fn pinning_again_unpins() {
        let mut pins = Pins::default();
        pins.toggle(chip(0));
        assert_eq!(pins.sheets(), [chip(0)]);
        pins.toggle(chip(0));
        assert!(pins.is_empty());
    }

    #[test]
    fn a_third_pin_takes_off_the_oldest() {
        let mut pins = Pins::default();
        for page in 0..3 {
            pins.toggle(chip(page));
        }
        assert_eq!(pins.sheets(), [chip(1), chip(2)]);
        pins.toggle(chip(1));
        assert_eq!(pins.sheets(), [chip(2)]);
    }

    #[test]
    fn loading_keeps_the_rules() {
        let missing = Sheet::new(Entry::Manual(DeviceKind::Tally2), 99);
        let note = Sheet::new(Entry::Note(TutorialNote(99)), 0);
        let pins = Pins::from(vec![chip(0), missing, chip(1), chip(0), note, chip(2)]);
        assert_eq!(pins.sheets(), [chip(1), chip(2)]);
    }

    #[test]
    fn pins_load_as_saved() {
        let mut pins = Pins::default();
        pins.toggle(chip(1));
        pins.toggle(Sheet::new(Entry::Note(TutorialNote(0)), 0));
        let text = ron::to_string(&pins).unwrap();
        assert_eq!(ron::from_str::<Pins>(&text).unwrap(), pins);
    }
}
