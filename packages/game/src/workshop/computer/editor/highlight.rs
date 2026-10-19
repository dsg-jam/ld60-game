use std::collections::HashMap;
use std::ops::Range;

use bevy::{
    math::Affine2,
    prelude::*,
    text::{EditableText, TextBackgroundColor, TextLineYBounds, TextViewport},
    ui::UiGlobalTransform,
};
use ld60_machine::AsmError;

use super::Editor;
use super::lexer::{Kind, Lexicon};
use super::shades::{Shade, Shades};
use crate::workshop::job::{Job, JobId};

const MNEMONIC: Color = Color::srgb(0.82, 1.0, 0.84);
const DIRECTIVE: Color = Color::srgb(0.95, 1.0, 0.6);
const LABEL: Color = Color::srgb(0.6, 1.0, 0.3);
const REGISTER: Color = Color::srgb(0.45, 1.0, 0.55);
const PORT: Color = Color::srgb(0.35, 0.95, 0.85);
const NUMBER: Color = Color::srgb(0.55, 0.85, 0.7);
/// At least 3:1 against the selection color of the editor.
const COMMENT: Color = Color::srgb(0.28, 0.68, 0.36);
const PLAIN: Color = Color::srgb(0.35, 0.78, 0.43);
/// Underlines source the assembler rejects.
const ERROR: Color = Color::srgb(1.0, 0.8, 0.35);
/// Lines styled above and below the visible ones. The styled lines only
/// change when the view scrolls past a multiple of this.
const MARGIN_LINES: usize = 16;

/// The colored copy of the visible editor text.
///
/// `EditableText` draws all of its text in one color, so the editor text is
/// transparent and this node draws the visible lines again on top, one
/// `TextSpan` per [`Segment`]. The root text holds the empty lines above
/// them.
#[derive(Component, Debug, Default)]
#[require(Text, TextLayout = TextLayout::no_wrap(), Pickable = Pickable::IGNORE)]
pub(super) struct Highlight {
    source: String,
    job: Option<JobId>,
    lexicon: Lexicon,
    marks: Marks,
    /// The source lines the spans show.
    lines: Range<usize>,
    /// Distance between two lines of the editor text, in physical pixels.
    pitch: f32,
}

/// A run of source text drawn in one style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Segment {
    pub(super) range: Range<usize>,
    pub(super) kind: Kind,
    /// Part of the source the assembler rejects.
    pub(super) error: bool,
    pub(super) shade: Shade,
}

