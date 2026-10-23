/// The marks each way of printing leaves in the ink of a page. Everything is
/// worked out from the canvas in its texels, so the marks stay put on the
/// page as the camera moves, and fade out where they get smaller than a
/// pixel.

import ld60_geometry::noise::{hash, hash1, noise, specks, visible};

/// Matches `Press` in `press.rs`.
const DOT_MATRIX: u32 = 0u;
const OFFSET: u32 = 1u;
const PHOTOCOPY: u32 = 2u;
const SPIRIT: u32 = 3u;
const LETTERPRESS: u32 = 4u;
const TYPEWRITER: u32 = 5u;

/// The bare page of a canvas, in linear color. Matches `PAPER` in
/// `paper.rs`.
const PAPER: vec3<f32> = vec3<f32>(0.888, 0.839, 0.716);
const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);
/// Texels one bilinear tap covers well.
const TAP_SPAN: f32 = 1.5;
const MAX_TAPS: f32 = 4.0;
/// Less black ink than this lets through stays black.
const DARKEST: f32 = 0.002;

/// Between the rows and the columns of a dot matrix printer's dots, in
/// texels: 9 pins over the height of a line, with the dots of a row
/// overlapping.
const DOT_PITCH: vec2<f32> = vec2<f32>(2.2, 3.4);
const DOT_RADIUS: f32 = 1.85;
/// How deep letterpress type presses into the paper, in meters.
const IMPRESSION: f32 = 0.00006;

struct Printed {
    /// The page with its print, in linear color.
    color: vec3<f32>,
    /// How far the type pressed into the page, in meters.
    relief: f32,
}

/// The ink a process puts down at a point.
struct Ink {
    /// How much of the point the ink covers.
    cover: f32,
    /// How much light the ink lets through where it covers the paper.
    level: f32,
    /// How hard the ink was put down, which scales its density.
    strike: f32,
}

/// The canvas averaged over the footprint `dx` by `dy`. The canvas has no
/// mipmaps, so far away and at a slant one tap would skip over the letters.
fn average(canvas: texture_2d<f32>, smp: sampler, uv: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>) -> vec3<f32> {
    let texels = vec2<f32>(textureDimensions(canvas));
    let footprint = vec2<f32>(length(dx * texels), length(dy * texels));
    let taps = clamp(ceil(footprint / TAP_SPAN), vec2<f32>(1.0), vec2<f32>(MAX_TAPS));
    var sum = vec3<f32>(0.0);
    for (var j = 0.0; j < taps.y; j += 1.0) {
        for (var i = 0.0; i < taps.x; i += 1.0) {
            let offset = (vec2<f32>(i, j) + 0.5) / taps - 0.5;
            sum += textureSampleLevel(canvas, smp, uv + dx * offset.x + dy * offset.y, 0.0).rgb;
        }
    }
    return sum / (taps.x * taps.y);
}

/// Widens the footprint `d` to at least `size` texels, for a blurred tap.
fn widened(d: vec2<f32>, texels: vec2<f32>, size: f32) -> vec2<f32> {
    return d * max(1.0, size / max(length(d * texels), 1e-4));
}

/// The black ink in a color of the canvas, as how much light it lets
/// through.
fn black(color: vec3<f32>) -> f32 {
    let through = min(color / PAPER, vec3<f32>(1.0));
    return max(max(through.r, through.g), through.b);
}

/// Colored ink in a color of the canvas, like a stamp or a colored band, as
/// how much light it lets through. Black ink is a little warm, so only
/// clear colors count.
fn tint(color: vec3<f32>) -> vec3<f32> {
    let through = min(color / PAPER, vec3<f32>(1.0));
    let tinted = through / max(max(max(through.r, through.g), through.b), 1e-4);
    let chroma = 1.0 - min(min(tinted.r, tinted.g), tinted.b);
    return mix(vec3<f32>(1.0), tinted, smoothstep(0.2, 0.45, chroma));
}

