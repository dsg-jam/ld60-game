use bevy::{math::Affine3A, prelude::*};
use ld60_geometry::block::Block;

use super::{Cable, POINTS, RADIUS};

/// The cable is this much longer than the way between its ends, so it
/// hangs a little, and comes out of the computer or goes back in to keep it
/// so.
const SLACK: f32 = 1.25;
const EXTRA: f32 = 0.05;
/// How fast the cable comes out of or goes back into the computer, in
/// meters per second.
const REEL: f32 = 0.6;
/// Steps of the simulation per tick. Small steps keep it stiff and calm.
const SUBSTEPS: usize = 12;
/// The simulation moves on in ticks of this many seconds.
const TICK: f32 = 1.0 / 60.0;
/// A slow frame runs at most this many ticks and drops the rest.
const MAX_TICKS: u32 = 4;
/// Points from the plug end that bend over to where the plug is between
/// ticks.
const CATCH_UP: usize = 8;
/// How soft the cable bends, in meters per newton. Zero would be a rod.
const BEND_COMPLIANCE: f32 = 2e-7;
/// How much of its speed the cable loses per second.
const DAMPING: f32 = 8.0;
/// How much of its sliding a point on the counter keeps per step.
const FRICTION: f32 = 0.3;
/// A point on the counter slower than this, in meters per second, stops.
const STICK: f32 = 0.05;
/// The cable sleeps once no point moves farther than this in a tick.
const SLEEP: f32 = 5e-5;
const GRAVITY: f32 = 9.81;
/// Times the shown curve is pushed out of things each frame.
const SHOWN_PASSES: usize = 2;
/// How the drawn curve, a uniform cubic B-spline, mixes the points around
/// a point of the cable, and around the middle of a piece.
const KNOT: [f32; 3] = [1.0 / 6.0, 4.0 / 6.0, 1.0 / 6.0];
const QUARTER: [f32; 4] = [27.0 / 384.0, 235.0 / 384.0, 121.0 / 384.0, 1.0 / 384.0];
const MIDDLE: [f32; 4] = [1.0 / 48.0, 23.0 / 48.0, 23.0 / 48.0, 1.0 / 48.0];
const THREE_QUARTERS: [f32; 4] = [1.0 / 384.0, 121.0 / 384.0, 235.0 / 384.0, 27.0 / 384.0];
/// Points this close to the device turn along with it between ticks, in
/// meters from its box.
const CARRY_REACH: f32 = 0.03;

/// A box the cable keeps out of, where it is and how it got there.
struct Solid {
    at: Affine3A,
    /// The inverse of `at`.
    back: Affine3A,
    /// Moves a point along with the box from where the box was a moment
    /// ago.
    moved: Affine3A,
    bounds: Block,
}

impl Solid {
    fn new(at: Affine3A, then: Affine3A, bounds: Block) -> Self {
        Self {
            at,
            back: at.inverse(),
            moved: at * then.inverse(),
            bounds,
        }
    }
}

/// Where the cable ends at the plug, the way it leaves the plug, and the
/// box of the device it is plugged into.
#[derive(Clone, Copy, Debug)]
pub(super) struct Ends {
    pub(super) end: Vec3,
    pub(super) exit: Vec3,
    pub(super) device: Option<(Affine3A, Block)>,
    /// The manuals file while it lies open.
    pub(super) file: Option<(Affine3A, Block)>,
}

impl Ends {
    /// The ends `t` of the way from these to `other`.
    fn towards(&self, other: &Self, t: f32) -> Self {
        let device = match (self.device, other.device) {
            (Some((from, _)), Some((to, bounds))) => {
                let (_, a, p) = from.to_scale_rotation_translation();
                let (_, b, q) = to.to_scale_rotation_translation();
                Some((
                    Affine3A::from_rotation_translation(a.slerp(b, t), p.lerp(q, t)),
                    bounds,
                ))
            }
            (_, device) => device,
        };
        Self {
            end: self.end.lerp(other.end, t),
            exit: self.exit.lerp(other.exit, t).normalize_or(other.exit),
            device,
            file: other.file,
        }
    }
}

