use std::f32::consts::TAU;

use super::dsp::{Glide, Noise, Tone};

/// The whine is as loud as its level at this speed, in meters per second.
const SPEED: f32 = 12.0;
/// Seconds for the speed to reach a new value.
const GLIDE: f32 = 0.05;
const GEAR_LEVEL: f32 = 0.35;
const INVERTER_LEVEL: f32 = 0.12;

/// An electric motor driving an axle through a reduction gear.
#[derive(Clone, Copy, Debug)]
pub(super) struct Motor {
    /// Motor turns per turn of the wheels.
    pub(super) gear: f32,
    /// In meters.
    pub(super) wheel: f32,
    /// The magnetic forces in the motor ring at this many times its turns.
    pub(super) order: f32,
    /// Teeth on the motor's gear, which whine as they mesh.
    pub(super) teeth: f32,
    /// The inverter switches at a fixed pitch, in hertz.
    pub(super) inverter: f32,
    pub(super) level: f32,
}

/// The whine of an electric motor: the main order of its magnetic forces
/// with two overtones, the gear mesh and the inverter. All but the inverter
/// follow the speed.
pub(super) struct MotorSound {
    motor: Motor,
    speed: Glide,
    magnet: Tone,
    gear: Tone,
    inverter: Tone,
    level: f32,
}

impl Motor {
    pub(super) fn sound(self, seed: u32) -> MotorSound {
        let mut noise = Noise::new(seed.wrapping_add(10));
        MotorSound {
            motor: self,
            speed: Glide::from_first(GLIDE),
            magnet: Tone::new(noise.unit() * TAU),
            gear: Tone::new(noise.unit() * TAU),
            inverter: Tone::new(noise.unit() * TAU),
            level: 0.0,
        }
    }
}

impl MotorSound {
    pub(super) fn block(&mut self, speed: f32) {
        let motor = self.motor;
        let speed = self.speed.follow(speed).max(0.0);
        let turns = speed / (TAU * motor.wheel) * motor.gear;
        self.magnet.tune(turns * motor.order);
        self.gear.tune(turns * motor.teeth);
        self.inverter.tune(motor.inverter);
        self.level = motor.level * (speed / SPEED).powi(2);
    }

    pub(super) fn next(&mut self) -> f32 {
        let x = self.magnet.next();
        let x2 = 2.0 * x * x - 1.0;
        let x3 = (4.0 * x * x - 3.0) * x;
        let magnet = x + 0.4 * x2 + 0.15 * x3;
        let gear = self.gear.next() * GEAR_LEVEL;
        let inverter = self.inverter.next() * INVERTER_LEVEL;
        (magnet + gear + inverter) * self.level
    }
}
