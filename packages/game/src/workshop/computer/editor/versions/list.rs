use crate::locale::Message;

/// Most versions a job keeps, including the factory firmware. The player
/// can save one less, so a restore always has room to keep the changes.
pub(super) const MAX_VERSIONS: usize = 16;

/// Saved copies of the firmware source for one job.
///
/// The factory firmware is always the first version and stays in the list.
#[derive(Debug)]
pub(super) struct VersionList {
    versions: Vec<Version>,
    next_number: u32,
    selected: usize,
    /// The source of the last flash.
    flashed: Option<String>,
    /// The version that the next removal deletes.
    armed: Option<u32>,
}

/// A saved copy of the source.
#[derive(Debug)]
pub(super) struct Version {
    /// Stays the same when other versions are removed. The factory
    /// firmware is 0.
    pub(super) number: u32,
    pub(super) label: Message,
    pub(super) text: String,
    /// Saved by a restore, not by the player.
    pub(super) auto: bool,
}

/// What a removal of the selected version did.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Removal {
    /// The factory firmware stays.
    Factory,
    /// The next removal of this version deletes it.
    Armed(u32),
    Removed(u32),
}

/// Why a version could not be saved.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum SaveError {
    Full,
    /// The same source is already saved under this number.
    Duplicate(u32),
}

impl VersionList {
    pub(super) fn new(factory: &str) -> Self {
        Self {
            versions: vec![Version {
                number: 0,
                label: Message::new("editor-version-factory"),
                text: factory.into(),
                auto: false,
            }],
            next_number: 1,
            selected: 0,
            flashed: None,
            armed: None,
        }
    }

    pub(super) fn versions(&self) -> &[Version] {
        &self.versions
    }

    pub(super) fn factory(&self) -> &Version {
        &self.versions[0]
    }

    pub(super) fn selected(&self) -> usize {
        self.selected
    }

    pub(super) fn selected_version(&self) -> &Version {
        &self.versions[self.selected]
    }

    pub(super) fn select(&mut self, index: usize) {
        self.selected = index.min(self.versions.len() - 1);
        self.disarm();
    }

    /// Moves the selection by `step` rows and stops at either end.
    pub(super) fn select_by(&mut self, step: isize) {
        self.select(self.selected.saturating_add_signed(step));
    }

    /// Cancels a pending removal.
    pub(super) fn disarm(&mut self) {
        self.armed = None;
    }

    /// Saves `text` as a new version, selects it and returns its number.
    /// A duplicate selects the version with the same text instead.
    pub(super) fn save(&mut self, text: &str, label: Message) -> Result<u32, SaveError> {
        self.disarm();
        if let Some(index) = self.position(text) {
            self.selected = index;
            return Err(SaveError::Duplicate(self.versions[index].number));
        }
        if self.versions.len() >= MAX_VERSIONS - 1 {
            return Err(SaveError::Full);
        }
        let number = self.push(text, label, false);
        self.selected = self.versions.len() - 1;
        Ok(number)
    }

    /// Saves `text` before a restore replaces it, unless a version has it
    /// already. Returns the number of the new version.
    ///
    /// A full list drops its oldest automatic save. It drops the selected
    /// version only if that is the only automatic save, so read the text to
    /// restore first. Otherwise the selection stays on the same version.
    pub(super) fn keep(&mut self, text: &str, label: Message) -> Result<Option<u32>, SaveError> {
        self.disarm();
        if self.position(text).is_some() {
            return Ok(None);
        }
        let mut dropped_selected = false;
        if self.versions.len() >= MAX_VERSIONS {
            let auto = |(index, version): (usize, &Version)| version.auto.then_some(index);
            let mut autos = self.versions.iter().enumerate().filter_map(auto);
            let oldest = autos
                .clone()
                .find(|&index| index != self.selected)
                .or_else(|| autos.next())
                .ok_or(SaveError::Full)?;
            self.versions.remove(oldest);
            dropped_selected = oldest == self.selected;
            if oldest < self.selected {
                self.selected -= 1;
            }
        }
        let number = self.push(text, label, true);
        if dropped_selected {
            self.selected = self.versions.len() - 1;
        }
        Ok(Some(number))
    }

    /// Removes the selected version on the second call in a row.
    pub(super) fn remove_selected(&mut self) -> Removal {
        if self.selected == 0 {
            return Removal::Factory;
        }
        let number = self.versions[self.selected].number;
        if self.armed != Some(number) {
            self.armed = Some(number);
            return Removal::Armed(number);
        }
        self.versions.remove(self.selected);
        self.select(self.selected);
        Removal::Removed(number)
    }

    pub(super) fn mark_flashed(&mut self, text: &str) {
        self.flashed = Some(text.into());
    }

    pub(super) fn is_flashed(&self, version: &Version) -> bool {
        self.flashed.as_deref() == Some(version.text.as_str())
    }

    fn position(&self, text: &str) -> Option<usize> {
        self.versions
            .iter()
            .position(|version| version.text == text)
    }

