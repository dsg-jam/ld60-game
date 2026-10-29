mod sprite;

use bevy::{
    picking::hover::HoverMap,
    platform::collections::HashMap,
    prelude::*,
    window::{CursorIcon, PrimaryWindow},
};

use self::sprite::Sprite;
use super::crosshair::Prompt;
use super::focus::{self, Clickable, Disabled, Focus};
use crate::state::AppState;

/// Diameter of the dot cursor, in logical pixels.
pub const DOT_SIZE: f32 = 10.0;
/// Diameter of the dot cursor over a [`Clickable`] in the desk view.
pub const DOT_HOVER_SIZE: f32 = 16.0;
pub const DOT_OUTLINE: f32 = 1.5;
/// Opaque, since Firefox miscolors translucent cursor pixels.
pub const DOT_FILL: Color = Color::srgb(1.0, 0.98, 0.9);
pub const DOT_INK: Color = Color::srgb_u8(0x4a, 0x44, 0x3a);

/// `#` is the outline, `.` the fill and a space is transparent.
const ARROW: &[&str] = &[
    "#",
    "##",
    "#.#",
    "#..#",
    "#...#",
    "#....#",
    "#.....#",
    "#......#",
    "#.......#",
    "#........#",
    "#.....#####",
    "#..#..#",
    "#.# #..#",
    "##  #..#",
    "#    #..#",
    "     #..#",
    "      ##",
];
const FINGER: &[&str] = &[
    "     ##",
    "    #..#",
    "    #..#",
    "    #..#",
    "    #..###",
    "    #..#..###",
    "    #..#..#..##",
    " ## #..#..#..#.#",
    "#..##..#..#..#.#",
    "#...#..........#",
    " #.............#",
    "  #............#",
    "  #...........#",
    "   #..........#",
    "   #.........#",
    "    #........#",
    "    #........#",
    "    ##########",
];
/// Screen pixels per pixel art pixel.
const SCALE: i32 = 2;
const ART_FILL: Color = Color::srgb_u8(0xd8, 0xcf, 0xb8);
/// Seconds to change from one cursor to another.
const TRANSITION_TIME: f32 = 0.12;
/// Images shown between two cursors.
const TRANSITION_FRAMES: usize = 5;

/// Picks the system cursor for what is under the mouse.
///
/// The free cursor is an arrow over a computer screen, a pointing finger
/// over a key and a dot everywhere else. Changes fade through a few
/// prepared frames, since system cursors can't animate.
pub(super) struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Cursors::load).add_systems(
            Update,
            Cursors::update
                .after(Prompt::update)
                .run_if(resource_exists::<Cursors>),
        );
    }
}

/// Asks for the arrow cursor while set, wherever the mouse is. A screen
/// under the mouse shows its own pointer, so it wants the arrow.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WantsArrow(pub bool);

/// Asks for the finger cursor while set and the mouse is over the entity,
/// like a key that can be pressed.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WantsFinger(pub bool);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Kind {
    Arrow,
    Finger,
    Dot,
    HoverDot,
    /// While the crosshair shows a [`Prompt`] in place of the cursor.
    Hidden,
}

impl Kind {
    const ALL: [Self; 5] = [
        Self::Arrow,
        Self::Finger,
        Self::Dot,
        Self::HoverDot,
        Self::Hidden,
    ];

    fn sprite(self) -> Sprite {
        match self {
            Self::Arrow => Sprite::pixel_art(ARROW, SCALE, IVec2::ZERO, ART_FILL, DOT_INK),
            Self::Finger => Sprite::pixel_art(FINGER, SCALE, IVec2::new(5, 0), ART_FILL, DOT_INK),
            Self::Dot => Self::dot(DOT_SIZE),
            Self::HoverDot => Self::dot(DOT_HOVER_SIZE),
            Self::Hidden => Sprite::pixel_art(&[" "], 1, IVec2::ZERO, ART_FILL, DOT_INK),
        }
    }

    fn dot(diameter: f32) -> Sprite {
        Sprite::dot(diameter, DOT_OUTLINE, DOT_FILL, DOT_INK)
    }

    fn diameter(self) -> Option<f32> {
        match self {
            Self::Dot => Some(DOT_SIZE),
            Self::HoverDot => Some(DOT_HOVER_SIZE),
            _ => None,
        }
    }