impl Cable {
    /// Lays the cable out straight from the computer to `end` to fall into
    /// place from.
    pub(super) fn lay(&mut self, end: Vec3) {
        self.length = Self::length_for(self.socket, end);
        self.points = (0..POINTS)
            .map(|index| {
                let t = index as f32 / (POINTS - 1) as f32;
                let point = self.socket.lerp(end, t);
                point.with_y(point.y.max(self.floor_at(point)))
            })
            .collect();
        self.previous = self.points.clone();
        self.ticked = self.points.clone();
    }

    /// Moves the cable on by `dt` seconds in whole ticks. Returns whether it
    /// moved.
    pub(super) fn advance(&mut self, dt: f32, ends: &Ends) -> bool {
        self.behind += dt;
        // Rounding would otherwise leave a whole tick behind now and then.
        let ticks = (self.behind / TICK + 1e-3) as u32;
        self.behind = (self.behind - ticks as f32 * TICK).max(0.0);
        let mut moved = false;
        for _ in 0..ticks.min(MAX_TICKS) {
            moved |= self.tick(ends);
        }
        moved
    }

    /// Moves the cable on by one tick, unless it sleeps and the plug has not
    /// moved. Returns whether it moved.
    pub(super) fn tick(&mut self, ends: &Ends) -> bool {
        let wanted = Self::length_for(self.socket, ends.end);
        // Something the cable lies on or around moving wakes it too.
        let same_solids = self
            .last
            .is_some_and(|last| last.device == ends.device && last.file == ends.file);
        if wanted == self.length
            && same_solids
            && self
                .asleep
                .is_some_and(|end| end.distance(ends.end) < SLEEP)
        {
            self.ticked.clone_from(&self.points);
            self.before_last = self.last;
            return false;
        }
        self.length += (wanted - self.length).clamp(-REEL * TICK, REEL * TICK);
        let before = self.points.clone();
        self.ticked.clone_from(&before);
        let from = self.last.unwrap_or(*ends);
        let mut previous = from;
        for substep in 1..=SUBSTEPS {
            let between = from.towards(ends, substep as f32 / SUBSTEPS as f32);
            self.step(TICK / SUBSTEPS as f32, &between, &previous);
            previous = between;
        }
        self.before_last = self.last;
        self.last = Some(*ends);
        let moved = before
            .iter()
            .zip(&self.points)
            .map(|(a, b)| a.distance(*b))
            .fold(0.0, f32::max);
        self.asleep = (moved < SLEEP).then_some(ends.end);
        true
    }

