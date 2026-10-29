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
import package::walls::{cladding_color, cladding_height, stains};
import ld60_geometry::{
    bump::bumped,
    noise::{hash, noise},
};

// Face space: x runs along the wall, y up and z into the building, in meters.

struct Facade {
    origin: vec3<f32>,
    seed: f32,
    size: vec3<f32>,
    cell: vec2<f32>,
    window: vec2<f32>,
    lit: f32,
    glow: f32,
    facing: f32,
    floors: f32,
    archetype: u32,
    cladding: u32,
    /// How wet the walls are, from 0 to 1.
    wetness: f32,
    /// How weathered the walls are, from 0 to 1.
    age: f32,
    covers: array<vec4<f32>, COVERS>,
}

/// Matches `COVERS` in `buildings.rs`.
const COVERS: u32 = 4u;
/// Matches `Archetype` in `buildings.rs`.
const OFFICE: u32 = 0u;
const HOME: u32 = 1u;

// The prepass binds the time elsewhere than the main pass.
@if(PREPASS_PIPELINE)
@group(0) @binding(1) var<uniform> globals: Globals;

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> facade: Facade;

/// Window pitch and floor height of the wall being drawn, fitted to its
/// width.
var<private> cell: vec2<f32>;

/// How far the glass sits behind the wall.
const INSET: f32 = 0.18;

const GLASS: vec3<f32> = vec3<f32>(0.01, 0.012, 0.018);
const TUNGSTEN: vec3<f32> = vec3<f32>(1.0, 0.45, 0.15);
const NEUTRAL: vec3<f32> = vec3<f32>(1.0, 0.85, 0.65);
const DAYLIGHT: vec3<f32> = vec3<f32>(0.9, 0.95, 1.0);
const TUBE: vec3<f32> = vec3<f32>(0.8, 1.0, 0.85);
const TV_BLUE: vec3<f32> = vec3<f32>(0.35, 0.55, 1.0);
const CYAN: vec3<f32> = vec3<f32>(0.1, 0.8, 1.0);
const MAGENTA: vec3<f32> = vec3<f32>(1.0, 0.15, 0.6);
/// Light from the street falling into dark rooms.
const STREET: vec3<f32> = vec3<f32>(0.6, 0.7, 1.0);
/// Light bounced up the walls from the lit street, and from the hazy sky.
const UPLIGHT: vec3<f32> = vec3<f32>(0.18, 0.12, 0.15);
const UPLIGHT_HEIGHT: f32 = 12.0;
/// Rain leaves walls damp rather than glossy.
const WET_ROUGHNESS: f32 = 0.6;
const SKYLIGHT: vec3<f32> = vec3<f32>(0.03, 0.028, 0.045);
const HALLWAY: f32 = 0.04;
/// Seconds a bulb takes to light up and to go dark.
const WARM_UP: f32 = 0.25;
const COOL_DOWN: f32 = 0.6;

fn inside(p: vec2<f32>, lo: vec2<f32>, hi: vec2<f32>) -> bool {
    return all(p >= lo) && all(p <= hi);
}

struct Hit {
    t: f32,
    point: vec3<f32>,
    /// Points back towards the ray.
    normal: vec3<f32>,
}

fn safe_inverse(d: vec3<f32>) -> vec3<f32> {
    return 1.0 / select(vec3<f32>(1e-5), d, abs(d) > vec3<f32>(1e-5));
}

/// Where a ray starting inside the box from 0 to `size` leaves it.
fn room_wall(origin: vec3<f32>, d: vec3<f32>, size: vec3<f32>) -> Hit {
    let far = select(vec3<f32>(0.0), size, d > vec3<f32>(0.0));
    let t3 = (far - origin) * safe_inverse(d);
    var hit: Hit;
    hit.t = min(t3.x, min(t3.y, t3.z));
    hit.point = origin + d * hit.t;
    if hit.t == t3.x {
        hit.normal = vec3<f32>(-sign(d.x), 0.0, 0.0);
    } else if hit.t == t3.y {
        hit.normal = vec3<f32>(0.0, -sign(d.y), 0.0);
    } else {
        hit.normal = vec3<f32>(0.0, 0.0, -1.0);
    }
    return hit;
}

