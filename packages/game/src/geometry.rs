pub(crate) mod bar;
pub(crate) mod block;
pub(crate) mod frame;
pub(crate) mod quads;
pub(crate) mod shape;
pub(crate) mod surface;

use bevy::{prelude::*, shader::load_shader_library};

/// Shapes, their surfaces, and the shader helpers other shaders share.
pub(crate) struct GeometryPlugin;

impl Plugin for GeometryPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "geometry/noise.wesl");
        load_shader_library!(app, "geometry/bump.wesl");
        load_shader_library!(app, "geometry/streak.wesl");
        app.add_plugins(shape::ShapePlugin);
    }
}
