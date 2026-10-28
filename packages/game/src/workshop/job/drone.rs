use std::collections::VecDeque;
use std::f32::consts::{FRAC_PI_3, FRAC_PI_4};

use bevy::prelude::*;
use ld60_core::audio::{Play, Sound};
use ld60_core::layout::counter::CounterLayout;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use rand::{RngExt, SeedableRng, rngs::SmallRng};

use crate::device::{Carried, Casing};

/// The lane the drone flies along the street, just outside the shop, see
/// [`CounterLayout::place`]. It turns in over the counter from there.
const LANE: f32 = -2.2;
/// How high the lane is over the counter top, over the lamp.
const LANE_HEIGHT: f32 = 0.55;
/// How far left or right down the lane the drone comes from and goes to,
/// out of sight of the player, in meters.
const AWAY: f32 = 4.0;
/// How high over the counter the drone flies between stops, clear of the
/// lamp. It drops straight down to a stop from there.
const CRUISE: f32 = 0.55;
/// How far over a thing the drone hangs to pick it up or set it down, so
/// the claw just clears it.
const HOLD: f32 = 0.1;
const ENTER_TIME: f32 = 2.4;
const HOP_TIME: f32 = 0.9;
const DIP_TIME: f32 = 0.9;
/// Seconds a sheet takes to go up into the pod.
const STOW_TIME: f32 = 0.6;
/// How small a sheet gets as it goes into the pod, folded up.
const STOWED: f32 = 0.08;
/// Seconds the customer takes to try a device they hand back.
const TRY_TIME: f32 = 1.2;
/// How far the drone leans into its flight, in radians.
const TILT: f32 = 0.25;
/// Turns of the rotors each second.
const SPIN: f32 = 30.0;

const BODY: Vec3 = Vec3::new(0.06, 0.022, 0.06);
const ARM: Vec3 = Vec3::new(0.15, 0.006, 0.012);
const MOTOR: Vec3 = Vec3::new(0.016, 0.014, 0.016);
const BLADE: Vec3 = Vec3::new(0.06, 0.001, 0.008);
/// The pod under the body that sheets go into.
const POD: Vec3 = Vec3::new(0.042, 0.02, 0.036);
const HOOK: Vec3 = Vec3::new(0.004, 0.02, 0.004);
/// Where the tether leaves the claw, below the middle of the drone.
const CLAW_BOTTOM: f32 = -(BODY.y / 2.0 + POD.y + HOOK.y);
const TETHER: f32 = 0.0025;
const CLAW: Vec3 = Vec3::new(0.03, 0.006, 0.012);
const LIGHT: Vec3 = Vec3::new(0.008, 0.004, 0.008);
/// Worn colors of the night city, one for the shell of each drone.
const SHELLS: [Color; 6] = [
    Color::srgb_u8(0xc8, 0xa4, 0x3a),
    Color::srgb_u8(0x3a, 0x7a, 0x7a),
    Color::srgb_u8(0xa8, 0x55, 0x2e),
    Color::srgb_u8(0x4a, 0x4e, 0x54),
    Color::srgb_u8(0xc8, 0xc0, 0xa8),
    Color::srgb_u8(0x9a, 0x62, 0x70),
];
/// From kept nice to left out in the rain.
const SHELL_FINISHES: [Finish; 4] = [Finish::Plastic, Finish::Paint, Finish::Metal, Finish::Rust];
const LIGHTS: [Color; 3] = [
    Color::srgb_u8(0x50, 0xe0, 0x70),
    Color::srgb_u8(0xe0, 0x9a, 0x30),
    Color::srgb_u8(0x40, 0xd0, 0xe0),
];
/// Smallest and largest size of a drone, as a scale.
const SIZES: (f32, f32) = (0.85, 1.2);
const FRAME: Color = Color::srgb_u8(0x2e, 0x30, 0x34);
const BLADE_GRAY: Color = Color::srgb_u8(0x9a, 0x9e, 0xa4);

/// The delivery drone that carries devices between the customers and the
/// counter. It flies a [`Flight`]. Each customer has their own, picked by
/// `seed`.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Drone {
    seed: u64,
}

/// How the drone of a customer looks.
#[derive(Clone, Copy, Debug)]
struct Look {
    shell: Color,
    finish: Finish,
    light: Color,
    /// Two arms in a cross carry four rotors, three carry six.
    crosses: usize,
    size: f32,
}

