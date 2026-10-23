use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_core::audio::{Play, Sound};
use ld60_core::player::focus::{Clickable, Focused};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use super::board::BoardLayout;
use super::{Connected, Holdable};

/// Thickness of the back cover. The board and the header fit inside it.
pub(super) const COVER: f32 = 0.005;
const SCREW: Vec3 = Vec3::new(0.0055, 0.0014, 0.0055);
/// How far a loose screw stands out of its hole.
const SCREW_OUT: f32 = 0.004;
const SCREW_METAL: Color = Color::srgb_u8(0xc8, 0xca, 0xce);

/// The parts of a casing whose click prompt changes.
type Prompted<'a> = (
    &'a mut Clickable,
    Option<&'a Screw>,
    Has<CoverPlate>,
    Has<Header>,
);

pub(super) struct CasingPlugin;

impl Plugin for CasingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Casing::show)
            .add_observer(Screw::press)
            .add_observer(CoverPlate::press)
            .add_observer(Board::press)
            .add_observer(Header::press);
    }
}

/// The back cover of a device and its screws. The debug header on the board
/// is under the cover.
#[derive(Component, Debug)]
pub(crate) struct Casing {
    /// Whether each screw is out.
    loose: Vec<bool>,
    open: bool,
    /// The box around the device, from the cover to the tops of its keys.
    /// Open, it rests on its keys.
    bounds: Block,
}

/// Where the screws and the header of a device sit on its underside, see
/// [`Block::top_spot`].
pub(super) struct CasingSpots {
    pub(super) screws: &'static [Vec2],
    pub(super) header: Vec2,
}

/// Holds the cover, its screws and the sticker, so they come off together.
#[derive(Component, Debug)]
struct Cover;

#[derive(Component, Debug)]
struct CoverPlate;

#[derive(Component, Debug)]
struct Screw {
    index: usize,
    /// Height of the screw while it is in.
    rest: f32,
}

#[derive(Component, Debug)]
struct Board;

/// Where the debug cable plugs in.
#[derive(Component, Debug)]
pub(crate) struct Header;

impl Casing {
    fn new(screws: usize, bounds: Block) -> Self {
        Self {
            loose: vec![false; screws],
            open: false,
            bounds,
        }
    }

    /// The box around the device, in its own space.
    pub(crate) fn bounds(&self) -> &Block {
        &self.bounds
    }

    /// Whether the cover is on and all screws are in, as the customer
    /// brought it.
    pub(crate) fn sealed(&self) -> bool {
        !self.open && !self.loose.contains(&true)
    }

    /// Spawns the cover with `sticker` on it into `device`, below `shell`,
    /// and the board of `layout` with `face` and its parts under the cover.
    /// Returns the casing of a device `height` tall.
    #[expect(clippy::too_many_arguments, reason = "the parts of a casing")]
    pub(super) fn spawn(
        device: &mut ChildSpawnerCommands,
        shell: &Block,
        color: Color,
        spots: &CasingSpots,
        sticker: impl Bundle,
        layout: &BoardLayout,
        face: impl Bundle,
        height: f32,
    ) -> Self {
        let plate = Block::new(shell.size.with_y(COVER));
        let underside = |spot: Vec2| plate.top_spot(spot).with_y(0.0);
        device
            .spawn((Cover, Transform::IDENTITY, Visibility::Inherited))
            .with_children(|cover| {
                cover.spawn((
                    CoverPlate,
                    Clickable {
                        prompt: "click-cover-screwed",
                    },
                    Shape::cuboid(&plate, color),
                    Finish::Plastic,
                ));
                for (index, &spot) in spots.screws.iter().enumerate() {
                    let head = Block::new(SCREW).translated(underside(spot) - Vec3::Y * SCREW.y);
                    cover.spawn((
                        Screw {
                            index,
                            rest: head.center.y,
                        },
                        Clickable {
                            prompt: "click-unscrew",
                        },
                        Shape::cylinder(&head, SCREW_METAL),
                        Finish::Metal,
                    ));
                }
                cover.spawn(sticker);
            });
        let board = || {
            (
                Board,
                Clickable {
                    prompt: "click-cover-on",
                },
            )
        };
        device.spawn((
            board(),
            Shape::cuboid(&layout.board, BoardLayout::edge()),
            Finish::Plastic,
        ));
        device.spawn((board(), face));
        device.spawn((
            Header,
            Clickable {
                prompt: "click-plug-in",
            },
            Shape::cuboid(&layout.header, BoardLayout::header_color()),
            Finish::Plastic,
        ));
        for (part, color) in layout.parts() {
            device.spawn((Shape::cuboid(part, color), Finish::Plastic));
        }
        Self::new(spots.screws.len(), Block::new(shell.size.with_y(height)))
    }

