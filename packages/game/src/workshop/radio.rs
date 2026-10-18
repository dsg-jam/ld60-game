mod broadcast;
mod front;

use bevy::{picking::events::PointerScroll, prelude::*};

use self::broadcast::{TRACKS, Tuner};
use self::front::Front;
use crate::geometry::block::Block;
use crate::geometry::shape::Shape;
use crate::geometry::surface::{Finish, SurfaceMaterial};
use crate::parts::dot_matrix::DotMatrix;
use crate::parts::gauge::Gauge;
use crate::parts::knob::Knob;
use crate::parts::latch_button::LatchButton;
use crate::player::focus::{Focus, FocusTarget, Focused, View};

const CASE: Vec3 = Vec3::new(0.3, 0.19, 0.11);
/// Twelve characters of five dots.
const DISPLAY_COLUMNS: u32 = 60;
const DISPLAY_DOT: f32 = 0.0017;
/// From the right end and the front of the case to the power button.
const POWER_INSET: Vec2 = Vec2::new(0.04, 0.025);
const ANTENNA: Vec2 = Vec2::new(0.006, 0.38);
const ANTENNA_TILT: f32 = 0.5;
/// From the right end of the case to the antenna.
const ANTENNA_INSET: f32 = 0.012;
/// The swivel the antenna turns in, half sunk into the case.
const ANTENNA_MOUNT: Vec2 = Vec2::new(0.014, 0.012);
const START_VOLUME: f32 = 0.5;
/// Camera in front of the radio while focused, a little above it to see the
/// power button on top.
const VIEW: Vec3 = Vec3::new(0.0, 0.26, 0.38);

const WOOD: Color = Color::srgb_u8(0x6a, 0x44, 0x28);
const CHROME: Color = Color::srgb_u8(0xb8, 0xbc, 0xc0);
const RED: Color = Color::srgb_u8(0xd0, 0x2a, 0x1a);

pub(crate) struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(broadcast::BroadcastPlugin)
            .add_systems(Update, (Radio::follow_controls, Radio::show).chain())
            .add_observer(Radio::on_add)
            .add_observer(Radio::scroll_volume);
    }
}

/// An FM radio that plays music while switched on.
///
/// The state follows its controls: the power button, the volume knob and the
/// tuning knob. The dial and the display show it.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[require(Transform, Visibility, Tuner)]
pub(crate) struct Radio {
    on: bool,
    /// From 0 to 1.
    volume: f32,
    /// The station the dial points at.
    station: usize,
}

impl Default for Radio {
    fn default() -> Self {
        Self {
            on: true,
            volume: START_VOLUME,
            station: 0,
        }
    }
}

/// What a control on the radio is for.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    Power,
    Volume,
    Tuning,
}

impl Radio {
    fn on_add(
        add: On<Add<Radio>>,
        radios: Query<&Radio>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    ) {
        let Ok(radio) = radios.get(add.entity) else {
            return;
        };
        let case = Block::new(CASE);
        let display = DotMatrix::new(DISPLAY_COLUMNS, DISPLAY_DOT);
        let front = Front::new(&display);
        let (dial_at, dial_size) = front.dial();
        let power_at = Vec3::new(
            CASE.x / 2.0 - POWER_INSET.x,
            CASE.y,
            CASE.z / 2.0 - POWER_INSET.y,
        );
        let gauge = Gauge {
            size: dial_size,
            marks: TRACKS.len(),
            value: 0.0,
            lit: radio.on,
        };
        let volume = Knob::Smooth(radio.volume);
        let tuning = Knob::Stepped {
            position: radio.station,
            count: TRACKS.len(),
        };
        let power = LatchButton {
            down: radio.on,
            color: RED,
        };

        commands
            .entity(add.entity)
            .insert(FocusTarget {
                prompt: "click-radio",
                focus: Focus::Radio,
                view: View::Local(
                    Transform::from_translation(VIEW).looking_at(Vec3::Y * CASE.y / 2.0, Vec3::Y),
                ),
            })
            .with_children(|parent| {
                parent.spawn((Shape::cuboid(&case, WOOD), Finish::Wood));
                front.spawn_panel(parent, &mut meshes, &mut surfaces);
                parent.spawn((display, front.display()));
                parent.spawn((gauge, dial_at));
                parent.spawn((Control::Volume, volume, front.left_knob()));
                parent.spawn((Control::Tuning, tuning, front.right_knob()));
                parent.spawn((Control::Power, power, Transform::from_translation(power_at)));
                Self::spawn_antenna(parent);
            });
    }

