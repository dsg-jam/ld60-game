mod synth;

use std::sync::Arc;

use bevy::{audio::Volume, ecs::entity::EntityHashSet, prelude::*};

use self::synth::Samples;
use crate::parts::keypad::PressableKey;

const CLICK_VOLUME: f32 = 0.35;
/// Keys differ a little in pitch, so typing doesn't sound like a loop.
const CLICK_PITCHES: [f32; 5] = [0.92, 0.97, 1.0, 1.04, 1.08];

/// Sounds of interactions, all made in code.
pub(crate) struct AudioPlugin;

/// Whether pressing keys makes a sound.
#[derive(Resource, Debug)]
pub(crate) struct KeyClicks(pub(crate) bool);

impl Default for KeyClicks {
    fn default() -> Self {
        Self(true)
    }
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyClicks>()
            .add_systems(Startup, Sounds::load)
            .add_systems(Update, Sounds::click_keys.run_if(resource_exists::<Sounds>));
    }
}

#[derive(Resource)]
struct Sounds {
    click: Handle<AudioSource>,
}

impl Sounds {
    fn load(mut sources: ResMut<Assets<AudioSource>>, mut commands: Commands) {
        commands.insert_resource(Self {
            click: sources.add(AudioSource {
                bytes: Arc::from(Samples::click().wav()),
            }),
        });
    }

    fn click_keys(
        sounds: Res<Sounds>,
        enabled: Res<KeyClicks>,
        keys: Query<(Entity, &PressableKey), Changed<PressableKey>>,
        mut down: Local<EntityHashSet>,
        mut commands: Commands,
    ) {
        for (entity, key) in &keys {
            if !key.is_pressed() {
                down.remove(&entity);
            } else if down.insert(entity) && enabled.0 {
                let pitch = CLICK_PITCHES[entity.to_bits() as usize % CLICK_PITCHES.len()];
                commands.spawn((
                    AudioPlayer(sounds.click.clone()),
                    PlaybackSettings::DESPAWN
                        .with_volume(Volume::Linear(CLICK_VOLUME))
                        .with_speed(pitch),
                ));
            }
        }
    }
}