    /// Where the points show this frame: between the last two ticks, as
    /// far as the time since the last one, with the plug end where the plug
    /// is now. Ticks come at their own pace, so the cable would move in
    /// steps between frames.
    pub(super) fn shown(&self, ends: &Ends) -> Vec<Vec3> {
        let last = POINTS - 1;
        let t = self.behind / TICK;
        let mut shown: Vec<Vec3> = self
            .ticked
            .iter()
            .zip(&self.points)
            .map(|(a, b)| a.lerp(*b, t))
            .collect();
        // The shown points lie where they were with these ends. The plug
        // has moved on since, with the device it is in, and carries the
        // points near it along.
        let then = match (self.before_last, self.last) {
            (Some(before), Some(last)) => before.towards(&last, t),
            (_, Some(last)) => last,
            _ => *ends,
        };
        let smooth = |share: f32| share * share * (3.0 - 2.0 * share);
        match (then.device, ends.device) {
            // Points that lie on or near the device turn along with it, so
            // the cable wrapped round it stays round it.
            (Some((was, bounds)), Some((now, _))) => {
                let back = was.inverse();
                for point in &mut shown[3..last - 2] {
                    let local = back.transform_point3(*point) - bounds.center;
                    let away = (local.abs() - bounds.size / 2.0).max(Vec3::ZERO).length();
                    let share = smooth((1.0 - away / CARRY_REACH).clamp(0.0, 1.0));
                    let carried = now.transform_point3(local + bounds.center);
                    *point = point.lerp(carried, share);
                }
            }
            _ => {
                let pull = ends.end - then.end;
                for k in 1..=CATCH_UP {
                    let share = 1.0 - k as f32 / (CATCH_UP + 1) as f32;
                    shown[last - 2 - k] += pull * smooth(share);
                }
            }
        }
        let spacing = self.length / last as f32;
        for k in 0..3 {
            shown[last - k] = ends.end + ends.exit * k as f32 * spacing;
        }
        // Between ticks, the device has turned on from where the points
        // kept out of it.
        let held = |index: usize| {
            if (3..last - 2).contains(&index) {
                1.0
            } else {
                0.0
            }
        };
        Self::keep_apart(&mut shown, &held);
        let came = shown.clone();
        let solids = self.solids(ends, ends);
        for _ in 0..SHOWN_PASSES {
            self.keep_curve_out(&mut shown, &came, &solids, &held);
        }
        shown
    }

    /// How long the cable wants to be between `socket` and `end`.
    fn length_for(socket: Vec3, end: Vec3) -> f32 {
        socket.distance(end) * SLACK + EXTRA
    }

    /// One small step of `dt` seconds, from the ends `before` to `ends`.
    fn step(&mut self, dt: f32, ends: &Ends, before: &Ends) {
        let last = POINTS - 1;
        let spacing = self.length / last as f32;
        let free = 3..last - 2;
        let keep = (1.0 - DAMPING * dt).max(0.0);
        for index in free.clone() {
            let point = self.points[index];
            let speed = (point - self.previous[index]) * keep;
            self.previous[index] = point;
            self.points[index] = point + speed + Vec3::NEG_Y * GRAVITY * dt * dt;
        }
        let pinned = [
            (0, self.socket),
            (1, self.socket + self.out * spacing),
            (2, self.socket + self.out * 2.0 * spacing),
            (last - 2, ends.end + ends.exit * 2.0 * spacing),
            (last - 1, ends.end + ends.exit * spacing),
            (last, ends.end),
        ];
        for (index, at) in pinned {
            self.previous[index] = self.points[index];
            self.points[index] = at;
        }
        let weight = |index: usize| if free.contains(&index) { 1.0 } else { 0.0 };
        for index in 0..last {
            self.stretch(index, spacing, weight(index), weight(index + 1));
        }
        let bend = BEND_COMPLIANCE / (dt * dt);
        for index in 1..last {
            self.bend(
                index,
                bend,
                [weight(index - 1), weight(index), weight(index + 1)],
            );
        }
        Self::keep_apart(&mut self.points, &weight);
        let solids = self.solids(ends, before);
        for index in free.clone() {
            let mut point = self.keep_out(self.points[index], self.previous[index], &solids);
            point = point.with_y(point.y.max(self.floor_at(point)));
            if point.y <= self.floor_at(point) + 1e-5 {
                let previous = &mut self.previous[index];
                let slide = (point - *previous).with_y(0.0);
                // Static friction holds a point that barely moves, the rest
                // slides on and slows down.
                if slide.length() < STICK * dt {
                    point = previous.with_y(point.y);
                } else {
                    *previous = point - slide * FRICTION;
                }
                previous.y = point.y;
            }
            self.points[index] = point;
        }
        let mut points = std::mem::take(&mut self.points);
        self.keep_curve_out(&mut points, &self.previous, &solids, &weight);
        self.points = points;
    }