    /// Turns a screw in or out. Screws only turn while the cover is on.
    fn turn_screw(&mut self, index: usize) -> bool {
        match self.loose.get_mut(index) {
            Some(loose) if !self.open => {
                *loose = !*loose;
                true
            }
            _ => false,
        }
    }

    /// Takes off the cover once all screws are out.
    fn lift_cover(&mut self) -> bool {
        let lifted = !self.open && self.loose.iter().all(|loose| *loose);
        self.open |= lifted;
        lifted
    }

    /// Puts the cover back on, unless the cable is plugged in.
    fn close_cover(&mut self, plugged: bool) -> bool {
        let closed = self.open && !plugged;
        self.open &= !closed;
        closed
    }

    /// Shows the cover and the screws as the casing is, and what a click on
    /// them does.
    fn show(
        casings: Query<(Entity, &Casing, Has<Connected>)>,
        children: Query<&Children>,
        mut covers: Query<&mut Visibility, With<Cover>>,
        mut screws: Query<(&Screw, &mut Transform)>,
        mut parts: Query<Prompted>,
    ) {
        for (device, casing, plugged) in &casings {
            let all_loose = casing.loose.iter().all(|loose| *loose);
            for part in children.iter_descendants(device) {
                if let Ok(mut visibility) = covers.get_mut(part) {
                    *visibility = if casing.open {
                        Visibility::Hidden
                    } else {
                        Visibility::Inherited
                    };
                }
                let loose =
                    |screw: &Screw| casing.loose.get(screw.index).copied().unwrap_or_default();
                if let Ok((screw, mut transform)) = screws.get_mut(part) {
                    transform.translation.y =
                        screw.rest - if loose(screw) { SCREW_OUT } else { 0.0 };
                }
                if let Ok((mut clickable, screw, plate, header)) = parts.get_mut(part) {
                    clickable.prompt = match (screw, plate, header) {
                        (Some(screw), ..) if loose(screw) => "click-screw-in",
                        (Some(_), ..) => "click-unscrew",
                        (None, true, _) if all_loose => "click-cover-off",
                        (None, true, _) => "click-cover-screwed",
                        (None, false, true) if plugged => "click-unplug",
                        (None, false, true) => "click-plug-in",
                        (None, false, false) if plugged => "click-cover-plugged",
                        (None, false, false) => "click-cover-on",
                    };
                }
            }
        }
    }

    /// The device that `part` belongs to, if the player holds it.
    fn held<'a>(
        part: Entity,
        focused: &Focused,
        parents: &Query<&ChildOf>,
        casings: &'a mut Query<(Entity, &mut Casing, &mut Holdable, Has<Connected>)>,
    ) -> Option<(Entity, Mut<'a, Casing>, Mut<'a, Holdable>, bool)> {
        if !focused.holds(part, parents) {
            return None;
        }
        let device = parents
            .iter_ancestors(part)
            .find(|ancestor| casings.contains(*ancestor))?;
        let (_, casing, holdable, plugged) = casings.get_mut(device).ok()?;
        Some((device, casing, holdable, plugged))
    }
}

impl Screw {
    fn press(
        mut press: On<PointerPress>,
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        screws: Query<&Screw>,
        mut casings: Query<(Entity, &mut Casing, &mut Holdable, Has<Connected>)>,
        mut commands: Commands,
    ) {
        let Ok(screw) = screws.get(press.entity) else {
            return;
        };
        if press.button != PointerButton::Primary {
            return;
        }
        let Some((_, mut casing, ..)) =
            Casing::held(press.entity, &focused, &parents, &mut casings)
        else {
            return;
        };
        press.propagate(false);
        let sound = if casing.turn_screw(screw.index) {
            Sound::Detent
        } else {
            Sound::Refuse
        };
        commands.trigger(Play(sound));
    }
}

