use super::contents::Contents;
use super::pins::Pins;

/// The pages that are in the file, without the ones pinned to the monitor.
/// Pages are counted among all pages of the [`Contents`], like their print
/// numbers them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct InFile {
    /// The number of each page in the file.
    numbers: Vec<usize>,
}

impl InFile {
    pub(super) fn new(contents: &Contents, pins: &Pins) -> Self {
        let numbers = contents
            .pages()
            .into_iter()
            .enumerate()
            .filter(|(_, (_, sheet))| sheet.is_none_or(|sheet| !pins.contains(sheet)))
            .map(|(number, _)| number)
            .collect();
        Self { numbers }
    }

    /// The number of the `page`th page in the file.
    pub(super) fn number(&self, page: usize) -> Option<usize> {
        self.numbers.get(page).copied()
    }

    /// Where page `number` is in the file, unless it is pinned.
    pub(super) fn page(&self, number: usize) -> Option<usize> {
        self.numbers.binary_search(&number).ok()
    }

    /// The first page in the file from page `number` on.
    pub(super) fn from(&self, number: usize) -> Option<usize> {
        let page = self.numbers.partition_point(|&other| other < number);
        (page < self.numbers.len()).then_some(page)
    }

    pub(super) fn numbers(&self) -> &[usize] {
        &self.numbers
    }
}

#[cfg(test)]
mod tests {
    use ld60_game_run::Run;

    use super::*;

    #[test]
    fn pinned_pages_are_not_in_the_file() {
        let contents = Contents::of(&Run::default());
        let mut pins = Pins::default();
        let pinned = contents.sheet_at(2).unwrap();
        pins.pin(pinned);
        let in_file = InFile::new(&contents, &pins);
        assert_eq!(in_file.numbers().len(), contents.pages().len() - 1);
        assert_eq!(in_file.page(2), None);
        assert_eq!(in_file.from(2), Some(2));
        assert_eq!(in_file.number(2), Some(3));
        assert_eq!(in_file.page(3), Some(2));
        assert_eq!(in_file.from(99), None);
    }
}
