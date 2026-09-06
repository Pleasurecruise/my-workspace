use super::Cache;
use crate::NotesError;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn concurrent_first_reads_share_one_request() {
    let cache = Cache::default();
    let requests = AtomicUsize::new(0);
    let fetch = || async {
        requests.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        Ok::<_, String>(42)
    };
    let (first, second) = tokio::join!(
        cache.read("account", false, fetch()),
        cache.read("account", false, fetch())
    );
    assert_eq!(first.unwrap(), 42);
    assert_eq!(second.unwrap(), 42);
    for _ in 0..10 {
        assert_eq!(cache.read("account", false, fetch()).await.unwrap(), 42);
    }
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    cache.read("account", true, fetch()).await.unwrap();
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    cache.read("other-account", false, fetch()).await.unwrap();
    cache.read("account", false, fetch()).await.unwrap();
    assert_eq!(requests.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn failures_remain_cached_until_manual_refresh() {
    for error in [
        NotesError::Failed("session initialization failed".into()),
        NotesError::VerificationRequired(1034),
        NotesError::VerificationRequired(10035),
        NotesError::VerificationRequired(10041),
    ] {
        let cache = Cache::default();
        let requests = AtomicUsize::new(0);
        let fetch = || async {
            requests.fetch_add(1, Ordering::SeqCst);
            Err::<usize, _>(error.clone())
        };
        for _ in 0..10 {
            assert!(cache.read("game-and-login", false, fetch()).await.is_err());
        }
        assert_eq!(requests.load(Ordering::SeqCst), 1);
        assert_eq!(
            cache
                .read("game-and-login", true, async { Ok(7) })
                .await
                .unwrap(),
            7
        );
        assert_eq!(
            cache.read("game-and-login", false, fetch()).await.unwrap(),
            7
        );
        assert_eq!(requests.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn failed_manual_refresh_does_not_start_automatic_retries() {
    let cache = Cache::default();
    assert_eq!(
        cache
            .read("daily-notes", false, async { Ok::<_, String>(1) })
            .await
            .unwrap(),
        1
    );
    assert!(
        cache
            .read("daily-notes", true, async { Err("offline".into()) })
            .await
            .is_err()
    );
    assert_eq!(
        cache
            .read("daily-notes", false, async { panic!("must not retry") })
            .await
            .unwrap_err(),
        "offline"
    );
}

#[tokio::test]
async fn pending_read_does_not_block_other_keys() {
    let cache = Cache::default();
    cache
        .read("cached", false, async { Ok::<_, String>(42) })
        .await
        .unwrap();
    let (started, ready) = tokio::sync::oneshot::channel();
    let pending = cache.read("slow", false, async {
        started.send(()).unwrap();
        std::future::pending::<Result<i32, String>>().await
    });
    let other = async {
        ready.await.unwrap();
        let cached = cache.read("cached", false, async {
            panic!("cached read must not fetch")
        });
        let fresh = cache.read("fresh", false, async { Ok(7) });
        let (cached, fresh) = tokio::join!(cached, fresh);
        assert_eq!(cached.unwrap(), 42);
        assert_eq!(fresh.unwrap(), 7);
    };
    tokio::pin!(pending);
    tokio::select! {
        _ = &mut pending => panic!("slow request must remain pending"),
        result = tokio::time::timeout(std::time::Duration::from_millis(100), other) => {
            result.expect("independent cache keys must not wait for the slow provider");
        }
    }
}
