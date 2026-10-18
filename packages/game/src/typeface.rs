use bevy::{asset::AssetId, prelude::*};

/// The full Fira Mono Medium. Bevy's own copy only has the letters of
/// English.
const DEFAULT_FONT: &str = "fonts/fira-mono-medium.ttf";

pub(crate) struct TypefacePlugin;

impl Plugin for TypefacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DefaultFont>()
            .add_systems(Update, DefaultFont::replace);
    }
}

/// The font all text uses unless it names another.
#[derive(Resource, Debug)]
struct DefaultFont(Handle<Font>);

impl FromWorld for DefaultFont {
    fn from_world(world: &mut World) -> Self {
        Self(world.resource::<AssetServer>().load(DEFAULT_FONT))
    }
}

impl DefaultFont {
    /// Puts the font in place of Bevy's default once it is loaded.
    fn replace(
        mut events: MessageReader<AssetEvent<Font>>,
        font: Res<DefaultFont>,
        mut fonts: ResMut<Assets<Font>>,
    ) {
        let loaded = events
            .read()
            .any(|event| event.is_loaded_with_dependencies(&font.0) || event.is_modified(&font.0));
        if loaded && let Some(full) = fonts.get(&font.0).cloned() {
            fonts.insert(AssetId::default(), full).unwrap();
        }
    }
}
