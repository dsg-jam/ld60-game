use std::f32::consts::FRAC_PI_2;

use bevy::{
    input::mouse::MouseScrollUnit,
    picking::{
        events::{PointerDrag, PointerPress, PointerScroll},
        pointer::PointerButton,
    },
    prelude::*,
};

use crate::audio::{Play, Sound};
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::Finish;
use crate::player::focus::Focused;

const RADIUS: f32 = 0.016;
const DEPTH: f32 = 0.018;
/// The line on the cap. It starts a little off the middle and reaches
/// almost to the edge.
const MARK: Vec3 = Vec3::new(0.002, 0.001, RADIUS * 0.8);
const MARK_START: f32 = RADIUS * 0.1;
const CAP: Color = Color::srgb_u8(0x22, 0x1e, 0x1a);
const MARK_COLOR: Color = Color::srgb_u8(0xd9, 0xcf, 0xb4);
/// How far a smooth knob turns from one end to the other, in radians.
const SWEEP: f32 = 4.7;
/// How far a stepped knob turns from one step to the next, in radians.
const STEP: f32 = 0.7;
const PER_PIXEL: f32 = 0.004;
const PER_LINE: f32 = 0.05;
/// Browsers scroll in pixels, about this many per notch of the wheel.
const PIXELS_PER_LINE: f32 = 100.0;
/// A smooth knob ticks softly this many times from one end to the other.
const RATCHET: f32 = 24.0;

pub(crate) struct KnobPlugin;

impl Plugin for KnobPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Knob::show)
            .add_observer(Knob::on_add)
            .add_observer(Knob::press)
            .add_observer(Knob::drag)
            .add_observer(Knob::scroll);
    }
}

/// A round knob sticking out of a panel along +Z, with a line that shows how
/// far it is turned. It turns while the device it is on is focused.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub(crate) enum Knob {
    /// Turns smoothly from 0 to 1, with a soft ratchet, by dragging or
    /// scrolling.
    Smooth(f32),
    /// Snaps to one of `count` positions. A click moves on to the next one,
    /// round to the first after the last.
    Stepped { position: usize, count: usize },
}

impl Knob {
    /// How far the knob is turned, from 0 to 1.
    pub(crate) fn value(&self) -> f32 {
        match *self {
            Self::Smooth(value) => value,
            Self::Stepped { position, count } => position as f32 / (count.max(2) - 1) as f32,
        }
    }

    /// The position of a stepped knob.
    pub(crate) fn position(&self) -> Option<usize> {
        match *self {
            Self::Smooth(_) => None,
            Self::Stepped { position, .. } => Some(position),
        }
    }

    /// Turns a smooth knob by `y` of the mouse wheel. Returns whether the
    /// knob took it. Stepped knobs don't, so the wheel can do something
    /// else.
    pub(crate) fn wheel(&mut self, unit: MouseScrollUnit, y: f32, commands: &mut Commands) -> bool {
        if !matches!(self, Self::Smooth(_)) {
            return false;
        }
        let lines = match unit {
            MouseScrollUnit::Line => y,
            MouseScrollUnit::Pixel => y / PIXELS_PER_LINE,
        };
        self.turn(lines * PER_LINE, commands);
        true
    }

    fn on_add(add: On<Add<Knob>>, mut commands: Commands) {
        let mark = Block::new(MARK).translated(Vec3::new(0.0, DEPTH, -MARK.z / 2.0 - MARK_START));
        let cap = Block::new(Vec3::new(2.0 * RADIUS, DEPTH, 2.0 * RADIUS));
        commands.entity(add.entity).with_children(|knob| {
            knob.spawn((
                Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                Visibility::default(),
            ))
            .with_children(|turned| {
                turned.spawn((Shape::cylinder(&cap, CAP), Finish::Plastic));
                turned.spawn(Shape::cuboid(&mark, MARK_COLOR));
            });
        });
    }

    /// Turns a smooth knob by `by`, ticking as it passes each step of the
    /// ratchet.
    fn turn(&mut self, by: f32, commands: &mut Commands) {
        if let Self::Smooth(value) = self {
            let before = (*value * RATCHET).floor();
            *value = (*value + by).clamp(0.0, 1.0);
            if (*value * RATCHET).floor() != before {
                commands.trigger(Play(Sound::Ratchet));
            }
        }
    }

    fn press(
        mut press: On<PointerPress>,
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        mut knobs: Query<&mut Knob>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || !focused.holds(press.entity, &parents) {
            return;
        }
        let Ok(mut knob) = knobs.get_mut(press.entity) else {
            return;
        };
        press.propagate(false);
        if let Self::Stepped { position, count } = &mut *knob {
            *position = (*position + 1) % (*count).max(1);
            commands.trigger(Play(Sound::Detent));
        }
    }

    fn drag(
        mut drag: On<PointerDrag>,
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        mut knobs: Query<&mut Knob>,
        mut commands: Commands,
    ) {
        if drag.button != PointerButton::Primary || !focused.holds(drag.entity, &parents) {
            return;
        }
        if let Ok(mut knob) = knobs.get_mut(drag.entity) {
            drag.propagate(false);
            // Dragging up or right turns it up.
            knob.turn((drag.delta.x - drag.delta.y) * PER_PIXEL, &mut commands);
        }
    }

    fn scroll(
        mut scroll: On<PointerScroll>,
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        mut knobs: Query<&mut Knob>,
        mut commands: Commands,
    ) {
        if !focused.holds(scroll.entity, &parents) {
            return;
        }
        let Ok(mut knob) = knobs.get_mut(scroll.entity) else {
            return;
        };
        if knob.wheel(scroll.unit, scroll.y, &mut commands) {
            scroll.propagate(false);
        }
    }

    fn show(mut knobs: Query<(&Knob, &mut Transform), Changed<Knob>>) {
        for (knob, mut transform) in &mut knobs {
            let turn = match *knob {
                Self::Smooth(value) => (value - 0.5) * SWEEP,
                Self::Stepped { position, .. } => position as f32 * STEP,
            };
            transform.rotation = Quat::from_rotation_z(-turn);
        }
    }
}
