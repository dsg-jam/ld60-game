use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    input::{
        InputSystems,
        mouse::{MouseScrollUnit, MouseWheel},
    },
    prelude::*,
};
use ld60_core::player::hints::{ACCENT, INK, SHADOW};
use ld60_locale::{Localized, Message};
use serde::Deserialize;

use super::Page;
use super::list::{Action, Entries, Entry, Navigation};

const CREDITS: &str = "credits.ron";
const HEADING_SIZE: f32 = 22.0;
const TEXT_SIZE: f32 = 18.0;
const DETAIL_SIZE: f32 = 15.0;
const DETAIL: Color = Color::srgba(1.0, 0.98, 0.9, 0.6);
const SECTION_GAP: f32 = 22.0;
const LINE_GAP: f32 = 4.0;
/// The scrolling area, in percent of the window's width and height.
const AREA: Vec2 = Vec2::new(40.0, 55.0);
/// How far a notch of the mouse wheel or an arrow key scrolls, in pixels.
const SCROLL_STEP: f32 = 48.0;

pub(super) struct CreditsPlugin;

impl Plugin for CreditsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Credits>()
            .init_asset_loader::<CreditsLoader>()
            .init_resource::<CreditsHandle>()
            .add_systems(
                PreUpdate,
                CreditsPage::scroll
                    .in_set(Navigation)
                    .after(InputSystems)
                    .run_if(in_state(Page::Credits)),
            );
    }
}

/// Everyone and everything the game is built on, from `credits.ron`.
#[derive(Asset, TypePath, Deserialize, Debug)]
pub(super) struct Credits {
    made_by: String,
    engine: Vec<Work>,
    libraries: Vec<Work>,
    assets: Vec<Work>,
    research: Vec<Paper>,
}

/// Software or an asset the game uses.
#[derive(Deserialize, Debug)]
struct Work {
    title: String,
    author: String,
    url: Option<String>,
    license: String,
    /// Whether the game uses a changed version.
    #[serde(default)]
    modified: bool,
}

/// A paper or article a technique in the game comes from.
#[derive(Deserialize, Debug)]
struct Paper {
    authors: String,
    title: String,
    url: String,
    published: String,
}

#[derive(Resource, Debug)]
pub(super) struct CreditsHandle(Handle<Credits>);

#[derive(Default, TypePath)]
struct CreditsLoader;

/// The credits page: a scrolling list of the [`Credits`] and a way back.
pub(super) struct CreditsPage;

/// One line of text on the credits page.
type Line = (Localized, TextFont, TextColor, TextShadow, Pickable);

/// The part of the credits page that scrolls.
#[derive(Component, Debug)]
struct Scroll;

impl FromWorld for CreditsHandle {
    fn from_world(world: &mut World) -> Self {
        Self(world.resource::<AssetServer>().load(CREDITS))
    }
}

impl CreditsHandle {
    pub(super) fn get<'a>(&self, credits: &'a Assets<Credits>) -> Option<&'a Credits> {
        credits.get(&self.0)
    }
}

impl AssetLoader for CreditsLoader {
    type Asset = Credits;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Credits, BevyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ron::de::from_bytes(&bytes)?)
    }

    fn extensions(&self) -> &[&str] {
        &[]
    }
}

impl CreditsPage {
    /// The credits, or only the way back if they aren't loaded.
    pub(super) fn bundle(credits: Option<&Credits>) -> impl Bundle {
        let sections = credits.map(Self::sections).unwrap_or_default();
        (
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(SECTION_GAP),
                ..default()
            },
            Children::spawn((
                Spawn((
                    Scroll,
                    Node {
                        width: vw(AREA.x),
                        height: vh(AREA.y),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(SECTION_GAP),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    ScrollPosition::default(),
                    Children::spawn(SpawnIter(sections.into_iter())),
                )),
                Spawn((
                    Entries,
                    children![Entry::bundle(
                        Action::Back,
                        Entries::label(Message::new("menu-back")),
                    )],
                )),
            )),
        )
    }