/// What hangs from a [`Tether`], apart from the drone and the tethers.
type Hanging = (Without<Drone>, Without<Tether>);

/// The line from the claw of a [`Drone`] down to the device it carries.
#[derive(Component, Debug)]
pub(super) struct Tether {
    item: Entity,
}

/// A rotor of a [`Drone`], spinning.
#[derive(Component, Debug)]
pub(super) struct Rotor;

/// What a [`Drone`] does next.
#[derive(Debug)]
enum Step {
    Fly {
        to: Vec3,
        time: f32,
    },
    Wait(f32),
    /// Picks up the thing under it. A sheet goes into the pod, out of
    /// sight, a device hangs from the claw.
    Grab {
        item: Entity,
        stow: bool,
    },
    /// Sets down what it carries at its rest.
    Drop {
        item: Entity,
        rest: Transform,
    },
}

impl Step {
    fn time(&self) -> f32 {
        match self {
            Self::Fly { time, .. } => *time,
            Self::Wait(time) => *time,
            Self::Grab { .. } | Self::Drop { .. } => 0.0,
        }
    }
}

/// The steps of a [`Drone`], one after the other. What it still carries at
/// the end goes with it.
#[derive(Component, Debug)]
pub(crate) struct Flight {
    steps: VecDeque<Step>,
    /// Seconds to wait before it comes.
    delay: f32,
    elapsed: f32,
    /// Where the current step started.
    from: Vec3,
    carried: Vec<Cargo>,
    /// Whether it has come out of the street yet.
    came: bool,
}

/// Something a [`Drone`] carries.
#[derive(Debug)]
struct Cargo {
    item: Entity,
    /// Where it hangs from the drone.
    offset: Vec3,
    /// How far it went into the pod, from 0 to 1, if it goes there.
    stowing: Option<f32>,
}

impl Flight {
    /// The lane over the street in front of `x`.
    fn lane(x: f32) -> Vec3 {
        let front = CounterLayout::new()
            .place(Vec2::new(0.0, LANE), 0.0)
            .translation;
        Vec3::new(x, front.y + LANE_HEIGHT, front.z)
    }

    /// Where the drone of the customer `seed` comes from and goes to: down
    /// the lane to the left or to the right.
    fn away(seed: u64) -> Vec3 {
        let side = if seed.is_multiple_of(2) { -1.0 } else { 1.0 };
        Self::lane(side * AWAY)
    }

    fn new(seed: u64, delay: f32, carried: Vec<Cargo>, steps: Vec<Step>) -> Self {
        Self {
            steps: steps.into(),
            delay,
            elapsed: 0.0,
            from: Self::away(seed),
            carried,
            came: false,
        }
    }

    /// What it carries now or will pick up or set down.
    pub(crate) fn items(&self) -> impl Iterator<Item = Entity> {
        let steps = self.steps.iter().filter_map(|step| match *step {
            Step::Grab { item, .. } | Step::Drop { item, .. } => Some(item),
            Step::Fly { .. } | Step::Wait(_) => None,
        });
        self.carried.iter().map(|cargo| cargo.item).chain(steps)
    }

    /// Seconds from the start until after step `count`, delay included.
    fn time_until(&self, count: usize) -> f32 {
        self.delay + self.steps.iter().take(count).map(Step::time).sum::<f32>()
    }

    /// Seconds until the drone is gone.
    pub(super) fn duration(&self) -> f32 {
        self.time_until(self.steps.len())
    }

    /// Seconds until it set something down.
    fn landed(&self) -> f32 {
        let down = self
            .steps
            .iter()
            .rposition(|step| matches!(step, Step::Drop { .. }))
            .map_or(0, |index| index + 1);
        self.time_until(down)
    }

    /// Comes along the lane from `seed`'s side to the front of `rest`.
    fn come(steps: &mut Vec<Step>, seed: u64, rest: Vec3) {
        steps.extend([
            Step::Fly {
                to: Self::lane(Self::away(seed).x),
                time: 0.0,
            },
            Step::Fly {
                to: Self::lane(rest.x),
                time: ENTER_TIME,
            },
        ]);
    }

