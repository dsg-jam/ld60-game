mod dsp;
mod fans;
mod motor;
mod tire;
mod voice;

use std::f32::consts::FRAC_PI_4;
use std::sync::Arc;

use bevy::{audio::AddAudioSource, math::Affine3A, prelude::*};

use self::fans::Fans;
use self::motor::Motor;
use self::tire::Tire;
use self::voice::{Controls, Heard, Part, Voice};
use super::super::street::{ROAD_PUDDLES, ROAD_SEED};
use super::super::weather::Weather;
use super::super::wet::Wet;
use super::Vehicle;
use super::flying::SkyCar;
use super::models::{Body, WHEEL_RADIUS, WHEELBASE};
use crate::audio::Mixer;

/// The closest cars are heard, up to this many and this many meters away.
const CARS: usize = 3;
const CAR_RANGE: f32 = 45.0;
const SKY_CARS: usize = 4;
const SKY_RANGE: f32 = 300.0;
/// A voice keeps playing a little beyond its range, so it doesn't stop and
/// start at the edge.
const KEEP: f32 = 1.1;
const CAR_VOLUME: f32 = 0.019;
const SKY_VOLUME: f32 = 0.009;
/// Millimeters of water on a soaked road.
const FILM: f32 = 1.0;
/// Millimeters of water in a puddle per unit [`Wet::puddle`] is above its
/// level.
const PUDDLE: f32 = 40.0;
/// Share of a puddle's water left for the rear wheels, after the front ones
/// pushed it aside.
const WAKE: f32 = 0.6;
/// Sound in the air loses its highs over distance: a one pole low-pass at
/// this many hertz times meters over the distance comes close for the
/// kilohertz that matter.
const AIR: f32 = 700_000.0;
/// The shadowed ear hears little above this, in hertz.
const SHADOW_HZ: f32 = 2_000.0;
/// How far a sound right at the side moves over, from 0 for the middle to 1
/// for only one ear.
const WIDTH: f32 = 0.8;
/// Sounds from behind are a little duller in both ears.
const BEHIND: f32 = 0.3;
/// The stalls and the crowd shield the street. Sound fades with the squared
/// distance and keeps little above this.
const STREET: Way = Way {
    reach: 6.0,
    power: 2,
    highest: 4_000.0,
};
/// Nothing shields the sky.
const SKY: Way = Way {
    reach: 20.0,
    power: 1,
    highest: 16_000.0,
};

pub(super) struct TrafficSoundPlugin;

impl Plugin for TrafficSoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<Voice>().add_systems(
            PostUpdate,
            (
                (Vehicle::pick, Vehicle::follow).chain(),
                (SkyCar::pick, SkyCar::follow).chain(),
            )
                .after(TransformSystems::Propagate),
        );
    }
}

/// A vehicle the player hears, and the controls of its voice.
#[derive(Component, Debug)]
struct Played(Arc<Controls>);

/// Where and how the player hears.
struct Ears {
    at: Vec3,
    view: Affine3A,
}

/// How sound travels from a part of the city to the player.
struct Way {
    /// Meters at which it is half as loud as up close, to the power.
    reach: f32,
    power: i32,
    /// Hertz above which little arrives.
    highest: f32,
}

impl Vehicle {
    fn pick(
        listener: Single<&GlobalTransform, With<SpatialListener>>,
        cars: Query<(Entity, &Vehicle, &GlobalTransform, Option<&Played>)>,
        mut voices: ResMut<Assets<Voice>>,
        mut commands: Commands,
    ) {
        let ear = listener.translation();
        let cars = cars.iter().map(|(entity, vehicle, transform, played)| {
            let distance = transform.translation().distance(ear);
            (entity, distance, (played, vehicle.body))
        });
        pick(
            cars,
            CARS,
            CAR_RANGE,
            &mut voices,
            &mut commands,
            |body, seed| {
                let mut parts: Vec<_> = (0..4).map(|_| Part::Tire(body.tire(seed))).collect();
                parts.push(Part::Motor(body.motor(seed)));
                Voice::new(parts, CAR_RANGE * KEEP, seed)
            },
        );
    }

