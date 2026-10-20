use bevy::{
    picking::{
        events::{PointerClick, PointerEnter, PointerLeave},
        pointer::PointerButton,
    },
    prelude::*,
};

use super::contents::{Contents, Entry, Page, Section};
use super::in_file::InFile;
use super::moves::MoveSheet;
use super::pins::Sheet;
use crate::locale::{Localized, Message};
use crate::parts::paper::{INK, PAPER, Paper, PaperButton};

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

/// The button at the top of a page that pins the page to the monitor of
/// the shop computer.
#[derive(Component, Debug)]
pub(super) struct PinButton(pub(super) Sheet);

/// Where the links on the pages of `file` go.
pub(super) struct Links<'a> {
    pub(super) file: Entity,
    pub(super) in_file: &'a InFile,
    /// Whether the monitor has no room for another pinned page.
    pub(super) rail_full: bool,
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

    /// A button with a frame, filled so it stands out on the band. Hovering
    /// it shows `hint`.
    fn button(hint: Message) -> impl Bundle {
        (
            Node {
                padding: UiRect::axes(px(10), px(2)),
                border: px(2).all(),
                ..default()
            },
            BorderColor::all(INK),
            BackgroundColor(PAPER),
            Self { idle: PAPER },
            PaperButton(Some(hint)),
        )
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

impl PinButton {
    pub(super) fn label() -> Message {
        Message::new("manuals-pin")
    }

    pub(super) fn pin_on_click(
        click: On<PointerClick>,
        buttons: Query<&PinButton>,
        mut commands: Commands,
    ) {
        if click.button != PointerButton::Primary {
            return;
        }
        if let Ok(button) = buttons.get(click.entity) {
            commands.trigger(MoveSheet::Pin(button.0));
        }
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
    /// with a band in the color of its section, a button that pins the page
    /// and a link back to the index.
    pub(super) fn write(
        &self,
        (page, sheet): (Page, Option<Sheet>),
        links: &Links,
        content: &mut ChildSpawnerCommands,
    ) {
        match sheet {
            None => self.write_index(links, content),
            Some(sheet) => {
                let pin = if links.rail_full {
                    "hint-pin-full"
                } else {
                    "hint-pin"
                };
                content.spawn(sheet.band()).with_child((
                    Node {
                        column_gap: px(8),
                        ..default()
                    },
                    children![
                        (
                            Hover::button(Message::new(pin)),
                            PinButton(sheet),
                            children![Paper::text(PinButton::label())]
                        ),
                        (
                            Hover::button(Message::new("hint-open-index")),
                            PageLink {
                                file: links.file,
                                page: Some(0),
                            },
                            children![Paper::text(Message::new("manuals-index"))],
                        ),
                    ],
                ));
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
        page.spawn(Paper::title(Message::new("manuals-spine")));
        page.spawn(Paper::faded(Message::new("manuals-index-hint")));
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
