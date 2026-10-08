pub mod credentials;
mod store;

use futures_util::StreamExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;
use vault::Stored;

pub use store::Notification;
use store::{Message, Store};

const SUBSCRIPTION_URL: &str = "https://ntfy.you-find.me/mail-summary/sse";
const LINE_LIMIT: usize = 1024 * 1024;

pub type Listener = Arc<dyn Fn(Vec<Notification>) + Send + Sync>;

pub struct Inbox {
    store: Arc<Store>,
    subscription: Mutex<Subscription>,
}

#[derive(Default)]
struct Subscription {
    active: bool,
    stop: Option<watch::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl Inbox {
    pub fn new(path: PathBuf) -> Self {
        Self {
            store: Arc::new(Store::new(path)),
            subscription: Mutex::new(Subscription::default()),
        }
    }

    pub async fn read(&self) -> Result<Vec<Notification>, String> {
        match &*self.store.snapshot.read().await {
            Ok(snapshot) => Ok(snapshot.notifications.clone()),
            Err(message) => Err(message.clone()),
        }
    }

    pub async fn mark_read(&self, id: &str) -> Result<Vec<Notification>, String> {
        self.store.mark_read(id).await
    }

    /// `None` keeps the current activity and reconnects with the latest credentials.
    pub async fn subscribe(&self, active: Option<bool>, listener: Listener) -> Result<(), String> {
        let mut subscription = self.subscription.lock().await;
        subscription
            .update(active, |stop| async {
                if let Err(message) = &*self.store.snapshot.read().await {
                    return Err(message.clone());
                }
                let credentials = match credentials::read().map_err(|error| error.to_string())? {
                    Stored::Missing => return Ok(None),
                    Stored::Ready(credentials) => credentials,
                };
                Ok(Some(tokio::spawn(run(
                    self.store.clone(),
                    credentials.token,
                    listener,
                    stop,
                ))))
            })
            .await
    }
}

impl Subscription {
    async fn update<F>(
        &mut self,
        active: Option<bool>,
        start: impl FnOnce(watch::Receiver<()>) -> F,
    ) -> Result<(), String>
    where
        F: std::future::Future<Output = Result<Option<JoinHandle<()>>, String>>,
    {
        if let Some(active) = active {
            if self.active == active && self.task.as_ref().is_some_and(|task| !task.is_finished()) {
                return Ok(());
            }
            self.active = active;
        }
        if let Some(task) = self.task.take() {
            // Stop network waits, but let an accepted message finish its disk/memory transaction.
            self.stop.take();
            let _ = task.await;
        }
        if self.active {
            let (stop, receiver) = watch::channel(());
            self.task = start(receiver).await?;
            self.stop = self.task.as_ref().map(|_| stop);
        }
        Ok(())
    }
}

async fn run(store: Arc<Store>, token: String, listener: Listener, mut stop: watch::Receiver<()>) {
    let client = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .user_agent(concat!("Vesper/", env!("CARGO_PKG_VERSION"), " inbox"))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            tracing::error!(%error, "could not create ntfy client");
            return;
        }
    };
    loop {
        let since = match &*store.snapshot.read().await {
            Ok(snapshot) => match &snapshot.last_id {
                Some(last_id) => last_id.clone(),
                None => "all".to_owned(),
            },
            Err(error) => {
                tracing::warn!(%error, "notification storage is unavailable");
                return;
            }
        };
        let request = client
            .get(SUBSCRIPTION_URL)
            .bearer_auth(&token)
            .query(&[("since", since)])
            .send();
        let response = tokio::select! {
            biased;
            _ = stop.changed() => return,
            response = request => response,
        };
        match response {
            Ok(response) => match response.error_for_status() {
                Ok(response) => consume(&store, &listener, response, &mut stop).await,
                Err(error) => {
                    tracing::warn!(%error, "ntfy subscription was rejected");
                    tokio::select! {
                        biased;
                        _ = stop.changed() => return,
                        _ = tokio::time::sleep(Duration::from_secs(15)) => {},
                    }
                    continue;
                }
            },
            Err(error) => {
                tracing::warn!(%error, "ntfy subscription could not connect");
                tokio::select! {
                    biased;
                    _ = stop.changed() => return,
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {},
                }
                continue;
            }
        }
        tokio::select! {
            biased;
            _ = stop.changed() => return,
            _ = tokio::time::sleep(Duration::from_secs(2)) => {},
        }
    }
}

async fn consume(
    store: &Store,
    listener: &Listener,
    response: reqwest::Response,
    stop: &mut watch::Receiver<()>,
) {
    let mut response = response.bytes_stream();
    let mut pending = Vec::new();
    loop {
        let chunk = tokio::select! {
            biased;
            _ = stop.changed() => return,
            chunk = response.next() => chunk,
        };
        let Some(chunk) = chunk else {
            return;
        };
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                tracing::warn!(%error, "ntfy subscription stream ended");
                return;
            }
        };
        pending.extend_from_slice(&chunk);
        if pending.len() > LINE_LIMIT && !pending.contains(&b'\n') {
            tracing::warn!("ntfy sent an oversized SSE line");
            return;
        }
        while let Some(newline) = pending.iter().position(|byte| *byte == b'\n') {
            if stop.has_changed().is_err() {
                return;
            }
            if newline > LINE_LIMIT {
                pending.drain(..=newline);
                tracing::warn!("ntfy sent an oversized SSE line");
                continue;
            }
            let line = pending.drain(..=newline).collect::<Vec<_>>();
            accept_line(store, listener, &line).await;
        }
    }
}

async fn accept_line(store: &Store, listener: &Listener, line: &[u8]) {
    let Ok(line) = std::str::from_utf8(line) else {
        tracing::warn!("ntfy sent a non-UTF-8 SSE line");
        return;
    };
    let Some(data) = line.trim_end_matches(['\n', '\r']).strip_prefix("data:") else {
        return;
    };
    let message = match serde_json::from_str::<Message>(data.trim_start()) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!(%error, "ntfy sent an invalid message payload");
            return;
        }
    };
    match store.accept(message).await {
        Ok(Some(notifications)) => listener(notifications),
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, "could not persist notification"),
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
