mod travel;

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};
use ld60_game_audio::{Play, Sound};
#[cfg(feature = "debug_port")]
use ld60_game_core::debug_port::DebugMethods;
use ld60_game_core::layout::counter::CounterLayout;
use ld60_game_player::focus::{ClickTool, Clickable, Focused, Prompting};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

pub use self::travel::Turning;
use super::board::{BoardLayout, CHIP_BLACK, LEG_TIN};
use super::phillips::Phillips;
use super::snap::Snap;
use super::tool::ToolSpot;
use super::{Connected, Holdable, Turned};

/// Thickness of the walls of a device.
pub(super) const WALL: f32 = 0.0015;
/// Thickness of the back cover, a wall like the others.
pub(super) const COVER: f32 = WALL;
/// The posts the board sits on, from the front down to the board. The
/// screws go through the cover and the board into them.
const BOSS: f32 = 0.0026;
/// The hole in the end of a post that takes a screw.
const PILOT: Vec3 = Vec3::new(0.0018, 0.0002, 0.0018);
const SCREW: Vec3 = Phillips::HEAD;
const SHANK: Vec3 = Vec3::new(0.0022, 0.008, 0.0022);
/// The hole through the cover that a screw goes through.
const COVER_HOLE: f32 = 0.0013;
/// Screws that are out lie this far from the left side of the device, and
/// this far apart.
const ASIDE: f32 = 0.014;
const SCREW_GAP: f32 = 0.014;
/// Gap between the device and its cover lying on the mat.
const COVER_GAP: f32 = 0.02;
const SCREW_METAL: Color = Color::srgb_u8(0xc8, 0xca, 0xce);
const HOLE_DARK: Color = Color::srgb_u8(0x10, 0x10, 0x12);

/// Moves the cover and the screws, see [`Casing::travel`]. Whatever follows
/// a screw runs after it.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CasingTravel;

/// The parts of a casing that say what a click does while the device is
/// focused.
type Prompted = (
    Prompting,
    Has<CoverPlate>,
    Has<Header>,
    Has<Board>,
    Has<Screw>,
);
type Clicked = Or<(
    With<CoverPlate>,
    With<Header>,
    With<Board>,
    With<Screw>,
    With<Hole>,
)>;

/// The parts of a casing that show or hide.
type Shown = Or<(With<Screw>, With<Hole>, With<Aside>)>;

pub(super) struct CasingPlugin;

impl Plugin for CasingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Casing::lay_out,
                Casing::travel.after(Holdable::follow).in_set(CasingTravel),
                Casing::show,
            )
                .chain(),
        )
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
pub struct Casing {
    /// Whether each screw is out.
    loose: Vec<bool>,
    open: bool,
    /// The box around the device, from the cover to the tops of its keys.
    /// Open, it rests on its keys.
    bounds: Block,
    /// The screws lying next to the device, one per screw, shown while it
    /// is out.
    aside: Vec<Entity>,
    /// Where each screw goes in, in the space of the cover. The screw points
    /// along +Y.
    holes: Vec<Vec3>,
    /// How far the cover is on its way from the device to the mat, from 0
    /// to 1. Unknown until the cover first shows.
    cover: Option<f32>,
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

/// A screw that is out, lying on the mat next to its device, or on its way
/// between there and its hole.
#[derive(Component, Debug)]
struct Aside {
    /// How far it is on its way from its hole to the mat, from 0 to 1.
    progress: f32,
}

#[derive(Component, Debug)]
struct Board;

/// Where the debug cable plugs in.
#[derive(Component, Debug)]
pub struct Header;

impl Casing {
    fn new(holes: Vec<Vec3>, bounds: Block) -> Self {
        Self {
            loose: vec![false; holes.len()],
            open: false,
            bounds,
            aside: Vec::new(),
            holes,
            cover: None,
        }
    }