/// Hard edged ink from `cover`, with the edge at `level`. Where a texel
/// covers less than a pixel, the edge stays as soft as the canvas.
fn crisp(cover: f32, level: f32, pixel: f32) -> f32 {
    let width = clamp(pixel, 0.3, 1.0);
    return smoothstep(max(level - 0.5 * width, 0.0), min(level + 0.5 * width, 1.0), cover);
}

/// The light the ink lets through.
fn through(ink: Ink) -> f32 {
    return mix(1.0, pow(max(ink.level, DARKEST), ink.strike), ink.cover);
}

/// Each dot is struck by a pin through a cloth ribbon. Letters are made of
/// the dots on a grid wherever the canvas has ink. The ribbon dries out in
/// bands and some pins strike weaker.
fn dot_matrix(canvas: texture_2d<f32>, smp: sampler, p: vec2<f32>, texels: vec2<f32>, pixel: f32, run: f32) -> f32 {
    let base = floor(p / DOT_PITCH - 0.5);
    let soft = max(pixel, 0.7);
    // The structure of single dots fades out before it would flicker.
    let detail = 1.0 - smoothstep(1.0, 2.2, pixel);
    var light = 1.0;
    for (var j = 0; j < 2; j++) {
        for (var i = 0; i < 2; i++) {
            let cell = base + vec2<f32>(f32(i), f32(j));
            let center = (cell + 0.5) * DOT_PITCH;
            // A pin fires for any stroke between it and the next, so thin
            // lines between two rows still print.
            let reach = vec2<f32>(0.0, DOT_PITCH.y * 0.3);
            let above = black(textureSampleLevel(canvas, smp, (center - reach) / texels, 0.0).rgb);
            let below = black(textureSampleLevel(canvas, smp, (center + reach) / texels, 0.0).rgb);
            let level = min(above, below);
            let fired = smoothstep(0.4, 0.55, 1.0 - level);
            let d = length(p - center);
            let pin = 1.0 - 0.2 * step(0.8, hash1(vec2<f32>(cell.y % 9.0, 1.0), run + 1.0));
            let ribbon = 0.8 + 0.3 * noise(cell * vec2<f32>(0.02, 0.15), run + 2.0);
            let strike = mix(0.9, pin * ribbon * (0.85 + 0.2 * hash1(cell, run + 3.0)), detail);
            var ink: Ink;
            ink.cover = fired * clamp((DOT_RADIUS - d) / soft + 0.5, 0.0, 1.0);
            ink.level = level;
            ink.strike = strike * mix(1.0, 0.75 + 0.35 * (1.0 - d / DOT_RADIUS), detail);
            light *= through(ink);
        }
    }
    return light;
}

/// A dark speck of dried ink, with a ring where it kept the ink off the
/// paper around it. Returns the speck and the ring.
fn hickey(p: vec2<f32>, pixel: f32, s: f32) -> vec2<f32> {
    let size = 160.0;
    let cell = floor(p / size);
    let h = hash(cell, s + 40.0);
    if h.x > 0.04 {
        return vec2<f32>(0.0);
    }
    let center = (cell + 0.2 + 0.6 * h.yz) * size;
    let d = length(p - center);
    let core = mix(1.5, 3.0, h.w);
    let soft = max(pixel, 0.5);
    let speck = clamp((core - d) / soft + 0.5, 0.0, 1.0);
    let ring = clamp((core * 2.6 - d) / soft + 0.5, 0.0, 1.0) * (1.0 - speck);
    return vec2<f32>(speck, ring) * visible(core * 2.0, pixel);
}

