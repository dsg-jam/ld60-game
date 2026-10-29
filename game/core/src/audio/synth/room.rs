use std::f32::consts::PI;

use bevy::prelude::*;
use ld60_geometry::block::Block;

use crate::layout::shop::{PILLAR, ShopLayout};

/// Meters per second.
const SOUND_SPEED: f32 = 343.0;
/// Share of the sound that surfaces swallow around 1 kHz, from common
/// tables of absorption coefficients.
const PLASTER: f32 = 0.04;
const PLANKS: f32 = 0.1;
const WOOL_RUG: f32 = 0.3;
/// Shelves full of boxes swallow and scatter much of what hits them.
const STOCKED_SHELF: f32 = 0.4;
/// The cabinet, the counter and the pillars.
const PAINTED_WOOD: f32 = 0.06;
/// Sound that leaves through the open front never comes back.
const OPEN: f32 = 1.0;
/// Square meters of open window that a seated person swallows as much as.
const PLAYER: f32 = 0.5;
/// Things on the counter only send sound up and to the sides, so they are
/// twice as loud as they would be in free air.
const DIRECTIVITY: f32 = 2.0;

/// How the shop answers a sound on the counter, after the image source
/// method of Allen and Berkley, "Image method for efficiently simulating
/// small-room acoustics", and Eyring's reverberation time.
#[derive(Debug)]
pub(super) struct Room {
    /// The first reflections off the walls and the ceiling.
    pub(super) echoes: Vec<Echo>,
    /// Seconds for the sound to die down by 60 dB.
    pub(super) decay: f32,
    /// Energy of the diffuse tail after the echoes, against the sound
    /// heard directly.
    pub(super) tail: f32,
    /// Seconds from the direct sound until the tail starts.
    pub(super) tail_start: f32,
}

/// A reflection, heard `delay` seconds after the direct sound and `gain`
/// times as loud.
#[derive(Debug)]
pub(super) struct Echo {
    pub(super) delay: f32,
    pub(super) gain: f32,
}

impl Room {
    /// The shop heard by the seated player leaning in towards `source`
    /// until it is `reach` meters away.
    pub(super) fn leaning_in(source: Vec3, seat: Vec3, reach: f32) -> Self {
        Self::new(source, source + (seat - source).normalize() * reach)
    }

    /// The shop heard from `ear` when something at `source` on the counter
    /// makes a sound.
    pub(super) fn new(source: Vec3, ear: Vec3) -> Self {
        let shop = ShopLayout::new();
        let air = shop.air();
        let (min, max) = (air.min(), air.max());
        // Each surface as its axis, its position on it and how much it
        // swallows. The floor is under the counter top, out of reach.
        let surfaces = [
            (Vec3::X, min.x, PLASTER),
            (Vec3::X, max.x, PLASTER),
            (Vec3::Y, max.y, PLASTER),
            (Vec3::Z, max.z, PLASTER),
            (Vec3::Z, min.z, OPEN),
        ];
        let direct = source.distance(ear);
        let echoes: Vec<Echo> = surfaces
            .iter()
            .filter(|(_, _, absorption)| *absorption < 1.0)
            .map(|&(axis, at, absorption)| {
                let image = source + axis * 2.0 * (at - source.dot(axis));
                let path = image.distance(ear);
                Echo {
                    delay: (path - direct) / SOUND_SPEED,
                    gain: direct / path * (1.0 - absorption).sqrt(),
                }
            })
            .collect();

        let size = air.size;
        let covered = |block: &Block| block.size.y * block.size.z;
        let front = size.x * size.y;
        let opening = (size.x - 2.0 * PILLAR.x) * (max.y - source.y);
        let absorbers = [
            (size.x * size.z - shop.rug.size.x * shop.rug.size.z, PLANKS),
            (shop.rug.size.x * shop.rug.size.z, WOOL_RUG),
            (size.x * size.z, PLASTER),
            (size.x * size.y, PLASTER),
            (
                2.0 * size.y * size.z - covered(&shop.shelf) - covered(&shop.cabinet),
                PLASTER,
            ),
            (covered(&shop.shelf), STOCKED_SHELF),
            (covered(&shop.cabinet), PAINTED_WOOD),
            (opening, OPEN),
            (front - opening, PAINTED_WOOD),
        ];
        let surface: f32 = absorbers.iter().map(|(area, _)| area).sum();
        let sabins = PLAYER
            + absorbers
                .iter()
                .map(|(area, absorption)| area * absorption)
                .sum::<f32>();
        let mean = sabins / surface;
        let volume = size.x * size.y * size.z;
        let decay = 0.161 * volume / (-surface * (1.0 - mean).ln());
        // The diffuse field against the direct sound, minus what the echoes
        // already bring.
        let diffuse = 16.0 * PI * direct * direct * (1.0 - mean) / (DIRECTIVITY * sabins);
        let early: f32 = echoes.iter().map(|echo| echo.gain * echo.gain).sum();
        let tail_start = echoes.iter().map(|echo| echo.delay).fold(0.0, f32::max);
        Self {
            echoes,
            decay,
            tail: (diffuse - early).max(0.0),
            tail_start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::desk::DeskSpots;

    #[test]
    fn shop_is_small_and_open() {
        let spots = DeskSpots::new();
        let room = Room::new(spots.manuals, spots.seat);
        assert_eq!(room.echoes.len(), 4);
        assert!(
            room.echoes
                .iter()
                .all(|echo| echo.delay > 0.0 && echo.gain < 1.0)
        );
        assert!((0.1..1.0).contains(&room.decay), "{room:?}");
    }
}
