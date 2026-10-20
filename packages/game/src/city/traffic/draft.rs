use bevy::prelude::*;

/// How fast a gust travels from a vehicle to the shop, in meters per
/// second, so it comes a little after the vehicle passed. Faster than any
/// vehicle, so each gust comes from one place.
const GUST_SPEED: f32 = 20.0;
/// Within this many meters a vehicle's gust reaches the shop in full.
const REACH: f32 = 3.0;
/// The share of a vehicle's speed that the air along its path carries.
const CARRY: f32 = 0.08;
/// The share of a vehicle's speed that pushes the air away from it, towards
/// the shop.
const PUSH: f32 = 0.05;

/// Air that passing vehicles push into the shop, in meters per second.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Draft(pub(crate) Vec3);

impl Draft {
    /// The gust at the shop, at the origin, from a vehicle at `at` going at
    /// `velocity`. It left the vehicle where it was a moment ago.
    pub(super) fn from_vehicle(at: Vec3, velocity: Vec3) -> Vec3 {
        let then = at - velocity * at.length() / GUST_SPEED;
        let distance = then.length();
        if distance < f32::EPSILON {
            return Vec3::ZERO;
        }
        let falloff = 1.0 / (1.0 + (distance / REACH).powi(2));
        let away = -then / distance;
        (velocity * CARRY + away * velocity.length() * PUSH) * falloff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gust_follows_the_vehicle_and_fades_with_distance() {
        let velocity = Vec3::X * 11.0;
        let passing = |x: f32| Draft::from_vehicle(Vec3::new(x, 0.0, -4.0), velocity);
        let strongest = (-40..40)
            .map(|x| x as f32 * 0.5)
            .max_by(|&a, &b| passing(a).length().total_cmp(&passing(b).length()))
            .unwrap();
        assert!(
            strongest > 0.0,
            "the gust comes after the car, at {strongest}"
        );
        assert!(passing(strongest).x > 0.0);
        assert!(passing(strongest).z > 0.0, "pushed towards the shop");
        assert!(passing(60.0).length() < passing(strongest).length() / 20.0);
    }
}
