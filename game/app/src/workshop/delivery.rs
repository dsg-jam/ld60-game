mod drone;

use bevy::prelude::*;
use ld60_game_core::layout::counter::CounterLayout;
use ld60_game_device::{Carried, Emulator, Handheld};
use ld60_game_player::camera::PlayerCamera;
use ld60_game_player::focus::Focus;
use ld60_game_run::chore::{Busy, Chore};
use ld60_game_run::job::{Job, JobOnBench};
use ld60_game_run::pickup::Complaint;
use ld60_game_run::tutorial::{FirstNote, TutorialNote};
use ld60_game_run::{Leftover, Run};

use self::drone::Tether;
pub(super) use self::drone::{Drone, Flight};
use super::printer::Printing;
use super::receipt::Receipt;
use super::ticket::JobTicket;

pub(super) const DEVICE_SPOT: Vec2 = Vec2::new(0.03, 0.3);
pub(super) const DEVICE_YAW: f32 = 0.15;
pub(super) const TICKET_SPOT: Vec2 = Vec2::new(-0.85, 0.5);
pub(super) const TICKET_YAW: f32 = 0.12;
/// Seconds from the drone setting a device down, or leaving, until the next
/// sheet prints.
const AFTER_LANDING: f32 = 0.4;
/// Seconds from the drone leaving with a job until it brings the next.
const NEXT_JOB: f32 = 0.8;

pub(crate) struct DeliveryPlugin;

impl Plugin for DeliveryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (Flight::fly, Tether::follow, Drone::blow).chain(),
                Drone::spin,
                FirstDelivery::send.run_if(resource_exists::<FirstDelivery>),
            ),
        )
        .add_observer(JobItem::spawn_items)
        .add_observer(HandIn::pick_up)
        .add_observer(Drone::on_add)
        .add_observer(Flight::clear_cargo);
    }
}

/// The device and ticket of the first job of a run. They come once the
/// player sat down and read the first note.
#[derive(Resource, Debug)]
struct FirstDelivery {
    device: Entity,
    ticket: Entity,
}

/// A job item the drone may carry: where it rests, if anywhere, and whether
/// it is the ticket.
type Carryable<'a> = (Entity, Option<&'a Transform>, Has<JobTicket>);

/// The items of the next job come in with the drone after this many seconds,
/// instead of being on the counter at once.
#[derive(Resource, Debug)]
struct Arrive(f32);

/// The device, ticket and notes of the current [`Job`]. They leave the desk
/// when the customer takes the device home.
#[derive(Component, Debug, Default)]
#[require(Leftover)]
pub(crate) struct JobItem;

/// Hands in the stamped job and lets the customer test it. Once they take
/// the device home, the next job comes on the bench.
#[derive(Event, Debug)]
pub(crate) struct HandIn;

impl JobItem {
    /// Puts the device and the ticket of the job that came on the bench on
    /// the counter, instead of those of the job before. After a pickup they
    /// come in, see [`Arrive`].
    fn spawn_items(
        _: On<JobOnBench>,
        job: Option<Res<Job>>,
        arrive: Option<Res<Arrive>>,
        run: Res<Run>,
        old: Query<Entity, With<JobItem>>,
        mut commands: Commands,
    ) {
        let Some(job) = job else {
            return;
        };
        for item in &old {
            commands.entity(item).despawn();
        }
        let counter = CounterLayout::new();
        let items = [
            commands
                .spawn((
                    Name::new("device"),
                    JobItem,
                    Handheld::for_job(&job.level, &job.buggy),
                    counter.place(DEVICE_SPOT, DEVICE_YAW),
                ))
                .id(),
            commands
                .spawn((
                    Name::new("job ticket"),
                    JobItem,
                    JobTicket::new(&job.level, job.visit),
                    counter.place(TICKET_SPOT, TICKET_YAW),
                ))
                .id(),
        ];
        // A new run starts with an empty counter.
        let first = TutorialNote::for_job(&job)
            .is_some_and(|note| note == TutorialNote::FIRST && !run.notes().contains(&0));
        if first {
            let [device, ticket] = items;
            commands
                .entity(device)
                .insert((Carried, Visibility::Hidden));
            commands.entity(ticket).insert(Visibility::Hidden);
            commands.insert_resource(FirstDelivery { device, ticket });
            return;
        }
        // The drone brings the device, then the ticket comes out of the
        // printer.
        if let Some(arrive) = arrive {
            commands.remove_resource::<Arrive>();
            let [device, ticket] = items;
            let rest = counter.place(DEVICE_SPOT, DEVICE_YAW);
            let (flight, landed) = Flight::bring(job.customer(), device, rest, arrive.0);
            flight.spawn(job.customer(), &mut commands);
            commands.entity(device).insert(Carried);
            commands.entity(ticket).insert(Printing::ticket(
                counter.place(TICKET_SPOT, TICKET_YAW),
                landed + AFTER_LANDING,
            ));
        }
    }
}

