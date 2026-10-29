mod synth;

use std::sync::Arc;

use bevy::{
    audio::{AudioSinkPlayback, SpatialScale, Volume},
    ecs::entity::EntityHashSet,
    platform::collections::HashMap,
    prelude::*,
};

use self::synth::{Noise, Samples};
use crate::parts::keypad::PressableKey;
use crate::player::camera::PlayerCamera;
use crate::settings::Settings;

/// How loud the game is while a menu is open.
const DUCKED: f32 = 0.35;
/// Seconds for the sound to fade into the duck of a menu, and back out.
const DUCK_TIME: f32 = 0.7;
/// Seconds for the sounds of the world to fade out when its time stops, and
/// back in when it goes on.
pub const HUSH: f32 = 0.08;
/// Keys at least this wide in meters, like the space bar, rest on a
/// stabilizer wire and sound a little lower.
const LONG_KEY: f32 = 0.08;
const LONG_KEY_PITCH: f32 = 0.94;
/// Meters between the ears of the player.
const EAR_GAP: f32 = 0.2;
/// So small that rodio never fades a sound with distance. Sounds on the
/// desk are all within reach.
const NEAR: SpatialScale = SpatialScale::new(0.002);
/// Rodio plays a sound straight ahead at three quarters of its volume on
/// both ears.
const CENTER_LOSS: f32 = 0.75;
/// How much each press of a key varies in volume.
const KEY_NUDGE: f32 = 0.15;
/// Sounds that would stand out if they always came the same get this many
/// takes, played in random order.
const TAKES: u32 = 6;

/// Sounds of interactions, all made in code.
pub struct AudioPlugin;

/// A one-shot sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sound {
    /// The desk lamp's switch going on.
    LampOn,
    /// The desk lamp's switch going off.
    LampOff,
    /// A customer paid for a repair.
    Chime,
    /// A coin dropping into the till as it counts up.
    Coin,
    /// The printer having news, before it prints a receipt.
    Chirp,
    /// The rubber stamp hitting the ticket, or a device put down hard.
    Thunk,
    /// A knob snapping into its next position.
    Detent,
    /// A smooth knob's soft ratchet, ticking as it turns.
    Ratchet,
    /// A push button latching down.
    Latch,
    /// A push button springing back up.
    Unlatch,
    /// A radio between two stations.
    Static,
    /// A key switch bottoming out.
    KeyDown,
    /// A key switch springing back up.
    KeyUp,
    /// A page of paper turning over.
    PageTurn,
    /// A divider of card turning over, stiffer and slower than a page.
    DividerTurn,
    /// A page of the manuals file sliding off or onto its rings, quieter
    /// than a turn.
    PageSlide,
    /// The lever of the manuals file opening its rings.
    RingsOpen,
    /// The rings of the manuals file snapping shut.
    RingsShut,
    /// The hinge of the pin rail on the monitor turning.
    Hinge,
    /// Something the player asked for can't be done.
    Refuse,
    /// A screw turned out of a device.
    ScrewOut,
    /// A screw turned into a device.
    ScrewIn,
    /// The debug plug pushed onto a device.
    PlugIn,
    /// The debug plug pulled off a device.
    PlugOut,
    /// The back cover of a device lifting off.
    CoverOff,
    /// The back cover of a device pressed back on.
    CoverOn,
    /// Rain on the road, a loop.
    AsphaltRain,
    /// Rain on the sidewalk, a loop.
    ConcreteRain,
    /// Rain on a puddle, a loop.
    PuddleRain,
    /// Rain on a canvas awning, a loop.
    AwningRain,
    /// Rain on the shop's roof, a loop.
    RoofRain,
    /// A drip off an awning hitting the ground.
    Drip,
    /// A drip off an awning falling into a puddle.
    Plunk,
}

impl Sound {
    /// The sounds played with [`Play`]. The radio plays its static itself,
    /// from where it stands.
    const ALL: [Self; 25] = [
        Self::LampOn,
        Self::LampOff,
        Self::Chime,
        Self::Coin,
        Self::Chirp,
        Self::Thunk,
        Self::Detent,
        Self::Ratchet,
        Self::Latch,
        Self::Unlatch,
        Self::KeyDown,
        Self::KeyUp,
        Self::PageTurn,
        Self::DividerTurn,
        Self::PageSlide,
        Self::RingsOpen,
        Self::RingsShut,
        Self::Hinge,
        Self::Refuse,
        Self::ScrewOut,
        Self::ScrewIn,
        Self::PlugIn,
        Self::PlugOut,
        Self::CoverOff,
        Self::CoverOn,
    ];