    fn sections(credits: &Credits) -> Vec<impl Bundle> {
        let works = |works: &[Work]| works.iter().map(Self::work).collect::<Vec<_>>();
        vec![
            Self::section(
                "credits-made-by",
                vec![vec![Self::line(
                    Localized::verbatim(&credits.made_by),
                    TEXT_SIZE,
                    INK,
                )]],
            ),
            Self::section("credits-engine", works(&credits.engine)),
            Self::section("credits-libraries", works(&credits.libraries)),
            Self::section("credits-assets", works(&credits.assets)),
            Self::section(
                "credits-research",
                credits.research.iter().map(Self::paper).collect(),
            ),
        ]
    }

    /// A heading with `entries` below it, each a few lines.
    fn section(heading: &'static str, entries: Vec<Vec<Line>>) -> impl Bundle {
        let entries = entries.into_iter().map(|lines| {
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(LINE_GAP),
                    ..default()
                },
                Children::spawn(SpawnIter(lines.into_iter())),
            )
        });
        (
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(SECTION_GAP / 2.0),
                ..default()
            },
            Children::spawn((
                Spawn(Self::line(
                    Message::new(heading).into(),
                    HEADING_SIZE,
                    ACCENT,
                )),
                SpawnIter(entries.collect::<Vec<_>>().into_iter()),
            )),
        )
    }

    fn work(work: &Work) -> Vec<Line> {
        let detail = Message::new(if work.modified {
            "credits-work-modified"
        } else {
            "credits-work"
        })
        .with("author", work.author.as_str())
        .with("license", work.license.as_str());
        let mut lines = vec![
            Self::line(Localized::verbatim(&work.title), TEXT_SIZE, INK),
            Self::line(detail.into(), DETAIL_SIZE, DETAIL),
        ];
        if let Some(url) = &work.url {
            lines.push(Self::line(Localized::verbatim(url), DETAIL_SIZE, DETAIL));
        }
        lines
    }

    fn paper(paper: &Paper) -> Vec<Line> {
        let detail = Message::new("credits-paper")
            .with("authors", paper.authors.as_str())
            .with("published", paper.published.as_str());
        vec![
            Self::line(Localized::verbatim(&paper.title), TEXT_SIZE, INK),
            Self::line(detail.into(), DETAIL_SIZE, DETAIL),
            Self::line(Localized::verbatim(&paper.url), DETAIL_SIZE, DETAIL),
        ]
    }

    fn line(text: Localized, size: f32, color: Color) -> Line {
        (
            text,
            TextFont::from_font_size(FontSize::Px(size)),
            TextColor(color),
            SHADOW,
            Pickable::IGNORE,
        )
    }

    /// Scrolls with the mouse wheel and the arrow keys.
    fn scroll(
        mut wheel: MessageReader<MouseWheel>,
        keys: Res<ButtonInput<KeyCode>>,
        mut scrolls: Query<&mut ScrollPosition, With<Scroll>>,
    ) {
        let lines: f32 = wheel
            .read()
            .map(|event| match event.unit {
                MouseScrollUnit::Line => event.y,
                MouseScrollUnit::Pixel => event.y / SCROLL_STEP,
            })
            .sum();
        let step = |key| f32::from(u8::from(keys.just_pressed(key)));
        let down = step(KeyCode::ArrowDown) - step(KeyCode::ArrowUp) - lines;
        if down == 0.0 {
            return;
        }
        for mut scroll in &mut scrolls {
            scroll.0.y = (scroll.0.y + down * SCROLL_STEP).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_locale::Locale;

    use super::*;

    #[test]
    fn credits_load_and_show() {
        let path = format!("{}/{CREDITS}", env!("LD60_ASSETS"));
        let credits: Credits = ron::de::from_bytes(&std::fs::read(path).unwrap()).unwrap();
        let texts = Localized::spawned_by(|page| {
            page.spawn(CreditsPage::bundle(Some(&credits)));
        });
        assert!(texts.len() > credits.research.len());
        Locale::on_disk().assert_formats(texts);
    }
}