/// The canvas at `uv` as `press` prints it. `run` picks what the whole print
/// run shares and `s` what differs from sheet to sheet.
fn printed(
    canvas: texture_2d<f32>,
    smp: sampler,
    uv_in: vec2<f32>,
    dx: vec2<f32>,
    dy: vec2<f32>,
    press: u32,
    run: f32,
    s: f32,
) -> Printed {
    let texels = vec2<f32>(textureDimensions(canvas));
    let pixel = max(length(dx * texels), length(dy * texels));
    var uv = uv_in;
    var p = uv * texels;
    let page = hash(vec2<f32>(run, s), 50.0);

    // Where the type lands on the page.
    if press == PHOTOCOPY {
        // Laid a little askew on the glass.
        let angle = (page.x - 0.5) * 0.012;
        let middle = texels * 0.5 + (page.yz - 0.5) * 12.0;
        let q = p - middle;
        p = middle + vec2<f32>(q.x * cos(angle) - q.y * sin(angle), q.x * sin(angle) + q.y * cos(angle));
    } else if press == TYPEWRITER {
        // Each letter strikes a little above or below the line.
        p.y += (noise(p / vec2<f32>(20.0, 80.0), run + 4.0) - 0.5) * 2.4;
    }
    uv = p / texels;

    let color = average(canvas, smp, uv, dx, dy);
    var spot = tint(color);
    let bare = black(color);
    // The ink under the point, from its darkest neighbor, as the edge of a
    // letter lets through some paper.
    var level = bare;
    if pixel < 2.5 {
        var darkest = bare;
        for (var i = 0; i < 4; i++) {
            let corner = vec2<f32>(f32(i % 2), f32(i / 2)) * 2.0 - 1.0;
            darkest = min(darkest, black(textureSampleLevel(canvas, smp, uv + corner / texels, 0.0).rgb));
        }
        // Far away a pixel covers the letters anyway.
        level = mix(darkest, bare, smoothstep(1.5, 2.5, pixel));
    }
    let cover = clamp((1.0 - bare) / max(1.0 - level, 0.05), 0.0, 1.0);

    var ink: Ink;
    ink.cover = cover;
    ink.level = level;
    ink.strike = 1.0;
    var light = 1.0;
    var relief = 0.0;
    // How the ink colors light that goes through it, and how dense it is.
    var absorb = vec3<f32>(1.0);
    var density = 1.0;
    // Toner and dye on the bare paper, as density.
    var haze = 0.0;

    switch press {
        case DOT_MATRIX: {
            let far = 1.0 - smoothstep(3.0, 5.0, pixel);
            ink.strike = 0.9;
            light = mix(through(ink), dot_matrix(canvas, smp, p, texels, pixel, run), far);
            absorb = vec3<f32>(1.0, 0.98, 0.9);
            density = 0.6;
        }
        case OFFSET: {
            ink.cover = crisp(cover, 0.42, pixel);
            // The press feeds ink in zones across the sheet, and solids
            // mottle a little.
            ink.strike = (0.9 + 0.15 * noise(vec2<f32>(p.x / 240.0, 0.5), run + 5.0))
                * (1.0 + 0.12 * (noise(p / 3.0, s + 6.0) - 0.5) * visible(3.0, pixel));
            let speck = hickey(p, pixel, s);
            ink.cover = max(ink.cover * (1.0 - speck.y), speck.x);
            ink.level = min(ink.level, mix(1.0, 0.03, speck.x));
            light = through(ink);
            // The colored plate sits a little off the black one.
            let shift = (page.yz - 0.5) * 2.5 / texels;
            spot = tint(average(canvas, smp, uv + shift, dx, dy));
        }
        case PHOTOCOPY: {
            // The copier blurs a little, then turns gray into black or
            // white, with ragged edges.
            let soft = average(canvas, smp, uv, widened(dx, texels, 2.4), widened(dy, texels, 2.4));
            let soft_cover = clamp((1.0 - black(soft)) / max(1.0 - level, 0.05), 0.0, 1.0);
            let ragged = (noise(p / 2.5, s + 7.0) - 0.5) * 0.45 * visible(2.5, pixel);
            ink.cover = crisp(soft_cover + ragged, mix(0.28, 0.45, page.w), pixel);
            // It copies in black and white only.
            ink.level = min(level, dot(spot, LUMA));
            spot = vec3<f32>(1.0);
            // Large solids come out pale in the middle.
            if soft_cover > 0.8 {
                let wide = average(canvas, smp, uv, widened(dx, texels, 14.0), widened(dy, texels, 14.0));
                ink.strike = 1.0 - 0.45 * smoothstep(0.85, 1.0, 1.0 - black(wide));
            }
            light = through(ink);
            haze = 0.03 + 0.05 * noise(p / 300.0, run + 8.0);
            haze += 3.0 * specks(p, 50.0, 0.3, 0.4, 1.4, s + 9.0, pixel);
            // The shadow of the open lid along one edge of the page.
            let edges = vec4<f32>(p, texels - p);
            let side = u32(page.x * 4.0) % 4u;
            let shadow = 1.0 - smoothstep(0.0, mix(30.0, 90.0, page.y) * (0.6 + 0.8 * noise(p / 90.0, s + 10.0)), edges[side]);
            haze += 0.6 * shadow * step(0.4, hash1(vec2<f32>(run, 2.0), s));
            density = 0.85;
        }
        case SPIRIT: {
            // Purple dye that bleeds along the fibers of the paper, thinner
            // in some places.
            let soft = average(canvas, smp, uv, widened(dx, texels, 3.0), widened(dy, texels, 3.0));
            let soft_cover = clamp((1.0 - black(soft)) / max(1.0 - level, 0.05), 0.0, 1.0);
            let along = normalize(vec2<f32>(1.0, (page.y - 0.5) * 0.6));
            let fiber = noise(vec2<f32>(dot(p, along) / 7.0, dot(p, vec2<f32>(-along.y, along.x)) / 1.2), s + 11.0);
            ink.cover = crisp(soft_cover + (fiber - 0.5) * 0.4 * visible(1.2, pixel), 0.4, max(pixel, 0.7));
            ink.strike = (0.55 + 0.6 * noise(p / 350.0, s + 12.0)) * mix(0.75, 1.15, mix(uv.x, uv.y, page.z));
            light = through(ink);
            haze = 0.02 + 0.03 * noise(p / 200.0, s + 13.0);
            absorb = vec3<f32>(1.2, 1.45, 0.35);
            density = 0.55;
        }
        case LETTERPRESS: {
            // The type squeezes ink out to its edges and presses into the
            // paper.
            let soft = average(canvas, smp, uv, widened(dx, texels, 3.0), widened(dy, texels, 3.0));
            let soft_cover = clamp((1.0 - black(soft)) / max(1.0 - level, 0.05), 0.0, 1.0);
            let worn = (noise(p / 2.0, run + 14.0) - 0.5) * 0.2 * visible(2.0, pixel);
            ink.cover = crisp(cover + worn, 0.48, pixel);
            let rim = clamp((cover - soft_cover) * 2.5, 0.0, 1.0) * visible(2.0, pixel);
            ink.strike = (0.75 + 0.4 * rim) * (0.8 + 0.35 * noise(p / 40.0, s + 15.0));
            light = through(ink);
            relief = -IMPRESSION * soft_cover;
            absorb = vec3<f32>(1.0, 1.02, 1.08);
            density = 0.95;
        }
        case TYPEWRITER: {
            // Each key strikes as hard as the typist hit it, through a
            // cloth ribbon.
            let soft = average(canvas, smp, uv, widened(dx, texels, 1.8), widened(dy, texels, 1.8));
            let soft_cover = clamp((1.0 - black(soft)) / max(1.0 - level, 0.05), 0.0, 1.0);
            ink.cover = crisp(soft_cover, 0.4, max(pixel, 0.5));
            let weave = sin(p.x * 3.1) * sin(p.y * 3.1) * visible(2.0, pixel);
            ink.strike = (0.55 + 0.6 * noise(p / vec2<f32>(18.0, 40.0), run + 16.0))
                * (0.9 + 0.2 * noise(p / vec2<f32>(20.0, 8.0), run + 17.0))
                * (0.92 + 0.08 * weave);
            light = through(ink);
            absorb = vec3<f32>(1.0, 1.0, 0.95);
            density = 0.7;
        }
        default: {
            light = through(ink);
        }
    }

    let ink_density = -log(max(light, DARKEST)) * density + haze;
    var out: Printed;
    out.color = PAPER * exp(-ink_density * absorb) * spot;
    out.relief = relief;
    return out;
}
