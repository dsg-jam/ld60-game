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
    /// Wind at the height of the clouds, in meters per second.
    wind: vec2<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> sky: Sky;

/// Light pollution from the streets, warm and low.
const CITY_GLOW: vec3<f32> = vec3<f32>(0.06, 0.025, 0.02);
const CLOUD_GLOW: vec3<f32> = vec3<f32>(0.12, 0.075, 0.07);
/// Height of the cloud layer, in meters.
const CLOUD_HEIGHT: f32 = 500.0;
/// Size of a cloud, in meters.
const CLOUD_SIZE: f32 = 200.0;
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

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = normalize(in.world_position.xyz - view.world_position);
    let up = max(d.y, 0.0);

    var color = sky.horizon * mix(1.0, sky.zenith, sqrt(up)) + CITY_GLOW * exp(-up * 8.0);
    let clear = 1.0 - 0.8 * sky.rain;
    color += stars(d) * smoothstep(0.05, 0.4, up) * clear;
    let without_planet = color;
    color = mix(planet(d, color), without_planet, 0.3 + 0.4 * sky.rain);

    // Low clouds, lit from below by the city.
    // Projected onto a flat layer overhead, so they shrink towards the
    // horizon.
    let p = (d.xz / (up + 0.08) * CLOUD_HEIGHT + sky.wind * globals.time) / CLOUD_SIZE;
    // Scattered clouds in clear weather, closing up in the rain.
    let edge = mix(0.58, 0.44, sky.rain);
    let cloud = smoothstep(edge, edge + 0.12, fbm(p, 80.0));
    let lit = CLOUD_GLOW * (0.6 + 0.6 * exp(-up * 4.0)) * mix(0.5, 1.3, fbm(p * 3.0, 90.0));
    color = mix(color, lit, cloud * 0.92);

    // Meets the fog of the distant buildings at the horizon.
    color = mix(sky.horizon, color, smoothstep(0.0, 0.12, d.y));
    return vec4<f32>(color, 1.0);
}