    /// Moves `points` so the curve drawn through them keeps out of things,
    /// at each point and between them, since the curve cuts the corners of
    /// the chain. The points were at `came` with the ends `before`. Each
    /// point moves as far as its share of the curve and its `weight` allow,
    /// as in XPBD.
    fn keep_curve_out(
        &self,
        points: &mut [Vec3],
        came: &[Vec3],
        solids: &[Solid],
        weight: &impl Fn(usize) -> f32,
    ) {
        for start in 0..points.len() - 2 {
            for mix in [&KNOT[..], &QUARTER, &MIDDLE, &THREE_QUARTERS] {
                let Some(from) = points.get(start..start + mix.len()) else {
                    continue;
                };
                let shares: f32 = mix
                    .iter()
                    .enumerate()
                    .map(|(k, w)| w * w * weight(start + k))
                    .sum();
                if shares == 0.0 {
                    continue;
                }
                let blend = |points: &[Vec3]| points.iter().zip(mix).map(|(p, w)| *p * *w).sum();
                let on_curve: Vec3 = blend(from);
                let came_from = blend(&came[start..start + mix.len()]);
                let push = self.keep_out(on_curve, came_from, solids) - on_curve;
                if push == Vec3::ZERO {
                    continue;
                }
                for (k, w) in mix.iter().enumerate() {
                    points[start + k] += push * w * weight(start + k) / shares;
                }
            }
        }
    }

    /// Keeps the points `index` and the next one `spacing` apart.
    fn stretch(&mut self, index: usize, spacing: f32, a: f32, b: f32) {
        if a + b == 0.0 {
            return;
        }
        let off = self.points[index + 1] - self.points[index];
        let length = off.length();
        if length < 1e-7 {
            return;
        }
        let fix = off / length * (length - spacing) / (a + b);
        self.points[index] += fix * a;
        self.points[index + 1] -= fix * b;
    }

    /// Keeps pieces of the cable that are not neighbours a cable apart, so
    /// it does not go through itself.
    fn keep_apart(points: &mut [Vec3], weight: &impl Fn(usize) -> f32) {
        let pieces = points.len() - 1;
        for i in 0..pieces {
            for j in i + 2..pieces {
                let [a0, a1, b0, b1] = [i, i + 1, j, j + 1].map(|k| points[k]);
                let reach = (a0.distance(a1) + b0.distance(b1)) / 2.0 + 2.0 * RADIUS;
                if (a0 + a1).distance(b0 + b1) / 2.0 > reach {
                    continue;
                }
                let (s, t) = closest_between(a0, a1, b0, b1);
                let off = b0.lerp(b1, t) - a0.lerp(a1, s);
                let distance = off.length();
                if distance >= 2.0 * RADIUS || distance < 1e-9 {
                    continue;
                }
                let shares = [(i, -(1.0 - s)), (i + 1, -s), (j, 1.0 - t), (j + 1, t)]
                    .map(|(k, share)| (k, share, weight(k)));
                let sum: f32 = shares.iter().map(|(_, share, w)| share * share * w).sum();
                if sum == 0.0 {
                    continue;
                }
                let push = off / distance * (2.0 * RADIUS - distance) / sum;
                for (k, share, w) in shares {
                    points[k] += push * share * w;
                }
            }
        }
    }

    /// Straightens the bend at `index` against its neighbours, as far as
    /// `compliance` lets it.
    fn bend(&mut self, index: usize, compliance: f32, [a, b, c]: [f32; 3]) {
        let curve = self.points[index - 1] - 2.0 * self.points[index] + self.points[index + 1];
        let length = curve.length();
        let weights = a + 4.0 * b + c;
        if length < 1e-7 || weights == 0.0 {
            return;
        }
        let normal = curve / length;
        let lambda = -length / (weights + compliance);
        self.points[index - 1] += normal * lambda * a;
        self.points[index] -= normal * lambda * 2.0 * b;
        self.points[index + 1] += normal * lambda * c;
    }

