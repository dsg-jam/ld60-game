mod click;
mod fall;
mod travel;

use std::f32::consts::PI;

use bevy::prelude::*;
#[cfg(feature = "debug_port")]
use ld60_game_core::debug_port::{DebugMethod, DebugMethods, ParamKind};
use ld60_game_core::layout::counter::CounterLayout;
use ld60_game_player::focus::{ClickTool, Clickable, Focused, Prompting};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use self::fall::Fall;
pub use self::travel::Turning;
use super::board::{BoardLayout, CHIP_BLACK, LEG_TIN};
use super::phillips::Phillips;
use super::snap::Snap;
use super::thread::Thread;
use super::tool::ToolSpot;
use super::{Connected, Holdable};

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
/// The hole through the cover that a screw goes through.
const COVER_HOLE: f32 = 0.0013;
/// Each screw head sits in a round pocket in the cover this deep, a little
/// wider than the head, so it is almost flush with the cover.
pub(super) const POCKET: f32 = 0.0012;
const POCKET_RADIUS: f32 = SCREW.x / 2.0 + 0.0003;
/// The bottom of a pocket, which the screw goes through.
const POCKET_FLOOR: f32 = COVER - POCKET;
/// Screws that are out roll no further than this from where the device
/// rests, to its left, right and front. Behind it the shop manuals lie open,
/// so the screws stay out of there.
const SCREW_ROOM: f32 = 0.03;
/// The room starts this far in front of the back of the device, so its
/// corners stay out of the file when the device is turned a little.
const ROOM_BACK: f32 = 0.006;
/// Gap between the room of the screws and the cover lying on the mat.
const COVER_GAP: f32 = 0.008;
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
type Shown = With<Hole>;
/// What makes a screw in its hole a target for the screwdriver.
type ScrewTarget = (ClickTool, ToolSpot, Snap);

pub(super) struct CasingPlugin;

impl Plugin for CasingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
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
        app.add_debug_method(
            DebugMethod::new("ld60.device", "Works the devices as the player would.")
                .param("open", ParamKind::Flag, "opens or closes the cover")
                .param("plugged", ParamKind::Flag, "plugs the cable in or pulls it")
                .param(
                    "turn",
                    ParamKind::Numbers,
                    "x,y,z degrees for a held device",
                )
                .param("screw", ParamKind::Numbers, "turns this screw in or out"),
            Casing::debug_device,
        );
    }
}

