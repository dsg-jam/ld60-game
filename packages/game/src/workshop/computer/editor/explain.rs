use bevy::{prelude::*, text::EditableText};
use ld60_calco::datasheet::{INSTRUCTIONS, InstructionDoc};
use ld60_locale::{Localized, Message};

use super::Editor;
use super::dump::Dump;
use crate::workshop::job::Job;
use crate::workshop::levels::Levels;
use crate::workshop::run::Origin;

/// A line under the editor that says what the instruction under the cursor
/// does, as the datasheet says it.
#[derive(Component, Debug)]
pub(super) struct Explain;

/// The hint of a tutorial job, over the [`Explain`] line.
#[derive(Component, Debug)]
pub(super) struct HintLine;

impl Explain {
    pub(super) fn update(
        editor: Single<&EditableText, With<Editor>>,
        dump: Res<Dump>,
        mut lines: Query<&mut Localized, With<Explain>>,
    ) {
        let text = (*dump == Dump::Source)
            .then(|| {
                let editor = editor.editor();
                let source = editor.raw_text();
                let caret = editor.raw_selection().focus().index();
                let start = source.get(..caret)?.rfind('\n').map_or(0, |at| at + 1);
                let end = source[start..]
                    .find('\n')
                    .map_or(source.len(), |at| start + at);
                Self::explain(&source[start..end])
            })
            .flatten()
            .unwrap_or_default();
        for mut line in &mut lines {
            line.set_if_neq(Localized::verbatim(text.clone()));
        }
    }

    /// The datasheet example and meaning of the instruction on `line`, if
    /// it has one.
    fn explain(line: &str) -> Option<String> {
        let code = line.split(';').next()?;
        // Past a label, like `count:`.
        let code = code.rsplit(':').next()?.trim();
        let (mnemonic, operands) = code.split_once(char::is_whitespace).unwrap_or((code, ""));
        let mnemonic = match mnemonic.to_uppercase().as_str() {
            "HLT" => "WAIT".to_owned(),
            other => other.to_owned(),
        };
        let operands = operands.trim().to_uppercase();
        let doc = INSTRUCTIONS
            .iter()
            .find(|doc| Self::describes(doc, &mnemonic, &operands))?;
        Some(format!("{}  {}", doc.example, doc.meaning))
    }

    /// Whether `doc` is the row of `mnemonic` with these `operands`.
    fn describes(doc: &InstructionDoc, mnemonic: &str, operands: &str) -> bool {
        let mut syntax = doc.syntax.split_whitespace();
        if syntax.next() != Some(mnemonic) {
            return false;
        }
        let form = syntax.collect::<Vec<_>>().join(" ");
        if form.starts_with('[') || operands.starts_with('[') {
            return form.starts_with('[') == operands.starts_with('[');
        }
        // `LDI A, ii` and `LDI B, ii` differ in the register.
        match form.split_once(',') {
            Some((register, _)) => operands.starts_with(register),
            None => true,
        }
    }
}

impl HintLine {
    /// Shows the hint of a tutorial job once its firmware is in the editor.
    pub(super) fn update(
        job: Option<Res<Job>>,
        dump: Res<Dump>,
        mut lines: Query<(&mut Localized, &mut Node), With<HintLine>>,
    ) {
        let hint = job
            .filter(|job| *dump == Dump::Source && matches!(job.origin, Origin::Lesson { .. }))
            .filter(|job| job.level.firmware.is_some())
            .map(|job| Levels::text(&job.level, "hint"));
        for (mut line, mut node) in &mut lines {
            let display = if hint.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
            if let Some(hint) = &hint {
                line.set_if_neq(Message::clone(hint).into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explains_the_instruction_on_a_line() {
        let meaning = |line| Explain::explain(line).unwrap();
        assert_eq!(
            meaning("count:  WAIT"),
            "WAIT  Wait for the next key press."
        );
        assert_eq!(
            meaning("        LD   M1   ; left"),
            "LD M2  Copy cell M2 to A."
        );
        assert_eq!(
            meaning("ld [b+3]"),
            "LD [B+2]  Copy the cell 2 after B to A. With B at 3, that is M5."
        );
        assert_eq!(meaning("LDI B, 4"), "LDI B, 4  Put 4 in B.");
        assert_eq!(meaning("hlt"), "WAIT  Wait for the next key press.");
        assert_eq!(Explain::explain("; just a comment"), None);
        assert_eq!(Explain::explain("start:"), None);
    }
}
