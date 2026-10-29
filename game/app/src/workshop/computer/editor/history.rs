use std::collections::VecDeque;
use std::time::Duration;

/// Typing after a pause this long starts a new undo step.
const PAUSE: Duration = Duration::from_secs(1);
/// Oldest undo steps are dropped beyond this many steps.
const MAX_STEPS: usize = 500;
/// Oldest undo steps are dropped while all kept text is larger than this.
const MAX_BYTES: usize = 4 << 20;
/// Up to this many characters inserted in one frame still count as typing.
/// More is a paste.
const MAX_BURST: usize = 3;

/// The editor text and selection at one point in time.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub(super) text: String,
    /// Byte indices of the selection anchor and focus. Equal for a caret.
    pub(super) selection: (usize, usize),
}

/// Undo and redo steps of the editor.
///
/// The history does not see edits themselves. It compares each new
/// [`Snapshot`] with the last one and groups typing into steps, like
/// typing a word.
#[derive(Debug)]
pub(super) struct History {
    /// The states between steps, oldest first. `states[at]` is the current
    /// one. Later states can be redone.
    states: VecDeque<Snapshot>,
    at: usize,
    /// The last edit, while the next edit may still join its step.
    open: Option<Edit>,
    last_edit: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edit {
    /// A few characters typed at the caret.
    Type {
        first: char,
        last: char,
    },
    /// One character removed before the caret.
    Backspace,
    /// One character removed after the caret.
    Delete,
    Other,
}

impl Default for History {
    fn default() -> Self {
        Self::new(Snapshot::default())
    }
}

impl History {
    /// Starts over with `snapshot` as the unchanged state.
    pub(super) fn new(snapshot: Snapshot) -> Self {
        Self {
            states: VecDeque::from([snapshot]),
            at: 0,
            open: None,
            last_edit: Duration::ZERO,
        }
    }

    /// Records the editor state at time `now`.
    pub(super) fn observe(&mut self, snapshot: Snapshot, now: Duration) {
        let current = &self.states[self.at];
        if snapshot == *current {
            return;
        }
        if snapshot.text == current.text {
            // The caret moved. Typing somewhere else is a new step.
            self.states[self.at].selection = snapshot.selection;
            self.open = None;
            return;
        }
        let edit = Edit::between(current, &snapshot);
        let joins = now.saturating_sub(self.last_edit) < PAUSE
            && self.open.is_some_and(|last| last.continues_with(edit));
        if joins {
            self.states[self.at] = snapshot;
        } else {
            self.states.truncate(self.at + 1);
            self.states.push_back(snapshot);
            self.at += 1;
            self.trim();
        }
        self.open = (edit != Edit::Other).then_some(edit);
        self.last_edit = now;
    }

    /// Steps back and returns the state to show, if there is one.
    pub(super) fn undo(&mut self) -> Option<&Snapshot> {
        self.at = self.at.checked_sub(1)?;
        self.open = None;
        self.states.get(self.at)
    }

    /// Steps forward again after [`History::undo`].
    pub(super) fn redo(&mut self) -> Option<&Snapshot> {
        if self.at + 1 >= self.states.len() {
            return None;
        }
        self.at += 1;
        self.open = None;
        self.states.get(self.at)
    }

    /// Keeps the next edit out of the current undo step.
    pub(super) fn seal(&mut self) {
        self.open = None;
    }

