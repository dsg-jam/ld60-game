use bevy::prelude::*;

/// The paper a print run went onto, and how far it has aged since.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stock {
    /// The fresh paper, as a shade of the canvas's paper color.
    tint: Vec3,
    /// How far the paper has yellowed, from 0 for new to 1 for old.
    age: f32,
}

impl Stock {
    /// The continuous forms in the shop's printer, fairly new.
    pub const SHOP: Self = Self::new(Vec3::new(1.02, 1.03, 1.12), 0.15);
    /// Bright coated paper, like a chip maker's databook.
    pub const BRIGHT: Self = Self::new(Vec3::new(1.08, 1.1, 1.3), 0.05);
    /// Creamy book paper from a print shop.
    pub const CREAM: Self = Self::new(Vec3::new(1.0, 0.96, 0.84), 0.35);
    /// Cheap grayish copier paper.
    pub const RECYCLED: Self = Self::new(Vec3::new(0.85, 0.86, 0.88), 0.25);
    /// Soft duplicator paper that has gone yellow over the years.
    pub const OLD: Self = Self::new(Vec3::new(0.97, 0.88, 0.66), 0.75);
    /// Thick laid paper from a small print shop.
    pub const LAID: Self = Self::new(Vec3::new(0.98, 0.93, 0.78), 0.5);
    /// Typing paper kept in a drawer.
    pub const TYPING: Self = Self::new(Vec3::new(1.0, 0.96, 0.86), 0.45);
    /// Newsprint, gray and quick to yellow.
    pub const NEWSPRINT: Self = Self::new(Vec3::new(0.88, 0.84, 0.7), 0.6);

    const fn new(tint: Vec3, age: f32) -> Self {
        Self { tint, age }
    }

    /// The stock for the shader, see `stock.wesl`.
    pub(super) fn params(self) -> Vec4 {
        self.tint.extend(self.age)
    }
}

impl Default for Stock {
    fn default() -> Self {
        Self::SHOP
    }
}
