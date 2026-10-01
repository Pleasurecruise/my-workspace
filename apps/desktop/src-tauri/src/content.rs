pub(crate) mod knowledge;
pub(crate) mod memos;
pub(crate) mod moment;

use crate::CommandResponse;
use consumers::api::memos::ListFilters;
use consumers::view::{Channel, ChannelView, Repository};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use tauri::Manager;

const ASSET_LIMIT: usize = 64;
const ASSET_BYTES: usize = 128 * 1024 * 1024;
const VIEW_TTL: Duration = Duration::from_secs(30);

pub(crate) struct ChannelRequest {
    pub(crate) channel: Channel,
    pub(crate) cursor: Option<String>,
    pub(crate) filters: ListFilters,
    pub(crate) read_cached_first_page: bool,
}

impl ChannelRequest {
    pub(crate) fn initial(channel: Channel) -> Self {
        Self {
            channel,
            cursor: None,
            filters: ListFilters::default(),
            read_cached_first_page: true,
        }
    }
}

struct CachedView {
    data: ChannelView,
    loaded_at: Instant,
}

#[derive(Default)]
struct ViewSlot {
    revision: u64,
    cached: Option<CachedView>,
}

#[derive(Default)]
struct ViewCache {
    memos: ViewSlot,
    moment: ViewSlot,
    knowledge: ViewSlot,
}

impl ViewCache {
    fn slot(&mut self, channel: Channel) -> &mut ViewSlot {
        match channel {
            Channel::Memos => &mut self.memos,
            Channel::Moment => &mut self.moment,
            Channel::Knowledge => &mut self.knowledge,
        }
    }

    fn clear(&mut self, channel: Channel) {
        let slot = self.slot(channel);
        slot.revision += 1;
        slot.cached = None;
    }
}

type AssetRequest = tokio::sync::Mutex<Option<Result<Arc<Vec<u8>>, String>>>;

#[derive(Default)]
struct AssetCache {
    data: HashMap<String, Arc<Vec<u8>>>,
    requests: HashMap<String, Weak<AssetRequest>>,
    order: VecDeque<String>,
    bytes: usize,
}

impl AssetCache {
    fn get(&mut self, key: &str) -> Option<Arc<Vec<u8>>> {
        let data = Arc::clone(self.data.get(key)?);
        self.order.retain(|cached| cached != key);
        self.order.push_back(key.to_owned());
        Some(data)
    }

    fn insert(&mut self, key: String, data: Arc<Vec<u8>>) {
        if data.len() > ASSET_BYTES {
            return;
        }
        if let Some(previous) = self.data.remove(&key) {
            self.bytes -= previous.len();
            self.order.retain(|cached| cached != &key);
        }
        self.bytes += data.len();
        self.order.push_back(key.clone());
        self.data.insert(key, data);
        while self.order.len() > ASSET_LIMIT || self.bytes > ASSET_BYTES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(removed) = self.data.remove(&oldest) {
                self.bytes -= removed.len();
            }
        }
    }

    fn clear(&mut self) {
        self.data.clear();
        self.requests.clear();
        self.order.clear();
        self.bytes = 0;
    }
}

pub(crate) struct Content {
    repository: tokio::sync::Mutex<Option<Arc<Repository>>>,
    views: tokio::sync::Mutex<ViewCache>,
    assets: tokio::sync::Mutex<AssetCache>,
    pub(crate) knowledge: knowledge::Reader,
}

impl Default for Content {
    fn default() -> Self {
        Self {
            repository: tokio::sync::Mutex::new(None),
            views: tokio::sync::Mutex::new(ViewCache::default()),
            assets: tokio::sync::Mutex::new(AssetCache::default()),
            knowledge: knowledge::Reader::default(),
        }
    }
}

impl Content {
    pub(crate) async fn repository(&self) -> Result<Arc<Repository>, String> {
        let mut state = self.repository.lock().await;
        if let Some(repository) = state.as_ref() {
            return Ok(Arc::clone(repository));
        }
        let repository = ::cms::r2::Store::from_credentials()
            .await
            .map(Repository::new)
            .map(Arc::new)
            .map_err(|error| error.to_string())?;
        *state = Some(Arc::clone(&repository));
        Ok(repository)
    }

