import bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material;
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};
import ld60_geometry::noise::{PI, TAU, hash1, noise};

/// Matches `POINTS` in `cable.rs`.
const POINTS: u32 = 48u;
/// Yarns winding each way around the cable.
const CARRIERS: f32 = 12.0;
/// How far a yarn winds around for each step along, the tangent of the
/// braid angle.
const PITCH: f32 = 1.2;
/// Filaments laid side by side in each yarn.
const FILAMENTS: f32 = 5.0;
/// How strongly the yarns bulge across and dive under at their ends.
const BULGE: f32 = 0.35;
const DIVE: f32 = 0.5;
/// Light left deep between the yarns.
const CREVICE: f32 = 0.45;
/// Shade of the braid where it is too fine to see.
const AVERAGE: f32 = 0.8;
const ANISOTROPY: f32 = 0.75;

struct Rope {
    points: array<vec4<f32>, POINTS>,
    normals: array<vec4<f32>, POINTS>,
    radius: f32,
    spacing: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> rope: Rope;

/// A sleeve of yarns braided two over, two under, wound in both ways
/// around the cable.
@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let normal = pbr_input.world_normal;
    let axis = normalize(in.world_tangent.xyz - normal * dot(in.world_tangent.xyz, normal));
    let around = cross(axis, normal);

    // Turns along the cable from the plug, so the braid stays on the plug
    // while the cable reels in and out at the socket.
    let turns = (f32(POINTS - 1u) - in.uv.x) * rope.spacing / (TAU * rope.radius);
    let s1 = CARRIERS * (in.uv.y - turns * PITCH);
    let s2 = CARRIERS * (in.uv.y + turns * PITCH);
    let cell = vec2<i32>(floor(vec2<f32>(s1, s2)));
    let f = fract(vec2<f32>(s1, s2));
    let k = ((cell.x + cell.y) % 4 + 4) % 4;

    var across: f32;
    var span: f32;
    var yarn: f32;
    var sideways: vec3<f32>;
    var lengthways: vec3<f32>;
    if k < 2 {
        across = f.x;
        span = (f32(k) + f.y) / 2.0;
        yarn = f32(cell.x);
        sideways = normalize(around + axis * PITCH);
        lengthways = normalize(around * PITCH - axis);
    } else {
        across = f.y;
        span = (f32(k - 2) + f.x) / 2.0;
        yarn = f32(cell.y) + 100.0;
        sideways = normalize(around - axis * PITCH);
        lengthways = normalize(around * PITCH + axis);
    }

    let detail = 1.0 - smoothstep(0.3, 0.8, max(fwidth(s1), fwidth(s2)));
    let c = across * 2.0 - 1.0;
    let crown = sqrt(max(1.0 - c * c, 0.0));
    let ends = sin(PI * span);
    let tilt = c / sqrt(max(1.0 - c * c, 0.05));
    let bent = normalize(normal + (sideways * tilt * BULGE - lengthways * cos(PI * span) * DIVE) * detail);

    let filaments = 0.85 + 0.15 * cos(c * FILAMENTS * PI);
    let fuzz = 0.9 + 0.2 * noise(vec2<f32>(span * 40.0, yarn), 3.0);
    let shade = mix(CREVICE, 1.0, crown * (0.4 + 0.6 * ends)) * filaments * fuzz;
    let tint = 0.92 + 0.16 * hash1(vec2<f32>(yarn, 0.0), 7.0);
    let albedo = mix(AVERAGE, shade * tint, detail);

    pbr_input.N = bent;
    pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * albedo, 1.0);
    pbr_input.material.perceptual_roughness = mix(
        pbr_input.material.perceptual_roughness,
        0.9,
        (1.0 - crown) * detail,
    );
@if(STANDARD_MATERIAL_ANISOTROPY) {
    let along = normalize(mix(axis, lengthways, detail));
    let tangent = normalize(along - bent * dot(along, bent));
    pbr_input.anisotropy_strength = ANISOTROPY * mix(0.4, 1.0, detail);
    pbr_input.anisotropy_T = tangent;
    pbr_input.anisotropy_B = cross(bent, tangent);
}

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
