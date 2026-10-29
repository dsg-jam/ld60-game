import bevy_pbr::render::view_transformations::position_world_to_clip;
@if(PREPASS_PIPELINE)
import bevy_pbr::prepass::io::{Vertex, VertexOutput, decompress_vertex};
@else
import bevy_pbr::render::forward_io::{Vertex, VertexOutput, decompress_vertex};

/// Matches `POINTS` in `cable.rs`.
const POINTS: u32 = 48u;
const TAU: f32 = 6.283185307;

struct Rope {
    points: array<vec4<f32>, POINTS>,
    normals: array<vec4<f32>, POINTS>,
    radius: f32,
    spacing: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> rope: Rope;

/// The point `k` of the cable, with one more mirrored past each end, so the
/// curve starts and ends on the end points.
fn point(k: i32) -> vec3<f32> {
    let last = i32(POINTS) - 1;
    if k < 0 {
        return 2.0 * rope.points[0].xyz - rope.points[1].xyz;
    }
    if k > last {
        return 2.0 * rope.points[last].xyz - rope.points[last - 1].xyz;
    }
    return rope.points[k].xyz;
}

/// Puts each ring of the tube on a uniform cubic B-spline through the
/// points of the cable. Unlike a Catmull-Rom spline it bends smoothly at the
/// points and stays between them, so it does not dip into what the cable
/// lies on. The points are in world space, so the transform of the mesh
/// does not count.
@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let i = min(u32(vertex.uv.x), POINTS - 2u);
    let t = vertex.uv.x - f32(i);
    let p0 = point(i32(i) - 1);
    let p1 = point(i32(i));
    let p2 = point(i32(i) + 1);
    let p3 = point(i32(i) + 2);
    let s = 1.0 - t;
    let center = (s * s * s * p0 + (3.0 * t * t * t - 6.0 * t * t + 4.0) * p1
        + (-3.0 * t * t * t + 3.0 * t * t + 3.0 * t + 1.0) * p2 + t * t * t * p3) / 6.0;
    let tangent = normalize(-s * s * p0 + (3.0 * t * t - 4.0 * t) * p1
        + (-3.0 * t * t + 2.0 * t + 1.0) * p2 + t * t * p3);
    let turned = mix(rope.normals[i].xyz, rope.normals[i + 1u].xyz, t);
    let normal = normalize(turned - tangent * dot(turned, tangent));
    let angle = vertex.uv.y * TAU;
    let outward = cos(angle) * normal + sin(angle) * cross(tangent, normal);
    let world = center + rope.radius * outward;

    var out: VertexOutput;
    out.position = position_world_to_clip(world);
    out.world_position = vec4<f32>(world, 1.0);
@if(!PREPASS_PIPELINE)
    out.world_normal = outward;
@if(PREPASS_PIPELINE && NORMAL_PREPASS_OR_DEFERRED_PREPASS)
    out.world_normal = outward;
@if(!PREPASS_PIPELINE && VERTEX_TANGENTS)
    out.world_tangent = vec4<f32>(tangent, 1.0);
@if(PREPASS_PIPELINE && NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_TANGENTS)
    out.world_tangent = vec4<f32>(tangent, 1.0);
@if(VERTEX_UVS_A)
    out.uv = vertex.uv;
@if(PREPASS_PIPELINE && MOTION_VECTOR_PREPASS)
    out.previous_world_position = out.world_position;
@if(PREPASS_PIPELINE && UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
}
@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;
    return out;
}
