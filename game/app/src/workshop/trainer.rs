#[cfg(feature = "debug_port")]
mod command;
mod customer;
mod datasheet;
mod flyer;
mod layout;
mod movement;
mod sheet;

use bevy::prelude::*;
use ld60_devices::Movement;
use ld60_game_audio::{Play, PlayAt, Sound};
use ld60_game_parts::led::Led;
use ld60_game_parts::push_button::{PushButton, Pushed};
use ld60_game_parts::toggle::Toggle;
use ld60_game_player::cursor::WantsFinger;
use ld60_game_player::focus::{Focus, FocusTarget, Focused, View};
use ld60_game_run::job::HeldJobs;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;
use ld60_locale::Locale;
use ld60_tick::Effect;

use self::customer::WatchJob;
pub(crate) use self::datasheet::Datasheet;
pub(crate) use self::flyer::Flyer;
use self::layout::{CASE, PLATE_COLOR};
use self::movement::Hand;

/// Instructions a second while running. Fast enough that a program
/// without a wait spins the hand.
const SPEED: f32 = 200.0;
/// Seconds between two ticks of the crystal.
const TICK: f32 = 1.0;
/// Seconds the UV lamp takes to erase the chip.
const ERASE_TIME: f32 = 3.0;
/// Camera above the front of the panel while focused, and where it looks,
/// to see the movement in front of the panel too.
const VIEW: Vec3 = Vec3::new(0.0, 0.34, 0.36);
const VIEW_AT: Vec3 = Vec3::new(0.0, 0.0, 0.07);

const RED: Color = Color::srgb_u8(0xff, 0x30, 0x20);
const GREEN: Color = Color::srgb_u8(0x40, 0xff, 0x50);
const AMBER: Color = Color::srgb_u8(0xff, 0xb0, 0x20);
const VIOLET: Color = Color::srgb_u8(0xa0, 0x40, 0xff);
const BUTTON: Color = Color::srgb_u8(0xd8, 0xd4, 0xc8);
const ERASE_BUTTON: Color = Color::srgb_u8(0xc0, 0x30, 0x28);

pub(crate) struct TrainerPlugin;

impl Plugin for TrainerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Trainer::print,
                Trainer::run,
                Trainer::show,
                Trainer::arm_controls,
            )
                .chain(),
        )
        .add_observer(Trainer::on_add)
        .add_observer(Datasheet::on_add)
        .add_observer(Flyer::on_add)
        .add_observer(Trainer::push);
        // The watch job replaces the old first jobs for now.
        app.init_resource::<HeldJobs>();
        WatchJob::register(app);
        #[cfg(feature = "debug_port")]
        command::register(app);
    }
}

/// A front panel to program and step the chip of a watch movement on the
/// mat. Switches set a word, lamps show the address and the word there.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Trainer {
    /// The movement it works on, if one is on the mat, and its model.
    movement: Option<Movement>,
    model: Option<Entity>,
    /// Seconds until the next tick.
    until_tick: f32,
    /// Seconds left of an erase.
    erasing: Option<f32>,
    /// Instructions owed to the running chip.
    owed: f32,
}

impl Default for Trainer {
    fn default() -> Self {
        Self {
            movement: None,
            model: None,
            until_tick: TICK,
            erasing: None,
            owed: 0.0,
        }
    }
}

/// What a control on the panel is for.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    /// The switch for one bit of the word to deposit.
    Data(u8),
    Run,
    Reset,
    Erase,
    Next,
    Deposit,
    Step,
}

/// What a lamp on the panel shows.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Lamp {
    Address(u8),
    Data(u8),
    Wait,
    Erasing,
}

/// The top of the case, printed once the words are loaded.
#[derive(Component, Debug)]
struct Unprinted;

