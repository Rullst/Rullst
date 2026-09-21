use super::{CloudError, ObjectMetadata};
use std::{collections::BTreeMap, sync::Mutex};

const MAX_OBJECTS: usize = 256;
const MAX_BYTES: usize = 64 * 1024 * 1024;

#[derive(Default)]
pub(super) struct MockStore(Mutex<State>);

#[derive(Default)]
struct State {
    objects: BTreeMap<String, Vec<u8>>,
    total: usize,
}

impl MockStore {
    pub(super) fn put(&self, key: &str, bytes: &[u8]) -> Result<(), CloudError> {
        let mut state = self.0.lock().map_err(|_| CloudError::MockUnavailable)?;
        let old_len = state.objects.get(key).map_or(0, Vec::len);
        let total = state
            .total
            .checked_sub(old_len)
            .and_then(|n| n.checked_add(bytes.len()))
            .ok_or(CloudError::MockUnavailable)?;
        if total > MAX_BYTES
            || (!state.objects.contains_key(key) && state.objects.len() >= MAX_OBJECTS)
        {
            return Err(CloudError::MockUnavailable);
        }
        state.objects.insert(key.to_string(), bytes.to_vec());
        state.total = total;
        Ok(())
    }

    pub(super) fn get(&self, key: &str) -> Result<Vec<u8>, CloudError> {
        self.0
            .lock()
            .map_err(|_| CloudError::MockUnavailable)?
            .objects
            .get(key)
            .cloned()
            .ok_or(CloudError::NotFound)
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
        Ok(())
    }
}
