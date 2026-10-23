use std::f32::consts::FRAC_PI_2;

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};
use ld60_core::audio::{Play, Sound};
#[cfg(feature = "debug_port")]
use ld60_core::debug_port::DebugMethods;
use ld60_core::layout::counter::CounterLayout;
use ld60_core::player::focus::{Clickable, Focused};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use super::board::BoardLayout;
use super::{Connected, Holdable, Turned};

/// Thickness of the back cover. The board and the header fit inside it.
pub(super) const COVER: f32 = 0.005;
const SCREW: Vec3 = Vec3::new(0.0055, 0.0014, 0.0055);
const SHANK: Vec3 = Vec3::new(0.0022, 0.008, 0.0022);
/// The empty hole of a screw that is out.
const HOLE: Vec3 = Vec3::new(0.0034, 0.0003, 0.0034);
/// Screws that are out lie this far from the left side of the device, and
/// this far apart.
const ASIDE: f32 = 0.014;
const SCREW_GAP: f32 = 0.014;
const SCREW_METAL: Color = Color::srgb_u8(0xc8, 0xca, 0xce);
const HOLE_DARK: Color = Color::srgb_u8(0x10, 0x10, 0x12);

/// The parts of a casing whose click prompt changes.
type Prompted<'a> = (&'a mut Clickable, Has<CoverPlate>, Has<Header>, Has<Board>);

/// The parts of a casing that show or hide.
type Shown = Or<(With<Cover>, With<Screw>, With<Hole>, With<Aside>)>;

pub(super) struct CasingPlugin;

impl Plugin for CasingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Casing::lay_out, Casing::show).chain())
            .add_observer(Screw::press)
            .add_observer(Hole::press)
            .add_observer(CoverPlate::press)
            .add_observer(Board::press)
            .add_observer(Header::press);
        #[cfg(feature = "debug_port")]
        app.add_debug_method("ld60.device", Casing::debug_device);
    }
}

