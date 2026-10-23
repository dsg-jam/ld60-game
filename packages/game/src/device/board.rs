use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use ld60_core::parts::pcb::{Pad, Pcb, PcbArt, PcbMaterial, Router};
use ld60_geometry::block::Block;
use rand::{RngExt, SeedableRng, rngs::SmallRng};

/// Thickness of the board.
pub(super) const THICKNESS: f32 = 0.0012;
/// How far the board stays inside the edges of the body.
const INSET: f32 = 0.003;
const HEADER: Vec3 = Vec3::new(0.012, 0.0025, 0.005);
/// Header pins sit this far apart, a tenth of an inch.
const PIN_PITCH: f32 = 0.00254;
const HEADER_PAD: Vec2 = Vec2::splat(0.0017);
/// Where the chip sits on the board, see [`Block::top_spot`].
const CHIP_SPOT: Vec2 = Vec2::new(0.0, 0.45);
const CHIP: Vec3 = Vec3::new(0.0062, 0.0016, 0.0102);
const CHIP_PITCH: f32 = 0.00127;
/// The keypad contacts run down one side, see [`Block::top_spot`].
const KEYPAD_SPOT: Vec2 = Vec2::new(0.72, 0.2);
const BATTERY_SPOT: Vec2 = Vec2::new(-0.62, 0.3);
const CONTACT: Vec2 = Vec2::new(0.0016, 0.0011);
const FINGER: Vec2 = Vec2::new(0.0008, 0.0022);
const BATTERY_PAD: Vec2 = Vec2::new(0.0032, 0.0024);
const SPRING: Vec3 = Vec3::new(0.0024, 0.0012, 0.0018);
/// A small capacitor or resistor, 1.6 by 0.8 millimeters.
const PASSIVE: Vec2 = Vec2::new(0.0016, 0.0008);
const PASSIVE_HEIGHT: f32 = 0.0006;
const TRACE: f32 = 0.00025;
const POWER_TRACE: f32 = 0.0005;
/// The grid the traces are routed on.
const PITCH: f32 = 0.0004;
const TEXT: f32 = 0.0011;
/// Keeps the face of the board from fighting with the board.
const FACE_GAP: f32 = 0.00005;
/// The edge of the board, where the mask shows on the cut glass fiber.
const EDGE: Color = Color::srgb_u8(0x1e, 0x3a, 0x22);
const CHIP_BLACK: Color = Color::srgb_u8(0x1c, 0x1c, 0x1e);
const CAPACITOR: Color = Color::srgb_u8(0x9a, 0x84, 0x5e);
const RESISTOR: Color = Color::srgb_u8(0x24, 0x24, 0x26);
const HEADER_BLACK: Color = Color::srgb_u8(0x1a, 0x1a, 0x1c);
const SPRING_METAL: Color = Color::srgb_u8(0xb8, 0xb8, 0xb4);

/// The signals on a board. Power runs wider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Net {
    Power,
    Ground,
    Reset,
    Clock,
    Data,
    Display(u32),
    Key(u32),
}

/// Where the parts of a board sit, see [`Block::top_spot`] of the board, and
/// the holes for the screws in the device's space.
pub(super) struct BoardSpots<'a> {
    pub(super) header: Vec2,
    pub(super) display: Vec2,
    pub(super) screws: &'a [Vec3],
}

/// A capacitor or resistor between two nets.
struct Passive {
    block: Block,
    nets: [Net; 2],
    label: String,
}

/// Where the board of a device and the parts on it sit, in the device's
/// space. The parts are on the side that faces the back cover.
pub(super) struct BoardLayout {
    pub(super) board: Block,
    pub(super) header: Block,
    chip: Block,
    passives: Vec<Passive>,
    springs: Vec<Block>,
    /// Middle of the display contacts, see [`Block::top_spot`].
    display: Vec2,
    /// The face of the board towards the cover, as a quad facing down.
    face: Transform,
    screws: Vec<Vec3>,
    seed: u64,
}

impl Net {
    const fn id(self) -> u32 {
        match self {
            Self::Power => 1,
            Self::Ground => 2,
            Self::Reset => 3,
            Self::Clock => 4,
            Self::Data => 5,
            Self::Display(line) => 10 + line,
            Self::Key(line) => 30 + line,
        }
    }

