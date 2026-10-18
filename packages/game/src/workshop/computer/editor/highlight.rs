use std::ops::Range;

use bevy::{prelude::*, text::EditableText};
use ld60_machine::AsmError;

use super::Editor;
use super::lexer::{Kind, Lexicon};
use crate::workshop::job::Job;

const MNEMONIC: Color = Color::srgb(0.82, 1.0, 0.84);
const DIRECTIVE: Color = Color::srgb(0.95, 1.0, 0.6);
const LABEL: Color = Color::srgb(0.6, 1.0, 0.3);
const REGISTER: Color = Color::srgb(0.45, 1.0, 0.55);
const PORT: Color = Color::srgb(0.35, 0.95, 0.85);
const NUMBER: Color = Color::srgb(0.55, 0.85, 0.7);
const COMMENT: Color = Color::srgb(0.24, 0.6, 0.31);
const PLAIN: Color = Color::srgb(0.35, 0.78, 0.43);
/// Underlines source the assembler rejects.
const ERROR: Color = Color::srgb(1.0, 0.8, 0.35);

/// The colored copy of the editor text.
///
/// `EditableText` draws all of its text in one color, so the editor text is
/// transparent and this node draws it again on top, one `TextSpan` per
/// [`Segment`].
#[derive(Component, Debug, Default)]
#[require(Text, TextLayout = TextLayout::no_wrap(), Pickable = Pickable::IGNORE)]
pub(super) struct Highlight {
    source: String,
    job: Option<usize>,
}

/// A run of source text drawn in one style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Segment {
    pub(super) range: Range<usize>,
    pub(super) kind: Kind,
    /// Part of the source the assembler rejects.
    pub(super) error: bool,
}

impl Highlight {
    pub(super) fn bundle(font: TextFont) -> impl Bundle {
        (
            Highlight::default(),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            font,
        )
    }

    /// Restyles the spans when the source or the job changes.
    pub(super) fn refresh(
        job: Option<Res<Job>>,
        editor: Single<Ref<EditableText>, With<Editor>>,
        overlay: Single<(Entity, &mut Highlight, &TextFont, Option<&Children>)>,
        mut spans: Query<(&mut TextSpan, &mut TextColor, Has<Underline>)>,
        mut commands: Commands,
    ) {
        let (root, mut highlight, font, children) = overlay.into_inner();
        let job_index = job.as_ref().map(|job| job.index);
        if !editor.is_changed() && job_index == highlight.job {
            return;
        }
        // Includes text the IME is still composing.
        let source = editor.editor().raw_text().to_owned();
        if source == highlight.source && job_index == highlight.job {
            return;
        }
        let isa = job.as_ref().map(|job| job.isa());
        let lexicon = isa
            .map(|isa| Lexicon::new(isa, &source))
            .unwrap_or_default();
        let errors = isa
            .and_then(|isa| isa.assemble(&source).err())
            .unwrap_or_default();
        let segments = Segment::split(&source, &lexicon, &errors);

        let children = children
            .map(|children| children.to_vec())
            .unwrap_or_default();
        for (index, segment) in segments.iter().enumerate() {
            let text = &source[segment.range.clone()];
            let color = TextColor(segment.color());
            let Some((entity, (mut span, mut span_color, underlined))) = children
                .get(index)
                .and_then(|&entity| Some((entity, spans.get_mut(entity).ok()?)))
            else {
                let mut span =
                    commands.spawn((TextSpan::new(text), font.clone(), color, ChildOf(root)));
                if segment.error {
                    span.insert((Underline, UnderlineColor(ERROR)));
                }
                continue;
            };
            if span.0 != text {
                span.0 = text.into();
            }
            span_color.set_if_neq(color);
            if segment.error && !underlined {
                commands
                    .entity(entity)
                    .insert((Underline, UnderlineColor(ERROR)));
            } else if !segment.error && underlined {
                commands
                    .entity(entity)
                    .remove::<(Underline, UnderlineColor)>();
            }
        }
        for &entity in children.iter().skip(segments.len()) {
            commands.entity(entity).despawn();
        }

        highlight.source = source;
        highlight.job = job_index;
    }

    /// Moves the spans with the scrolled editor text.
    pub(super) fn follow_scroll(
        editor: Single<&EditableText, With<Editor>>,
        mut overlay: Single<&mut Node, With<Highlight>>,
    ) {
        let offset = editor.viewport.offset;
        let (left, top) = (px(-offset.x), px(-offset.y));
        if overlay.left != left || overlay.top != top {
            overlay.left = left;
            overlay.top = top;
        }
    }
}

