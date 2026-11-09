use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};
use ld60_game_audio::{Play, Sound};
use ld60_game_player::cursor::WantsFinger;
use ld60_game_player::focus::Focused;
use ld60_geometry::block::Block;
use ld60_geometry::shape::Shape;
use ld60_geometry::surface::Finish;

/// One cell of the housing, so that cells side by side along X make a
/// row of switches. Its slot, and the slider in the slot.
pub const CELL: Vec3 = Vec3::new(0.012, 0.005, 0.022);
const SLOT: Vec3 = Vec3::new(0.0065, 0.001, 0.015);
const SLIDER: Vec3 = Vec3::new(0.0055, 0.003, 0.006);
/// How far the slider sits from the middle of its slot, either way.
const TRAVEL: f32 = 0.0042;
/// Seconds the slider takes to slide over.
const SLIDE_TIME: f32 = 0.05;

const HOUSING: Color = Color::srgb_u8(0x2a, 0x4c, 0x9c);
const SLOT_COLOR: Color = Color::srgb_u8(0x14, 0x18, 0x22);
const SLIDER_COLOR: Color = Color::srgb_u8(0xf0, 0xee, 0xe6);

pub struct DipPlugin;

impl Plugin for DipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, DipSwitch::show)
            .add_observer(DipSwitch::on_add)
            .add_observer(DipSwitch::flip);
    }
}

/// A slide switch in a cell of a DIP housing, standing on the panel at
/// the origin. On slides it towards -Z, the back of the panel. A click
/// flips it while the thing it is on is focused.
#[derive(Component, Debug, PartialEq, Eq)]
#[require(Transform, Visibility, WantsFinger)]
pub struct DipSwitch {
    pub on: bool,
}

/// The slider of a [`DipSwitch`].
#[derive(Component, Debug)]
struct Slider;

impl DipSwitch {
    fn on_add(add: On<Add<DipSwitch>>, switches: Query<&DipSwitch>, mut commands: Commands) {
        let Ok(switch) = switches.get(add.entity) else {
            return;
        };
        let housing = Block::new(CELL).translated(Vec3::Y * CELL.y / 2.0);
        let slot = Block::new(SLOT).translated(Vec3::Y * (CELL.y + SLOT.y / 2.0 - 0.0006));
        let slider = Block::new(SLIDER).translated(Vec3::Y * SLIDER.y / 2.0);
        let on = switch.on;
        commands.entity(add.entity).with_children(|parent| {
            parent.spawn((Shape::cuboid(&housing, HOUSING), Finish::Plastic));
            parent.spawn((Shape::cuboid(&slot, SLOT_COLOR), Finish::Plastic));
            parent
                .spawn((
                    Slider,
                    Transform::from_translation(Self::spot(on)),
                    Visibility::default(),
                ))
                .with_child((Shape::cuboid(&slider, SLIDER_COLOR), Finish::Plastic));
        });
    }

    /// Where the slider stands, on or off.
    fn spot(on: bool) -> Vec3 {
        let z = if on { -TRAVEL } else { TRAVEL };
        Vec3::new(0.0, CELL.y, z)
    }

    fn flip(
        mut press: On<PointerPress>,
        focused: Res<Focused>,
        parents: Query<&ChildOf>,
        mut switches: Query<&mut DipSwitch>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary || !focused.holds(press.entity, &parents) {
            return;
        }
        let Ok(mut switch) = switches.get_mut(press.entity) else {
            return;
        };
        press.propagate(false);
        switch.on = !switch.on;
        commands.trigger(Play(if switch.on {
            Sound::Latch
        } else {
            Sound::Unlatch
        }));
    }

    fn show(
        time: Res<Time>,
        switches: Query<(&DipSwitch, &Children)>,
        mut sliders: Query<&mut Transform, With<Slider>>,
    ) {
        let step = 2.0 * TRAVEL * time.delta_secs() / SLIDE_TIME;
        for (switch, children) in &switches {
            for &child in children {
                if let Ok(mut transform) = sliders.get_mut(child) {
                    let goal = Self::spot(switch.on);
                    let way = goal - transform.translation;
                    if way.length() > 0.0 {
                        transform.translation += way.clamp_length_max(step);
                    }
                }
            }
        }
    }
}