    const fn width(self) -> f32 {
        match self {
            Self::Power | Self::Ground => POWER_TRACE,
            _ => TRACE,
        }
    }
}

impl BoardLayout {
    /// The board inside `shell`, at `top` of the cover. `seed` places the
    /// small parts and routes the traces, so boards of one revision are
    /// alike.
    pub(super) fn new(shell: &Block, top: f32, spots: &BoardSpots, seed: u64) -> Self {
        let board = Block::new(Vec3::new(
            shell.size.x - 2.0 * INSET,
            THICKNESS,
            shell.size.z - 2.0 * INSET,
        ))
        .translated(Vec3::Y * (top - THICKNESS));
        let below = |size: Vec3, spot: Vec2| {
            Block::new(size).translated(board.top_spot(spot).with_y(board.min().y - size.y))
        };
        let header = below(HEADER, spots.header);
        let chip = below(CHIP, CHIP_SPOT);
        let battery = board.top_spot(BATTERY_SPOT);
        let springs = [-1.0, 1.0]
            .map(|side| {
                Block::new(SPRING).translated(
                    battery.with_y(board.min().y - SPRING.y)
                        + Vec3::Z * side * (BATTERY_PAD.y * 0.7),
                )
            })
            .to_vec();
        let face = Transform::from_translation(board.bottom() - Vec3::Y * FACE_GAP)
            .with_rotation(Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_z(PI));
        let mut layout = Self {
            board,
            header,
            chip,
            passives: Vec::new(),
            springs,
            display: spots.display,
            face,
            screws: spots.screws.to_vec(),
            seed,
        };
        layout.place_passives();
        layout
    }

    /// The parts on the board, with their colors.
    pub(super) fn parts(&self) -> impl Iterator<Item = (&Block, Color)> {
        let passives = self.passives.iter().map(|passive| {
            let color = if passive.label.starts_with('C') {
                CAPACITOR
            } else {
                RESISTOR
            };
            (&passive.block, color)
        });
        [(&self.chip, CHIP_BLACK)]
            .into_iter()
            .chain(passives)
            .chain(self.springs.iter().map(|spring| (spring, SPRING_METAL)))
    }

