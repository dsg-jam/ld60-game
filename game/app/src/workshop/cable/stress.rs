use std::f32::consts::PI;

use bevy::{math::Affine3A, prelude::*};
use ld60_devices::DeviceKind;
use ld60_game_core::layout::counter::CounterLayout;
use ld60_game_device::Handheld;
use ld60_game_manuals::ShopManuals;
use ld60_geometry::block::Block;

use super::solver::Ends;
use super::{Cable, POINTS, Plug, RADIUS};
use crate::workshop::footprint::counter_props;
use ld60_game_core::layout::desk::MANUALS_OPEN_SPOT;

use crate::workshop::delivery;

/// How far a held device lifts off the counter, as in the device crate.
const LIFT: f32 = 0.1;
/// Points at each end that the boot and the plug place.
const HELD_ENDS: usize = 3;
/// Samples of the drawn curve per piece of the cable.
const SAMPLES: usize = 8;
/// Frames per second of a run.
const FPS: f32 = 144.0;

/// The deepest the cable goes into something, and into what.
#[derive(Debug, Default)]
struct Worst {
    depth: f32,
    what: &'static str,
    tick: usize,
    /// Which piece of the cable, counted from the computer.
    piece: f32,
}

struct Room {
    solids: Vec<(&'static str, Affine3A, Block)>,
}

impl Room {
    fn new(counter: &CounterLayout) -> Self {
        let mut solids: Vec<_> = counter_props(counter)
            .iter()
            .filter(|prop| !matches!(prop.name(), "device" | "debug cable plug"))
            .map(|prop| {
                let (at, block) = prop.solid();
                (prop.name(), at, block)
            })
            .collect();
        solids.push(("counter top", Affine3A::IDENTITY, counter.top));
        solids.push(("mat", Affine3A::IDENTITY, counter.mat));
        Self { solids }
    }

