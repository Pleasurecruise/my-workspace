use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// A bounded in-memory TTL cache with request coalescing. Failed reads are
/// cached alongside successful ones so a temporarily broken provider is not
/// hammered by the Dashboard polling loop.
pub(crate) struct Cache<T> {
    state: Mutex<Option<(Instant, T)>>,
    inflight: Mutex<()>,
}

impl<T> Cache<T> {
    pub(crate) const fn new() -> Self {
        Self {
            state: Mutex::const_new(None),
            inflight: Mutex::const_new(()),
        }
    }

    pub(crate) async fn read(&self, ttl: Duration, fetch: impl std::future::Future<Output = T>) -> T
    where
        T: Clone,
    {
        if let Some(value) = self.fresh(ttl).await {
            return value;
        }
        let _gate = self.inflight.lock().await;
        if let Some(value) = self.fresh(ttl).await {
            return value;
        }
        let value = fetch.await;
        *self.state.lock().await = Some((Instant::now(), value.clone()));
        value
    }

    async fn fresh(&self, ttl: Duration) -> Option<T>
    where
        T: Clone,
    {
        let state = self.state.lock().await;
        state
            .as_ref()
            .filter(|(loaded_at, _)| loaded_at.elapsed() < ttl)
            .map(|(_, value)| value.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn coalesces_concurrent_misses() {
        let cache = Cache::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let (first, second) = tokio::join!(
            cache.read(Duration::from_secs(60), async {
                calls.fetch_add(1, Ordering::SeqCst);
                tokio::task::yield_now().await;
                1_u32
            }),
            cache.read(Duration::from_secs(60), async {
                calls.fetch_add(1, Ordering::SeqCst);
                2_u32
            }),
        );
        assert_eq!(first, 1);
        assert_eq!(second, 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn expires_after_ttl() {
        let cache = Cache::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let first = cache
            .read(Duration::from_millis(10), async {
                calls.fetch_add(1, Ordering::SeqCst);
                "ready"
            })
            .await;
        let second = cache
            .read(Duration::from_millis(10), async {
                calls.fetch_add(1, Ordering::SeqCst);
                "stale"
            })
            .await;
        assert_eq!(first, "ready");
        assert_eq!(second, "ready");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        tokio::time::sleep(Duration::from_millis(20)).await;
        let third = cache
            .read(Duration::from_millis(10), async {
                calls.fetch_add(1, Ordering::SeqCst);
                "fresh"
            })
            .await;
        assert_eq!(third, "fresh");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
    #[tokio::test]
    async fn caches_failures_until_expiry_then_recovers() {
        let cache = Cache::new();
        let ttl = Duration::from_secs(60);
        assert_eq!(
            cache.read(ttl, async { Err::<u32, _>("offline") }).await,
            Err("offline")
        );
        assert_eq!(
            cache
                .read(ttl, async { panic!("cached failures must not retry") })
                .await,
            Err("offline")
        );
        assert_eq!(cache.read(Duration::ZERO, async { Ok(42) }).await, Ok(42));
    }

    #[tokio::test]
    async fn a_cancelled_fetch_does_not_block_the_next_read() {
        let cache = Cache::new();
        {
            let mut read =
                std::pin::pin!(cache.read(Duration::from_secs(60), std::future::pending::<u32>()));
            tokio::select! {
                _ = &mut read => panic!("pending read completed"),
                _ = tokio::task::yield_now() => {},
            }
        }
        assert_eq!(cache.read(Duration::from_secs(60), async { 42 }).await, 42);
    }
}
