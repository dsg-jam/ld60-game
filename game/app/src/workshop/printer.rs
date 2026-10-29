use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_game_audio::{Play, Sound};
use ld60_game_parts::paper::Sheltered;
use ld60_game_player::focus::{Clickable, Focus, FocusTarget};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use super::receipt::{self, Receipt};
use super::ticket::JobTicket;

/// Behind the job ticket, see [`CounterLayout::place`].
///
/// [`CounterLayout::place`]: ld60_game_core::layout::counter::CounterLayout::place
pub(super) const SPOT: Vec2 = Vec2::new(-0.85, 0.02);
pub(super) const YAW: f32 = 0.12;
pub(super) const BODY: Vec3 = Vec3::new(0.2, 0.06, 0.1);
/// The dark base the housing sits on, on rubber feet.
const BASE: Vec3 = Vec3::new(0.2, 0.017, 0.1);
const FOOT: Vec3 = Vec3::new(0.012, 0.003, 0.012);
const HOUSING: Vec3 = Vec3::new(0.194, 0.04, 0.096);
/// The cover over the paper roll, a little raised, with a seam around it.
const LID: Vec3 = Vec3::new(0.14, 0.003, 0.06);
const SEAM: f32 = 0.0015;
const SLOT: Vec3 = Vec3::new(0.17, 0.001, 0.006);
/// The metal edge in front of the slot that the paper tears against.
const TEAR_BAR: Vec3 = Vec3::new(0.172, 0.003, 0.004);
/// The slot sits towards the back, so the paper leaning out of it stays in
/// the printer below the slot.
const SLOT_BACK: f32 = 0.03;
const BUTTON: Vec3 = Vec3::new(0.012, 0.004, 0.012);
const LED: Vec3 = Vec3::new(0.005, 0.002, 0.005);
const LABEL: Vec3 = Vec3::new(0.06, 0.012, 0.001);
const VENT: Vec3 = Vec3::new(0.001, 0.003, 0.05);
const BEIGE: Color = Color::srgb_u8(0xd6, 0xcf, 0xbc);
const LID_BEIGE: Color = Color::srgb_u8(0xe0, 0xda, 0xc8);
const DARK: Color = Color::srgb_u8(0x2c, 0x2c, 0x2e);
const SLOT_DARK: Color = Color::srgb_u8(0x10, 0x10, 0x12);
const METAL: Color = Color::srgb_u8(0xb8, 0xbc, 0xc0);
const LED_GREEN: Color = Color::srgb_u8(0x50, 0xe0, 0x70);
const LED_AMBER: Color = Color::srgb_u8(0xe0, 0x9a, 0x30);
/// How far the paper leans back from upright as it comes out, in radians.
/// It faces up into the light of the lamp.
const LEAN: f32 = 0.7;

const PRINT_TIME: f32 = 2.0;
/// Seconds between two steps of the print head, each one a tick.
const STEP_TIME: f32 = 0.08;
/// How much of the sheet comes out of the slot.
const PRINTED: f32 = 0.85;
const TEAR_TIME: f32 = 0.7;
/// How high a torn sheet arcs on its way to its place.
const ARC: f32 = 0.06;

pub(crate) struct PrinterPlugin;

impl Plugin for PrinterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Printing::animate)
            .add_observer(Printer::on_add)
            .add_observer(Printing::hide)
            .add_observer(Printing::tear_on_press);
    }
}

/// The printer on the counter. Receipts and the tickets of new jobs come
/// out of it.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Printer;

