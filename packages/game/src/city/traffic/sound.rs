use std::ops::Range;
use std::time::Duration;

use bevy::prelude::*;
use rand::{RngExt, rngs::SmallRng};

use super::super::rng;
use super::super::soundscape::{CitySounds, Emitter};
use super::super::street::{ROAD_PUDDLES, ROAD_SEED};
use super::super::weather::Weather;
use super::super::wet::Wet;
use super::Vehicle;
use super::flying::SkyCar;
use super::models::{TRACK, WHEELBASE};
use crate::audio::{Mixer, Sound};

const SEED: u64 = 11;
/// Cars on the street are half as loud this many meters away, and only
/// heard within the range.
const TIRE_REACH: f32 = 8.0;
const TIRE_RANGE: f32 = 70.0;
/// How loud tires are on a dry road, compared to a soaked one.
const DRY: f32 = 0.4;
/// How much louder tires get while they roll through a puddle.
const PUDDLE: f32 = 0.6;
/// How far above the puddle level water has to be for a splash. The thin
/// edges of puddles stay quiet.
const SPLASH_DEPTH: f32 = 0.01;
/// Seconds between two splashes of one car, at least.
const SPLASH_GAP: f32 = 0.25;
const HOVER_REACH: f32 = 30.0;
const HOVER_RANGE: f32 = 160.0;
/// Only the closest flying cars are heard.
const HOVERS: usize = 4;
/// Engines differ a little, so cars don't all sound the same.
const PITCH: Range<f32> = 0.9..1.1;

pub(super) struct TrafficSoundPlugin;

impl Plugin for TrafficSoundPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SoundRng(rng(SEED)))
            .register_required_components::<Vehicle, Rolling>()
            .register_required_components::<SkyCar, Hovering>()
            .add_systems(
                Update,
                (Rolling::sound, Hovering::sound).run_if(resource_exists::<CitySounds>),
            );
    }
}

#[derive(Resource)]
struct SoundRng(SmallRng);

/// The sound of a car's tires on the street.
#[derive(Component, Debug, Default)]
struct Rolling {
    /// The [`Emitter`] of the tires, while the car is close enough.
    tires: Option<Entity>,
    /// Whether the left and right front wheels are in a puddle.
    in_puddle: [bool; 2],
    /// Seconds since the last splash.
    since_splash: f32,
}

/// The sound of a flying car.
#[derive(Component, Debug, Default)]
struct Hovering {
    engine: Option<Entity>,
}

