use std::f32::consts::TAU;

use bevy::prelude::*;
use ld60_game_core::layout::counter::CounterLayout;
use ld60_geometry::block::Block;

use super::Casing;
use crate::phillips::Phillips;
use crate::thread::Thread;

const GRAVITY: f32 = 9.81;
/// Steps of the simulation per frame, and the longest frame it takes in
/// full.
const SUBSTEPS: usize = 16;
const MAX_FRAME: f32 = 1.0 / 30.0;
/// Of steel on the rubber mat or the wooden counter.
const STATIC_FRICTION: f32 = 0.6;
const DYNAMIC_FRICTION: f32 = 0.4;
const RESTITUTION: f32 = 0.1;
/// How much of its turning a screw that touches something loses per
/// second, as it rolls on the soft mat.
const ROLLING_DRAG: f32 = 20.0;
/// A screw slower than these, in meters and radians per second, for this
/// many seconds lies still.
const STILL_SPEED: f32 = 0.004;
const STILL_SPIN: f32 = 0.4;
const STILL_TIME: f32 = 0.25;
/// Points around each ring of the shape.
const RING_POINTS: usize = 8;
/// How fast a screw that came to rest on top of the device topples as it
/// is pushed off, in radians per second.
const NUDGE_TOPPLE: f32 = 2.0;
/// A push off the device carries the middle of a screw this far past the
/// edge.
const PAST_EDGE: f32 = 0.004;
/// A screw that hits the mat or the device slower than this, in meters per
/// second, makes no sound.
const SILENT_SPEED: f32 = 0.04;
/// A screw counts as over the room once its middle is this far inside the
/// walls.
const ROOM_INSET: f32 = 0.002;

/// A screw falling onto the counter and rolling to a stop.
///
/// It is one rigid body against the floor, the device, the walls of the
/// room it may roll in once it is over it and the other screws, solved as
/// in Müller et al., "Detailed Rigid
/// Body Simulation with Extended Position Based Dynamics" (2020).
#[derive(Clone, Copy, Debug)]
pub(super) struct Fall {
    /// The middle of its mass.
    at: Vec3,
    rotation: Quat,
    velocity: Vec3,
    spin: Vec3,
    /// Seconds it has been slow.
    still: f32,
    asleep: bool,
    /// Whether it has come over the room. Only then do its walls hold it.
    in_room: bool,
}

/// What a falling screw lands on and rolls in.
pub(super) struct Ground {
    /// The mat on the counter.
    mat: Block,
    /// The height of the counter around it.
    counter: f32,
    /// Where the screws may roll: the middle of the room, turned as it is,
    /// and its size across the counter.
    pub(super) room: Transform,
    room_size: Vec2,
    /// The box of the device, where it is, and its size.
    device: Option<(Transform, Block)>,
    /// Where the device rests when it is put down, turned by its yaw only,
    /// and half its size across. Nothing stays over it.
    footprint: Option<(Transform, Vec2)>,
}

/// A screw hitting the mat or the device.
#[derive(Clone, Copy, Debug)]
pub(super) struct Landing {
    /// Which of the screws stepped together.
    pub(super) screw: usize,
    pub(super) at: Vec3,
    /// How fast it came in, in meters per second.
    pub(super) speed: f32,
}

/// A contact of a screw against something, for its velocity pass.
struct Contact {
    body: usize,
    other: Option<usize>,
    /// From the middle of the mass of each body to the contact.
    arm: Vec3,
    other_arm: Vec3,
    /// From the other body or the ground, into the screw.
    normal: Vec3,
    /// The push along the normal in this step.
    push: f32,
    /// The speed along the normal before the step.
    approach: f32,
}

impl Fall {
    /// A screw at `pose`, of an entity whose origin is where the head meets
    /// the shank, moving at `velocity` and turning at `spin`.
    pub(super) fn new(pose: Transform, velocity: Vec3, spin: Vec3) -> Self {
        Self {
            at: pose.transform_point(Self::middle()),
            rotation: pose.rotation,
            velocity,
            spin,
            still: 0.0,
            asleep: false,
            in_room: false,
        }
    }

