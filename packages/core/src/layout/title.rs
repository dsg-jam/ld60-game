use bevy::prelude::*;

/// The camera looks at the shop from across the street.
pub const EYE: Vec3 = Vec3::new(2.2, 1.5, -5.0);
/// How far the camera sways to the side and up, in meters.
pub const SWAY: Vec2 = Vec2::new(0.35, 0.06);
/// Where the camera can be on the title screen, at the corners of its sway.
pub const TITLE_EYES: [Vec3; 4] = [
    Vec3::new(EYE.x - SWAY.x, EYE.y - SWAY.y, EYE.z),
    Vec3::new(EYE.x - SWAY.x, EYE.y + SWAY.y, EYE.z),
    Vec3::new(EYE.x + SWAY.x, EYE.y - SWAY.y, EYE.z),
    Vec3::new(EYE.x + SWAY.x, EYE.y + SWAY.y, EYE.z),
];