    fn follow(
        listener: Single<&GlobalTransform, With<SpatialListener>>,
        time: Res<Time<Virtual>>,
        mixer: Res<Mixer>,
        weather: Res<Weather>,
        cars: Query<(&Vehicle, &GlobalTransform, &Played)>,
    ) {
        let ears = Ears::new(&listener);
        for (vehicle, transform, played) in &cars {
            let controls = &played.0;
            controls.hold(time.is_paused());
            controls.gain.set(CAR_VOLUME * mixer.gain());
            let velocity = Vec3::X * vehicle.velocity;
            let speed = vehicle.velocity.abs();
            let wheels = vehicle.body.wheels();
            for (index, (wheel, heard)) in wheels.iter().zip(&controls.parts).enumerate() {
                let at = transform.transform_point(*wheel);
                let wake = if index < 2 { 1.0 } else { WAKE };
                heard.water.set(water(at.xz(), weather.wetness, wake));
                heard.speed.set(speed);
                ears.hear(heard, at, velocity, &STREET);
            }
            let motor = &controls.parts[4];
            motor.speed.set(speed);
            let at = transform.transform_point(vehicle.body.motor_at());
            ears.hear(motor, at, velocity, &STREET);
        }
    }
}

impl SkyCar {
    fn pick(
        listener: Single<&GlobalTransform, With<SpatialListener>>,
        cars: Query<(Entity, &SkyCar, &GlobalTransform, Option<&Played>)>,
        mut voices: ResMut<Assets<Voice>>,
        mut commands: Commands,
    ) {
        let ear = listener.translation();
        let cars = cars.iter().map(|(entity, car, transform, played)| {
            let distance = if car.velocity().is_some() {
                transform.translation().distance(ear)
            } else {
                f32::INFINITY
            };
            (entity, distance, (played, ()))
        });
        pick(
            cars,
            SKY_CARS,
            SKY_RANGE,
            &mut voices,
            &mut commands,
            |(), seed| {
                let fans = Fans {
                    blades: 260.0 * vary(seed, 0.12),
                    level: 1.0,
                };
                Voice::new(vec![Part::Fans(fans)], SKY_RANGE * KEEP, seed)
            },
        );
    }

    fn follow(
        listener: Single<&GlobalTransform, With<SpatialListener>>,
        time: Res<Time<Virtual>>,
        mixer: Res<Mixer>,
        cars: Query<(&SkyCar, &GlobalTransform, &Played)>,
    ) {
        let ears = Ears::new(&listener);
        for (car, transform, played) in &cars {
            let controls = &played.0;
            controls.hold(time.is_paused());
            controls.gain.set(SKY_VOLUME * mixer.gain());
            let velocity = car.velocity().unwrap_or_default();
            let fans = &controls.parts[0];
            fans.speed.set(velocity.length());
            let at = transform.transform_point(SkyCar::FANS);
            ears.hear(fans, at, velocity, &SKY);
        }
    }

    /// The middle of the pods the fans sit in.
    const FANS: Vec3 = Vec3::new(-0.6, 0.15, 0.0);
}

impl Body {
    fn tire(self, seed: u32) -> Tire {
        let pitch = vary(seed.wrapping_add(1), 0.05);
        match self {
            Self::Sedan => Tire {
                width: 0.205,
                load: 1.0,
                pitch: 0.03 * pitch,
                drain: 2.0,
            },
            // Light truck tires, wider, with a coarser and deeper tread.
            Self::Van => Tire {
                width: 0.225,
                load: 1.8,
                pitch: 0.035 * pitch,
                drain: 2.5,
            },
        }
    }

    fn motor(self, seed: u32) -> Motor {
        let gear = vary(seed, 0.05);
        match self {
            Self::Sedan => Motor {
                gear: 9.0 * gear,
                wheel: WHEEL_RADIUS,
                order: 8.0,
                teeth: 19.0,
                inverter: 3_100.0,
                level: 0.5,
            },
            // A bigger, slower motor under more load.
            Self::Van => Motor {
                gear: 7.5 * gear,
                wheel: WHEEL_RADIUS,
                order: 6.0,
                teeth: 17.0,
                inverter: 2_300.0,
                level: 0.7,
            },
        }
    }

    /// Sedans drive their front wheels. Delivery vans drive the rear ones,
    /// under the load.
    fn motor_at(self) -> Vec3 {
        let x = match self {
            Self::Sedan => WHEELBASE,
            Self::Van => -WHEELBASE,
        };
        Vec3::new(x, WHEEL_RADIUS, 0.0)
    }
}

