use bevy::{
    picking::{
        events::{PointerClick, PointerEnter, PointerLeave},
        pointer::PointerButton,
    },
    prelude::*,
};
use ld60_game_core::parts::paper::{Paper, PaperButton};
use ld60_locale::{Localized, Message};

use super::contents::{Contents, Entry, Page, Section};
use super::in_file::InFile;
use super::pins::Sheet;

/// Under the pointer, buttons and index lines take this color.
const HOVER: Color = Color::srgba_u8(0x2a, 0x26, 0x22, 0x30);

const BAND_PADDING: UiRect = UiRect {
    left: Val::Px(10.0),
    right: Val::Px(10.0),
    top: Val::Px(4.0),
    bottom: Val::Px(4.0),
};
const LINE_PADDING: UiRect = UiRect {
    left: Val::Px(10.0),
    right: Val::Px(10.0),
    top: Val::Px(2.0),
    bottom: Val::Px(2.0),
};
/// The links to the pages of a manual sit under its line in the index.
const PAGE_LINKS_INDENT: f32 = 30.0;

/// A line of the index, or the INDEX button at the top of a page. A click
/// opens `page` of the manuals file `file`, unless it is pinned to the
/// monitor.
#[derive(Component, Debug)]
pub(super) struct PageLink {
    pub(super) file: Entity,
    pub(super) page: Option<usize>,
}

/// Where the links on the pages of `file` go.
pub(super) struct Links<'a> {
    pub(super) file: Entity,
    pub(super) in_file: &'a InFile,
}

/// Something to click on a page that shows when the pointer is over it.
#[derive(Component, Debug)]
pub(super) struct Hover {
    idle: Color,
}

impl Hover {
    pub(super) fn show_enter(
        enter: On<PointerEnter>,
        mut nodes: Query<&mut BackgroundColor, With<Hover>>,
    ) {
        if let Ok(mut background) = nodes.get_mut(enter.entity) {
            background.0 = HOVER;
        }
    }

    pub(super) fn show_leave(
        leave: On<PointerLeave>,
        mut nodes: Query<(&Hover, &mut BackgroundColor)>,
    ) {
        if let Ok((hover, mut background)) = nodes.get_mut(leave.entity) {
            background.0 = hover.idle;
        }
    }

    /// A line of the index or a link in it.
    fn line(node: Node, hint: Message) -> impl Bundle {
        (
            node,
            BackgroundColor(Color::NONE),
            Self { idle: Color::NONE },
            PaperButton(Some(hint)),
        )
    }
}

impl PageLink {
    pub(super) fn open_on_click(
        click: On<PointerClick>,
        links: Query<&PageLink>,
        mut papers: Query<&mut Paper>,
    ) {
        if click.button != PointerButton::Primary {
            return;
        }
        let Some((file, page)) = links
            .get(click.entity)
            .ok()
            .and_then(|link| Some((link.file, link.page?)))
        else {
            return;
        };
        if let Ok(mut paper) = papers.get_mut(file) {
            paper.open_page(page);
        }
    }
}

impl Contents {
    /// Writes `page` of the manuals file. Every page but the index starts
    /// with a band in the color of its section.
    pub(super) fn write(
        &self,
        (page, sheet): (Page, Option<Sheet>),
        links: &Links,
        content: &mut ChildSpawnerCommands,
    ) {
        match sheet {
            None => self.write_index(links, content),
            Some(sheet) => {
                content.spawn(sheet.band());
                page.write(content);
            }
        }
    }

    /// The first page of `entry` in the file, if any.
    fn first_in_file(&self, entry: Entry, in_file: &InFile) -> Option<usize> {
        let start = self.start_of(entry)?;
        let page = in_file.from(start)?;
        (in_file.number(page)? < start + entry.pages().len()).then_some(page)
    }

    fn write_index(&self, links: &Links, page: &mut ChildSpawnerCommands) {
        let link = |target: Option<usize>, name: Localized| {
            let hint = match target {
                Some(_) => Message::new("hint-open").with("target", name),
                None => Message::new("hint-pinned"),
            };
            (
                PageLink {
                    file: links.file,
                    page: target,
                },
                hint,
            )
        };
        for section in Section::ALL {
            page.spawn((
                Node {
                    padding: BAND_PADDING,
                    margin: UiRect::top(px(8)),
                    ..default()
                },
                BackgroundColor(section.color()),
                children![Paper::heading(section.label())],
            ));
            let mut entries = self
                .entries()
                .iter()
                .filter(|entry| entry.section() == section)
                .peekable();
            if entries.peek().is_none()
                && let Some(empty) = section.empty()
            {
                page.spawn((
                    Node {
                        padding: LINE_PADDING,
                        ..default()
                    },
                    children![Paper::faded(empty)],
                ));
            }
            for &entry in entries {
                let Some(start) = self.start_of(entry) else {
                    continue;
                };
                let (target, hint) = link(self.first_in_file(entry, links.in_file), entry.label());
                page.spawn((
                    Hover::line(
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                            padding: LINE_PADDING,
                            ..default()
                        },
                        hint,
                    ),
                    target,
                    children![
                        Paper::text(entry.label()),
                        Paper::text(Localized::verbatim((start + 1).to_string())),
                    ],
                ));
                let names: Vec<_> = entry
                    .pages()
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, page)| Some((start + index, page.short_name()?)))
                    .collect();
                if names.is_empty() {
                    continue;
                }
                page.spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(6),
                    margin: UiRect::left(px(PAGE_LINKS_INDENT)),
                    ..default()
                })
                .with_children(|row| {
                    for (number, name) in names {
                        let (target, hint) = link(links.in_file.page(number), name.clone());
                        row.spawn((
                            Hover::line(
                                Node {
                                    padding: LINE_PADDING,
                                    ..default()
                                },
                                hint,
                            ),
                            target,
                            children![Paper::text(name)],
                        ));
                    }
                });
            }
        }
    }
}

impl Sheet {
    /// The band at the top of the page in the color of its section, with
    /// the names of the section, the entry and the page.
    pub(super) fn band(self) -> impl Bundle {
        let entry = self.entry();
        let names = [
            Some(entry.section().label().into()),
            Some(entry.label()),
            self.page().and_then(Page::heading),
        ];
        (
            Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: BAND_PADDING,
                ..default()
            },
            BackgroundColor(entry.section().color()),
            children![(
                Node {
                    column_gap: px(14),
                    ..default()
                },
                Children::spawn(SpawnIter(names.into_iter().flatten().map(Paper::heading))),
            )],
        )
    }
}
