use std::time::Duration;

/// Typing after a pause this long starts a new undo step.
const PAUSE: Duration = Duration::from_secs(1);
/// Oldest undo steps are dropped beyond this.
const MAX_STEPS: usize = 500;

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
/// [`Snapshot`] with the last one and groups single character edits into
/// steps, like typing a word.
#[derive(Debug, Default)]
pub(super) struct History {
    current: Snapshot,
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// Whether the next edit may join the last undo step.
    open: bool,
    last_edit: Duration,
}

#[derive(Debug)]
struct Step {
    before: Snapshot,
    after: Snapshot,
    edit: Edit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edit {
    /// One character typed at the caret.
    Type(char),
    /// One character removed before the caret.
    Backspace,
    /// One character removed after the caret.
    Delete,
    Other,
}

impl History {
    /// Starts over with `snapshot` as the unchanged state.
    pub(super) fn new(snapshot: Snapshot) -> Self {
        Self {
            current: snapshot,
            ..Self::default()
        }
    }

    /// Records the editor state at time `now`.
    pub(super) fn observe(&mut self, snapshot: Snapshot, now: Duration) {
        if snapshot == self.current {
            return;
        }
        if snapshot.text == self.current.text {
            // The caret moved. Typing somewhere else is a new step.
            self.open = false;
            self.current = snapshot;
            return;
        }
        let edit = Edit::between(&self.current, &snapshot);
        let joins = self.open
            && now.saturating_sub(self.last_edit) < PAUSE
            && self
                .undo
                .last()
                .is_some_and(|step| step.edit.continues_with(edit));
        match self.undo.last_mut() {
            Some(step) if joins => {
                step.after = snapshot.clone();
                step.edit = edit;
            }
            _ => {
                self.undo.push(Step {
                    before: std::mem::take(&mut self.current),
                    after: snapshot.clone(),
                    edit,
                });
                if self.undo.len() > MAX_STEPS {
                    self.undo.remove(0);
                }
            }
        }
        self.redo.clear();
        self.open = edit != Edit::Other;
        self.last_edit = now;
        self.current = snapshot;
    }

    /// Steps back and returns the state to show, if there is one.
    pub(super) fn undo(&mut self) -> Option<&Snapshot> {
        let step = self.undo.pop()?;
        self.current = step.before.clone();
        self.redo.push(step);
        self.open = false;
        Some(&self.current)
    }

    /// Steps forward again after [`History::undo`].
    pub(super) fn redo(&mut self) -> Option<&Snapshot> {
        let step = self.redo.pop()?;
        self.current = step.after.clone();
        self.undo.push(step);
        self.open = false;
        Some(&self.current)
    }

    /// Keeps the next edit out of the current undo step.
    pub(super) fn seal(&mut self) {
        self.open = false;
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
        let unchanged = |keep: usize, from: usize, to: usize| {
            old.get(..keep) == new.get(..keep) && old.get(from..) == new.get(to..)
        };
        if new_caret > caret && unchanged(caret, caret, new_caret) {
            let mut typed = new[caret..new_caret].chars();
            if let (Some(c), None) = (typed.next(), typed.next()) {
                return Self::Type(c);
            }
        } else if new_caret < caret && unchanged(new_caret, caret, new_caret) {
            if old[new_caret..caret].chars().count() == 1 {
                return Self::Backspace;
            }
        } else if new_caret == caret && old.len() > new.len() {
            let removed = old.len() - new.len();
            if unchanged(caret, caret + removed, caret)
                && old[caret..caret + removed].chars().count() == 1
            {
                return Self::Delete;
            }
        }
        Self::Other
    }

    /// Whether `next` belongs in the same undo step as `self`.
    fn continues_with(self, next: Self) -> bool {
        match (self, next) {
            // A word starts a new step, the space after it does not.
            (Self::Type(last), Self::Type(next)) => !last.is_whitespace() || next.is_whitespace(),
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
        let mut current = history.current.text.clone();
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
    fn handles_multibyte_characters() {
        let mut history = History::new(at("", 0));
        type_text(&mut history, "\u{e9}\u{fc}", 0);
        history.observe(at("\u{e9}", 2), ms(300));
        assert_eq!(history.undo(), Some(&at("\u{e9}\u{fc}", 4)));
        assert_eq!(history.undo(), Some(&at("", 0)));
    }
}
