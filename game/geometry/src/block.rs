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
pub struct Block {
    pub center: Vec3,
    pub size: Vec3,
}

impl Block {
    /// A block resting on the ground (y = 0), centered on the other axes.
    pub fn new(size: Vec3) -> Self {
        Self {
            center: Vec3::Y * size.y / 2.0,
            size,
        }
    }

    /// The block spanning two opposite corners.
    pub fn between(a: Vec3, b: Vec3) -> Self {
        Self {
            center: (a + b) / 2.0,
            size: (b - a).abs(),
        }
    }

    pub fn min(&self) -> Vec3 {
        self.center - self.size / 2.0
    }

    pub fn max(&self) -> Vec3 {
        self.center + self.size / 2.0
    }

    /// Center of the bottom face.
    pub fn bottom(&self) -> Vec3 {
        self.center.with_y(self.min().y)
    }

    /// A point on the top face.
    ///
    /// `spot` goes from -1 to 1 along x and along z (back to front).
    pub fn top_spot(&self, spot: Vec2) -> Vec3 {
        let offset = spot * self.size.xz() / 2.0;
        Vec3::new(
            self.center.x + offset.x,
            self.max().y,
            self.center.z + offset.y,
        )
    }

    pub fn transform(&self) -> Transform {
        Transform::from_translation(self.center)
    }

    /// Moves the center onto the center of `other`.
    pub fn centered_on(self, other: &Block) -> Self {
        Self {
            center: other.center,
            ..self
        }
    }

    pub fn on_top_of(self, other: &Block) -> Self {
        self.with_y(other.max().y + self.size.y / 2.0)
    }

    /// Rests the block on the top face of `other` at `spot`, see
    /// [`Block::top_spot`].
    pub fn resting_on(self, other: &Block, spot: Vec2) -> Self {
        self.standing_at(other.top_spot(spot))
    }

    /// Places the block so the center of its bottom face is at `point`.
    pub fn standing_at(self, point: Vec3) -> Self {
        Self {
            center: point + Vec3::Y * self.size.y / 2.0,
            ..self
        }
    }

    pub fn below(self, other: &Block) -> Self {
        self.with_y(other.min().y - self.size.y / 2.0)
    }

    pub fn left_of(self, other: &Block) -> Self {
        self.with_x(other.min().x - self.size.x / 2.0)
    }

    pub fn right_of(self, other: &Block) -> Self {
        self.with_x(other.max().x + self.size.x / 2.0)
    }

    /// Places the block against the front face of `other`.
    pub fn in_front_of(self, other: &Block) -> Self {
        self.with_z(other.max().z + self.size.z / 2.0)
    }

    /// Places the block against the back face of `other`.
    pub fn behind(self, other: &Block) -> Self {
        self.with_z(other.min().z - self.size.z / 2.0)
    }

    pub fn translated(self, offset: Vec3) -> Self {
        Self {
            center: self.center + offset,
            ..self
        }
    }

    fn with_x(self, x: f32) -> Self {
        Self {
            center: self.center.with_x(x),
            ..self
        }
    }

    fn with_y(self, y: f32) -> Self {
        Self {
            center: self.center.with_y(y),
            ..self
        }
    }

    fn with_z(self, z: f32) -> Self {
        Self {
            center: self.center.with_z(z),
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
        let stand = Block::new(Vec3::new(0.2, 0.03, 0.2));
        let body = Block::new(Vec3::new(0.38, 0.32, 0.36)).on_top_of(&stand);
        assert_close(body.min().y, stand.max().y);

        let neck = Block::new(Vec3::new(0.28, 0.24, 0.12))
            .centered_on(&body)
            .behind(&body);
        assert_close(neck.max().z, body.min().z);
        assert_close(neck.center.y, body.center.y);
    }

    #[test]
    fn resting_on_a_spot() {
        let top = Block::new(Vec3::new(2.0, 0.1, 1.0));
        let item = Block::new(Vec3::splat(0.2)).resting_on(&top, Vec2::new(1.0, -1.0));
        assert_close(item.min().y, top.max().y);
        assert_close(item.center.x, 1.0);
        assert_close(item.center.z, -0.5);
    }

    #[test]
    fn frame_surrounds_opening() {
        let opening = Block::new(Vec3::new(0.3, 0.2, 0.0));
        let top = Block::new(Vec3::new(0.3, 0.02, 0.01))
            .centered_on(&opening)
            .on_top_of(&opening);
        let left = Block::new(Vec3::new(0.02, 0.2, 0.01))
            .centered_on(&opening)
            .left_of(&opening);
        assert_close(top.min().y, opening.max().y);
        assert_close(left.max().x, opening.min().x);
    }
}
