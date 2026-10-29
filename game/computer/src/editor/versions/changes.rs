use std::fmt;

/// Lines added and removed from one text to another.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct LineChanges {
    added: usize,
    removed: usize,
}

impl LineChanges {
    /// Counts the lines outside the longest common subsequence of lines.
    pub(super) fn between(old: &str, new: &str) -> Self {
        let old: Vec<_> = old.lines().collect();
        let new: Vec<_> = new.lines().collect();
        let mut row = vec![0; new.len() + 1];
        for line in &old {
            let mut diagonal = 0;
            for (index, other) in new.iter().enumerate() {
                let above = row[index + 1];
                row[index + 1] = if line == other {
                    diagonal + 1
                } else {
                    above.max(row[index])
                };
                diagonal = above;
            }
        }
        let common = row[new.len()];
        Self {
            added: new.len() - common,
            removed: old.len() - common,
        }
    }
}

impl fmt::Display for LineChanges {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "+{} -{}", self.added, self.removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_added_and_removed_lines() {
        let factory = "a\nb\nc\nd";
        assert_eq!(LineChanges::between(factory, factory).to_string(), "+0 -0");
        assert_eq!(
            LineChanges::between(factory, "a\nx\nb\nc\nd").to_string(),
            "+1 -0"
        );
        assert_eq!(
            LineChanges::between(factory, "a\nB\nc").to_string(),
            "+1 -2"
        );
        assert_eq!(LineChanges::between(factory, "").to_string(), "+0 -4");
    }
}