    /// The takes of the sound as assets, for playing them in other ways
    /// than [`Play`].
    pub fn sources(self) -> Vec<AudioSource> {
        (0..self.takes())
            .map(|take| AudioSource {
                bytes: Arc::from(self.samples(take).wav()),
            })
            .collect()
    }

    fn takes(self) -> u32 {
        match self {
            Self::LampOn
            | Self::LampOff
            | Self::Coin
            | Self::Detent
            | Self::Ratchet
            | Self::Latch
            | Self::Unlatch
            | Self::Static
            | Self::PageTurn
            | Self::DividerTurn
            | Self::PageSlide
            | Self::RingsOpen
            | Self::RingsShut
            | Self::Hinge
            | Self::Drip
            | Self::Plunk
            | Self::ScrewOut
            | Self::ScrewIn
            | Self::PlugIn
            | Self::PlugOut
            | Self::CoverOff
            | Self::CoverOn => TAKES,
            Self::KeyDown | Self::KeyUp => 2 * TAKES,
            Self::AsphaltRain
            | Self::ConcreteRain
            | Self::PuddleRain
            | Self::AwningRain
            | Self::RoofRain => 2,
            Self::Chime | Self::Chirp | Self::Thunk | Self::Refuse => 1,
        }
    }

    fn samples(self, take: u32) -> Samples {
        match self {
            Self::LampOn => Samples::lamp_switch(take, true),
            Self::LampOff => Samples::lamp_switch(take, false),
            Self::Chime => Samples::chime(),
            Self::Coin => Samples::coin(take),
            Self::Chirp => Samples::chirp(),
            Self::Thunk => Samples::thunk(),
            Self::Detent => Samples::detent(take),
            Self::Ratchet => Samples::ratchet(take),
            Self::Latch => Samples::latch(take),
            Self::Unlatch => Samples::unlatch(take),
            Self::Static => Samples::radio_static(take),
            Self::KeyDown => Samples::key_down(take),
            Self::KeyUp => Samples::key_up(take),
            Self::PageTurn => Samples::page_turn(take),
            Self::DividerTurn => Samples::divider_turn(take),
            Self::PageSlide => Samples::page_turn(take + TAKES),
            Self::RingsOpen => Samples::rings_open(take),
            Self::RingsShut => Samples::rings_shut(take),
            Self::Hinge => Samples::hinge(take),
            Self::Refuse => Samples::refuse(),
            Self::ScrewOut => Samples::screw_out(take),
            Self::ScrewIn => Samples::screw_in(take),
            Self::PlugIn => Samples::plug_in(take),
            Self::PlugOut => Samples::plug_out(take),
            Self::CoverOff => Samples::cover_off(take),
            Self::CoverOn => Samples::cover_on(take),
            Self::AsphaltRain => Samples::asphalt_rain(take),
            Self::ConcreteRain => Samples::concrete_rain(take),
            Self::PuddleRain => Samples::puddle_rain(take),
            Self::AwningRain => Samples::awning_rain(take),
            Self::RoofRain => Samples::roof_rain(take),
            Self::Drip => Samples::drip(take),
            Self::Plunk => Samples::plunk(take),
        }
    }

    pub fn volume(self) -> f32 {
        match self {
            Self::LampOn | Self::LampOff => 0.22,
            Self::Chime => 0.4,
            Self::Coin => 0.25,
            Self::Chirp => 0.3,
            Self::Thunk => 0.6,
            Self::Detent => 0.5,
            Self::Ratchet => 0.12,
            Self::Latch | Self::Unlatch => 0.5,
            Self::Static => 0.4,
            Self::KeyDown => 0.17,
            Self::KeyUp => 0.1,
            Self::PageTurn => 0.29,
            Self::DividerTurn => 0.35,
            Self::PageSlide => 0.18,
            Self::RingsOpen | Self::RingsShut => 0.24,
            Self::Hinge => 0.12,
            Self::Refuse => 0.12,
            Self::ScrewOut | Self::ScrewIn => 0.7,
            Self::PlugIn | Self::PlugOut => 0.35,
            Self::CoverOff | Self::CoverOn => 0.55,
            Self::AsphaltRain | Self::ConcreteRain => 0.125,
            Self::PuddleRain => 0.35,
            Self::AwningRain => 0.2,
            Self::RoofRain => 0.125,
            Self::Drip | Self::Plunk => 0.25,
        }
    }
}

/// How loud the game is overall.
#[derive(Resource, Debug)]
pub struct Mixer {
    /// From 0 to 1.
    pub volume: f32,
    /// Quieter while a menu is open.
    ducked: bool,
    /// How far the sound has faded into the duck, from 0 to 1.
    duck: f32,
    /// How much of the world is heard, from 0 while its time stands still
    /// to 1.
    world: f32,
}

/// Plays a [`Sound`].
#[derive(Event, Debug)]
pub struct Play(pub Sound);

