/// What the outer walls of buildings are clad in, and how weather and age
/// marked them. Positions are in meters across the wall, with y going up.

import package::geometry::noise::{hash1, noise};

/// Matches `Cladding` in `buildings/cladding.rs`.
const CONCRETE: u32 = 0u;
const PANELS: u32 = 1u;
const TILES: u32 = 2u;
const BRICK: u32 = 3u;
const PLASTER: u32 = 4u;

const BRICK_SIZE: vec2<f32> = vec2<f32>(0.24, 0.075);
const TILE_SIZE: f32 = 0.1;
const MORTAR: vec3<f32> = vec3<f32>(0.32, 0.3, 0.27);
const BARE_CONCRETE: vec3<f32> = vec3<f32>(0.27, 0.27, 0.26);
const RUST_STAIN: vec3<f32> = vec3<f32>(0.3, 0.14, 0.06);
const ALGAE: vec3<f32> = vec3<f32>(0.1, 0.14, 0.06);
const SALT: vec3<f32> = vec3<f32>(0.5, 0.5, 0.47);
/// Share of the windows with streaks below them.
const STREAKY: f32 = 0.4;

/// A brick or tile of a regular pattern, and how close to its joint a point
/// is.
struct Unit {
    id: vec2<f32>,
    /// Distance to the nearest joint, in meters.
    joint: f32,
}

/// The unit at `wall`. `shift` moves every other row along by that share of
/// a unit.
fn unit(wall: vec2<f32>, size: vec2<f32>, shift: f32) -> Unit {
    let row = floor(wall.y / size.y);
    let x = wall.x / size.x + shift * (row % 2.0);
    var out: Unit;
    out.id = vec2<f32>(floor(x), row);
    let f = vec2<f32>(fract(x), fract(wall.y / size.y)) * size;
    out.joint = min(min(f.x, size.x - f.x), min(f.y, size.y - f.y));
    return out;
}

/// How much of the point is a joint between bricks, tiles or panels.
fn joint(distance: f32, width: f32) -> f32 {
    return 1.0 - smoothstep(width * 0.5, width, distance);
}

/// The panel between two floors and two windows that a point lies on.
fn panel(wall: vec2<f32>, cell: vec2<f32>) -> Unit {
    // Seams run through the piers between the windows.
    return unit(wall + vec2<f32>(0.0, 0.5 * cell.y), cell, 0.0);
}

/// Plaster that came off, showing the concrete under it.
fn peeled(wall: vec2<f32>, seed: f32) -> f32 {
    let n = noise(wall * 1.3, seed + 30.0) * 0.75 + noise(wall * 7.0, seed + 31.0) * 0.25;
    return smoothstep(0.8, 0.82, n);
}

/// Bumps of the cladding in meters.
fn cladding_height(kind: u32, wall: vec2<f32>, cell: vec2<f32>, seed: f32) -> f32 {
    let fine = 0.0005 * noise(wall * 40.0, seed + 2.0);
    switch kind {
        case PANELS: {
            return fine - 0.008 * joint(panel(wall, cell).joint, 0.012);
        }
        case TILES: {
            return fine - 0.002 * joint(unit(wall, vec2<f32>(TILE_SIZE), 0.0).joint, 0.004);
        }
        case BRICK: {
            let brick = unit(wall, BRICK_SIZE, 0.5);
            return 0.001 * noise(wall * 30.0, seed + 3.0) - 0.006 * joint(brick.joint, 0.01);
        }
        case PLASTER: {
            let uneven = 0.0015 * noise(wall * 6.0, seed + 1.0) + fine;
            return uneven - 0.003 * peeled(wall, seed);
        }
        default: {
            let pores = step(0.94, hash1(floor(wall * 220.0), seed)) * -0.0008;
            let uneven = 0.002 * noise(wall * 3.0, seed + 1.0) + fine;
            let seam = -0.006 * joint(abs(fract(wall.y / cell.y + 0.5) - 0.5) * cell.y, 0.03);
            return pores + uneven + seam;
        }
    }
}

/// The color of the cladding, from the building's color `base`.
fn cladding_color(kind: u32, base: vec3<f32>, wall: vec2<f32>, cell: vec2<f32>, seed: f32) -> vec3<f32> {
    let mottled = mix(0.85, 1.08, noise(wall * 1.5, seed + 5.0));
    switch kind {
        case PANELS: {
            let slab = panel(wall, cell);
            let shade = mix(0.9, 1.08, hash1(slab.id, seed + 6.0));
            return mix(base * shade * mottled, base * 0.55, joint(slab.joint, 0.012));
        }
        case TILES: {
            let tile = unit(wall, vec2<f32>(TILE_SIZE), 0.0);
            let shade = mix(0.85, 1.1, hash1(tile.id, seed + 7.0));
            // Now and then a tile fell off.
            let missing = step(0.995, hash1(tile.id, seed + 8.0));
            let tiled = mix(base * shade, BARE_CONCRETE, missing);
            return mix(tiled, base * 0.5, joint(tile.joint, 0.004));
        }
        case BRICK: {
            let brick = unit(wall, BRICK_SIZE, 0.5);
            let h = hash1(brick.id, seed + 9.0);
            let shade = mix(0.7, 1.2, h * h);
            return mix(base * shade * mottled, MORTAR, joint(brick.joint, 0.01));
        }
        case PLASTER: {
            return mix(base * mottled, BARE_CONCRETE * mottled, peeled(wall, seed));
        }
        default: {
            return base * mottled;
        }
    }
}