    /// Goes back out to the lane in front of `rest` and down it.
    fn go(steps: &mut Vec<Step>, seed: u64, rest: Vec3) {
        steps.extend([
            Step::Fly {
                to: Self::lane(rest.x),
                time: HOP_TIME,
            },
            Step::Fly {
                to: Self::away(seed),
                time: ENTER_TIME,
            },
        ]);
    }

    /// Flies over to `rest`, down to it and up again, with `acts` in
    /// between.
    fn visit(steps: &mut Vec<Step>, rest: Vec3, acts: impl IntoIterator<Item = Step>) {
        let above = rest + Vec3::Y * CRUISE;
        steps.extend([
            Step::Fly {
                to: above,
                time: HOP_TIME,
            },
            Step::Fly {
                to: rest + Vec3::Y * HOLD,
                time: DIP_TIME,
            },
        ]);
        steps.extend(acts);
        steps.push(Step::Fly {
            to: above,
            time: DIP_TIME,
        });
    }

    /// The drone of the customer `seed` takes `items` away for good, one
    /// after the other. The ones marked go into the pod.
    pub(super) fn take(seed: u64, items: &[(Entity, Transform, bool)]) -> Self {
        let mut steps = Vec::new();
        let (Some(first), Some(last)) = (items.first(), items.last()) else {
            return Self::new(seed, 0.0, Vec::new(), steps);
        };
        Self::come(&mut steps, seed, first.1.translation);
        for &(item, rest, stow) in items {
            // It waits for a sheet to go into the pod.
            let wait = stow.then_some(Step::Wait(STOW_TIME));
            Self::visit(
                &mut steps,
                rest.translation,
                [Step::Grab { item, stow }].into_iter().chain(wait),
            );
        }
        Self::go(&mut steps, seed, last.1.translation);
        Self::new(seed, 0.0, Vec::new(), steps)
    }

    /// The drone of the customer `seed` brings `item` to `rest` after
    /// `delay`. Also returns the seconds until it is down.
    pub(super) fn bring(seed: u64, item: Entity, rest: Transform, delay: f32) -> (Self, f32) {
        let mut steps = Vec::new();
        Self::come(&mut steps, seed, rest.translation);
        Self::visit(&mut steps, rest.translation, [Step::Drop { item, rest }]);
        Self::go(&mut steps, seed, rest.translation);
        let cargo = Cargo {
            item,
            offset: Vec3::NEG_Y * HOLD,
            stowing: None,
        };
        let flight = Self::new(seed, delay, vec![cargo], steps);
        let landed = flight.landed();
        (flight, landed)
    }

    /// The drone of the customer `seed` takes `item` to them to try, out of
    /// sight, and brings it back to `rest`. Also returns the seconds until
    /// it is back down.
    pub(super) fn try_out(seed: u64, item: Entity, rest: Transform) -> (Self, f32) {
        let mut steps = Vec::new();
        Self::come(&mut steps, seed, rest.translation);
        Self::visit(
            &mut steps,
            rest.translation,
            [Step::Grab { item, stow: false }],
        );
        Self::go(&mut steps, seed, rest.translation);
        steps.push(Step::Wait(TRY_TIME));
        Self::come(&mut steps, seed, rest.translation);
        Self::visit(&mut steps, rest.translation, [Step::Drop { item, rest }]);
        Self::go(&mut steps, seed, rest.translation);
        let flight = Self::new(seed, 0.0, Vec::new(), steps);
        let landed = flight.landed();
        (flight, landed)
    }