/// Where a ray hits a solid box from `lo` to `hi`. `t` is negative on a miss.
fn solid_box(origin: vec3<f32>, d: vec3<f32>, lo: vec3<f32>, hi: vec3<f32>) -> Hit {
    let inv = safe_inverse(d);
    let a = (lo - origin) * inv;
    let b = (hi - origin) * inv;
    let near = min(a, b);
    let far = max(a, b);
    let t0 = max(near.x, max(near.y, near.z));
    let t1 = min(far.x, min(far.y, far.z));
    var hit: Hit;
    hit.t = select(-1.0, t0, t0 < t1 && t0 > 0.0);
    hit.point = origin + d * t0;
    if t0 == near.x {
        hit.normal = vec3<f32>(-sign(d.x), 0.0, 0.0);
    } else if t0 == near.y {
        hit.normal = vec3<f32>(0.0, -sign(d.y), 0.0);
    } else {
        hit.normal = vec3<f32>(0.0, 0.0, -sign(d.z));
    }
    return hit;
}

/// Muted wall paint for room interiors.
fn paint(x: f32) -> vec3<f32> {
    switch u32(fract(x) * 6.0) {
        case 0u: { return vec3<f32>(0.75, 0.7, 0.6); }
        case 1u: { return vec3<f32>(0.55, 0.6, 0.62); }
        case 2u: { return vec3<f32>(0.7, 0.55, 0.45); }
        case 3u: { return vec3<f32>(0.6, 0.65, 0.5); }
        case 4u: { return vec3<f32>(0.8, 0.78, 0.72); }
        default: { return vec3<f32>(0.5, 0.45, 0.5); }
    }
}

/// One of four strengths, picked anew `rate` times a second with a short
/// blend in between.
fn stepped(id: vec2<f32>, t: f32, rate: f32, salt: f32, levels: vec4<f32>) -> f32 {
    let x = t * rate;
    let k = floor(x);
    let now = levels[u32(hash(id + vec2<f32>(k, 0.0), salt).x * 3.999)];
    let before = levels[u32(hash(id + vec2<f32>(k - 1.0, 0.0), salt).x * 3.999)];
    return mix(before, now, smoothstep(0.0, 0.15, fract(x)));
}

/// Color and strength of the light in the room behind a window.
fn window_light(id: vec2<f32>, h: vec4<f32>, h2: vec4<f32>, face: f32) -> vec4<f32> {
    let t = globals.time;
    var on = false;
    var color = NEUTRAL;
    // Mostly dim rooms, a few bright ones that bloom.
    var strength = mix(0.05, 1.0, h.z * h.z * h.z);
    if h.w > 0.94 {
        strength = 1.0 + 2.0 * fract(h.w * 17.0);
    }
    if facade.archetype == OFFICE {
        // Offices light whole floors.
        let floor_lit = hash(vec2<f32>(0.0, id.y), facade.seed + face + 7.0).x < facade.lit * 1.4;
        on = select(h.x < 0.05, h.x < 0.85, floor_lit);
        color = select(mix(NEUTRAL, DAYLIGHT, h.y), TUBE, h.y > 0.7);
        strength = mix(0.4, 0.8, h.z);
        // A failing tube stutters between strengths in bursts.
        if h.w < 0.03 {
            let burst = smoothstep(0.75, 0.85, noise(vec2<f32>(t * 0.3 + h.z * 40.0, 3.0), facade.seed));
            let level = stepped(id, t, 14.0, facade.seed + 3.0, vec4<f32>(0.05, 0.5, 1.0, 0.5));
            strength *= mix(1.0, level, burst);
        }
    } else if facade.archetype == HOME {
        on = h.x < facade.lit;
        let k = h.y * h.y;
        color = mix(mix(TUNGSTEN, NEUTRAL, smoothstep(0.0, 0.7, k)), DAYLIGHT, smoothstep(0.85, 1.0, k));
        if h2.x > 0.97 {
            color = select(CYAN, MAGENTA, h2.y > 0.5);
        }
        // A TV jumps between strengths at each scene cut.
        if h.w < 0.08 {
            let rate = mix(0.7, 2.0, h2.y);
            let level = stepped(id, t, rate, facade.seed + 5.0, vec4<f32>(0.2, 0.45, 0.7, 1.0));
            let shimmer = 0.92 + 0.08 * noise(vec2<f32>(t * 20.0, h.z * 50.0), facade.seed);
            color = mix(TV_BLUE, DAYLIGHT, h2.z * 0.5);
            strength = 0.5 * level * shimmer;
        }
    } else {
        on = h.x < 0.04;
        color = mix(TUNGSTEN, NEUTRAL, h.y);
    }
    var level = select(0.0, 1.0, on);
    // Some rooms switch on and off every few minutes.
    if h2.x < 0.15 {
        let period = mix(240.0, 900.0, h2.y);
        let since = fract(t / period + h2.z) * period;
        let lit_for = facade.lit * period;
        level = smoothstep(0.0, WARM_UP, since) * (1.0 - smoothstep(lit_for, lit_for + COOL_DOWN, since));
    }
    return vec4<f32>(color, strength * level);
}