    /// A screw at `pose` on top of the device, pushed towards its nearest
    /// edge and toppling that way at `topple`, so it tips over and falls
    /// off. It goes at `speed` at least, and fast enough to clear where the
    /// device rests. `forward` leans the push towards the front, from 0 for
    /// straight out to 1 for halfway.
    pub(super) fn tipped(
        pose: Transform,
        ground: &Ground,
        forward: f32,
        (speed, topple): (f32, f32),
    ) -> Self {
        let mut fall = Self::new(pose, Vec3::ZERO, Vec3::ZERO);
        if let Some((out, distance)) = ground.off_device(fall.at) {
            let height = (fall.at.y - ground.floor_at(fall.at)).max(0.0);
            let speed = speed.max(Self::clearing(distance, height));
            let out = (out + ground.front() * forward).normalize_or(out);
            fall.push(out, speed, topple);
        }
        fall
    }

    /// How fast a screw `height` over the floor has to go to slide
    /// `distance` to the edge of what it lies on and fall clear of it, see
    /// [`PAST_EDGE`].
    fn clearing(distance: f32, height: f32) -> f32 {
        let falling = (2.0 * height / GRAVITY).sqrt().max(1e-3);
        ((PAST_EDGE / falling).powi(2) + 2.0 * DYNAMIC_FRICTION * GRAVITY * distance).sqrt()
    }

    /// Where the entity of the screw is.
    pub(super) fn pose(&self) -> Transform {
        Transform::from_translation(self.at - self.rotation * Self::middle())
            .with_rotation(self.rotation)
    }

    pub(super) fn asleep(&self) -> bool {
        self.asleep
    }

    /// Stops it where it is.
    pub(super) fn hold(&mut self) {
        self.asleep = true;
    }

    /// Pushes it off the top of the device, if it lies there, towards the
    /// nearest edge and just fast enough to slide over it.
    fn push_off(&mut self, ground: &Ground) {
        let Some((out, distance)) = ground.off_device(self.at) else {
            return;
        };
        let height = (self.at.y - ground.floor_at(self.at)).max(0.0);
        self.push(out, Self::clearing(distance, height), NUDGE_TOPPLE);
    }

    /// Wakes it, moving along `out` at `speed` and toppling that way at
    /// `topple`.
    fn push(&mut self, out: Vec3, speed: f32, topple: f32) {
        self.velocity = out * speed;
        self.spin = Vec3::Y.cross(out) * topple;
        self.asleep = false;
        self.still = 0.0;
    }

    /// The middle of the mass, from where the head meets the shank, with
    /// the shank along +Y. The head is the heavier part.
    fn middle() -> Vec3 {
        let (head, shank) = Self::masses();
        let head_at = -Phillips::HEAD.y / 2.0;
        let shank_at = Thread::SIZE.y / 2.0;
        Vec3::Y * (head * head_at + shank * shank_at) / (head + shank)
    }

    /// The share of the mass in the head and in the shank, by volume.
    fn masses() -> (f32, f32) {
        let head = (Phillips::HEAD.x / 2.0).powi(2) * Phillips::HEAD.y;
        // The thread fills a little over half the space around the core.
        let shank = (Thread::SIZE.x / 2.0 * 0.85).powi(2) * Thread::SIZE.y;
        (head, shank)
    }

    /// One over the moments of inertia about the middle of the mass, along
    /// the screw's own axes. It turns easily about its length.
    fn inverse_inertia() -> Vec3 {
        let (head, shank) = Self::masses();
        let total = head + shank;
        let (head, shank) = (head / total, shank / total);
        let middle = Self::middle().y;
        let (head_r, shank_r) = (Phillips::HEAD.x / 2.0, Thread::SIZE.x / 2.0 * 0.85);
        let (head_h, shank_h) = (Phillips::HEAD.y, Thread::SIZE.y);
        let along = 0.5 * (head * head_r * head_r + shank * shank_r * shank_r);
        let across = |mass: f32, radius: f32, height: f32, at: f32| {
            mass * (radius * radius / 4.0 + height * height / 12.0 + (at - middle).powi(2))
        };
        let across = across(head, head_r, head_h, -head_h / 2.0)
            + across(shank, shank_r, shank_h, shank_h / 2.0);
        Vec3::new(1.0 / across, 1.0 / along, 1.0 / across)
    }