    pub(crate) async fn clear_assets(&self) {
        self.assets.lock().await.clear();
    }

    pub(crate) async fn asset(&self, key: &str) -> Result<Arc<Vec<u8>>, String> {
        let request = {
            let mut cache = self.assets.lock().await;
            if let Some(data) = cache.get(key) {
                return Ok(data);
            }
            cache
                .requests
                .retain(|_, request| request.strong_count() > 0);
            match cache.requests.get(key).and_then(Weak::upgrade) {
                Some(request) => request,
                None => {
                    let request = Arc::new(AssetRequest::new(None));
                    cache
                        .requests
                        .insert(key.to_owned(), Arc::downgrade(&request));
                    request
                }
            }
        };
        let mut loaded = request.lock().await;
        let result = match loaded.as_ref() {
            Some(result) => result.clone(),
            None => {
                let result = match self.repository().await {
                    Ok(repository) => consumers::view::asset(key, repository.as_ref())
                        .await
                        .map(Arc::new)
                        .map_err(|error| error.to_string()),
                    Err(message) => Err(message),
                };
                *loaded = Some(result.clone());
                result
            }
        };
        let mut cache = self.assets.lock().await;
        if cache
            .requests
            .get(key)
            .is_some_and(|pending| pending.ptr_eq(&Arc::downgrade(&request)))
        {
            cache.requests.remove(key);
            if let Ok(data) = &result {
                cache.insert(key.to_owned(), Arc::clone(data));
            }
        }
        result
    }

    pub(crate) async fn invalidate_view(&self, channel: Channel) {
        self.views.lock().await.clear(channel);
        if channel == Channel::Knowledge {
            self.knowledge.clear().await;
        }
    }

    pub(crate) async fn reset(&self) {
        self.reset_views().await;
        self.clear_assets().await;
        *self.repository.lock().await = None;
    }

    pub(crate) async fn reset_views(&self) {
        self.knowledge.clear().await;
        let mut views = self.views.lock().await;
        for channel in [Channel::Memos, Channel::Moment, Channel::Knowledge] {
            views.clear(channel);
        }
    }

    pub(crate) async fn channel(&self, request: ChannelRequest) -> CommandResponse<ChannelView> {
        let ChannelRequest {
            channel,
            cursor,
            filters,
            read_cached_first_page,
        } = request;
        let cacheable = cursor.is_none()
            && filters.search.is_none()
            && filters.tags.is_empty()
            && !filters.sort_by_updated
            && !filters.archived_only
            && !filters.favorites_only;
        self.load_view(channel, cacheable, read_cached_first_page, async move {
            let view = match channel {
                Channel::Memos => {
                    let page = consumers::api::memos::list(cursor, &filters).await;
                    page.map(|page| ChannelView::Memos {
                        memos: page.memos,
                        next_cursor: page.next_cursor,
                    })
                }
                Channel::Knowledge => {
                    let page = match cursor {
                        Some(cursor) => consumers::api::knowledge::list(Some(cursor)).await,
                        None => consumers::api::knowledge::overview().await,
                    };
                    page.map(|page| ChannelView::Knowledge {
                        newspaper: consumers::api::knowledge::latest_newspaper_issues(
                            &page.documents,
                        ),
                        knowledge: page.documents,
                        next_cursor: page.cursor,
                    })
                }
                Channel::Moment => {
                    let page = consumers::api::moment::list().await;
                    page.map(|page| ChannelView::Moment {
                        photos: page.photos,
                        total: page.total,
                    })
                }
            };
            view.map_err(|error| error.to_string())
        })
        .await
    }