    fn trim(&mut self) {
        let mut bytes: usize = self.states.iter().map(|state| state.text.len()).sum();
        while self.at > 0 && (self.states.len() > MAX_STEPS + 1 || bytes > MAX_BYTES) {
            if let Some(state) = self.states.pop_front() {
                bytes -= state.text.len();
                self.at -= 1;
            }
        }
    }
}

impl Edit {
    fn between(old: &Snapshot, new: &Snapshot) -> Self {
        let (anchor, caret) = old.selection;
        let (new_anchor, new_caret) = new.selection;
        if anchor != caret || new_anchor != new_caret {
            return Self::Other;
        }
        let (old, new) = (old.text.as_str(), new.text.as_str());
        // Both texts match before `keep`, and from `from` in the old one
        // and `to` in the new one.
        let unchanged = |keep: usize, from: usize, to: usize| {
            let same = |a: Option<&str>, b: Option<&str>| a.is_some() && a == b;
            same(old.get(..keep), new.get(..keep)) && same(old.get(from..), new.get(to..))
        };
        let single = |text: Option<&str>| text.is_some_and(|text| text.chars().count() == 1);
        if new_caret > caret && unchanged(caret, caret, new_caret) {
            let typed = new.get(caret..new_caret).unwrap_or_default();
            let count = typed.chars().count();
            if let (Some(first), Some(last)) = (typed.chars().next(), typed.chars().next_back())
                && (count == 1 || (count <= MAX_BURST && !typed.contains('\n')))
            {
                return Self::Type { first, last };
            }
        } else if new_caret < caret && unchanged(new_caret, caret, new_caret) {
            if single(old.get(new_caret..caret)) {
                return Self::Backspace;
            }
        } else if new_caret == caret
            && let Some(end) = (caret + old.len()).checked_sub(new.len())
            && end > caret
            && unchanged(caret, end, caret)
            && single(old.get(caret..end))
        {
            return Self::Delete;
        }
        Self::Other
    }