    /// Points on the outside of the screw, from the middle of the mass: the
    /// round edge of the head, the rim where it meets the shank, the crests
    /// of the thread and the point.
    fn hull() -> Vec<Vec3> {
        let middle = Self::middle();
        let head = Phillips::HEAD;
        let thread = Thread::SIZE;
        let rings = [
            (head.x / 2.0 - 0.0004, -head.y),
            (head.x / 2.0, -head.y * 0.6),
            (head.x / 2.0, 0.0),
            (thread.x / 2.0, 0.001),
            (thread.x / 2.0, thread.y * 0.5),
            (thread.x / 2.0, thread.y * 0.8),
        ];
        rings
            .iter()
            .flat_map(|&(radius, y)| {
                (0..RING_POINTS).map(move |k| {
                    let angle = TAU * k as f32 / RING_POINTS as f32;
                    Vec3::new(angle.cos() * radius, y, angle.sin() * radius)
                })
            })
            .chain([Vec3::Y * thread.y])
            .map(|point| point - middle)
            .collect()
    }

    /// Balls along the screw that keep screws apart, from the middle of the
    /// mass, with their radii.
    fn balls() -> [(Vec3, f32); 4] {
        let middle = Self::middle();
        let thread = Thread::SIZE;
        [
            (Vec3::NEG_Y * Phillips::HEAD.y / 2.0, Phillips::HEAD.x / 2.0),
            (Vec3::Y * thread.y * 0.2, thread.x / 2.0),
            (Vec3::Y * thread.y * 0.5, thread.x / 2.0),
            (Vec3::Y * thread.y * 0.8, thread.x / 2.0),
        ]
        .map(|(at, radius)| (at - middle, radius))
    }

    fn inverse_inertia_world(&self) -> Mat3 {
        let turn = Mat3::from_quat(self.rotation);
        turn * Mat3::from_diagonal(Self::inverse_inertia()) * turn.transpose()
    }

    /// How hard it is to move the point at `arm` along `normal`, as one
    /// over an effective mass. The mass of a screw is one.
    fn give(&self, arm: Vec3, normal: Vec3) -> f32 {
        let turn = arm.cross(normal);
        1.0 + turn.dot(self.inverse_inertia_world() * turn)
    }

    /// Moves it by `push`, a change of position, at `arm`.
    fn shift(&mut self, push: Vec3, arm: Vec3) {
        self.at += push;
        let turn = self.inverse_inertia_world() * arm.cross(push);
        self.rotation = (self.rotation
            + Quat::from_xyzw(turn.x, turn.y, turn.z, 0.0) * self.rotation * 0.5)
            .normalize();
    }

    /// Changes its speed by `impulse` at `arm`.
    fn kick(&mut self, impulse: Vec3, arm: Vec3) {
        self.velocity += impulse;
        self.spin += self.inverse_inertia_world() * arm.cross(impulse);
    }

    fn speed_at(&self, arm: Vec3) -> Vec3 {
        self.velocity + self.spin.cross(arm)
    }

