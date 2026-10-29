use bevy::prelude::*;
use ld60_game_core::audio::{Play, Sound};
use ld60_game_core::layout::counter::CounterLayout;
use ld60_game_core::parts::paper::{Flutter, Paper, Press, Stock};
use ld60_game_core::parts::screen::UiCanvas;
use ld60_game_core::player::focus::{Focus, FocusOn, FocusTarget, Focused, View};
use ld60_game_core::state::AppState;
use ld60_locale::{Localized, Message};

use super::chore::PlanetSet;
use super::run::{Leftover, Rent, Run};

/// Next to the receipt, a little closer to the player.
pub(super) const SPOT: Vec2 = Vec2::new(-0.6, -0.06);
pub(super) const YAW: f32 = -0.12;
pub(super) const SHEET_SIZE: Vec2 = Vec2::new(0.12, 0.12);
const CANVAS_SIZE: UVec2 = UVec2::new(560, 560);
/// Lifts the note onto the receipt if they overlap.
const SHEET_GAP: f32 = 0.004;

pub(crate) struct RentPlugin;

impl Plugin for RentPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(RentNote::collect)
            .add_observer(RentNote::on_add)
            .add_systems(
                Update,
                RentNote::close_shop.run_if(in_state(AppState::Playing)),
            );
    }
}

/// The landlord's note, left on the counter when the planet sets.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Visibility, Leftover)]
pub(crate) struct RentNote(Rent);

impl RentNote {
    pub(crate) fn rent(&self) -> Rent {
        self.0
    }

    /// The landlord takes the rent and leaves a note about it.
    pub(super) fn collect(
        _: On<PlanetSet>,
        mut run: ResMut<Run>,
        notes: Query<Entity, With<RentNote>>,
        mut commands: Commands,
    ) {
        let rent = run.pay_rent();
        for note in &notes {
            commands.entity(note).despawn();
        }
        commands.trigger(Play(Sound::Thunk));
        let note = commands
            .spawn((
                Name::new("rent note"),
                Self(rent),
                CounterLayout::new().place(SPOT, YAW),
                FocusTarget {
                    prompt: "click-rent",
                    focus: Focus::Paper,
                    view: View::Local(Paper::reading_view(Self::sheet_center())),
                },
            ))
            .id();
        commands.trigger(FocusOn { entity: note });
    }

    /// If the till was short, the shop closes once the player puts the
    /// landlord's note down.
    fn close_shop(
        notes: Query<(Entity, &RentNote)>,
        focused: Res<Focused>,
        mut next: ResMut<NextState<AppState>>,
    ) {
        let read = notes.iter().any(|(entity, note)| {
            matches!(note.0, Rent::Short { .. }) && focused.get() != Some(entity)
        });
        if read {
            next.set(AppState::Closed);
        }
    }

    fn sheet_center() -> Vec3 {
        Vec3::Y * SHEET_GAP
    }

    fn on_add(
        add: On<Add<RentNote>>,
        notes: Query<&RentNote>,
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
    ) {
        let Ok(&note) = notes.get(add.entity) else {
            return;
        };
        let canvas = UiCanvas::spawn_print(&mut commands, &mut images, CANVAS_SIZE, add.entity);
        let page = Paper::spawn_page(
            &mut commands,
            &canvas,
            (1, 1),
            Message::new("rent-footer"),
            |page| note.write(page),
        );
        // The landlord runs his notes off on a spirit duplicator.
        commands
            .entity(page)
            .insert(Press::Spirit.run("rent").on(Stock::NEWSPRINT));
        let document = ("rent", note.0);
        let mut sheet = Entity::PLACEHOLDER;
        commands.entity(add.entity).with_children(|note| {
            sheet = note
                .spawn((
                    Paper::sheet(
                        &canvas,
                        document,
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

    fn write(self, page: &mut ChildSpawnerCommands) {
        page.spawn(Paper::title(Message::new("rent-title")));
        let text = match self.0 {
            Rent::Paid { amount, next, .. } => Message::new("rent-paid")
                .with("amount", amount)
                .with("next", next),
            Rent::Owned => Message::new("rent-owned"),
            Rent::Short { amount, till } => Message::new("rent-short")
                .with("amount", amount)
                .with("till", till),
        };
        page.spawn(Paper::text(text));
        if let Rent::Paid {
            deed: Some(price), ..
        } = self.0
        {
            page.spawn(Paper::text(Message::new("rent-offer").with("price", price)));
        }
        page.spawn(Paper::faded(Localized::from(Message::new("rent-signed"))));
    }
}

#[cfg(test)]
mod tests {
    use ld60_locale::Locale;

    use super::*;

    #[test]
    fn every_note_formats() {
        let locale = Locale::on_disk();
        for rent in [
            Rent::Paid {
                amount: 10,
                next: 16,
                deed: None,
            },
            Rent::Paid {
                amount: 10,
                next: 16,
                deed: Some(150),
            },
            Rent::Owned,
            Rent::Short {
                amount: 10,
                till: 1,
            },
        ] {
            locale.assert_formats(Localized::spawned_by(|page| RentNote(rent).write(page)));
        }
        locale.assert_formats([Message::new("rent-footer"), Message::new("click-rent")]);
    }

    #[test]
    fn a_short_till_closes_the_shop_after_the_note() {
        let mut world = World::new();
        world.init_resource::<Run>();
        world.init_resource::<Focused>();
        world.init_resource::<NextState<Focus>>();
        world.init_resource::<NextState<AppState>>();
        world.add_observer(RentNote::collect);
        world.add_observer(FocusOn::apply);
        world.trigger(PlanetSet);
        world.flush();
        let note = world
            .query_filtered::<Entity, With<RentNote>>()
            .single(&world);
        assert_eq!(world.resource::<Focused>().get(), note.ok());
        world.run_system_cached(RentNote::close_shop).unwrap();
        assert!(matches!(
            *world.resource::<NextState<AppState>>(),
            NextState::Unchanged
        ));
        world.resource_scope(|world, mut focus: Mut<NextState<Focus>>| {
            world.resource_mut::<Focused>().leave(&mut focus);
        });
        world.run_system_cached(RentNote::close_shop).unwrap();
        assert!(matches!(
            *world.resource::<NextState<AppState>>(),
            NextState::Pending(AppState::Closed)
        ));
    }
}
