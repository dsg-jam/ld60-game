use bevy::prelude::*;
use ld60_devices::DeviceKind;
use ld60_geometry::block::Block;

use super::handheld::Model;

/// A device seen from above, flattened to colored rectangles.
///
/// The rectangles are in meters, centered on the device, with +Y towards the
/// back of the device. The first one is the body, the others lie on it.
#[derive(Clone, Debug, PartialEq)]
pub struct Silhouette {
    parts: Vec<(Rect, Color)>,
}

impl Silhouette {
    pub fn of(device: DeviceKind) -> Self {
        Model::of(device).silhouette(device.digits())
    }

    pub(super) fn new(parts: Vec<(Rect, Color)>) -> Self {
        Self { parts }
    }

    /// The footprint of `block`, with the back of the device at +Y.
    pub(super) fn top_view(block: &Block) -> Rect {
        Rect::from_center_size(Vec2::new(block.center.x, -block.center.z), block.size.xz())
    }

    pub fn parts(&self) -> &[(Rect, Color)] {
        &self.parts
    }

    /// The rectangle around all parts.
    pub fn bounds(&self) -> Rect {
        self.parts
            .iter()
            .fold(Rect::EMPTY, |bounds, (part, _)| bounds.union(*part))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_lie_on_the_body() {
        for device in DeviceKind::ALL {
            let silhouette = Silhouette::of(device);
            let (body, _) = silhouette.parts()[0];
            assert_eq!(silhouette.bounds(), body, "{device:?}");
            assert!(silhouette.parts().len() > 2, "{device:?}");
        }
    }

    #[test]
    fn devices_look_different() {
        for (index, &device) in DeviceKind::ALL.iter().enumerate() {
            for &other in &DeviceKind::ALL[index + 1..] {
                assert_ne!(Silhouette::of(device), Silhouette::of(other), "{device:?}");
            }
        }
    }
}