    /// The web skips crossfades, since Firefox miscolors their translucent
    /// frames. The prompt of the crosshair comes at once, so the cursor goes
    /// at once too.
    fn animates_to(self, to: Self) -> bool {
        self != Self::Hidden
            && to != Self::Hidden
            && (cfg!(not(target_family = "wasm"))
                || (self.diameter().is_some() && to.diameter().is_some()))
    }

    /// The image `t` of the way from `self` to `to`.
    ///
    /// Dots change size instead of fading.
    fn between(self, to: Self, t: f32) -> Sprite {
        match (self.diameter(), to.diameter()) {
            (Some(from), Some(to)) => Self::dot(from + (to - from) * t),
            _ => self.sprite().blend(&to.sprite(), t),
        }
    }
}

#[derive(Resource)]
struct Cursors {
    still: HashMap<Kind, CursorIcon>,
    transitions: HashMap<(Kind, Kind), Vec<CursorIcon>>,
    from: Kind,
    to: Kind,
    /// Seconds since the change from `from` to `to` started.
    elapsed: f32,
}

impl Cursors {
    fn load(
        window: Single<Entity, With<PrimaryWindow>>,
        mut images: ResMut<Assets<Image>>,
        mut commands: Commands,
    ) {
        let still: HashMap<_, _> = Kind::ALL
            .iter()
            .map(|&kind| (kind, kind.sprite().icon(&mut images)))
            .collect();
        let mut transitions = HashMap::default();
        for from in Kind::ALL {
            for to in Kind::ALL
                .into_iter()
                .filter(|&to| to != from && from.animates_to(to))
            {
                let frames = (1..=TRANSITION_FRAMES)
                    .map(|i| {
                        let t = i as f32 / (TRANSITION_FRAMES + 1) as f32;
                        from.between(to, t).icon(&mut images)
                    })
                    .collect();
                transitions.insert((from, to), frames);
            }
        }
        commands.entity(*window).insert(still[&Kind::Dot].clone());
        commands.insert_resource(Self {
            still,
            transitions,
            from: Kind::Dot,
            to: Kind::Dot,
            elapsed: 0.0,
        });
    }

    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn update(
        mut cursors: ResMut<Cursors>,
        time: Res<Time<Real>>,
        app_state: Res<State<AppState>>,
        state: Res<State<Focus>>,
        hovers: Res<HoverMap>,
        parents: Query<&ChildOf>,
        arrows: Query<&WantsArrow>,
        fingers: Query<&WantsFinger>,
        targets: Query<Has<Disabled>, With<Clickable>>,
        prompt: Res<Prompt>,
        mut icon: Single<&mut CursorIcon, With<PrimaryWindow>>,
    ) {
        let under: Vec<Entity> = focus::under_mouse(&hovers, &parents).collect();
        let wanted = if *app_state.get() == AppState::Playing && prompt.at_cursor() {
            Kind::Hidden
        } else if *app_state.get() != AppState::Playing || arrows.iter().any(|arrow| arrow.0) {
            Kind::Arrow
        } else if under
            .iter()
            .any(|&entity| fingers.get(entity).is_ok_and(|finger| finger.0))
        {
            Kind::Finger
        } else if *state.get() == Focus::Desk
            && under.iter().find_map(|&entity| targets.get(entity).ok()) == Some(false)
        {
            Kind::HoverDot
        } else {
            Kind::Dot
        };

        let progress = cursors.elapsed / TRANSITION_TIME;
        if wanted != cursors.to {
            // Start from whichever image is closer to what is shown now.
            cursors.from = if progress < 0.5 {
                cursors.from
            } else {
                cursors.to
            };
            cursors.to = wanted;
            cursors.elapsed = 0.0;
        } else {
            cursors.elapsed += time.delta_secs();
        }

        let progress = cursors.elapsed / TRANSITION_TIME;
        let shown = match cursors.transitions.get(&(cursors.from, cursors.to)) {
            Some(frames) if progress < 1.0 => &frames[(progress * frames.len() as f32) as usize],
            _ => &cursors.still[&cursors.to],
        };
        icon.set_if_neq(shown.clone());
    }
}
