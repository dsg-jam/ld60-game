use bevy::prelude::*;
use ld60_game_core::audio::Sound;
use ld60_game_core::layout::counter::CounterLayout;
use ld60_game_core::parts::paper::{Flutter, Paper};
use ld60_game_core::parts::screen::UiCanvas;
use ld60_game_core::player::focus::{Focus, FocusTarget, View};
use ld60_levels::Level;
use ld60_locale::{Localized, Message};

use super::bin::Binned;
use super::levels::Levels;
use super::pickup::{Payment, Pickup, TRIES};
use super::printer::Printing;
use super::run::Leftover;
use super::ticket::JobTicket;

pub(super) const SPOT: Vec2 = Vec2::new(-0.41, -0.01);
pub(super) const YAW: f32 = 0.1;
pub(super) const SHEET_SIZE: Vec2 = Vec2::new(0.12, 0.163);
const CANVAS_SIZE: UVec2 = UVec2::new(560, 760);
/// Lifts the slip onto the sticky notes there.
const SHEET_GAP: f32 = 0.0025;
const LABEL_WIDTH: f32 = 120.0;
const VALUE_WIDTH: f32 = 344.0;
const MEASURE_WIDTH: f32 = 172.0;

pub(crate) struct ReceiptPlugin;

impl Plugin for ReceiptPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Receipt::on_add);
    }
}

/// The slip left on the counter after a pickup. It says what the customer
/// paid, what the bench measured on a working device and what the customer
/// wrote.
#[derive(Component, Clone, Debug)]
#[require(Transform, Visibility, Leftover)]
pub(crate) struct Receipt {
    job: Localized,
    customer: Message,
    pickup: Pickup,
    /// Money in the till after the pickup.
    till: u32,
    /// Picks one of the ways the customer puts their note.
    pick: u32,
}

impl Receipt {
    pub(crate) fn new(level: &Level, pickup: Pickup, till: u32) -> Self {
        Self {
            job: JobTicket::job(level),
            customer: Levels::text(level, Level::CUSTOMER),
            pickup,
            till,
            pick: level.serial().bytes().map(u32::from).sum(),
        }
    }

    /// Replaces the slip of the last pickup with the new one, which prints
    /// after `delay` seconds.
    pub(crate) fn hand_out(
        self,
        old: impl IntoIterator<Item = Entity>,
        delay: f32,
        commands: &mut Commands,
    ) {
        // The old slip goes into the bin.
        for entity in old {
            commands.entity(entity).remove::<Receipt>();
            Binned::send(entity, commands);
        }
        let landed = match self.pickup {
            Pickup::Paid(_) => Sound::Chime,
            Pickup::HandedBack { .. } | Pickup::Left(_) => Sound::Thunk,
        };
        // It comes out of the printer, then lies down at its spot.
        let rest = CounterLayout::new().place(SPOT, YAW);
        commands.spawn((
            Name::new("receipt"),
            self,
            rest,
            Printing::receipt(rest, delay, landed),
            FocusTarget {
                prompt: "click-receipt",
                focus: Focus::Paper,
                view: View::Local(Paper::reading_view(Self::sheet_center())),
            },
        ));
    }

    pub(super) fn sheet_center() -> Vec3 {
        Vec3::Y * SHEET_GAP
    }