impl Printer {
    fn on_add(add: On<Add<Printer>>, mut commands: Commands) {
        let base = Block::new(BASE).translated(Vec3::Y * FOOT.y);
        let housing = Block::new(HOUSING).on_top_of(&base);
        let top = |size: Vec3, x: f32, z: f32| {
            Block::new(size)
                .on_top_of(&housing)
                .translated(Vec3::new(x, 0.0, z))
        };
        // The cover takes the left of the top, the controls the right.
        let lid_x = -(HOUSING.x - LID.x) / 2.0 + 0.008;
        let lid_z = 0.012;
        let seam = top(
            LID + Vec3::new(2.0 * SEAM, -LID.y / 2.0, 2.0 * SEAM),
            lid_x,
            lid_z,
        );
        let lid = top(LID, lid_x, lid_z);
        let slot = top(SLOT, 0.0, -SLOT_BACK);
        let tear_bar = top(TEAR_BAR, 0.0, -SLOT_BACK + 0.006);
        let controls = HOUSING.x / 2.0 - 0.016;
        let button = top(BUTTON, controls, 0.03);
        let leds = [(0.008, LED_GREEN), (-0.004, LED_AMBER)]
            .map(|(z, color)| (top(LED, controls, z), color));
        let label = Block::new(LABEL).translated(Vec3::new(
            -0.05,
            housing.center.y - LABEL.y / 2.0,
            (HOUSING.z + LABEL.z) / 2.0,
        ));
        let vents = [-0.008, 0.0, 0.008].map(|y| {
            Block::new(VENT).translated(Vec3::new(
                (HOUSING.x + VENT.x) / 2.0,
                housing.center.y + y,
                0.0,
            ))
        });
        let feet = [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)].map(|(x, z)| {
            Block::new(FOOT).translated(Vec3::new(
                x * (BASE.x / 2.0 - FOOT.x),
                0.0,
                z * (BASE.z / 2.0 - FOOT.z),
            ))
        });
        commands.entity(add.entity).with_children(|printer| {
            printer.spawn((Shape::cuboid(&base, DARK), Finish::Plastic));
            printer.spawn((Shape::cuboid(&housing, BEIGE), Finish::Plastic));
            printer.spawn((Shape::cuboid(&seam, SLOT_DARK), Finish::Plastic));
            printer.spawn((Shape::cuboid(&lid, LID_BEIGE), Finish::Plastic));
            printer.spawn((Shape::cuboid(&slot, SLOT_DARK), Finish::Plastic));
            printer.spawn((Shape::cuboid(&tear_bar, METAL), Finish::Plated));
            printer.spawn((Shape::cylinder(&button, DARK), Finish::Grip));
            for (led, color) in leds {
                printer.spawn((Shape::cylinder(&led, color), Finish::Plastic));
            }
            printer.spawn((Shape::cuboid(&label, METAL), Finish::Plated));
            for vent in vents {
                printer.spawn((Shape::cuboid(&vent, SLOT_DARK), Finish::Plastic));
            }
            for foot in feet {
                printer.spawn((Shape::cylinder(&foot, DARK), Finish::Fabric));
            }
        });
    }

    /// The middle of the slot, with +Y out of it.
    fn slot(at: &Transform) -> Transform {
        *at * Transform::from_xyz(0.0, BODY.y, -SLOT_BACK)
    }
}

/// A sheet coming out of the [`Printer`], then going to its place. A receipt
/// goes there by itself. A job ticket waits for a click that tears it off.
#[derive(Component, Debug)]
pub(crate) struct Printing {
    elapsed: f32,
    delay: f32,
    /// Where it rests once it is in its place.
    rest: Transform,
    /// The middle of its sheet, in its own space.
    sheet: Vec3,
    length: f32,
    /// Whether it tears off by itself once it is out.
    tears_off: bool,
    /// What it sounds like as it starts to print, and as it lands.
    sounds: (Option<Sound>, Option<Sound>),
    /// How long ago it was torn off, once it is.
    torn: Option<f32>,
    /// Taken off while it prints, so a click tears it off instead.
    target: Option<FocusTarget>,
}

impl Printing {
    /// The ticket of a new job, after `delay` seconds.
    pub(crate) fn ticket(rest: Transform, delay: f32) -> Self {
        Self {
            sounds: (None, None),
            ..Self::new(
                rest,
                JobTicket::sheet_center(),
                JobTicket::SHEET_SIZE.y,
                delay,
            )
        }
    }

    /// A receipt after `delay` seconds. The printer chirps as it has it, and
    /// it lands with `landed`.
    pub(crate) fn receipt(rest: Transform, delay: f32, landed: Sound) -> Self {
        Self {
            tears_off: true,
            sounds: (Some(Sound::Chirp), Some(landed)),
            ..Self::new(rest, Receipt::sheet_center(), receipt::SHEET_SIZE.y, delay)
        }
    }

    fn new(rest: Transform, sheet: Vec3, length: f32, delay: f32) -> Self {
        Self {
            elapsed: 0.0,
            delay,
            rest,
            sheet,
            length,
            tears_off: false,
            sounds: (None, None),
            torn: None,
            target: None,
        }
    }

    fn printed(&self) -> bool {
        self.elapsed >= self.delay + PRINT_TIME
    }