    /// A chrome rod leaning back from a swivel on the top right.
    fn spawn_antenna(radio: &mut ChildSpawnerCommands) {
        radio
            .spawn((
                Transform::from_xyz(CASE.x / 2.0 - ANTENNA_INSET, CASE.y, -CASE.z / 4.0)
                    .with_rotation(Quat::from_rotation_z(-ANTENNA_TILT)),
                Visibility::default(),
            ))
            .with_children(|antenna| {
                let sunk = -Vec3::Y * ANTENNA_MOUNT.y / 2.0;
                let mount =
                    Block::new(Vec3::new(ANTENNA_MOUNT.x, ANTENNA_MOUNT.y, ANTENNA_MOUNT.x))
                        .translated(sunk);
                let rod = Block::new(Vec3::new(ANTENNA.x, ANTENNA.y, ANTENNA.x)).translated(sunk);
                antenna.spawn((Shape::cylinder(&mount, CHROME), Finish::Metal));
                antenna.spawn((Shape::cylinder(&rod, CHROME), Finish::Metal));
            });
    }

    /// The mouse wheel anywhere over the radio turns its volume knob.
    fn scroll_volume(
        mut scroll: On<PointerScroll>,
        focused: Res<Focused>,
        radios: Query<&Children, With<Radio>>,
        mut knobs: Query<(&Control, &mut Knob)>,
        mut commands: Commands,
    ) {
        if focused.get() != Some(scroll.entity) {
            return;
        }
        let Ok(children) = radios.get(scroll.entity) else {
            return;
        };
        scroll.propagate(false);
        for &child in children {
            if let Ok((Control::Volume, mut knob)) = knobs.get_mut(child) {
                knob.wheel(scroll.unit, scroll.y, &mut commands);
            }
        }
    }

    fn follow_controls(
        mut radios: Query<&mut Radio>,
        knobs: Query<(&Control, &Knob, &ChildOf), Changed<Knob>>,
        buttons: Query<(&Control, &LatchButton, &ChildOf), Changed<LatchButton>>,
    ) {
        for (control, knob, child_of) in &knobs {
            let Ok(mut radio) = radios.get_mut(child_of.parent()) else {
                continue;
            };
            match control {
                Control::Volume => {
                    radio.set_if_neq(Radio {
                        volume: knob.value(),
                        ..*radio
                    });
                }
                Control::Tuning => {
                    if let Some(station) = knob.position() {
                        radio.set_if_neq(Radio { station, ..*radio });
                    }
                }
                Control::Power => {}
            }
        }
        for (control, button, child_of) in &buttons {
            if *control == Control::Power
                && let Ok(mut radio) = radios.get_mut(child_of.parent())
            {
                radio.set_if_neq(Radio {
                    on: button.down,
                    ..*radio
                });
            }
        }
    }

    /// The dial points at the station and lights up while the radio is on.
    /// The display shows who and what is playing.
    #[expect(clippy::type_complexity, reason = "Bevy query")]
    fn show(
        radios: Query<(&Radio, &Tuner, &Children), Or<(Changed<Radio>, Changed<Tuner>)>>,
        mut gauges: Query<&mut Gauge>,
        mut screens: Query<&mut DotMatrix>,
    ) {
        for (radio, tuner, children) in &radios {
            for &child in children {
                if let Ok(mut gauge) = gauges.get_mut(child) {
                    gauge.value = gauge.mark(radio.station);
                    gauge.lit = radio.on;
                }
                if let Ok(mut screen) = screens.get_mut(child) {
                    if let Some(track) = tuner.track().map(|track| &TRACKS[track]) {
                        screen.set_pages(&[track.artist, track.title]);
                    }
                    screen.lit = radio.on;
                }
            }
        }
    }
}
