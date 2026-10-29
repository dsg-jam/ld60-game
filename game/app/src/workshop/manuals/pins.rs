use std::fmt;

use bevy::prelude::*;
use ld60_game_core::parts::paper::{Paper, Print};
use ld60_game_core::parts::screen::UiCanvas;
use serde::de::{MapAccess, Visitor, value::MapAccessDeserializer};
use serde::{Deserialize, Deserializer, Serialize};

use super::contents::{Contents, Entry, Page};
use crate::workshop::run::{NewRun, Run};
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
/// the shop computer, each in one of [`Pins::MAX`] slots on the rail. Slot
/// 0 is next to the monitor. A pinned sheet keeps its slot and is not in the
/// file until it is unpinned. A new run starts with an empty rail.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Vec<SavedSlot>", into = "Vec<Option<Sheet>>")]
pub(crate) struct Pins([Option<Sheet>; Pins::MAX]);

/// A slot of [`Pins`] as saved. Saves from before slots list the sheets
/// alone.
struct SavedSlot(Option<Sheet>);

impl Sheet {
    pub(super) fn new(entry: Entry, page: usize) -> Self {
        Self { entry, page }
    }

    /// The page of the tutorial note `note`.
    pub(crate) fn note(note: TutorialNote) -> Self {
        Self::new(Entry::Note(note), 0)
    }

    pub(super) fn entry(self) -> Entry {
        self.entry
    }

    /// Where the sheet is among all pages of `contents`.
    pub(super) fn number(self, contents: &Contents) -> Option<usize> {
        Some(contents.start_of(self.entry)? + self.page)
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

    /// What hangs in each slot.
    pub(crate) fn slots(&self) -> &[Option<Sheet>] {
        &self.0
    }

    /// How many slots there are up to the last one in use.
    pub(crate) fn used(&self) -> usize {
        self.0
            .iter()
            .rposition(Option::is_some)
            .map_or(0, |last| last + 1)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.used() == 0
    }

    pub(crate) fn contains(&self, sheet: Sheet) -> bool {
        self.slot_of(sheet).is_some()
    }

    pub(crate) fn slot_of(&self, sheet: Sheet) -> Option<usize> {
        self.0.iter().position(|&other| other == Some(sheet))
    }

    /// Puts the pinned sheets back into the file, as a new run starts.
    pub(super) fn clear_for_new_run(_: On<NewRun>, mut pins: ResMut<Pins>) {
        *pins = Pins::default();
    }

    /// Unpins the sheets whose entry is not in the manuals file of `run`.
    pub(crate) fn keep_filed(&mut self, run: &Run) {
        let contents = Contents::of(run);
        for slot in &mut self.0 {
            if slot.is_some_and(|sheet| !contents.entries().contains(&sheet.entry)) {
                *slot = None;
            }
        }
    }

    /// Whether every slot holds a sheet already.
    pub(crate) fn is_full(&self) -> bool {
        self.free_slot().is_none()
    }

    /// The free slot closest to the monitor, where the next sheet goes.
    pub(crate) fn free_slot(&self) -> Option<usize> {
        self.0.iter().position(Option::is_none)
    }

    /// Pins `sheet` in the free slot closest to the monitor, unless it is
    /// pinned already or the rail is full.
    pub(crate) fn pin(&mut self, sheet: Sheet) {
        if !self.contains(sheet)
            && let Some(slot) = self.free_slot()
        {
            self.0[slot] = Some(sheet);
        }
    }

    /// Takes `sheet` off the rail, back into the manuals file. The other
    /// sheets stay where they are.
    pub(crate) fn unpin(&mut self, sheet: Sheet) {
        if let Some(slot) = self.slot_of(sheet) {
            self.0[slot] = None;
        }
    }
}

impl From<Vec<SavedSlot>> for Pins {
    /// Keeps the sheets that exist in their slots, each once.
    fn from(slots: Vec<SavedSlot>) -> Self {
        let mut pins = Self::default();
        for (slot, SavedSlot(sheet)) in slots.into_iter().take(Self::MAX).enumerate() {
            if let Some(sheet) = sheet
                && sheet.page().is_some()
                && !pins.contains(sheet)
            {
                pins.0[slot] = Some(sheet);
            }
        }
        pins
    }
}

impl From<Pins> for Vec<Option<Sheet>> {
    fn from(pins: Pins) -> Self {
        pins.0.to_vec()
    }
}

impl<'de> Deserialize<'de> for SavedSlot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(SavedSlotVisitor)
    }
}

