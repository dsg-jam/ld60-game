use bevy::{
    picking::{
        events::{PointerClick, PointerEnter, PointerLeave},
        pointer::PointerButton,
    },
    prelude::*,
};

use super::contents::{Contents, Page, Section};
use super::pins::Sheet;
use crate::locale::{Localized, Message};
use crate::parts::paper::{INK, PAPER, Paper, PaperButton};
use crate::workshop::run::Run;

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
/// opens `page` of the manuals file `file`.
#[derive(Component, Debug)]
pub(super) struct PageLink {
    pub(super) file: Entity,
    pub(super) page: usize,
}

/// The button at the top of a page that pins the page to the monitor of
/// the shop computer, or takes it off again.
#[derive(Component, Debug)]
pub(super) struct PinButton(pub(super) Sheet);

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

    /// A button with a frame, filled so it stands out on the band.
    fn button() -> impl Bundle {
        (
            Node {
                padding: UiRect::axes(px(10), px(2)),
                border: px(2).all(),
                ..default()
            },
            BorderColor::all(INK),
            BackgroundColor(PAPER),
            Self { idle: PAPER },
            PaperButton::default(),
        )
    }

    /// A line of the index or a link in it.
    fn line(node: Node) -> impl Bundle {
        (
            node,
            BackgroundColor(Color::NONE),
            Self { idle: Color::NONE },
            PaperButton::default(),
        )
    }
}

impl PinButton {
    pub(super) fn label(pinned: bool) -> Message {
        Message::new(if pinned {
            "manuals-unpin"
        } else {
            "manuals-pin"
        })
    }

    pub(super) fn toggle_on_click(
        click: On<PointerClick>,
        buttons: Query<&PinButton>,
        mut run: ResMut<Run>,
    ) {
        if click.button != PointerButton::Primary {
            return;
        }
        if let Ok(button) = buttons.get(click.entity) {
            run.pins_mut().toggle(button.0);
        }
    }

    /// Says whether the page is pinned.
    pub(super) fn show_state(
        run: Res<Run>,
        buttons: Query<(Ref<PinButton>, &Children)>,
        mut labels: Query<&mut Localized>,
    ) {
        for (button, children) in &buttons {
            if !run.is_changed() && !button.is_added() {
                continue;
            }
            let label = Self::label(run.pins().contains(button.0));
            for &child in children {
                if let Ok(mut text) = labels.get_mut(child) {
                    text.set_if_neq(label.clone().into());
                }
            }
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
        let Ok(link) = links.get(click.entity) else {
            return;
        };
        if let Ok(mut paper) = papers.get_mut(link.file) {
            paper.open_page(link.page);
        }
    }
}

impl Contents {
    /// Writes `page` of the manuals file `file`. Every page but the index
    /// starts with a band in the color of its section, a button that pins
    /// the page and a link back to the index.
    pub(super) fn write(
        &self,
        file: Entity,
        (page, sheet): (Page, Option<Sheet>),
        content: &mut ChildSpawnerCommands,
    ) {
        match sheet {
            None => self.write_index(file, content),
            Some(sheet) => {
                content.spawn(sheet.band()).with_child((
                    Node {
                        column_gap: px(8),
                        ..default()
                    },
                    children![
                        (
                            Hover::button(),
                            PinButton(sheet),
                            children![Paper::text(PinButton::label(false))]
                        ),
                        (
                            Hover::button(),
                            PageLink { file, page: 0 },
                            children![Paper::text(Message::new("manuals-index"))],
                        ),
                    ],
                ));
                page.write(content);
            }
        }
    }

    fn write_index(&self, file: Entity, page: &mut ChildSpawnerCommands) {
        page.spawn(Paper::title(Message::new("manuals-spine")));
        page.spawn(Paper::faded(Message::new("manuals-index-hint")));
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
                page.spawn((
                    Hover::line(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        padding: LINE_PADDING,
                        ..default()
                    }),
                    PageLink { file, page: start },
                    children![
                        Paper::text(entry.label()),
                        Paper::text(Localized::verbatim((start + 1).to_string())),
                    ],
                ));
                let links: Vec<_> = entry
                    .pages()
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, page)| Some((start + index, page.short_name()?)))
                    .collect();
                if links.is_empty() {
                    continue;
                }
                page.spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(6),
                    margin: UiRect::left(px(PAGE_LINKS_INDENT)),
                    ..default()
                })
                .with_children(|row| {
                    for (page, name) in links {
                        row.spawn((
                            Hover::line(Node {
                                padding: LINE_PADDING,
                                ..default()
                            }),
                            PageLink { file, page },
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
