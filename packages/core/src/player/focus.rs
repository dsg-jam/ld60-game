use std::iter;

use bevy::{
    picking::{
        events::PointerPress,
        hover::HoverMap,
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
};

use super::lens::Lens;
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
pub enum Focus {
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
pub struct Clickable {
    /// The message that says what a click does.
    pub prompt: &'static str,
}

/// What a part prompts now, for [`Clickable::show`].
pub type Prompting = (Option<&'static Clickable>, Has<Disabled>);

impl Clickable {
    /// Makes `entity` prompt `wanted`, with whether the action can be done
    /// now, or not at all. `(clickable, disabled)` is what it prompts now.
    pub fn show(
        commands: &mut Commands,
        entity: Entity,
        (clickable, disabled): (Option<&Self>, bool),
        wanted: Option<(&'static str, bool)>,
    ) {
        if clickable.map(|clickable| (clickable.prompt, !disabled)) == wanted {
            return;
        }
        let mut entity = commands.entity(entity);
        match wanted {
            Some((prompt, true)) => {
                entity.insert(Self { prompt }).remove::<Disabled>();
            }
            Some((prompt, false)) => {
                entity.insert((Self { prompt }, Disabled));
            }
            None => {
                entity.remove::<(Self, Disabled)>();
            }
        }
    }
}

/// A [`Clickable`] whose action can't be done right now. Its prompt shows
/// dimmed, without the mouse button, and it does not shimmer.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Disabled;

/// A tool drawn next to the prompt of a [`Clickable`], for a part that is
/// worked with it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClickTool {
    Screwdriver,
    Plug,
}

/// Makes an entity focusable from the seated view, and [`Clickable`] with
/// `prompt`.
#[derive(Component, Clone, Copy, Debug)]
pub struct FocusTarget {
    /// The message that says what a click does.
    pub prompt: &'static str,
    pub focus: Focus,
    pub view: View,
}

/// Camera pose while focused on a [`FocusTarget`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum View {
    /// Relative to the target.
    Local(Transform),
    /// A fixed pose in the world.
    World(Transform),
}

/// A second camera pose of a [`FocusTarget`], to look aside while focused
/// on it. Going back with right click or Escape first comes back from the
/// aside.
#[derive(Component, Clone, Copy, Debug)]
pub struct Aside {
    /// Relative to the target.
    pub view: Transform,
    pub open: bool,
}

impl Aside {
    pub fn new(view: Transform) -> Self {
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
    pub fn resolve(self, target: &GlobalTransform) -> Transform {
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
pub struct FocusOn {
    pub entity: Entity,
}

impl FocusOn {
    pub fn apply(
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
pub struct Focused(Option<Entity>);

impl Focused {
    pub fn get(&self) -> Option<Entity> {
        self.0
    }

    /// Whether `entity` is the focused one or a part of it.
    pub fn holds(&self, entity: Entity, parents: &Query<&ChildOf>) -> bool {
        self.0.is_some_and(|focused| {
            entity == focused
                || parents
                    .iter_ancestors(entity)
                    .any(|parent| parent == focused)
        })
    }

    pub fn leave(&mut self, next: &mut NextState<Focus>) {
        self.0 = None;
        next.set(Focus::Desk);
    }

    /// Right click or Escape goes back to the desk, or first from an open
    /// [`Aside`] and then from a [`Zoom`] or the [`Lens`] back to the focused
    /// view.
    pub(super) fn leave_on_input(
        mouse: Res<ButtonInput<MouseButton>>,
        keys: Res<ButtonInput<KeyCode>>,
        mut focused: ResMut<Focused>,
        mut next: ResMut<NextState<Focus>>,
        mut views: Query<(Option<&mut Aside>, Option<&mut Zoom>)>,
        mut lens: ResMut<Lens>,
    ) {
        let leave = mouse.just_pressed(MouseButton::Right) || keys.just_pressed(KeyCode::Escape);
        let Some(entity) = focused.0.filter(|_| leave) else {
            return;
        };
        let (aside, zoom) = views.get_mut(entity).unwrap_or_default();
        if let Some(mut aside) = aside.filter(|aside| aside.open) {
            aside.open = false;
        } else if let Some(mut zoom) = zoom.filter(|zoom| zoom.zoomed()) {
            zoom.reset();
        } else if lens.zoomed() {
            lens.reset();
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

/// The prompt of the [`Clickable`] under the mouse. While focused only the
/// parts of the focused thing count, since clicks elsewhere do nothing.
pub fn hovered_prompt(
    hovers: &HoverMap,
    parents: &Query<&ChildOf>,
    clickables: &Query<&Clickable>,
    focused: &Focused,
) -> Option<&'static str> {
    hovered_clickable(hovers, parents, clickables, focused)
        .and_then(|entity| clickables.get(entity).ok())
        .map(|clickable| clickable.prompt)
}

/// The [`Clickable`] whose prompt [`hovered_prompt`] shows.
pub fn hovered_clickable(
    hovers: &HoverMap,
    parents: &Query<&ChildOf>,
    clickables: &Query<&Clickable>,
    focused: &Focused,
) -> Option<Entity> {
    let Some(focused) = focused.get() else {
        return under_mouse(hovers, parents).find(|&entity| clickables.contains(entity));
    };
    hovers
        .get(&PointerId::Mouse)
        .into_iter()
        .flat_map(|hits| hits.keys())
        .find_map(|&hit| {
            let path: Vec<Entity> = iter::once(hit)
                .chain(parents.iter_ancestors(hit))
                .take_while(|&entity| entity != focused)
                .collect();
            let inside = path.last().is_some_and(|&last| {
                parents
                    .get(last)
                    .is_ok_and(|parent| parent.parent() == focused)
            });
            path.into_iter()
                .filter(|_| inside)
                .find(|&entity| clickables.contains(entity))
        })
}

/// Entities under the mouse, each followed by its ancestors.
pub fn under_mouse<'a>(
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
    use bevy::{ecs::system::RunSystemOnce, picking::backend::HitData};

    use super::*;

    #[test]
    fn focused_prompts_come_from_parts_of_the_focused_thing() {
        let mut world = World::new();
        let clickable = |prompt| Clickable { prompt };
        let radio = world.spawn(clickable("click-radio")).id();
        let knob = world
            .spawn((clickable("click-volume"), ChildOf(radio)))
            .id();
        let cap = world.spawn(ChildOf(knob)).id();
        let manuals = world.spawn(clickable("click-manuals")).id();
        let prompt = |world: &mut World, hit: Entity, focused: Option<Entity>| {
            let mut hits = bevy::ecs::entity::EntityHashMap::default();
            hits.insert(hit, HitData::new(Entity::PLACEHOLDER, 1.0, None, None));
            let mut hovers = HoverMap::default();
            hovers.insert(PointerId::Mouse, hits);
            world.insert_resource(hovers);
            world.insert_resource(Focused(focused));
            world
                .run_system_once(
                    |hovers: Res<HoverMap>,
                     parents: Query<&ChildOf>,
                     clickables: Query<&Clickable>,
                     focused: Res<Focused>| {
                        hovered_prompt(&hovers, &parents, &clickables, &focused)
                    },
                )
                .unwrap()
        };
        assert_eq!(prompt(&mut world, cap, None), Some("click-volume"));
        assert_eq!(prompt(&mut world, manuals, None), Some("click-manuals"));
        assert_eq!(prompt(&mut world, cap, Some(radio)), Some("click-volume"));
        assert_eq!(prompt(&mut world, radio, Some(radio)), None);
        assert_eq!(prompt(&mut world, manuals, Some(radio)), None);
    }

    #[test]
    fn escape_comes_back_from_the_aside_first() {
        let mut world = World::new();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<NextState<Focus>>();
        world.init_resource::<Lens>();
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
        world.init_resource::<Lens>();
        let mut aside = Aside::new(Transform::IDENTITY);
        aside.open = true;
        let mut zoom = Zoom::to(View::Local(Transform::IDENTITY), 2);
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

    #[test]
    fn escape_comes_back_from_the_lens_zoom_first() {
        let mut world = World::new();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<NextState<Focus>>();
        let mut lens = Lens::default();
        lens.step(1.0);
        world.insert_resource(lens);
        let target = world.spawn_empty().id();
        world.insert_resource(Focused(Some(target)));
        let escape = |world: &mut World| {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::Escape);
            keys.clear();
            keys.press(KeyCode::Escape);
            world.run_system_cached(Focused::leave_on_input).unwrap();
        };
        escape(&mut world);
        assert!(!world.resource::<Lens>().zoomed());
        assert_eq!(world.resource::<Focused>().get(), Some(target));
        escape(&mut world);
        assert_eq!(world.resource::<Focused>().get(), None);
    }
}