/// Marks of weather and age on a wall.
struct Stains {
    /// Factor for the color.
    darken: f32,
    /// A color stains tint the wall with, and how much.
    tint: vec3<f32>,
    amount: f32,
    /// Where rain water runs down, from 0 to 1.
    runoff: f32,
}

/// Lines of water, dirt or rust running down from `top` at `x`, `length`
/// meters long. `wall` is the point to shade.
fn trickle(wall: vec2<f32>, x: vec2<f32>, top: f32, length: f32, seed: f32) -> f32 {
    let below = top - wall.y;
    if below < 0.0 || below >= length || wall.x < x.x || wall.x > x.y {
        return 0.0;
    }
    // Several thin lines, each running its own length.
    let line = noise(vec2<f32>(wall.x * 70.0, 0.5), seed);
    let reach = length * (0.3 + 0.7 * noise(vec2<f32>(wall.x * 25.0, 1.5), seed + 1.0));
    let fade = 1.0 - smoothstep(0.0, reach, below);
    // Water spreads out a little lower down.
    let wobble = noise(vec2<f32>(wall.x * 20.0, wall.y * 2.0), seed + 2.0);
    return smoothstep(0.45, 0.7, line + 0.15 * wobble) * fade * fade;
}

/// Stains on a wall `width` meters wide and `height` high. `ground` is the
/// height above the street, `cell` the window grid and `window` the size of
/// a window in it.
fn stains(
    wall: vec2<f32>,
    width: f32,
    height: f32,
    ground: f32,
    cell: vec2<f32>,
    window: vec2<f32>,
    age: f32,
    seed: f32,
) -> Stains {
    var out: Stains;
    out.darken = mix(0.75, 1.15, noise(wall * 0.2, seed));

    // Under each sill. Long ones reach the floor below, so look at the
    // window above as well.
    let id = floor(wall / cell);
    var streaks = 0.0;
    var rust = 0.0;
    for (var k = 0.0; k < 2.0; k += 1.0) {
        let above = id + vec2<f32>(0.0, k);
        let h = hash1(above, seed + 40.0);
        let center = (above.x + 0.5) * cell.x;
        let half = cell.x * window.x * 0.5;
        let sill = (above.y + 0.5 - window.y * 0.5) * cell.y;
        // Only some sills leak.
        let leaks = step(hash1(above, seed + 41.0), STREAKY);
        let length = cell.y * mix(0.2, 1.6, h) * age * leaks;
        let streak = trickle(wall, vec2<f32>(center - half, center + half), sill, length, seed + above.x * 3.0 + above.y * 7.0);
        streaks = max(streaks, streak);
        // Rusty water from air conditioners and railings.
        rust = max(rust, streak * step(0.75, h));
    }
    out.darken *= 1.0 - 0.5 * streaks * (1.0 - rust);
    out.tint = RUST_STAIN;
    out.amount = 0.6 * rust;

    // Dirt washed down from the roof edge.
    let roof_length = mix(1.0, 5.0, noise(vec2<f32>(wall.x * 0.7, 0.5), seed + 50.0)) * age;
    let roof = trickle(wall, vec2<f32>(-1.0, width + 1.0), height, roof_length, seed + 51.0);
    let roof_band = 1.0 - smoothstep(0.0, 0.8, height - wall.y);
    out.darken *= 1.0 - 0.45 * max(roof, roof_band * 0.6);

    // Damp rising from the street, with a line of salt where it dries.
    let damp_top = mix(0.4, 1.8, noise(vec2<f32>(wall.x * 0.6, 0.5), seed + 60.0)) * age;
    let damp = 1.0 - smoothstep(damp_top - 0.2, damp_top, ground);
    let salt = (1.0 - smoothstep(0.0, 0.06, abs(ground - damp_top))) * age;
    out.darken *= (1.0 - 0.35 * damp) * (1.0 + 0.25 * salt);
    if salt > out.amount {
        out.tint = SALT;
        out.amount = salt * 0.3;
    }

    // Algae in the damp corners.
    let corner = 1.0 - smoothstep(0.0, 0.8, min(wall.x, width - wall.x));
    let algae = max(corner * (1.0 - smoothstep(0.0, 6.0, ground)), damp * 0.5)
        * smoothstep(0.4, 0.8, noise(wall * 2.0, seed + 70.0)) * age;
    if algae > out.amount {
        out.tint = ALGAE;
        out.amount = algae * 0.5;
    }

    // Patches where the wall was mended, in a slightly different shade.
    let patch_size = vec2<f32>(2.3, 1.7);
    let patch_cell = floor(wall / patch_size);
    let patch_h = hash1(patch_cell, seed + 80.0);
    if patch_h < 0.06 * age {
        let f = fract(wall / patch_size);
        let inset = 0.15;
        if all(f > vec2<f32>(inset)) && all(f < vec2<f32>(1.0 - inset * 0.5)) {
            out.darken *= mix(0.8, 1.15, fract(patch_h * 57.0));
        }
    }

    out.runoff = clamp(max(streaks, roof) + damp * 0.5, 0.0, 1.0);
    return out;
}
