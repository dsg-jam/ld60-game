use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};
use ld60_devices::{ChipSpec, DeviceKind};
use ld60_levels::{Catalog, GenerateError, Level, LevelSource, Order, Reference};
use ld60_locale::Message;

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
pub(crate) struct Levels {
    #[deref]
    catalog: Catalog,
    /// The [`Reference::source`] of each device, in the order of
    /// [`DeviceKind::ALL`].
    firmware: Vec<String>,
}

/// The level folders in play order, each with its loaded files.
#[derive(Asset, TypePath, Debug)]
struct CatalogFile {
    order: Order,
    levels: Vec<(String, Handle<LevelFile>)>,
    /// The [`Reference::source`] of each device, in the order of
    /// [`DeviceKind::ALL`].
    firmware: Vec<String>,
}

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
        let (id, attribute) = level.message(&attribute.into());
        let message = Message::new(id).attribute(attribute);
        level
            .text_args()
            .into_iter()
            .fold(message, |message, (name, value)| message.with(name, value))
    }

    /// The working firmware of `device`, compiled for `chip`.
    pub(crate) fn reference(
        &self,
        device: DeviceKind,
        chip: ChipSpec,
    ) -> Result<Reference, GenerateError> {
        let index = DeviceKind::ALL
            .iter()
            .position(|&kind| kind == device)
            .unwrap_or_default();
        Reference::compile(device, chip, &self.firmware[index])
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
            .levels
            .iter()
            .map(|(path, handle)| Some(files.get(handle)?.source(path.clone())))
            .collect();
        if let Some(levels) = levels {
            commands.insert_resource(Self {
                catalog: Catalog::new(catalog.order.clone(), levels),
                firmware: catalog.firmware.clone(),
            });
        }
    }
}

#[cfg(test)]
impl Levels {
    const DIR: &str = concat!(env!("LD60_ASSETS"), "/levels");

    /// The levels in the repository, for tests.
    pub(crate) fn on_disk() -> Self {
        Self::new(Catalog::read(std::path::Path::new(Self::DIR)).unwrap())
    }

    /// The order of the catalog in the repository.
    pub(crate) fn order_on_disk() -> Order {
        let file = format!("{}/{}", Self::DIR, Catalog::FILE);
        Catalog::order(&std::fs::read_to_string(file).unwrap()).unwrap()
    }

    /// `catalog` with the firmware in the repository.
    pub(crate) fn new(catalog: Catalog) -> Self {
        let firmware = DeviceKind::ALL
            .iter()
            .map(|&device| {
                let file = format!("{}/{}", Self::DIR, Reference::source(device));
                std::fs::read_to_string(file).unwrap()
            })
            .collect();
        Self { catalog, firmware }
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
        let order = Catalog::order(&String::from_utf8(bytes)?)?;
        let levels = order
            .paths()
            .map(|path| {
                let file = LevelSource::file(path, LevelSource::LEVEL_FILE);
                let file = load_context.path().resolve_embed_str(&file)?;
                Ok((path.to_owned(), load_context.load(file)))
            })
            .collect::<Result<_, BevyError>>()?;
        let mut firmware = Vec::new();
        for device in DeviceKind::ALL {
            let file = load_context
                .path()
                .resolve_embed_str(Reference::source(device))?;
            firmware.push(String::from_utf8(
                load_context.read_asset_bytes(file).await?,
            )?);
        }
        Ok(CatalogFile {
            order,
            levels,
            firmware,
        })
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
    use ld60_devices::{BoardRev, DeviceKind};
    use ld60_levels::{Difficulty, Generated, JobText, Reference, Symptom, TestCase, Variant};
    use ld60_locale::Locale;

    use super::*;

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

    /// A generated job with every remark a generated job can have.
    fn generated() -> Level {
        let source = std::fs::read_to_string(format!(
            "{}/levels/{}",
            env!("LD60_ASSETS"),
            Reference::source(DeviceKind::Summa6)
        ))
        .unwrap();
        let difficulty = Difficulty::for_job(0, DeviceKind::Summa6);
        let reference = Reference::compile(DeviceKind::Summa6, difficulty.chip(), &source).unwrap();
        let mut level = Generated::new(&reference, 0, &difficulty)
            .unwrap()
            .variant
            .level;
        level.remarks = Symptom::ALL
            .into_iter()
            .map(JobText::also)
            .chain([JobText::FAULT_REMARK.into()])
            .collect();
        level
    }

    /// `level` with `text` about a first test that presses `keys`.
    fn told(level: &Level, text: JobText, keys: &str) -> Level {
        let mut level = level.clone();
        level.tests[0] = TestCase {
            keys: keys.into(),
            display: "000046".into(),
        };
        level.text = Some(text);
        level
    }

    #[test]
    fn every_generated_job_has_its_text() {
        let base = generated();
        let mut texts = Vec::new();
        for customer in JobText::CUSTOMERS {
            for symptom in Symptom::ALL {
                for take in 1..=JobText::TAKES {
                    let text = JobText {
                        customer: customer.into(),
                        symptom,
                        take,
                    };
                    let woke = symptom.shows_on_wake().then_some("");
                    for keys in ["12+34="].into_iter().chain(woke) {
                        let level = told(&base, text.clone(), keys);
                        let remarks = level.remarks.iter().map(|key| Level::remark(key));
                        for attribute in [Level::CUSTOMER.into(), Level::COMPLAINT.into()]
                            .into_iter()
                            .chain(remarks)
                        {
                            texts.push(Levels::text(&level, attribute));
                        }
                    }
                }
            }
        }
        Locale::on_disk().assert_formats(texts);
    }

    #[test]
    fn generated_complaints_name_the_keys() {
        let text = JobText {
            customer: JobText::CUSTOMERS[0].into(),
            symptom: Symptom::NoResponse,
            take: 1,
        };
        let level = told(&generated(), text, "12+34=");
        let complaint = Levels::text(&level, Level::COMPLAINT);
        let complaint = Locale::on_disk().text(&complaint.into());
        assert!(complaint.contains("12+34="), "{complaint}");
        assert!(complaint.contains("the = button"), "{complaint}");
        assert!(complaint.contains("000046"), "{complaint}");
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
