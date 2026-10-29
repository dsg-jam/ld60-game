pub(super) mod arm;
pub(super) mod sway;

use std::f32::consts::FRAC_PI_2;

use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_game_core::parts::paper::{Flutter, Paper};
use ld60_game_core::parts::screen::UiCanvas;
use ld60_game_core::player::camera::{PlayerCamera, framing};
use ld60_game_core::player::focus::{Aside, Clickable, Focus, FocusOn};
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

use self::arm::Arm;
use self::sway::Sway;
use super::monitor::Monitor;
use crate::workshop::manuals::{MoveSheet, Pins, RailSlot, Sheet};
use crate::workshop::tutorial::FirstNote;

/// A page of the manuals file at five eighths of its size, so the sheets
/// leave the tools behind them in sight.
const SHEET_SIZE: Vec2 = Vec2::new(0.1, 0.1375);
const CANVAS_SIZE: UVec2 = UVec2::new(640, 880);
/// Along the rod, between the hinge and the first sheet.
const MONITOR_GAP: f32 = 0.022;
const SHEET_GAP: f32 = 0.006;
/// How far each sheet hangs in front of the one before, so tilted sheets
/// overlap instead of cutting through each other.
const SHEET_STAGGER: f32 = 0.0012;
/// How far the rod reaches past the last sheet.
const ROD_END: f32 = 0.008;
const ROD_RADIUS: f32 = 0.0025;
/// The plate on the side of the monitor that holds the hinge. Its top sits
/// [`MOUNT_DROP`] below the top of the monitor and its front [`MOUNT_BACK`]
/// behind the front, so it stays clear of both edges.
const MOUNT: Vec3 = Vec3::new(0.004, 0.022, 0.03);
const MOUNT_DROP: f32 = 0.004;
const MOUNT_BACK: f32 = 0.006;
/// The upright pin of the hinge, on the outside of the mount.
const KNUCKLE: Vec2 = Vec2::new(0.009, 0.014);
/// How far the rod swings forward from the side of the monitor, in
/// radians.
const SWING: f32 = 0.5;
const CLIP: Vec3 = Vec3::new(0.014, 0.009, 0.006);
/// How far the clips reach above the rod.
const CLIP_ABOVE: f32 = 0.001;
/// How far a sheet reaches up into its clip.
const CLIP_GRIP: f32 = 0.003;
/// Between the rod's axis and the sheets.
const SHEET_SPACE: f32 = 0.0005;
/// Free space around the sheets while looking at them, as a fraction of the
/// window.
const VIEW_MARGIN: f32 = 0.06;

/// The monitor shades the sheets, so they give off a little warm light of
/// their own to stay readable and look like the pages in the file.
const SHEET_GLOW: LinearRgba = LinearRgba::rgb(0.42, 0.32, 0.2);

const METAL: Color = Color::srgb_u8(0x8c, 0x90, 0x94);
const CLIP_COLOR: Color = Color::srgb_u8(0x30, 0x30, 0x34);

/// Sheets of the manuals file hanging from a rail on the left side of the
/// monitor, see [`Pins`]. The [`Aside`] of the computer looks at them. The
/// rail folds back while no sheet is pinned, see [`Arm`].
#[derive(Component)]
pub(super) struct PinBoard {
    /// Where the rail is fixed to the monitor, swung out.
    rail_space: Transform,
    slots: Vec<Slot>,
    /// The pins the slots show.
    shown: Option<Pins>,
}

/// A sheet hanging from the rail. A click on it, from the seat or while
/// looking at the sheets from the computer, unpins it and puts it back in
/// the manuals file.
#[derive(Component, Debug)]
pub(super) struct Hanging(Sheet);

/// Where one pinned sheet hangs.
struct Slot {
    canvas: UiCanvas,
    sheet: Entity,
    /// The center of the sheet along the rail.
    center: Vec3,
    page: Option<Entity>,
}