    /// Moves `screws` on by `dt` seconds. Sleeping ones stay put, but the
    /// others still bump into them. The hardest hit of each screw on the
    /// mat or the device, if it is loud enough to hear, goes into
    /// `landings`.
    pub(super) fn step_all(
        screws: &mut [Fall],
        ground: &Ground,
        dt: f32,
        landings: &mut Vec<Landing>,
    ) {
        let first = landings.len();
        if screws.iter().all(Fall::asleep) {
            return;
        }
        let h = dt.min(MAX_FRAME) / SUBSTEPS as f32;
        let start: Vec<Fall> = screws.to_vec();
        let hull = Self::hull();
        for _ in 0..SUBSTEPS {
            let before: Vec<Fall> = screws.to_vec();
            for screw in screws.iter_mut().filter(|screw| !screw.asleep) {
                screw.velocity += Vec3::NEG_Y * GRAVITY * h;
                screw.at += screw.velocity * h;
                let spin = screw.spin * h * 0.5;
                screw.rotation = (screw.rotation
                    + Quat::from_xyzw(spin.x, spin.y, spin.z, 0.0) * screw.rotation)
                    .normalize();
                screw.in_room |= ground.over_room(screw.at);
            }
            let mut contacts = Vec::new();
            for (index, screw) in screws.iter_mut().enumerate() {
                if screw.asleep {
                    continue;
                }
                for &point in &hull {
                    screw.collide(index, point, &before[index], ground, &mut contacts);
                }
            }
            Self::keep_apart(screws, &before, &mut contacts);
            for (screw, was) in screws.iter_mut().zip(&before) {
                if screw.asleep {
                    continue;
                }
                screw.velocity = (screw.at - was.at) / h;
                let turn = screw.rotation * was.rotation.inverse();
                let turn = if turn.w < 0.0 { -turn } else { turn };
                screw.spin = 2.0 * Vec3::new(turn.x, turn.y, turn.z) / h;
            }
            for contact in &contacts {
                Self::bounce(screws, contact, h);
                let speed = -contact.approach;
                if contact.other.is_some() || contact.normal.y < 0.7 || speed < SILENT_SPEED {
                    continue;
                }
                let at = screws[contact.body].at + contact.arm;
                match landings[first..]
                    .iter_mut()
                    .find(|landing| landing.screw == contact.body)
                {
                    Some(landing) if landing.speed < speed => {
                        *landing = Landing {
                            screw: contact.body,
                            at,
                            speed,
                        };
                    }
                    Some(_) => {}
                    None => landings.push(Landing {
                        screw: contact.body,
                        at,
                        speed,
                    }),
                }
            }
            for (index, screw) in screws.iter_mut().enumerate() {
                if screw.asleep {
                    continue;
                }
                if contacts.iter().any(|contact| contact.body == index) {
                    screw.spin *= (-ROLLING_DRAG * h).exp();
                }
            }
        }
        // Contacts nudge a screw back and forth within a frame, so how far it
        // got over the whole frame tells whether it lies still.
        let frame = h * SUBSTEPS as f32;
        for (screw, was) in screws.iter_mut().zip(&start) {
            if screw.asleep {
                continue;
            }
            let speed = screw.at.distance(was.at) / frame;
            let spin = screw.rotation.angle_between(was.rotation) / frame;
            let slow = speed < STILL_SPEED && spin < STILL_SPIN;
            screw.still = if slow { screw.still + frame } else { 0.0 };
            if screw.still >= STILL_TIME {
                screw.asleep = true;
                screw.velocity = Vec3::ZERO;
                screw.spin = Vec3::ZERO;
                // Nothing stays on top of the device, whose cover comes off.
                screw.push_off(ground);
            }
        }
    }

    /// Keeps the point `local` of the screw, in its own space from the
    /// middle of the mass, above the ground and inside the room, with
    /// friction. The screw was `was` a step ago.
    fn collide(
        &mut self,
        index: usize,
        local: Vec3,
        was: &Fall,
        ground: &Ground,
        contacts: &mut Vec<Contact>,
    ) {
        let point = self.at + self.rotation * local;
        let mut planes = Vec::new();
        let floor = ground.floor_at(point);
        if point.y < floor {
            planes.push((Vec3::Y, floor - point.y, true));
        }
        if let Some((device, bounds)) = &ground.device {
            let inside = device.rotation.inverse() * (point - device.translation) - bounds.center;
            let half = bounds.size / 2.0;
            if (half - inside.abs()).min_element() > 0.0 {
                // Out through the nearest face, but never down through the
                // one it stands on.
                let exit = (0..3)
                    .flat_map(|axis| [(axis, 1.0), (axis, -1.0)])
                    .map(|(axis, side)| {
                        let mut out = Vec3::ZERO;
                        out[axis] = side;
                        (half[axis] - side * inside[axis], device.rotation * out)
                    })
                    .filter(|(_, normal)| normal.y > -0.5)
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                if let Some((depth, normal)) = exit {
                    planes.push((normal, depth, normal.y > 0.7));
                }
            }
        }
        let inside = ground.room.rotation.inverse() * (point - ground.room.translation);
        let half = ground.room_size / 2.0;
        for (axis, size) in [(Vec3::X, half.x), (Vec3::Z, half.y)] {
            let along = inside.dot(axis);
            if self.in_room && along.abs() > size {
                let inward = ground.room.rotation * axis * -along.signum();
                planes.push((inward, along.abs() - size, false));
            }
        }
        for (normal, depth, on_floor) in planes {
            let arm = self.rotation * local;
            let approach = was.speed_at(was.rotation * local).dot(normal);
            let push = depth / self.give(arm, normal);
            self.shift(normal * push, arm);
            // Static friction holds the point where it touched, as far as
            // the push along the normal allows.
            let slid = (self.at + self.rotation * local) - (was.at + was.rotation * local);
            let slid = slid - normal * slid.dot(normal);
            if on_floor && slid.length() > 1e-9 {
                let along = slid.normalize();
                let arm = self.rotation * local;
                let hold = slid.length() / self.give(arm, along);
                if hold < STATIC_FRICTION * push {
                    self.shift(-along * hold, arm);
                }
            }
            contacts.push(Contact {
                body: index,
                other: None,
                arm: self.rotation * local,
                other_arm: Vec3::ZERO,
                normal,
                push,
                approach,
            });
        }
    }

