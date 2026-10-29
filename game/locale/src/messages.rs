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
        load_context: &mut LoadContext<'_>,
    ) -> Result<Messages, BevyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        // Fluent skips the entries it cannot parse and keeps the rest.
        let resource = FluentResource::try_new(String::from_utf8(bytes)?).unwrap_or_else(
            |(resource, errors)| {
                for error in errors {
                    let before = &resource.source().as_bytes()[..error.pos.start];
                    let line = before.iter().filter(|&&byte| byte == b'\n').count() + 1;
                    warn!("{}:{line}: {error}", load_context.path());
                }
                resource
            },
        );
        Ok(Messages(Arc::new(resource)))
    }

    fn extensions(&self) -> &[&str] {
        &["ftl"]
    }
}