    /// Whether `next` belongs in the same undo step as `self`.
    fn continues_with(self, next: Self) -> bool {
        match (self, next) {
            // A word starts a new step, the space after it does not.
            (Self::Type { last, .. }, Self::Type { first, .. }) => {
                !last.is_whitespace() || first.is_whitespace()
            }
            (Self::Backspace, Self::Backspace) | (Self::Delete, Self::Delete) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str, caret: usize) -> Snapshot {
        Snapshot {
            text: text.into(),
            selection: (caret, caret),
        }
    }

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Types `text` at the end, one character every 100 ms from `start`.
    fn type_text(history: &mut History, text: &str, start: u64) -> u64 {
        let mut current = history.states[history.at].text.clone();
        let mut time = start;
        for c in text.chars() {
            current.push(c);
            history.observe(at(&current, current.len()), ms(time));
            time += 100;
        }
        time
    }

    #[test]
    fn groups_typing_by_word() {
        let mut history = History::new(at("", 0));
        type_text(&mut history, "LDI A", 0);
        assert_eq!(history.undo(), Some(&at("LDI ", 4)));
        assert_eq!(history.undo(), Some(&at("", 0)));
        assert_eq!(history.undo(), None);
    }

    #[test]
    fn a_pause_starts_a_new_step() {
        let mut history = History::new(at("", 0));
        let time = type_text(&mut history, "LD", 0);
        type_text(&mut history, "I", time + 2000);
        assert_eq!(history.undo(), Some(&at("LD", 2)));
    }

    #[test]
    fn moving_the_caret_starts_a_new_step() {
        let mut history = History::new(at("", 0));
        history.observe(at("a", 1), ms(0));
        history.observe(at("a", 0), ms(10));
        history.observe(at("ba", 1), ms(20));
        assert_eq!(history.undo(), Some(&at("a", 0)));
        assert_eq!(history.undo(), Some(&at("", 0)));
    }

    #[test]
    fn groups_backspace_and_delete() {
        let mut history = History::new(at("abcd", 4));
        history.observe(at("abc", 3), ms(0));
        history.observe(at("ab", 2), ms(10));
        history.observe(at("a", 1), ms(20));
        assert_eq!(history.undo(), Some(&at("abcd", 4)));
        let mut history = History::new(at("abcd", 0));
        history.observe(at("bcd", 0), ms(0));
        history.observe(at("cd", 0), ms(10));
        history.observe(at("ccd", 1), ms(20));
        assert_eq!(history.undo(), Some(&at("cd", 0)));
        assert_eq!(history.undo(), Some(&at("abcd", 0)));
    }

    #[test]
    fn pastes_and_indents_are_steps_of_their_own() {
        let mut history = History::new(at("x", 1));
        history.observe(at("x    ", 5), ms(0));
        history.observe(at("x    y", 6), ms(10));
        history.observe(at("x    yLDI A, 1", 14), ms(20));
        history.observe(at("x    yLDI A, 1z", 15), ms(30));
        assert_eq!(history.undo(), Some(&at("x    yLDI A, 1", 14)));
        assert_eq!(history.undo(), Some(&at("x    y", 6)));
        assert_eq!(history.undo(), Some(&at("x    ", 5)));
        assert_eq!(history.undo(), Some(&at("x", 1)));
    }

    #[test]
    fn restores_the_selection() {
        let selected = Snapshot {
            text: "LDI A, 1".into(),
            selection: (7, 4),
        };
        let mut history = History::new(at("LDI A, 1", 0));
        history.observe(selected.clone(), ms(0));
        history.observe(at("LDI 2", 5), ms(10));
        assert_eq!(history.undo(), Some(&selected));
        assert_eq!(history.redo(), Some(&at("LDI 2", 5)));
    }

    #[test]
    fn a_new_edit_clears_redo() {
        let mut history = History::new(at("", 0));
        history.observe(at("a", 1), ms(0));
        history.undo();
        history.observe(at("b", 1), ms(10));
        assert_eq!(history.redo(), None);
        assert_eq!(history.undo(), Some(&at("", 0)));
    }

    #[test]
    fn undo_and_redo_do_not_join_with_typing() {
        let mut history = History::new(at("", 0));
        history.observe(at("a", 1), ms(0));
        history.observe(at("ab", 2), ms(10));
        history.undo();
        history.redo();
        history.observe(at("abc", 3), ms(20));
        assert_eq!(history.undo(), Some(&at("ab", 2)));
    }

    #[test]
    fn seal_ends_the_step() {
        let mut history = History::new(at("", 0));
        history.observe(at("a", 1), ms(0));
        history.seal();
        history.observe(at("ab", 2), ms(10));
        assert_eq!(history.undo(), Some(&at("a", 1)));
    }

    #[test]
    fn a_few_characters_in_one_frame_are_typing() {
        let mut history = History::new(at("", 0));
        history.observe(at("LD", 2), ms(0));
        history.observe(at("LDI ", 4), ms(10));
        history.observe(at("LDI A,", 6), ms(20));
        assert_eq!(history.undo(), Some(&at("LDI ", 4)));
        assert_eq!(history.undo(), Some(&at("", 0)));
    }

    #[test]
    fn ignores_carets_outside_the_text() {
        let old = at("ab", 14);
        for new in [at("ab\u{65e5}\u{672c}", 11), at("a", 30), at("abc", 1)] {
            assert_eq!(Edit::between(&old, &new), Edit::Other);
            assert_eq!(Edit::between(&new, &old), Edit::Other);
        }
        let mut history = History::new(old);
        history.observe(at("ab\u{65e5}\u{672c}", 11), ms(0));
        assert_eq!(history.undo(), Some(&at("ab", 14)));
    }

    #[test]
    fn drops_the_oldest_steps_beyond_the_byte_limit() {
        let big = "x".repeat(MAX_BYTES / 4 - 1);
        let mut history = History::new(at("", 0));
        for i in 0..5 {
            history.observe(at(&format!("{big}{i}"), 0), ms(i * 2000));
        }
        let kept = history
            .states
            .iter()
            .map(|state| state.text.len())
            .sum::<usize>();
        assert!(kept <= MAX_BYTES);
        assert_eq!(history.states.len(), 4);
        for _ in 0..3 {
            assert!(history.undo().is_some());
        }
        assert_eq!(history.undo(), None);
    }

    #[test]
    fn handles_multibyte_characters() {
        let mut history = History::new(at("", 0));
        type_text(&mut history, "\u{e9}\u{fc}", 0);
        history.observe(at("\u{e9}", 2), ms(300));
        assert_eq!(history.undo(), Some(&at("\u{e9}\u{fc}", 4)));
        assert_eq!(history.undo(), Some(&at("", 0)));
    }
}
