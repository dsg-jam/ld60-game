//! Pinning a page of the file to the rail at the monitor and putting it
//! back. The lever opens the rings, the page slides off them and flies to
//! its slot on the rail, and the rings shut again. A page that goes back
//! flies from the rail and drops onto the open rings. One page moves at a
//! time, and the others wait their turn.

use std::collections::VecDeque;
use std::f32::consts::PI;

use bevy::{platform::hash::fixed_hash_one, prelude::*};

use super::Filed;
use super::index::Links;
use super::model::{COVER, HOLE_RADIUS, Parts, SHEET_SIZE};
use super::pins::{Pins, Sheet};
use super::pose::Pose;
use super::stack::{Stack, holes, stray};
use super::top_sheet::TopSheet;
use crate::audio::{PlayAt, Sound};
use crate::parts::paper::{
    Binding, LeafShape, Paper, PaperMaterial, Pressed, RELAX_TIME, Wrinkles, reprinted,
};
use crate::player::focus::{Focus, Focused};
use crate::workshop::run::Run;

/// Seconds the lever takes to open the rings.
const OPEN_TIME: f32 = 0.15;
/// Seconds a pinned page takes to slide off the rings.
const LIFT_TIME: f32 = 0.2;
/// Seconds a page flies between the file and the rail.
const FLY_TIME: f32 = 0.5;
/// Seconds a page that goes back takes to drop onto the rings.
const DROP_TIME: f32 = 0.2;
/// Seconds the rings take to shut.
const SHUT_TIME: f32 = 0.15;
/// Seconds a page stays after it arrives, until the sheet that takes its
/// place shows.
const LINGER: f32 = 0.1;
/// How far a pinned page lifts off the rings, in meters.
const LIFT: f32 = 0.03;
/// How far a pinned page slides away from the rings as it lifts.
const SLIDE: f32 = 0.015;
/// How far a flying page rises above the straight way.
const ARC: f32 = 0.12;
/// How far a flying page rolls about the way it flies, in radians.
const ROLL: f32 = 0.35;
/// How far above the top edge of the file a page starts to drop in.
const ABOVE: f32 = 0.03;
/// How far a page that arrives at the rail stays in front of the sheet
/// that takes its place.
const IN_FRONT: f32 = 0.0005;

/// Asks the manuals file to pin a sheet to the rail or to put it back.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MoveSheet {
    Pin(Sheet),
    Unpin(Sheet),
}

/// A sheet on the rail, in the slot it holds, see [`Pins`].
#[derive(Component, Debug)]
pub(crate) struct RailSlot(pub(crate) usize);

/// The pages of a file on their way to or from the rail.
#[derive(Component, Debug, Default)]
pub(super) struct SheetMoves {
    waiting: VecDeque<MoveSheet>,
    flight: Option<Flight>,
}

#[derive(Debug)]
struct Flight {
    sheet: Sheet,
    pin: bool,
    slot: usize,
    elapsed: f32,
    seed: u64,
    wrinkles: Wrinkles,
    shape: LeafShape,
    /// The page once it has left the rings or the rail.
    flyer: Option<Flyer>,
    /// The page the canvas for flights shows.
    print: Option<Entity>,
    /// Whether the page shows like on the side it flies to yet.
    swapped: bool,
    /// Whether it has arrived and the rail holds it, or no longer does.
    arrived: bool,
    /// Whether the rings started to shut.
    shutting: bool,
}

#[derive(Debug)]
struct Flyer {
    entity: Entity,
    mesh: Handle<Mesh>,
    /// Where it left from.
    from: Transform,
    /// The share of its wrinkles it showed as it left.
    share: f32,
    /// The paper on the side of the file and on the side of the rail.
    paper: [Handle<PaperMaterial>; 2],
}

impl MoveSheet {
    fn sheet(self) -> Sheet {
        match self {
            Self::Pin(sheet) | Self::Unpin(sheet) => sheet,
        }
    }
}

impl SheetMoves {
    /// The pins the file is filed for: those on the rail and the page on its
    /// way there.
    pub(super) fn filed_pins(&self, pins: &Pins) -> Pins {
        let mut pins = pins.clone();
        if let Some(flight) = &self.flight
            && flight.pin
            && flight.flyer.is_some()
        {
            pins.pin(flight.sheet);
        }
        pins
    }

    #[cfg(test)]
    pub(super) fn waiting(&self) -> Vec<MoveSheet> {
        self.waiting.iter().copied().collect()
    }

