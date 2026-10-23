/// Thin streaks of light, like falling rain, that always face the camera.

/// Roughly the angle a pixel covers, in radians.
const PIXEL_ANGLE: f32 = 0.0012;

struct Streak {
    world: vec3<f32>,
    /// Streaks thinner than a pixel get wider and fainter instead, after
    /// Persson's phone-wire anti-aliasing. Scales their brightness.
    thinning: f32,
}

/// The corner `uv` of a streak `width` wide, from `start` to `start +
/// along`, turned to face `eye`.
fn streak(start: vec3<f32>, along: vec3<f32>, width: f32, uv: vec2<f32>, eye: vec3<f32>) -> Streak {
    let side = normalize(cross(along, eye - start));
    let wide = max(width, distance(start, eye) * PIXEL_ANGLE);
    var out: Streak;
    out.world = start + side * (uv.x - 0.5) * wide + along * uv.y;
    out.thinning = width / wide;
    return out;
}

/// Brightest in the middle of a streak, fading to its sides.
fn across(uv: vec2<f32>) -> f32 {
    return 1.0 - abs(uv.x * 2.0 - 1.0);
}