    /// The boxes the cable keeps out of with the ends `ends`, which were
    /// `before` a moment ago.
    fn solids(&self, ends: &Ends, before: &Ends) -> Vec<Solid> {
        let moving = |now: Option<(Affine3A, Block)>, then: Option<(Affine3A, Block)>| {
            now.map(|(at, bounds)| Solid::new(at, then.map_or(at, |(then, _)| then), bounds))
        };
        moving(ends.device, before.device)
            .into_iter()
            .chain(moving(ends.file, before.file))
            .chain(
                self.solids
                    .iter()
                    .chain([&(Affine3A::IDENTITY, self.mat)])
                    .map(|&(at, bounds)| Solid::new(at, at, bounds)),
            )
            .collect()
    }

    /// `point`, moved out of `solids`. It was at `came_from` a moment ago.
    fn keep_out(&self, point: Vec3, came_from: Vec3, solids: &[Solid]) -> Vec3 {
        solids.iter().fold(point, |point, solid| {
            self.around(point, solid.moved.transform_point3(came_from), solid)
        })
    }

    /// `point`, moved out of `bounds` of a thing at `transform`, rounded by
    /// the radius of the cable, if it is inside. From inside the box itself
    /// it leaves through the side it came in by from `came_from`, or
    /// through the nearest side above the counter if it was inside already.
    fn around(&self, point: Vec3, came_from: Vec3, solid: &Solid) -> Vec3 {
        let (transform, inverse, bounds) = (solid.at, solid.back, &solid.bounds);
        let local = inverse.transform_point3(point) - bounds.center;
        let core = bounds.size / 2.0;
        if (local.abs() - core).max_element() > 0.0 {
            // Outside the box, the cable keeps its radius from the nearest
            // point of it. Round around the edges, so a point slides over
            // them as the box turns.
            let nearest = local.clamp(-core, core);
            let off = local - nearest;
            let distance = off.length();
            if distance >= RADIUS || distance < 1e-9 {
                return point;
            }
            return transform.transform_point3(nearest + off / distance * RADIUS + bounds.center);
        }
        let half = core + RADIUS;
        let room = half - local.abs();
        let through = |axis: usize, side: f32| {
            let mut moved = local;
            moved[axis] = half[axis] * side.signum();
            transform.transform_point3(moved + bounds.center)
        };
        let was = inverse.transform_point3(came_from) - bounds.center;
        let out = was.abs() - half;
        if out.max_element() > 0.0 {
            let axis = out.max_position();
            return through(axis, was[axis]);
        }
        (0..3)
            .flat_map(|axis| [(axis, -1.0), (axis, 1.0)])
            .map(|(axis, side)| (half[axis] - side * local[axis], through(axis, side)))
            .filter(|(_, moved)| moved.y >= self.floor_at(*moved) - 1e-4)
            .min_by(|(a, _), (b, _)| a.total_cmp(b))
            .map_or_else(
                || {
                    let axis = room.min_position();
                    through(axis, local[axis])
                },
                |(_, moved)| moved,
            )
    }

    /// The lowest the middle of the cable gets at `point`: on the mat where
    /// it lies over it, otherwise on the counter. Over the edge of the mat,
    /// it bends round it like round any other box.
    fn floor_at(&self, point: Vec3) -> f32 {
        let (min, max) = (self.mat.min(), self.mat.max());
        let over = point.xz().cmpge(min.xz()).all() && point.xz().cmple(max.xz()).all();
        if over && point.y >= max.y {
            max.y + RADIUS
        } else {
            self.floor
        }
    }
}

