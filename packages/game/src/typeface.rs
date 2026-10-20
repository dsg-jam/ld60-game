use bevy::{asset::AssetId, prelude::*};

/// Takes the place of Bevy's built-in default font, which only has the
/// letters of English.
const DEFAULT_FONT: &str = "fonts/fira-mono-medium.ttf";

pub(crate) struct TypefacePlugin;

impl Plugin for TypefacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DefaultFont>()
            .add_systems(Update, DefaultFont::install);
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
    /// Fills the empty default font slot once the font is loaded. Bevy only
    /// registers a font the first time its slot is filled, and text without
    /// a font waits until then.
    fn install(font: Res<DefaultFont>, mut fonts: ResMut<Assets<Font>>) {
        if fonts.contains(AssetId::default()) {
            return;
        }
        if let Some(full) = fonts.get(&font.0).cloned() {
            fonts.insert(AssetId::default(), full).unwrap();
        }
    }
}
