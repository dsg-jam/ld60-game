/// Most versions a job keeps, including the factory firmware.
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
}

/// A saved copy of the source.
#[derive(Debug)]
pub(super) struct Version {
    /// Stays the same when other versions are removed. The factory
    /// firmware is 0.
    pub(super) number: u32,
    pub(super) label: String,
    pub(super) text: String,
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
                label: "FACTORY".into(),
                text: factory.into(),
            }],
            next_number: 1,
            selected: 0,
            flashed: None,
        }
    }

    pub(super) fn versions(&self) -> &[Version] {
        &self.versions
    }

    pub(super) fn selected(&self) -> usize {
        self.selected
    }

    pub(super) fn selected_version(&self) -> &Version {
        &self.versions[self.selected]
    }

    pub(super) fn select(&mut self, index: usize) {
        self.selected = index.min(self.versions.len() - 1);
    }

    /// Moves the selection by `step` rows and stops at either end.
    pub(super) fn select_by(&mut self, step: isize) {
        self.select(self.selected.saturating_add_signed(step));
    }

    /// Saves `text` as a new version, selects it and returns its number.
    /// A duplicate selects the version with the same text instead.
    pub(super) fn save(&mut self, text: &str, label: String) -> Result<u32, SaveError> {
        if let Some(index) = self
            .versions
            .iter()
            .position(|version| version.text == text)
        {
            self.selected = index;
            return Err(SaveError::Duplicate(self.versions[index].number));
        }
        if self.versions.len() >= MAX_VERSIONS {
            return Err(SaveError::Full);
        }
        let number = self.next_number;
        self.next_number += 1;
        self.versions.push(Version {
            number,
            label,
            text: text.into(),
        });
        self.selected = self.versions.len() - 1;
        Ok(number)
    }

    /// Removes the selected version and returns its number. The factory
    /// firmware stays.
    pub(super) fn remove_selected(&mut self) -> Option<u32> {
        if self.selected == 0 {
            return None;
        }
        let removed = self.versions.remove(self.selected);
        self.select(self.selected);
        Some(removed.number)
    }

    pub(super) fn mark_flashed(&mut self, text: &str) {
        self.flashed = Some(text.into());
    }

    pub(super) fn is_flashed(&self, version: &Version) -> bool {
        self.flashed.as_deref() == Some(version.text.as_str())
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
        assert_eq!(list.selected_version().label, "FACTORY");
    }

    #[test]
    fn keeps_the_factory_firmware() {
        let mut list = VersionList::new("factory");
        assert_eq!(list.remove_selected(), None);
        list.save("fix", "A".into()).unwrap();
        list.select(0);
        assert_eq!(list.remove_selected(), None);
        assert_eq!(saved(&list), [0, 1]);
    }

    #[test]
    fn numbers_stay_after_removal() {
        let mut list = VersionList::new("factory");
        assert_eq!(list.save("one", "A".into()), Ok(1));
        assert_eq!(list.save("two", "B".into()), Ok(2));
        list.select(1);
        assert_eq!(list.remove_selected(), Some(1));
        assert_eq!(saved(&list), [0, 2]);
        assert_eq!(list.selected_version().number, 2);
        assert_eq!(list.remove_selected(), Some(2));
        assert_eq!(list.selected(), 0);
        assert_eq!(list.save("three", "C".into()), Ok(3));
    }

    #[test]
    fn selection_picks_the_text_to_restore() {
        let mut list = VersionList::new("factory");
        list.save("one", "A".into()).unwrap();
        list.save("two", "B".into()).unwrap();
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
            list.save("factory", "A".into()),
            Err(SaveError::Duplicate(0))
        );
        for n in 1..MAX_VERSIONS {
            list.save(&n.to_string(), "A".into()).unwrap();
        }
        assert_eq!(list.save("more", "A".into()), Err(SaveError::Full));
    }

    #[test]
    fn a_duplicate_selects_the_saved_version() {
        let mut list = VersionList::new("factory");
        list.save("one", "A".into()).unwrap();
        assert_eq!(
            list.save("factory", "B".into()),
            Err(SaveError::Duplicate(0))
        );
        assert_eq!(list.selected(), 0);
    }

    #[test]
    fn marks_the_flashed_version() {
        let mut list = VersionList::new("factory");
        list.save("one", "A".into()).unwrap();
        list.mark_flashed("one");
        let flashed: Vec<_> = list
            .versions()
            .iter()
            .map(|version| list.is_flashed(version))
            .collect();
        assert_eq!(flashed, [false, true]);
    }
}