impl PinBoard {
    /// Spawns the rail and the clips next to `monitor` as children of
    /// `parent`, which owns the canvases.
    pub(super) fn spawn(
        commands: &mut Commands,
        parent: Entity,
        monitor: &Monitor,
        images: &mut Assets<Image>,
        meshes: &mut Assets<Mesh>,
    ) -> Self {
        let body = monitor.body;
        let mount = Block::new(MOUNT).left_of(&body).translated(Vec3::new(
            0.0,
            body.max().y - MOUNT_DROP - MOUNT.y,
            body.max().z - MOUNT_BACK - MOUNT.z / 2.0,
        ));
        let hinge = Vec3::new(
            mount.min().x - KNUCKLE.x / 2.0,
            mount.center.y,
            mount.max().z - KNUCKLE.x / 2.0,
        );
        let knuckle = Block::new(Vec3::new(KNUCKLE.x, KNUCKLE.y, KNUCKLE.x))
            .translated(hinge - Vec3::Y * KNUCKLE.y / 2.0);
        let rail_space =
            Transform::from_translation(hinge).with_rotation(Quat::from_rotation_y(SWING));
        let length = MONITOR_GAP
            + Pins::MAX as f32 * SHEET_SIZE.x
            + (Pins::MAX - 1) as f32 * SHEET_GAP
            + ROD_END;
        let rail = Block::between(
            Vec3::new(-length, -ROD_RADIUS, -ROD_RADIUS),
            Vec3::new(KNUCKLE.x / 2.0, ROD_RADIUS, ROD_RADIUS),
        );
        // Lies along the rod, which the capsule stands up along.
        let rod = (
            Transform::from_translation(rail.center)
                .with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
            Visibility::default(),
            children![(
                Shape::capsule(
                    &Block::new(Vec3::new(rail.size.y, rail.size.x, rail.size.z))
                        .translated(Vec3::NEG_Y * rail.size.x / 2.0),
                    METAL,
                ),
                Finish::Metal,
            )],
        );
        let canvases: Vec<_> = (0..Pins::MAX)
            .map(|_| UiCanvas::spawn_print(commands, images, CANVAS_SIZE, parent))
            .collect();
        let mut slots = Vec::new();
        commands.entity(parent).with_children(|computer| {
            computer.spawn((
                Transform::default(),
                Visibility::default(),
                children![
                    (Shape::cuboid(&mount, METAL), Finish::Metal),
                    (Shape::cylinder(&knuckle, METAL), Finish::Metal),
                ],
            ));
            let arm = Arm::new(SWING);
            computer
                .spawn((
                    Name::new("pin rail"),
                    rail_space.with_rotation(arm.rotation()),
                    arm,
                    Visibility::default(),
                ))
                .with_children(|board| {
                    board.spawn(rod);
                    for (index, canvas) in canvases.into_iter().enumerate() {
                        let x = -MONITOR_GAP
                            - (SHEET_GAP + SHEET_SIZE.x) * index as f32
                            - SHEET_SIZE.x / 2.0;
                        let stagger = SHEET_STAGGER * index as f32;
                        let clip = Block::new(CLIP).translated(Vec3::new(
                            x,
                            ROD_RADIUS + CLIP_ABOVE - CLIP.y,
                            stagger + CLIP.z / 2.0,
                        ));
                        board.spawn((Shape::cuboid(&clip, CLIP_COLOR), Finish::Metal));
                        let center = Vec3::new(
                            x,
                            clip.min().y + CLIP_GRIP - SHEET_SIZE.y / 2.0,
                            stagger - SHEET_SPACE,
                        );
                        let sheet = board
                            .spawn((
                                Paper::hanging_sheet(
                                    &canvas,
                                    ("pinned", index),
                                    SHEET_SIZE,
                                    center,
                                    SHEET_GLOW,
                                    meshes,
                                ),
                                Flutter::default(),
                                RailSlot(index),
                                Sway::new(center.with_y(clip.min().y + CLIP_GRIP), center, index),
                                Visibility::Hidden,
                            ))
                            .id();
                        slots.push(Slot {
                            canvas,
                            sheet,
                            center,
                            page: None,
                        });
                    }
                });
        });
        Self {
            rail_space,
            slots,
            shown: None,
        }
    }

