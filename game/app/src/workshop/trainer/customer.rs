use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_devices::MARKS;
use ld60_devices::{Movement, Wanted};
use ld60_game_audio::{Play, Sound};
use ld60_game_city::{Ask, CallOff, Left, Summon, Visitor};
#[cfg(feature = "debug_port")]
use ld60_game_core::debug_port::{DebugMethod, DebugMethods, ParamKind};
use ld60_game_core::layout::counter::{CounterLayout, STOOL_HEIGHT};
use ld60_game_core::state::AppState;
use ld60_game_device::Holdable;
use ld60_game_player::camera::PlayerCamera;
use ld60_game_player::focus::{Clickable, Focus, FocusOn, FocusTarget};
use ld60_game_player::hints::Hints;
use ld60_game_run::{Leftover, NewRun, Run};
use ld60_tick::Revision;
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

use super::super::delivery::{DEVICE_SPOT, DEVICE_YAW};
use super::{Datasheet, Flyer, Trainer, movement, sheet};

/// Seconds after the shop opens before someone walking by is asked over.
/// Their walk to the counter gives the player time to look around.
const SETTLE: f32 = 1.0;
/// Seconds of waiting to be seen before calling out.
const CALL_AFTER: f32 = 2.5;
/// Seconds of holding something out before shaking it.
const URGE_AFTER: f32 = 5.0;
/// Seconds the player has the movement to themselves before the flyer.
const FLYER_AFTER: f32 = 10.0;
/// Seconds of being looked at that count as being seen, at the counter and
/// seated.
const SEEN_AFTER: f32 = 0.3;
const COME_AFTER: f32 = 1.5;
/// Seconds of holding a hand out for the movement before sitting back.
const GIVE_UP: f32 = 10.0;
/// Seconds the customer watches the movement before deciding.
const JUDGE_FOR: f32 = 4.0;
const FEE: u32 = 3;
/// The player looks at someone when the view is this many radians from
/// its head at most, standing and seated.
const LOOK_CONE: f32 = 0.3;
const SEATED_CONE: f32 = 0.15;
/// The customer stands this far along the counter and this far in front
/// of it, and holds things out this high above the top and this far over
/// it.
const STAND_X: f32 = 0.08;
const STAND_BACK: f32 = 0.3;
const HOLD_UP: f32 = 0.28;
const HOLD_OVER: f32 = 0.4;
/// Things are held out on the side of the hand that holds them, clear of
/// the head.
const HOLD_ASIDE: f32 = 0.15;
/// Where the flyer and the datasheet lie once taken, on the mat.
const FLYER_SPOT: Vec2 = Vec2::new(-0.65, 0.3);
const FLYER_YAW: f32 = 0.1;
const DATASHEET_SPOT: Vec2 = Vec2::new(0.75, 0.35);
const DATASHEET_YAW: f32 = -0.15;
/// In the customer's hand, the flyer lies on the datasheet, this far off
/// its middle and turned this far.
const ON_DATASHEET: Vec3 = Vec3::new(0.01, 0.001, 0.02);
const ON_DATASHEET_YAW: f32 = 0.2;

/// A regular brings a bare watch movement for the first job of a run,
/// hands it over and then the flyer of what it should do, and waits on a
/// stool to take it back.
#[derive(Component, Debug)]
#[require(Transform, Visibility, Leftover)]
pub(super) struct WatchJob {
    step: Step,
    /// Seconds since the step began, and of being looked at in it.
    clock: f32,
    looked: f32,
    /// The movement while the panel does not work on it.
    brought: Option<Movement>,
    /// The model of the movement, and the datasheet with the flyer on it
    /// as the customer brings them.
    movement: Entity,
    papers: Entity,
    datasheet: Entity,
    flyer: Entity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Settle,
    Coming,
    /// At the counter, waiting to be seen.
    Noticing,
    /// Holding the movement out.
    Offering,
    /// The player has the movement.
    Trying,
    /// Calling with the flyer, and holding it out.
    Calling,
    OfferingFlyer,
    /// Seated, watching.
    Watching,
    /// At the counter with the hand out for the movement.
    Collecting,
    Judging,
    /// Holding out the movement that does not do what it should.
    Returning,
    Done,
}

/// The movement, datasheet or flyer the customer offers or wants now.
#[derive(Component, Debug)]
struct Offered;

/// The player takes what the customer offers, or gives what it wants.
#[derive(Event, Debug)]
struct HandOver {
    entity: Entity,
}