    pub(super) fn fly(
        time: Res<Time>,
        mut drones: Query<(Entity, &mut Flight, &mut Transform, &mut Visibility), With<Drone>>,
        mut items: Query<(&mut Transform, &mut Visibility), Without<Drone>>,
        tethers: Query<(Entity, &Tether)>,
        mut commands: Commands,
    ) {
        let dt = time.delta_secs();
        for (drone, mut flight, mut transform, mut visibility) in &mut drones {
            let flight = &mut *flight;
            // Out of sight, with what it brings, until it comes.
            let waiting = flight.delay > 0.0;
            flight.delay -= dt;
            let shown = if waiting {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            visibility.set_if_neq(shown);
            // What it brings shows once it comes. What it takes later goes
            // into the pod and stays out of sight.
            if waiting || !flight.came {
                for cargo in &flight.carried {
                    if let Ok((_, mut visibility)) = items.get_mut(cargo.item) {
                        visibility.set_if_neq(shown);
                    }
                }
                flight.came = !waiting;
            }
            if waiting {
                transform.translation = flight.from;
                continue;
            }
            flight.elapsed += dt;
            let mut tilt = Quat::IDENTITY;
            match flight.steps.front() {
                Some(&Step::Fly { to, time }) => {
                    let t = if time > 0.0 {
                        (flight.elapsed / time).min(1.0)
                    } else {
                        1.0
                    };
                    let eased = t * t * (3.0 - 2.0 * t);
                    transform.translation = flight.from.lerp(to, eased);
                    // Leans into the way it goes while it speeds up.
                    let along = (to - flight.from).with_y(0.0);
                    if let Some(axis) = along.cross(Vec3::NEG_Y).try_normalize() {
                        let lean = TILT * (std::f32::consts::PI * t).sin();
                        tilt = Quat::from_axis_angle(axis, -lean);
                    }
                    if t >= 1.0 {
                        flight.from = to;
                        flight.elapsed = 0.0;
                        flight.steps.pop_front();
                    }
                }
                Some(&Step::Wait(time)) => {
                    if flight.elapsed >= time {
                        flight.elapsed = 0.0;
                        flight.steps.pop_front();
                    }
                }
                Some(&Step::Grab { item, stow }) => {
                    if !stow {
                        Tether::attach(drone, item, &mut commands);
                    }
                    if let Ok((item_at, _)) = items.get(item) {
                        flight.carried.push(Cargo {
                            item,
                            offset: item_at.translation - transform.translation,
                            stowing: stow.then_some(0.0),
                        });
                        commands.entity(item).try_insert(Carried);
                    }
                    flight.elapsed = 0.0;
                    flight.steps.pop_front();
                }
                Some(&Step::Drop { item, rest }) => {
                    flight.carried.retain(|cargo| cargo.item != item);
                    for (line, tether) in &tethers {
                        if tether.item == item {
                            commands.entity(line).despawn();
                        }
                    }
                    if let Ok((mut item_at, _)) = items.get_mut(item) {
                        *item_at = rest;
                    }
                    commands.entity(item).try_remove::<Carried>();
                    commands.trigger(Play(Sound::Thunk));
                    flight.elapsed = 0.0;
                    flight.steps.pop_front();
                }
                None => {
                    for cargo in &flight.carried {
                        commands.entity(cargo.item).try_despawn();
                    }
                    commands.entity(drone).despawn();
                    continue;
                }
            }
            transform.rotation = tilt;
            for cargo in &mut flight.carried {
                let Ok((mut item_at, mut shown)) = items.get_mut(cargo.item) else {
                    continue;
                };
                let mut offset = cargo.offset;
                // A sheet folds up as it goes up into the pod.
                if let Some(stowing) = cargo.stowing.as_mut() {
                    *stowing = (*stowing + dt / STOW_TIME).min(1.0);
                    let eased = *stowing * *stowing * (3.0 - 2.0 * *stowing);
                    offset = offset.lerp(Vec3::ZERO, eased);
                    item_at.scale = Vec3::splat(1.0 - (1.0 - STOWED) * eased);
                    if *stowing >= 1.0 {
                        shown.set_if_neq(Visibility::Hidden);
                    }
                }
                item_at.translation = transform.translation + offset;
            }
        }
    }

    /// Sends the drone of the customer `seed` stands for, the one its
    /// flight was planned for.
    pub(super) fn spawn(self, seed: u64, commands: &mut Commands) {
        let from = self.from;
        let brought: Vec<_> = self.carried.iter().map(|cargo| cargo.item).collect();
        let drone = commands
            .spawn((
                Name::new("drone"),
                Drone { seed },
                self,
                Transform::from_translation(from).with_scale(Vec3::splat(Look::of(seed).size)),
                Visibility::Hidden,
            ))
            .id();
        for item in brought {
            Tether::attach(drone, item, commands);
        }
    }
}

impl Tether {
    fn attach(drone: Entity, item: Entity, commands: &mut Commands) {
        let line = Block::new(Vec3::new(TETHER, 1.0, TETHER));
        commands.spawn((
            Tether { item },
            Shape::cylinder(&line, FRAME),
            Finish::Fabric,
            ChildOf(drone),
        ));
    }

