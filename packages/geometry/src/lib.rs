//! Mesh shapes, their surfaces, and the shader helpers other shaders share.

pub mod bar;
pub mod block;
pub mod frame;
pub mod noise;
pub mod quads;
pub mod shape;
pub mod surface;

use bevy::{prelude::*, shader::load_shader_library};

/// Shapes, their surfaces, and the shader helpers other shaders share.
pub struct GeometryPlugin;

impl Plugin for GeometryPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "noise.wesl");
        load_shader_library!(app, "bump.wesl");
        load_shader_library!(app, "streak.wesl");
        app.add_plugins(shape::ShapePlugin);
    }
}