impl Rolling {
    /// Plays the tires of the cars close by. They hiss louder the wetter the
    /// road is and splash through puddles.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn sound(
        time: Res<Time>,
        weather: Res<Weather>,
        mixer: Res<Mixer>,
        mut sounds: ResMut<CitySounds>,
        mut rng: ResMut<SoundRng>,
        listener: Single<&GlobalTransform, With<SpatialListener>>,
        mut cars: Query<(Entity, &GlobalTransform, &mut Rolling), With<Vehicle>>,
        mut emitters: Query<&mut Emitter>,
        mut commands: Commands,
    ) {
        let ear = listener.translation();
        for (car, transform, mut rolling) in &mut cars {
            let distance = transform.translation().distance(ear);
            let Some(tires) =
                rolling.keep_close(car, distance, &sounds, &mixer, &mut rng.0, &mut commands)
            else {
                continue;
            };
            rolling.since_splash += time.delta_secs();
            let mut depth = 0.0_f32;
            for (index, side) in [-1.0, 1.0].into_iter().enumerate() {
                let wheel = Vec3::new(WHEELBASE, 0.0, side * TRACK);
                let at = transform.transform_point(wheel).xz();
                let water = Wet::puddle(at, weather.wetness, ROAD_PUDDLES, ROAD_SEED);
                depth = depth.max(water);
                let in_puddle = water > SPLASH_DEPTH;
                if in_puddle && !rolling.in_puddle[index] && rolling.since_splash > SPLASH_GAP {
                    rolling.since_splash = 0.0;
                    let level = (0.4 + water * 8.0).min(1.0);
                    let splash =
                        Emitter::new(Sound::Splash, Some(TIRE_REACH), rng.0.random_range(PITCH))
                            .with_level(level);
                    commands.entity(car).with_child((
                        splash.bundle(
                            sounds.splash.next().clone(),
                            PlaybackSettings::DESPAWN,
                            &mixer,
                            distance,
                        ),
                        Transform::from_translation(wheel),
                    ));
                }
                rolling.in_puddle[index] = in_puddle;
            }
            if let Ok(mut emitter) = emitters.get_mut(tires) {
                let wet = DRY + (1.0 - DRY) * weather.wetness;
                let puddle = if depth > SPLASH_DEPTH { PUDDLE } else { 0.0 };
                emitter.level = wet + puddle;
            }
        }
    }

    /// Starts the tires once the car comes into range and stops them once
    /// it is well out of it. Returns the tires while they play.
    fn keep_close(
        &mut self,
        car: Entity,
        distance: f32,
        sounds: &CitySounds,
        mixer: &Mixer,
        rng: &mut SmallRng,
        commands: &mut Commands,
    ) -> Option<Entity> {
        match self.tires {
            None if distance < TIRE_RANGE => {
                let source = sounds.tires[rng.random_range(0..sounds.tires.len())].clone();
                let tires = Emitter::new(Sound::Tires, Some(TIRE_REACH), rng.random_range(PITCH))
                    .with_level(DRY);
                let entity = commands
                    .spawn((
                        tires.bundle(source, looping(rng), mixer, distance),
                        Transform::from_xyz(0.0, 0.3, 0.0),
                        ChildOf(car),
                    ))
                    .id();
                self.tires = Some(entity);
            }
            Some(tires) if distance > TIRE_RANGE * 1.1 => {
                commands.entity(tires).despawn();
                self.tires = None;
                self.in_puddle = [false; 2];
            }
            _ => {}
        }
        self.tires
    }
}

impl Hovering {
    /// Plays the flying cars closest to the player. A car keeps its sound
    /// until it leaves, so the sounds don't jump between cars.
    fn sound(
        mixer: Res<Mixer>,
        sounds: Res<CitySounds>,
        mut rng: ResMut<SoundRng>,
        listener: Single<&GlobalTransform, With<SpatialListener>>,
        mut cars: Query<(Entity, &GlobalTransform, &Visibility, &mut Hovering), With<SkyCar>>,
        mut commands: Commands,
    ) {
        let ear = listener.translation();
        let mut heard = 0;
        let mut close = Vec::new();
        for (car, transform, visibility, mut hovering) in &mut cars {
            let distance = transform.translation().distance(ear);
            let near = *visibility != Visibility::Hidden && distance < HOVER_RANGE;
            match hovering.engine {
                Some(engine) if !near => {
                    commands.entity(engine).despawn();
                    hovering.engine = None;
                }
                Some(_) => heard += 1,
                None if near => close.push((car, distance)),
                None => {}
            }
        }
        close.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (car, distance) in close.into_iter().take(HOVERS.saturating_sub(heard)) {
            let Ok((_, _, _, mut hovering)) = cars.get_mut(car) else {
                continue;
            };
            let source = sounds.hover[rng.0.random_range(0..sounds.hover.len())].clone();
            let engine = Emitter::new(Sound::Hover, Some(HOVER_REACH), rng.0.random_range(PITCH))
                .with_level(1.0);
            let entity = commands
                .spawn((
                    engine.bundle(source, looping(&mut rng.0), &mixer, distance),
                    Transform::default(),
                    ChildOf(car),
                ))
                .id();
            hovering.engine = Some(entity);
        }
    }
}

/// A loop starting somewhere in the middle, so cars don't sound in step.
fn looping(rng: &mut SmallRng) -> PlaybackSettings {
    PlaybackSettings::LOOP.with_start_position(Duration::from_secs_f32(rng.random_range(0.0..3.0)))
}
