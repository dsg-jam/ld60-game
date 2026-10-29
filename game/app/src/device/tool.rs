use bevy::{
    ecs::system::SystemParam, picking::hover::HoverMap, prelude::*,
    transform::helper::TransformHelper,
};
use ld60_game_core::player::focus::{self, ClickTool, Clickable, Disabled, Focused};

/// Where the tool for a part touches it, in the part's space. The tool
/// points along +Y, into the part.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct ToolSpot(pub(crate) Vec3);

/// The part the pointer is on, if a tool works it.
///
/// Its pose comes from the transforms, so a device that moved this frame
/// counts where it is now. Systems that also write a [`Transform`] hold it in
/// a [`ParamSet`].
#[derive(SystemParam)]
pub(crate) struct Aim<'w, 's> {
    hovers: Res<'w, HoverMap>,
    focused: Res<'w, Focused>,
    parents: Query<'w, 's, &'static ChildOf>,
    clickables: Query<'w, 's, &'static Clickable>,
    spots: Query<'w, 's, (&'static ToolSpot, &'static ClickTool), Without<Disabled>>,
    poses: TransformHelper<'w, 's>,
}

impl Aim<'_, '_> {
    /// Where `tool` would touch the part under the pointer, if it works
    /// that part and can now.
    pub(crate) fn at(&self, tool: ClickTool) -> Option<Transform> {
        let part =
            focus::hovered_clickable(&self.hovers, &self.parents, &self.clickables, &self.focused)?;
        let (spot, works) = self.spots.get(part).ok()?;
        let at = self.poses.compute_global_transform(part).ok()?;
        (*works == tool).then(|| at.compute_transform() * Transform::from_translation(spot.0))
    }
}
