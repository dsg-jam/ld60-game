use bevy::{platform::collections::HashMap, prelude::*};
use ld60_core::audio::{Mixer, Sound};
use ld60_core::layout::shop;

use super::super::buildings::Skyline;
use super::super::drips::{DripEdge, DripEdges};
use super::super::soundscape::Emitter;
use super::super::street::water;
use super::super::weather::Weather;
use super::super::{CURB_HEIGHT, FACADES};

/// Seconds for the rain to grow or fade with the weather.
const GLIDE: f32 = 3.0;
/// Patches of the sidewalks and the road around the player, so the rain
/// comes from all over the street.
const STREET: [(Sound, Vec3); 4] = [
    (Sound::ConcreteRain, Vec3::new(-7.0, 0.0, -2.5)),
    (Sound::AsphaltRain, Vec3::new(-3.5, -CURB_HEIGHT, -8.5)),
    (Sound::AsphaltRain, Vec3::new(3.5, -CURB_HEIGHT, -8.5)),
    (Sound::ConcreteRain, Vec3::new(7.0, 0.0, -2.5)),
];
/// The left and right half of the shop's roof, above the player.
const ROOF: [Vec3; 2] = [Vec3::new(-0.8, 3.5, -1.0), Vec3::new(0.8, 3.5, -1.0)];
/// Only the nearest awnings and puddles are heard.
const AWNINGS: usize = 2;
const PUDDLES: usize = 2;
/// See [`Emitter`].
const REACH: f32 = 6.0;
/// Awnings this wide in meters sound at their usual level.
const AWNING_WIDTH: f32 = 2.5;
/// Puddles are looked for in squares this many meters wide, out to
/// `SEARCH` meters to each side of the shop. Squares with puddles have to
/// be `APART` meters apart.
const PATCH: f32 = 2.0;
const SEARCH: f32 = 12.0;
const APART: f32 = 3.0;
/// Meters between the spots in a square where the water is checked.
const SPACING: f32 = 0.25;
/// Puddles grow louder with their water up to this many square meters.
const PUDDLE_AREA: f32 = 1.0;

pub(super) struct RainSoundPlugin;

impl Plugin for RainSoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, RainSound::spawn.after(Skyline::spawn))
            .add_systems(Update, RainSound::follow_weather);
    }
}

/// Rain falling on one kind of surface, heard from where it is.
#[derive(Component, Clone, Copy, Debug)]
enum RainSound {
    Street,
    Roof,
    /// An awning `size` times as wide as usual.
    Awning {
        size: f32,
    },
    /// The puddles in a square of the ground, which cover `area` square
    /// meters of it.
    Puddle {
        square: Vec2,
        area: f32,
    },
}

impl RainSound {
    fn spawn(
        mixer: Res<Mixer>,
        edges: Res<DripEdges>,
        mut sources: ResMut<Assets<AudioSource>>,
        mut commands: Commands,
    ) {
        let near = Vec3::new(0.0, 0.0, shop::FRONT);
        let mut rain: Vec<(RainSound, Sound, Vec3)> = STREET
            .into_iter()
            .map(|(sound, at)| (Self::Street, sound, at))
            .collect();
        rain.extend(ROOF.map(|at| (Self::Roof, Sound::RoofRain, at)));
        let mut awnings: Vec<_> = edges.0.iter().collect();
        awnings.sort_by(|a, b| {
            let a = near.distance(Self::middle(a));
            a.total_cmp(&near.distance(Self::middle(b)))
        });
        rain.extend(awnings.into_iter().take(AWNINGS).map(|edge| {
            let size = (edge.from.distance(edge.to) / AWNING_WIDTH).sqrt();
            (Self::Awning { size }, Sound::AwningRain, Self::middle(edge))
        }));
        rain.extend(Self::puddles(near.xz()).into_iter().map(|(square, at)| {
            (
                Self::Puddle { square, area: 0.0 },
                Sound::PuddleRain,
                at.extend(0.0).xzy() - Vec3::Y * CURB_HEIGHT,
            )
        }));

        let mut takes: HashMap<Sound, (Vec<Handle<AudioSource>>, usize)> = HashMap::default();
        for (rain, sound, at) in rain {
            let (handles, used) = takes.entry(sound).or_insert_with(|| {
                let handles = sound
                    .sources()
                    .into_iter()
                    .map(|source| sources.add(source))
                    .collect();
                (handles, 0)
            });
            let source = handles[*used % handles.len()].clone();
            *used += 1;
            let reach = matches!(rain, Self::Awning { .. } | Self::Puddle { .. }).then_some(REACH);
            commands.spawn((
                Name::new("rain sound"),
                rain,
                Emitter::new(sound, reach, 1.0).bundle(source, PlaybackSettings::LOOP, &mixer, 0.0),
                Transform::from_translation(at),
            ));
        }
    }

    /// Where the rain on the awning that drips off `edge` is heard from.
    fn middle(edge: &DripEdge) -> Vec3 {
        edge.from.midpoint(edge.to) + Vec3::Y * 0.1
    }

    /// The squares of the ground near `near` with the most water once it is
    /// soaked, and the middle of their water.
    fn puddles(near: Vec2) -> Vec<(Vec2, Vec2)> {
        let mut squares = Vec::new();
        let mut x = -SEARCH;
        while x < SEARCH {
            let mut z = shop::FRONT - PATCH;
            while z > FACADES {
                let square = Vec2::new(x, z);
                let wet = Self::spots(square)
                    .filter(|&spot| water(spot, 1.0) > 0.0)
                    .collect::<Vec<_>>();
                if !wet.is_empty() {
                    let middle = wet.iter().sum::<Vec2>() / wet.len() as f32;
                    let score = wet.len() as f32 / (1.0 + middle.distance_squared(near) / 16.0);
                    squares.push((score, square, middle));
                }
                z -= PATCH;
            }
            x += PATCH;
        }
        squares.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut picked: Vec<(Vec2, Vec2)> = Vec::new();
        for (_, square, middle) in squares {
            if picked.len() < PUDDLES
                && picked
                    .iter()
                    .all(|(_, other)| other.distance(middle) > APART)
            {
                picked.push((square, middle));
            }
        }
        picked
    }

    /// The spots in the square with its corner at `square`.
    fn spots(square: Vec2) -> impl Iterator<Item = Vec2> {
        let steps = (PATCH / SPACING) as usize;
        (0..steps * steps).map(move |index| {
            let along = Vec2::new((index % steps) as f32, (index / steps) as f32);
            square + (along + 0.5) * SPACING
        })
    }

    fn follow_weather(
        time: Res<Time>,
        weather: Res<Weather>,
        mut rain: Query<(&mut RainSound, &mut Emitter)>,
    ) {
        let step = time.delta_secs() / GLIDE;
        for (mut sound, mut emitter) in &mut rain {
            if weather.is_changed()
                && let Self::Puddle { square, area } = &mut *sound
            {
                let wet = Self::spots(*square)
                    .filter(|&spot| water(spot, weather.wetness) > 0.0)
                    .count();
                *area = wet as f32 * SPACING * SPACING;
            }
            let level = sound.level(weather.rain);
            emitter.level += (level - emitter.level).clamp(-step, step);
        }
    }

    /// The roof and the awnings drum along with any rain. The street fills
    /// in as the rain gets heavy, so light rain sounds like single drops.
    fn level(self, rain: f32) -> f32 {
        match self {
            Self::Street => rain.powf(1.5),
            Self::Roof => rain,
            Self::Awning { size } => rain * size,
            Self::Puddle { area, .. } => rain * (area / PUDDLE_AREA).sqrt().min(1.0),
        }
    }
}