    /// Keeps the balls of each two screws apart.
    fn keep_apart(screws: &mut [Fall], before: &[Fall], contacts: &mut Vec<Contact>) {
        let balls = Self::balls();
        for i in 0..screws.len() {
            for j in i + 1..screws.len() {
                if screws[i].asleep && screws[j].asleep {
                    continue;
                }
                for &(a, ra) in &balls {
                    for &(b, rb) in &balls {
                        let (first, second) = (screws[i], screws[j]);
                        let (arm_a, arm_b) = (first.rotation * a, second.rotation * b);
                        let off = (first.at + arm_a) - (second.at + arm_b);
                        let distance = off.length();
                        let depth = ra + rb - distance;
                        if depth <= 0.0 || distance < 1e-9 {
                            continue;
                        }
                        let normal = off / distance;
                        let give = |screw: &Fall, arm: Vec3, normal: Vec3| {
                            if screw.asleep {
                                0.0
                            } else {
                                screw.give(arm, normal)
                            }
                        };
                        let (wa, wb) = (give(&first, arm_a, normal), give(&second, arm_b, -normal));
                        if wa + wb == 0.0 {
                            continue;
                        }
                        let push = depth / (wa + wb);
                        let approach =
                            (before[i].speed_at(arm_a) - before[j].speed_at(arm_b)).dot(normal);
                        if !first.asleep {
                            screws[i].shift(normal * push, arm_a);
                        }
                        if !second.asleep {
                            screws[j].shift(-normal * push, arm_b);
                        }
                        contacts.push(Contact {
                            body: i,
                            other: Some(j),
                            arm: arm_a,
                            other_arm: arm_b,
                            normal,
                            push,
                            approach,
                        });
                    }
                }
            }
        }
    }

    /// Slows the sliding at `contact` and lets it bounce back a little.
    fn bounce(screws: &mut [Fall], contact: &Contact, h: f32) {
        let body = screws[contact.body];
        let other = contact.other.map(|other| (other, screws[other]));
        let relative = body.speed_at(contact.arm)
            - other.map_or(Vec3::ZERO, |(_, other)| other.speed_at(contact.other_arm));
        let normal_speed = relative.dot(contact.normal);
        let sliding = relative - contact.normal * normal_speed;
        let mut change = Vec3::ZERO;
        if sliding.length() > 1e-9 {
            let slow = (DYNAMIC_FRICTION * contact.push / h).min(sliding.length());
            change -= sliding.normalize() * slow;
        }
        // Slow touches don't bounce, or a screw would never come to rest.
        let restitution = if -contact.approach > 2.0 * GRAVITY * h {
            RESTITUTION
        } else {
            0.0
        };
        change += contact.normal * (-normal_speed + (-restitution * contact.approach).max(0.0));
        let give = |screw: &Fall, arm: Vec3| {
            if screw.asleep {
                0.0
            } else {
                screw.give(arm, change.normalize_or_zero())
            }
        };
        let total = give(&body, contact.arm)
            + other.map_or(0.0, |(_, other)| give(&other, contact.other_arm));
        if total == 0.0 || change == Vec3::ZERO {
            return;
        }
        let impulse = change / total;
        if !body.asleep {
            screws[contact.body].kick(impulse, contact.arm);
        }
        if let Some((index, other)) = other
            && !other.asleep
        {
            screws[index].kick(-impulse, contact.other_arm);
        }
    }
}

