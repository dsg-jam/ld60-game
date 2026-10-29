use bevy::{asset::AssetId, prelude::*};

/// Takes the place of Bevy's built-in default font, which only has the
/// letters of English. It is part of the binary, so text shows from the first
/// frame, even on the loading screen.
const DEFAULT_FONT: &[u8] =
    include_bytes!(concat!(env!("LD60_ASSETS"), "/fonts/fira-mono-medium.ttf"));

pub struct TypefacePlugin;

impl Plugin for TypefacePlugin {
    fn build(&self, app: &mut App) {
        let font = Font::from_bytes(DEFAULT_FONT.to_vec());
        app.world_mut()
            .resource_mut::<Assets<Font>>()
            .insert(AssetId::default(), font)
            .unwrap();
    }
}
