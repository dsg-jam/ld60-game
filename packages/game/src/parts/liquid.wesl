import bevy_pbr::render::{
    mesh_functions::get_local_from_world,
    pbr_fragment::pbr_input_from_standard_material,
};
@if(PREPASS_PIPELINE)
import bevy_pbr::{
    deferred::functions::deferred_output,
    prepass::io::{VertexOutput, FragmentOutput},
};
@else
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};
import package::geometry::bump::bumped;

const WAVES: u32 = 10u;

/// Matches `LiquidParams` in `liquid.rs`.
struct Liquid {
    radius: f32,
    /// Per mode: the heights of its two waves, its Bessel root and its
    /// number of waves around.
    waves: array<vec4<f32>, 10>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> liquid: Liquid;

/// The Bessel function of the first kind of order `m`, from its power
/// series, which is exact enough for the small arguments of the modes.
fn bessel(m: f32, x: f32) -> f32 {
    let half = x * 0.5;
    var term = 1.0;
    for (var i = 1.0; i <= m; i += 1.0) {
        term *= half / i;
    }
    var sum = term;
    for (var k = 1.0; k < 24.0; k += 1.0) {
        term *= -half * half / (k * (k + m));
        sum += term;
    }
    return sum;
}

/// Height of the surface at `p`, from the middle of the container.
fn height(p: vec2<f32>) -> f32 {
    let r = min(length(p) / liquid.radius, 1.0);
    let angle = atan2(p.y, p.x);
    var h = 0.0;
    for (var i = 0u; i < WAVES; i++) {
        let wave = liquid.waves[i];
        let around = wave.w * angle;
        h += bessel(wave.w, wave.z * r) * (wave.x * cos(around) + wave.y * sin(around));
    }
    return h;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let local = (get_local_from_world(in.instance_index) * in.world_position).xyz;
    pbr_input.N = bumped(pbr_input.N, in.world_position.xyz, height(local.xy));

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
