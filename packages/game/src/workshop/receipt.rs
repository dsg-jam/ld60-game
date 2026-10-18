use bevy::prelude::*;
use ld60_levels::Level;

use super::counter::CounterLayout;
use super::pickup::{Earnings, Pickup};
use crate::audio::{Play, Sound};
use crate::locale::{Localized, Message};
use crate::parts::paper::Paper;
use crate::parts::screen::UiCanvas;
use crate::player::focus::{Focus, FocusOn, FocusTarget, View};

const SPOT: Vec2 = Vec2::new(-0.12, -0.8);
const YAW: f32 = 0.1;
const SHEET_SIZE: Vec2 = Vec2::new(0.12, 0.163);
const CANVAS_SIZE: UVec2 = UVec2::new(560, 760);
/// Lifts the slip onto the sticky notes there.
const SHEET_GAP: f32 = 0.0025;
const LABEL_WIDTH: f32 = 120.0;
const VALUE_WIDTH: f32 = 344.0;

pub(crate) struct ReceiptPlugin;

impl Plugin for ReceiptPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Receipt::on_add);
    }
}

/// The slip left on the counter after a pickup. It says whether the customer
/// paid and quotes their note if they did not.
#[derive(Component, Clone, Debug)]
#[require(Transform, Visibility)]
pub(crate) struct Receipt {
    job: String,
    customer: String,
    pickup: Pickup,
    earnings: Earnings,
}

impl Receipt {
    pub(crate) fn new(level: &Level, pickup: Pickup, earnings: Earnings) -> Self {
        Self {
            job: level.id.to_uppercase(),
            customer: level.customer.name.clone(),
            pickup,
            earnings,
        }
    }

    /// Replaces the slip of the last pickup and shows the new one.
    pub(crate) fn hand_out(self, old: impl IntoIterator<Item = Entity>, commands: &mut Commands) {
        for entity in old {
            commands.entity(entity).despawn();
        }
        match self.pickup {
            Pickup::Paid(_) => commands.trigger(Play(Sound::Chime)),
            Pickup::HandedBack { .. } | Pickup::Left(_) => commands.trigger(Play(Sound::Thunk)),
        }
        let receipt = commands
            .spawn((
                Name::new("receipt"),
                self,
                CounterLayout::new().place(SPOT, YAW),
                FocusTarget {
                    prompt: "click-receipt",
                    focus: Focus::Paper,
                    view: View::Local(Paper::reading_view(Self::sheet_center())),
                },
            ))
            .id();
        commands.trigger(FocusOn { entity: receipt });
    }

    fn sheet_center() -> Vec3 {
        Vec3::Y * SHEET_GAP
    }

    fn on_add(
        add: On<Add<Receipt>>,
        receipts: Query<&Receipt>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let Ok(receipt) = receipts.get(add.entity) else {
            return;
        };
        let canvas = UiCanvas::spawn(&mut commands, &mut images, CANVAS_SIZE, add.entity);
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
                .spawn(Paper::sheet(
                    &canvas,
                    SHEET_SIZE,
                    Self::sheet_center(),
                    &mut meshes,
                    &mut materials,
                ))
                .id();
        });
        commands
            .entity(add.entity)
            .insert(Paper::new(sheet, SHEET_SIZE, vec![page]));
    }

    fn write(&self, page: &mut ChildSpawnerCommands) {
        let (title, paid) = match &self.pickup {
            Pickup::Paid(amount) => ("receipt-title-paid", *amount),
            Pickup::HandedBack { .. } => ("receipt-title-handed-back", 0),
            Pickup::Left(_) => ("receipt-title-left", 0),
        };
        page.spawn(Paper::title(Message::new(title)));
        page.spawn(Paper::faded(Message::new("ticket-subtitle")));
        for (label, value) in [
            ("ticket-job", Localized::verbatim(&self.job)),
            ("ticket-customer", Localized::verbatim(&self.customer)),
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
        if let Some(note) = self.pickup.note() {
            page.spawn(Paper::heading(Message::new("receipt-note")));
            page.spawn(Paper::text(Message::new("paper-quote").with("text", note)));
        }
        page.spawn(Paper::heading(Message::new("receipt-till")));
        page.spawn(Paper::text(
            Message::new("receipt-till-total").with("amount", self.earnings.0),
        ));
    }
}
