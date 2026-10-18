mod sprite;

use bevy::{
    picking::hover::HoverMap,
    platform::collections::HashMap,
    prelude::*,
    window::{CursorIcon, PrimaryWindow},
};

use self::sprite::Sprite;
use super::focus::{self, Clickable, Focus};
use crate::parts::{keypad::PressableKey, screen::ScreenSurface};

/// Diameter of the dot cursor, in logical pixels.
pub(crate) const DOT_SIZE: f32 = 10.0;
/// Diameter of the dot cursor over a [`Clickable`] in the desk view.
pub(crate) const DOT_HOVER_SIZE: f32 = 16.0;
pub(crate) const DOT_OUTLINE: f32 = 1.5;
pub(crate) const DOT_FILL: Color = Color::srgba(1.0, 0.98, 0.9, 0.9);
pub(crate) const DOT_INK: Color = Color::srgb_u8(0x4a, 0x44, 0x3a);

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
        app.add_systems(Startup, Cursors::load)
            .add_systems(Update, Cursors::update.run_if(resource_exists::<Cursors>));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Kind {
    Arrow,
    Finger,
    Dot,
    HoverDot,
}

impl Kind {
    const ALL: [Self; 4] = [Self::Arrow, Self::Finger, Self::Dot, Self::HoverDot];

    fn sprite(self) -> Sprite {
        match self {
            Self::Arrow => Sprite::pixel_art(ARROW, SCALE, IVec2::ZERO, ART_FILL, DOT_INK),
            Self::Finger => Sprite::pixel_art(FINGER, SCALE, IVec2::new(5, 0), ART_FILL, DOT_INK),
            Self::Dot => Self::dot(DOT_SIZE),
            Self::HoverDot => Self::dot(DOT_HOVER_SIZE),
        }
    }

    fn dot(diameter: f32) -> Sprite {
        Sprite::dot(diameter, DOT_OUTLINE, DOT_FILL, DOT_INK)
    }

    /// The image `t` of the way from `self` to `to`.
    ///
    /// Dots change size instead of fading.
    fn between(self, to: Self, t: f32) -> Sprite {
        let diameter = |kind| match kind {
            Self::Dot => Some(DOT_SIZE),
            Self::HoverDot => Some(DOT_HOVER_SIZE),
            _ => None,
        };
        match (diameter(self), diameter(to)) {
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
            for to in Kind::ALL.into_iter().filter(|&to| to != from) {
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
        time: Res<Time>,
        state: Res<State<Focus>>,
        hovers: Res<HoverMap>,
        parents: Query<&ChildOf>,
        surfaces: Query<&ScreenSurface>,
        keys: Query<&PressableKey>,
        targets: Query<(), With<Clickable>>,
        mut icon: Single<&mut CursorIcon, With<PrimaryWindow>>,
    ) {
        let under: Vec<Entity> = focus::under_mouse(&hovers, &parents).collect();
        let wanted = if surfaces.iter().any(ScreenSurface::hovered) {
            Kind::Arrow
        } else if under
            .iter()
            .any(|&entity| keys.get(entity).is_ok_and(|key| key.active))
        {
            Kind::Finger
        } else if *state.get() == Focus::Desk
            && under.iter().any(|&entity| targets.contains(entity))
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
