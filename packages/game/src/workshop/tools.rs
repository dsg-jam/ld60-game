//! The rubber stamp and the service bell, for finishing a job.

use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_core::audio::{Play, Sound};
use ld60_core::player::focus::{Clickable, Focus};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use super::job::{Flight, HandIn, Job, JobItem};
use super::printer::Printing;
use super::ticket::JobTicket;
use crate::device::Casing;

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

pub(super) const BELL_BASE_SIZE: Vec3 = Vec3::new(0.07, 0.012, 0.07);
const BELL_DOME_RADIUS: f32 = 0.032;
/// The dome is a flattened sphere.
const BELL_DOME_FLATTEN: f32 = 0.75;
const BELL_PLUNGER_SIZE: Vec3 = Vec3::new(0.008, 0.012, 0.008);
const BELL_BASE: Color = Color::srgb_u8(0x22, 0x20, 0x1e);
const BRASS: Color = Color::srgb_u8(0xc8, 0x9a, 0x4a);

pub(crate) struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Stamping::animate, Tool::arm))
            .add_observer(Tool::on_add)
            .add_observer(Tool::use_on_press);
    }
}

/// A tool on the counter that finishes a job.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(Transform, Visibility)]
pub(crate) enum Tool {
    /// Stamps the ticket of the job when the bell rings.
    Stamp,
    /// Calls the customer for the job. The stamp stamps its ticket first.
    Bell,
}

impl Tool {
    /// Ringing the bell sends the stamp onto the ticket, see [`Stamping`].
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn use_on_press(
        mut press: On<PointerPress>,
        state: Res<State<Focus>>,
        tools: Query<(Entity, &Tool, &Transform, Has<Stamping>)>,
        casings: Query<&Casing, With<JobItem>>,
        printing: Query<(&Printing, Has<JobTicket>)>,
        flights: Query<(), With<Flight>>,
        job: Option<Res<Job>>,
        tickets: Query<&GlobalTransform, With<JobTicket>>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || *state.get() != Focus::Desk {
            return;
        }
        if tools.get(press.entity).map(|(_, tool, ..)| *tool) != Ok(Self::Bell) {
            return;
        }
        press.propagate(false);
        commands.trigger(Play(Sound::Ding));
        let stamping = tools.iter().any(|(.., stamping)| stamping);
        if Self::bell(&casings, &printing, !flights.is_empty() || stamping) != "click-bell" {
            commands.trigger(Play(Sound::Refuse));
            return;
        }
        let Some(job) = job else {
            return;
        };
        if job.stamped {
            commands.trigger(HandIn);
            return;
        }
        let stamp = tools.iter().find(|(_, tool, ..)| **tool == Self::Stamp);
        if let (Some((stamp, _, &rest, _)), Ok(ticket)) = (stamp, tickets.single()) {
            commands.entity(stamp).insert(Stamping {
                elapsed: 0.0,
                rest,
                onto: ticket
                    .transform_point(JobTicket::stamp_spot())
                    .with_y(rest.translation.y + TICKET),
            });
        }
    }