/// The byte ranges of assembler errors, by 0-based line.
#[derive(Debug, Default)]
pub(super) struct Marks(HashMap<usize, Vec<Range<usize>>>);

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

    /// Restyles the spans when the source, the job or the visible lines
    /// change.
    ///
    /// Runs after the edits of this frame and before UI layout, so the spans
    /// match the editor text in the same frame.
    pub(super) fn refresh(
        job: Option<Res<Job>>,
        shades: Res<Shades>,
        editor: Single<Ref<EditableText>, With<Editor>>,
        overlay: Single<(
            Entity,
            &mut Highlight,
            &mut Text,
            &TextFont,
            Option<&Children>,
        )>,
        mut spans: Query<(
            &mut TextSpan,
            &mut TextColor,
            &mut TextBackgroundColor,
            Has<Underline>,
        )>,
        mut commands: Commands,
    ) {
        let (root, mut highlight, mut padding, font, children) = overlay.into_inner();
        let highlight = &mut *highlight;
        let job_id = job.as_ref().map(|job| job.id);
        // Includes text the IME is still composing.
        let source = editor.editor().raw_text();
        let changed =
            job_id != highlight.job || (editor.is_changed() && source != highlight.source);
        if changed {
            let isa = job.as_ref().map(|job| job.isa());
            highlight.lexicon = isa.map(|isa| Lexicon::new(isa, source)).unwrap_or_default();
            let errors = isa
                .and_then(|isa| isa.assemble(source).err())
                .unwrap_or_default();
            highlight.marks = Marks::new(&errors);
            highlight.source = source.to_owned();
            highlight.job = job_id;
        }
        let lines = highlight.visible(&editor.viewport);
        if !changed && !shades.is_changed() && lines == highlight.lines {
            return;
        }
        highlight.lines = lines.clone();
        // Empty lines above the styled ones keep the line positions of the
        // editor layout.
        let padding_lines = lines.start.min(highlight.source.matches('\n').count());
        if padding.0.len() != padding_lines {
            padding.0 = "\n".repeat(padding_lines);
        }
        let source = &highlight.source;
        let segments = Segment::split(source, &highlight.lexicon, &highlight.marks, &shades, lines);

        let children = children
            .map(|children| children.to_vec())
            .unwrap_or_default();
        for (index, segment) in segments.iter().enumerate() {
            let text = &source[segment.range.clone()];
            let color = TextColor(segment.color());
            let background = TextBackgroundColor(segment.shade.color());
            let Some((entity, (mut span, mut span_color, mut span_background, underlined))) =
                children
                    .get(index)
                    .and_then(|&entity| Some((entity, spans.get_mut(entity).ok()?)))
            else {
                let mut span = commands.spawn((
                    TextSpan::new(text),
                    font.clone(),
                    color,
                    background,
                    ChildOf(root),
                ));
                if segment.error {
                    span.insert((Underline, UnderlineColor(ERROR)));
                }
                continue;
            };
            if span.0 != text {
                span.0 = text.into();
            }
            span_color.set_if_neq(color);
            span_background.set_if_neq(background);
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
    }

    /// Puts the spans over the editor text.
    ///
    /// The editor scrolls until late in the frame, after UI layout. This
    /// runs after that and moves the laid out node to the final scroll
    /// position, with the same transform the editor text gets.
    pub(super) fn place(
        editor: Single<(&EditableText, &ComputedNode, &UiGlobalTransform), With<Editor>>,
        overlay: Single<(&mut Highlight, &ComputedNode, &mut UiGlobalTransform), Without<Editor>>,
    ) {
        let (text, editor_node, editor_transform) = editor.into_inner();
        let (mut highlight, node, mut transform) = overlay.into_inner();
        if let Some(layout) = text.editor().try_layout() {
            let mut tops = layout
                .lines()
                .map(|line| TextLineYBounds::from_line(&line).min);
            if let (Some(first), Some(second)) = (tops.next(), tops.next()) {
                highlight.pitch = second - first;
            }
        }
        let shift = editor_node.content_box().min - text.viewport.offset - node.content_box().min;
        transform.set_if_neq(UiGlobalTransform::from(
            **editor_transform * Affine2::from_translation(shift),
        ));
    }

    /// The 0-based line at `y` physical pixels below the top of the editor
    /// text, once the line height is known.
    pub(super) fn line_at(&self, y: f32) -> Option<usize> {
        (self.pitch > 0.0 && y >= 0.0).then(|| (y / self.pitch) as usize)
    }

    /// The lines to style for the editor view. All lines until the line
    /// height is known.
    fn visible(&self, viewport: &TextViewport) -> Range<usize> {
        if self.pitch <= 0.0 {
            return 0..usize::MAX;
        }
        let first = (viewport.offset.y / self.pitch) as usize;
        let count = (viewport.size.y / self.pitch).ceil() as usize + 1;
        let start = first.saturating_sub(MARGIN_LINES) / MARGIN_LINES * MARGIN_LINES;
        start..start + count + 3 * MARGIN_LINES
    }
}

