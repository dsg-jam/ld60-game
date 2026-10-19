use bevy::prelude::*;

use super::monitor::Monitor;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;
use crate::parts::paper::Paper;
use crate::parts::screen::UiCanvas;
use crate::player::focus::Aside;
use crate::workshop::manuals::Pins;
use crate::workshop::run::Run;

/// A page of the manuals file at five eighths of its size, so the sheets
/// leave the tools behind them in sight.
const SHEET_SIZE: Vec2 = Vec2::new(0.1, 0.1375);
const CANVAS_SIZE: UVec2 = UVec2::new(640, 880);
/// Between the monitor and the first sheet. It keeps the sheets out of
/// sight while the player looks at the screen.
const MONITOR_GAP: f32 = 0.05;
const SHEET_GAP: f32 = 0.01;
const RAIL_THICKNESS: f32 = 0.006;
/// How far the rail sits below the top of the monitor.
const RAIL_DROP: f32 = 0.004;
/// How far the rail sits behind the front of the monitor.
const RAIL_BACK: f32 = 0.012;
/// How far the rail swings forward from the left side of the monitor, in
/// radians.
const SWING: f32 = 0.5;
const MOUNT: Vec3 = Vec3::new(0.01, 0.025, 0.03);
const CLIP: Vec3 = Vec3::new(0.014, 0.009, 0.003);
/// How far the clips reach up over the rail.
const CLIP_OVERLAP: f32 = 0.003;
/// How far a sheet reaches up into its clip.
const CLIP_GRIP: f32 = 0.003;
/// Between the back of a clip and the sheet it holds.
const SHEET_SPACE: f32 = 0.0005;
/// Distance of the camera from the sheets while looking at them.
const VIEW_DISTANCE: f32 = 0.18;

/// The monitor shades the sheets, so they give off a little warm light of
/// their own to stay readable and look like the pages in the file.
const SHEET_GLOW: LinearRgba = LinearRgba::rgb(0.42, 0.32, 0.2);

const METAL: Color = Color::srgb_u8(0x8c, 0x90, 0x94);
const CLIP_COLOR: Color = Color::srgb_u8(0x30, 0x30, 0x34);

/// Sheets of the manuals file hanging from a rail on the left side of the
/// monitor, see [`Pins`]. The [`Aside`] of the computer looks at them. The
/// rail only shows while a sheet is pinned.
#[derive(Component)]
pub(super) struct PinBoard {
    /// Where the rail is fixed to the monitor, swung forward.
    rail_space: Transform,
    /// The mount and the rail with its clips.
    parts: [Entity; 2],
    slots: Vec<Slot>,
    /// The pins the slots show.
    shown: Option<Pins>,
}

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
        let hinge = Vec3::new(
            body.min().x,
            body.max().y - RAIL_DROP - RAIL_THICKNESS / 2.0,
            body.max().z - RAIL_BACK,
        );
        let rail_space =
            Transform::from_translation(hinge).with_rotation(Quat::from_rotation_y(SWING));
        let mount = Block::new(MOUNT).left_of(&body).translated(Vec3::new(
            0.0,
            hinge.y - MOUNT.y / 2.0,
            hinge.z,
        ));
        let length = MONITOR_GAP + Pins::MAX as f32 * (SHEET_SIZE.x + SHEET_GAP);
        let rail = Block::between(
            Vec3::new(-length, -RAIL_THICKNESS / 2.0, -RAIL_THICKNESS / 2.0),
            Vec3::splat(RAIL_THICKNESS / 2.0).with_x(0.0),
        );
        let canvases: Vec<_> = (0..Pins::MAX)
            .map(|_| UiCanvas::spawn_print(commands, images, CANVAS_SIZE, parent))
            .collect();
        let mut slots = Vec::new();
        let mut parts = [Entity::PLACEHOLDER; 2];
        commands.entity(parent).with_children(|computer| {
            parts[0] = computer
                .spawn((
                    Shape::cuboid(&mount, METAL),
                    Finish::Metal,
                    Visibility::Hidden,
                ))
                .id();
            parts[1] = computer
                .spawn((Name::new("pin rail"), rail_space, Visibility::Hidden))
                .with_children(|board| {
                    board.spawn((Shape::cuboid(&rail, METAL), Finish::Metal));
                    for (index, canvas) in canvases.into_iter().enumerate() {
                        let x = -MONITOR_GAP
                            - (SHEET_GAP + SHEET_SIZE.x) * index as f32
                            - SHEET_SIZE.x / 2.0;
                        let clip = Block::new(CLIP).below(&rail).translated(Vec3::new(
                            x,
                            CLIP_OVERLAP,
                            rail.max().z + CLIP.z / 2.0,
                        ));
                        board.spawn((Shape::cuboid(&clip, CLIP_COLOR), Finish::Metal));
                        let center = Vec3::new(
                            x,
                            clip.min().y + CLIP_GRIP - SHEET_SIZE.y / 2.0,
                            clip.min().z - SHEET_SPACE,
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
                })
                .id();
        });
        Self {
            rail_space,
            parts,
            slots,
            shown: None,
        }
    }

    /// The camera pose that looks straight at the first `count` sheets.
    fn view(&self, count: usize) -> Transform {
        let shown = &self.slots[..count.clamp(1, self.slots.len())];
        let center = shown.iter().map(|slot| slot.center).sum::<Vec3>() / shown.len() as f32;
        let eye = center + Vec3::Z * VIEW_DISTANCE;
        let at = self.rail_space.transform_point(center);
        Transform::from_translation(self.rail_space.transform_point(eye)).looking_at(at, Vec3::Y)
    }

    /// Hangs the pinned sheets and points the [`Aside`] at them.
    pub(super) fn follow_pins(
        run: Res<Run>,
        mut boards: Query<(&mut PinBoard, &mut Aside)>,
        mut visibilities: Query<&mut Visibility>,
        mut commands: Commands,
    ) {
        for (mut board, mut aside) in &mut boards {
            if board.shown.as_ref() == Some(run.pins()) {
                continue;
            }
            let sheets = run.pins().sheets();
            let shown = |shown: bool| {
                if shown {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                }
            };
            for part in board.parts {
                if let Ok(mut visibility) = visibilities.get_mut(part) {
                    visibility.set_if_neq(shown(!sheets.is_empty()));
                }
            }
            for (index, slot) in board.slots.iter_mut().enumerate() {
                if let Some(page) = slot.page.take() {
                    commands.entity(page).despawn();
                }
                slot.page = sheets.get(index).and_then(|sheet| {
                    commands.entity(slot.sheet).insert(sheet.print());
                    sheet.spawn(&mut commands, &slot.canvas)
                });
                if let Ok(mut visibility) = visibilities.get_mut(slot.sheet) {
                    visibility.set_if_neq(shown(slot.page.is_some()));
                }
            }
            aside.view = board.view(sheets.len());
            if run.pins().is_empty() {
                aside.open = false;
            }
            board.shown = Some(run.pins().clone());
        }
    }
}
