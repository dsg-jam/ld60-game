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
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> rope: Rope;

/// Puts each ring of the tube on a Catmull-Rom spline through the points of
/// the cable. The points are in world space, so the transform of the mesh
/// does not count.
@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let i = min(u32(vertex.uv.x), POINTS - 2u);
    let t = vertex.uv.x - f32(i);
    let p0 = rope.points[max(i, 1u) - 1u].xyz;
    let p1 = rope.points[i].xyz;
    let p2 = rope.points[i + 1u].xyz;
    let p3 = rope.points[min(i + 2u, POINTS - 1u)].xyz;
    let a = 2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3;
    let b = 3.0 * (p1 - p2) + p3 - p0;
    let center = p1 + 0.5 * t * ((p2 - p0) + t * (a + t * b));
    let tangent = normalize((p2 - p0) + t * (2.0 * a + 3.0 * t * b));
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
