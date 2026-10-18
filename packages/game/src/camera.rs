use bevy::{
    core_pipeline::tonemapping::DebandDither,
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
};

#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;
use crate::look::LookAngles;

/// How quickly the camera moves to a new view.
const DECAY_RATE: f32 = 8.0;
/// Faster rate for mouse look, so it feels direct.
const LOOK_DECAY_RATE: f32 = 25.0;
/// Distance from the seat below which the camera counts as seated.
const SEATED_DISTANCE: f32 = 0.01;

pub(crate) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Focus>()
            .init_resource::<Focused>()
            .add_systems(
                Update,
                (Focused::leave_on_right_click, PlayerCamera::follow).chain(),
            )
            .add_observer(FocusOn::apply)
            .add_observer(FocusTarget::focus_on_press);
        #[cfg(feature = "debug_port")]
        app.add_debug_method("ld60.focus", Focused::debug_focus);
    }
}

/// What the player is looking at.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum Focus {
    #[default]
    Desk,
    Computer,
    Device,
}

/// The camera of the seated player.
#[derive(Component, Debug)]
#[require(Camera3d, IsDefaultUiCamera, DebandDither::Disabled)]
pub(crate) struct PlayerCamera {
    /// Pose while sitting at the desk and looking straight ahead.
    seat: Transform,
}

impl PlayerCamera {
    pub(crate) fn seated_at(seat: Transform) -> impl Bundle {
        (Self { seat }, seat)
    }

    pub(crate) fn seat(&self) -> Transform {
        self.seat
    }

    fn follow(
        time: Res<Time>,
        focused: Res<Focused>,
        angles: Res<LookAngles>,
        targets: Query<(&FocusTarget, &GlobalTransform)>,
        camera: Single<(&PlayerCamera, &mut Transform)>,
    ) {
        let (camera, mut transform) = camera.into_inner();
        let focus_view =
            focused
                .0
                .and_then(|entity| targets.get(entity).ok())
                .map(|(target, global)| match target.view {
                    View::Local(view) => global.mul_transform(view).compute_transform(),
                    View::Seated => camera.seat,
                });
        let (goal, decay_rate) = match focus_view {
            Some(view) => (view, DECAY_RATE),
            None => {
                let goal = angles.apply(camera.seat);
                let seated = transform.translation.distance(goal.translation) < SEATED_DISTANCE;
                (goal, if seated { LOOK_DECAY_RATE } else { DECAY_RATE })
            }
        };

        let dt = time.delta_secs();
        transform
            .translation
            .smooth_nudge(&goal.translation, decay_rate, dt);
        transform
            .rotation
            .smooth_nudge(&goal.rotation, decay_rate, dt);
    }
}

/// Makes an entity clickable from the seated view.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct FocusTarget {
    pub(crate) focus: Focus,
    pub(crate) view: View,
}

/// Camera pose while focused on a [`FocusTarget`].
#[derive(Clone, Copy, Debug)]
pub(crate) enum View {
    /// Relative to the target.
    Local(Transform),
    /// Back at the seat, looking straight ahead.
    Seated,
}

impl FocusTarget {
    fn focus_on_press(
        mut press: On<PointerPress>,
        targets: Query<(), With<FocusTarget>>,
        state: Res<State<Focus>>,
        mut commands: Commands,
    ) {
        if press.button != PointerButton::Primary
            || *state.get() != Focus::Desk
            || !targets.contains(press.entity)
        {
            return;
        }
        press.propagate(false);
        commands.trigger(FocusOn {
            entity: press.entity,
        });
    }
}

/// Moves the camera to a [`FocusTarget`].
#[derive(EntityEvent, Debug)]
pub(crate) struct FocusOn {
    pub(crate) entity: Entity,
}

impl FocusOn {
    fn apply(
        focus_on: On<FocusOn>,
        targets: Query<&FocusTarget>,
        mut focused: ResMut<Focused>,
        mut next: ResMut<NextState<Focus>>,
    ) {
        if let Ok(target) = targets.get(focus_on.entity) {
            focused.0 = Some(focus_on.entity);
            next.set(target.focus);
        }
    }
}

/// The entity behind the current [`Focus`], if any.
#[derive(Resource, Default, Debug)]
pub(crate) struct Focused(Option<Entity>);

impl Focused {
    pub(crate) fn get(&self) -> Option<Entity> {
        self.0
    }

    fn leave(&mut self, next: &mut NextState<Focus>) {
        self.0 = None;
        next.set(Focus::Desk);
    }

    fn leave_on_right_click(
        mouse: Res<ButtonInput<MouseButton>>,
        mut focused: ResMut<Focused>,
        mut next: ResMut<NextState<Focus>>,
    ) {
        if mouse.just_pressed(MouseButton::Right) && focused.0.is_some() {
            focused.leave(&mut next);
        }
    }

    /// `ld60.focus { "target": "<name>" }` focuses the named entity.
    ///
    /// `"desk"` goes back to the seated view.
    #[cfg(feature = "debug_port")]
    fn debug_focus(
        In(params): In<Option<serde_json::Value>>,
        targets: Query<(Entity, &Name), With<FocusTarget>>,
        mut focused: ResMut<Focused>,
        mut next: ResMut<NextState<Focus>>,
        mut commands: Commands,
    ) -> bevy::remote::BrpResult {
        use bevy::remote::{BrpError, error_codes};

        let target = params
            .as_ref()
            .and_then(|params| params.get("target"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| BrpError {
                code: error_codes::INVALID_PARAMS,
                message: "expected a string `target`".into(),
                data: None,
            })?;
        if target == "desk" {
            focused.leave(&mut next);
            return Ok(serde_json::Value::Null);
        }
        let (entity, _) = targets
            .iter()
            .find(|(_, name)| name.as_str() == target)
            .ok_or_else(|| BrpError {
                code: error_codes::INVALID_PARAMS,
                message: format!("no focus target named {target}"),
                data: None,
            })?;
        commands.trigger(FocusOn { entity });
        Ok(serde_json::Value::Null)
    }
}
