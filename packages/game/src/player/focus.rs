use std::iter;

use bevy::{
    picking::{
        events::PointerPress,
        hover::HoverMap,
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
};

use super::zoom::Zoom;
#[cfg(feature = "debug_port")]
use crate::debug_port::DebugMethods;

pub(super) struct FocusPlugin;

impl Plugin for FocusPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Focus>()
            .init_resource::<Focused>()
            .add_systems(
                Update,
                (
                    Focused::leave_on_input,
                    Aside::close_unfocused.run_if(resource_changed::<Focused>),
                ),
            )
            .add_observer(FocusOn::apply)
            .add_observer(FocusTarget::make_clickable)
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
    Paper,
    Radio,
}

/// Something the player can click from the desk view.
///
/// Hovering it shows the shimmer and what a click does.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Clickable {
    /// The message that says what a click does.
    pub(crate) prompt: &'static str,
}

/// Makes an entity focusable from the seated view, and [`Clickable`] with
/// `prompt`.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct FocusTarget {
    /// The message that says what a click does.
    pub(crate) prompt: &'static str,
    pub(crate) focus: Focus,
    pub(crate) view: View,
}

/// Camera pose while focused on a [`FocusTarget`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum View {
    /// Relative to the target.
    Local(Transform),
    /// A fixed pose in the world.
    World(Transform),
}

/// A second camera pose of a [`FocusTarget`], to look aside while focused
/// on it. Going back with right click or Escape first comes back from the
/// aside.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct Aside {
    /// Relative to the target.
    pub(crate) view: Transform,
    pub(crate) open: bool,
}

impl Aside {
    pub(crate) fn new(view: Transform) -> Self {
        Self { view, open: false }
    }

    fn close_unfocused(focused: Res<Focused>, mut asides: Query<(Entity, &mut Aside)>) {
        for (entity, mut aside) in &mut asides {
            if aside.open && focused.get() != Some(entity) {
                aside.open = false;
            }
        }
    }
}

impl View {
    /// The camera pose, for a target at `target`.
    pub(crate) fn resolve(self, target: &GlobalTransform) -> Transform {
        match self {
            Self::Local(view) => target.mul_transform(view).compute_transform(),
            Self::World(view) => view,
        }
    }
}

impl FocusTarget {
    fn make_clickable(
        add: On<Add<FocusTarget>>,
        targets: Query<&FocusTarget>,
        mut commands: Commands,
    ) {
        if let Ok(target) = targets.get(add.entity) {
            commands.entity(add.entity).insert(Clickable {
                prompt: target.prompt,
            });
        }
    }

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
    pub(crate) fn apply(
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

    /// Whether `entity` is the focused one or a part of it.
    pub(crate) fn holds(&self, entity: Entity, parents: &Query<&ChildOf>) -> bool {
        self.0.is_some_and(|focused| {
            entity == focused
                || parents
                    .iter_ancestors(entity)
                    .any(|parent| parent == focused)
        })
    }

    pub(crate) fn leave(&mut self, next: &mut NextState<Focus>) {
        self.0 = None;
        next.set(Focus::Desk);
    }

    /// Right click or Escape goes back to the desk, or first from an open
    /// [`Aside`] and then from a [`Zoom`] back to the focused view.
    pub(super) fn leave_on_input(
        mouse: Res<ButtonInput<MouseButton>>,
        keys: Res<ButtonInput<KeyCode>>,
        mut focused: ResMut<Focused>,
        mut next: ResMut<NextState<Focus>>,
        mut views: Query<(Option<&mut Aside>, Option<&mut Zoom>)>,
    ) {
        let leave = mouse.just_pressed(MouseButton::Right) || keys.just_pressed(KeyCode::Escape);
        let Some(entity) = focused.0.filter(|_| leave) else {
            return;
        };
        let Ok((aside, zoom)) = views.get_mut(entity) else {
            focused.leave(&mut next);
            return;
        };
        if let Some(mut aside) = aside.filter(|aside| aside.open) {
            aside.open = false;
        } else if let Some(mut zoom) = zoom.filter(|zoom| zoom.zoomed()) {
            zoom.reset();
        } else {
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

/// Entities under the mouse, each followed by its ancestors.
pub(crate) fn under_mouse<'a>(
    hovers: &'a HoverMap,
    parents: &'a Query<&ChildOf>,
) -> impl Iterator<Item = Entity> + 'a {
    hovers
        .get(&PointerId::Mouse)
        .into_iter()
        .flat_map(|hits| hits.keys())
        .flat_map(|&hit| iter::once(hit).chain(parents.iter_ancestors(hit)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_comes_back_from_the_aside_first() {
        let mut world = World::new();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<NextState<Focus>>();
        let mut aside = Aside::new(Transform::IDENTITY);
        aside.open = true;
        let target = world.spawn(aside).id();
        world.insert_resource(Focused(Some(target)));
        let escape = |world: &mut World| {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::Escape);
            keys.clear();
            keys.press(KeyCode::Escape);
            world.run_system_cached(Focused::leave_on_input).unwrap();
        };
        escape(&mut world);
        assert!(!world.get::<Aside>(target).unwrap().open);
        assert_eq!(world.resource::<Focused>().get(), Some(target));
        escape(&mut world);
        assert_eq!(world.resource::<Focused>().get(), None);
    }

    #[test]
    fn escape_comes_back_from_the_aside_then_the_zoom() {
        let mut world = World::new();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<NextState<Focus>>();
        let mut aside = Aside::new(Transform::IDENTITY);
        aside.open = true;
        let mut zoom = Zoom::ahead(0.1, 2);
        zoom.step_by(1);
        let target = world.spawn((aside, zoom)).id();
        world.insert_resource(Focused(Some(target)));
        let right_click = |world: &mut World| {
            let mut mouse = world.resource_mut::<ButtonInput<MouseButton>>();
            mouse.release(MouseButton::Right);
            mouse.clear();
            mouse.press(MouseButton::Right);
            world.run_system_cached(Focused::leave_on_input).unwrap();
        };
        right_click(&mut world);
        assert!(!world.get::<Aside>(target).unwrap().open);
        assert!(world.get::<Zoom>(target).unwrap().zoomed());
        right_click(&mut world);
        assert!(!world.get::<Zoom>(target).unwrap().zoomed());
        assert_eq!(world.resource::<Focused>().get(), Some(target));
        right_click(&mut world);
        assert_eq!(world.resource::<Focused>().get(), None);
    }
}
