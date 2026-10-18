/// Solid wood, cut from a log: growth rings around a pith outside the
/// board, darker latewood, pores along the fibers and a sheen that moves
/// with the view, after Liu, Marschner and Dye, "Procedural wood textures".

import package::geometry::noise::{hash, hash1, noise, visible};

/// Width of one year's growth, in meters.
const RING: f32 = 0.006;
/// Pores run along the fibers, a few millimeters long and very thin.
const PORE_ALONG: f32 = 120.0;
const PORE_ACROSS: f32 = 2200.0;
const PORE_DEPTH: f32 = 0.00025;
/// Softer earlywood wears down a little below the latewood.
const EARLYWOOD_DEPTH: f32 = 0.00006;
/// Width of the fiber highlight, in radians.
const SHEEN_WIDTH: f32 = 0.22;

struct Wood {
    /// Factor for the wood's color.
    shade: f32,
    /// Factor for its roughness.
    roughness: f32,
    /// Bumps in meters.
    height: f32,
    /// Direction of the fibers, in the same space as the point.
    fiber: vec3<f32>,
}

/// Wood at `p` in a board with half size `h`, its fibers along the longest
/// side. `pixel` is the size of a pixel on the surface, in meters, which
/// fades out detail too fine to see.
fn wood(p: vec3<f32>, h: vec3<f32>, s: f32, pixel: f32) -> Wood {
    // The board's axes: along the fibers, and across them.
    var along = vec3<f32>(1.0, 0.0, 0.0);
    var across_a = vec3<f32>(0.0, 1.0, 0.0);
    var across_b = vec3<f32>(0.0, 0.0, 1.0);
    if h.z >= h.x && h.z >= h.y {
        along = vec3<f32>(0.0, 0.0, 1.0);
        across_b = vec3<f32>(1.0, 0.0, 0.0);
    } else if h.y >= h.x {
        along = vec3<f32>(0.0, 1.0, 0.0);
        across_a = vec3<f32>(1.0, 0.0, 0.0);
    }
    let z = dot(p, along);
    let q = vec2<f32>(dot(p, across_a), dot(p, across_b));
    let half_across = vec2<f32>(dot(h, across_a), dot(h, across_b));

    // The pith lies a few centimeters below the board, where the log's
    // center was. The board was sawn a little askew to the trunk, so the
    // rings cut its face in arches.
    let k = hash(vec2<f32>(s, 7.0), 40.0);
    let below = half_across.x + mix(0.03, 0.1, k.x);
    let askew = vec2<f32>(mix(0.1, 0.2, k.w) * select(-1.0, 1.0, k.y > 0.5), mix(-0.05, 0.05, k.z));
    let sideways = mix(-0.4, 0.4, hash1(vec2<f32>(s, 8.0), 40.0)) * half_across.y;
    let pith = vec2<f32>(-below, sideways) + askew * z;
    let d = q - pith;
    let theta = atan2(d.y, d.x);
    // Rings wander as the tree grew unevenly.
    let wander = 0.02 * (noise(vec2<f32>(z * 1.5, theta * 3.0), s) - 0.5)
        + 0.0015 * (noise(vec2<f32>(z * 7.0, theta * 11.0), s + 1.0) - 0.5);
    let r = length(d) + wander;
    // Some years grew faster than others.
    let years = r / RING;
    let t = years + 0.6 * noise(vec2<f32>(years * 0.15, 0.5), s + 2.0);
    let f = fract(t);
    // Each year's latewood is as dark as that summer was dry.
    let summer = mix(0.35, 1.0, hash(vec2<f32>(floor(t), s), 41.0).x);
    let latewood = smoothstep(0.55, 0.72, f) * (1.0 - smoothstep(0.92, 1.0, f)) * summer;

    // Fine detail fades to its average where it gets smaller than a pixel.
    let ring_detail = visible(RING, pixel);
    let pore_detail = visible(1.0 / PORE_ACROSS, pixel);
    let pore_noise = noise(vec2<f32>(z * PORE_ALONG, (q.x + q.y * 0.77) * PORE_ACROSS), s + 3.0);
    // Earlywood has the larger pores.
    let pore = smoothstep(0.68, 0.85, pore_noise) * mix(1.0, 0.35, latewood) * pore_detail;
    let late = mix(0.3, latewood, ring_detail);

    var out: Wood;
    let streaks = mix(0.93, 1.06, noise(vec2<f32>(z * 0.6, r * 30.0), s + 4.0));
    out.shade = mix(1.06, 0.72, late) * (1.0 - 0.3 * pore - 0.06 * (1.0 - pore_detail)) * streaks;
    out.height = (-PORE_DEPTH * pore - EARLYWOOD_DEPTH * (1.0 - latewood)) * ring_detail;
    out.roughness = mix(1.0, 0.85, late) + 0.3 * pore;

    // Fibers undulate in and out of the surface, which moves the sheen.
    let wave = 0.3 * (noise(vec2<f32>(z * 9.0, r * 60.0), s + 5.0) - 0.5);
    out.fiber = normalize(along + (across_a + across_b) * wave);
    return out;
}

/// Light scattered off the fibers below the surface towards `view`, from
/// `light`. Highest where the two make about the same angle with `fiber`.
fn fiber_sheen(fiber: vec3<f32>, view: vec3<f32>, light: vec3<f32>) -> f32 {
    let psi = asin(clamp(dot(view, fiber), -1.0, 1.0)) + asin(clamp(dot(light, fiber), -1.0, 1.0));
    return exp(-psi * psi / (2.0 * SHEEN_WIDTH * SHEEN_WIDTH));
}
