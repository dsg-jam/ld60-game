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
import ld60_geometry::{
    bump::bumped,
    noise::{hash, noise, visible},
};

const WAVES: u32 = 10u;
/// Typical size of a bubble, in meters.
const BUBBLE: f32 = 0.0014;
/// Rafts of bubbles, about this far apart.
const RAFT: f32 = 0.007;
/// The ring of foam along the wall reaches this far in, as a share of the
/// radius.
const RING: f32 = 0.16;
const FOAM_ROUGHNESS: f32 = 0.35;
const BUBBLE_ROUGHNESS: f32 = 0.04;

/// Matches `LiquidParams` in `liquid.rs`.
struct Liquid {
    /// Its alpha is how much of the surface the foam covers.
    foam: vec4<f32>,
    radius: f32,
    /// How far the liquid has turned in the container, in radians.
    turn: f32,
    /// Per mode: the heights of its two waves, its Bessel root and its
    /// number of waves around.
    waves: array<vec4<f32>, WAVES>,
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

/// Height of one mode at `p`, from the middle of the container.
fn mode(wave: vec4<f32>, p: vec2<f32>) -> f32 {
    let r = min(length(p) / liquid.radius, 1.0);
    let around = wave.w * atan2(p.y, p.x);
    return bessel(wave.w, wave.z * r) * (wave.x * cos(around) + wave.y * sin(around));
}

struct Surface {
    height: f32,
    /// How far the waves have pushed the liquid at the surface sideways.
    drift: vec2<f32>,
}

/// The waves at `p`. In a surface wave the liquid moves sideways about as
/// far as the slope times the wavelength over 2 pi.
fn surface(p: vec2<f32>) -> Surface {
    let e = liquid.radius * 0.01;
    var out: Surface;
    for (var i = 0u; i < WAVES; i++) {
        let wave = liquid.waves[i];
        let h = mode(wave, p);
        let slope = vec2<f32>(mode(wave, p + vec2<f32>(e, 0.0)) - h, mode(wave, p + vec2<f32>(0.0, e)) - h) / e;
        let k = wave.z / liquid.radius;
        out.height += h;
        out.drift -= slope / k;
    }
    return out;
}

struct Bubble {
    /// Distance from the middle of the nearest bubble, in cells.
    distance: f32,
    /// Radius of that bubble, in cells.
    radius: f32,
}

/// The nearest of the bubbles scattered one per cell around `p`.
fn bubble(p: vec2<f32>) -> Bubble {
    let cell = floor(p);
    var best: Bubble;
    var gap = 9.0;
    for (var y = -1.0; y <= 1.0; y += 1.0) {
        for (var x = -1.0; x <= 1.0; x += 1.0) {
            let c = cell + vec2<f32>(x, y);
            let h = hash(c, 41.0);
            let center = c + 0.2 + h.xy * 0.6;
            let radius = mix(0.3, 0.6, h.z * h.z);
            let d = length(p - center);
            if d - radius < gap {
                gap = d - radius;
                best = Bubble(d, radius);
            }
        }
    }
    return best;
}

/// How much foam floats at `q`. Bubbles drift up the meniscus to the wall
/// and to each other, after Vella and Mahadevan, "The Cheerios effect": a
/// broken ring along the wall, and a few rafts.
fn foam_cover(q: vec2<f32>) -> f32 {
    let cover = liquid.foam.a;
    let r = length(q) / liquid.radius;
    let reach = RING * cover * mix(0.3, 1.6, noise(q / RAFT * 0.8, 51.0));
    let ring = smoothstep(1.0 - reach, 1.0 - reach * 0.5, r);
    let raft = smoothstep(0.82 - 0.1 * cover, 0.86 - 0.1 * cover, noise(q / RAFT, 52.0));
    return max(ring, raft * cover);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let local = (get_local_from_world(in.instance_index) * in.world_position).xyz;
    let p = local.xy;
    let pixel = length(fwidth(p));
    let waves = surface(p);

    // Foam rides on the liquid as it sways and slowly turns.
    let turn = liquid.turn;
    let moved = p - waves.drift;
    let q = vec2<f32>(cos(turn) * moved.x - sin(turn) * moved.y, sin(turn) * moved.x + cos(turn) * moved.y);
    let b = bubble(q / BUBBLE);
    let detail = visible(BUBBLE, pixel);
    // Bubbles in the foam, and the film between them.
    let inside = 1.0 - smoothstep(b.radius * 0.85, b.radius, b.distance);
    let foam = foam_cover(q) * mix(0.7, 1.0, detail);
    let dome = sqrt(max(b.radius * b.radius - b.distance * b.distance, 0.0)) * BUBBLE * 0.6;

    var color = pbr_input.material.base_color.rgb;
    let film = liquid.foam.rgb * mix(0.8, 1.1, b.radius);
    color = mix(color, mix(film, film * 0.7, inside * detail), foam);
    pbr_input.material.base_color = vec4<f32>(color, pbr_input.material.base_color.a);
    let rough = mix(FOAM_ROUGHNESS, BUBBLE_ROUGHNESS, inside * detail);
    pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, rough, foam);

    let height = waves.height + foam * (0.0003 + dome * inside * detail);
    pbr_input.N = bumped(pbr_input.N, in.world_position.xyz, height);

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
