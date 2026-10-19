//! The noise of `noise.wesl` on the CPU, for things that need to know what
//! the shaders draw.

use bevy::prelude::*;

/// The PCG hash, as in `noise.wesl`.
fn pcg(seed: u32) -> u32 {
    let state = seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// A random number from 0 to 1 for a cell, like `hash1` in `noise.wesl`.
pub(crate) fn hash1(cell: Vec2, salt: f32) -> f32 {
    let state = pcg(cell.x as i32 as u32 ^ pcg(cell.y as i32 as u32 ^ pcg((salt * 977.0) as u32)));
    state as f32 / u32::MAX as f32
}

/// Smooth noise from 0 to 1 with features of size 1, like `noise` in
/// `noise.wesl`.
pub(crate) fn noise(p: Vec2, salt: f32) -> f32 {
    let i = p.floor();
    let f = p - i;
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash1(i, salt);
    let b = hash1(i + Vec2::X, salt);
    let c = hash1(i + Vec2::Y, salt);
    let d = hash1(i + Vec2::ONE, salt);
    a.lerp(b, u.x).lerp(c.lerp(d, u.x), u.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_stays_in_range() {
        for i in 0..1000 {
            let p = Vec2::new(i as f32 * 0.37 - 180.0, i as f32 * 0.11 - 50.0);
            assert!((0.0..=1.0).contains(&noise(p, 3.0)));
        }
    }
}
