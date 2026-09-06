use std::{collections::HashMap, future::Future, hash::Hash, sync::Arc};
use tokio::sync::Mutex;

/// Game reads never expire or retry implicitly, including failed initialization.
/// The lock coalesces concurrent first reads before any provider I/O begins.
pub(super) struct Cache<K, T, E>(Mutex<HashMap<K, Arc<Entry<T, E>>>>);

struct Entry<T, E>(Mutex<Option<Result<T, E>>>);

impl<K, T, E> Default for Cache<K, T, E> {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

impl<K: Eq + Hash, T: Clone, E: Clone> Cache<K, T, E> {
    pub(super) async fn map_error(&self, key: &K, update: impl FnOnce(E) -> E) {
        let entry = self.0.lock().await.get(key).cloned();
        if let Some(entry) = entry {
            let mut cached = entry.0.lock().await;
            if let Some(Err(error)) = &*cached {
                *cached = Some(Err(update(error.clone())));
            }
        }
    }

    pub(super) async fn read(
        &self,
        key: K,
        refresh: bool,
        fetch: impl Future<Output = Result<T, E>>,
    ) -> Result<T, E> {
        let entry = {
            let mut entries = self.0.lock().await;
            entries
                .entry(key)
                .or_insert_with(|| Arc::new(Entry(Mutex::new(None))))
                .clone()
        };
        let mut cached = entry.0.lock().await;
        if !refresh && let Some(result) = &*cached {
            return result.clone();
        }
        let result = fetch.await;
        *cached = Some(result.clone());
        result
    }
}

#[cfg(test)]
#[path = "../tests/unit/cache.rs"]
mod tests;