impl FirstDelivery {
    /// Sends the first job once the player is back at the desk from the
    /// first note.
    fn send(
        delivery: Res<FirstDelivery>,
        job: Res<Job>,
        first_note: Option<Res<FirstNote>>,
        state: Res<State<Focus>>,
        camera: Single<(&PlayerCamera, &Transform)>,
        mut commands: Commands,
    ) {
        let (camera, at) = *camera;
        if first_note.is_some() || *state.get() != Focus::Desk || !camera.seated(at) {
            return;
        }
        commands.remove_resource::<FirstDelivery>();
        let counter = CounterLayout::new();
        let (flight, landed) = Flight::bring(
            job.customer(),
            delivery.device,
            counter.place(DEVICE_SPOT, DEVICE_YAW),
            NEXT_JOB,
        );
        flight.spawn(job.customer(), &mut commands);
        commands.entity(delivery.ticket).insert(Printing::ticket(
            counter.place(TICKET_SPOT, TICKET_YAW),
            landed + AFTER_LANDING,
        ));
    }
}

impl HandIn {
    fn pick_up(
        _: On<HandIn>,
        job: Option<ResMut<Job>>,
        items: Query<Carryable, With<JobItem>>,
        emulators: Query<&Emulator, With<JobItem>>,
        receipts: Query<Entity, With<Receipt>>,
        mut run: ResMut<Run>,
        mut commands: Commands,
    ) {
        let Some(mut job) = job.filter(|job| job.stamped) else {
            return;
        };
        let tried = match emulators.single() {
            Ok(emulator) => job.try_out(emulator.rom()),
            Err(_) => Err(Complaint::Dead),
        };
        let pickup = job.pick_up(tried);
        run.settle(&job, &pickup);
        let ends_job = pickup.ends_job();
        let receipt = Receipt::new(&job.level, pickup, run.till());
        let chore = if ends_job {
            Chore::HandIn
        } else {
            Chore::Return
        };
        commands.trigger(Busy(chore));
        // The drone takes the ticket and the device to the customer.
        let mut carried: Vec<_> = items
            .iter()
            .filter_map(|(item, rest, ticket)| Some((item, *rest?, ticket)))
            .collect();
        if !ends_job {
            // They try it and send it back.
            let back = carried
                .iter()
                .find(|(.., ticket)| !ticket)
                .map(|&(device, rest, _)| {
                    let (flight, landed) = Flight::try_out(job.customer(), device, rest);
                    flight.spawn(job.customer(), &mut commands);
                    landed
                });
            receipt.hand_out(receipts, back.unwrap_or(0.0) + AFTER_LANDING, &mut commands);
            return;
        }
        // Items that are nowhere go at once.
        for (item, rest, _) in &items {
            if rest.is_none() {
                commands.entity(item).despawn();
            }
        }
        // The ticket first, it lies on the way.
        carried.sort_by_key(|&(.., ticket)| !ticket);
        for &(item, ..) in &carried {
            commands.entity(item).remove::<JobItem>();
        }
        let take = Flight::take(job.customer(), &carried);
        let gone = take.duration();
        take.spawn(job.customer(), &mut commands);
        receipt.hand_out(receipts, gone + AFTER_LANDING, &mut commands);
        commands.insert_resource(Arrive(gone + NEXT_JOB));
        commands.run_system_cached(Job::start);
    }
}

