import bevy_pbr::render::{
    forward_io::VertexOutput,
    mesh_view_bindings::{globals, view},
};
import package::geometry::noise::{hash, noise};

struct Sky {
    horizon: vec3<f32>,
    /// Brightness of the zenith relative to the horizon.
    zenith: f32,
    /// Direction of the planet's center.
    planet: vec3<f32>,
    /// Angular radius of the planet.
    planet_size: f32,
    axis: vec3<f32>,
    planet_light: f32,
    /// Towards the sun, which is below the horizon.
    sun: vec3<f32>,
    rain: f32,
    /// How far the wind has blown the clouds, in meters.
    drift: vec2<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> sky: Sky;
/// Red: billowy cloud shapes. Green: finer noise that wears their edges.
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var cloud_noise: texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var cloud_sampler: sampler;

/// Light pollution from the streets, warm and low.
const CITY_GLOW: vec3<f32> = vec3<f32>(0.06, 0.025, 0.02);
/// The city lights the clouds from below.
const CITY_LIGHT: vec3<f32> = vec3<f32>(0.13, 0.08, 0.07);
/// The planet lights them faintly from above.
const PLANET_GLOW: vec3<f32> = vec3<f32>(0.03, 0.035, 0.05);
/// Bottom and top of the cloud layer, in meters.
const CLOUD_BASE: f32 = 450.0;
const CLOUD_TOP: f32 = 1100.0;
/// The noise repeats every this many meters for the shapes, and the
/// detail. Matches `CLOUD_TILE` in `sky.rs`.
const SHAPE_SIZE: f32 = 2400.0;
const DETAIL_SIZE: f32 = 400.0;
/// Light the clouds take out of a ray, per meter at full density.
const EXTINCTION: f32 = 0.02;
/// Samples along a ray through the layer.
const CLOUD_STEPS: u32 = 32u;
/// Clouds further away than this fade into the haze.
const CLOUD_REACH: f32 = 15000.0;
const BAND_LIGHT: vec3<f32> = vec3<f32>(0.62, 0.5, 0.36);
const BAND_DARK: vec3<f32> = vec3<f32>(0.26, 0.38, 0.42);
const RING: vec3<f32> = vec3<f32>(0.55, 0.48, 0.4);
const RING_INNER: f32 = 1.35;
const RING_OUTER: f32 = 2.3;
const STAR_CELLS: f32 = 300.0;

fn fbm(p: vec2<f32>, salt: f32) -> f32 {
    return noise(p, salt) * 0.5 + noise(p * 2.1, salt + 1.0) * 0.3 + noise(p * 4.3, salt + 2.0) * 0.2;
}

fn stars(d: vec3<f32>) -> vec3<f32> {
    let p = vec2<f32>(atan2(d.z, d.x), asin(d.y)) * STAR_CELLS;
    let cell = floor(p);
    let h = hash(cell, 60.0);
    if h.x < 0.97 {
        return vec3<f32>(0.0);
    }
    let center = cell + 0.25 + h.yz * 0.5;
    let glow = 1.0 - smoothstep(0.0, 0.35, length(p - center));
    let twinkle = 0.65 + 0.35 * sin(globals.time * (1.0 + h.w * 3.0) + h.y * 40.0);
    let tint = mix(vec3<f32>(0.7, 0.8, 1.0), vec3<f32>(1.0, 0.85, 0.7), h.w);
    return tint * glow * twinkle * (h.x - 0.97) * 20.0;
}

/// The planet over `background`, with its rings.
fn planet(d: vec3<f32>, background: vec3<f32>) -> vec3<f32> {
    let c = sky.planet;
    let r = sin(sky.planet_size);
    let along = dot(d, c);
    if along <= 0.0 {
        return background;
    }
    var color = background;

    // Distance along the view ray to the sphere, if it hits.
    let miss = 1.0 - along * along;
    var hit = 1e9;
    if miss < r * r {
        hit = along - sqrt(r * r - miss);
        let n = normalize(d * hit - c);
        let lit = max(dot(n, sky.sun), 0.0);
        let latitude = dot(n, sky.axis);
        let longitude = atan2(dot(n, cross(sky.axis, c)), dot(n, -c));
        let swirl = fbm(vec2<f32>(longitude * 3.0, latitude * 14.0), 70.0);
        let bands = 0.5 + 0.5 * sin(latitude * 22.0 + swirl * 5.0);
        let surface = mix(BAND_DARK, BAND_LIGHT, bands);
        let facing = max(dot(n, -d), 0.0);
        let rim = pow(1.0 - facing, 3.0) * lit;
        color = surface * (lit * sky.planet_light + 0.004) + vec3<f32>(0.2, 0.3, 0.4) * rim * sky.planet_light;
    }

    // Rings in the plane around the axis, in front of or behind the planet.
    let facing_axis = dot(d, sky.axis);
    if abs(facing_axis) > 1e-4 {
        let t = dot(c, sky.axis) / facing_axis;
        let q = d * t - c;
        let radius = length(q) / r;
        if t > 0.0 && t < hit && radius > RING_INNER && radius < RING_OUTER {
            let x = (radius - RING_INNER) / (RING_OUTER - RING_INNER);
            let ringlets = noise(vec2<f32>(radius * 40.0, 0.5), 100.0) * 0.6 + noise(vec2<f32>(radius * 150.0, 0.5), 101.0) * 0.4;
            let density = ringlets * smoothstep(0.0, 0.05, x)
                * (1.0 - smoothstep(0.9, 1.0, x)) * (1.0 - 0.8 * smoothstep(0.55, 0.6, x) * (1.0 - smoothstep(0.6, 0.65, x)));
            // The planet's shadow falls across the rings.
            let behind = dot(q, sky.sun) < 0.0 && length(q - sky.sun * dot(q, sky.sun)) < r;
            let light = select(sky.planet_light, 0.003, behind);
            color = mix(color, RING * light, density * 0.8);
        }
    }
    return color;
}

/// Noise from 0 to 1 that changes from pixel to pixel without clumping,
/// after Jimenez, "Next Generation Post Processing in Call of Duty: Advanced
/// Warfare".
fn interleaved_gradient_noise(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

fn remap(v: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    return to_min + (v - from_min) / (from_max - from_min) * (to_max - to_min);
}

/// How thick the clouds are at `p`, after Schneider, "The Real-time
/// Volumetric Cloudscapes of Horizon Zero Dawn".
fn cloud_density(p: vec3<f32>) -> f32 {
    let height = (p.y - CLOUD_BASE) / (CLOUD_TOP - CLOUD_BASE);
    if height < 0.0 || height > 1.0 {
        return 0.0;
    }
    let moved = p - vec3<f32>(sky.drift.x, 0.0, sky.drift.y);
    let shape = textureSampleLevel(cloud_noise, cloud_sampler, moved / SHAPE_SIZE, 0.0).r;
    // Flat, rounded bottoms and tops that thin out.
    let profile = smoothstep(0.0, 0.12, height) * smoothstep(1.0, 0.35, height);
    // Scattered clouds in clear weather, closing up in the rain.
    let coverage = mix(0.3, 0.55, sky.rain);
    let base = remap(shape * profile, 1.0 - coverage, 1.0, 0.0, 1.0);
    if base <= 0.0 {
        return 0.0;
    }
    // Wisps at the edges, more so towards the top.
    let detail = textureSampleLevel(cloud_noise, cloud_sampler, moved / DETAIL_SIZE, 0.0).g;
    return max(remap(base, detail * mix(0.25, 0.45, height), 1.0, 0.0, 1.0), 0.0);
}

/// How much light reaches `p` from `toward`, through the cloud in between.
/// Light that scattered several times inside reaches deeper, which a second,
/// weaker and less absorbed term stands in for, after Wrenninge et al.,
/// "Oz: The Great and Volumetric".
fn cloud_light(p: vec3<f32>, toward: vec3<f32>) -> f32 {
    var depth = 0.0;
    for (var i = 1.0; i <= 3.0; i += 1.0) {
        depth += cloud_density(p + toward * 60.0 * i * i) * 60.0 * (2.0 * i - 1.0);
    }
    let optical = depth * EXTINCTION;
    return exp(-optical) + 0.5 * exp(-optical * 0.25);
}

/// The clouds along the ray from `origin` towards `d`: the light they send
/// back in rgb, and the share of what lies behind them that shows through
/// in alpha.
fn clouds(origin: vec3<f32>, d: vec3<f32>, pixel: vec2<f32>) -> vec4<f32> {
    if d.y < 0.01 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let start = (CLOUD_BASE - origin.y) / d.y;
    let end = min((CLOUD_TOP - origin.y) / d.y, CLOUD_REACH);
    if start >= end {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let step = (end - start) / f32(CLOUD_STEPS);
    // Starts each pixel at a different point, so the steps don't show as
    // bands.
    let jitter = interleaved_gradient_noise(pixel);
    // Light scattered forward towards the planet glows a little brighter.
    let phase = 0.6 + 0.4 * pow(max(dot(d, sky.planet), 0.0), 2.0);
    let fade = 1.0 - smoothstep(CLOUD_REACH * 0.4, CLOUD_REACH, start);
    var light = vec3<f32>(0.0);
    var through = 1.0;
    for (var i = 0u; i < CLOUD_STEPS; i++) {
        let t = start + (f32(i) + jitter) * step;
        let p = origin + d * t;
        let density = cloud_density(p) * fade;
        if density <= 0.0 {
            continue;
        }
        let absorbed = 1.0 - exp(-density * EXTINCTION * step);
        // Dark edges where little light has scattered inside yet.
        let powder = 1.0 - exp(-density * 4.0);
        let below = CITY_LIGHT * cloud_light(p, vec3<f32>(0.0, -1.0, 0.0));
        let above = PLANET_GLOW * cloud_light(p, sky.planet) * phase;
        light += through * absorbed * (below + above + sky.horizon) * mix(0.5, 1.0, powder);
        through *= 1.0 - absorbed;
        if through < 0.02 {
            break;
        }
    }
    return vec4<f32>(light, through);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = normalize(in.world_position.xyz - view.world_position);
    let up = max(d.y, 0.0);

    var color = sky.horizon * mix(1.0, sky.zenith, sqrt(up)) + CITY_GLOW * exp(-up * 8.0);
    let clear = 1.0 - 0.8 * sky.rain;
    color += stars(d) * smoothstep(0.05, 0.4, up) * clear;
    let without_planet = color;
    color = mix(planet(d, color), without_planet, 0.3 + 0.4 * sky.rain);

    let clouds = clouds(view.world_position, d, in.position.xy);
    color = color * clouds.a + clouds.rgb;

    // Meets the fog of the distant buildings at the horizon.
    color = mix(sky.horizon, color, smoothstep(0.0, 0.12, d.y));
    return vec4<f32>(color, 1.0);
}
