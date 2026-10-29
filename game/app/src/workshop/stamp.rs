//! The rubber stamp, for finishing a job.

use bevy::{
    ecs::system::SystemParam,
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_game_core::audio::{Play, Sound};
use ld60_game_core::player::focus::{Clickable, Focus, Prompting};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use super::delivery::{Flight, HandIn, JobItem};
use super::job::Job;
use super::printer::Printing;
use super::ticket::JobTicket;
use ld60_game_device::Casing;

pub(super) const STAMP_BASE_SIZE: Vec3 = Vec3::new(0.06, 0.018, 0.04);
const STAMP_PAD_SIZE: Vec3 = Vec3::new(0.058, 0.005, 0.038);
const STAMP_NECK_SIZE: Vec3 = Vec3::new(0.012, 0.03, 0.012);
const STAMP_KNOB_SIZE: Vec3 = Vec3::new(0.03, 0.024, 0.03);
const STAMP_WOOD: Color = Color::srgb_u8(0x9a, 0x6e, 0x48);
const STAMP_RUBBER: Color = Color::srgb_u8(0x5a, 0x1c, 0x18);
const STAMP_HANDLE: Color = Color::srgb_u8(0xb0, 0x2e, 0x24);

/// Seconds the stamp takes to the ticket and back.
const STAMP_TIME: f32 = 2.6;
/// Shares of [`STAMP_TIME`]. The stamp is over the ticket at the first,
/// hits it at the second, holds until the third and is back over it at the
/// fourth.
const OVER: f32 = 0.32;
const DOWN: f32 = 0.42;
const HOLD: f32 = 0.55;
const UP: f32 = 0.65;
/// How high the stamp waits over the ticket before it comes down.
const HOVER: f32 = 0.04;
/// How high the stamp lifts before it moves over, clear of the till next
/// to it.
const HOP: f32 = 0.13;
/// Shares of a hop spent going up and coming down.
const HOP_UP: f32 = 0.25;
/// The ticket lies this thick on the counter.
const TICKET: f32 = 0.001;

pub(crate) struct StampPlugin;

impl Plugin for StampPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Stamping::animate, RubberStamp::arm))
            .add_observer(RubberStamp::on_add)
            .add_observer(RubberStamp::press);
    }
}

/// The rubber stamp on the counter. Stamping the ticket hands the job in.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct RubberStamp;

/// What the parts of a job say about stamping its ticket now.
#[derive(SystemParam)]
struct Ready<'w, 's> {
    casings: Query<'w, 's, &'static Casing, With<JobItem>>,
    printing: Query<'w, 's, (), With<Printing>>,
    flights: Query<'w, 's, (), With<Flight>>,
}

impl Ready<'_, '_> {
    /// Nobody takes a gadget home in pieces.
    fn sealed(&self) -> bool {
        self.casings.iter().all(Casing::sealed)
    }

    /// The stamp only stamps once the device is closed up, the new ticket
    /// is on the clipboard and nothing is on its way.
    fn can_stamp(&self, stamping: bool) -> bool {
        self.sealed() && !stamping && self.flights.is_empty() && self.printing.is_empty()
    }
}

impl RubberStamp {
    /// Sends the stamp onto the ticket, see [`Stamping`].
    fn press(
        mut press: On<PointerPress>,
        state: Res<State<Focus>>,
        stamps: Query<(&Transform, Has<Stamping>), With<Self>>,
        ready: Ready,
        tickets: Query<&GlobalTransform, With<JobTicket>>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || *state.get() != Focus::Desk {
            return;
        }
        let Ok((&rest, stamping)) = stamps.get(press.entity) else {
            return;
        };
        press.propagate(false);
        let Ok(ticket) = tickets.single() else {
            return;
        };
        if !ready.can_stamp(stamping) {
            commands.trigger(Play(Sound::Refuse));
            return;
        }
        commands.entity(press.entity).insert(Stamping {
            elapsed: 0.0,
            rest,
            onto: ticket
                .transform_point(JobTicket::stamp_spot())
                .with_y(rest.translation.y + TICKET),
        });
    }