/// Plays a [`Sound`] from a point in the world, so the player hears where
/// it comes from.
#[derive(Event, Debug)]
pub struct PlayAt(pub Sound, pub Vec3);

impl Default for Mixer {
    fn default() -> Self {
        Self {
            volume: 1.0,
            // The game starts on the title screen.
            ducked: true,
            duck: 1.0,
            world: 1.0,
        }
    }
}

impl Mixer {
    fn follow_settings(settings: Res<Settings>, mut mixer: ResMut<Mixer>) {
        mixer.volume = settings.volume();
    }

    /// What every sound is multiplied by.
    pub fn gain(&self) -> f32 {
        self.volume * Self::ducking(self.duck)
    }

    /// The factor of a duck `duck` of the way in. It fades evenly in
    /// decibels.
    fn ducking(duck: f32) -> f32 {
        DUCKED.powf(EaseFunction::SmoothStep.sample_clamped(duck))
    }

    pub fn duck<const ON: bool>(mut mixer: ResMut<Mixer>) {
        mixer.ducked = ON;
    }

    /// Plays a sound of the world at `volume` while its time runs, and
    /// holds it where it is while time stands still.
    pub fn hold(&self, sink: &mut impl AudioSinkPlayback, volume: f32) {
        sink.set_volume(Volume::Linear(volume * self.world));
        if self.world > 0.0 {
            sink.play();
        } else {
            sink.pause();
        }
    }

    /// Fades the world out while its time stands still.
    fn follow_time(time: Res<Time<Real>>, world: Res<Time<Virtual>>, mut mixer: ResMut<Mixer>) {
        let target = if world.is_paused() { 0.0 } else { 1.0 };
        if mixer.world != target {
            let step = time.delta_secs() / HUSH;
            mixer.world += (target - mixer.world).clamp(-step, step);
        }
    }

    fn follow_duck(time: Res<Time<Real>>, mut mixer: ResMut<Mixer>) {
        let target = if mixer.ducked { 1.0 } else { 0.0 };
        if mixer.duck != target {
            let step = time.delta_secs() / DUCK_TIME;
            mixer.duck += (target - mixer.duck).clamp(-step, step);
        }
    }
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Mixer>()
            .add_systems(Startup, Sounds::load)
            .add_systems(PreUpdate, (Mixer::follow_time, Mixer::follow_duck))
            .add_systems(
                Update,
                (
                    Sounds::click_keys.run_if(resource_exists::<Sounds>),
                    Mixer::follow_settings.run_if(resource_changed::<Settings>),
                    Held::follow.run_if(resource_changed::<Mixer>),
                ),
            )
            .add_observer(Sounds::play)
            .add_observer(Sounds::play_at)
            .add_observer(listen);
    }
}

#[derive(Resource)]
struct Sounds(HashMap<Sound, Takes<Handle<AudioSource>>>);

/// A one-shot that started while the world's time ran, at this volume. It
/// holds still with it.
#[derive(Component, Debug)]
struct Held(f32);

/// The takes of a sound, picked at random.
pub struct Takes<T> {
    takes: Vec<T>,
    dice: Noise,
    /// The take picked last.
    last: Option<usize>,
}

impl Sounds {
    fn load(mut sources: ResMut<Assets<AudioSource>>, mut commands: Commands) {
        let sounds = Sound::ALL
            .into_iter()
            .enumerate()
            .map(|(index, sound)| {
                let handles = sound
                    .sources()
                    .into_iter()
                    .map(|source| sources.add(source));
                (sound, Takes::new(handles.collect(), index as u32))
            })
            .collect();
        commands.insert_resource(Self(sounds));
    }

    fn play(
        play: On<Play>,
        sounds: Option<ResMut<Sounds>>,
        mixer: Res<Mixer>,
        mut commands: Commands,
    ) {
        if let Some(mut sounds) = sounds
            && let Some(source) = sounds.0.get_mut(&play.0).map(|takes| takes.next().clone())
        {
            let volume = play.0.volume() * mixer.gain();
            let mut sound = commands.spawn((
                AudioPlayer(source),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
            ));
            Held::start(&mut sound, &mixer, volume);
        }
    }

    fn play_at(
        play: On<PlayAt>,
        sounds: Option<ResMut<Sounds>>,
        mixer: Res<Mixer>,
        mut commands: Commands,
    ) {
        let PlayAt(sound, at) = *play;
        if let Some(mut sounds) = sounds
            && let Some(source) = sounds.0.get_mut(&sound).map(|takes| takes.next().clone())
        {
            Self::play_from(
                &mut commands,
                &mixer,
                source,
                at,
                PlaybackSettings::DESPAWN
                    .with_volume(Volume::Linear(sound.volume() * mixer.gain())),
            );
        }
    }