    /// Where the screw `index` of a device with `bounds` lies while it is
    /// out, in a row along the left side, from the device's spot on the
    /// counter. Yaw and spot only, so it stays put when the device is turned
    /// over.
    pub fn aside_at(bounds: &Block, index: usize, rest: &Transform) -> Transform {
        let yaw = Self::yaw(rest);
        let offset = Vec3::new(
            -bounds.size.x / 2.0 - ASIDE,
            0.0,
            -bounds.size.z / 2.0 + SCREW_GAP * (index as f32 + 0.5),
        );
        let at = rest.translation.with_y(0.0) + yaw * offset;
        // Lying on its side, with the shank along the row.
        Transform::from_translation(at.with_y(Self::ground(at) + SCREW.x / 2.0))
            .with_rotation(yaw * Quat::from_rotation_z(-FRAC_PI_2))
    }

    /// Where the cover of a device with `bounds` lies while it is off, on
    /// the left of the device past its screws, with its outside up. Like
    /// [`Self::aside_at`], it stays put when the device is turned over.
    pub fn cover_at(bounds: &Block, rest: &Transform) -> Transform {
        let yaw = Self::yaw(rest);
        let at = rest.translation.with_y(0.0)
            + yaw * Vec3::NEG_X * (bounds.size.x + ASIDE + SCREW.x + COVER_GAP);
        // The plate reaches from its origin to `COVER` inside, so turned over
        // it lies below its origin.
        Transform::from_translation(at.with_y(Self::ground(at) + COVER))
            .with_rotation(yaw * Quat::from_rotation_z(PI))
    }

    /// The turn of a device resting at `rest` around the vertical.
    fn yaw(rest: &Transform) -> Quat {
        let along = rest.rotation * Vec3::Z;
        Quat::from_rotation_y(along.x.atan2(along.z))
    }