    /// The camera pose that looks straight at the first `count` slots.
    pub(super) fn view(&self, count: usize, perspective: &PerspectiveProjection) -> Transform {
        let shown = &self.slots[..count.clamp(1, self.slots.len())];
        let corners = shown.iter().flat_map(|slot| {
            [Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0)].map(|corner| {
                self.rail_space
                    .transform_point(slot.center + (corner * SHEET_SIZE / 2.0).extend(0.0))
            })
        });
        framing(self.rail_space.rotation, corners, perspective, VIEW_MARGIN)
    }

    /// Hangs the pinned sheets. The [`Aside`] goes back to the screen once
    /// none are left.
    pub(super) fn follow_pins(
        pins: Res<Pins>,
        mut boards: Query<(&mut PinBoard, &mut Aside)>,
        mut visibilities: Query<&mut Visibility>,
        mut commands: Commands,
    ) {
        for (mut board, mut aside) in &mut boards {
            if board.shown.as_ref() == Some(&*pins) {
                continue;
            }
            let slots = pins.slots();
            let shown = |shown: bool| {
                if shown {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                }
            };
            for (index, slot) in board.slots.iter_mut().enumerate() {
                if let Some(page) = slot.page.take() {
                    commands.entity(page).despawn();
                }
                slot.page = slots[index].and_then(|sheet| {
                    commands
                        .entity(slot.sheet)
                        .insert((sheet.print(), Hanging(sheet)));
                    sheet.spawn(&mut commands, &slot.canvas)
                });
                if slot.page.is_none() {
                    commands.entity(slot.sheet).remove::<Hanging>();
                }
                if let Ok(mut visibility) = visibilities.get_mut(slot.sheet) {
                    visibility.set_if_neq(shown(slot.page.is_some()));
                }
            }
            if pins.is_empty() {
                aside.open = false;
            }
            board.shown = Some(pins.clone());
        }
    }

    /// The hanging sheets say what a click does from the seat and at the
    /// computer.
    pub(super) fn arm(
        state: Res<State<Focus>>,
        boards: Query<&Aside, With<PinBoard>>,
        sheets: Query<(Entity, Option<&Clickable>), With<Hanging>>,
        mut commands: Commands,
    ) {
        let prompt = Self::click(&state, &boards).map(|click| match click {
            SheetClick::Unpin => "click-unpin",
            SheetClick::LookAt => "click-look-pins",
        });
        for (sheet, clickable) in &sheets {
            if clickable.map(|clickable| clickable.prompt) == prompt {
                continue;
            }
            match prompt {
                Some(prompt) => commands.entity(sheet).insert(Clickable { prompt }),
                None => commands.entity(sheet).remove::<Clickable>(),
            };
        }
    }

    /// What a click on a hanging sheet does. In the wide view of the
    /// computer the sheets only peek in from the side, so a click there
    /// turns to them first.
    fn click(state: &State<Focus>, boards: &Query<&Aside, With<PinBoard>>) -> Option<SheetClick> {
        match state.get() {
            Focus::Desk => Some(SheetClick::Unpin),
            Focus::Computer if boards.iter().any(|aside| !aside.open) => Some(SheetClick::LookAt),
            Focus::Computer => Some(SheetClick::Unpin),
            _ => None,
        }
    }

    /// Shows the first note on the rail once the player sits at the bench,
    /// so the camera does not swing to it on its way in from the street.
    pub(super) fn show_first_note(
        camera: Single<(&PlayerCamera, &Transform)>,
        mut boards: Query<(Entity, &mut Aside), With<PinBoard>>,
        mut commands: Commands,
    ) {
        let (camera, at) = *camera;
        if !camera.seated(at) {
            return;
        }
        commands.remove_resource::<FirstNote>();
        for (board, mut aside) in &mut boards {
            aside.open = true;
            commands.trigger(FocusOn { entity: board });
        }
    }

    pub(super) fn put_back_on_press(
        mut press: On<PointerPress>,
        state: Res<State<Focus>>,
        sheets: Query<&Hanging>,
        mut boards: Query<&mut Aside, With<PinBoard>>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary {
            return;
        }
        let Ok(hanging) = sheets.get(press.entity) else {
            return;
        };
        match Self::click(&state, &boards.as_readonly()) {
            Some(SheetClick::Unpin) => commands.trigger(MoveSheet::Unpin(hanging.0)),
            Some(SheetClick::LookAt) => {
                for mut aside in &mut boards {
                    aside.open = true;
                }
            }
            None => return,
        }
        press.propagate(false);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SheetClick {
    Unpin,
    LookAt,
}
