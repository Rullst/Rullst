use super::{CloudError, ObjectMetadata};
use std::{collections::BTreeMap, sync::Mutex};
#[cfg(feature = "storage-multipart")]
mod multipart;

const MAX_OBJECTS: usize = 256;
const MAX_BYTES: usize = 64 * 1024 * 1024;

pub(super) struct MockStore(
    Mutex<State>,
    #[cfg(feature = "storage-multipart")] uuid::Uuid,
);

impl Default for MockStore {
    fn default() -> Self {
        Self(
            Mutex::new(State::default()),
            #[cfg(feature = "storage-multipart")]
            uuid::Uuid::new_v4(),
        )
    }
}

#[derive(Default)]
struct State {
    objects: BTreeMap<String, Vec<u8>>,
    total: usize,
    #[cfg(feature = "storage-multipart")]
    uploads: BTreeMap<String, multipart::Upload>,
    #[cfg(feature = "storage-multipart")]
    markers: BTreeMap<String, String>,
    #[cfg(feature = "storage-multipart")]
    next_upload: u64,
}

impl MockStore {
    #[cfg(feature = "storage-multipart")]
    pub(super) fn incarnation(&self) -> uuid::Uuid {
        self.1
    }
    pub(super) fn put(&self, key: &str, bytes: &[u8]) -> Result<(), CloudError> {
        let mut state = self.0.lock().map_err(|_| CloudError::MockUnavailable)?;
        let old_len = state.objects.get(key).map_or(0, Vec::len);
        let total = state
            .total
            .checked_sub(old_len)
            .and_then(|n| n.checked_add(bytes.len()))
            .ok_or(CloudError::MockUnavailable)?;
        let entries = state.objects.len();
        #[cfg(feature = "storage-multipart")]
        let entries = entries + state.uploads.len();
        if total > MAX_BYTES || (!state.objects.contains_key(key) && entries >= MAX_OBJECTS) {
            return Err(CloudError::MockUnavailable);
        }
        state.objects.insert(key.to_string(), bytes.to_vec());
        #[cfg(feature = "storage-multipart")]
        state.markers.remove(key);
        state.total = total;
        Ok(())
    }

    pub(super) fn get(&self, key: &str, limit: usize) -> Result<Vec<u8>, CloudError> {
        let state = self.0.lock().map_err(|_| CloudError::MockUnavailable)?;
        let bytes = state.objects.get(key).ok_or(CloudError::NotFound)?;
        if bytes.len() > limit {
            return Err(CloudError::SizeLimit);
        }
        Ok(bytes.clone())
    }

    pub(super) fn metadata(&self, key: &str) -> Result<ObjectMetadata, CloudError> {
        let state = self.0.lock().map_err(|_| CloudError::MockUnavailable)?;
        let value = state.objects.get(key).ok_or(CloudError::NotFound)?;
        Ok(ObjectMetadata {
            size_bytes: value.len() as u64,
            etag: None,
        })
    }

    pub(super) fn delete(&self, key: &str) -> Result<(), CloudError> {
        let mut state = self.0.lock().map_err(|_| CloudError::MockUnavailable)?;
        if let Some(value) = state.objects.remove(key) {
            state.total -= value.len();
        }
        #[cfg(feature = "storage-multipart")]
        state.markers.remove(key);
        Ok(())
    }
}