    /// What the bell says a press does. It only calls the customer once the
    /// device is closed up, the new ticket is on the clipboard and nothing
    /// is on its way.
    fn bell(
        casings: &Query<&Casing, With<JobItem>>,
        printing: &Query<(&Printing, Has<JobTicket>)>,
        busy: bool,
    ) -> &'static str {
        // Nobody takes a gadget home in pieces.
        if !casings.iter().all(Casing::sealed) {
            "click-bell-open"
        } else if printing
            .iter()
            .any(|(printing, ticket)| ticket && printing.waiting())
        {
            "click-bell-ticket"
        } else if busy || !printing.is_empty() {
            "click-bell-wait"
        } else {
            "click-bell"
        }
    }

    fn arm(
        casings: Query<&Casing, With<JobItem>>,
        printing: Query<(&Printing, Has<JobTicket>)>,
        flights: Query<(), With<Flight>>,
        stamps: Query<(), With<Stamping>>,
        mut bells: Query<&mut Clickable, With<Tool>>,
    ) {
        let busy = !flights.is_empty() || !stamps.is_empty();
        let prompt = Self::bell(&casings, &printing, busy);
        for mut clickable in &mut bells {
            clickable.set_if_neq(Clickable { prompt });
        }
    }

    fn on_add(
        add: On<Add<Tool>>,
        tools: Query<&Tool>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Ok(&tool) = tools.get(add.entity) else {
            return;
        };
        let mut entity = commands.entity(add.entity);
        if tool == Self::Bell {
            entity.insert(Clickable {
                prompt: "click-bell",
            });
        }
        match tool {
            Self::Stamp => {
                let pad = Block::new(STAMP_PAD_SIZE);
                let base = Block::new(STAMP_BASE_SIZE).on_top_of(&pad);
                let neck = Block::new(STAMP_NECK_SIZE).on_top_of(&base);
                let knob = Block::new(STAMP_KNOB_SIZE).on_top_of(&neck);
                entity.with_children(|stamp| {
                    stamp.spawn((Shape::cuboid(&pad, STAMP_RUBBER), Finish::Plastic));
                    stamp.spawn((Shape::cuboid(&base, STAMP_WOOD), Finish::Wood));
                    stamp.spawn((Shape::cylinder(&neck, STAMP_WOOD), Finish::Wood));
                    stamp.spawn((Shape::cylinder(&knob, STAMP_HANDLE), Finish::Plastic));
                });
            }
            Self::Bell => {
                let base = Block::new(BELL_BASE_SIZE);
                let dome_height = BELL_DOME_RADIUS * BELL_DOME_FLATTEN;
                let plunger = Block::new(BELL_PLUNGER_SIZE)
                    .translated(Vec3::Y * (base.max().y + dome_height - 0.002));
                let brass = materials.add(StandardMaterial {
                    base_color: BRASS,
                    metallic: 1.0,
                    perceptual_roughness: 0.3,
                    ..default()
                });
                entity.with_children(|bell| {
                    bell.spawn((Shape::cylinder(&base, BELL_BASE), Finish::Wood));
                    bell.spawn((
                        Mesh3d(meshes.add(Sphere::new(BELL_DOME_RADIUS))),
                        MeshMaterial3d(brass.clone()),
                        Transform::from_translation(Vec3::Y * base.max().y).with_scale(Vec3::new(
                            1.0,
                            BELL_DOME_FLATTEN,
                            1.0,
                        )),
                    ));
                    bell.spawn((
                        Mesh3d(meshes.add(Cylinder::new(
                            BELL_PLUNGER_SIZE.x / 2.0,
                            BELL_PLUNGER_SIZE.y,
                        ))),
                        MeshMaterial3d(brass),
                        plunger.transform(),
                    ));
                });
            }
        }
    }
}

/// The stamp hopping onto the job ticket after the bell rings, stamping it
/// and hopping back. Then the customer picks the job up.
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
        mut job: Option<ResMut<Job>>,
        mut stamps: Query<(Entity, &mut Stamping, &mut Transform)>,
        mut commands: Commands,
    ) {
        for (stamp, mut stamping, mut transform) in &mut stamps {
            let before = stamping.elapsed / STAMP_TIME;
            stamping.elapsed += time.delta_secs();
            let t = (stamping.elapsed / STAMP_TIME).min(1.0);
            transform.translation = stamping.at(t);
            if before < DOWN && t >= DOWN {
                if let Some(job) = job.as_mut() {
                    job.stamped = true;
                }
                commands.trigger(Play(Sound::Thunk));
            }
            if t >= 1.0 {
                *transform = stamping.rest;
                commands.entity(stamp).remove::<Stamping>();
                commands.trigger(HandIn);
            }
        }
    }
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}
