/// Punched holes and the wear a binder leaves around them.

import package::geometry::noise::{TAU, hash, noise};

/// Share of the holes that tore out towards the edge.
const TEARS: f32 = 0.25;
/// Share of the holes that are stretched or creased.
const WORN: f32 = 0.6;

/// What the hole at `hole.xy` with radius `hole.z` does to the paper at
/// `p`, in meters from the top left of a sheet of `size`. `x` is 0 in the
/// hole or a tear and 1 on paper. `y` is how creased the paper is. Each
/// `s` wears the hole in its own way.
fn punched(p: vec2<f32>, size: vec2<f32>, hole: vec3<f32>, s: f32, pixel: f32) -> vec2<f32> {
    if hole.z <= 0.0 {
        return vec2<f32>(1.0, 0.0);
    }
    let r = hole.z;
    let d = p - hole.xy;
    let h = hash(vec2<f32>(s, hole.y * 1000.0), 90.0);

    // Pulling on the page wears the hole towards the nearest edge.
    var out = vec2<f32>(-1.0, 0.0);
    var reach = hole.x;
    if size.x - hole.x < reach {
        out = vec2<f32>(1.0, 0.0);
        reach = size.x - hole.x;
    }
    if hole.y < reach {
        out = vec2<f32>(0.0, -1.0);
        reach = hole.y;
    }
    if size.y - hole.y < reach {
        out = vec2<f32>(0.0, 1.0);
        reach = size.y - hole.y;
    }
    let along = dot(d, out);
    let across = dot(d, vec2<f32>(-out.y, out.x));

    let worn = step(h.x, WORN);
    let stretch = 1.0 + worn * h.y * 0.4;
    let q = vec2<f32>(max(along, 0.0) / stretch + min(along, 0.0), across);
    let ragged = r * (1.0 + 0.08 * (noise(d * 2500.0, s) - 0.5));
    var paper = smoothstep(-0.5 * pixel, 0.5 * pixel, length(q) - ragged);

    if h.x < TEARS {
        let end = mix(0.6, 1.05, h.z) * reach;
        let t = clamp(along / end, 0.0, 1.0);
        let wobble = 0.0004 * (noise(vec2<f32>(along * 2000.0, h.w * 50.0), s + 3.0) - 0.5);
        let width = mix(0.0005, 0.00004, t);
        let slit = abs(across - wobble) - width;
        let inside = step(0.0, along) * step(along, end);
        paper = min(paper, mix(1.0, smoothstep(-0.5 * pixel, 0.5 * pixel, slit), inside));
    }

    let near = 1.0 - smoothstep(r, r * 3.5, length(d));
    let angle = atan2(d.y, d.x);
    let lines = pow(abs(sin(angle * 3.0 + h.w * TAU + 3.0 * noise(d * 600.0, s + 4.0))), 6.0);
    let crease = worn * near * (0.3 + 0.7 * lines) * mix(0.4, 1.0, h.z);
    return vec2<f32>(paper, crease);
}
