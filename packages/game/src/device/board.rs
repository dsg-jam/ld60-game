use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use ld60_core::parts::pcb::{Pcb, PcbArt, PcbMaterial};
use ld60_geometry::block::Block;
use rand::{RngExt, SeedableRng, rngs::SmallRng};

/// Thickness of the board.
pub(super) const THICKNESS: f32 = 0.0012;
/// How far the board stays inside the edges of the body.
const INSET: f32 = 0.006;
const HEADER: Vec3 = Vec3::new(0.012, 0.0025, 0.005);
/// Header pins sit this far apart, a tenth of an inch.
const PIN_PITCH: f32 = 0.00254;
const HEADER_PINS: usize = 5;
/// Where the chip sits on the board, see [`Block::top_spot`].
const CHIP_SPOT: Vec2 = Vec2::new(0.0, 0.3);
const CHIP: Vec3 = Vec3::new(0.0062, 0.0016, 0.0102);
const CHIP_PINS: usize = 8;
const CHIP_PITCH: f32 = 0.00127;
/// A small capacitor or resistor, 1.6 by 0.8 millimeters.
const SMALL_PART: Vec3 = Vec3::new(0.0016, 0.0006, 0.0008);
const SMALL_PARTS: usize = 5;
const TRACE: f32 = 0.0003;
const POWER_TRACE: f32 = 0.0007;
const TEXT: f32 = 0.0012;
/// Keeps the face of the board from fighting with the board.
const FACE_GAP: f32 = 0.00005;
/// The edge of the board, where the mask shows on the cut glass fiber.
const EDGE: Color = Color::srgb_u8(0x1e, 0x3a, 0x22);
const CHIP_BLACK: Color = Color::srgb_u8(0x1c, 0x1c, 0x1e);
const CAPACITOR: Color = Color::srgb_u8(0x9a, 0x84, 0x5e);
const RESISTOR: Color = Color::srgb_u8(0x24, 0x24, 0x26);
const HEADER_BLACK: Color = Color::srgb_u8(0x1a, 0x1a, 0x1c);

/// Where the board of a device and the parts on it sit, in the device's
/// space. The parts are on the side that faces the back cover.
pub(super) struct BoardLayout {
    pub(super) board: Block,
    pub(super) header: Block,
    /// The chip and the small parts, with their colors.
    pub(super) parts: Vec<(Block, Color)>,
    /// The face of the board towards the cover, as a quad facing down.
    face: Transform,
    screws: Vec<Vec3>,
}

impl BoardLayout {
    /// The board inside `shell`, at `top` of the cover, with the header at
    /// `header` and holes for `screws`, see [`Block::top_spot`]. `seed` places
    /// the small parts.
    pub(super) fn new(shell: &Block, top: f32, header: Vec2, screws: &[Vec2], seed: u64) -> Self {
        let board = Block::new(Vec3::new(
            shell.size.x - 2.0 * INSET,
            THICKNESS,
            shell.size.z - 2.0 * INSET,
        ))
        .translated(Vec3::Y * (top - THICKNESS));
        let below = |block: Vec3, spot: Vec2| {
            Block::new(block).translated(board.top_spot(spot).with_y(board.min().y - block.y))
        };
        let header = below(HEADER, header);
        let chip = below(CHIP, CHIP_SPOT);
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut parts = vec![(chip, CHIP_BLACK)];
        for index in 0..SMALL_PARTS {
            // Along the sides of the chip.
            let side = if index % 2 == 0 { -1.0 } else { 1.0 };
            let along = (index / 2) as f32 * 0.25 - 0.2 + rng.random_range(-0.04..0.04);
            let spot = CHIP_SPOT + Vec2::new(side * rng.random_range(0.42..0.6), along);
            let color = if rng.random_bool(0.5) {
                CAPACITOR
            } else {
                RESISTOR
            };
            parts.push((below(SMALL_PART, spot), color));
        }
        let face = Transform::from_translation(board.bottom() - Vec3::Y * FACE_GAP)
            .with_rotation(Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_z(PI));
        let screws = screws
            .iter()
            .map(|&spot| board.top_spot(spot).with_y(board.min().y))
            .collect();
        Self {
            board,
            header,
            parts,
            face,
            screws,
        }
    }

    /// The face of the board with its copper, mask and print, for a board
    /// called `name`.
    pub(super) fn face(
        &self,
        name: &str,
        seed: u64,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
        pcbs: &mut Assets<PcbMaterial>,
    ) -> impl Bundle {
        let size = self.board.size.xz();
        let mask = images.add(self.art(name, seed).image());
        (
            Mesh3d(meshes.add(Rectangle::from_size(size))),
            MeshMaterial3d(pcbs.add(Pcb::material(mask))),
            self.face,
        )
    }

    pub(super) fn edge() -> Color {
        EDGE
    }

    pub(super) fn header_color() -> Color {
        HEADER_BLACK
    }

    /// Where `point` of the device lies on the face, in meters from its
    /// middle.
    fn to_art(&self, point: Vec3) -> Vec2 {
        (self.face.rotation.inverse() * (point - self.face.translation)).xy()
    }

    /// The footprint of `block` on the face, as its middle and size.
    fn footprint(&self, block: &Block) -> (Vec2, Vec2) {
        let a = self.to_art(block.min());
        let b = self.to_art(block.max());
        ((a + b) / 2.0, (a - b).abs())
    }