    /// The face of the board with its copper, mask and print, for a board
    /// called `name`.
    pub(super) fn face(
        &self,
        name: &str,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
        pcbs: &mut Assets<PcbMaterial>,
    ) -> impl Bundle {
        let size = self.board.size.xz();
        let mask = images.add(self.art(name).image());
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

    /// Puts the decoupling capacitors and pull up resistors next to what
    /// they serve, where they fit.
    fn place_passives(&mut self) {
        let mut rng = SmallRng::seed_from_u64(self.seed);
        let chip = self.chip.center;
        let header = self.header.center;
        let battery = self.board.top_spot(BATTERY_SPOT);
        let keypad = self.board.top_spot(KEYPAD_SPOT);
        let wanted = [
            ("C", [Net::Power, Net::Ground], chip),
            ("C", [Net::Power, Net::Ground], header),
            ("R", [Net::Reset, Net::Power], chip),
            ("C", [Net::Power, Net::Ground], battery),
            ("R", [Net::Key(0), Net::Power], keypad),
            ("R", [Net::Key(1), Net::Power], keypad),
        ];
        let (mut capacitors, mut resistors) = (0, 0);
        for (kind, nets, near) in wanted {
            let turned = rng.random_bool(0.5);
            let size = if turned { PASSIVE.yx() } else { PASSIVE };
            let size = size.extend(PASSIVE_HEIGHT).xzy();
            let spot = (0..40).find_map(|_| {
                let offset = Vec3::new(
                    rng.random_range(-1.0..1.0),
                    0.0,
                    rng.random_range(-1.0..1.0),
                );
                let at = near + offset.normalize_or_zero() * rng.random_range(0.006..0.012);
                let block = Block::new(size).translated(at.with_y(self.board.min().y - size.y));
                self.fits(&block).then_some(block)
            });
            let Some(block) = spot else {
                continue;
            };
            let count = if kind == "C" {
                capacitors += 1;
                capacitors
            } else {
                resistors += 1;
                resistors
            };
            self.passives.push(Passive {
                block,
                nets,
                label: format!("{kind}{count}"),
            });
        }
    }

    /// Whether `block` lies on the board, clear of the other parts, the
    /// contacts and the screws.
    fn fits(&self, block: &Block) -> bool {
        const CLEARANCE: f32 = 0.0015;
        let clear = |other: &Block| {
            let gap =
                (block.center - other.center).abs().xz() - (block.size + other.size).xz() / 2.0;
            gap.max_element() > CLEARANCE
        };
        let half = self.board.size.xz() / 2.0 - 0.002;
        let inside = ((block.center - self.board.center).xz().abs() + block.size.xz() / 2.0)
            .cmplt(half)
            .all();
        let keypad =
            Block::new(Vec3::new(0.004, 0.001, 0.012)).translated(self.board.top_spot(KEYPAD_SPOT));
        let display = Block::new(Vec3::new(0.012, 0.001, 0.004))
            .translated(self.board.top_spot(self.display));
        inside
            && clear(&self.header)
            && clear(&self.chip)
            && clear(&keypad)
            && clear(&display)
            && self.springs.iter().all(clear)
            && self.passives.iter().all(|other| clear(&other.block))
            && self
                .screws
                .iter()
                .all(|screw| (screw.xz() - block.center.xz()).length() > 0.005)
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

    /// The pads of the board with their nets. Pads without a net are not
    /// connected.
    fn pads(&self) -> Vec<(Vec2, Vec2, Option<Net>)> {
        let mut pads = Vec::new();

        let (header, header_size) = self.footprint(&self.header);
        let header_nets = [
            Some(Net::Power),
            Some(Net::Ground),
            Some(Net::Clock),
            Some(Net::Data),
            Some(Net::Reset),
            Some(Net::Ground),
            None,
            None,
            None,
            None,
        ];
        let pins = Self::pins(header, header_size, 5, PIN_PITCH, PIN_PITCH / 2.0);
        pads.extend(
            pins.into_iter()
                .zip(header_nets)
                .map(|(pin, net)| (pin, HEADER_PAD, net)),
        );

        let (chip, chip_size) = self.footprint(&self.chip);
        let lead = if chip_size.x > chip_size.y {
            Vec2::new(0.0006, 0.0015)
        } else {
            Vec2::new(0.0015, 0.0006)
        };
        let chip_nets = [
            Net::Reset,
            Net::Clock,
            Net::Data,
            Net::Key(0),
            Net::Key(1),
            Net::Key(2),
            Net::Key(3),
            Net::Ground,
            Net::Display(0),
            Net::Display(1),
            Net::Display(2),
            Net::Display(3),
            Net::Display(4),
            Net::Display(5),
            Net::Display(6),
            Net::Power,
        ];
        let across = chip_size.min_element() / 2.0 + 0.0004;
        let pins = Self::pins(chip, chip_size, 8, CHIP_PITCH, across);
        pads.extend(
            pins.into_iter()
                .zip(chip_nets)
                .map(|(pin, net)| (pin, lead, Some(net))),
        );

        let display = self.to_art(self.board.top_spot(self.display));
        for line in 0..7 {
            let x = (line as f32 - 3.0) * 0.0016;
            pads.push((
                display + Vec2::new(x, 0.0),
                FINGER,
                Some(Net::Display(line)),
            ));
        }

        let keypad = self.to_art(self.board.top_spot(KEYPAD_SPOT));
        for line in 0..4 {
            let y = (line as f32 - 1.5) * 0.0028;
            pads.push((keypad + Vec2::new(0.0, y), CONTACT, Some(Net::Key(line))));
        }

        for (spring, net) in self.springs.iter().zip([Net::Ground, Net::Power]) {
            pads.push((self.to_art(spring.center), BATTERY_PAD, Some(net)));
        }

        for passive in &self.passives {
            let (middle, size) = self.footprint(&passive.block);
            let (pad, gap) = if size.x > size.y {
                (
                    Vec2::new(size.x * 0.4, size.y),
                    Vec2::new(size.x * 0.4, 0.0),
                )
            } else {
                (
                    Vec2::new(size.x, size.y * 0.4),
                    Vec2::new(0.0, size.y * 0.4),
                )
            };
            pads.push((middle - gap, pad, Some(passive.nets[0])));
            pads.push((middle + gap, pad, Some(passive.nets[1])));
        }
        pads
    }

    /// Draws the board: every trace runs between two pads of its net.
    fn art(&self, name: &str) -> PcbArt {
        let size = self.board.size.xz();
        let mut art = PcbArt::new(size);
        let mut router = Router::new(size, PITCH, 0.0015);
        for &screw in &self.screws {
            router.block(self.to_art(screw), Vec2::splat(0.0055));
            art.via(self.to_art(screw), 0.0022, 0.0012);
        }
        let pads = self.pads();
        let routed: Vec<(Pad, Vec2, Vec2, Option<Net>)> = pads
            .iter()
            .enumerate()
            .map(|(index, &(center, size, net))| {
                let pad = router.pad(center, size, index as u32);
                art.pad(center, size);
                (pad, center, size, net)
            })
            .collect();

        let mut nets: Vec<Net> = Vec::new();
        for &(.., net) in &routed {
            if let Some(net) = net.filter(|net| !nets.contains(net)) {
                nets.push(net);
            }
        }
        // Signals first, so power, which can go round, does not wall them in.
        nets.sort_by_key(|net| matches!(net, Net::Power | Net::Ground));
        let mut rng = SmallRng::seed_from_u64(self.seed ^ 0x5a5a);
        for net in nets {
            let members: Vec<_> = routed.iter().filter(|pad| pad.3 == Some(net)).collect();
            let Some((&first, rest)) = members.split_first() else {
                continue;
            };
            router.seed(first.0, net.id());
            let mut joined = vec![first.1];
            for &&(pad, center, ..) in rest {
                if let Some(path) = router.route(pad, center, net.id(), &joined) {
                    art.trace(&path, net.width());
                    // Power and ground go down to the planes inside here
                    // and there.
                    if matches!(net, Net::Power | Net::Ground)
                        && path.len() > 2
                        && rng.random_bool(0.5)
                    {
                        art.via(path[path.len() / 2], 0.0005, 0.00025);
                    }
                    joined.push(center);
                }
            }
        }
        self.print(&mut art, name);
        art
    }

    /// The silkscreen: outlines, names of the parts and of the board. Each
    /// name goes in the first place next to its part that keeps clear of the
    /// pads, the holes, the parts and the other names.
    fn print(&self, art: &mut PcbArt, name: &str) {
        let mut taken: Vec<(Vec2, Vec2)> = self
            .pads()
            .into_iter()
            .map(|(center, size, _)| (center, size + 0.0006))
            .collect();
        taken.extend(
            self.screws
                .iter()
                .map(|&screw| (self.to_art(screw), Vec2::splat(0.0056))),
        );
        let mut parts = vec![self.footprint(&self.header), self.footprint(&self.chip)];
        parts.extend(
            self.passives
                .iter()
                .map(|passive| self.footprint(&passive.block)),
        );
        parts.extend(self.springs.iter().map(|spring| self.footprint(spring)));
        taken.extend(parts.iter().map(|&(center, size)| (center, size + 0.0004)));

        let (header, header_size) = parts[0];
        Self::outline(art, header, header_size + 0.0012);
        let (chip, chip_size) = parts[1];
        Self::outline(art, chip, chip_size + 0.0004);
        art.silk_line(
            &[chip + (chip_size / 2.0 - 0.0008) * Vec2::new(-1.0, 1.0)],
            0.0006,
        );

        let mut labels = vec![
            ("J1 DEBUG".to_owned(), header, header_size + 0.0012, TEXT),
            ("U1".to_owned(), chip, chip_size, TEXT),
        ];
        for passive in &self.passives {
            let (middle, size) = self.footprint(&passive.block);
            labels.push((passive.label.clone(), middle, size, TEXT * 0.8));
        }
        let contacts = [
            ("LCD", self.display, Vec2::new(0.012, 0.0024)),
            ("KEYS", KEYPAD_SPOT, Vec2::new(0.0018, 0.0098)),
            ("BAT", BATTERY_SPOT, Vec2::new(0.0034, 0.0058)),
        ];
        for (text, spot, size) in contacts {
            let middle = self.to_art(self.board.top_spot(spot));
            labels.push((text.to_owned(), middle, size, TEXT * 0.8));
        }
        for (text, middle, size, height) in labels {
            let room = PcbArt::text_size(&text, height);
            let gap = (size + room) / 2.0 + 0.0006;
            let around = [
                Vec2::new(0.0, gap.y),
                Vec2::new(0.0, -gap.y),
                Vec2::new(gap.x, 0.0),
                Vec2::new(-gap.x, 0.0),
            ];
            if let Some(at) = self.free_place(&mut taken, room, around.map(|off| middle + off)) {
                art.silk_text(&text, at, height);
            }
        }

        let lines = [(name, TEXT), ("DSG ELECTRONICS", TEXT * 0.8)];
        let room = Vec2::new(
            lines
                .iter()
                .map(|(text, height)| PcbArt::text_size(text, *height).x)
                .fold(0.0, f32::max),
            TEXT * 1.6 + TEXT * 0.8,
        );
        let reach = self.board.size.xz() / 2.0 - 0.0025 - room / 2.0;
        let spots = [
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
        ];
        if let Some(at) = self.free_place(&mut taken, room, spots.map(|spot| spot * reach)) {
            art.silk_text(name, at, TEXT);
            art.silk_text("DSG ELECTRONICS", at - Vec2::Y * (TEXT * 1.6), TEXT * 0.8);
        }
    }

    /// The top left of the first of the `centers` where `room` fits on the
    /// board and clear of `taken`, which it then takes too.
    fn free_place(
        &self,
        taken: &mut Vec<(Vec2, Vec2)>,
        room: Vec2,
        centers: impl IntoIterator<Item = Vec2>,
    ) -> Option<Vec2> {
        let inside = self.board.size.xz() / 2.0 - 0.001;
        let center = centers.into_iter().find(|&center| {
            let on_board = (center.abs() + room / 2.0).cmplt(inside).all();
            on_board
                && taken.iter().all(|&(other, size)| {
                    ((center - other).abs() - (room + size) / 2.0).max_element() > 0.0
                })
        })?;
        taken.push((center, room));
        Some(center + Vec2::new(-room.x / 2.0, room.y / 2.0))
    }

    /// The pins of a part at `middle` of `size`: two rows of `count` along
    /// its long side, `pitch` apart and `across` from the middle.
    fn pins(middle: Vec2, size: Vec2, count: usize, pitch: f32, across: f32) -> Vec<Vec2> {
        let long_x = size.x > size.y;
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

    fn layout() -> BoardLayout {
        let shell = Block::new(Vec3::new(0.09, 0.009, 0.15)).translated(Vec3::Y * 0.005);
        let spots = BoardSpots {
            header: Vec2::new(0.0, -0.1),
            display: Vec2::new(0.0, -0.6),
            screws: &[Vec3::new(0.036, 0.0, 0.064)],
        };
        BoardLayout::new(&shell, 0.005, &spots, 3)
    }

    #[test]
    fn the_parts_fit_inside_the_cover() {
        let layout = layout();
        for (part, _) in layout.parts().chain([(&layout.header, HEADER_BLACK)]) {
            assert!(part.min().y > 0.0, "{part:?}");
            assert!(part.max().y <= layout.board.min().y + 1e-6);
        }
    }

    #[test]
    fn the_passives_do_not_overlap() {
        let layout = layout();
        assert!(layout.passives.len() >= 4, "{}", layout.passives.len());
        for (index, a) in layout.passives.iter().enumerate() {
            for b in &layout.passives[index + 1..] {
                let gap = (a.block.center - b.block.center).abs().xz()
                    - (a.block.size + b.block.size).xz() / 2.0;
                assert!(gap.max_element() > 0.0);
            }
        }
    }
}