/// What the camera sees of a room behind a window, by following the view ray
/// into a box, after van Dongen, "Interior Mapping". A lamp at the ceiling
/// lights it with `light`, and the street outside with `spill` through the
/// window.
fn room(origin: vec3<f32>, d: vec3<f32>, h: vec4<f32>, h2: vec4<f32>, light: vec4<f32>, spill: vec3<f32>) -> vec3<f32> {
    let depth = mix(3.0, 6.0, h2.x);
    var hit = room_wall(origin, d, vec3<f32>(cell, depth));
    let wall = paint(h2.y);
    var albedo = wall;
    if hit.normal.y < -0.5 {
        albedo = vec3<f32>(0.9);
    } else if hit.normal.y > 0.5 {
        albedo = wall * 0.45;
    } else if abs(hit.normal.x) > 0.5 {
        albedo = wall * 0.85;
    }
    // One piece of furniture standing on the floor.
    let lo = vec3<f32>(cell.x * mix(0.05, 0.45, h2.z), 0.0, depth * mix(0.3, 0.6, h2.w));
    let hi = lo + vec3<f32>(cell.x * mix(0.25, 0.45, h2.w), mix(0.4, 1.1, h2.z), depth * 0.3);
    let thing = solid_box(origin, d, lo, hi);
    if thing.t > 0.0 && thing.t < hit.t {
        hit = thing;
        albedo = paint(h2.z + 0.37) * 0.5;
    }
    let lamp = vec3<f32>(cell.x * mix(0.3, 0.7, h.z), cell.y - 0.15, depth * 0.4);
    let to_lamp = lamp - hit.point;
    let lambert = max(dot(hit.normal, normalize(to_lamp)), 0.0);
    let lamp_light = (0.1 + lambert * 3.0 / (1.0 + 1.5 * dot(to_lamp, to_lamp))) * light.rgb * light.w;
    // The street lights what faces the window, less so further in.
    let facing_window = 0.5 + 0.5 * max(-hit.normal.z, 0.0);
    let street = spill * facing_window * (0.3 + 0.7 * exp(-hit.point.z * 0.5));
    // Some dark rooms get a little light through a door from the rest of
    // the flat.
    let door = vec3<f32>(select(0.0, cell.x, h2.w > 0.8), 1.0, depth * 0.8);
    let to_door = door - hit.point;
    let through_door = select(0.0, HALLWAY, h2.w > 0.6) * max(dot(hit.normal, normalize(to_door)), 0.0)
        / (1.0 + dot(to_door, to_door));
    return albedo * (lamp_light + street + TUNGSTEN * through_door * (1.0 - min(light.w, 1.0)));
}

/// Curtains or blinds in front of the room. `f` is the position in the
/// opening from 0 to 1. Returns how much they cover and how bright they are.
fn drapes(f: vec2<f32>, h2: vec4<f32>) -> vec2<f32> {
    if facade.archetype == OFFICE {
        let drawn = h2.w * h2.w;
        let slat = 0.55 + 0.45 * step(0.5, fract(f.y * 30.0));
        return vec2<f32>(step(1.0 - drawn, f.y), slat * 0.5);
    }
    let width = select(mix(0.1, 0.4, h2.w), 0.5, h2.w > 0.9);
    let edge = min(f.x, 1.0 - f.x);
    let fold = 0.75 + 0.25 * sin(f.x * 40.0 + h2.z * 6.0);
    return vec2<f32>(1.0 - smoothstep(width, width + 0.03, edge), fold * 0.4);
}

struct Surface {
    /// Emitted light.
    light: vec3<f32>,
    /// How much of the wall is glass.
    glass: f32,
    /// Darkening of the wall, for window reveals.
    shade: f32,
}

/// Whether the window `id` sits behind a sign. Only the shop side has
/// signs.
fn covered(id: vec2<f32>, front: bool) -> bool {
    if !front {
        return false;
    }
    let center = (id + 0.5) * cell;
    let margin = cell * 0.5;
    for (var i = 0u; i < COVERS; i++) {
        let cover = facade.covers[i];
        if cover.z > cover.x && all(center > cover.xy - margin) && all(center < cover.zw + margin) {
            return true;
        }
    }
    return false;
}