impl Trainer {
    fn on_add(add: On<Add<Trainer>>, mut commands: Commands) {
        let case = Block::new(CASE.with_y(CASE.y - 0.001));
        let top = |at: Vec2| Transform::from_xyz(at.x, CASE.y, at.y);
        commands
            .entity(add.entity)
            .insert(FocusTarget {
                prompt: "click-trainer",
                focus: Focus::Trainer,
                view: View::Local(
                    Transform::from_translation(VIEW).looking_at(VIEW_AT.with_y(CASE.y), Vec3::Y),
                ),
            })
            .with_children(|parent| {
                parent.spawn((Shape::cuboid(&case, PLATE_COLOR), Finish::Paint));
                parent.spawn((Unprinted, Transform::default()));
                for (control, at) in layout::controls() {
                    let mut entity = parent.spawn((control, top(at)));
                    match control {
                        Control::Data(_) => entity.insert(Toggle { up: false }),
                        Control::Run => entity.insert(Toggle { up: false }),
                        Control::Erase => entity.insert(PushButton::new(ERASE_BUTTON)),
                        Control::Reset | Control::Next | Control::Deposit | Control::Step => {
                            entity.insert(PushButton::new(BUTTON))
                        }
                    };
                }
                for (lamp, at) in layout::lamps() {
                    let color = match lamp {
                        Lamp::Address(_) => RED,
                        Lamp::Data(_) => GREEN,
                        Lamp::Wait => AMBER,
                        Lamp::Erasing => VIOLET,
                    };
                    parent.spawn((lamp, Led { color, on: false }, top(at)));
                }
            });
    }

    /// Prints the top of the case once the words are loaded.
    fn print(
        locale: Res<Locale>,
        plates: Query<Entity, With<Unprinted>>,
        mut images: ResMut<Assets<Image>>,
        mut commands: Commands,
    ) {
        if plates.is_empty() || !locale.is_ready() {
            return;
        }
        let image = images.add(layout::plate(&locale));
        let top = Block::new(CASE.with_y(0.001)).translated(Vec3::Y * (CASE.y - 0.001));
        for entity in &plates {
            commands.entity(entity).remove::<Unprinted>().insert((
                Shape::textured_cuboid(&top, image.clone(), Rect::new(0.0, 0.0, 1.0, 1.0)),
                Finish::Paint,
            ));
        }
    }

    /// Does what a pushed button on a panel is for.
    fn push(
        push: On<Pushed>,
        controls: Query<(&Control, &ChildOf)>,
        mut trainers: Query<(&mut Trainer, &Children)>,
        switches: Query<(&Control, &Toggle)>,
        mut commands: Commands,
    ) {
        let Ok((&control, child_of)) = controls.get(push.entity) else {
            return;
        };
        let Ok((mut trainer, children)) = trainers.get_mut(child_of.parent()) else {
            return;
        };
        let erasing = trainer.erasing.is_some();
        let Some(movement) = trainer.movement.as_mut() else {
            if !matches!(control, Control::Data(_) | Control::Run) {
                commands.trigger(Play(Sound::Refuse));
            }
            return;
        };
        let pc = movement.cpu().pc();
        match control {
            Control::Reset => movement.cpu_mut().reset(),
            Control::Next => movement.cpu_mut().set_pc(pc + 1),
            Control::Step => {
                movement.step();
            }
            Control::Erase => {
                if !erasing {
                    trainer.erasing = Some(ERASE_TIME);
                }
            }
            Control::Deposit => {
                let word = switches
                    .iter_many(children)
                    .filter_map(|part| match part {
                        Ok((Control::Data(bit), toggle)) if toggle.up => Some(1 << bit),
                        _ => None,
                    })
                    .sum();
                let written = !erasing
                    && movement
                        .rom_mut()
                        .program_word(usize::from(pc), word)
                        .is_ok();
                if written {
                    movement.cpu_mut().set_pc(pc + 1);
                } else {
                    commands.trigger(Play(Sound::Refuse));
                }
            }
            Control::Data(_) | Control::Run => {}
        }
    }