impl CoverPlate {
    fn press(
        mut press: On<PointerPress>,
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        plates: Query<(), With<CoverPlate>>,
        mut casings: Query<(Entity, &mut Casing, &mut Holdable, Has<Connected>)>,
        mut commands: Commands,
    ) {
        if !plates.contains(press.entity) || press.button != PointerButton::Primary {
            return;
        }
        let Some((_, mut casing, mut holdable, _)) =
            Casing::held(press.entity, &focused, &parents, &mut casings)
        else {
            return;
        };
        press.propagate(false);
        let sound = if casing.lift_cover() {
            holdable.turn_over(casing.bounds.size.y);
            Sound::Unlatch
        } else {
            Sound::Refuse
        };
        commands.trigger(Play(sound));
    }
}

impl Board {
    fn press(
        mut press: On<PointerPress>,
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        boards: Query<(), With<Board>>,
        mut casings: Query<(Entity, &mut Casing, &mut Holdable, Has<Connected>)>,
        mut commands: Commands,
    ) {
        if !boards.contains(press.entity) || press.button != PointerButton::Primary {
            return;
        }
        let Some((_, mut casing, mut holdable, plugged)) =
            Casing::held(press.entity, &focused, &parents, &mut casings)
        else {
            return;
        };
        press.propagate(false);
        let sound = if casing.close_cover(plugged) {
            holdable.turn_over(casing.bounds.size.y);
            Sound::Latch
        } else {
            Sound::Refuse
        };
        commands.trigger(Play(sound));
    }
}

impl Header {
    /// Plugs the debug cable in, or pulls it out.
    fn press(
        mut press: On<PointerPress>,
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        headers: Query<(), With<Header>>,
        mut casings: Query<(Entity, &mut Casing, &mut Holdable, Has<Connected>)>,
        mut commands: Commands,
    ) {
        if !headers.contains(press.entity) || press.button != PointerButton::Primary {
            return;
        }
        let Some((device, casing, _, plugged)) =
            Casing::held(press.entity, &focused, &parents, &mut casings)
        else {
            return;
        };
        press.propagate(false);
        if !casing.open {
            commands.trigger(Play(Sound::Refuse));
        } else if plugged {
            commands.entity(device).remove::<Connected>();
            commands.trigger(Play(Sound::Unlatch));
        } else {
            commands.entity(device).insert(Connected);
            commands.trigger(Play(Sound::Latch));
        }
    }
}

#[cfg(test)]
mod tests {
    use ld60_locale::{Locale, Message};

    use super::*;

    #[test]
    fn every_prompt_formats() {
        Locale::on_disk().assert_formats(
            [
                "click-unscrew",
                "click-screw-in",
                "click-cover-off",
                "click-cover-screwed",
                "click-cover-on",
                "click-cover-plugged",
                "click-plug-in",
                "click-unplug",
            ]
            .map(Message::new),
        );
    }

    #[test]
    fn the_cover_comes_off_only_without_screws() {
        let mut casing = Casing::new(2, Block::new(Vec3::splat(0.02)));
        assert!(casing.sealed());
        assert!(!casing.lift_cover());
        assert!(casing.turn_screw(0));
        assert!(!casing.lift_cover());
        assert!(casing.turn_screw(1));
        assert!(casing.lift_cover());
        assert!(casing.open);
        assert!(!casing.turn_screw(0));
    }

    #[test]
    fn the_cover_stays_off_while_plugged_in() {
        let mut casing = Casing::new(1, Block::new(Vec3::splat(0.02)));
        casing.turn_screw(0);
        casing.lift_cover();
        assert!(!casing.close_cover(true));
        assert!(casing.close_cover(false));
        assert!(!casing.sealed());
        casing.turn_screw(0);
        assert!(casing.sealed());
    }
}