impl Segment {
    /// Splits `lines` of `source` into segments that cover all of them,
    /// including their line breaks.
    pub(super) fn split(
        source: &str,
        lexicon: &Lexicon,
        marks: &Marks,
        shades: &Shades,
        lines: Range<usize>,
    ) -> Vec<Self> {
        let mut segments = Vec::new();
        let mut offset = 0;
        for (index, line) in source.split('\n').enumerate().take(lines.end) {
            let start = offset;
            offset += line.len() + 1;
            if index < lines.start {
                continue;
            }
            let shade = shades.line(index);
            let tokens = lexicon.tokens(line);
            let marked: Vec<_> = marks
                .line(index)
                .iter()
                .filter_map(|span| mark(line, span.clone()))
                .collect();
            let mut cuts: Vec<_> = tokens
                .iter()
                .map(|token| &token.range)
                .chain(&marked)
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
                let error = marked
                    .iter()
                    .any(|mark| mark.start <= range.start && range.end <= mark.end);
                let range = start + range.start..start + range.end;
                push(&mut segments, source, range, kind, error, shade);
            }
            let end = start + line.len();
            if end < source.len() {
                push(
                    &mut segments,
                    source,
                    end..end + 1,
                    Kind::Text,
                    false,
                    shade,
                );
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

impl Marks {
    pub(super) fn new(errors: &[AsmError]) -> Self {
        let mut lines = HashMap::<usize, Vec<_>>::new();
        for error in errors {
            lines
                .entry(error.line.saturating_sub(1))
                .or_default()
                .push(error.span.clone());
        }
        Self(lines)
    }

    fn line(&self, index: usize) -> &[Range<usize>] {
        self.0.get(&index).map_or(&[], Vec::as_slice)
    }
}

/// Appends a segment, or grows the last one if the style looks the same.
/// Whitespace has no color, so it joins any neighbor.
fn push(
    segments: &mut Vec<Segment>,
    source: &str,
    range: Range<usize>,
    kind: Kind,
    error: bool,
    shade: Shade,
) {
    let blank = |range: &Range<usize>| source[range.clone()].trim().is_empty();
    if let Some(last) = segments.last_mut()
        && last.error == error
        && last.shade == shade
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
    segments.push(Segment {
        range,
        kind,
        error,
        shade,
    });
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
    use ld60_machine::AsmErrorKind;

    use super::*;

    const ALL: Range<usize> = 0..usize::MAX;

    fn styles<'a>(source: &'a str, errors: &[AsmError]) -> Vec<(&'a str, Kind, bool)> {
        let lexicon = Lexicon::new(ld60_calco::Revision::C.isa(), source);
        Segment::split(
            source,
            &lexicon,
            &Marks::new(errors),
            &Shades::default(),
            ALL,
        )
        .into_iter()
        .map(|segment| (&source[segment.range], segment.kind, segment.error))
        .collect()
    }

    fn error(line: usize, span: Range<usize>) -> AsmError {
        AsmError {
            line,
            span,
            kind: AsmErrorKind::RomFull,
        }
    }

    fn joined(source: &str, lines: Range<usize>) -> String {
        let lexicon = Lexicon::new(ld60_calco::Revision::C.isa(), source);
        let segments = Segment::split(
            source,
            &lexicon,
            &Marks::default(),
            &Shades::default(),
            lines,
        );
        assert!(
            segments
                .windows(2)
                .all(|w| w[0].range.end == w[1].range.start)
        );
        segments.iter().map(|s| &source[s.range.clone()]).collect()
    }

    #[test]
    fn covers_the_whole_source() {
        for source in ["", "\n", "NOP\n\n  LDI A, 1 ; x\n", "\u{fc}:\u{e9}"] {
            assert_eq!(joined(source, ALL), source);
        }
    }

    #[test]
    fn covers_only_the_given_lines() {
        let source = "a: NOP\nLDI A, 1\n\nJMP a\n";
        assert_eq!(joined(source, 1..3), "LDI A, 1\n\n");
        assert_eq!(joined(source, 3..9), "JMP a\n");
        assert_eq!(joined(source, 7..9), "");
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
    fn underlines_ops_the_revision_lacks() {
        use ld60_calco::Revision;
        use ld60_machine::Isa;
        let source = "INC\nADC M1\n";
        assert!(Revision::B.isa().assemble(source).is_ok());
        let isa = Revision::A.isa();
        let errors = isa.assemble(source).unwrap_err();
        assert_eq!(
            errors[0].kind,
            AsmErrorKind::Unsupported {
                form: "ADC".into(),
                isa: "CalCo Rev A",
            }
        );
        let lexicon = Lexicon::new(isa, source);
        let underlined: Vec<&str> = Segment::split(
            source,
            &lexicon,
            &Marks::new(&errors),
            &Shades::default(),
            ALL,
        )
        .into_iter()
        .filter(|segment| segment.error)
        .map(|segment| &source[segment.range])
        .collect();
        assert_eq!(underlined, ["ADC"]);
    }

    #[test]
    fn widens_empty_spans() {
        assert_eq!(mark("LDI A,", 6..6), Some(5..6));
        assert_eq!(mark("A\u{fc}", 3..9), Some(1..3));
        assert_eq!(mark("A\u{fc}", 2..2), Some(0..1));
        assert_eq!(mark("", 0..0), None);
    }

    #[test]
    fn keeps_shaded_lines_apart() {
        use crate::workshop::computer::editor::listing::Listing;
        use ld60_machine::Isa;

        let source = "NOP\nNOP\nNOP";
        let isa = ld60_calco::Revision::C.isa();
        let listing = Listing::new(isa.assemble(source).unwrap());
        let shades = Shades::new(&listing, [1], [], None);
        let lexicon = Lexicon::new(isa, source);
        let segments = Segment::split(source, &lexicon, &Marks::default(), &shades, ALL);
        let shaded: Vec<_> = segments
            .iter()
            .map(|segment| (&source[segment.range.clone()], segment.shade))
            .collect();
        assert_eq!(
            shaded,
            [
                ("NOP\n", Shade::Plain),
                ("NOP\n", Shade::Traced),
                ("NOP", Shade::Plain)
            ]
        );
    }
}