    /// How deep the cable around `point` goes into the deepest solid, and
    /// the device and file of `ends`.
    fn depth(&self, point: Vec3, ends: &Ends) -> (f32, &'static str) {
        let moving = [("device", ends.device), ("open file", ends.file)];
        self.solids
            .iter()
            .map(|&(name, at, block)| (name, at, block))
            .chain(
                moving
                    .into_iter()
                    .filter_map(|(name, solid)| solid.map(|(at, block)| (name, at, block))),
            )
            .map(|(name, at, block)| {
                let local = at.inverse().transform_point3(point) - block.center;
                let q = local.abs() - block.size / 2.0;
                let distance = q.max(Vec3::ZERO).length() + q.max_element().min(0.0);
                (RADIUS - distance, name)
            })
            .fold(
                (0.0, ""),
                |worst, this| if this.0 > worst.0 { this } else { worst },
            )
    }
}

/// The drawn middle of the cable, as the shader puts it on a B-spline.
fn curve(points: &[Vec3]) -> Vec<Vec3> {
    let point = |k: isize| -> Vec3 {
        let last = points.len() as isize - 1;
        if k < 0 {
            2.0 * points[0] - points[1]
        } else if k > last {
            2.0 * points[last as usize] - points[last as usize - 1]
        } else {
            points[k as usize]
        }
    };
    (HELD_ENDS..points.len() - 1 - HELD_ENDS)
        .flat_map(|i| (0..SAMPLES).map(move |s| (i, s as f32 / SAMPLES as f32)))
        .map(|(i, t)| {
            let i = i as isize;
            let (p0, p1, p2, p3) = (point(i - 1), point(i), point(i + 1), point(i + 2));
            let s = 1.0 - t;
            (s * s * s * p0
                + (3.0 * t * t * t - 6.0 * t * t + 4.0) * p1
                + (-3.0 * t * t * t + 3.0 * t * t + 3.0 * t + 1.0) * p2
                + t * t * t * p3)
                / 6.0
        })
        .collect()
}

/// How deep the drawn cable goes into itself: twice its radius less the
/// distance of the closest two samples that are not near each other along
/// it.
fn crossing(points: &[Vec3]) -> (f32, f32) {
    let samples = curve(points);
    let apart = 4 * SAMPLES;
    let mut worst = (0.0, 0.0);
    for i in 0..samples.len() {
        for j in i + apart..samples.len() {
            let depth = 2.0 * RADIUS - samples[i].distance(samples[j]);
            if depth > worst.0 {
                worst = (depth, HELD_ENDS as f32 + i as f32 / SAMPLES as f32);
            }
        }
    }
    worst
}

/// Runs the cable for `seconds` with the ends at each time, frame by frame
/// as the game does, and finds where it goes deepest: by the points the
/// solver keeps, by the curve it draws, and into itself.
fn run(room: &Room, cable: &mut Cable, seconds: f32, ends: impl Fn(f32) -> Ends) -> [Worst; 3] {
    let mut worst = [Worst::default(), Worst::default(), Worst::default()];
    for frame in 0..(seconds * FPS) as usize {
        let ends = ends(frame as f32 / FPS);
        cable.advance(1.0 / FPS, &ends);
        // The solved points are where the last tick left them, with the
        // ends of that tick.
        let ticked = cable.last.unwrap_or(ends);
        let solved = cable.points[HELD_ENDS..POINTS - HELD_ENDS].to_vec();
        let shown = cable.shown(&ends);
        let drawn = curve(&shown);
        for (slot, (samples, ends)) in [(solved, ticked), (drawn, ends)].iter().enumerate() {
            let per_piece = if slot == 0 { 1.0 } else { SAMPLES as f32 };
            for (index, &point) in samples.iter().enumerate() {
                let (depth, what) = room.depth(point, ends);
                if depth > worst[slot].depth {
                    worst[slot] = Worst {
                        depth,
                        what,
                        tick: frame,
                        piece: HELD_ENDS as f32 + index as f32 / per_piece,
                    };
                }
            }
        }
        let (depth, piece) = crossing(&shown);
        if depth > worst[2].depth {
            worst[2] = Worst {
                depth,
                what: "itself",
                tick: frame,
                piece,
            };
        }
    }
    worst
}

fn device(kind: DeviceKind) -> Block {
    Block::new(Handheld::body_size(kind))
}

/// The device turned by `turn` where the player holds it, plugged in: its
/// middle `LIFT` over where it rests.
fn held(counter: &CounterLayout, kind: DeviceKind, turn: Quat) -> Ends {
    let bounds = device(kind);
    let rest = counter.place(delivery::DEVICE_SPOT, delivery::DEVICE_YAW);
    let middle = rest.transform_point(bounds.center) + Vec3::Y * LIFT;
    let at = Affine3A::from_rotation_translation(turn, middle - turn * bounds.center);
    let exit = turn * Vec3::NEG_Y;
    Ends {
        end: at.transform_point3(Vec3::new(0.0, 0.0, 0.01)) + exit * 0.008,
        exit,
        device: Some((at, bounds)),
        file: None,
    }
}

/// The device lying open on the mat, board up, turned by `yaw`, plugged in.
fn on_mat(counter: &CounterLayout, kind: DeviceKind, yaw: f32) -> Ends {
    let bounds = device(kind);
    let rest = counter.place(delivery::DEVICE_SPOT, delivery::DEVICE_YAW + yaw);
    let turn = rest.rotation * Quat::from_rotation_z(PI);
    let at = Affine3A::from_rotation_translation(turn, rest.translation + Vec3::Y * bounds.size.y);
    Ends {
        end: at.transform_point3(Vec3::new(0.0, 0.0, 0.01)) + Vec3::Y * 0.008,
        exit: Vec3::Y,
        device: Some((at, bounds)),
        file: None,
    }
}

fn resting(counter: &CounterLayout) -> Ends {
    let rest = Plug::rest_at(counter);
    let exit = rest.rotation * Vec3::Z;
    Ends {
        end: rest.translation + exit * (super::PLUG.z / 2.0),
        exit,
        device: None,
        file: None,
    }
}

/// Asserts that the cable, solved and drawn, went no deeper than `limit`
/// meters into anything or into itself.
fn assert_within(name: &str, [solved, drawn, crossed]: &[Worst; 3], limit: f32) {
    for (kind, worst) in [("solved", solved), ("drawn", drawn), ("crossing", crossed)] {
        assert!(
            worst.depth <= limit,
            "{name}: {kind} cable {:.2} mm into {} at frame {}, piece {:.1}",
            worst.depth * 1000.0,
            worst.what,
            worst.tick,
            worst.piece,
        );
    }
}

#[test]
fn a_still_cable_stays_out_of_everything() {
    let counter = CounterLayout::new();
    let room = Room::new(&counter);
    let rest = resting(&counter);
    let mut cable = Cable::new(&counter, Plug::rest_at(&counter));
    // It is laid out straight, in the air, and falls first.
    assert_within("falling", &run(&room, &mut cable, 2.0, |_| rest), 0.75e-3);
    assert_within("rest", &run(&room, &mut cable, 3.0, |_| rest), 0.15e-3);
    let open = (
        counter
            .place_on_mat(MANUALS_OPEN_SPOT, 0.0)
            .compute_affine(),
        Block::new(ShopManuals::OPEN),
    );
    let file = Ends {
        file: Some(open),
        ..rest
    };
    assert_within("file", &run(&room, &mut cable, 3.0, |_| file), 0.15e-3);
    for kind in DeviceKind::ALL {
        for yaw in [0.0, 1.2, 2.4] {
            let mut cable = Cable::new(&counter, Plug::rest_at(&counter));
            run(&room, &mut cable, 3.0, |_| rest);
            let ends = on_mat(&counter, kind, yaw);
            let name = format!("{kind:?} on the mat at yaw {yaw}");
            // The plug jumps onto the device, and the cable follows.
            assert_within(&name, &run(&room, &mut cable, 2.0, |_| ends), 1e-3);
            assert_within(&name, &run(&room, &mut cable, 3.0, |_| ends), 0.15e-3);
        }
    }
}

#[test]
fn turning_the_held_device_keeps_the_cable_out_of_it() {
    let counter = CounterLayout::new();
    let room = Room::new(&counter);
    for kind in DeviceKind::ALL {
        for (speed, limit) in [(1.0, 1e-3), (3.0, 1e-3), (8.0, 1.5e-3)] {
            let mut cable = Cable::new(&counter, Plug::rest_at(&counter));
            let start = held(&counter, kind, Quat::IDENTITY);
            run(&room, &mut cable, 3.0, |_| start);
            let worst = run(&room, &mut cable, 6.0, |time| {
                let a = time * speed;
                held(
                    &counter,
                    kind,
                    Quat::from_euler(EulerRot::XYZ, a, a * 1.7, a * 0.6),
                )
            });
            assert_within(
                &format!("{kind:?} turning {speed} per second"),
                &worst,
                limit,
            );
        }
    }
}

#[test]
fn turning_the_held_device_does_not_whip_the_cable() {
    let counter = CounterLayout::new();
    for kind in [DeviceKind::Tally2, DeviceKind::Lock4] {
        for (speed, worst_limit, most_limit) in [(1.0, 8e-3, 0.5e-3), (3.0, 20e-3, 2.5e-3)] {
            let mut cable = Cable::new(&counter, Plug::rest_at(&counter));
            let start = held(&counter, kind, Quat::IDENTITY);
            for _ in 0..(3.0 * FPS) as usize {
                cable.advance(1.0 / FPS, &start);
            }
            let frames: Vec<Vec<Vec3>> = (0..(6.0 * FPS) as usize)
                .map(|frame| {
                    let a = frame as f32 / FPS * speed;
                    let turn = Quat::from_euler(EulerRot::XYZ, a, a * 1.7, a * 0.6);
                    let ends = held(&counter, kind, turn);
                    cable.advance(1.0 / FPS, &ends);
                    cable.shown(&ends)
                })
                .collect();
            // How much each point speeds up or slows down from one frame
            // to the next.
            let mut jolts: Vec<f32> = frames
                .windows(3)
                .flat_map(|f| {
                    (HELD_ENDS..POINTS - HELD_ENDS)
                        .map(move |i| (f[0][i] - 2.0 * f[1][i] + f[2][i]).length())
                })
                .collect();
            jolts.sort_by(f32::total_cmp);
            let worst = jolts[jolts.len() - 1];
            let most = jolts[jolts.len() * 99 / 100];
            let name = format!("{kind:?} turning {speed} per second");
            assert!(
                worst < worst_limit,
                "{name}: a jolt of {:.1} mm",
                worst * 1000.0
            );
            assert!(
                most < most_limit,
                "{name}: 1% of jolts over {:.2} mm",
                most * 1000.0
            );
        }
    }
}