    /// Whether it is out and waits to be torn off.
    pub(crate) fn waiting(&self) -> bool {
        self.printed() && self.torn.is_none() && !self.tears_off
    }

    /// The pose with `out` meters of the sheet out of `slot`, leaning back.
    fn out_of(&self, slot: Transform, out: f32) -> Transform {
        let lean = slot.rotation * Quat::from_rotation_x(-LEAN);
        let at = slot.translation + lean * Vec3::Y * (out - self.length / 2.0);
        // The sheet of a paper lies flat in its own space, so it stands up
        // turned by a quarter.
        Transform::from_translation(at).with_rotation(lean * Quat::from_rotation_x(FRAC_PI_2))
            * Transform::from_translation(-self.sheet)
    }

    /// A sheet only shows once it comes out of the slot, not where it will
    /// lie and not inside the printer while it waits. It keeps still in the
    /// air until it lies where it goes.
    fn hide(
        insert: On<Insert<Printing>>,
        mut visibilities: Query<&mut Visibility>,
        mut commands: Commands,
    ) {
        if let Ok(mut visibility) = visibilities.get_mut(insert.entity) {
            *visibility = Visibility::Hidden;
        }
        commands.entity(insert.entity).insert(Sheltered);
    }

    fn animate(
        time: Res<Time>,
        printers: Query<&Transform, (With<Printer>, Without<Printing>)>,
        mut sheets: Query<(
            Entity,
            &mut Printing,
            &mut Transform,
            &mut Visibility,
            Option<&FocusTarget>,
        )>,
        mut commands: Commands,
    ) {
        let Some(slot) = printers.iter().next().map(Printer::slot) else {
            return;
        };
        let dt = time.delta_secs();
        for (entity, mut printing, mut transform, mut visibility, target) in &mut sheets {
            if let Some(&target) = target {
                printing.target = Some(target);
                let mut entity = commands.entity(entity);
                entity.remove::<(FocusTarget, Clickable)>();
                if !printing.tears_off {
                    entity.insert(Clickable {
                        prompt: "click-tear-ticket",
                    });
                }
            }
            let before = printing.elapsed;
            printing.elapsed += dt;
            let delay = printing.delay;
            if let (true, Some(sound)) = (
                before < delay && printing.elapsed >= delay,
                printing.sounds.0,
            ) {
                commands.trigger(Play(sound));
            }
            visibility.set_if_neq(if printing.elapsed < delay {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            });
            let t = ((printing.elapsed - delay) / PRINT_TIME).clamp(0.0, 1.0);
            let printed = printing.out_of(slot, PRINTED * printing.length * t);
            let steps = |elapsed: f32| ((elapsed - delay) / STEP_TIME).floor();
            if t > 0.0 && t < 1.0 && steps(printing.elapsed) > steps(before) {
                commands.trigger(Play(Sound::Ratchet));
            }
            if printing.tears_off && printing.printed() && printing.torn.is_none() {
                printing.torn = Some(0.0);
                commands.trigger(Play(Sound::PageTurn));
            }
            let Some(torn) = printing.torn.as_mut() else {
                *transform = printed;
                continue;
            };
            *torn += dt;
            let t = (*torn / TEAR_TIME).min(1.0);
            let eased = t * t * (3.0 - 2.0 * t);
            *transform = Transform::from_translation(
                printed.translation.lerp(printing.rest.translation, eased)
                    + Vec3::Y * ARC * (PI * eased).sin(),
            )
            .with_rotation(printed.rotation.slerp(printing.rest.rotation, eased));
            if t >= 1.0 {
                *transform = printing.rest;
                if let Some(sound) = printing.sounds.1 {
                    commands.trigger(Play(sound));
                }
                let mut entity = commands.entity(entity);
                entity.remove::<(Printing, Sheltered)>();
                if let Some(target) = printing.target {
                    entity.insert(target);
                }
            }
        }
    }

    /// A click on the printed ticket tears it off.
    fn tear_on_press(
        mut press: On<PointerPress>,
        state: Res<State<Focus>>,
        mut sheets: Query<&mut Printing>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || *state.get() != Focus::Desk {
            return;
        }
        let Ok(mut printing) = sheets.get_mut(press.entity) else {
            return;
        };
        press.propagate(false);
        if printing.waiting() {
            printing.torn = Some(0.0);
            commands.trigger(Play(Sound::PageTurn));
        }
    }
}