    /// Whether the open page is about to leave, so no leaf may turn.
    pub(super) fn holds_open_page(&self) -> bool {
        self.flight
            .as_ref()
            .is_some_and(|flight| flight.pin && flight.flyer.is_none())
    }

    /// Lets the move wait its turn, unless the sheet moves already.
    pub(super) fn wait(event: On<MoveSheet>, mut files: Query<&mut SheetMoves>) {
        let sheet = event.sheet();
        for mut moves in &mut files {
            let busy = moves
                .flight
                .as_ref()
                .is_some_and(|flight| flight.sheet == sheet)
                || moves.waiting.iter().any(|other| other.sheet() == sheet);
            if !busy {
                moves.waiting.push_back(*event);
            }
        }
    }

    /// Starts the next move that still makes sense. A page is only pinned
    /// if it is the page the player reads and the rail has room for it. It
    /// waits while the file still opens or turns to it.
    fn start_next(
        &mut self,
        (file, reading): (Entity, bool),
        (filed, paper, stack, pose): (&Filed, &Paper, &Stack, &Pose),
        pins: &Pins,
        commands: &mut Commands,
    ) -> bool {
        while let Some(next) = self.waiting.pop_front() {
            let sheet = next.sheet();
            let (pin, slot) = match next {
                MoveSheet::Pin(_) => {
                    let open = filed
                        .in_file
                        .number(paper.current_page())
                        .and_then(|number| filed.contents.sheet_at(number));
                    let Some(slot) = pins.free_slot().filter(|_| reading && open == Some(sheet))
                    else {
                        continue;
                    };
                    if !pose.is_open() || !stack.is_at(paper.current_page()) {
                        self.waiting.push_front(next);
                        return false;
                    }
                    (true, slot)
                }
                MoveSheet::Unpin(_) => match pins.slot_of(sheet) {
                    Some(slot) => (false, slot),
                    None => continue,
                },
            };
            let seed = fixed_hash_one(("manuals", Some(sheet)));
            let wrinkles = Wrinkles::held(SHEET_SIZE, seed, &Binding::rings());
            let print = if pin {
                write_filed(commands, file, filed, sheet, pins.is_full())
            } else {
                sheet.spawn(commands, &filed.flight)
            };
            self.flight = Some(Flight {
                sheet,
                pin,
                slot,
                elapsed: 0.0,
                seed,
                wrinkles,
                shape: LeafShape::new(&wrinkles),
                flyer: None,
                print,
                swapped: false,
                arrived: false,
                shutting: false,
            });
            return true;
        }
        false
    }