    fn art(&self, name: &str, seed: u64) -> PcbArt {
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut art = PcbArt::new(self.board.size.xz());
        let half = art.size() / 2.0;

        let (header, header_size) = self.footprint(&self.header);
        let header_pins = Self::pins(header, header_size, HEADER_PINS, PIN_PITCH, true);
        for &pin in &header_pins {
            art.pad(pin, Vec2::splat(0.0017));
        }
        Self::outline(&mut art, header, header_size + 0.0012);
        art.silk_text(
            "J1 DEBUG",
            header + Vec2::new(-header_size.x / 2.0, header_size.y / 2.0 + 0.0025),
            TEXT,
        );

        let (chip, chip_size) = self.footprint(&self.parts[0].0);
        let chip_pins = Self::pins(chip, chip_size, CHIP_PINS, CHIP_PITCH, false);
        let lead = if chip_size.x > chip_size.y {
            Vec2::new(0.0006, 0.0015)
        } else {
            Vec2::new(0.0015, 0.0006)
        };
        for &pin in &chip_pins {
            art.pad(pin, lead);
        }
        Self::outline(&mut art, chip, chip_size + 0.0004);
        art.silk_text(
            "U1",
            chip + Vec2::new(chip_size.x / 2.0 + 0.0012, chip_size.y / 2.0),
            TEXT,
        );
        art.silk_line(
            &[chip + (chip_size / 2.0 - 0.0008) * Vec2::new(-1.0, 1.0)],
            0.0006,
        );

        // The debug lines run from the header to the chip, straight out of
        // the header and in at 45 degrees.
        for (index, &from) in header_pins.iter().enumerate() {
            let to = chip_pins[(index * 3) % chip_pins.len()];
            let offset = (to.x - from.x).abs().min((to.y - from.y).abs());
            let bend = Vec2::new(from.x, to.y - (to.y - from.y).signum() * offset);
            art.trace(&[from, bend, to], TRACE);
            if rng.random_bool(0.3) {
                art.via(bend, 0.0005, 0.00025);
            }
        }

        // Power around the edge, and the other chip lines out to the display
        // and the keys.
        let ring = half - 0.003;
        art.trace(
            &[
                Vec2::new(-ring.x, -ring.y),
                Vec2::new(ring.x, -ring.y),
                Vec2::new(ring.x, ring.y),
                Vec2::new(-ring.x, ring.y),
                Vec2::new(-ring.x, -ring.y),
            ],
            POWER_TRACE,
        );
        for &pin in chip_pins.iter().skip(1).step_by(2) {
            let outward = if pin.x > chip.x { 1.0 } else { -1.0 };
            let out = pin + Vec2::new(outward * rng.random_range(0.002..0.006), 0.0);
            let edge = Vec2::new(
                out.x,
                half.y
                    * rng.random_range(0.4..0.85)
                    * if rng.random_bool(0.5) { 1.0 } else { -1.0 },
            );
            art.trace(&[pin, out, edge], TRACE);
            art.via(edge, 0.0005, 0.00025);
        }

        for (block, _) in &self.parts[1..] {
            let (middle, size) = self.footprint(block);
            let (pad, gap) = if size.x > size.y {
                (
                    Vec2::new(size.x * 0.35, size.y),
                    Vec2::new(size.x * 0.4, 0.0),
                )
            } else {
                (
                    Vec2::new(size.x, size.y * 0.35),
                    Vec2::new(0.0, size.y * 0.4),
                )
            };
            art.pad(middle - gap, pad);
            art.pad(middle + gap, pad);
            art.trace(&[middle + gap, chip + (middle - chip) * 0.4], TRACE);
        }

        for &screw in &self.screws {
            art.via(self.to_art(screw), 0.003, 0.0016);
        }
        let corner = Vec2::new(-half.x + 0.004, -half.y + 0.0045);
        art.silk_text(name, corner + Vec2::Y * (TEXT * 1.6), TEXT);
        art.silk_text("DSG ELECTRONICS", corner, TEXT * 0.8);
        art
    }

    /// The pins of a part at `middle` of `size`: two rows of `count` along
    /// its long side, `pitch` apart. Through-hole rows sit on the part,
    /// surface mounted rows stick out at its sides.
    fn pins(middle: Vec2, size: Vec2, count: usize, pitch: f32, through: bool) -> Vec<Vec2> {
        let long_x = size.x > size.y;
        let across = if through {
            pitch / 2.0
        } else {
            size.min_element() / 2.0 + 0.0004
        };
        let mut pins = Vec::with_capacity(2 * count);
        for row in [-1.0, 1.0] {
            for index in 0..count {
                let along = (index as f32 - (count as f32 - 1.0) / 2.0) * pitch;
                let offset = if long_x {
                    Vec2::new(along, row * across)
                } else {
                    Vec2::new(row * across, along)
                };
                pins.push(middle + offset);
            }
        }
        pins
    }

    fn outline(art: &mut PcbArt, middle: Vec2, size: Vec2) {
        let half = size / 2.0;
        art.silk_line(
            &[
                middle + Vec2::new(-half.x, -half.y),
                middle + Vec2::new(half.x, -half.y),
                middle + Vec2::new(half.x, half.y),
                middle + Vec2::new(-half.x, half.y),
                middle + Vec2::new(-half.x, -half.y),
            ],
            0.00018,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parts_fit_inside_the_cover() {
        let shell = Block::new(Vec3::new(0.065, 0.015, 0.085)).translated(Vec3::Y * 0.005);
        let layout = BoardLayout::new(&shell, 0.005, Vec2::new(0.0, -0.65), &[], 1);
        let parts = layout
            .parts
            .iter()
            .map(|(block, _)| block)
            .chain([&layout.header]);
        for part in parts {
            assert!(part.min().y > 0.0, "{part:?}");
            assert!(part.max().y <= layout.board.min().y + 1e-6);
        }
    }
}