    /// Runs the line from the claw to the top of the device.
    pub(super) fn follow(
        drones: Query<&Transform, (With<Drone>, Without<Tether>)>,
        items: Query<(&Transform, Option<&Casing>), Hanging>,
        mut tethers: Query<(&Tether, &ChildOf, &mut Transform)>,
    ) {
        for (tether, parent, mut transform) in &mut tethers {
            let (Ok(drone), Ok((item, casing))) =
                (drones.get(parent.parent()), items.get(tether.item))
            else {
                continue;
            };
            let top = casing.map_or(0.0, |casing| casing.bounds().max().y);
            let target = drone
                .compute_affine()
                .inverse()
                .transform_point3(item.transform_point(Vec3::Y * top));
            let claw = Vec3::Y * CLAW_BOTTOM;
            let along = target - claw;
            let length = along.length();
            *transform = Transform::from_translation(claw + along / 2.0)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, along / length.max(1e-6)))
                .with_scale(Vec3::new(1.0, length, 1.0));
        }
    }
}

impl Look {
    fn of(seed: u64) -> Self {
        let mut rng = SmallRng::seed_from_u64(seed);
        Self {
            shell: SHELLS[rng.random_range(0..SHELLS.len())],
            finish: SHELL_FINISHES[rng.random_range(0..SHELL_FINISHES.len())],
            light: LIGHTS[rng.random_range(0..LIGHTS.len())],
            crosses: if rng.random_bool(0.3) { 3 } else { 2 },
            size: rng.random_range(SIZES.0..SIZES.1),
        }
    }
}

impl Drone {
    pub(super) fn on_add(add: On<Add<Drone>>, drones: Query<&Drone>, mut commands: Commands) {
        let Ok(drone) = drones.get(add.entity) else {
            return;
        };
        let look = Look::of(drone.seed);
        let body = Block::new(BODY).translated(Vec3::NEG_Y * BODY.y / 2.0);
        let light = Block::new(LIGHT).on_top_of(&body);
        let pod = Block::new(POD).translated(Vec3::NEG_Y * (BODY.y / 2.0 + POD.y));
        let hook =
            Block::new(HOOK).translated(pod.min().with_x(0.0).with_z(0.0) - Vec3::Y * HOOK.y);
        let claw = Block::new(CLAW).translated(hook.min());
        let arm = Block::new(ARM).translated(Vec3::NEG_Y * ARM.y / 2.0);
        commands.entity(add.entity).with_children(|drone| {
            drone.spawn((Shape::cuboid(&body, look.shell), look.finish));
            drone.spawn((Shape::cuboid(&light, look.light), Finish::Plastic));
            drone.spawn((Shape::cuboid(&pod, FRAME), Finish::Plastic));
            drone.spawn((Shape::cylinder(&hook, FRAME), Finish::Metal));
            drone.spawn((Shape::cuboid(&claw, FRAME), Finish::Metal));
            // Arms in a star, a motor and a rotor at each end.
            let step = if look.crosses == 2 {
                2.0 * FRAC_PI_4
            } else {
                FRAC_PI_3
            };
            for index in 0..look.crosses {
                let rotation = Quat::from_rotation_y(FRAC_PI_4 + step * index as f32);
                drone
                    .spawn((Transform::from_rotation(rotation), Visibility::Inherited))
                    .with_children(|cross| {
                        cross.spawn((Shape::cuboid(&arm, FRAME), Finish::Metal));
                        for end in [-1.0, 1.0] {
                            let at = Vec3::X * end * ARM.x / 2.0;
                            let motor = Block::new(MOTOR).translated(at - Vec3::Y * MOTOR.y / 2.0);
                            cross.spawn((Shape::cylinder(&motor, FRAME), Finish::Metal));
                            let blade = Block::new(BLADE).translated(at + Vec3::Y * MOTOR.y / 2.0);
                            cross.spawn((
                                Rotor,
                                Shape::cuboid(&blade, BLADE_GRAY),
                                Finish::Plastic,
                            ));
                        }
                    });
            }
        });
    }

    pub(super) fn spin(time: Res<Time>, mut rotors: Query<&mut Transform, With<Rotor>>) {
        let turn = Quat::from_rotation_y(time.delta_secs() * SPIN * std::f32::consts::TAU);
        for mut rotor in &mut rotors {
            rotor.rotation = turn * rotor.rotation;
        }
    }
}