    fn push(&mut self, text: &str, label: Message, auto: bool) -> u32 {
        let number = self.next_number;
        self.next_number += 1;
        self.versions.push(Version {
            number,
            label,
            text: text.into(),
            auto,
        });
        number
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(list: &VersionList) -> Vec<u32> {
        list.versions()
            .iter()
            .map(|version| version.number)
            .collect()
    }

    #[test]
    fn starts_with_the_factory_firmware() {
        let list = VersionList::new("factory");
        assert_eq!(saved(&list), [0]);
        assert_eq!(list.selected_version().text, "factory");
        assert_eq!(
            list.selected_version().label,
            Message::new("editor-version-factory")
        );
    }

    #[test]
    fn keeps_the_factory_firmware() {
        let mut list = VersionList::new("factory");
        assert_eq!(list.remove_selected(), Removal::Factory);
        list.save("fix", Message::new("A")).unwrap();
        list.select(0);
        assert_eq!(list.remove_selected(), Removal::Factory);
        assert_eq!(list.remove_selected(), Removal::Factory);
        assert_eq!(saved(&list), [0, 1]);
    }

    #[test]
    fn numbers_stay_after_removal() {
        let mut list = VersionList::new("factory");
        assert_eq!(list.save("one", Message::new("A")), Ok(1));
        assert_eq!(list.save("two", Message::new("B")), Ok(2));
        list.select(1);
        assert_eq!(list.remove_selected(), Removal::Armed(1));
        assert_eq!(list.remove_selected(), Removal::Removed(1));
        assert_eq!(saved(&list), [0, 2]);
        assert_eq!(list.selected_version().number, 2);
        assert_eq!(list.remove_selected(), Removal::Armed(2));
        assert_eq!(list.remove_selected(), Removal::Removed(2));
        assert_eq!(list.selected(), 0);
        assert_eq!(list.save("three", Message::new("C")), Ok(3));
    }

    #[test]
    fn removal_needs_a_second_press() {
        let mut list = VersionList::new("factory");
        list.save("one", Message::new("A")).unwrap();
        list.save("two", Message::new("B")).unwrap();
        assert_eq!(list.remove_selected(), Removal::Armed(2));
        list.select_by(-1);
        assert_eq!(list.remove_selected(), Removal::Armed(1));
        list.disarm();
        assert_eq!(list.remove_selected(), Removal::Armed(1));
        assert_eq!(list.remove_selected(), Removal::Removed(1));
        assert_eq!(saved(&list), [0, 2]);
    }

    #[test]
    fn keeps_unsaved_text_without_moving_the_selection() {
        let mut list = VersionList::new("factory");
        list.save("one", Message::new("A")).unwrap();
        list.select(0);
        assert_eq!(list.keep("one", Message::new("B")), Ok(None));
        assert_eq!(list.keep("draft", Message::new("B")), Ok(Some(2)));
        assert_eq!(list.selected_version().text, "factory");
        assert!(list.versions()[2].auto);
    }

    #[test]
    fn a_full_list_drops_the_oldest_automatic_save() {
        let mut list = VersionList::new("factory");
        list.save("saved", Message::new("A")).unwrap();
        for n in 2..MAX_VERSIONS {
            list.keep(&n.to_string(), Message::new("B")).unwrap();
        }
        list.select(2);
        assert_eq!(list.keep("new", Message::new("B")), Ok(Some(16)));
        assert_eq!(saved(&list)[..4], [0, 1, 2, 4]);
        assert_eq!(list.selected_version().number, 2);
    }

    #[test]
    fn a_full_list_still_keeps_the_changes_of_a_restore() {
        let mut list = VersionList::new("factory");
        for n in 1..MAX_VERSIONS - 1 {
            list.save(&n.to_string(), Message::new("A")).unwrap();
        }
        assert_eq!(list.save("more", Message::new("A")), Err(SaveError::Full));
        list.select(0);
        assert_eq!(list.keep("draft", Message::new("B")), Ok(Some(15)));
        assert_eq!(list.versions().len(), MAX_VERSIONS);
        assert_eq!(list.selected_version().text, "factory");

        list.select(MAX_VERSIONS - 1);
        assert_eq!(list.keep("edit", Message::new("B")), Ok(Some(16)));
        assert_eq!(list.versions().len(), MAX_VERSIONS);
        assert_eq!(list.selected_version().text, "edit");
    }

    #[test]
    fn selection_picks_the_text_to_restore() {
        let mut list = VersionList::new("factory");
        list.save("one", Message::new("A")).unwrap();
        list.save("two", Message::new("B")).unwrap();
        list.select_by(-1);
        assert_eq!(list.selected_version().text, "one");
        list.select_by(-5);
        assert_eq!(list.selected_version().text, "factory");
        list.select_by(5);
        assert_eq!(list.selected_version().text, "two");
    }

    #[test]
    fn refuses_duplicates_and_overflow() {
        let mut list = VersionList::new("factory");
        assert_eq!(
            list.save("factory", Message::new("A")),
            Err(SaveError::Duplicate(0))
        );
        for n in 1..MAX_VERSIONS - 1 {
            list.save(&n.to_string(), Message::new("A")).unwrap();
        }
        assert_eq!(list.save("more", Message::new("A")), Err(SaveError::Full));
    }

    #[test]
    fn a_duplicate_selects_the_saved_version() {
        let mut list = VersionList::new("factory");
        list.save("one", Message::new("A")).unwrap();
        assert_eq!(
            list.save("factory", Message::new("B")),
            Err(SaveError::Duplicate(0))
        );
        assert_eq!(list.selected(), 0);
    }

    #[test]
    fn marks_the_flashed_version() {
        let mut list = VersionList::new("factory");
        list.save("one", Message::new("A")).unwrap();
        list.mark_flashed("one");
        let flashed: Vec<_> = list
            .versions()
            .iter()
            .map(|version| list.is_flashed(version))
            .collect();
        assert_eq!(flashed, [false, true]);
    }
}