struct SavedSlotVisitor;

impl<'de> Visitor<'de> for SavedSlotVisitor {
    type Value = SavedSlot;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sheet or none")
    }

    fn visit_none<E>(self) -> Result<SavedSlot, E> {
        Ok(SavedSlot(None))
    }

    fn visit_unit<E>(self) -> Result<SavedSlot, E> {
        Ok(SavedSlot(None))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<SavedSlot, D::Error> {
        Sheet::deserialize(deserializer).map(|sheet| SavedSlot(Some(sheet)))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<SavedSlot, A::Error> {
        Sheet::deserialize(MapAccessDeserializer::new(map)).map(|sheet| SavedSlot(Some(sheet)))
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
        pins.pin(other);
        pins.pin(chip(0));
        pins.keep_filed(&Run::default());
        assert_eq!(pins.slots(), [None, Some(chip(0))]);
    }

    #[test]
    fn a_sheet_is_pinned_once_until_it_is_unpinned() {
        let mut pins = Pins::default();
        pins.pin(chip(0));
        pins.pin(chip(0));
        assert_eq!(pins.slots(), [Some(chip(0)), None]);
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
        assert_eq!(pins.slots(), [Some(chip(0)), Some(chip(1))]);
    }

    #[test]
    fn sheets_keep_their_slots() {
        let mut pins = Pins::default();
        pins.pin(chip(0));
        pins.pin(chip(1));
        pins.unpin(chip(0));
        assert_eq!(pins.slots(), [None, Some(chip(1))]);
        assert_eq!(pins.used(), 2);
        pins.pin(chip(2));
        assert_eq!(pins.slots(), [Some(chip(2)), Some(chip(1))]);
        assert_eq!(pins.slot_of(chip(1)), Some(1));
    }

    #[test]
    fn loading_keeps_the_rules() {
        let missing = Sheet::new(Entry::Manual(DeviceKind::Tally2), 99);
        let note = Sheet::new(Entry::Note(TutorialNote(99)), 0);
        let load = |sheets: Vec<Option<Sheet>>| {
            Pins::from(sheets.into_iter().map(SavedSlot).collect::<Vec<_>>())
        };
        assert_eq!(
            load(vec![Some(missing), Some(chip(0))]).slots(),
            [None, Some(chip(0))]
        );
        assert_eq!(
            load(vec![Some(chip(0)), Some(chip(0))]).slots(),
            [Some(chip(0)), None]
        );
        assert_eq!(
            load(vec![Some(note), None, Some(chip(1))]).slots(),
            [None, None]
        );
    }

    #[test]
    fn pins_load_as_saved() {
        let mut pins = Pins::default();
        pins.pin(chip(1));
        pins.pin(Sheet::new(Entry::Note(TutorialNote(0)), 0));
        pins.unpin(chip(1));
        let text = ron::to_string(&pins).unwrap();
        assert_eq!(ron::from_str::<Pins>(&text).unwrap(), pins);
    }

    #[test]
    fn pins_from_before_slots_load_in_order() {
        let manual = Sheet::new(Entry::Manual(DeviceKind::Tally2), 1);
        let text = ron::to_string(&vec![manual, chip(0)]).unwrap();
        let pins = ron::from_str::<Pins>(&text).unwrap();
        assert_eq!(pins.slots(), [Some(manual), Some(chip(0))]);
    }
}
