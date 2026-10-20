use bevy::prelude::*;
use ld60_devices::DeviceKind;

use super::clutter::Clutter;
use super::computer::ComputerLayout;
use super::counter::CounterLayout;
use super::manuals::ShopManuals;
use super::{
    BELL_SPOT, COMPUTER_SPOT, COMPUTER_YAW, LAMP_SPOT, LAMP_YAW, MANUALS_SPOT, MANUALS_YAW,
    RADIO_SPOT, RADIO_YAW, STAMP_SPOT, STAMP_YAW, job, lamp, radio, receipt, rent, salvage, ticket,
    tools,
};
use crate::device::Handheld;
use crate::geometry::block::Block;

/// Space kept between the keyboard and the mat.
const MAT_MARGIN: f32 = 0.01;

/// The rectangle a prop covers on the counter, seen from above.
pub(super) struct Footprint {
    name: &'static str,
    at: Transform,
    /// The prop in its own space.
    area: Block,
}

impl Footprint {
    pub(super) fn new(name: &'static str, at: Transform, area: Block) -> Self {
        Self { name, at, area }
    }

    fn corners(&self) -> [Vec2; 4] {
        let (min, max) = (self.area.min(), self.area.max());
        [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(min.x, min.y, max.z),
        ]
        .map(|corner| self.at.transform_point(corner).xz())
    }

    fn axes(&self) -> [Vec2; 2] {
        [Vec3::X, Vec3::Z].map(|axis| (self.at.rotation * axis).xz())
    }

    /// Whether the two rectangles share any area, by the separating axis
    /// test.
    fn overlaps(&self, other: &Self) -> bool {
        let (ours, theirs) = (self.corners(), other.corners());
        let span = |corners: &[Vec2; 4], axis: Vec2| {
            corners
                .iter()
                .map(|corner| corner.dot(axis))
                .fold((f32::MAX, f32::MIN), |(min, max), at| {
                    (min.min(at), max.max(at))
                })
        };
        self.axes().into_iter().chain(other.axes()).all(|axis| {
            let (a, b) = (span(&ours, axis), span(&theirs, axis));
            a.1 > b.0 && b.1 > a.0
        })
    }
}

fn sheet(size: Vec2) -> Block {
    Block::new(size.extend(0.0).xzy())
}

fn counter_props(counter: &CounterLayout) -> Vec<Footprint> {
    let computer = ComputerLayout::new();
    let at_computer = counter.place(COMPUTER_SPOT, COMPUTER_YAW);
    let device = DeviceKind::ALL
        .map(Handheld::body_size)
        .into_iter()
        .fold(Vec3::ZERO, Vec3::max);
    let mut props = vec![
        Footprint::new("monitor", at_computer, computer.monitor_body()),
        Footprint::new("keyboard", at_computer, computer.keyboard_placement),
        Footprint::new(
            "lamp",
            counter.place(LAMP_SPOT, LAMP_YAW),
            Block::new(lamp::BASE_SIZE),
        ),
        Footprint::new(
            "radio",
            counter.place(RADIO_SPOT, RADIO_YAW),
            Block::new(radio::CASE),
        ),
        Footprint::new(
            "stamp",
            counter.place(STAMP_SPOT, STAMP_YAW),
            Block::new(tools::STAMP_BASE_SIZE),
        ),
        Footprint::new(
            "bell",
            counter.place(BELL_SPOT, 0.0),
            Block::new(tools::BELL_BASE_SIZE),
        ),
        Footprint::new(
            "shop manuals",
            counter.place(MANUALS_SPOT, MANUALS_YAW),
            Block::new(ShopManuals::CLOSED),
        ),
        Footprint::new(
            "receipt",
            counter.place(receipt::SPOT, receipt::YAW),
            sheet(receipt::SHEET_SIZE),
        ),
        Footprint::new(
            "rent note",
            counter.place(rent::SPOT, rent::YAW),
            sheet(rent::SHEET_SIZE),
        ),
        Footprint::new(
            "salvage crate",
            counter.place(salvage::SPOT, salvage::YAW),
            Block::new(salvage::CRATE_SIZE),
        ),
        Footprint::new(
            "job ticket",
            counter.place(job::TICKET_SPOT, job::TICKET_YAW),
            Block::new(ticket::BOARD_SIZE),
        ),
        Footprint::new(
            "device",
            counter.place(job::DEVICE_SPOT, job::DEVICE_YAW),
            Block::new(device),
        ),
    ];
    props.extend(Clutter::footprints(counter));
    props
}

#[test]
fn the_keyboard_stays_off_the_mat() {
    let counter = CounterLayout::new();
    let keyboard = Footprint::new(
        "keyboard",
        counter.place(COMPUTER_SPOT, COMPUTER_YAW),
        ComputerLayout::new().keyboard_placement,
    );
    let mat_min = counter.mat.min().xz() - MAT_MARGIN;
    let mat_max = counter.mat.max().xz() + MAT_MARGIN;
    for corner in keyboard.corners() {
        assert!(
            corner.cmplt(mat_min).any() || corner.cmpgt(mat_max).any(),
            "keyboard corner {corner} lies on the mat"
        );
    }
}

#[test]
fn no_two_counter_props_overlap() {
    let props = counter_props(&CounterLayout::new());
    for (index, prop) in props.iter().enumerate() {
        for other in &props[index + 1..] {
            assert!(
                !prop.overlaps(other),
                "{} overlaps {}",
                prop.name,
                other.name
            );
        }
    }
}