    /// Moves the pages to and from the rail, and works the lever and the
    /// rings for them.
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "Bevy system"
    )]
    pub(super) fn fly(
        time: Res<Time>,
        (state, focused): (Res<State<Focus>>, Res<Focused>),
        mut run: ResMut<Run>,
        mut files: Query<(
            Entity,
            &mut SheetMoves,
            &Parts,
            &Filed,
            &Stack,
            &Pose,
            &Paper,
        )>,
        slots: Query<(
            Entity,
            &RailSlot,
            &GlobalTransform,
            &MeshMaterial3d<PaperMaterial>,
            &Wrinkles,
        )>,
        sheets: Query<(
            &GlobalTransform,
            Option<&TopSheet>,
            Option<&MeshMaterial3d<PaperMaterial>>,
        )>,
        mut transforms: Query<&mut Transform>,
        mut cameras: Query<&mut Camera>,
        mut assets: (ResMut<Assets<PaperMaterial>>, ResMut<Assets<Mesh>>),
        mut commands: Commands,
    ) {
        for (file, mut moves, parts, filed, stack, pose, paper) in &mut files {
            let lever = sheets.get(parts.spine).map_or(Vec3::ZERO, |(spine, ..)| {
                spine.transform_point(Parts::lever_spot())
            });
            let reading = *state.get() == Focus::Paper && focused.get() == Some(file);
            if moves.flight.is_none()
                && moves.start_next(
                    (file, reading),
                    (filed, paper, stack, pose),
                    run.pins(),
                    &mut commands,
                )
            {
                commands.trigger(PlayAt(Sound::RingsOpen, lever));
            }
            let flying = moves.flight.is_some();
            if let Ok(mut camera) = cameras.get_mut(filed.flight.camera)
                && camera.is_active != flying
            {
                camera.is_active = flying;
            }
            let Some(flight) = &mut moves.flight else {
                continue;
            };
            flight.elapsed += time.delta_secs();
            parts.open_rings(flight.rings(), &mut transforms);
            if !flight.shutting && flight.elapsed >= flight.shut_start() {
                flight.shutting = true;
                commands.trigger(PlayAt(Sound::RingsShut, lever));
            }
            let Some((slot, _, rail, rail_paper, rail_wrinkles)) =
                slots.iter().find(|(_, slot, ..)| slot.0 == flight.slot)
            else {
                continue;
            };
            let mut target = rail.compute_transform();
            target.scale = Vec3::splat(rail_wrinkles.size().x / SHEET_SIZE.x);
            if !flight.pin && flight.flyer.is_none() {
                let flat = (1.0 - flight.elapsed / OPEN_TIME).max(0.0);
                commands.entity(slot).insert(Pressed {
                    relaxed: RELAX_TIME * flat,
                    relaxing: false,
                });
            }
            if flight.flyer.is_none() && flight.elapsed >= OPEN_TIME {
                let (materials, meshes) = (&mut *assets.0, &mut *assets.1);
                let Some((file_paper, rail_paper)) = sheets
                    .get(parts.sheet)
                    .ok()
                    .and_then(|(.., paper)| materials.get(paper?))
                    .cloned()
                    .zip(materials.get(rail_paper).cloned())
                else {
                    continue;
                };
                let mut file_side = reprinted(&file_paper, filed.flight.image.clone());
                let holes = holes(stray(flight.seed))
                    .map(|hole| (hole + SHEET_SIZE / 2.0).extend(HOLE_RADIUS));
                file_side.extension.set_sheet(&flight.wrinkles, holes);
                file_side.extension.set_print(flight.sheet.print());
                let mut rail_side = reprinted(&rail_paper, filed.flight.image.clone());
                rail_side.extension.set_print(flight.sheet.print());
                let paper = [materials.add(file_side), materials.add(rail_side)];
                let (from, share) = if flight.pin {
                    let Ok((open, top, _)) = sheets.get(parts.sheet) else {
                        continue;
                    };
                    (open.compute_transform(), top.map_or(0.0, TopSheet::share))
                } else {
                    commands.entity(slot).insert(Visibility::Hidden);
                    (target, 0.0)
                };
                let mesh = meshes.add(flight.shape.lying(true, share));
                let entity = commands
                    .spawn((
                        Name::new("moving page"),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(paper[usize::from(!flight.pin)].clone()),
                        from,
                        Visibility::Visible,
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.trigger(PlayAt(Sound::PageTurn, from.translation));
                flight.flyer = Some(Flyer {
                    entity,
                    mesh,
                    from,
                    share,
                    paper,
                });
            }
            let Some(flyer) = &flight.flyer else {
                continue;
            };
            let (pose_at, share, along) = if flight.pin {
                let to = target
                    .with_translation(target.translation + target.rotation * Vec3::Z * IN_FRONT);
                flight.pinning(flyer, to)
            } else {
                let Ok((spine, ..)) = sheets.get(parts.spine) else {
                    continue;
                };
                let spine = spine.compute_transform();
                let index = flight
                    .sheet
                    .number(&filed.contents)
                    .and_then(|number| filed.in_file.from(number))
                    .and_then(|page| stack.index_of(page))
                    .unwrap_or(usize::MAX);
                let hung = spine * stack.sheet_before(index, pose.cover(), stray(flight.seed));
                flight.unpinning(flyer, hung, spine.rotation * Vec3::NEG_Z)
            };
            if let Ok(mut transform) = transforms.get_mut(flyer.entity) {
                *transform = pose_at;
            }
            if let Some(mut mesh) = assets.1.get_mut(&flyer.mesh) {
                *mesh = flight.shape.lying(true, share);
            }
            if !flight.swapped && along >= 0.5 {
                flight.swapped = true;
                if let Some(print) = flight.print.take() {
                    commands.entity(print).despawn();
                }
                flight.print = if flight.pin {
                    flight.sheet.spawn(&mut commands, &filed.flight)
                } else {
                    let full = run.pins().is_full();
                    write_filed(&mut commands, file, filed, flight.sheet, full)
                };
                let side = usize::from(flight.pin);
                commands
                    .entity(flyer.entity)
                    .insert(MeshMaterial3d(flyer.paper[side].clone()));
                if !flight.pin {
                    commands.trigger(PlayAt(Sound::PageTurn, pose_at.translation));
                }
            }
            if !flight.arrived && flight.elapsed >= flight.arrive() {
                flight.arrived = true;
                let pins = run.pins_mut();
                if flight.pin {
                    pins.pin(flight.sheet);
                    commands.entity(slot).insert(Pressed {
                        relaxed: 0.0,
                        relaxing: true,
                    });
                } else {
                    pins.unpin(flight.sheet);
                }
            }
            if flight.elapsed >= flight.end() {
                commands.entity(flyer.entity).despawn();
                if let Some(print) = flight.print.take() {
                    commands.entity(print).despawn();
                }
                parts.open_rings(0.0, &mut transforms);
                moves.flight = None;
            }
        }
    }
}

impl Flight {
    /// When the page arrives at the rail or on the rings.
    fn arrive(&self) -> f32 {
        if self.pin {
            OPEN_TIME + LIFT_TIME + FLY_TIME
        } else {
            OPEN_TIME + FLY_TIME + DROP_TIME
        }
    }

    /// When the rings start to shut. They shut behind a pinned page while
    /// it flies.
    fn shut_start(&self) -> f32 {
        if self.pin {
            OPEN_TIME + LIFT_TIME + FLY_TIME / 2.0
        } else {
            self.arrive()
        }
    }

    fn end(&self) -> f32 {
        (self.arrive() + LINGER).max(self.shut_start() + SHUT_TIME)
    }

    /// How far the rings are open, from 0 shut to 1.
    fn rings(&self) -> f32 {
        let shut = self.shut_start();
        if self.elapsed < shut {
            smoothstep(self.elapsed / OPEN_TIME)
        } else {
            1.0 - smoothstep((self.elapsed - shut) / SHUT_TIME)
        }
    }

    /// Where a pinned page is on its way to `to` on the rail, the share of
    /// its wrinkles and how far it has flown, from 0 to 1. It lifts off the
    /// rings and slides away from them, then flies over, flattening on the
    /// way.
    fn pinning(&self, flyer: &Flyer, to: Transform) -> (Transform, f32, f32) {
        let lifting = (self.elapsed - OPEN_TIME) / LIFT_TIME;
        let off = flyer.from.rotation * Vec3::new(SLIDE, 0.0, LIFT);
        let mut lifted = flyer.from;
        lifted.translation += off * smoothstep(lifting);
        if lifting < 1.0 {
            return (lifted, flyer.share, 0.0);
        }
        let along = ((self.elapsed - OPEN_TIME - LIFT_TIME) / FLY_TIME).min(1.0);
        let share = flyer.share * (1.0 - smoothstep(along / 0.6));
        (arc(lifted, to, along), share, along)
    }

    /// Where a page that goes back is on its way to hang at `hung` in the
    /// file, how much of its wrinkles it shows and how far it has flown,
    /// from 0 to 1. It flies to above the file and drops in against `up`,
    /// which runs along the rings to the top of the file.
    fn unpinning(&self, flyer: &Flyer, hung: Transform, up: Vec3) -> (Transform, f32, f32) {
        let mut above = hung;
        above.translation += up * (COVER.y + ABOVE);
        let along = ((self.elapsed - OPEN_TIME) / FLY_TIME).min(1.0);
        if along < 1.0 {
            return (arc(flyer.from, above, along), 0.0, along);
        }
        let dropping = ((self.elapsed - OPEN_TIME - FLY_TIME) / DROP_TIME).min(1.0);
        let fall = dropping * dropping;
        (
            above.with_translation(above.translation.lerp(hung.translation, fall)),
            0.0,
            1.0,
        )
    }
}

/// Writes `sheet` the way the file prints it on the canvas for flights.
fn write_filed(
    commands: &mut Commands,
    file: Entity,
    filed: &Filed,
    sheet: Sheet,
    rail_full: bool,
) -> Option<Entity> {
    let contents = &filed.contents;
    let pages = contents.pages();
    let number = sheet.number(contents)?;
    let page = *pages.get(number)?;
    let links = Links {
        file,
        in_file: &filed.in_file,
        rail_full,
    };
    Some(Paper::spawn_page(
        commands,
        &filed.flight,
        (number + 1, pages.len()),
        page.0.footer(),
        |content| contents.write(page, &links, content),
    ))
}

/// `along` of the way from `from` to `to`, in an arc and rolling a little
/// about the way.
fn arc(from: Transform, to: Transform, along: f32) -> Transform {
    let eased = smoothstep(along);
    let middle = from.translation.lerp(to.translation, 0.5) + Vec3::Y * ARC;
    let near = from.translation.lerp(middle, eased);
    let far = middle.lerp(to.translation, eased);
    let way = (to.translation - from.translation).normalize_or_zero();
    let roll = Quat::from_axis_angle(way, ROLL * (PI * eased).sin());
    Transform {
        translation: near.lerp(far, eased),
        rotation: roll * from.rotation.slerp(to.rotation, eased),
        scale: from.scale.lerp(to.scale, eased),
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