/// A window of the upper floors, with the glass set back and a room behind.
fn window(upper: vec2<f32>, d: vec3<f32>, face: f32, front: bool, blend: f32) -> Surface {
    var out: Surface;
    out.shade = 1.0;
    let id = floor(upper / cell);
    // The parapet above the top floor has no windows.
    if covered(id, front) || id.y >= facade.floors {
        return out;
    }
    // Far away the windows are smaller than a pixel. Use their average.
    let average = facade.lit * 0.25 * facade.window.x * facade.window.y * NEUTRAL;
    if blend > 0.999 {
        out.light = average;
        return out;
    }
    let q = upper - id * cell;
    let lo = cell * (0.5 - facade.window * 0.5);
    let hi = cell * (0.5 + facade.window * 0.5);
    if !inside(q, lo, hi) {
        out.light = average * blend;
        return out;
    }
    let h = hash(id, facade.seed + face);
    let h2 = hash(id, facade.seed + face + 11.0);
    let light = window_light(id, h, h2, face);
    let qg = q + d.xy / d.z * INSET;
    var near = vec3<f32>(0.0);
    if inside(qg, lo, hi) {
        let f = (qg - lo) / (hi - lo);
        let drape = drapes(f, h2);
        let spill = STREET * (0.008 + 0.012 * h.z);
        let inside = room(vec3<f32>(qg, INSET), d, h, h2, light, spill);
        let seen = mix(inside, drape.y * (light.rgb * light.w + spill), drape.x);
        // The frame splits the window into two panes.
        let frame = step(abs(f.x - 0.5), 0.015);
        near = seen * (1.0 - frame);
        out.glass = 1.0 - frame;
    } else {
        // The ray hits the side of the opening before the glass.
        near = light.rgb * light.w * 0.03;
        out.shade = select(0.35, 0.8, qg.y < lo.y);
    }
    out.light = mix(near, average, blend);
    return out;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let n = normalize(in.world_normal);
    let p = in.world_position.xyz - facade.origin;
    let along = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), n));
    let width = select(facade.size.x, facade.size.z, abs(n.x) > abs(n.z));
    // Measured from the wall's left end, seen from outside.
    let u = dot(p, along) + select(0.0, width, along.x + along.z < 0.0);
    cell = vec2<f32>(width / max(round(width / facade.cell.x), 1.0), facade.cell.y);
    let upper = vec2<f32>(u, p.y);
    // Derivatives need uniform control flow, so take them up front.
    let window_pixel = fwidth(upper / cell);
    let window_blend = smoothstep(0.08, 0.3, max(window_pixel.x, window_pixel.y));
    let bumps = cladding_height(facade.cladding, upper, cell, facade.seed);
    let rough_wall = bumped(n, in.world_position.xyz, bumps);

    if abs(n.y) < 0.5 {
        let face = select(0.0, 1.0, abs(n.x) > abs(n.z)) + select(0.0, 2.0, n.x + n.z < 0.0);
        let front = n.z * facade.facing > 0.5;
        let view = -pbr_input.V;
        let d = vec3<f32>(dot(view, along), view.y, -dot(view, n));
        var surface = window(upper, d, face, front, window_blend);
        surface.light *= facade.glow;
        let base = pbr_input.material.base_color.rgb;
        var wall = cladding_color(facade.cladding, base, upper, cell, facade.seed);
        let marks = stains(upper, width, facade.size.y, in.world_position.y, cell, facade.window, facade.age, facade.seed + face * 13.0);
        wall = mix(wall * marks.darken, marks.tint, marks.amount);
        // Ledges along each floor catch the light.
        let ledge = 1.0 - smoothstep(0.0, 0.18, fract(upper.y / cell.y) * cell.y);
        wall *= 1.0 + 0.3 * ledge * step(cell.y, upper.y);
        // Rain darkens the wall and makes it shine, most where it runs down.
        let wet = facade.wetness * clamp(0.2 + 0.8 * marks.runoff, 0.0, 1.0);
        wall *= (1.0 - 0.2 * wet) * surface.shade;
        pbr_input.material.base_color = vec4<f32>(mix(wall, GLASS, surface.glass), 1.0);
        pbr_input.material.perceptual_roughness = mix(mix(0.95, WET_ROUGHNESS, wet), 0.15, surface.glass);
        // Walls reflect little, glass the usual amount.
        pbr_input.material.reflectance = vec3<f32>(mix(0.25, 0.5, surface.glass));
        pbr_input.N = normalize(mix(rough_wall, n, surface.glass));
        let bounce = (UPLIGHT * exp(-in.world_position.y / UPLIGHT_HEIGHT) + SKYLIGHT) * wall * (1.0 - surface.glass);
        // Like `StandardMaterial`, emitted light ignores the camera exposure.
        pbr_input.material.emissive = vec4<f32>(surface.light + bounce, 0.0);
    } else {
        pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * 0.6, 1.0);
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
