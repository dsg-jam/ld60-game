//! The divider cards with tabs: one in front of the index and one in front
//! of each section.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};
use ld60_game_parts::paper::{INK, Paper, PaperButton};
use ld60_game_parts::screen::UiCanvas;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::{Finish, Form, SurfaceMaterial};
use ld60_locale::{Localized, Message};

use super::contents::{Contents, Section};
use super::model::{HOLE_RADIUS, SHEET_SIZE};
use super::punched::punched_box;
use super::stack::{CARD, holes, stray};

const INDEX: Color = Color::srgb_u8(0xcc, 0xc6, 0xb8);
/// How far a divider reaches past the pages on the other edges, so its
/// color shows on the side of the stack.
const LIP: f32 = 0.0006;
/// Length of a tab along the outer edge of the pages.
const TAB_LENGTH: f32 = 0.04;
/// How far a tab reaches out of the outer edge of the pages.
pub(super) const TAB_OUT: f32 = 0.014;
/// Room for each tab along the edge, so they don't cover each other.
const TAB_SLOT: f32 = SHEET_SIZE.y / Tab::ALL.len() as f32;
const TAB_LABEL: UVec2 = UVec2::new(200, 64);
const TAB_FONT_SIZE: f32 = 30.0;

/// What a divider stands in front of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tab {
    Index,
    Section(Section),
}

/// A divider card on the rings of `file`.
#[derive(Component, Debug)]
pub(super) struct DividerTab {
    pub(super) file: Entity,
}

/// The tab of a divider. A click on it opens page `start` of `file`.
#[derive(Component, Debug)]
pub(super) struct TabLink {
    file: Entity,
    start: usize,
}

impl Tab {
    pub(super) const ALL: [Self; 5] = [
        Self::Index,
        Self::Section(Section::Gadgets),
        Self::Section(Section::Chips),
        Self::Section(Section::Errata),
        Self::Section(Section::Notes),
    ];

    pub(super) fn label(self) -> Message {
        match self {
            Self::Index => Message::new("manuals-index"),
            Self::Section(section) => section.label(),
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Index => INDEX,
            Self::Section(section) => section.color(),
        }
    }

    /// Picks how the edges of its divider stray.
    pub(super) fn seed(self) -> u64 {
        1000 + self.slot() as u64
    }

    fn slot(self) -> usize {
        Self::ALL.iter().position(|&tab| tab == self).unwrap_or(0)
    }

    /// The page behind the divider, if it has one.
    pub(super) fn start(self, contents: &Contents) -> Option<usize> {
        match self {
            Self::Index => Some(0),
            Self::Section(section) => contents.start_of_section(section),
        }
    }

    pub(super) fn hint(self) -> Message {
        Message::new("hint-tab").with("tab", self.label())
    }
}

impl DividerTab {
    /// The canvas size for the labels of every tab.
    pub(super) fn labels_size() -> UVec2 {
        TAB_LABEL * UVec2::new(Tab::ALL.len() as u32, 1)
    }

    pub(super) fn write_labels(commands: &mut Commands, labels: &UiCanvas) {
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                labels.target(),
            ))
            .with_children(|row| {
                for tab in Tab::ALL {
                    row.spawn((
                        Node {
                            width: px(TAB_LABEL.x),
                            height: px(TAB_LABEL.y),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(tab.color()),
                        children![(
                            Localized::from(tab.label()),
                            TextFont {
                                font_size: FontSize::Px(TAB_FONT_SIZE),
                                ..default()
                            },
                            TextColor(INK),
                        )],
                    ));
                }
            });
    }

    /// The divider for `tab` in front of page `start`, with its tab the one
    /// at `position` from the top, so the tabs stay together when a section
    /// has none. Its origin is where the middle of a page would be, and it
    /// is punched for the rings like one.
    pub(super) fn spawn(
        parent: &mut ChildSpawnerCommands,
        file: Entity,
        (tab, position, start): (Tab, usize, usize),
        labels: &UiCanvas,
        (meshes, surfaces): (&mut Assets<Mesh>, &mut Assets<SurfaceMaterial>),
    ) -> Entity {
        let count = Tab::ALL.len() as f32;
        let slot = tab.slot() as f32;
        let region = Rect::new(slot / count, 0.0, (slot + 1.0) / count, 1.0);
        let lip = Vec2::X * LIP / 2.0;
        let body = punched_box(
            Vec3::new(SHEET_SIZE.x + LIP, CARD, SHEET_SIZE.y + 2.0 * LIP),
            &holes(stray(tab.seed())).map(|hole| hole - lip),
            HOLE_RADIUS,
        );
        let body = (
            Mesh3d(meshes.add(body)),
            MeshMaterial3d(surfaces.add(Finish::Paper.material(tab.color(), Form::Freeform, slot))),
            Transform::from_translation(lip.extend(0.0).xzy()),
        );
        let label = Block::new(Vec3::new(TAB_LENGTH, CARD / 2.0, TAB_OUT));
        let along = -SHEET_SIZE.y / 2.0 + TAB_SLOT * (position as f32 + 0.5);
        parent
            .spawn((
                DividerTab { file },
                Transform::default(),
                Visibility::Inherited,
            ))
            .with_children(|divider| {
                divider.spawn(body);
                divider
                    .spawn((
                        TabLink { file, start },
                        PaperButton(Some(tab.hint())),
                        Transform::from_xyz((SHEET_SIZE.x + TAB_OUT) / 2.0, 0.0, along)
                            .with_rotation(Quat::from_rotation_y(-FRAC_PI_2)),
                        Visibility::Inherited,
                    ))
                    .with_children(|tab| {
                        // Printed on both sides, so it reads face up on either
                        // side of the rings.
                        tab.spawn((
                            Shape::textured_cuboid(&label, labels.image.clone(), region),
                            Finish::Paper,
                        ));
                        tab.spawn((
                            Transform::from_rotation(Quat::from_rotation_x(PI)),
                            Visibility::Inherited,
                            children![(
                                Shape::textured_cuboid(&label, labels.image.clone(), region),
                                Finish::Paper,
                            )],
                        ));
                    });
            })
            .id()
    }

    pub(super) fn open_on_click(
        mut click: On<PointerClick>,
        links: Query<&TabLink>,
        mut papers: Query<&mut Paper>,
    ) {
        if click.button != PointerButton::Primary {
            return;
        }
        let Ok(link) = links.get(click.entity) else {
            return;
        };
        click.propagate(false);
        if let Ok(mut paper) = papers.get_mut(link.file) {
            paper.open_page(link.start);
        }
    }
}