    fn on_add(
        add: On<Add<Receipt>>,
        receipts: Query<&Receipt>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
    ) {
        let Ok(receipt) = receipts.get(add.entity) else {
            return;
        };
        let canvas = UiCanvas::spawn_print(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let page = Paper::spawn_page(
            &mut commands,
            &canvas,
            (1, 1),
            Message::new("ticket-footer"),
            |page| {
                receipt.write(page);
            },
        );
        let mut sheet = Entity::PLACEHOLDER;
        commands.entity(add.entity).with_children(|slip| {
            sheet = slip
                .spawn((
                    Paper::sheet(
                        &canvas,
                        ("receipt", receipt.till),
                        SHEET_SIZE,
                        Self::sheet_center(),
                        &mut meshes,
                    ),
                    Flutter::default(),
                ))
                .id();
        });
        commands
            .entity(add.entity)
            .insert(Paper::new(sheet, SHEET_SIZE, vec![page]));
    }

    fn write(&self, page: &mut ChildSpawnerCommands) {
        let (title, paid) = match &self.pickup {
            Pickup::Paid(payment) => ("receipt-title-paid", payment.fee),
            Pickup::HandedBack { .. } => ("receipt-title-handed-back", 0),
            Pickup::Left(_) => ("receipt-title-left", 0),
        };
        page.spawn(Paper::title(Message::new(title)));
        page.spawn(Paper::faded(Message::new("ticket-subtitle")));
        for (label, value) in [
            ("ticket-job", self.job.clone()),
            ("ticket-customer", self.customer.clone().into()),
            (
                "receipt-paid",
                Message::new("receipt-amount").with("amount", paid).into(),
            ),
        ] {
            page.spawn(Paper::row([
                (Message::new(label).into(), LABEL_WIDTH),
                (value, VALUE_WIDTH),
            ]));
        }
        if let Pickup::Paid(payment) = &self.pickup {
            for (label, amount) in [
                ("receipt-tip", payment.tip()),
                ("receipt-gift", payment.gift()),
            ] {
                if amount > 0 {
                    page.spawn(Paper::row([
                        (Message::new(label).into(), LABEL_WIDTH),
                        (
                            Message::new("receipt-amount").with("amount", amount).into(),
                            VALUE_WIDTH,
                        ),
                    ]));
                }
            }
        }
        if let Pickup::Paid(payment) = &self.pickup {
            Self::write_measure(page, payment);
        }
        page.spawn(Paper::heading(Message::new("receipt-note-heading")));
        page.spawn(Paper::text(
            Message::new("paper-quote").with("text", self.pickup.note(self.pick)),
        ));
        page.spawn(Paper::heading(Message::new("receipt-till")));
        page.spawn(Paper::text(
            Message::new("receipt-till-total").with("amount", self.till),
        ));
    }
}

impl Receipt {
    /// The bench log: the repair next to the factory fix, and the tries.
    fn write_measure(page: &mut ChildSpawnerCommands, payment: &Payment) {
        page.spawn(Paper::heading(Message::new("receipt-bench")));
        page.spawn(Paper::row([
            (Message::new("receipt-measure").into(), LABEL_WIDTH),
            (Message::new("receipt-this-fix").into(), MEASURE_WIDTH),
            (Message::new("receipt-factory-fix").into(), MEASURE_WIDTH),
        ]));
        let par = payment.par;
        let rows = [
            (
                "receipt-words",
                "receipt-words-met",
                payment.measure.words,
                par.map(|par| par.words),
                payment.neat(),
            ),
            (
                "receipt-cycles",
                "receipt-cycles-met",
                payment.measure.cycles,
                par.map(|par| par.cycles),
                payment.quick(),
            ),
        ];
        for (label, met_id, mine, par, met) in rows {
            let id = if met { met_id } else { "receipt-value" };
            let par = par.map_or_else(
                || Message::new("receipt-unknown"),
                |par| Message::new("receipt-value").with("value", par),
            );
            page.spawn(Paper::row([
                (Message::new(label).into(), LABEL_WIDTH),
                (Message::new(id).with("value", mine).into(), MEASURE_WIDTH),
                (par.into(), MEASURE_WIDTH),
            ]));
        }
        page.spawn(Paper::row([
            (Message::new("receipt-tries").into(), LABEL_WIDTH),
            (
                Message::new("receipt-tries-used")
                    .with("tries", payment.returns + 1)
                    .with("of", TRIES)
                    .into(),
                VALUE_WIDTH,
            ),
        ]));
    }
}

#[cfg(test)]
mod tests {
    use ld60_locale::Locale;

    use super::*;
    use crate::workshop::levels::Levels;
    use crate::workshop::pickup::Complaint;

    #[test]
    fn every_receipt_formats() {
        let locale = Locale::on_disk();
        let level = Levels::on_disk().levels()[0].load().unwrap();
        let mut late = Payment::tipped(10);
        late.returns = 2;
        late.measure.words += 1;
        let mut slow = Payment::tipped(10);
        slow.measure.cycles += 1;
        let mut unknown = Payment::example(10);
        unknown.par = None;
        let mut pickups = [
            Payment::example(10),
            Payment::tipped(10),
            late,
            slow,
            unknown,
        ]
        .map(Pickup::Paid)
        .to_vec();
        for complaint in Complaint::examples() {
            pickups.push(Pickup::HandedBack {
                complaint: complaint.clone(),
                tries_left: 2,
            });
            pickups.push(Pickup::Left(complaint));
        }
        for pickup in pickups {
            for pick in 0..3 {
                let mut receipt = Receipt::new(&level, pickup.clone(), 10);
                receipt.pick = pick;
                locale.assert_formats(Localized::spawned_by(|page| receipt.write(page)));
            }
        }
    }
}