impl WatchJob {
    pub(super) fn register(app: &mut App) {
        app.add_systems(
            Update,
            (Self::open, Self::follow)
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        .add_observer(Self::take)
        .add_observer(Self::hand_over)
        .add_observer(Self::leave)
        .add_observer(Self::restart);
        #[cfg(feature = "debug_port")]
        app.add_debug_method(
            DebugMethod::new("ld60.watch_job", "Returns the step of the watch job.")
                .param("hurry", ParamKind::Flag, "skips the wait of the step")
                .param(
                    "take",
                    ParamKind::Flag,
                    "takes or gives what the customer offers or wants",
                ),
            Self::debug_step,
        );
    }

    /// Each run starts with this job. The movement comes with its hand
    /// stopped wherever it stopped.
    fn open(jobs: Query<(), With<WatchJob>>, run: Res<Run>, mut commands: Commands) {
        if !jobs.is_empty() {
            return;
        }
        let stopped = SmallRng::seed_from_u64(run.seed()).random_range(1..MARKS);
        let movement = commands
            .spawn((
                Name::new("brought movement"),
                Leftover,
                Transform::from_xyz(0.0, -10.0, 0.0),
                Visibility::default(),
            ))
            .with_children(movement::spawn(stopped))
            .id();
        let datasheet = commands
            .spawn((Name::new("tick datasheet"), Leftover, Datasheet))
            .id();
        let flyer = commands
            .spawn((
                Name::new("flyer"),
                Leftover,
                Flyer { seed: 1 },
                Transform::from_translation(ON_DATASHEET)
                    .with_rotation(Quat::from_rotation_y(ON_DATASHEET_YAW)),
            ))
            .id();
        let papers = commands
            .spawn((
                Name::new("brought papers"),
                Leftover,
                Transform::from_xyz(0.0, -10.0, 0.0),
                Visibility::Hidden,
            ))
            .add_children(&[datasheet, flyer])
            .id();
        commands.spawn((
            Name::new("watch job"),
            WatchJob {
                step: Step::Settle,
                clock: 0.0,
                looked: 0.0,
                brought: Some(Movement::blank(Revision::A).stopped_at(stopped)),
                movement,
                papers,
                datasheet,
                flyer,
            },
        ));
    }

    /// Moves the job on as the customer and the player go.
    fn follow(
        time: Res<Time>,
        focus: Res<State<Focus>>,
        camera: Single<&GlobalTransform, With<PlayerCamera>>,
        mut jobs: Query<(Entity, &mut WatchJob)>,
        mut visitors: Query<&mut Visitor>,
        mut run: ResMut<Run>,
        mut commands: Commands,
    ) {
        let dt = time.delta_secs();
        for (errand, mut job) in &mut jobs {
            job.clock += dt;
            if job.step == Step::Settle {
                if job.clock > SETTLE {
                    commands.trigger(Summon {
                        errand,
                        stand: Self::stand(),
                        carrying: job.movement,
                    });
                    job.go(Step::Coming);
                }
                continue;
            }
            let Some(mut visitor) = visitors.iter_mut().find(|visitor| visitor.errand == errand)
            else {
                continue;
            };
            let cone = if visitor.seated() {
                SEATED_CONE
            } else {
                LOOK_CONE
            };
            let face = visitor.head();
            // Only from inside the shop, where the customer faces.
            let seen = *focus.get() == Focus::Desk
                && camera.translation().z > face.z
                && camera.forward().angle_between(face - camera.translation()) < cone;
            job.looked = if seen { job.looked + dt } else { 0.0 };
            let urge = job.clock > URGE_AFTER;
            let hold_out = Ask::HoldOut {
                at: Self::hold_out(),
                urge,
            };
            match job.step {
                Step::Settle | Step::Done => {}
                Step::Coming => {
                    if visitor.at_counter() {
                        job.go(Step::Noticing);
                    }
                }
                Step::Noticing => {
                    visitor.ask = if job.clock > CALL_AFTER {
                        Ask::Call
                    } else {
                        Ask::Wait
                    };
                    if job.looked > SEEN_AFTER {
                        Self::offer(&mut commands, job.movement, "click-take-movement");
                        job.go(Step::Offering);
                    }
                }
                Step::Offering | Step::OfferingFlyer | Step::Returning => visitor.ask = hold_out,
                // Looking back at the customer early brings the papers at
                // once. Calling is for those who stay at the panel.
                Step::Trying => {
                    visitor.ask = Ask::Wait;
                    let seen = job.looked > SEEN_AFTER;
                    if seen || job.clock > FLYER_AFTER {
                        visitor.carrying = Some(job.papers);
                        commands.entity(job.papers).insert(Visibility::Inherited);
                    }
                    if seen {
                        Self::offer(&mut commands, job.papers, "click-take-papers");
                        job.go(Step::OfferingFlyer);
                    } else if job.clock > FLYER_AFTER {
                        job.go(Step::Calling);
                    }
                }
                Step::Calling => {
                    visitor.ask = Ask::Call;
                    if job.looked > SEEN_AFTER {
                        Self::offer(&mut commands, job.papers, "click-take-papers");
                        job.go(Step::OfferingFlyer);
                    }
                }
                Step::Watching => {
                    visitor.ask = Self::sit();
                    if visitor.seated() && job.looked > COME_AFTER {
                        // The movement can't be picked up while it is wanted.
                        commands
                            .entity(job.movement)
                            .remove::<(Holdable, FocusTarget, Hints)>();
                        Self::offer(&mut commands, job.movement, "click-hand-back");
                        job.go(Step::Collecting);
                    }
                }
                Step::Collecting => {
                    visitor.ask = Ask::HoldOut {
                        at: Self::hold_out(),
                        urge: false,
                    };
                    if job.clock > GIVE_UP {
                        Self::withdraw(&mut commands, job.movement);
                        commands.entity(job.movement).insert(Holdable::default());
                        job.go(Step::Watching);
                    }
                }
                Step::Judging => {
                    visitor.ask = Ask::Wait;
                    if job.clock > JUDGE_FOR {
                        job.judge(&mut visitor, &mut run, &mut commands);
                    }
                }
            }
        }
    }

    fn go(&mut self, step: Step) {
        self.step = step;
        self.clock = 0.0;
        self.looked = 0.0;
    }

    /// The customer tries the movement as the flyer shows. If it works, it
    /// pays and goes. If not, it hands the movement back.
    fn judge(&mut self, visitor: &mut Visitor, run: &mut Run, commands: &mut Commands) {
        let works = self
            .brought
            .as_ref()
            .is_some_and(|movement| Wanted::Ticking.check(movement).is_ok());
        if works {
            run.earn(FEE);
            commands.trigger(Play(Sound::Coin));
            visitor.ask = Ask::Leave;
            self.go(Step::Done);
        } else {
            Self::offer(commands, self.movement, "click-take-movement");
            self.go(Step::Returning);
        }
    }

    /// A click on what the customer offers or wants hands it over.
    fn take(
        mut press: On<PointerPress>,
        focus: Res<State<Focus>>,
        offered: Query<(), With<Offered>>,
        mut commands: Commands,
    ) {
        if press.button == PointerButton::Primary
            && *focus.get() == Focus::Desk
            && offered.contains(press.entity)
        {
            press.propagate(false);
            commands.trigger(HandOver {
                entity: press.entity,
            });
        }
    }

    /// Takes what the customer holds out, or hands the movement back.
    fn hand_over(
        hand_over: On<HandOver>,
        mut jobs: Query<(Entity, &mut WatchJob)>,
        mut visitors: Query<&mut Visitor>,
        mut trainers: Query<(Entity, &mut Trainer)>,
        mut commands: Commands,
    ) {
        let entity = hand_over.entity;
        let Ok((errand, mut job)) = jobs.single_mut() else {
            return;
        };
        let Some(mut visitor) = visitors.iter_mut().find(|visitor| visitor.errand == errand) else {
            return;
        };
        let Ok((panel, mut trainer)) = trainers.single_mut() else {
            return;
        };
        Self::withdraw(&mut commands, entity);
        let counter = CounterLayout::new();
        match job.step {
            Step::Offering if entity == job.movement => {
                job.put_down(&mut visitor, &mut trainer, &mut commands);
                commands.trigger(FocusOn { entity: panel });
                job.go(Step::Trying);
            }
            Step::Returning if entity == job.movement => {
                job.put_down(&mut visitor, &mut trainer, &mut commands);
                job.go(Step::Watching);
            }
            Step::OfferingFlyer if entity == job.papers => {
                visitor.carrying = None;
                commands.entity(job.flyer).remove::<ChildOf>().insert((
                    counter.place_on_mat(FLYER_SPOT, FLYER_YAW),
                    sheet::reading("click-flyer"),
                ));
                commands.entity(job.datasheet).remove::<ChildOf>().insert((
                    counter.place_on_mat(DATASHEET_SPOT, DATASHEET_YAW),
                    sheet::reading("click-datasheet"),
                ));
                commands.entity(job.papers).despawn();
                commands.trigger(Play(Sound::PageSlide));
                commands.trigger(FocusOn { entity: job.flyer });
                job.go(Step::Watching);
            }
            Step::Collecting if entity == job.movement => {
                job.brought = trainer.take();
                visitor.carrying = Some(job.movement);
                commands.trigger(Play(Sound::PlugOut));
                job.go(Step::Judging);
            }
            _ => {}
        }
    }

    /// Takes the movement from the customer and puts it down where devices
    /// go, for the panel to work on and the player to pick up.
    fn put_down(&mut self, visitor: &mut Visitor, trainer: &mut Trainer, commands: &mut Commands) {
        visitor.carrying = None;
        if let Some(movement) = self.brought.take() {
            trainer.mount(movement, self.movement);
        }
        commands.entity(self.movement).insert((
            CounterLayout::new().place(DEVICE_SPOT, DEVICE_YAW),
            Holdable::default(),
        ));
        commands.trigger(Play(Sound::PlugIn));
    }

    /// Once the customer is out of sight, so is the movement it took.
    fn leave(left: On<Left>, jobs: Query<&WatchJob>, mut commands: Commands) {
        if let Ok(job) = jobs.get(left.errand) {
            commands.entity(job.movement).despawn();
        }
    }

    /// A new run sends the customer away and clears the panel, for the job
    /// to start over.
    fn restart(
        _: On<NewRun>,
        jobs: Query<Entity, With<WatchJob>>,
        mut trainers: Query<&mut Trainer>,
        mut commands: Commands,
    ) {
        for errand in &jobs {
            commands.trigger(CallOff { errand });
        }
        for mut trainer in &mut trainers {
            trainer.take();
        }
    }

    fn offer(commands: &mut Commands, entity: Entity, prompt: &'static str) {
        commands
            .entity(entity)
            .insert((Offered, Clickable { prompt }));
    }

    fn withdraw(commands: &mut Commands, entity: Entity) {
        commands.entity(entity).remove::<(Offered, Clickable)>();
    }

    fn stand() -> Vec2 {
        Vec2::new(STAND_X, CounterLayout::new().top.min().z - STAND_BACK)
    }

    /// Where the customer holds things out to the player, over the counter.
    fn hold_out() -> Vec3 {
        let top = CounterLayout::new().top;
        Vec3::new(
            STAND_X + HOLD_ASIDE,
            top.max().y + HOLD_UP,
            top.min().z + HOLD_OVER,
        )
    }

    /// Sitting on the stool left of the counter, where the player sees it
    /// past the monitor.
    fn sit() -> Ask {
        let at = CounterLayout::new()
            .stools()
            .min_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap_or_default();
        Ask::Sit {
            at,
            height: STOOL_HEIGHT,
        }
    }

    #[cfg(feature = "debug_port")]
    fn debug_step(
        In(params): In<Option<serde_json::Value>>,
        mut jobs: Query<&mut WatchJob>,
        offered: Query<Entity, With<Offered>>,
        mut commands: Commands,
    ) -> bevy::remote::BrpResult {
        let flag = |name: &str| {
            params
                .as_ref()
                .and_then(|params| params.get(name))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        };
        if flag("take") {
            for entity in &offered {
                commands.trigger(HandOver { entity });
            }
        }
        let mut steps = Vec::new();
        for mut job in &mut jobs {
            if flag("hurry")
                && let Some(wait) = job.step.wait()
            {
                job.clock = job.clock.max(wait);
            }
            steps.push(format!("{:?}", job.step));
        }
        Ok(serde_json::json!(steps))
    }
}

impl Step {
    /// How long the step waits before it moves on by itself, if it does.
    #[cfg(feature = "debug_port")]
    fn wait(self) -> Option<f32> {
        match self {
            Self::Settle => Some(SETTLE),
            Self::Noticing => Some(CALL_AFTER),
            Self::Offering | Self::OfferingFlyer | Self::Returning => Some(URGE_AFTER),
            Self::Trying => Some(FLYER_AFTER),
            Self::Judging => Some(JUDGE_FOR),
            Self::Coming | Self::Calling | Self::Watching | Self::Collecting | Self::Done => None,
        }
    }
}
