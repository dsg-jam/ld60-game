use bevy::prelude::*;
use ld60_game_parts::dot_matrix::DotMatrix;
use ld60_geometry::block::Block;
use ld60_geometry::frame::Frame;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::{Finish, Form, SurfaceMaterial};

use super::CASE;

/// Wood shows around the front panel.
const BORDER: f32 = 0.012;
const PANEL: f32 = 0.004;
/// Width of the dark rim between the wooden case and the metal front panel.
const RIM: f32 = 0.0015;
/// How far the rim stands out of the wood.
const RIM_DEPTH: f32 = 0.0006;
const GRILLE: Vec2 = Vec2::new(0.13, 0.12);
/// Space between the grille and the left edge of the panel.
const GRILLE_INSET: f32 = 0.012;
/// How far the grille sits behind the front of the panel.
const GRILLE_RECESS: f32 = 0.0025;
/// The window in the panel that shows the dial.
const DIAL: Vec2 = Vec2::new(0.11, 0.04);
const DIAL_TOP: f32 = 0.125;
/// How far the dial's scale sits behind the front of the panel.
const DIAL_RECESS: f32 = 0.003;
const DISPLAY_TOP: f32 = 0.165;
/// How far the display stands out of the panel.
const DISPLAY_PROUD: f32 = 0.003;
const KNOB_Y: f32 = 0.057;
/// Distance of the knobs from the middle of the controls.
const KNOB_SPREAD: f32 = 0.03;

const FACE: Color = Color::srgb_u8(0xd9, 0xcf, 0xb4);
const GRILLE_METAL: Color = Color::srgb_u8(0x9a, 0x8a, 0x68);
const DARK: Color = Color::srgb_u8(0x22, 0x1e, 0x1a);

/// Where the parts on the front panel go.
pub(super) struct Front {
    panel: Block,
    grille: Block,
    dial: Block,
    display: Block,
}

impl Front {
    /// The front for `display`, which plugs into it.
    pub(super) fn new(display: &DotMatrix) -> Self {
        let back = CASE.z / 2.0;
        let face = back + PANEL;
        let panel = Self::panel();
        let grille = Self::grille();
        // The dial, the display and the knobs share the space right of the
        // grille.
        let middle = (grille.max().x + panel.max().x) / 2.0;
        let dial = Block::between(
            Vec3::new(middle - DIAL.x / 2.0, DIAL_TOP - DIAL.y, back),
            Vec3::new(middle + DIAL.x / 2.0, DIAL_TOP, face - DIAL_RECESS),
        );
        let outer = display.outer_size();
        let display = Block::between(
            Vec3::new(middle - outer.x / 2.0, DISPLAY_TOP - outer.y, face),
            Vec3::new(middle + outer.x / 2.0, DISPLAY_TOP, face + DISPLAY_PROUD),
        );
        Self {
            panel,
            grille,
            dial,
            display,
        }
    }

    fn panel() -> Block {
        Block::between(
            Vec3::new(-CASE.x / 2.0 + BORDER, BORDER, CASE.z / 2.0),
            Vec3::new(CASE.x / 2.0 - BORDER, CASE.y - BORDER, CASE.z / 2.0 + PANEL),
        )
    }

    /// The speaker grille, behind the left of the panel.
    pub(super) fn grille() -> Block {
        let panel = Self::panel();
        let left = panel.min().x + GRILLE_INSET;
        Block::between(
            Vec3::new(left, (CASE.y - GRILLE.y) / 2.0, panel.min().z),
            Vec3::new(
                left + GRILLE.x,
                (CASE.y + GRILLE.y) / 2.0,
                panel.max().z - GRILLE_RECESS,
            ),
        )
    }

    /// The front of the display, where it sits.
    pub(super) fn display(&self) -> Transform {
        Transform::from_translation(self.display.center.with_z(self.display.max().z))
    }

    /// The middle of the dial's scale behind the window, and its size.
    pub(super) fn dial(&self) -> (Transform, Vec2) {
        let at = self.dial.center.truncate().extend(self.dial.max().z);
        (Transform::from_translation(at), self.dial.size.truncate())
    }

    /// The knob left below the dial.
    pub(super) fn left_knob(&self) -> Transform {
        self.knob(-KNOB_SPREAD)
    }

    /// The knob right below the dial.
    pub(super) fn right_knob(&self) -> Transform {
        self.knob(KNOB_SPREAD)
    }

    fn knob(&self, x: f32) -> Transform {
        Transform::from_xyz(self.dial.center.x + x, KNOB_Y, self.panel.max().z)
    }

    /// The painted metal panel on a dark rim, with openings for the grille
    /// set back behind it, the dial and the display.
    pub(super) fn spawn_panel(
        &self,
        radio: &mut ChildSpawnerCommands,
        meshes: &mut Assets<Mesh>,
        surfaces: &mut Assets<SurfaceMaterial>,
    ) {
        let gap = Vec3::new(RIM, RIM, 0.0);
        let rim = Block::between(
            self.panel.min() - gap,
            self.panel.max().with_z(self.panel.min().z + RIM_DEPTH) + gap,
        );
        radio.spawn((Shape::cuboid(&rim, DARK), Finish::Plastic));
        let frame = Frame {
            outer: self.panel,
            holes: &[self.grille, self.dial, self.display],
        };
        radio.spawn((
            Mesh3d(meshes.add(frame.mesh())),
            MeshMaterial3d(surfaces.add(Finish::Paint.material(FACE, Form::Freeform, 0.0))),
        ));
        radio.spawn((
            Shape::cuboid(&self.grille, GRILLE_METAL),
            Finish::Perforated,
        ));
    }
}