    /// The height of the mat at `at` or, off the mat, of the counter.
    fn ground(at: Vec3) -> f32 {
        let mat = CounterLayout::new().mat;
        let on_mat = (mat.min().xz().cmple(at.xz()) & at.xz().cmple(mat.max().xz())).all();
        if on_mat { mat.max().y } else { mat.min().y }
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
    pub fn bounds(&self) -> &Block {
        &self.bounds
    }

    /// Whether the cover is on and all screws are in, as the customer
    /// brought it.
    pub fn sealed(&self) -> bool {
        !self.open && !self.loose.contains(&true)
    }

    /// Spawns the cover with `sticker` on it into `device`, below `shell`,
    /// and the board of `layout` with `faces` and its parts under the cover.
    /// Returns the casing of a device `height` tall.
    #[expect(clippy::too_many_arguments, reason = "the parts of a casing")]
    pub(super) fn spawn(
        device: &mut ChildSpawnerCommands,
        shell: &Block,
        color: Color,
        spots: &CasingSpots,
        sticker: impl Bundle,
        layout: &BoardLayout,
        faces: [impl Bundle; 2],
        marking: Handle<Image>,
        height: f32,
    ) -> Self {
        let plate = Block::new(shell.size.with_y(COVER));
        let underside = |spot: Vec2| plate.top_spot(spot).with_y(0.0);
        let holes: Vec<_> = spots.screws.iter().map(|&spot| underside(spot)).collect();
        device
            .spawn((
                Name::new("back cover"),
                Cover,
                Transform::IDENTITY,
                Visibility::Inherited,
            ))
            .with_children(|cover| {
                let drilled: Vec<Vec2> = holes.iter().map(|hole| hole.xz()).collect();
                cover.spawn((
                    CoverPlate,
                    Shape::drilled(&plate, &drilled, COVER_HOLE, color),
                    Finish::Plastic,
                ));
                for (index, &spot) in spots.screws.iter().enumerate() {
                    let head = Block::new(SCREW).translated(underside(spot) - Vec3::Y * SCREW.y);
                    cover.spawn((
                        Screw(index),
                        ClickTool::Screwdriver,
                        ToolSpot(Vec3::NEG_Y * SCREW.y / 2.0),
                        Snap,
                        Shape::turned(Phillips::head(), &head, SCREW_METAL),
                        Finish::Plated,
                    ));
                    // The mouth of the hole, to click once the screw is out.
                    cover.spawn((
                        Hole(index),
                        ClickTool::Screwdriver,
                        ToolSpot(Vec3::ZERO),
                        Snap,
                        Transform::from_translation(underside(spot) - Vec3::Y * COVER / 2.0),
                        Visibility::Hidden,
                    ));
                }
                cover.spawn(sticker);
            });
        for hole in &holes {
            let top = layout.board.max().y;
            let boss = Block::new(Vec3::new(
                2.0 * BOSS,
                shell.max().y - WALL - top,
                2.0 * BOSS,
            ))
            .translated(hole.with_y(top));
            let pilot = Block::new(PILOT).translated(hole.with_y(top - PILOT.y / 2.0));
            device.spawn((Shape::cylinder(&boss, color), Finish::Plastic));
            device.spawn((Shape::cylinder(&pilot, HOLE_DARK), Finish::Plastic));
        }
        for edge in layout.edges() {
            device.spawn((
                Board,
                Shape::cuboid(&edge, BoardLayout::edge()),
                Finish::Plastic,
            ));
        }
        for face in faces {
            device.spawn((Board, face));
        }
        device
            .spawn((
                Header,
                ClickTool::Plug,
                // The plug goes over the whole header.
                ToolSpot(Vec3::ZERO),
                Shape::cuboid(&layout.header, BoardLayout::header_color()),
                Finish::Plastic,
            ))
            .insert(Snap);
        // The parts count as the board, so a click on one puts the cover on.
        for (part, color) in layout.parts() {
            device.spawn((Board, Shape::cuboid(part, color), Finish::Plastic));
        }
        let chip = layout.chip();
        device
            .spawn((Name::new("chip"), chip.body, Visibility::Inherited))
            .with_child((
                Board,
                Shape::textured_cuboid(
                    &Block::new(chip.size).translated(Vec3::NEG_Y * chip.size.y / 2.0),
                    marking,
                    Rect::new(0.0, 0.0, 1.0, 1.0),
                ),
                Finish::Plastic,
            ));
        for leg in &chip.legs {
            device.spawn((Board, Shape::cuboid(leg, LEG_TIN), Finish::Plated));
        }
        device.spawn((
            Board,
            Shape::cylinder(&chip.dimple, CHIP_BLACK),
            Finish::Plastic,
        ));
        Self::new(holes, Block::new(shell.size.with_y(height)))
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
                            Aside {
                                progress: if casing.loose[index] { 1.0 } else { 0.0 },
                            },
                            Transform::default(),
                            Visibility::Hidden,
                        ))
                        .with_children(|screw| {
                            let head = Block::new(SCREW).translated(-Vec3::Y * SCREW.y / 2.0);
                            let shank = Block::new(SHANK).on_top_of(&head);
                            screw.spawn((
                                Shape::turned(Phillips::head(), &head, SCREW_METAL),
                                Finish::Plated,
                            ));
                            screw.spawn((Shape::cylinder(&shank, SCREW_METAL), Finish::Plated));
                        })
                        .id()
                })
                .collect();
        }
    }

    /// Shows the cover and the screws as the casing is, and what a click on
    /// them does. From the desk a click inspects the device, so its parts
    /// only say so once it is focused.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn show(
        focused: Res<Focused>,
        casings: Query<(Entity, &Casing, Has<Connected>)>,
        children: Query<&Children>,
        mut shown: Query<&mut Visibility, Shown>,
        screws: Query<&Screw>,
        holes: Query<&Hole>,
        asides: Query<&Aside>,
        parts: Query<Prompted, Clicked>,
        mut commands: Commands,
    ) {
        let visible = |show: bool| {
            if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        };
        for (device, casing, plugged) in &casings {
            let all_loose = casing.loose.iter().all(|loose| *loose);
            // A screw shows in its hole once it is all the way in, and the
            // hole while the screw is out or on its way.
            let out = |index: usize| {
                casing.loose.get(index).copied().unwrap_or_default()
                    || casing
                        .aside
                        .get(index)
                        .and_then(|&aside| asides.get(aside).ok())
                        .is_some_and(|aside| aside.progress > 0.0)
            };
            for part in children.iter_descendants(device) {
                let show = if let Ok(screw) = screws.get(part) {
                    Some(!out(screw.0))
                } else {
                    holes.get(part).ok().map(|hole| out(hole.0))
                };
                if let (Some(show), Ok(mut visibility)) = (show, shown.get_mut(part)) {
                    visibility.set_if_neq(visible(show));
                }
                if let Ok((now, plate, header, board, screw)) = parts.get(part) {
                    // The cover stays on while plugged in, and comes off once
                    // all screws are out.
                    let wanted = (focused.get() == Some(device)).then_some(
                        match (plate, header, board, screw) {
                            (true, ..) if casing.open => ("click-cover-on", !plugged),
                            (true, ..) => ("click-cover-off", all_loose),
                            (_, true, ..) if plugged => ("click-unplug", true),
                            (_, true, ..) => ("click-plug-in", true),
                            (_, _, true, _) => ("click-cover-on", !plugged),
                            (.., true) => ("click-unscrew", true),
                            _ => ("click-screw-in", true),
                        },
                    );
                    Clickable::show(&mut commands, part, now, wanted);
                }
            }
            for &screw in &casing.aside {
                let Ok(aside) = asides.get(screw) else {
                    continue;
                };
                if let Ok(mut visibility) = shown.get_mut(screw) {
                    visibility.set_if_neq(visible(aside.progress > 0.0));
                }
            }
        }
    }

    /// `ld60.device { "open": true, "plugged": true, "turn": [x, y, z] }`
    /// opens or closes the devices and plugs the cable in or pulls it, as
    /// the player would. `turn` sets how a held device is turned, as angles
    /// in degrees around the camera's axes. All fields are optional.
    /// Returns the state of each device.
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
        let turn = params
            .as_ref()
            .and_then(|params| params.get("turn"))
            .and_then(serde_json::Value::as_array)
            .and_then(|angles| {
                let angle = |i: usize| angles.get(i)?.as_f64().map(|a| (a as f32).to_radians());
                Some(Quat::from_euler(
                    EulerRot::XYZ,
                    angle(0)?,
                    angle(1)?,
                    angle(2)?,
                ))
            });
        let mut states = Vec::new();
        for (device, mut casing, mut holdable, mut plugged) in &mut casings {
            if let Some(turn) = turn {
                holdable.spin = turn;
            }
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
        // The screwdriver plays the turning, see `Casing::travel`.
        if !out {
            commands.trigger(Play(Sound::Refuse));
        }
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
        if !back {
            commands.trigger(Play(Sound::Refuse));
        }
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
        let Some((_, mut casing, mut holdable, plugged)) =
            Casing::held(press.entity, &focused, &parents, &mut casings)
        else {
            return;
        };
        press.propagate(false);
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
                "click-cover-on",
                "click-plug-in",
                "click-unplug",
            ]
            .map(Message::new),
        );
    }

    #[test]
    fn the_cover_comes_off_only_without_screws() {
        let mut casing = Casing::new(vec![Vec3::ZERO; 2], Block::new(Vec3::splat(0.02)));
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
        let mut casing = Casing::new(vec![Vec3::ZERO], Block::new(Vec3::splat(0.02)));
        casing.turn_screw(0);
        casing.lift_cover();
        assert!(!casing.close_cover(true));
        assert!(casing.close_cover(false));
        assert!(!casing.sealed());
        casing.turn_screw(0);
        assert!(casing.sealed());
    }
}
