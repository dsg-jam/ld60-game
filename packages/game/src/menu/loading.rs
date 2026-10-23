use bevy::prelude::*;
use ld60_core::pipelines::Pipelines;
use ld60_core::player::hints::IconFont;
use ld60_core::state::AppState;
use ld60_locale::Locale;

use crate::TITLE;
use crate::workshop::levels::Levels;

/// The colors of the page around the game on the web, which shows until the
/// game runs.
const BACKGROUND: Color = Color::srgb_u8(0x2a, 0x26, 0x24);
const TEXT: Color = Color::srgb_u8(0xe8, 0xdf, 0xd0);
const ACCENT: Color = Color::srgb_u8(0xf0, 0xb8, 0x60);
const TITLE_SIZE: f32 = 56.0;
const BAR: Vec2 = Vec2::new(384.0, 6.0);
const GAP: f32 = 32.0;
/// Frames in a row with nothing left to load before the title screen shows.
/// More pipelines come up as the scene comes into view.
const SETTLE_FRAMES: u32 = 30;
/// Seconds after which the title screen shows even if something never
/// finishes.
const GIVE_UP_SECS: f32 = 60.0;

pub(super) struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Loading), LoadingScreen::spawn)
            .add_systems(
                Update,
                LoadingScreen::follow.run_if(in_state(AppState::Loading)),
            );
    }
}

/// Covers the window until the assets are loaded and the shaders of the
/// scene are compiled.
#[derive(Component, Debug)]
struct LoadingScreen;

/// The filled part of the progress bar.
#[derive(Component, Debug)]
struct Fill;

/// How far the loading is, as the bar shows it.
#[derive(Debug, Default)]
struct Progress {
    /// From 0 to 1. It never goes back, even when new pipelines come up.
    shown: f32,
    /// Frames in a row with nothing left to load.
    settled: u32,
}

impl LoadingScreen {
    fn spawn(mut commands: Commands) {
        commands.spawn((
            LoadingScreen,
            DespawnOnExit(AppState::Loading),
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(GAP),
                ..default()
            },
            BackgroundColor(BACKGROUND),
            GlobalZIndex(10),
            children![
                (
                    Text::new(TITLE),
                    TextFont::from_font_size(FontSize::Px(TITLE_SIZE)),
                    TextColor(TEXT),
                ),
                (
                    Node {
                        width: px(BAR.x),
                        max_width: percent(80),
                        height: px(BAR.y),
                        ..default()
                    },
                    BackgroundColor(TEXT.with_alpha(0.2)),
                    children![(
                        Fill,
                        Node {
                            width: percent(0),
                            height: percent(100),
                            ..default()
                        },
                        BackgroundColor(ACCENT),
                    )],
                ),
            ],
        ));
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "each check reads its own resource"
    )]
    fn follow(
        time: Res<Time>,
        locale: Res<Locale>,
        levels: Option<Res<Levels>>,
        icons: Res<IconFont>,
        assets: Res<AssetServer>,
        pipelines: Res<Pipelines>,
        mut fill: Single<&mut Node, With<Fill>>,
        mut progress: Local<Progress>,
        mut next: ResMut<NextState<AppState>>,
    ) {
        let loaded = [
            locale.is_ready(),
            levels.is_some(),
            icons.is_loaded(&assets),
        ];
        let ready = progress.update(&loaded, &pipelines);
        fill.width = percent(progress.shown * 100.0);
        if ready {
            next.set(AppState::Title);
        } else if time.elapsed_secs() > GIVE_UP_SECS {
            warn!(
                pending_pipelines = pipelines.pending(),
                "loading takes too long, showing the title screen anyway"
            );
            next.set(AppState::Title);
        }
    }
}

impl Progress {
    /// Moves the bar on and returns whether the loading is done.
    fn update(&mut self, loaded: &[bool], pipelines: &Pipelines) -> bool {
        let files = loaded.iter().filter(|done| **done).count() as f32 / loaded.len() as f32;
        let (total, pending) = (pipelines.total(), pipelines.pending());
        let compiled = if total == 0 {
            0.0
        } else {
            (total - pending) as f32 / total as f32
        };
        self.shown = self.shown.max((files + compiled) / 2.0);
        let done = loaded.iter().all(|done| *done) && total > 0 && pending == 0;
        self.settled = if done { self.settled + 1 } else { 0 };
        self.settled >= SETTLE_FRAMES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_waits_until_everything_settles() {
        let mut progress = Progress::default();
        assert!(!progress.update(&[true, false], &Pipelines::counted(4, 0)));
        assert_eq!(progress.shown, 0.75);
        assert!(!progress.update(&[true, true], &Pipelines::counted(0, 0)));
        assert!(!progress.update(&[true, true], &Pipelines::counted(8, 4)));
        assert_eq!(progress.shown, 0.75);
        for _ in 1..SETTLE_FRAMES {
            assert!(!progress.update(&[true, true], &Pipelines::counted(8, 0)));
        }
        assert!(progress.update(&[true, true], &Pipelines::counted(8, 0)));
        assert_eq!(progress.shown, 1.0);
    }
}