    async fn load_view(
        &self,
        channel: Channel,
        cacheable: bool,
        read_cached: bool,
        fetch: impl std::future::Future<Output = Result<ChannelView, String>>,
    ) -> CommandResponse<ChannelView> {
        let revision = if cacheable {
            let mut views = self.views.lock().await;
            let slot = views.slot(channel);
            if read_cached
                && let Some(entry) = &slot.cached
                && entry.loaded_at.elapsed() <= VIEW_TTL
            {
                return CommandResponse::Ready {
                    data: entry.data.clone(),
                };
            }
            slot.revision += 1;
            Some(slot.revision)
        } else {
            None
        };
        match fetch.await {
            Ok(data) => {
                if let Some(revision) = revision {
                    let mut views = self.views.lock().await;
                    let slot = views.slot(channel);
                    if slot.revision == revision {
                        slot.cached = Some(CachedView {
                            data: data.clone(),
                            loaded_at: Instant::now(),
                        });
                    }
                }
                CommandResponse::Ready { data }
            }
            Err(message) => CommandResponse::Failed { message },
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelQuery {
    channel: String,
    cursor: Option<String>,
    search: Option<String>,
    tags: Vec<String>,
    sort_by_updated: bool,
    archived_only: bool,
    favorites_only: bool,
}

#[derive(Clone, serde::Serialize)]
pub(crate) struct InitialViews {
    memos: CommandResponse<ChannelView>,
    moment: CommandResponse<ChannelView>,
    knowledge: CommandResponse<ChannelView>,
}

#[tauri::command]
pub(crate) async fn initialize_views(app: tauri::AppHandle) -> InitialViews {
    let state = app.state::<Content>();
    let (memos, moment, knowledge) = tokio::join!(
        state.channel(ChannelRequest::initial(Channel::Memos,)),
        state.channel(ChannelRequest::initial(Channel::Moment,)),
        state.channel(ChannelRequest::initial(Channel::Knowledge,)),
    );
    InitialViews {
        memos,
        moment,
        knowledge,
    }
}

#[tauri::command]
pub(crate) async fn read_channel(
    query: ChannelQuery,
    app: tauri::AppHandle,
) -> CommandResponse<ChannelView> {
    let paginated = query.cursor.is_some();
    let started = Instant::now();
    let channel = match Channel::try_from(query.channel.as_str()) {
        Ok(channel) => channel,
        Err(error) => return Err::<ChannelView, _>(error).into(),
    };
    if channel == Channel::Moment && query.cursor.is_some() {
        return CommandResponse::Failed {
            message: "Moment returns one bounded gallery batch; cursors are not supported."
                .to_owned(),
        };
    }
    let response = app
        .state::<Content>()
        .channel(ChannelRequest {
            channel,
            cursor: query.cursor,
            filters: ListFilters {
                limit: None,
                search: query.search,
                tags: query.tags,
                sort_by_updated: query.sort_by_updated,
                archived_only: query.archived_only,
                favorites_only: query.favorites_only,
            },
            read_cached_first_page: false,
        })
        .await;
    match &response {
        CommandResponse::Ready { data } => {
            let (items, has_next) = match data {
                ChannelView::Memos {
                    memos, next_cursor, ..
                } => (memos.len(), next_cursor.is_some()),
                ChannelView::Moment { photos, .. } => (photos.len(), false),
                ChannelView::Knowledge {
                    knowledge,
                    next_cursor,
                    ..
                } => (knowledge.len(), next_cursor.is_some()),
            };
            tracing::info!(
                channel = ?channel,
                paginated,
                items,
                has_next,
                elapsed_ms = started.elapsed().as_millis(),
                "consumer channel ready"
            );
        }
        CommandResponse::Failed { message } => {
            tracing::warn!(
                channel = ?channel,
                paginated,
                error = %message,
                elapsed_ms = started.elapsed().as_millis(),
                "consumer channel failed"
            );
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memo_page(cursor: &str) -> ChannelView {
        ChannelView::Memos {
            memos: Vec::new(),
            next_cursor: Some(cursor.to_owned()),
        }
    }

    fn cursor(response: CommandResponse<ChannelView>) -> String {
        match response {
            CommandResponse::Ready {
                data: ChannelView::Memos { next_cursor, .. },
            } => next_cursor.unwrap(),
            _ => panic!("expected a memo page"),
        }
    }

    #[tokio::test]
    async fn startup_uses_fresh_cache_but_refresh_and_expiry_fetch() {
        let state = Content::default();
        let channel = Channel::Memos;
        assert_eq!(
            cursor(
                state
                    .load_view(channel, true, true, async { Ok(memo_page("first")) })
                    .await
            ),
            "first"
        );
        assert_eq!(
            cursor(
                state
                    .load_view(channel, true, true, async {
                        panic!("cache hit must not fetch")
                    })
                    .await
            ),
            "first"
        );
        assert_eq!(
            cursor(
                state
                    .load_view(channel, true, false, async { Ok(memo_page("refreshed")) })
                    .await
            ),
            "refreshed"
        );
        assert_eq!(
            cursor(
                state
                    .load_view(channel, false, false, async { Ok(memo_page("filtered")) })
                    .await
            ),
            "filtered"
        );
        assert_eq!(
            cursor(
                state
                    .load_view(channel, true, true, async {
                        panic!("filtered page must not replace startup cache")
                    })
                    .await
            ),
            "refreshed"
        );
        state
            .views
            .lock()
            .await
            .memos
            .cached
            .as_mut()
            .unwrap()
            .loaded_at = Instant::now() - VIEW_TTL - Duration::from_secs(1);
        assert_eq!(
            cursor(
                state
                    .load_view(channel, true, true, async { Ok(memo_page("expired")) })
                    .await
            ),
            "expired"
        );
    }

    #[tokio::test]
    async fn invalidation_prevents_pending_reads_from_repopulating_startup_cache() {
        for reset_all in [false, true] {
            let state = Content::default();
            let channel = Channel::Memos;
            let (send, reply) = tokio::sync::oneshot::channel();
            let pending = state.load_view(channel, true, false, async { reply.await.unwrap() });
            tokio::pin!(pending);
            assert!(futures_util::poll!(&mut pending).is_pending());
            if reset_all {
                state.reset_views().await;
            } else {
                state.invalidate_view(channel).await;
            }
            send.send(Ok(memo_page("stale"))).unwrap();
            pending.await;
            assert_eq!(
                cursor(
                    state
                        .load_view(channel, true, true, async { Ok(memo_page("current")) })
                        .await
                ),
                "current"
            );
        }
    }

    #[tokio::test]
    async fn late_reads_never_replace_newer_cache_entries() {
        let state = Content::default();
        let channel = Channel::Memos;
        let (send, reply) = tokio::sync::oneshot::channel();
        let pending = state.load_view(channel, true, false, async { reply.await.unwrap() });
        tokio::pin!(pending);
        assert!(futures_util::poll!(&mut pending).is_pending());
        state
            .load_view(channel, true, false, async { Ok(memo_page("newer")) })
            .await;
        send.send(Ok(memo_page("older"))).unwrap();
        pending.await;
        assert_eq!(
            cursor(
                state
                    .load_view(channel, true, true, async {
                        panic!("newer cache must survive")
                    })
                    .await
            ),
            "newer"
        );
    }

    #[tokio::test]
    async fn failed_refresh_preserves_settled_cache() {
        let state = Content::default();
        let channel = Channel::Memos;
        state
            .load_view(channel, true, true, async { Ok(memo_page("settled")) })
            .await;
        assert!(matches!(
            state
                .load_view(channel, true, false, async { Err("offline".to_owned()) })
                .await,
            CommandResponse::Failed { .. }
        ));
        assert_eq!(
            cursor(
                state
                    .load_view(channel, true, true, async {
                        panic!("settled cache must survive")
                    })
                    .await
            ),
            "settled"
        );
    }

    #[test]
    fn evicts_oldest_asset() {
        let mut cache = AssetCache::default();
        for index in 0..=ASSET_LIMIT {
            cache.insert(format!("img/{index}.jpg"), Arc::new(vec![index as u8]));
        }
        assert!(cache.get("img/0.jpg").is_none());
        assert_eq!(
            cache.get(&format!("img/{ASSET_LIMIT}.jpg")),
            Some(Arc::new(vec![ASSET_LIMIT as u8]))
        );
        cache.clear();
        assert!(cache.data.is_empty());
        assert_eq!(cache.bytes, 0);
    }

    #[test]
    fn keeps_recently_read_assets() {
        let mut cache = AssetCache::default();
        for index in 0..ASSET_LIMIT {
            cache.insert(format!("img/{index}.jpg"), Arc::new(vec![index as u8]));
        }
        assert_eq!(cache.get("img/0.jpg"), Some(Arc::new(vec![0])));

        cache.insert("img/new.jpg".to_owned(), Arc::new(vec![255]));

        assert!(cache.get("img/1.jpg").is_none());
        assert_eq!(cache.get("img/0.jpg"), Some(Arc::new(vec![0])));
        assert_eq!(cache.get("img/new.jpg"), Some(Arc::new(vec![255])));
        assert_eq!(cache.bytes, ASSET_LIMIT);
    }

    #[test]
    fn replaces_cached_assets() {
        let mut cache = AssetCache::default();
        cache.insert("img/photo.jpg".to_owned(), Arc::new(vec![1, 2, 3]));
        cache.insert("img/other.jpg".to_owned(), Arc::new(vec![4]));
        cache.insert("img/photo.jpg".to_owned(), Arc::new(vec![5, 6]));

        assert_eq!(cache.get("img/photo.jpg"), Some(Arc::new(vec![5, 6])));
        assert_eq!(cache.get("img/other.jpg"), Some(Arc::new(vec![4])));
        assert_eq!(cache.bytes, 3);
        assert_eq!(cache.order.len(), 2);
    }

    #[tokio::test]
    async fn shares_pending_asset_reads() {
        let state = Content::default();
        let request = Arc::new(AssetRequest::new(None));
        state
            .assets
            .lock()
            .await
            .requests
            .insert("photo".to_owned(), Arc::downgrade(&request));
        let mut loaded = request.lock().await;
        let first = state.asset("photo");
        let second = state.asset("photo");
        tokio::pin!(first, second);
        assert!(futures_util::poll!(&mut first).is_pending());
        assert!(futures_util::poll!(&mut second).is_pending());
        let bytes = Arc::new(vec![1, 2, 3]);
        *loaded = Some(Ok(Arc::clone(&bytes)));
        drop(loaded);

        let (first, second) = tokio::join!(first, second);
        assert!(Arc::ptr_eq(&first.expect("first read"), &bytes));
        assert!(Arc::ptr_eq(&second.expect("second read"), &bytes));
        assert!(Arc::ptr_eq(
            &state.asset("photo").await.expect("cached read"),
            &bytes
        ));
        assert!(state.assets.lock().await.requests.is_empty());
    }

    #[tokio::test]
    async fn shares_asset_errors_without_caching_them() {
        let state = Content::default();
        let request = Arc::new(AssetRequest::new(None));
        state
            .assets
            .lock()
            .await
            .requests
            .insert("photo".to_owned(), Arc::downgrade(&request));
        let mut loaded = request.lock().await;
        let first = state.asset("photo");
        let second = state.asset("photo");
        tokio::pin!(first, second);
        assert!(futures_util::poll!(&mut first).is_pending());
        assert!(futures_util::poll!(&mut second).is_pending());
        *loaded = Some(Err("download failed".to_owned()));
        drop(loaded);

        let (first, second) = tokio::join!(first, second);
        assert_eq!(first, Err("download failed".to_owned()));
        assert_eq!(second, Err("download failed".to_owned()));
        let cache = state.assets.lock().await;
        assert!(cache.requests.is_empty());
        assert!(cache.data.is_empty());
    }

    #[tokio::test]
    async fn cleared_assets_stay_invalidated() {
        let state = Content::default();
        let request = Arc::new(AssetRequest::new(None));
        state
            .assets
            .lock()
            .await
            .requests
            .insert("photo".to_owned(), Arc::downgrade(&request));
        let mut loaded = request.lock().await;
        let read = state.asset("photo");
        tokio::pin!(read);
        assert!(futures_util::poll!(&mut read).is_pending());
        state.clear_assets().await;
        let replacement = Arc::new(AssetRequest::new(None));
        state
            .assets
            .lock()
            .await
            .requests
            .insert("photo".to_owned(), Arc::downgrade(&replacement));
        *loaded = Some(Ok(Arc::new(vec![1])));
        drop(loaded);

        assert!(read.await.is_ok());
        let cache = state.assets.lock().await;
        assert!(cache.data.is_empty());
        assert!(cache.requests["photo"].ptr_eq(&Arc::downgrade(&replacement)));
    }
}