/// Where the pieces from `a0` to `a1` and from `b0` to `b1` come closest,
/// as a share of the way along each. After Ericson, "Real-Time Collision
/// Detection" (2004), 5.1.9.
fn closest_between(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> (f32, f32) {
    let (d1, d2, r) = (a1 - a0, b1 - b0, a0 - b0);
    let (a, e, f) = (d1.length_squared(), d2.length_squared(), d2.dot(r));
    if a < 1e-12 && e < 1e-12 {
        return (0.0, 0.0);
    }
    if a < 1e-12 {
        return (0.0, (f / e).clamp(0.0, 1.0));
    }
    let c = d1.dot(r);
    if e < 1e-12 {
        return ((-c / a).clamp(0.0, 1.0), 0.0);
    }
    let b = d1.dot(d2);
    let denom = a * e - b * b;
    let mut s = if denom > 1e-12 {
        ((b * f - c * e) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut t = (b * s + f) / e;
    if t < 0.0 {
        t = 0.0;
        s = (-c / a).clamp(0.0, 1.0);
    } else if t > 1.0 {
        t = 1.0;
        s = ((b - c) / a).clamp(0.0, 1.0);
    }
    (s, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cable(end: Vec3) -> Cable {
        let mut cable = Cable {
            socket: Vec3::new(0.0, 1.05, 0.0),
            out: Vec3::NEG_X,
            floor: 1.0,
            mat: Block::new(Vec3::new(0.2, 0.004, 0.2)).translated(Vec3::new(-0.3, 1.0, 0.0)),
            solids: Vec::new(),
            points: Vec::new(),
            previous: Vec::new(),
            ticked: Vec::new(),
            length: 0.0,
            asleep: None,
            behind: 0.0,
            last: None,
            before_last: None,
        };
        cable.lay(end);
        cable
    }

    fn settle(cable: &mut Cable, ends: &Ends) {
        for _ in 0..600 {
            cable.tick(ends);
        }
    }

    #[test]
    fn the_cable_comes_to_rest_on_the_counter() {
        let ends = Ends {
            end: Vec3::new(-0.4, 1.01, 0.1),
            exit: Vec3::X,
            device: None,
            file: None,
        };
        let mut cable = cable(ends.end);
        settle(&mut cable, &ends);
        assert!(cable.asleep.is_some());
        assert_eq!(cable.points[0], cable.socket);
        assert_eq!(cable.points[POINTS - 1], ends.end);
        for point in &cable.points {
            assert!(point.y >= cable.floor_at(*point) - 1e-4, "{point}");
        }
    }

    #[test]
    fn the_cable_keeps_time_at_any_frame_rate_and_stops_while_paused() {
        let ends = Ends {
            end: Vec3::new(-0.4, 1.01, 0.1),
            exit: Vec3::X,
            device: None,
            file: None,
        };
        let mut slow = cable(Vec3::new(-0.2, 1.2, 0.1));
        let mut fast = cable(Vec3::new(-0.2, 1.2, 0.1));
        for _ in 0..30 {
            slow.advance(1.0 / 30.0, &ends);
        }
        for _ in 0..144 {
            fast.advance(1.0 / 144.0, &ends);
        }
        for (a, b) in slow.points.iter().zip(&fast.points) {
            assert!(a.distance(*b) < 1e-3, "{a} {b}");
        }
        let paused = slow.points.clone();
        assert!(!slow.advance(0.0, &ends));
        assert_eq!(slow.points, paused);
    }

    #[test]
    fn the_cable_stays_out_of_the_device_in_any_turn() {
        let bounds = Block::new(Vec3::new(0.06, 0.02, 0.1));
        for step in 0..8 {
            let turn = Quat::from_euler(
                EulerRot::XYZ,
                step as f32 * 0.7,
                step as f32 * 1.3,
                step as f32 * 0.4,
            );
            let transform = Affine3A::from_rotation_translation(turn, Vec3::new(-0.25, 1.25, 0.15));
            let exit = turn * Vec3::NEG_Y;
            let end = transform.transform_point3(bounds.center.with_y(0.0)) + exit * 0.016;
            let ends = Ends {
                end,
                exit,
                device: Some((transform, bounds)),
                file: None,
            };
            let mut cable = cable(end);
            settle(&mut cable, &ends);
            for point in &cable.points[2..POINTS - 2] {
                let local = transform.inverse().transform_point3(*point) - bounds.center;
                let inside = (local.abs() - bounds.size / 2.0).max_element() < 0.0;
                assert!(!inside, "turn {step}: {point}");
            }
        }
    }

    /// Whether a point or the middle of a piece of `cable` is inside `bounds`
    /// at `transform`.
    fn inside(cable: &Cable, transform: Affine3A, bounds: &Block) -> Option<Vec3> {
        let middles = cable
            .points
            .windows(2)
            .map(|pair| (pair[0] + pair[1]) / 2.0);
        cable.points[3..POINTS - 3]
            .iter()
            .copied()
            .chain(middles.skip(3).take(POINTS - 7))
            .find(|point| {
                let local = transform.inverse().transform_point3(*point) - bounds.center;
                (local.abs() - bounds.size / 2.0).max_element() < 0.0
            })
    }

    #[test]
    fn plugging_in_keeps_the_cable_out_of_the_device_at_any_yaw() {
        let bounds = Block::new(Vec3::new(0.06, 0.02, 0.1));
        let rest = Ends {
            end: Vec3::new(-0.4, 1.01, 0.1),
            exit: Vec3::X,
            device: None,
            file: None,
        };
        for step in 0..12 {
            let yaw = Quat::from_rotation_y(step as f32 * 0.55);
            let transform = Affine3A::from_rotation_translation(yaw, Vec3::new(-0.25, 1.014, 0.05));
            let end = transform.transform_point3(Vec3::new(0.0, 0.01 + 0.008, 0.02));
            let plugged = Ends {
                end,
                exit: Vec3::Y,
                device: Some((transform, bounds)),
                file: None,
            };
            let mut cable = cable(rest.end);
            settle(&mut cable, &rest);
            for tick in 0..600 {
                cable.tick(&plugged);
                if let Some(point) = inside(&cable, transform, &bounds) {
                    panic!("yaw {step}, tick {tick}: {point}");
                }
            }
        }
    }

    #[test]
    fn turning_the_device_fast_keeps_the_cable_out_of_it() {
        let bounds = Block::new(Vec3::new(0.06, 0.02, 0.1));
        let center = Vec3::new(-0.25, 1.25, 0.15);
        let at = |angle: f32| {
            let turn = Quat::from_euler(EulerRot::XYZ, angle, angle * 1.7, angle * 0.6);
            let transform = Affine3A::from_rotation_translation(turn, center);
            let exit = turn * Vec3::NEG_Y;
            Ends {
                end: transform.transform_point3(bounds.center.with_y(-0.01)) + exit * 0.008,
                exit,
                device: Some((transform, bounds)),
                file: None,
            }
        };
        let mut cable = cable(at(0.0).end);
        settle(&mut cable, &at(0.0));
        for tick in 1..240 {
            let ends = at(tick as f32 * 0.15);
            cable.tick(&ends);
            let (transform, _) = ends.device.unwrap();
            if let Some(point) = inside(&cable, transform, &bounds) {
                panic!("tick {tick}: {point}");
            }
        }
    }

    #[test]
    fn a_resting_cable_moves_out_of_a_file_opened_under_it() {
        let mut ends = Ends {
            end: Vec3::new(-0.4, 1.01, 0.1),
            exit: Vec3::X,
            device: None,
            file: None,
        };
        let mut cable = cable(ends.end);
        settle(&mut cable, &ends);
        assert!(cable.asleep.is_some());
        let file = Block::new(Vec3::new(0.2, 0.012, 0.1));
        let transform = Affine3A::from_translation(Vec3::new(-0.2, 1.0, 0.05));
        ends.file = Some((transform, file));
        settle(&mut cable, &ends);
        assert_eq!(inside(&cable, transform, &file), None);
    }

    #[test]
    fn a_point_inside_moves_out_the_nearest_side() {
        let bounds = Block::new(Vec3::new(0.1, 0.02, 0.1));
        let transform = Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let inside = Vec3::new(1.0, 2.018, 3.0);
        let cable = cable(Vec3::ZERO);
        let moved = cable.around(inside, inside, &Solid::new(transform, transform, bounds));
        assert!(moved.y >= 2.02 + RADIUS - 1e-6, "{moved}");
        let outside = Vec3::new(1.2, 2.0, 3.0);
        assert_eq!(
            cable.around(outside, outside, &Solid::new(transform, transform, bounds)),
            outside
        );
    }

    #[test]
    fn the_cable_shows_between_ticks_with_its_end_on_the_plug() {
        let bounds = Block::new(Vec3::new(0.06, 0.02, 0.1));
        let at = |t: f32| {
            let turn = Quat::from_rotation_y(1.5 * t) * Quat::from_rotation_x(2.5);
            let transform = Affine3A::from_rotation_translation(turn, Vec3::new(-0.25, 1.2, 0.15));
            let exit = turn * Vec3::NEG_Y;
            Ends {
                end: transform.transform_point3(Vec3::ZERO) + exit * 0.016,
                exit,
                device: Some((transform, bounds)),
                file: None,
            }
        };
        let mut cable = cable(Vec3::new(-0.4, 1.01, 0.1));
        settle(&mut cable, &at(0.0));
        let dt = 1.0 / 144.0;
        let (shown, ticked): (Vec<_>, Vec<_>) = (0..288)
            .map(|frame| {
                let ends = at(frame as f32 * dt);
                cable.advance(dt, &ends);
                let shown = cable.shown(&ends);
                assert_eq!(shown[POINTS - 1], ends.end);
                (shown, cable.points.clone())
            })
            .unzip();
        let jolt = |frames: &[Vec<Vec3>]| {
            frames
                .windows(3)
                .skip(100)
                .flat_map(|f| {
                    (0..POINTS).map(move |i| (f[0][i] - 2.0 * f[1][i] + f[2][i]).length())
                })
                .fold(0.0, f32::max)
        };
        let (smooth, stepped) = (jolt(&shown), jolt(&ticked));
        assert!(smooth < stepped / 2.0, "{smooth} {stepped}");
    }

    #[test]
    fn closest_points_of_two_pieces() {
        let (s, t) = closest_between(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, -1.0),
            Vec3::new(0.5, 1.0, 1.0),
        );
        assert!((s - 0.75).abs() < 1e-6 && (t - 0.5).abs() < 1e-6, "{s} {t}");
        let (s, t) = closest_between(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(2.0, 1.0, 0.0),
            Vec3::new(3.0, 1.0, 0.0),
        );
        assert_eq!((s, t), (1.0, 0.0));
    }

    #[test]
    fn the_cable_slides_round_the_edges_of_a_box() {
        let cable = cable(Vec3::new(-0.4, 1.01, 0.1));
        let bounds = Block::new(Vec3::new(0.06, 0.02, 0.1));
        let transform = Affine3A::from_translation(Vec3::new(-0.25, 1.2, 0.15));
        let edge = bounds.center + Vec3::new(0.03, 0.01, 0.0);
        let near = edge + Vec3::new(1.0, 1.0, 0.0).normalize() * RADIUS / 2.0;
        let world = transform.transform_point3(near);
        let moved = transform.inverse().transform_point3(cable.around(
            world,
            world,
            &Solid::new(transform, transform, bounds),
        ));
        assert!((moved.distance(edge) - RADIUS).abs() < 1e-6, "{moved}");
        assert!((moved - edge).normalize().dot(near - edge) > 0.0);
        let far = edge + Vec3::X * RADIUS * 2.0;
        let world = transform.transform_point3(far);
        assert_eq!(
            cable.around(world, world, &Solid::new(transform, transform, bounds)),
            world
        );
    }
}
