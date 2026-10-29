use bevy::{ecs::system::SystemParam, picking::hover::HoverMap, prelude::*};
use ld60_core::player::focus::{self, ClickTool, Clickable, Disabled, Focused};

/// Where the tool for a part touches it, in the part's space. The tool
/// points along +Y, into the part.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct ToolSpot(pub(crate) Vec3);

/// The part the pointer is on, if a tool works it.
#[derive(SystemParam)]
pub(crate) struct Aim<'w, 's> {
    hovers: Res<'w, HoverMap>,
    focused: Res<'w, Focused>,
    parents: Query<'w, 's, &'static ChildOf>,
    clickables: Query<'w, 's, &'static Clickable>,
    spots: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            &'static ToolSpot,
            &'static ClickTool,
        ),
        Without<Disabled>,
    >,
}

impl Aim<'_, '_> {
    /// Where `tool` would touch the part under the pointer, if it works
    /// that part and can now.
    pub(crate) fn at(&self, tool: ClickTool) -> Option<Transform> {
        let part =
            focus::hovered_clickable(&self.hovers, &self.parents, &self.clickables, &self.focused)?;
        let (at, spot, works) = self.spots.get(part).ok()?;
        (*works == tool).then(|| at.compute_transform() * Transform::from_translation(spot.0))
    }
}
