use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};
use ld60_levels::{Catalog, Level, LevelSource};

use crate::locale::Message;

const CATALOG: &str = "levels/catalog.ron";

pub(crate) struct LevelsPlugin;

impl Plugin for LevelsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<CatalogFile>()
            .init_asset::<LevelFile>()
            .init_asset_loader::<CatalogLoader>()
            .init_asset_loader::<LevelLoader>()
            .init_resource::<CatalogHandle>()
            .add_systems(Update, Levels::follow_files);
    }
}

/// Every level, once the catalog and the level folders it lists are loaded.
#[derive(Resource, Debug, Deref)]
pub(crate) struct Levels(Catalog);

/// The level folders in play order, each with its loaded files.
#[derive(Asset, TypePath, Debug)]
struct CatalogFile(Vec<(String, Handle<LevelFile>)>);

/// The files of one level folder.
#[derive(Asset, TypePath, Debug)]
struct LevelFile {
    level: String,
    buggy: String,
    fixed: String,
}

#[derive(Resource, Debug)]
struct CatalogHandle(Handle<CatalogFile>);

/// Loads the catalog and starts loading every level it lists.
#[derive(Default, TypePath)]
struct CatalogLoader;

/// Loads a level file and the firmware files next to it.
#[derive(Default, TypePath)]
struct LevelLoader;

impl Levels {
    /// The `attribute` of the message of `level`, in the player's language.
    pub(crate) fn text(level: &Level, attribute: impl Into<String>) -> Message {
        let message = Message::new(level.id.clone()).attribute(attribute.into());
        level
            .text_args()
            .into_iter()
            .fold(message, |message, (name, value)| message.with(name, value))
    }

    /// Takes the levels once the catalog and all its levels are loaded, and
    /// again when a file changes.
    fn follow_files(
        mut catalog_events: MessageReader<AssetEvent<CatalogFile>>,
        mut level_events: MessageReader<AssetEvent<LevelFile>>,
        catalogs: Res<Assets<CatalogFile>>,
        files: Res<Assets<LevelFile>>,
        handle: Res<CatalogHandle>,
        mut commands: Commands,
    ) {
        let updates = catalog_events.read().filter(|e| is_update(e)).count()
            + level_events.read().filter(|e| is_update(e)).count();
        if updates == 0 {
            return;
        }
        let Some(catalog) = catalogs.get(&handle.0) else {
            return;
        };
        let levels = catalog
            .0
            .iter()
            .map(|(path, handle)| Some(files.get(handle)?.source(path.clone())))
            .collect();
        if let Some(levels) = levels {
            commands.insert_resource(Self(Catalog::new(levels)));
        }
    }
}

#[cfg(test)]
impl Levels {
    /// The levels in the repository, for tests.
    pub(crate) fn on_disk() -> Self {
        Self(Catalog::on_disk().unwrap())
    }

    pub(crate) fn new(catalog: Catalog) -> Self {
        Self(catalog)
    }
}

impl LevelFile {
    fn source(&self, path: String) -> LevelSource {
        LevelSource {
            path,
            level: self.level.clone(),
            buggy: self.buggy.clone(),
            fixed: self.fixed.clone(),
        }
    }
}

impl FromWorld for CatalogHandle {
    fn from_world(world: &mut World) -> Self {
        Self(world.resource::<AssetServer>().load(CATALOG))
    }
}

impl AssetLoader for CatalogLoader {
    type Asset = CatalogFile;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<CatalogFile, BevyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let levels = Catalog::order(&String::from_utf8(bytes)?)?
            .into_iter()
            .map(|path| {
                let file = LevelSource::file(&path, LevelSource::LEVEL_FILE);
                let file = load_context.path().resolve_embed_str(&file)?;
                Ok((path, load_context.load(file)))
            })
            .collect::<Result<_, BevyError>>()?;
        Ok(CatalogFile(levels))
    }

    fn extensions(&self) -> &[&str] {
        &[]
    }
}

impl AssetLoader for LevelLoader {
    type Asset = LevelFile;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<LevelFile, BevyError> {
        let mut level = Vec::new();
        reader.read_to_end(&mut level).await?;
        let mut read = async |file: &str| -> Result<String, BevyError> {
            let file = load_context.path().resolve_embed_str(file)?;
            Ok(String::from_utf8(
                load_context.read_asset_bytes(file).await?,
            )?)
        };
        Ok(LevelFile {
            buggy: read(LevelSource::BUGGY_FILE).await?,
            fixed: read(LevelSource::FIXED_FILE).await?,
            level: String::from_utf8(level)?,
        })
    }

    fn extensions(&self) -> &[&str] {
        &[]
    }
}

fn is_update<A: Asset>(event: &AssetEvent<A>) -> bool {
    matches!(
        event,
        AssetEvent::Added { .. } | AssetEvent::Modified { .. }
    )
}

#[cfg(test)]
mod tests {
    use ld60_devices::BoardRev;
    use ld60_levels::Variant;

    use super::*;
    use crate::locale::Locale;

    #[test]
    fn every_level_has_its_text() {
        let locale = Locale::on_disk();
        for source in Levels::on_disk().levels() {
            let level = source.load().unwrap();
            let remarks = level.remarks.iter().map(|key| Level::remark(key));
            for attribute in [Level::CUSTOMER.into(), Level::COMPLAINT.into()]
                .into_iter()
                .chain(remarks)
            {
                let text = Levels::text(&level, attribute);
                assert!(locale.has(&text), "{text:?}");
            }
        }
    }

    #[test]
    fn remarks_name_units_on_the_board_of_the_job() {
        let levels = Levels::on_disk();
        let source = levels.find("summa6/05-errata").unwrap();
        let level = Variant::with_board(source, 1, BoardRev::B).unwrap().level;
        let remark = Levels::text(&level, Level::remark("twin"));
        let text = Locale::on_disk().text(&remark.into());
        assert!(text.contains("6B-05530"), "{text}");
    }
}