    /// Plays `source` from `at`, as loud as with `settings` alone when
    /// straight ahead.
    fn play_from(
        commands: &mut Commands,
        mixer: &Mixer,
        source: Handle<AudioSource>,
        at: Vec3,
        settings: PlaybackSettings,
    ) {
        let volume = settings.volume.to_linear() / CENTER_LOSS;
        let mut sound = commands.spawn((
            AudioPlayer(source),
            settings
                .with_spatial(true)
                .with_spatial_scale(NEAR)
                .with_volume(Volume::Linear(volume)),
            Transform::from_translation(at),
        ));
        Held::start(&mut sound, mixer, volume);
    }

    /// Keys sound from where they are as they go down and as they come
    /// back up.
    fn click_keys(
        mut sounds: ResMut<Sounds>,
        mixer: Res<Mixer>,
        keys: Query<(Entity, &PressableKey, &GlobalTransform), Changed<PressableKey>>,
        mut down: Local<EntityHashSet>,
        mut commands: Commands,
    ) {
        for (entity, key, transform) in &keys {
            let sound = if key.is_pressed() {
                if !down.insert(entity) {
                    continue;
                }
                Sound::KeyDown
            } else {
                if !down.remove(&entity) {
                    continue;
                }
                Sound::KeyUp
            };
            let Some(takes) = sounds.0.get_mut(&sound) else {
                continue;
            };
            let pitch = if key.width() < LONG_KEY {
                1.0
            } else {
                LONG_KEY_PITCH
            };
            let volume = sound.volume() * mixer.gain() * takes.nudge(KEY_NUDGE);
            Self::play_from(
                &mut commands,
                &mixer,
                takes.next().clone(),
                transform.translation(),
                PlaybackSettings::DESPAWN
                    .with_volume(Volume::Linear(volume))
                    .with_speed(pitch),
            );
        }
    }
}

impl Held {
    /// Holds `sound` at `volume` with the world, if the world's time runs.
    /// Sounds that start while it stands still, like those of a menu, play
    /// on.
    fn start(sound: &mut EntityCommands, mixer: &Mixer, volume: f32) {
        if mixer.world > 0.0 {
            sound.insert(Self(volume));
        }
    }

    fn follow(
        mixer: Res<Mixer>,
        mut sinks: Query<(&Held, &mut AudioSink)>,
        mut spatial: Query<(&Held, &mut SpatialAudioSink)>,
    ) {
        for (held, mut sink) in &mut sinks {
            mixer.hold(&mut *sink, held.0);
        }
        for (held, mut sink) in &mut spatial {
            mixer.hold(&mut *sink, held.0);
        }
    }
}

impl<T> Takes<T> {
    /// Picks among `takes` in an order made from `seed`.
    pub fn new(takes: Vec<T>, seed: u32) -> Self {
        Self {
            takes,
            dice: Noise::new(seed),
            last: None,
        }
    }

    /// A factor from `1 - amount` to `1 + amount`, for playing a take a
    /// little differently each time.
    fn nudge(&mut self, amount: f32) -> f32 {
        1.0 + self.dice.next() * amount
    }

    /// One of the takes, never the same one twice in a row unless there is
    /// only one.
    #[expect(
        clippy::should_implement_trait,
        reason = "the takes never run out and hand out references"
    )]
    pub fn next(&mut self) -> &T {
        let count = self.takes.len();
        let roll = (self.dice.next() + 1.0) / 2.0;
        let mut pick = ((roll * count as f32) as usize).min(count.saturating_sub(1));
        if count > 1 && Some(pick) == self.last {
            pick = (pick + 1) % count;
        }
        self.last = Some(pick);
        &self.takes[pick]
    }
}

/// The player hears the world from where the camera is.
///
/// Rodio makes a sound louder on the ear farther from it, so the ears
/// swap places to put sounds on the right side.
fn listen(add: On<Add<PlayerCamera>>, mut commands: Commands) {
    let SpatialListener {
        left_ear_offset,
        right_ear_offset,
    } = SpatialListener::new(EAR_GAP);
    commands.entity(add.entity).insert(SpatialListener {
        left_ear_offset: right_ear_offset,
        right_ear_offset: left_ear_offset,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_duck_fades_evenly_between_full_and_ducked() {
        assert_eq!(Mixer::ducking(0.0), 1.0);
        assert!((Mixer::ducking(1.0) - DUCKED).abs() < 1e-6);
        let steps: Vec<f32> = (0..=10).map(|i| Mixer::ducking(i as f32 / 10.0)).collect();
        assert!(steps.windows(2).all(|pair| pair[1] < pair[0]), "{steps:?}");
    }
}
