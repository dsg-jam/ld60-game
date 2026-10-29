use std::error::Error;

use bevy::{input_focus::tab_navigation::TabIndex, prelude::*, text::EditableText};
use ld60_locale::{Localized, Message};

use super::footer::{Command, Run, Status};
use super::page::{ActivePage, Page};
use super::undo::Undo;
use super::{Editor, PHOSPHOR, PHOSPHOR_DIM};
use crate::workshop::job::Job;
use ld60_game_device::{Connected, Emulator};

/// Words of the bitstream on each line.
const WORDS_PER_LINE: usize = 6;
/// Lines of the bitstream that come in each second.
const LINES_PER_SECOND: f32 = 40.0;
const BUTTON_PADDING: f32 = 4.0;
/// Lines of the bitstream that fit on the screen above the button. It
/// scrolls like a terminal as more come in.
const VISIBLE_LINES: usize = 18;

/// How far the player got with the firmware of the job. It has to come off
/// the device before it can be edited.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Dump {
    /// Nothing read yet.
    #[default]
    Empty,
    /// The raw bits, read off the device.
    Bits,
    /// The disassembled source, in the editor.
    Source,
}

/// The page that shows the firmware coming off the device.
#[derive(Component, Debug, Default)]
pub(super) struct DumpPage {
    lines: Vec<String>,
    /// The firmware the bits disassemble to.
    source: String,
    /// How many of the lines show, as they come in.
    shown: f32,
}

/// The part of the [`DumpPage`] above its button, which holds the text.
#[derive(Component, Debug)]
pub(super) struct DumpBody;

/// The text of the [`DumpPage`].
#[derive(Component, Debug)]
pub(super) struct DumpText;

