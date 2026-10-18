import bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material;
@if(PREPASS_PIPELINE)
import bevy_pbr::{
    deferred::functions::deferred_output,
    prepass::io::{VertexOutput, FragmentOutput},
};
@else
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::globals,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};
@if(PREPASS_PIPELINE)
import bevy_render::globals::Globals;
import package::geometry::{
    bump::bumped,
    noise::{hash, hash1, noise},
};

struct Wet {
    /// From 0 for dry to 1 for soaked.
    wetness: f32,
    /// How much of the surface is under puddles, from 0 to 1.
    puddles: f32,
    /// Strength of rain drop ripples.
    ripples: f32,
    seed: f32,
    /// Z of the curbs, where water collects.
    curbs: vec2<f32>,
}

// The prepass binds the time elsewhere than the main pass.
@if(PREPASS_PIPELINE)
@group(0) @binding(1) var<uniform> globals: Globals;

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> wet: Wet;

/// Rings from rain drops in a grid of cells of `scale` meters. Returns the
/// slope of the water along x and z.
fn ripples(p: vec2<f32>, t: f32, scale: f32, salt: f32) -> vec2<f32> {
    let g = p / scale;
    let c = floor(g);
    var slope = vec2<f32>(0.0);
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let cell = c + vec2<f32>(f32(i), f32(j));
            let h = hash(cell, salt);
            let age = fract(t * mix(0.6, 1.2, h.w) + h.z);
            let d = g - (cell + h.xy);
            let distance = length(d);
            let x = distance - age * 0.9;
            let fade = (1.0 - age) * (1.0 - age);
            let envelope = exp(-x * x * 120.0) * fade;
            let wave = sin(x * 50.0);
            let rate = cos(x * 50.0) * 50.0 * envelope - wave * 240.0 * x * envelope;
            slope += d / max(distance, 1e-4) * rate;
        }
    }
    return slope / scale;
}

/// Bumps of the ground in meters: stones in the asphalt and cracks.
fn grain(p: vec2<f32>) -> f32 {
    let stone = hash1(floor(p * 90.0), wet.seed + 30.0);
    let cracks = smoothstep(0.012, 0.0, abs(noise(p * 0.7, wet.seed + 31.0) - 0.5)) * -0.003;
    return 0.0008 * stone + 0.0015 * noise(p * 6.0, wet.seed + 32.0) + cracks;
}

/// Lighter and darker stones in the asphalt, and patches of repairs.
fn stones(p: vec2<f32>) -> f32 {
    let stone = hash1(floor(p * 90.0), wet.seed + 30.0);
    let repair = smoothstep(0.62, 0.66, noise(p * 0.15, wet.seed + 33.0));
    return mix(0.85, 1.15, stone) * mix(1.0, 0.8, repair);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let p = in.world_position.xz;
    // Derivatives need uniform control flow, so take them up front.
    let rough_ground = bumped(normalize(in.world_normal), in.world_position.xyz, grain(p));

    if in.world_normal.y > 0.5 {
        let s = wet.seed;
        let pools = 0.5 * noise(p * 0.15, s) + 0.3 * noise(p * 0.4, s + 1.0) + 0.2 * noise(p * 1.2, s + 2.0);
        // Water gathers along the curbs.
        let curb = min(abs(in.world_position.z - wet.curbs.x), abs(in.world_position.z - wet.curbs.y));
        let gutter = 0.25 * (1.0 - smoothstep(0.0, 1.5, curb));
        let level = mix(1.0, 0.5, wet.puddles);
        let puddle = smoothstep(level - 0.02, level + 0.02, pools + gutter);
        let damp = wet.wetness * mix(0.6, 1.0, noise(p * 0.05, s + 3.0));

        var color = pbr_input.material.base_color.rgb * stones(p);
        color *= mix(1.0, 0.55, damp) * mix(1.0, 0.5, puddle);
        var rough = pbr_input.material.perceptual_roughness;
        rough = mix(rough, 0.3, damp);
        rough = mix(rough, 0.02, puddle);
        pbr_input.material.base_color = vec4<f32>(color, 1.0);
        pbr_input.material.perceptual_roughness = rough;

        // Drops make rings in the puddles and only tiny splashes elsewhere.
        let t = globals.time;
        let rings = ripples(p, t, 0.8, s + 9.0) + ripples(p, t * 1.3, 1.7, s + 13.0);
        let splashes = ripples(p, t * 3.0, 0.2, s + 17.0);
        let slope = rings * puddle + splashes * (1.0 - puddle) * damp * 0.15;
        let strength = wet.ripples * 0.03;
        let water = normalize(vec3<f32>(-slope.x * strength, 1.0, -slope.y * strength));
        // Water fills the grain of the ground.
        let n = normalize(mix(rough_ground, water, max(puddle, damp * 0.5)));
        pbr_input.N = n;
        pbr_input.clearcoat_N = n;
    }

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
