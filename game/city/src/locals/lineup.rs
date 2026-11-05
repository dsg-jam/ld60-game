use bevy::prelude::*;
use ld60_game_core::debug_port::{DebugMethod, DebugMethods, ParamKind};
use rand::SeedableRng;
use rand::rngs::SmallRng;
use serde_json::{Value, json};

use super::Figures;
use super::bird::{Bird, Birth, Carry, Dress, Posture};
use super::plumage::{Coat, PlumageMaterial};

/// Adds `ld60.lineup` to the debug port.
pub(super) struct LineupPlugin;

/// A local spawned in a row to look at, held at one point of its steps.
#[derive(Component, Debug)]
pub(super) struct Lineup {
    phase: Option<f32>,
    /// The height of the seat it sits on, if it sits, and how far down.
    seat: Option<(f32, f32)>,
}

impl Plugin for LineupPlugin {
    fn build(&self, app: &mut App) {
        app.add_debug_method(
            DebugMethod::new(
                "ld60.lineup",
                "Spawns a row of locals to look at, in place of the last row. Without \
                 `at` it only clears the row.",
            )
            .param("at", ParamKind::Numbers, "x,z of the first local")
            .param(
                "yaw",
                ParamKind::Numbers,
                "degrees the locals face, 0 is +Z",
            )
            .param(
                "spacing",
                ParamKind::Numbers,
                "meters between locals, along +X",
            )
            .param("species", ParamKind::Numbers, "kinds of heron, cycled")
            .param(
                "dress",
                ParamKind::Text,
                "bare, scarf or shawl, cycled, comma separated",
            )
            .param("umbrella", ParamKind::Flag, "carry an open umbrella")
            .param(
                "phase",
                ParamKind::Numbers,
                "points of the steps to walk in place at, cycled",
            )
            .param("seat", ParamKind::Numbers, "height to sit at instead")
            .param(
                "sitting",
                ParamKind::Numbers,
                "how far sat down, from 0 to 1, cycled",
            ),
            Lineup::spawn,
        )
        .add_systems(
            Update,
            Lineup::hold.after(Bird::open_umbrellas).before(Bird::pose),
        );
    }
}

impl Lineup {
    fn spawn(
        In(params): In<Option<Value>>,
        mut commands: Commands,
        figures: Res<Figures>,
        mut plumages: ResMut<Assets<PlumageMaterial>>,
        old: Query<Entity, With<Lineup>>,
    ) -> bevy::remote::BrpResult {
        for entity in &old {
            commands.entity(entity).despawn();
        }
        let params = params.unwrap_or(Value::Null);
        let numbers = |key: &str| -> Vec<f32> {
            match params.get(key) {
                Some(Value::Array(values)) => values
                    .iter()
                    .filter_map(Value::as_f64)
                    .map(|value| value as f32)
                    .collect(),
                Some(value) => value
                    .as_f64()
                    .map(|value| value as f32)
                    .into_iter()
                    .collect(),
                None => Vec::new(),
            }
        };
        let at = numbers("at");
        let [x, z] = [at.first(), at.get(1)];
        let (Some(&x), Some(&z)) = (x, z) else {
            return Ok(json!([]));
        };
        let yaw = numbers("yaw").first().copied().unwrap_or(0.0).to_radians();
        let spacing = numbers("spacing").first().copied().unwrap_or(1.0);
        let mut species: Vec<usize> = numbers("species").iter().map(|&n| n as usize).collect();
        if species.is_empty() {
            species = (0..Coat::SPECIES).collect();
        }
        let dresses: Vec<Dress> = params
            .get("dress")
            .and_then(Value::as_str)
            .unwrap_or("bare")
            .split(',')
            .filter_map(|name| Dress::named(name.trim()))
            .collect();
        let dresses = if dresses.is_empty() {
            vec![Dress::Bare]
        } else {
            dresses
        };
        let carry = if params.get("umbrella").and_then(Value::as_bool) == Some(true) {
            Carry::Always
        } else {
            Carry::Never
        };
        let phases = numbers("phase");
        let seat = numbers("seat").first().copied();
        let sittings = numbers("sitting");
        let mut rng = SmallRng::seed_from_u64(1);
        let count = species
            .len()
            .max(dresses.len())
            .max(phases.len())
            .max(sittings.len());
        let mut placed = Vec::with_capacity(count);
        for index in 0..count {
            let at = Vec3::new(x + index as f32 * spacing, 0.0, z);
            let birth = Birth {
                at: Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw)),
                posture: Posture::Out,
                carry,
                dress: dresses[index % dresses.len()],
                sheltered: false,
                coat: Coat::of(
                    species[index % species.len()],
                    &mut plumages,
                    &figures.bare,
                    &mut rng,
                ),
            };
            let bird = Bird::spawn(&mut commands, &figures, &mut rng, birth);
            commands.entity(bird).insert((
                Name::new("lineup"),
                Self {
                    phase: (!phases.is_empty()).then(|| phases[index % phases.len()]),
                    seat: seat.map(|height| {
                        let sitting = sittings.get(index % sittings.len().max(1));
                        (height, sitting.copied().unwrap_or(1.0))
                    }),
                },
            ));
            placed.push([at.x, at.z]);
        }
        Ok(json!(placed))
    }

    fn hold(mut birds: Query<(&Lineup, &mut Bird)>) {
        for (lineup, mut bird) in &mut birds {
            bird.hold_step(lineup.phase);
            if let Some((height, sitting)) = lineup.seat {
                bird.sit(height, sitting, false);
            }
            bird.open_umbrella_now();
        }
    }
}