/// The back cover of a device and its screws. The debug header on the board
/// is under the cover.
#[derive(Component, Debug)]
pub struct Casing {
    /// Whether each screw is out.
    loose: Vec<bool>,
    open: bool,
    /// The box around the device, from the cover to the tops of its keys.
    /// Open, it rests on its keys.
    bounds: Block,
    /// The screws, in their holes or out on the mat. Children of the device,
    /// placed in the world by [`Self::travel`].
    screws: Vec<Entity>,
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

/// A screw of the casing, in its hole, out on the mat next to its device,
/// or on its way between them.
#[derive(Component, Debug)]
struct Screw {
    /// Which hole it goes in.
    index: usize,
    /// How far it is on its way between its hole and the mat, from 0 to 1.
    /// Once it is out of its hole it falls and lies where it lands.
    progress: f32,
    /// How it falls and where it lies.
    fall: Option<Fall>,
    /// Whether it is carried between where it lies and its hole, on its
    /// way back in.
    carried: bool,
    /// How often it was taken out, which picks how it falls.
    drops: u32,
}

/// Where a screw goes back in.
#[derive(Component, Debug)]
struct Hole(usize);

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
            screws: Vec::new(),
            holes,
            cover: None,
        }
    }

    /// Where the screws of a device with `bounds` may roll once they are
    /// out: around where the device rests at `rest`, but for behind it, on
    /// the counter. Yaw and spot only, so it stays put when the device is
    /// turned over. Returns the middle and the size across the counter.
    pub fn screw_room(bounds: &Block, rest: &Transform) -> (Transform, Vec2) {
        let yaw = Self::yaw(rest);
        let at = rest.translation.with_y(0.0) + yaw * Vec3::Z * (SCREW_ROOM + ROOM_BACK) / 2.0;
        let size = bounds.size.xz() + Vec2::new(2.0 * SCREW_ROOM, SCREW_ROOM - ROOM_BACK);
        (Transform::from_translation(at).with_rotation(yaw), size)
    }

    /// Where the device rests at `rest`, turned by its yaw only, see
    /// [`Self::screw_room`].
    fn resting_spot(rest: &Transform) -> Transform {
        Transform::from_translation(rest.translation).with_rotation(Self::yaw(rest))
    }

    /// Where the cover of a device with `bounds` lies while it is off, on
    /// the left of the device past the room of its screws, with its outside
    /// up. Like [`Self::screw_room`], it stays put when the device is turned
    /// over.
    pub fn cover_at(bounds: &Block, rest: &Transform) -> Transform {
        let yaw = Self::yaw(rest);
        let at = rest.translation.with_y(0.0)
            + yaw * Vec3::NEG_X * (bounds.size.x + SCREW_ROOM + COVER_GAP);
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
        let holes: Vec<_> = spots
            .screws
            .iter()
            .map(|&spot| underside(spot) + Vec3::Y * POCKET)
            .collect();
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
                    Shape::drilled(&plate, &drilled, POCKET_RADIUS, color),
                    Finish::Plastic,
                ));
                for (index, &hole) in holes.iter().enumerate() {
                    // A hair short of the inside, so it does not fight with it.
                    let floor = Block::new(Vec3::new(
                        2.0 * POCKET_RADIUS,
                        POCKET_FLOOR - 0.00005,
                        2.0 * POCKET_RADIUS,
                    ))
                    .translated(hole);
                    cover.spawn((
                        CoverPlate,
                        Shape::drilled(&floor, &[Vec2::ZERO], COVER_HOLE, color),
                        Finish::Plastic,
                    ));
                    // The mouth of the hole, to click once the screw is out.
                    cover.spawn((
                        Hole(index),
                        ClickTool::Screwdriver,
                        ToolSpot(Vec3::ZERO),
                        Snap,
                        Transform::from_translation(hole),
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
        let screws = holes
            .iter()
            .enumerate()
            .map(|(index, &hole)| Screw::spawn(device, index, hole))
            .collect();
        Self {
            screws,
            ..Self::new(holes, Block::new(shell.size.with_y(height)))
        }
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
    /// them does. From the desk a click inspects the device, so its parts
    /// only say so once it is focused.
    #[expect(clippy::too_many_arguments, reason = "Bevy system")]
    fn show(
        focused: Res<Focused>,
        casings: Query<(Entity, &Casing, Has<Connected>)>,
        children: Query<&Children>,
        mut shown: Query<&mut Visibility, Shown>,
        screws: Query<(&Screw, Has<Snap>)>,
        holes: Query<&Hole>,
        parts: Query<Prompted, Clicked>,
        mut commands: Commands,
    ) {
        for (device, casing, plugged) in &casings {
            let all_loose = casing.loose.iter().all(|loose| *loose);
            // A screw is out from its first turn until it is all the way back
            // in. Then the screwdriver aims at the hole, not at the screw.
            let out = |index: usize| {
                casing.loose.get(index).copied().unwrap_or_default()
                    || casing
                        .screws
                        .get(index)
                        .and_then(|&screw| screws.get(screw).ok())
                        .is_some_and(|(screw, _)| screw.progress > 0.0)
            };
            for &entity in &casing.screws {
                match screws.get(entity) {
                    Ok((screw, true)) if out(screw.index) => {
                        commands.entity(entity).remove::<ScrewTarget>();
                    }
                    Ok((screw, false)) if !out(screw.index) => {
                        commands.entity(entity).insert(Screw::target());
                    }
                    _ => {}
                }
            }
            for part in children.iter_descendants(device) {
                if let (Ok(hole), Ok(mut visibility)) = (holes.get(part), shown.get_mut(part)) {
                    visibility.set_if_neq(if out(hole.0) {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    });
                }
                let Ok((now, plate, header, board, screw)) = parts.get(part) else {
                    continue;
                };
                let screw_out = screw && screws.get(part).is_ok_and(|(screw, _)| out(screw.index));
                // The cover stays on while plugged in, and comes off once all
                // screws are out.
                let wanted = (focused.get() == Some(device) && !screw_out).then_some(
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
    }

    /// `ld60.device { "open": true, "plugged": true, "turn": [x, y, z],
    /// "screw": 0 }` opens or closes the devices and plugs the cable in or
    /// pulls it, as the player would. `turn` sets how a held device is
    /// turned, as angles in degrees around the camera's axes. `screw` turns
    /// one screw in or out, like a click on it. All fields are optional.
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
            if let Some(screw) = params
                .as_ref()
                .and_then(|params| params.get("screw"))
                .and_then(serde_json::Value::as_u64)
            {
                casing.turn_screw(screw as usize);
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
}

impl Screw {
    /// Spawns the screw that goes in at `hole` into `device`, in its hole.
    fn spawn(device: &mut ChildSpawnerCommands, index: usize, hole: Vec3) -> Entity {
        device
            .spawn((
                Name::new("screw"),
                Self {
                    index,
                    progress: 0.0,
                    fall: None,
                    carried: false,
                    drops: 0,
                },
                Self::target(),
                Transform::from_translation(hole),
                Visibility::Inherited,
            ))
            .with_children(|screw| {
                let head = Block::new(SCREW).translated(-Vec3::Y * SCREW.y / 2.0);
                let shank = Block::new(Thread::SIZE).on_top_of(&head);
                screw.spawn((
                    Shape::turned(Phillips::head(), &head, SCREW_METAL),
                    Finish::Plated,
                ));
                screw.spawn((
                    Shape::turned(Thread::shank(), &shank, SCREW_METAL),
                    Finish::Plated,
                ));
            })
            .id()
    }

    /// What the screwdriver aims at, on the middle of the head.
    fn target() -> ScrewTarget {
        (
            ClickTool::Screwdriver,
            ToolSpot(Vec3::NEG_Y * SCREW.y / 2.0),
            Snap,
        )
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
