use super::{Store, StoreError};

/// Keeps the saves of this game apart from other pages on the same site.
const PREFIX: &str = "ld60-";

impl Store {
    fn local() -> Result<web_sys::Storage, StoreError> {
        web_sys::window()
            .and_then(|window| window.local_storage().ok().flatten())
            .ok_or(StoreError::NoStorage)
    }

    pub(super) fn read(&self, key: &str) -> Result<Option<String>, StoreError> {
        Self::local()?
            .get_item(&format!("{PREFIX}{key}"))
            .map_err(|_| StoreError::Refused)
    }

    pub(super) fn write(&self, key: &str, text: &str) -> Result<(), StoreError> {
        Self::local()?
            .set_item(&format!("{PREFIX}{key}"), text)
            .map_err(|_| StoreError::Refused)
    }
}