#[cfg(test)]
mod tests {
    use ld60_game_run::Origin;
    use ld60_game_run::levels::Levels;
    use ld60_game_run::pickup::{TIP, TRIES};
    use ld60_levels::{GAME_SEED, Variant, assemble, burn, initial_rom};

    use super::*;

    fn job(index: usize) -> Job {
        Job::load(&Levels::on_disk(), index).unwrap().unwrap()
    }

    /// The tutorial step of the job on the bench.
    fn step(world: &World) -> Option<usize> {
        match world.get_resource::<Job>()?.origin {
            Origin::Lesson { step, .. } => Some(step),
            _ => None,
        }
    }

    /// A world with the job at `index` flashed and stamped, and its device
    /// running the fixed firmware if `fixed`, else the buggy one.
    fn bench(index: usize, fixed: bool) -> World {
        let mut job = job(index);
        let levels = Levels::on_disk();
        let variant = Variant::new(&levels.levels()[index], GAME_SEED).unwrap();
        let mut rom = initial_rom(&job.level, &job.buggy).unwrap();
        if fixed {
            let image = assemble(&job.level, &variant.fixed).unwrap().image;
            rom = burn(&job.level, &rom, image).unwrap();
        }
        let device = ld60_levels::device(&job.level, rom).unwrap();
        job.flashed = true;
        job.stamped = true;
        let mut world = World::new();
        world.add_observer(HandIn::pick_up);
        world.insert_resource(job);
        world.insert_resource(levels);
        world.init_resource::<Run>();
        world.spawn((JobItem, Emulator::power_on(device)));
        world
    }

    #[test]
    fn a_job_on_the_bench_brings_its_device_and_ticket_once() {
        let mut world = World::new();
        world.add_observer(JobItem::spawn_items);
        world.init_resource::<Run>();
        world.insert_resource(job(0));
        world.trigger(JobOnBench);
        world.flush();
        world.trigger(JobOnBench);
        world.flush();
        assert_eq!(world.query::<&JobItem>().iter(&world).count(), 2);
    }

    #[test]
    fn ringing_twice_pays_once() {
        let mut world = bench(0, true);
        let reward = world.resource::<Job>().level.reward;
        world.trigger(HandIn);
        world.trigger(HandIn);
        world.flush();
        assert_eq!(world.resource::<Run>().till(), reward + 2 * TIP);
        assert_eq!(step(&world), Some(1));
    }

    #[test]
    fn ringing_twice_hands_back_once() {
        let mut world = bench(0, false);
        let id = world.resource::<Job>().id;
        world.trigger(HandIn);
        world.trigger(HandIn);
        world.flush();
        let job = world.resource::<Job>();
        assert_eq!((job.id, job.returns), (id, 1));
        assert_eq!(world.resource::<Run>().till(), 0);
        assert_eq!(world.query::<&JobItem>().iter(&world).count(), 1);
    }

    #[test]
    fn last_failed_try_moves_on_unpaid() {
        let last = Levels::on_disk().levels().len() - 1;
        for index in [0, last] {
            let mut world = bench(index, false);
            let id = world.resource::<Job>().id;
            let items: Vec<_> = world
                .query_filtered::<Entity, With<JobItem>>()
                .iter(&world)
                .collect();
            for _ in 0..TRIES {
                let mut job = world.resource_mut::<Job>();
                job.flashed = true;
                job.stamped = true;
                world.trigger(HandIn);
            }
            world.flush();
            assert_ne!(world.resource::<Job>().id, id, "job {index}");
            assert!(
                items.iter().all(|&item| world.get_entity(item).is_err()),
                "job {index}"
            );
            let run = world.resource::<Run>();
            assert_eq!((run.till(), run.stats().lost), (0, 1), "job {index}");
        }
    }
}
