use bevy::{
    ecs::system::SystemParam,
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};
use ld60_game_audio::{Play, Sound};
use ld60_game_player::focus::Focused;

use super::{Board, Casing, CoverPlate, Header, Hole, Screw};
use crate::{Connected, Holdable, Turned};

/// What a click on a part of a device needs to find the device.
#[derive(SystemParam)]
pub(super) struct PartClick<'w, 's> {
    focused: Res<'w, Focused>,
    turned: Res<'w, Turned>,
    parents: Query<'w, 's, &'static ChildOf>,
    casings: Query<
        'w,
        's,
        (
            Entity,
            &'static mut Casing,
            &'static mut Holdable,
            Has<Connected>,
        ),
    >,
}

impl PartClick<'_, '_> {
    /// The device whose part `press` clicked, with whether it is plugged
    /// in, if the player holds it and the click was a plain primary click.
    /// The click goes no further then.
    fn device(
        &mut self,
        press: &mut On<PointerClick>,
    ) -> Option<(Entity, Mut<'_, Casing>, Mut<'_, Holdable>, bool)> {
        if press.button != PointerButton::Primary
            || !self.turned.clicked()
            || !self.focused.holds(press.entity, &self.parents)
        {
            return None;
        }
        let device = self
            .parents
            .iter_ancestors(press.entity)
            .find(|ancestor| self.casings.contains(*ancestor))?;
        let (_, casing, holdable, plugged) = self.casings.get_mut(device).ok()?;
        press.propagate(false);
        Some((device, casing, holdable, plugged))
    }
}

impl Screw {
    pub(super) fn press(
        mut press: On<PointerClick>,
        screws: Query<&Screw>,
        mut click: PartClick,
        mut commands: Commands,
    ) {
        let Ok(screw) = screws.get(press.entity) else {
            return;
        };
        let Some((_, mut casing, ..)) = click.device(&mut press) else {
            return;
        };
        let out = casing.loose.get(screw.0) == Some(&false) && casing.turn_screw(screw.0);
        // The screwdriver plays the turning, see `Casing::travel`.
        if !out {
            commands.trigger(Play(Sound::Refuse));
        }
    }
}

impl Hole {
    /// Puts the screw back in from beside the device.
    pub(super) fn press(
        mut press: On<PointerClick>,
        holes: Query<&Hole>,
        mut click: PartClick,
        mut commands: Commands,
    ) {
        let Ok(hole) = holes.get(press.entity) else {
            return;
        };
        let Some((_, mut casing, ..)) = click.device(&mut press) else {
            return;
        };
        let back = casing.loose.get(hole.0) == Some(&true) && casing.turn_screw(hole.0);
        if !back {
            commands.trigger(Play(Sound::Refuse));
        }
    }
}

impl CoverPlate {
    pub(super) fn press(
        mut press: On<PointerClick>,
        plates: Query<(), With<CoverPlate>>,
        mut click: PartClick,
        mut commands: Commands,
    ) {
        if !plates.contains(press.entity) {
            return;
        }
        let Some((_, mut casing, mut holdable, plugged)) = click.device(&mut press) else {
            return;
        };
        // The cover on the mat goes back on, like a click on the board.
        let sound = if casing.open {
            if casing.close_cover(plugged) {
                holdable.turn_over(casing.bounds.size.y);
                Sound::CoverOn
            } else {
                Sound::Refuse
            }
        } else if casing.lift_cover() {
            holdable.turn_over(casing.bounds.size.y);
            Sound::CoverOff
        } else {
            Sound::Refuse
        };
        commands.trigger(Play(sound));
    }
}

impl Board {
    pub(super) fn press(
        mut press: On<PointerClick>,
        boards: Query<(), With<Board>>,
        mut click: PartClick,
        mut commands: Commands,
    ) {
        if !boards.contains(press.entity) {
            return;
        }
        let Some((_, mut casing, mut holdable, plugged)) = click.device(&mut press) else {
            return;
        };
        let sound = if casing.close_cover(plugged) {
            holdable.turn_over(casing.bounds.size.y);
            Sound::CoverOn
        } else {
            Sound::Refuse
        };
        commands.trigger(Play(sound));
    }
}

impl Header {
    /// Plugs the debug cable in, or pulls it out.
    pub(super) fn press(
        mut press: On<PointerClick>,
        headers: Query<(), With<Header>>,
        mut click: PartClick,
        mut commands: Commands,
    ) {
        if !headers.contains(press.entity) {
            return;
        }
        let Some((device, casing, _, plugged)) = click.device(&mut press) else {
            return;
        };
        if !casing.open {
            commands.trigger(Play(Sound::Refuse));
        } else if plugged {
            commands.entity(device).remove::<Connected>();
            commands.trigger(Play(Sound::PlugOut));
        } else {
            commands.entity(device).insert(Connected);
            commands.trigger(Play(Sound::PlugIn));
        }
    }
}