impl Ears {
    fn new(listener: &GlobalTransform) -> Self {
        Self {
            at: listener.translation(),
            view: listener.affine().inverse(),
        }
    }

    /// Tells `heard` how the part at `at`, moving at `velocity`, sounds
    /// from here.
    fn hear(&self, heard: &Heard, at: Vec3, velocity: Vec3, way: &Way) {
        let offset = at - self.at;
        let distance = offset.length().max(0.1);
        heard.distance.set(distance);
        heard.receding.set(velocity.dot(offset / distance));
        let fade = (way.reach / (way.reach + distance)).powi(way.power);
        let direction = self
            .view
            .transform_vector3(offset)
            .normalize_or(Vec3::NEG_Z);
        let side = (direction.x * WIDTH).clamp(-1.0, 1.0);
        let angle = (side + 1.0) * FRAC_PI_4;
        let highest = (AIR / distance).min(way.highest);
        let shadowed = highest.min(SHADOW_HZ);
        let behind = direction.z.max(0.0) * BEHIND;
        let away = [side.max(0.0), (-side).max(0.0)];
        for (ear, gain) in [angle.cos(), angle.sin()].into_iter().enumerate() {
            heard.gains[ear].set(gain * fade);
            let dull = (away[ear] + behind).min(1.0);
            heard.cutoffs[ear].set(highest + (shadowed - highest) * dull);
        }
    }
}

/// Starts voices for the closest `count` of `vehicles` in `range`, and
/// stops those that went out of it or got crowded out. Each comes with its
/// distance and, while it plays, its controls. `voice` makes the voice of
/// one from a seed.
fn pick<'a, T>(
    vehicles: impl Iterator<Item = (Entity, f32, (Option<&'a Played>, T))>,
    count: usize,
    range: f32,
    voices: &mut Assets<Voice>,
    commands: &mut Commands,
    mut voice: impl FnMut(T, u32) -> (Voice, Arc<Controls>),
) {
    let mut vehicles: Vec<_> = vehicles.collect();
    vehicles.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (rank, (entity, distance, (played, kind))) in vehicles.into_iter().enumerate() {
        match played {
            // One more than `count` may play for a moment, so two cars at
            // about the same distance don't take turns.
            Some(played) if distance > range * KEEP || rank > count => {
                played.0.stop();
                commands.entity(entity).remove::<Played>();
            }
            None if distance < range && rank < count => {
                let (sound, controls) = voice(kind, entity.index_u32());
                commands.spawn((
                    Name::new("traffic sound"),
                    AudioPlayer(voices.add(sound)),
                    PlaybackSettings::DESPAWN,
                ));
                commands.entity(entity).insert(Played(controls));
            }
            _ => {}
        }
    }
}

/// Millimeters of water at `at` on the road, `wake` of it in puddles.
fn water(at: Vec2, wetness: f32, wake: f32) -> f32 {
    let puddle = Wet::puddle(at, wetness, ROAD_PUDDLES, ROAD_SEED).max(0.0);
    FILM * wetness + PUDDLE * puddle * wake
}

/// A factor from `1 - amount` to `1 + amount` made from `seed`.
fn vary(seed: u32, amount: f32) -> f32 {
    let mut noise = dsp::Noise::new(seed.wrapping_add(40));
    1.0 + noise.next() * amount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_on_the_right_are_louder_in_the_right_ear() {
        let ears = Ears::new(&GlobalTransform::IDENTITY);
        let heard = Heard::default();
        ears.hear(&heard, Vec3::new(5.0, 0.0, -5.0), Vec3::ZERO, &STREET);
        assert!(heard.gains[1].get() > 2.0 * heard.gains[0].get());
        assert!(heard.cutoffs[0].get() < heard.cutoffs[1].get());
    }

    #[test]
    fn approaching_parts_have_negative_receding_speed() {
        let ears = Ears::new(&GlobalTransform::IDENTITY);
        let heard = Heard::default();
        ears.hear(&heard, Vec3::new(-20.0, 0.0, -5.0), Vec3::X * 11.0, &STREET);
        assert!(heard.receding.get() < -10.0);
    }
}