impl DumpPage {
    pub(super) fn bundle(font: TextFont) -> impl Bundle {
        (
            DumpPage::default(),
            Page::Dump,
            Name::new("dump"),
            // Kept to the screen, so the button stays below the bits.
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_height: px(0),
                overflow: Overflow::clip(),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(BUTTON_PADDING * 2.0),
                ..default()
            },
            TabIndex(0),
            children![
                (
                    DumpBody,
                    Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        min_height: px(0),
                        overflow: Overflow::clip(),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![(
                        DumpText,
                        Localized::from(Message::new("editor-dump-empty")),
                        font.clone(),
                        TextColor(PHOSPHOR_DIM),
                    )],
                ),
                // The same command as in the footer, big enough to see what
                // comes next. `Command::show_labels` names it.
                (
                    Command::Dump,
                    Node {
                        border: px(1).all(),
                        padding: px(BUTTON_PADDING).all(),
                        ..default()
                    },
                    BorderColor::all(PHOSPHOR),
                    Localized::verbatim(""),
                    font,
                    TextColor(PHOSPHOR),
                ),
            ],
        )
    }

    /// The bits of `words`, a few words to a line after the address. The
    /// erased words after the firmware, all ones, fold into one line.
    fn lines(words: &[u16], bits: u8) -> Vec<String> {
        let erased = u16::MAX >> (16 - u32::from(bits));
        let used = words
            .iter()
            .rposition(|&word| word != erased)
            .map_or(0, |last| last + 1)
            .next_multiple_of(WORDS_PER_LINE)
            .min(words.len());
        let width = usize::from(bits);
        let mut lines: Vec<_> = words[..used]
            .chunks(WORDS_PER_LINE)
            .enumerate()
            .map(|(index, chunk)| {
                let bits = chunk
                    .iter()
                    .map(|word| format!("{word:0width$b}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{:04X}  {bits}", index * WORDS_PER_LINE)
            })
            .collect();
        if used < words.len() {
            lines.push(format!(
                "{used:04X}-{:04X}  {} ...",
                words.len() - 1,
                "1".repeat(width)
            ));
        }
        lines
    }

    /// Reads the firmware off the device, then disassembles it into the
    /// editor.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    pub(super) fn run_on_command(
        run: On<Run>,
        job: Option<Res<Job>>,
        mut dump: ResMut<Dump>,
        mut active: ResMut<ActivePage>,
        emulators: Query<&Emulator, With<Connected>>,
        mut pages: Query<&mut DumpPage>,
        editor: Single<(&mut EditableText, &mut Undo), With<Editor>>,
        mut status: Single<&mut Localized, With<Status>>,
    ) {
        if run.0 != Command::Dump {
            return;
        }
        let Some(job) = job else {
            **status = Message::new("editor-no-job").into();
            return;
        };
        match *dump {
            Dump::Empty => {
                let Ok(emulator) = emulators.single() else {
                    **status = Message::new("editor-no-device").into();
                    return;
                };
                let image = emulator.rom().image();
                let source = ld60_levels::dumped_source(&job.level, &job.buggy, image)
                    .unwrap_or_else(|err| {
                        error!(error = &err as &dyn Error, "firmware does not disassemble");
                        job.buggy.clone()
                    });
                for mut page in &mut pages {
                    page.lines = Self::lines(image.words(), image.word_bits());
                    page.source.clone_from(&source);
                    page.shown = 0.0;
                }
                *dump = Dump::Bits;
                **status = Message::new("editor-dumped")
                    .with("words", image.len())
                    .into();
            }
            Dump::Bits => {
                let Some(source) = pages.iter().next().map(|page| page.source.clone()) else {
                    return;
                };
                let (mut text, mut undo) = editor.into_inner();
                undo.replace(&mut text, &source);
                undo.reset();
                *dump = Dump::Source;
                active.0 = Page::Source;
                **status = Message::new("editor-disassembled").into();
            }
            Dump::Source => {}
        }
    }

    /// Lets the bitstream come in line by line.
    pub(super) fn stream(
        time: Res<Time>,
        dump: Res<Dump>,
        mut pages: Query<&mut DumpPage>,
        mut bodies: Query<&mut Node, With<DumpBody>>,
        mut texts: Query<(&mut Localized, &mut TextColor), With<DumpText>>,
    ) {
        // The message waits in the middle, the bits come in from the top.
        let justify = if *dump == Dump::Empty {
            JustifyContent::Center
        } else {
            JustifyContent::FlexStart
        };
        for mut body in &mut bodies {
            if body.justify_content != justify {
                body.justify_content = justify;
            }
        }
        for mut page in &mut pages {
            for (mut text, mut color) in &mut texts {
                if *dump == Dump::Empty {
                    text.set_if_neq(Message::new("editor-dump-empty").into());
                    color.set_if_neq(TextColor(PHOSPHOR_DIM));
                    continue;
                }
                let total = page.lines.len() as f32;
                if page.shown >= total && !dump.is_changed() {
                    continue;
                }
                page.shown = (page.shown + time.delta_secs() * LINES_PER_SECOND).min(total);
                let shown = page.shown as usize;
                let first = shown.saturating_sub(VISIBLE_LINES);
                text.set_if_neq(Localized::verbatim(page.lines[first..shown].join("\n")));
                color.set_if_neq(TextColor(PHOSPHOR));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_locale::Locale;

    use super::*;

    #[test]
    fn every_dump_text_formats() {
        Locale::on_disk().assert_formats([
            Message::new("editor-dump-empty"),
            Message::new("editor-dumped").with("words", 64),
            Message::new("editor-disassembled"),
        ]);
    }

    #[test]
    fn the_bitstream_shows_each_word_in_bits_after_the_address() {
        let lines = DumpPage::lines(&[0x3E, 0x0F, 1, 2, 3, 4, 5], 8);
        assert_eq!(
            lines,
            [
                "0000  00111110 00001111 00000001 00000010 00000011 00000100",
                "0006  00000101",
            ]
        );
    }

    #[test]
    fn the_erased_rest_folds_into_one_line() {
        let mut words = vec![0xFF; 32];
        words[..3].copy_from_slice(&[0x3E, 0x0F, 0xFF]);
        assert_eq!(
            DumpPage::lines(&words, 8),
            [
                "0000  00111110 00001111 11111111 11111111 11111111 11111111",
                "0006-001F  11111111 ...",
            ]
        );
    }
}
