use bevy::prelude::*;

/// How far below a rotor its air still pushes, in meters.
const REACH: f32 = 0.5;
/// How much wider the jet gets per meter it falls.
const SPREAD: f32 = 0.35;
/// Within this height over the floor the air turns and runs out along it.
const GROUND: f32 = 0.06;
/// The air running out along the floor thins out over this height, and is
/// gone this far from where it hit.
const OUTWASH_HEIGHT: f32 = 0.15;
const OUTWASH_REACH: f32 = 1.2;

pub(super) struct DownwashPlugin;

impl Plugin for DownwashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Downwash>();
        #[cfg(feature = "debug_port")]
        {
            use crate::debug_port::DebugMethods;
            app.add_debug_method("ld60.wind", Downwash::debug_wind);
        }
    }
}

/// The air that hovering things push down, in meters per second. Whatever
/// flies fills in its [`Jet`]s each frame. Parts sample it with
/// [`Downwash::at`].
#[derive(Resource, Debug, Default)]
pub struct Downwash {
    pub jets: Vec<Jet>,
    /// Jets that stay until they are taken away, like ones set over the
    /// debug port.
    pub held: Vec<Jet>,
    /// The height of the floor the air runs out along.
    pub floor: f32,
}

/// The air pushed down from under one hovering thing.
#[derive(Clone, Copy, Debug)]
pub struct Jet {
    pub at: Vec3,
    /// The way the air goes, a unit vector.
    pub down: Vec3,
    pub radius: f32,
    /// How fast the air leaves, in meters per second.
    pub speed: f32,
}

impl Downwash {
    /// The air at `point`, in world space.
    pub fn at(&self, point: Vec3) -> Vec3 {
        self.jets
            .iter()
            .chain(&self.held)
            .map(|jet| jet.at(point, self.floor))
            .sum()
    }
}

#[cfg(feature = "debug_port")]
impl Downwash {
    /// `ld60.wind { "at": [x, y, z], "speed": 2.0, "radius": 0.1 }` holds a
    /// jet blowing straight down from `at`. `radius` is optional. Without
    /// parameters the held jets are taken away.
    fn debug_wind(
        In(params): In<Option<serde_json::Value>>,
        mut downwash: ResMut<Downwash>,
    ) -> bevy::remote::BrpResult {
        let Some(params) = params.filter(|params| !params.is_null()) else {
            downwash.held.clear();
            return Ok(serde_json::Value::Null);
        };
        let number = |name: &str| params.get(name).and_then(serde_json::Value::as_f64);
        let at = params
            .get("at")
            .and_then(serde_json::Value::as_array)
            .and_then(|xyz| {
                let axis = |i: usize| xyz.get(i)?.as_f64().map(|v| v as f32);
                Some(Vec3::new(axis(0)?, axis(1)?, axis(2)?))
            })
            .ok_or_else(|| bevy::remote::BrpError {
                code: bevy::remote::error_codes::INVALID_PARAMS,
                message: "expected `at` as [x, y, z]".into(),
                data: None,
            })?;
        downwash.held.push(Jet {
            at,
            down: Vec3::NEG_Y,
            radius: number("radius").unwrap_or(0.1) as f32,
            speed: number("speed").unwrap_or(2.0) as f32,
        });
        Ok(serde_json::Value::Null)
    }
}

impl Jet {
    fn at(&self, point: Vec3, floor: f32) -> Vec3 {
        self.column(point, floor) + self.outwash(point, floor)
    }

    /// The air under the rotors. It widens and slows as it falls and keeps
    /// its push.
    fn column(&self, point: Vec3, floor: f32) -> Vec3 {
        let offset = point - self.at;
        let depth = offset.dot(self.down);
        if !(0.0..REACH).contains(&depth) {
            return Vec3::ZERO;
        }
        let across = offset - self.down * depth;
        let width = self.width(depth);
        let speed = self.speed(depth) * (-across.length_squared() / (width * width)).exp();
        let low = (1.0 - (point.y - floor) / GROUND).clamp(0.0, 1.0);
        self.down * speed * (1.0 - low)
    }

    /// The air that hit the floor, running out along it and slowing as it
    /// spreads, like a wall jet.
    fn outwash(&self, point: Vec3, floor: f32) -> Vec3 {
        if self.down.y >= -0.5 {
            return Vec3::ZERO;
        }
        let fall = (floor - self.at.y) / self.down.y;
        if !(0.0..REACH).contains(&fall) {
            return Vec3::ZERO;
        }
        let hit = self.at + self.down * fall;
        let away = (point - hit).with_y(0.0);
        let distance = away.length();
        let width = self.width(fall);
        let height = (point.y - floor).max(0.0);
        let spread = (width / distance.max(width)) * (1.0 - distance / OUTWASH_REACH).max(0.0);
        let speed = self.speed(fall) * spread * (-height / OUTWASH_HEIGHT).exp();
        away.normalize_or_zero() * speed * (distance / width).min(1.0)
    }

    fn width(&self, depth: f32) -> f32 {
        self.radius + depth * SPREAD
    }

    /// How fast the air goes in the middle of the jet `depth` below it.
    fn speed(&self, depth: f32) -> f32 {
        self.speed * (self.radius / self.width(depth)).powi(2) * (1.0 - depth / REACH)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jet() -> Downwash {
        Downwash {
            jets: vec![Jet {
                at: Vec3::Y,
                down: Vec3::NEG_Y,
                radius: 0.1,
                speed: 2.0,
            }],
            ..default()
        }
    }

    #[test]
    fn the_air_blows_down_under_the_rotors_and_out_along_the_floor() {
        let wash = jet();
        assert!(wash.at(Vec3::new(0.0, 0.8, 0.0)).y < -0.3);
        assert_eq!(wash.at(Vec3::new(0.0, 1.2, 0.0)), Vec3::ZERO);
        assert!(wash.at(Vec3::new(0.5, 0.8, 0.0)).length() < 0.01);
        let high = wash.at(Vec3::new(0.05, 0.6, 0.0)).with_y(0.0);
        assert_eq!(high, Vec3::ZERO);
        let jet = Jet {
            at: Vec3::Y * 0.3,
            ..wash.jets[0]
        };
        let low = Downwash {
            jets: vec![jet],
            ..default()
        };
        assert!(low.at(Vec3::new(0.2, 0.01, 0.0)).x > 0.1);
        let near = low.at(Vec3::new(0.3, 0.01, 0.0)).x;
        let far = low.at(Vec3::new(0.6, 0.01, 0.0)).x;
        assert!(near > far && far > 0.0, "{near} {far}");
        assert!(low.at(Vec3::new(0.3, 0.5, 0.0)).x < near / 10.0);
    }
}