/// The back cover of a device and its screws. The debug header on the board
/// is under the cover.
#[derive(Component, Debug)]
#[component(on_remove = Casing::on_remove)]
pub(crate) struct Casing {
    /// Whether each screw is out.
    loose: Vec<bool>,
    open: bool,
    /// The box around the device, from the cover to the tops of its keys.
    /// Open, it rests on its keys.
    bounds: Block,
    /// The screws lying next to the device, one per screw, shown while it
    /// is out.
    aside: Vec<Entity>,
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
struct Screw(usize);

/// Where a screw goes back in.
#[derive(Component, Debug)]
struct Hole(usize);

/// A screw that is out, lying on the mat next to its device.
#[derive(Component, Debug)]
struct Aside {
    index: usize,
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
            aside: Vec::new(),
        }
    }

    /// Where the screw `index` of a device with `bounds` lies while it is
    /// out, in a row along the left side, from the device's spot on the
    /// counter. Yaw and spot only, so it stays put when the device is turned
    /// over.
    pub(crate) fn aside_at(bounds: &Block, index: usize, rest: &Transform) -> Transform {
        let along = rest.rotation * Vec3::Z;
        let yaw = Quat::from_rotation_y(along.x.atan2(along.z));
        let offset = Vec3::new(
            -bounds.size.x / 2.0 - ASIDE,
            0.0,
            -bounds.size.z / 2.0 + SCREW_GAP * (index as f32 + 0.5),
        );
        let mut at = rest.translation.with_y(0.0) + yaw * offset;
        let mat = CounterLayout::new().mat;
        let on_mat = (mat.min().xz().cmple(at.xz()) & at.xz().cmple(mat.max().xz())).all();
        at.y = if on_mat { mat.max().y } else { mat.min().y };
        // Lying on its side, with the shank along the row.
        Transform::from_translation(at + Vec3::Y * SCREW.x / 2.0)
            .with_rotation(yaw * Quat::from_rotation_z(-FRAC_PI_2))
    }

    fn on_remove(mut world: DeferredWorld, context: HookContext) {
        let Some(casing) = world.get::<Self>(context.entity) else {
            return;
        };
        let aside = casing.aside.clone();
        let mut commands = world.commands();
        for screw in aside {
            commands.entity(screw).try_despawn();
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
                        Screw(index),
                        Clickable {
                            prompt: "click-unscrew",
                        },
                        Shape::cylinder(&head, SCREW_METAL),
                        Finish::Metal,
                    ));
                    // Under the head, so it shows once the screw is out.
                    let hole = Block::new(HOLE).translated(underside(spot) - Vec3::Y * HOLE.y);
                    cover.spawn((
                        Hole(index),
                        Clickable {
                            prompt: "click-screw-in",
                        },
                        Shape::cylinder(&hole, HOLE_DARK),
                        Finish::Plastic,
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

    /// Spawns the screws that lie next to a new device while they are out.
    fn lay_out(mut casings: Query<&mut Casing, Added<Casing>>, mut commands: Commands) {
        for mut casing in &mut casings {
            casing.aside = (0..casing.loose.len())
                .map(|index| {
                    commands
                        .spawn((
                            Name::new("screw aside"),
                            Aside { index },
                            Transform::default(),
                            Visibility::Hidden,
                        ))
                        .with_children(|screw| {
                            let head = Block::new(SCREW).translated(-Vec3::Y * SCREW.y / 2.0);
                            let shank = Block::new(SHANK).on_top_of(&head);
                            screw.spawn((Shape::cylinder(&head, SCREW_METAL), Finish::Metal));
                            screw.spawn((Shape::cylinder(&shank, SCREW_METAL), Finish::Metal));
                        })
                        .id()
                })
                .collect();
        }
    }

    /// Shows the cover and the screws as the casing is, and what a click on
    /// them does.
    fn show(
        casings: Query<(Entity, &Casing, &Holdable, Has<Connected>)>,
        children: Query<&Children>,
        mut shown: Query<&mut Visibility, Shown>,
        screws: Query<&Screw>,
        holes: Query<&Hole>,
        mut asides: Query<(&Aside, &mut Transform)>,
        mut parts: Query<Prompted>,
    ) {
        let visible = |show: bool| {
            if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        };
        for (device, casing, holdable, plugged) in &casings {
            let all_loose = casing.loose.iter().all(|loose| *loose);
            let loose = |index: usize| casing.loose.get(index).copied().unwrap_or_default();
            for part in children.iter_descendants(device) {
                let show = if let Ok(screw) = screws.get(part) {
                    Some(!loose(screw.0))
                } else if let Ok(hole) = holes.get(part) {
                    Some(loose(hole.0))
                } else {
                    None
                };
                if let Ok(mut visibility) = shown.get_mut(part) {
                    let show = show.unwrap_or(!casing.open);
                    visibility.set_if_neq(visible(show));
                }
                if let Ok((mut clickable, plate, header, board)) = parts.get_mut(part) {
                    let prompt = match (plate, header, board) {
                        (true, ..) if all_loose => "click-cover-off",
                        (true, ..) => "click-cover-screwed",
                        (_, true, _) if plugged => "click-unplug",
                        (_, true, _) => "click-plug-in",
                        (.., true) if plugged => "click-cover-plugged",
                        (.., true) => "click-cover-on",
                        _ => clickable.prompt,
                    };
                    clickable.set_if_neq(Clickable { prompt });
                }
            }
            for &screw in &casing.aside {
                let Ok((aside, mut transform)) = asides.get_mut(screw) else {
                    continue;
                };
                *transform = Self::aside_at(&casing.bounds, aside.index, holdable.rest());
                if let Ok(mut visibility) = shown.get_mut(screw) {
                    visibility.set_if_neq(visible(loose(aside.index)));
                }
            }
        }
    }

    /// `ld60.device { "open": true, "plugged": true }` opens or closes the
    /// devices and plugs the cable in or pulls it, as the player would. Both
    /// fields are optional. Returns the state of each device.
    #[cfg(feature = "debug_port")]
    fn debug_device(
        In(params): In<Option<serde_json::Value>>,
        mut casings: Query<(Entity, &mut Casing, &mut Holdable, Has<Connected>)>,
        mut commands: Commands,
    ) -> bevy::remote::BrpResult {
        let flag = |name: &str| {
            params
                .as_ref()
                .and_then(|params| params.get(name))
                .and_then(serde_json::Value::as_bool)
        };
        let mut states = Vec::new();
        for (device, mut casing, mut holdable, mut plugged) in &mut casings {
            if flag("plugged") == Some(false) && plugged {
                commands.entity(device).remove::<Connected>();
                plugged = false;
            }
            match flag("open") {
                Some(true) if !casing.open => {
                    casing.loose.fill(true);
                    casing.lift_cover();
                    holdable.turn_over(casing.bounds.size.y);
                }
                Some(false) if casing.close_cover(plugged) => {
                    casing.loose.fill(false);
                    holdable.turn_over(casing.bounds.size.y);
                }
                _ => {}
            }
            if flag("plugged") == Some(true) && casing.open && !plugged {
                commands.entity(device).insert(Connected);
                plugged = true;
            }
            states.push(serde_json::json!({
                "open": casing.open,
                "sealed": casing.sealed(),
                "plugged": plugged,
            }));
        }
        Ok(states.into())
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
        mut press: On<PointerClick>,
        focused: Res<Focused>,
        turned: Res<Turned>,
        parents: Query<&ChildOf>,
        screws: Query<&Screw>,
        mut casings: Query<(Entity, &mut Casing, &mut Holdable, Has<Connected>)>,
        mut commands: Commands,
    ) {
        let Ok(screw) = screws.get(press.entity) else {
            return;
        };
        if press.button != PointerButton::Primary || !turned.clicked() {
            return;
        }
        let Some((_, mut casing, ..)) =
            Casing::held(press.entity, &focused, &parents, &mut casings)
        else {
            return;
        };
        press.propagate(false);
        let out = casing.loose.get(screw.0) == Some(&false) && casing.turn_screw(screw.0);
        commands.trigger(Play(if out { Sound::ScrewOut } else { Sound::Refuse }));
    }
}

impl Hole {
    /// Puts the screw back in from beside the device.
    fn press(
        mut press: On<PointerClick>,
        focused: Res<Focused>,
        turned: Res<Turned>,
        parents: Query<&ChildOf>,
        holes: Query<&Hole>,
        mut casings: Query<(Entity, &mut Casing, &mut Holdable, Has<Connected>)>,
        mut commands: Commands,
    ) {
        let Ok(hole) = holes.get(press.entity) else {
            return;
        };
        if press.button != PointerButton::Primary || !turned.clicked() {
            return;
        }
        let Some((_, mut casing, ..)) =
            Casing::held(press.entity, &focused, &parents, &mut casings)
        else {
            return;
        };
        press.propagate(false);
        let back = casing.loose.get(hole.0) == Some(&true) && casing.turn_screw(hole.0);
        commands.trigger(Play(if back { Sound::ScrewIn } else { Sound::Refuse }));
    }
}

impl CoverPlate {
    fn press(
        mut press: On<PointerClick>,
        focused: Res<Focused>,
        turned: Res<Turned>,
        parents: Query<&ChildOf>,
        plates: Query<(), With<CoverPlate>>,
        mut casings: Query<(Entity, &mut Casing, &mut Holdable, Has<Connected>)>,
        mut commands: Commands,
    ) {
        if !plates.contains(press.entity)
            || press.button != PointerButton::Primary
            || !turned.clicked()
        {
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
            Sound::CoverOff
        } else {
            Sound::Refuse
        };
        commands.trigger(Play(sound));
    }
}

impl Board {
    fn press(
        mut press: On<PointerClick>,
        focused: Res<Focused>,
        turned: Res<Turned>,
        parents: Query<&ChildOf>,
        boards: Query<(), With<Board>>,
        mut casings: Query<(Entity, &mut Casing, &mut Holdable, Has<Connected>)>,
        mut commands: Commands,
    ) {
        if !boards.contains(press.entity)
            || press.button != PointerButton::Primary
            || !turned.clicked()
        {
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
            Sound::CoverOn
        } else {
            Sound::Refuse
        };
        commands.trigger(Play(sound));
    }
}

impl Header {
    /// Plugs the debug cable in, or pulls it out.
    fn press(
        mut press: On<PointerClick>,
        focused: Res<Focused>,
        turned: Res<Turned>,
        parents: Query<&ChildOf>,
        headers: Query<(), With<Header>>,
        mut casings: Query<(Entity, &mut Casing, &mut Holdable, Has<Connected>)>,
        mut commands: Commands,
    ) {
        if !headers.contains(press.entity)
            || press.button != PointerButton::Primary
            || !turned.clicked()
        {
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
            commands.trigger(Play(Sound::PlugOut));
        } else {
            commands.entity(device).insert(Connected);
            commands.trigger(Play(Sound::PlugIn));
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
