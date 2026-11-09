#[cfg(feature = "debug_port")]
mod command;
mod customer;
mod datasheet;
mod flyer;
mod layout;
mod movement;
mod ribbon;
mod sheet;

use bevy::prelude::*;
use ld60_devices::Movement;
use ld60_game_audio::{PlayAt, Sound};
use ld60_game_parts::dip::DipSwitch;
use ld60_game_parts::led::Led;
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
use self::ribbon::Ribbon;

/// Instructions a second while running. Slow enough to see the lamp go
/// from row to row, fast enough that a program without a wait spins the
/// hand.
const SPEED: f32 = 10.0;
/// Seconds between two ticks of the crystal.
const TICK: f32 = 1.0;
/// Words the panel has a row of switches for.
const WORDS: usize = 4;
/// Camera above the front of the panel while focused, and where it looks,
/// to see the movement in front of the panel too.
const VIEW: Vec3 = Vec3::new(0.0, 0.3, 0.32);
const VIEW_AT: Vec3 = Vec3::new(0.0, 0.0, 0.11);

const GREEN: Color = Color::srgb_u8(0x40, 0xff, 0x50);

pub(crate) struct TrainerPlugin;

impl Plugin for TrainerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Trainer::print,
                Trainer::wire,
                Trainer::run,
                Trainer::show,
                Trainer::arm_controls,
                Hand::swing,
                Ribbon::lay,
            )
                .chain(),
        )
        .add_observer(Trainer::on_add)
        .add_observer(Datasheet::on_add)
        .add_observer(Flyer::on_add);
        // The watch job replaces the old first jobs for now.
        app.init_resource::<HeldJobs>();
        WatchJob::register(app);
        #[cfg(feature = "debug_port")]
        command::register(app);
    }
}

/// A front panel wired to the memory of a watch movement on the mat. Each
/// row of switches is a word of the program, on for 1. The lamp of the
/// row the chip is on lights, and RUN lets it run.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Trainer {
    /// The movement it works on, if one is on the mat, and its model.
    movement: Option<Movement>,
    model: Option<Entity>,
    /// Whether the switches show the movement's memory yet. They are set
    /// to it once it comes, and write to it after.
    synced: bool,
    /// Seconds until the next tick.
    until_tick: f32,
    /// Instructions owed to the running chip.
    owed: f32,
}

impl Default for Trainer {
    fn default() -> Self {
        Self {
            movement: None,
            model: None,
            synced: false,
            until_tick: TICK,
            owed: 0.0,
        }
    }
}

/// What a control on the panel is for.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    /// The switch for one bit of one word.
    Bit {
        word: usize,
        bit: u8,
    },
    Run,
}

/// What a lamp on the panel shows.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Lamp {
    /// The chip is on this word.
    Row(usize),
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
                        Control::Bit { .. } => entity.insert(DipSwitch { on: false }),
                        Control::Run => entity.insert(Toggle { up: false }),
                    };
                }
                for (lamp, at) in layout::lamps() {
                    parent.spawn((
                        lamp,
                        Led {
                            color: GREEN,
                            on: false,
                        },
                        top(at),
                    ));
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

    /// Sets the switches to the memory of a movement that just came, and
    /// writes what they say into it after.
    fn wire(
        mut trainers: Query<(&mut Trainer, &Children)>,
        mut switches: Query<(&Control, &mut DipSwitch)>,
    ) {
        for (mut trainer, children) in &mut trainers {
            let trainer = &mut *trainer;
            let Some(movement) = trainer.movement.as_mut() else {
                continue;
            };
            let mut words = [0; WORDS];
            let mut parts = switches.iter_many_mut(children);
            while let Some(part) = parts.fetch_next() {
                let Ok((&Control::Bit { word, bit }, mut switch)) = part else {
                    continue;
                };
                let on = movement.rom().image().get(word) >> bit & 1 != 0;
                if !trainer.synced {
                    switch.set_if_neq(DipSwitch { on });
                } else if switch.on {
                    words[word] |= 1 << bit;
                }
            }
            if trainer.synced {
                for (address, word) in words.into_iter().enumerate() {
                    if movement.rom().image().get(address) != word {
                        // The memory of the first chip takes any word.
                        let _ = movement.rom_mut().program_word(address, word);
                    }
                }
            }
            trainer.synced = true;
        }
    }

    /// Ticks the crystal and runs the chip while RUN is up. Down, the chip
    /// waits at word 0.
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
            let running = switches
                .iter_many(children)
                .flatten()
                .any(|(control, toggle)| *control == Control::Run && toggle.up);
            if !running {
                movement.power_on();
                trainer.until_tick = TICK;
                trainer.owed = 0.0;
                continue;
            }
            trainer.until_tick -= dt;
            if trainer.until_tick <= 0.0 {
                trainer.until_tick += TICK;
                movement.tick();
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

    /// The lamp of the word the chip is on lights. The hand shows where the
    /// movement points, and ticks as it moves.
    fn show(
        trainers: Query<(&Trainer, &Children)>,
        children: Query<&Children>,
        mut lamps: Query<(&Lamp, &mut Led)>,
        mut hands: Query<(&mut Hand, &GlobalTransform)>,
        mut commands: Commands,
    ) {
        for (trainer, parts) in &trainers {
            let movement = trainer.movement.as_ref();
            for &part in parts {
                if let Ok((lamp, mut led)) = lamps.get_mut(part) {
                    let Lamp::Row(word) = *lamp;
                    let on =
                        movement.is_some_and(|movement| usize::from(movement.cpu().at()) == word);
                    if led.on != on {
                        led.on = on;
                    }
                }
            }
            let (Some(movement), Some(model)) = (movement, trainer.model) else {
                continue;
            };
            for descendant in children.iter_descendants(model) {
                if let Ok((mut hand, placed)) = hands.get_mut(descendant)
                    && hand.go_to(movement.hand())
                {
                    commands.trigger(PlayAt(Sound::Ratchet, placed.translation()));
                }
            }
        }
    }

    /// Works on `movement` from power on, shown by `model`.
    pub(crate) fn mount(&mut self, mut movement: Movement, model: Entity) {
        movement.power_on();
        self.movement = Some(movement);
        self.model = Some(model);
        self.synced = false;
        self.until_tick = TICK;
    }

    /// Lets go of the movement, if it works on one.
    pub(crate) fn take(&mut self) -> Option<Movement> {
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
                "panel-run",
                "click-datasheet",
                "click-flyer",
                "click-take-device",
                "click-take-papers",
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
                "tick-sheet-switches",
            ]
            .map(Message::new),
        );
    }
}