    fn arm(
        ready: Ready,
        stamps: Query<(Entity, Prompting, Has<Stamping>), With<Self>>,
        mut commands: Commands,
    ) {
        for (stamp, now, stamping) in &stamps {
            let wanted = Some(("click-stamp", ready.can_stamp(stamping)));
            Clickable::show(&mut commands, stamp, now, wanted);
        }
    }

    fn on_add(add: On<Add<Self>>, mut commands: Commands) {
        let pad = Block::new(STAMP_PAD_SIZE);
        let base = Block::new(STAMP_BASE_SIZE).on_top_of(&pad);
        let neck = Block::new(STAMP_NECK_SIZE).on_top_of(&base);
        let knob = Block::new(STAMP_KNOB_SIZE).on_top_of(&neck);
        commands.entity(add.entity).with_children(|stamp| {
            stamp.spawn((Shape::cuboid(&pad, STAMP_RUBBER), Finish::Plastic));
            stamp.spawn((Shape::cuboid(&base, STAMP_WOOD), Finish::Wood));
            stamp.spawn((Shape::cylinder(&neck, STAMP_WOOD), Finish::Wood));
            stamp.spawn((Shape::cylinder(&knob, STAMP_HANDLE), Finish::Plastic));
        });
    }
}

/// The stamp hopping onto the job ticket, stamping it and hopping back.
/// Stamping hands the job in.
#[derive(Component, Debug)]
struct Stamping {
    elapsed: f32,
    rest: Transform,
    /// Where the stamp stands on the ticket.
    onto: Vec3,
}

impl Stamping {
    /// Where the stamp is `t` of the way, from 0 to 1.
    fn at(&self, t: f32) -> Vec3 {
        let home = self.rest.translation;
        let above = self.onto + Vec3::Y * HOVER;
        // Up, over at the height of the higher end and down.
        let hop = |from: Vec3, to: Vec3, t: f32| {
            let high = from.y.max(to.y) + HOP;
            let up = from.with_y(high);
            let down = to.with_y(high);
            if t < HOP_UP {
                from.lerp(up, smooth(t / HOP_UP))
            } else if t < 1.0 - HOP_UP {
                up.lerp(down, smooth((t - HOP_UP) / (1.0 - 2.0 * HOP_UP)))
            } else {
                down.lerp(to, smooth((t - 1.0 + HOP_UP) / HOP_UP))
            }
        };
        if t < OVER {
            hop(home, above, t / OVER)
        } else if t < DOWN {
            // Speeds up as it comes down, like a hand pressing.
            above.lerp(self.onto, ((t - OVER) / (DOWN - OVER)).powi(2))
        } else if t < HOLD {
            self.onto
        } else if t < UP {
            self.onto.lerp(above, smooth((t - HOLD) / (UP - HOLD)))
        } else {
            hop(above, home, (t - UP) / (1.0 - UP))
        }
    }

    fn animate(
        time: Res<Time>,
        ready: Ready,
        mut job: Option<ResMut<Job>>,
        mut stamps: Query<(Entity, &mut Stamping, &mut Transform)>,
        mut commands: Commands,
    ) {
        for (stamp, mut stamping, mut transform) in &mut stamps {
            let before = stamping.elapsed / STAMP_TIME;
            stamping.elapsed += time.delta_secs();
            let mut t = (stamping.elapsed / STAMP_TIME).min(1.0);
            // The device may have been opened since the press. Then the
            // stamp goes back without stamping.
            if before < OVER && t >= OVER && !ready.sealed() {
                commands.trigger(Play(Sound::Refuse));
                stamping.elapsed = UP * STAMP_TIME;
                t = UP;
            }
            transform.translation = stamping.at(t);
            if before < DOWN && (DOWN..UP).contains(&t) {
                if let Some(job) = job.as_mut() {
                    job.stamped = true;
                }
                commands.trigger(Play(Sound::Thunk));
                commands.trigger(HandIn);
            }
            if t >= 1.0 {
                *transform = stamping.rest;
                commands.entity(stamp).remove::<Stamping>();
            }
        }
    }
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}