impl Segment {
    /// Splits `source` into segments that cover all of it.
    pub(super) fn split(source: &str, lexicon: &Lexicon, errors: &[AsmError]) -> Vec<Self> {
        let mut segments = Vec::new();
        let mut offset = 0;
        for (index, line) in source.split('\n').enumerate() {
            let tokens = lexicon.tokens(line);
            let marks: Vec<_> = errors
                .iter()
                .filter(|error| error.line == index + 1)
                .filter_map(|error| mark(line, error.span.clone()))
                .collect();
            let mut cuts: Vec<_> = tokens
                .iter()
                .map(|token| &token.range)
                .chain(&marks)
                .flat_map(|range| [range.start, range.end])
                .chain([0, line.len()])
                .collect();
            cuts.sort_unstable();
            cuts.dedup();
            for cut in cuts.windows(2) {
                let range = cut[0]..cut[1];
                let kind = tokens
                    .iter()
                    .find(|token| token.range.start <= range.start && range.end <= token.range.end)
                    .map_or(Kind::Text, |token| token.kind);
                let error = marks
                    .iter()
                    .any(|mark| mark.start <= range.start && range.end <= mark.end);
                let range = offset + range.start..offset + range.end;
                push(&mut segments, source, range, kind, error);
            }
            offset += line.len();
            if offset < source.len() {
                push(&mut segments, source, offset..offset + 1, Kind::Text, false);
                offset += 1;
            }
        }
        segments
    }

    fn color(&self) -> Color {
        match self.kind {
            Kind::Comment => COMMENT,
            Kind::Directive => DIRECTIVE,
            Kind::Label => LABEL,
            Kind::Mnemonic => MNEMONIC,
            Kind::Number => NUMBER,
            Kind::Port => PORT,
            Kind::Register => REGISTER,
            Kind::Punctuation | Kind::Text => PLAIN,
        }
    }
}

/// Appends a segment, or grows the last one if the style looks the same.
/// Whitespace has no color, so it joins any neighbor.
fn push(segments: &mut Vec<Segment>, source: &str, range: Range<usize>, kind: Kind, error: bool) {
    let blank = |range: &Range<usize>| source[range.clone()].trim().is_empty();
    if let Some(last) = segments.last_mut()
        && last.error == error
    {
        if last.kind == kind || blank(&range) {
            last.range.end = range.end;
            return;
        }
        if blank(&last.range) {
            last.kind = kind;
            last.range.end = range.end;
            return;
        }
    }
    segments.push(Segment { range, kind, error });
}

/// The part of `line` to underline for an error at `span`.
fn mark(line: &str, span: Range<usize>) -> Option<Range<usize>> {
    let floor = |mut index: usize| {
        index = index.min(line.len());
        while !line.is_char_boundary(index) {
            index -= 1;
        }
        index
    };
    let (start, end) = (floor(span.start), floor(span.end));
    if start < end {
        return Some(start..end);
    }
    // An empty span points between two characters. Mark the one before it.
    let before = line[..start].chars().next_back()?;
    Some(start - before.len_utf8()..start)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexicon() -> Lexicon {
        Lexicon::new(ld60_calco::Revision::C.isa(), "")
    }

    fn styles<'a>(source: &'a str, errors: &[AsmError]) -> Vec<(&'a str, Kind, bool)> {
        Segment::split(source, &lexicon(), errors)
            .into_iter()
            .map(|segment| (&source[segment.range], segment.kind, segment.error))
            .collect()
    }

    fn error(line: usize, span: Range<usize>) -> AsmError {
        AsmError {
            line,
            span,
            message: String::new(),
        }
    }

    #[test]
    fn covers_the_whole_source() {
        for source in ["", "\n", "NOP\n\n  LDI A, 1 ; x\n", "\u{fc}:\u{e9}"] {
            let segments = Segment::split(source, &lexicon(), &[]);
            let joined: String = segments.iter().map(|s| &source[s.range.clone()]).collect();
            assert_eq!(joined, source);
            assert!(
                segments
                    .windows(2)
                    .all(|w| w[0].range.end == w[1].range.start)
            );
        }
    }

    #[test]
    fn joins_whitespace_with_neighbors() {
        use Kind::*;
        assert_eq!(
            styles("  LDI  A, 1\nNOP", &[]),
            vec![
                ("  LDI  ", Mnemonic, false),
                ("A", Register, false),
                (", ", Punctuation, false),
                ("1\n", Number, false),
                ("NOP", Mnemonic, false),
            ]
        );
    }

    #[test]
    fn underlines_the_span() {
        use Kind::*;
        let source = "NOP\nLDI A, 0x1FF ; big";
        assert_eq!(
            styles(source, &[error(2, 7..12)]),
            vec![
                ("NOP\nLDI ", Mnemonic, false),
                ("A", Register, false),
                (", ", Punctuation, false),
                ("0x1FF", Number, true),
                (" ; big", Comment, false),
            ]
        );
    }

    #[test]
    fn widens_empty_spans() {
        assert_eq!(mark("LDI A,", 6..6), Some(5..6));
        assert_eq!(mark("A\u{fc}", 3..9), Some(1..3));
        assert_eq!(mark("A\u{fc}", 2..2), Some(0..1));
        assert_eq!(mark("", 0..0), None);
    }
}