impl Ground {
    /// The mat and the counter, and the room the screws of a device with
    /// `bounds` resting at `rest` roll in.
    pub(super) fn of(bounds: &Block, rest: &Transform) -> Self {
        let mat = CounterLayout::new().mat;
        let (room, room_size) = Casing::screw_room(bounds, rest);
        Self {
            mat,
            counter: mat.min().y,
            room,
            room_size,
            device: None,
            footprint: None,
        }
    }

    /// With the box of the device at `device`, `bounds` in its own space,
    /// which rests at `rest` when it is put down.
    pub(super) fn with_device(self, device: Transform, rest: Transform, bounds: Block) -> Self {
        Self {
            device: Some((device, bounds)),
            footprint: Some((Casing::resting_spot(&rest), bounds.size.xz() / 2.0)),
            ..self
        }
    }

    /// The way off the device, level, and how far it is, if `point` is over
    /// where it rests when it is put down. It is the way to the nearest
    /// edge but the back one, see [`Casing::screw_room`].
    fn off_device(&self, point: Vec3) -> Option<(Vec3, f32)> {
        let (spot, half) = self.footprint.as_ref()?;
        let inside = (spot.rotation.inverse() * (point - spot.translation)).xz();
        if (*half - inside.abs()).min_element() <= 0.0 {
            return None;
        }
        [
            (Vec3::X, half.x - inside.x),
            (Vec3::NEG_X, half.x + inside.x),
            (Vec3::Z, half.y - inside.y),
        ]
        .into_iter()
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(out, distance)| (spot.rotation * out, distance))
    }

    /// The way to the front of where the device rests, level.
    fn front(&self) -> Vec3 {
        self.footprint
            .map_or(Vec3::Z, |(spot, _)| spot.rotation * Vec3::Z)
    }

    /// Whether `point` is over the room, a little inside its walls.
    fn over_room(&self, point: Vec3) -> bool {
        let inside = self.room.rotation.inverse() * (point - self.room.translation);
        inside
            .xz()
            .abs()
            .cmplt(self.room_size / 2.0 - ROOM_INSET)
            .all()
    }

    /// The height of the mat at `point` or, off the mat, of the counter.
    fn floor_at(&self, point: Vec3) -> f32 {
        let (min, max) = (self.mat.min(), self.mat.max());
        let on_mat = point.xz().cmpge(min.xz()).all() && point.xz().cmple(max.xz()).all();
        if on_mat { max.y } else { self.counter }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ground() -> Ground {
        Ground {
            mat: Block::new(Vec3::new(0.4, 0.003, 0.3)).translated(Vec3::Y),
            counter: 1.0,
            room: Transform::from_xyz(0.0, 1.0, 0.0),
            room_size: Vec2::new(0.034, 0.06),
            device: None,
            footprint: None,
        }
    }

    fn dropped(at: Vec3, velocity: Vec3, spin: Vec3) -> Fall {
        let lying = Quat::from_rotation_z(-1.4) * Quat::from_rotation_y(0.4);
        Fall::new(
            Transform::from_translation(at).with_rotation(lying),
            velocity,
            spin,
        )
    }

    fn run(screws: &mut [Fall], ground: &Ground, seconds: f32) {
        for _ in 0..(seconds * 60.0) as usize {
            Fall::step_all(screws, ground, 1.0 / 60.0, &mut Vec::new());
        }
    }

    /// The lowest point of `screw`, and how far it is outside the room.
    fn reach(screw: &Fall, ground: &Ground) -> (f32, f32) {
        let half = ground.room_size / 2.0;
        Fall::hull()
            .iter()
            .map(|point| screw.at + screw.rotation * *point)
            .fold((f32::MAX, 0.0f32), |(low, out), point| {
                let inside = point - ground.room.translation;
                let over = (inside.xz().abs() - half).max(Vec2::ZERO).max_element();
                (low.min(point.y), out.max(over))
            })
    }

    #[test]
    fn a_dropped_screw_comes_to_rest_lying_on_its_side() {
        let ground = ground();
        let mut screws = [dropped(
            Vec3::new(0.0, 1.02, 0.0),
            Vec3::new(0.06, 0.0, 0.03),
            Vec3::new(1.0, -2.0, 1.5),
        )];
        run(&mut screws, &ground, 4.0);
        let screw = &screws[0];
        assert!(screw.asleep(), "{screw:?}");
        let (low, out) = reach(screw, &ground);
        let floor = ground.mat.max().y;
        assert!((low - floor).abs() < 2e-4, "lowest point {low}");
        assert!(out < 2e-4, "{out} out of the room");
        let axis = screw.rotation * Vec3::Y;
        assert!(axis.y.abs() < 0.4, "standing at {axis}");
    }

    #[test]
    fn screws_tipped_off_a_device_lie_on_the_mat_beside_where_it_rests() {
        let rest = Transform::from_xyz(0.05, 1.003, 0.0);
        let bounds = Block::new(Vec3::new(0.06, 0.02, 0.1));
        // Lying on the mat, and held up while it is looked at.
        for lift in [0.0, 0.12] {
            let device = rest.with_translation(rest.translation + Vec3::Y * lift);
            let (room, room_size) = Casing::screw_room(&bounds, &rest);
            let ground = Ground {
                room,
                room_size,
                ..ground()
            }
            .with_device(device, rest, bounds);
            let top = device.translation.y + bounds.size.y;
            // Upright on the device with the head up, as it comes out.
            let upright = Quat::from_rotation_x(std::f32::consts::PI);
            // Screw holes sit close to an edge.
            let holes = [
                (0.026, -0.044),
                (0.074, -0.044),
                (0.026, 0.044),
                (0.074, 0.044),
                (0.026, 0.0),
                (0.074, 0.01),
                (0.05, 0.044),
                (0.04, -0.044),
            ];
            for (n, (x, z)) in holes.into_iter().enumerate() {
                let out = Transform::from_xyz(x, top + Thread::SIZE.y, z).with_rotation(upright);
                let forward = 0.2 + 0.4 * (n as f32 * 1.3).sin().abs();
                let mut screws = [Fall::tipped(out, &ground, forward, (0.05, 6.0))];
                run(&mut screws, &ground, 8.0);
                let screw = &screws[0];
                assert!(screw.asleep(), "{lift} {n}: {screw:?}");
                let (low, out) = reach(screw, &ground);
                assert!(out < 3e-4, "{lift} {n}: {out} out of the room");
                assert!(
                    (low - ground.mat.max().y).abs() < 3e-4,
                    "{lift} {n}: lowest point {low}"
                );
                assert!(
                    ground.off_device(screw.at).is_none(),
                    "{lift} {n}: where the device rests"
                );
            }
        }
    }

    #[test]
    fn a_dropped_screw_lands_a_few_times_softer_each_time_then_goes_quiet() {
        let ground = ground();
        let mut screws = [dropped(
            Vec3::new(0.0, 1.03, 0.0),
            Vec3::new(0.05, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 3.0),
        )];
        let mut speeds = Vec::new();
        for _ in 0..240 {
            let mut landings = Vec::new();
            Fall::step_all(&mut screws, &ground, 1.0 / 60.0, &mut landings);
            speeds.extend(landings.iter().map(|landing| landing.speed));
        }
        assert!(screws[0].asleep());
        assert!((2..12).contains(&speeds.len()), "{speeds:?}");
        assert!(speeds[0] > 0.5, "{speeds:?}");
        assert!(
            speeds[1..].iter().all(|&speed| speed < speeds[0]),
            "{speeds:?}"
        );
    }

    #[test]
    fn screws_dropped_on_each_other_lie_apart() {
        let ground = ground();
        let mut screws = [0.0, 0.004, 0.008].map(|lift| {
            dropped(
                Vec3::new(0.002 * lift * 100.0, 1.02 + lift, 0.0),
                Vec3::ZERO,
                Vec3::ZERO,
            )
        });
        run(&mut screws, &ground, 5.0);
        let balls = Fall::balls();
        for (i, a) in screws.iter().enumerate() {
            assert!(a.asleep(), "{a:?}");
            assert!(reach(a, &ground).0 > ground.mat.max().y - 2e-4);
            for b in &screws[i + 1..] {
                for (x, rx) in balls {
                    for (y, ry) in balls {
                        let gap = (a.at + a.rotation * x).distance(b.at + b.rotation * y) - rx - ry;
                        assert!(gap > -3e-4, "{gap}");
                    }
                }
            }
        }
    }
}
