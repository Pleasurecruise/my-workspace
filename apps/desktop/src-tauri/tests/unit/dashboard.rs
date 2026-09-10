use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn inactive_or_superseded_routes_do_not_start_reads() {
    let (active, receiver) = watch::channel(false);
    let reads = AtomicUsize::new(0);
    assert!(
        read_while_active(receiver, async { reads.fetch_add(1, Ordering::SeqCst) })
            .await
            .is_none()
    );
    active.send_replace(true);
    let receiver = active.subscribe();
    active.send_replace(false);
    active.send_replace(true);
    assert!(
        read_while_active(receiver, async { reads.fetch_add(1, Ordering::SeqCst) })
            .await
            .is_none()
    );
    assert_eq!(reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn navigation_drops_inflight_read_and_reentry_can_read_again() {
    let (active, receiver) = watch::channel(true);
    let (started, waiting) = tokio::sync::oneshot::channel();
    let (reply, response) = tokio::sync::oneshot::channel::<()>();
    let request = tokio::spawn(read_while_active(receiver, async move {
        started.send(()).unwrap();
        response.await.unwrap();
    }));
    waiting.await.unwrap();
    active.send_replace(false);
    assert!(request.await.unwrap().is_none());
    assert!(reply.send(()).is_err());
    active.send_replace(true);
    assert_eq!(
        read_while_active(active.subscribe(), async { 42 }).await,
        Some(42)
    );
}

#[tokio::test]
async fn navigation_cancels_reads_waiting_for_a_source_lock() {
    let lock = Arc::new(AsyncMutex::new(()));
    let guard = lock.lock().await;
    let (active, receiver) = watch::channel(true);
    let reads = AtomicUsize::new(0);
    let request = read_while_active(receiver, async {
        let _guard = lock.lock().await;
        reads.fetch_add(1, Ordering::SeqCst);
    });
    tokio::pin!(request);
    assert!(futures_util::poll!(&mut request).is_pending());
    active.send_replace(false);
    drop(guard);
    assert!(request.await.is_none());
    assert_eq!(reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn absent_widgets_and_invalid_layouts_never_start_provider_io() {
    let reads = AtomicUsize::new(0);
    for enabled in [Ok(false), Err("invalid layout".to_owned())] {
        let response = read_provider(enabled.clone(), async {
            reads.fetch_add(1, Ordering::SeqCst);
            Ok(42)
        })
        .await;
        match enabled {
            Ok(false) => assert!(matches!(response, CommandResponse::Ready { data: None })),
            Err(_) => assert!(matches!(response, CommandResponse::Failed { .. })),
            _ => unreachable!(),
        }
    }
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    assert!(matches!(
        read_provider(Ok(true), async {
            reads.fetch_add(1, Ordering::SeqCst);
            Ok(42)
        })
        .await,
        CommandResponse::Ready { data: Some(42) }
    ));
    assert_eq!(reads.load(Ordering::SeqCst), 1);
    assert!(matches!(
        read_provider(Ok(true), async { Err::<u8, _>("offline".to_owned()) }).await,
        CommandResponse::Failed { .. }
    ));
}
