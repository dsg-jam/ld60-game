/// Marks left by use. Each kind of surface wears in its own way: handled
/// things get scratched, walls get chipped and scuffed, floors get dented,
/// and steel rusts.
///
/// `pixel` is the size of a pixel on the surface in meters. Marks finer
/// than that fade out, so they don't sparkle as the camera moves.

import package::geometry::noise::{TAU, hash, noise, specks, visible};

/// Clusters of scratches where things were rubbed, each with its own
/// direction. Returns how scratched the point is.
fn scratches(uv: vec2<f32>, s: f32, wear: f32, pixel: f32) -> f32 {
    let size = 0.12;
    let cell = floor(uv / size);
    var amount = 0.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let c = cell + vec2<f32>(f32(i), f32(j));
            let h = hash(c, s + 20.0);
            if h.x > 0.25 * wear {
                continue;
            }
            let center = (c + h.yz) * size;
            let angle = h.w * TAU;
            let along = vec2<f32>(cos(angle), sin(angle));
            let d = uv - center;
            let falloff = 1.0 - smoothstep(0.0, size * 0.9, length(d));
            let line = noise(vec2<f32>(dot(d, along) * 4.0, dot(d, vec2<f32>(-along.y, along.x)) * 700.0), s + h.x * 50.0);
            amount = max(amount, smoothstep(0.9, 0.96, line) * falloff);
        }
    }
    return amount * visible(0.0015, pixel);
}

/// A chipped spot in plaster: the hole, and the dark rim around it.
struct Ding {
    hole: f32,
    rim: f32,
}

/// Small chips from knocks, a few millimeters across.
fn dings(uv: vec2<f32>, s: f32, wear: f32, pixel: f32) -> Ding {
    let size = 0.25;
    let cell = floor(uv / size);
    let h = hash(cell, s + 30.0);
    var out: Ding;
    if h.x > 0.3 * wear {
        return out;
    }
    let center = (cell + 0.2 + h.yz * 0.6) * size;
    let radius = mix(0.002, 0.008, h.w);
    // Ragged outlines.
    let d = length(uv - center) * (0.8 + 0.4 * noise(uv * 900.0, s + 31.0));
    let shows = visible(radius, pixel);
    out.hole = (1.0 - smoothstep(radius * 0.8, radius, d)) * shows;
    out.rim = (1.0 - smoothstep(radius, radius * 1.6, d)) * (1.0 - out.hole) * shows;
    return out;
}

/// Black marks from shoes and chairs low on a wall, `above` meters over the
/// floor.
fn scuffs(uv: vec2<f32>, above: f32, s: f32, wear: f32) -> f32 {
    let band = smoothstep(0.03, 0.08, above) * (1.0 - smoothstep(0.25, 0.45, above));
    let streak = smoothstep(0.72, 0.85, noise(vec2<f32>(uv.x * 6.0, above * 60.0), s + 32.0))
        * smoothstep(0.4, 0.7, noise(vec2<f32>(uv.x * 1.5, 0.5), s + 33.0));
    return band * streak * wear;
}

/// Dents in a floor where things were dropped, and the faint marks of
/// things dragged along the boards, which run along y.
fn floor_marks(uv: vec2<f32>, s: f32, wear: f32, pixel: f32) -> f32 {
    let dent_cell = floor(uv * 25.0);
    let h = hash(dent_cell, s + 34.0);
    let dent = step(1.0 - 0.06 * wear, h.x)
        * (1.0 - smoothstep(0.08, 0.2, length(fract(uv * 25.0) - h.yz)))
        * visible(0.004, pixel);
    let drag = smoothstep(0.86, 0.94, noise(vec2<f32>(uv.x * 400.0, uv.y * 2.0), s + 35.0))
        * smoothstep(0.5, 0.8, noise(uv * 2.0, s + 36.0))
        * visible(0.002, pixel);
    return max(dent, drag * 0.5) * wear;
}

/// Rust spots on steel, and the small pits they leave.
fn rust(uv: vec2<f32>, s: f32, wear: f32, pixel: f32) -> f32 {
    let spots = smoothstep(0.66, 0.78, noise(uv * 45.0, s + 37.0) * 0.6 + noise(uv * 160.0, s + 38.0) * 0.4);
    let pits = specks(uv, 0.003, 0.2, 0.00015, 0.0003, s + 39.0, pixel);
    return clamp(spots * 0.9 + pits, 0.0, 1.0) * wear;
}

/// Ribs along a grip, around its local y axis: 1 on a rib, 0 in the groove
/// between two.
fn grip_ridge(local: vec3<f32>, ribs: f32) -> f32 {
    let angle = atan2(local.z, local.x);
    return smoothstep(-0.3, 0.6, cos(angle * ribs));
}
