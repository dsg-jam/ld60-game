use bevy::prelude::*;

/// An axis-aligned box used to place parts relative to each other.
///
/// All placement methods return a moved copy and only change the axis they
/// talk about. +Y is up and +Z is the front, facing the player.
///
/// ```ignore
/// let stand = Block::new(Vec3::new(0.2, 0.03, 0.2));
/// let body = Block::new(Vec3::splat(0.3)).on_top_of(&stand);
/// assert_eq!(body.min().y, stand.max().y);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Block {
    pub(crate) center: Vec3,
    pub(crate) size: Vec3,
}

impl Block {
    /// A block resting on the ground (y = 0), centered on the other axes.
    pub(crate) fn new(size: Vec3) -> Self {
        Self {
            center: Vec3::Y * size.y / 2.0,
            size,
        }
    }

    pub(crate) fn min(&self) -> Vec3 {
        self.center - self.size / 2.0
    }

    pub(crate) fn max(&self) -> Vec3 {
        self.center + self.size / 2.0
    }

    /// Center of the bottom face.
    pub(crate) fn bottom(&self) -> Vec3 {
        self.center.with_y(self.min().y)
    }

    pub(crate) fn transform(&self) -> Transform {
        Transform::from_translation(self.center)
    }

    pub(crate) fn on_top_of(self, other: &Block) -> Self {
        self.with_y(other.max().y + self.size.y / 2.0)
    }

    pub(crate) fn translated(self, offset: Vec3) -> Self {
        Self {
            center: self.center + offset,
            ..self
        }
    }

    fn with_y(self, y: f32) -> Self {
        Self {
            center: self.center.with_y(y),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    #[test]
    fn stacking_touches_faces() {
        let base = Block::new(Vec3::new(0.3, 0.02, 0.1));
        let key = Block::new(Vec3::splat(0.01)).on_top_of(&base);
        assert_close(key.min().y, base.max().y);
        assert_close(key.bottom().y, base.max().y);
    }
}