    /// Ticks the crystal, erases and runs the chip while RUN is up.
    fn run(
        time: Res<Time>,
        mut trainers: Query<(&mut Trainer, &Children)>,
        switches: Query<(&Control, &Toggle)>,
    ) {
        let dt = time.delta_secs();
        for (mut trainer, children) in &mut trainers {
            let trainer = &mut *trainer;
            let Some(movement) = trainer.movement.as_mut() else {
                continue;
            };
            trainer.until_tick -= dt;
            if trainer.until_tick <= 0.0 {
                trainer.until_tick += TICK;
                movement.tick();
            }
            if let Some(left) = trainer.erasing {
                if left > dt {
                    trainer.erasing = Some(left - dt);
                } else {
                    trainer.erasing = None;
                    // The chip in a movement is always an EPROM.
                    let _ = movement.rom_mut().erase();
                }
            }
            let running = switches
                .iter_many(children)
                .flatten()
                .any(|(control, toggle)| *control == Control::Run && toggle.up);
            if !running {
                trainer.owed = 0.0;
                continue;
            }
            trainer.owed += SPEED * dt;
            while trainer.owed >= 1.0 {
                trainer.owed -= 1.0;
                if movement.step() == Effect::Asleep {
                    trainer.owed = 0.0;
                }
            }
        }
    }

    /// The lamps show the address, the word there and whether the chip
    /// waits. The hand shows where the movement points, and ticks as it
    /// moves.
    fn show(
        trainers: Query<(&Trainer, &Children)>,
        children: Query<&Children>,
        mut lamps: Query<(&Lamp, &mut Led)>,
        mut hands: Query<(&mut Transform, &GlobalTransform), With<Hand>>,
        mut commands: Commands,
    ) {
        for (trainer, parts) in &trainers {
            let movement = trainer.movement.as_ref();
            for &part in parts {
                if let Ok((lamp, mut led)) = lamps.get_mut(part) {
                    let on = movement.is_some_and(|movement| {
                        let cpu = movement.cpu();
                        let word = movement.rom().image().get(usize::from(cpu.pc()));
                        match *lamp {
                            Lamp::Address(bit) => cpu.pc() >> bit & 1 != 0,
                            Lamp::Data(bit) => word >> bit & 1 != 0,
                            Lamp::Wait => cpu.waiting(),
                            Lamp::Erasing => trainer.erasing.is_some(),
                        }
                    });
                    if led.on != on {
                        led.on = on;
                    }
                }
            }
            let (Some(movement), Some(model)) = (movement, trainer.model) else {
                continue;
            };
            for descendant in children.iter_descendants(model) {
                if let Ok((mut hand, placed)) = hands.get_mut(descendant) {
                    let goal = Hand::at(movement.hand());
                    if hand.rotation != goal {
                        hand.rotation = goal;
                        commands.trigger(PlayAt(Sound::Ratchet, placed.translation()));
                    }
                }
            }
        }
    }

    /// Works on `movement` from power on, shown by `model`.
    pub(crate) fn mount(&mut self, mut movement: Movement, model: Entity) {
        movement.power_on();
        self.movement = Some(movement);
        self.model = Some(model);
        self.until_tick = TICK;
    }

    /// Lets go of the movement, if it works on one.
    pub(crate) fn take(&mut self) -> Option<Movement> {
        self.erasing = None;
        self.model = None;
        self.movement.take()
    }

    /// The controls of the focused panel take clicks.
    fn arm_controls(
        focused: Res<Focused>,
        trainers: Query<(Entity, &Children), With<Trainer>>,
        mut fingers: Query<&mut WantsFinger, With<Control>>,
    ) {
        for (entity, children) in &trainers {
            let near = focused.get() == Some(entity);
            let mut iter = fingers.iter_many_mut(children);
            while let Some(finger) = iter.fetch_next() {
                if let Ok(mut finger) = finger {
                    finger.set_if_neq(WantsFinger(near));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_locale::Message;

    use super::*;

    #[test]
    fn every_word_formats() {
        Locale::on_disk().assert_formats(
            [
                "click-trainer",
                "panel-address",
                "panel-wait",
                "panel-data",
                "panel-run",
                "panel-reset",
                "panel-erase",
                "panel-next",
                "panel-deposit",
                "panel-step",
                "click-datasheet",
                "click-flyer",
                "click-take-movement",
                "click-take-flyer",
                "click-hand-back",
                "tick-sheet-title",
                "tick-sheet-revision",
                "tick-sheet-word",
                "tick-sheet-name",
                "tick-sheet-effect",
                "tick-op-nop",
                "tick-op-step",
                "tick-op-wait",
                "tick-sheet-other",
                "tick-sheet-order",
                "tick-sheet-erase",
            ]
            .map(Message::new),
        );
    }
}
