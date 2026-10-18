use std::sync::Arc;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};
use fluent_bundle::FluentResource;

/// The messages of one Fluent file.
#[derive(Asset, TypePath, Debug, Clone)]
pub(super) struct Messages(pub(super) Arc<FluentResource>);

#[derive(Default, TypePath)]
pub(super) struct MessagesLoader;

impl AssetLoader for MessagesLoader {
    type Asset = Messages;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Messages, BevyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        match FluentResource::try_new(String::from_utf8(bytes)?) {
            Ok(resource) => Ok(Messages(Arc::new(resource))),
            Err((_, mut errors)) => Err(errors.swap_remove(0).into()),
        }
    }

    fn extensions(&self) -> &[&str] {
        &["ftl"]
    }
}
