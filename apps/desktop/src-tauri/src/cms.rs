use crate::CommandResponse;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

const ASSET_LIMIT: usize = 64;
const ASSET_BYTES: usize = 128 * 1024 * 1024;
const VIEW_TTL: Duration = Duration::from_secs(30);

pub(crate) struct ChannelRequest {
    pub(crate) channel: consumers::view::Channel,
    pub(crate) cursor: Option<String>,
    pub(crate) filters: consumers::api::memos::ListFilters,
    pub(crate) read_cached_first_page: bool,
}

impl ChannelRequest {
    pub(crate) fn initial(channel: consumers::view::Channel) -> Self {
        Self {
            channel,
            cursor: None,
            filters: consumers::api::memos::ListFilters::default(),
            read_cached_first_page: true,
        }
    }
}

struct CachedView {
    data: consumers::view::ChannelView,
    loaded_at: Instant,
}

#[derive(Default)]
struct ViewCache {
    memos: Option<CachedView>,
    moment: Option<CachedView>,
    knowledge: Option<CachedView>,
}

impl ViewCache {
    fn get(&self, channel: consumers::view::Channel) -> Option<consumers::view::ChannelView> {
        let entry = match channel {
            consumers::view::Channel::Memos => self.memos.as_ref(),
            consumers::view::Channel::Moment => self.moment.as_ref(),
            consumers::view::Channel::Knowledge => self.knowledge.as_ref(),
        }?;
        (entry.loaded_at.elapsed() <= VIEW_TTL).then(|| entry.data.clone())
    }

    fn insert(&mut self, channel: consumers::view::Channel, data: consumers::view::ChannelView) {
        let entry = Some(CachedView {
            data,
            loaded_at: Instant::now(),
        });
        match channel {
            consumers::view::Channel::Memos => self.memos = entry,
            consumers::view::Channel::Moment => self.moment = entry,
            consumers::view::Channel::Knowledge => self.knowledge = entry,
        }
    }

    fn clear(&mut self, channel: consumers::view::Channel) {
        match channel {
            consumers::view::Channel::Memos => self.memos = None,
            consumers::view::Channel::Moment => self.moment = None,
            consumers::view::Channel::Knowledge => self.knowledge = None,
        }
    }
}

#[derive(Default)]
struct AssetCache {
    data: HashMap<String, Vec<u8>>,
    order: VecDeque<String>,
    bytes: usize,
}

impl AssetCache {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        let data = self.data.get(key)?.clone();
        self.order.retain(|cached| cached != key);
        self.order.push_back(key.to_owned());
        Some(data)
    }

    fn insert(&mut self, key: String, data: Vec<u8>) {
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
        self.order.clear();
        self.bytes = 0;
    }
}

pub(crate) struct CmsState {
    repository: tokio::sync::Mutex<Option<Arc<consumers::view::Repository>>>,
    views: tokio::sync::Mutex<ViewCache>,
    assets: tokio::sync::Mutex<AssetCache>,
}

impl Default for CmsState {
    fn default() -> Self {
        Self {
            repository: tokio::sync::Mutex::new(None),
            views: tokio::sync::Mutex::new(ViewCache::default()),
            assets: tokio::sync::Mutex::new(AssetCache::default()),
        }
    }
}

impl CmsState {
    pub(crate) async fn repository(&self) -> Result<Arc<consumers::view::Repository>, String> {
        let mut state = self.repository.lock().await;
        if let Some(repository) = state.as_ref() {
            return Ok(Arc::clone(repository));
        }
        let repository = cms_core::r2::Store::from_credentials()
            .await
            .map(consumers::view::Repository::new)
            .map(Arc::new)
            .map_err(|error| error.to_string())?;
        *state = Some(Arc::clone(&repository));
        Ok(repository)
    }

    pub(crate) async fn cached_asset(&self, key: &str) -> Option<Vec<u8>> {
        self.assets.lock().await.get(key)
    }

    pub(crate) async fn cache_asset(&self, key: String, data: Vec<u8>) {
        self.assets.lock().await.insert(key, data);
    }

    pub(crate) async fn clear_assets(&self) {
        self.assets.lock().await.clear();
    }

    pub(crate) async fn asset(&self, key: &str) -> Result<Vec<u8>, String> {
        if let Some(data) = self.cached_asset(key).await {
            return Ok(data);
        }
        let repository = self.repository().await?;
        let data = consumers::view::asset(key, repository.as_ref())
            .await
            .map_err(|error| error.to_string())?;
        self.cache_asset(key.to_owned(), data.clone()).await;
        Ok(data)
    }

    pub(crate) async fn invalidate_view(&self, channel: consumers::view::Channel) {
        self.views.lock().await.clear(channel);
    }

    pub(crate) async fn reset(&self) {
        self.reset_views().await;
        self.clear_assets().await;
        *self.repository.lock().await = None;
    }

    pub(crate) async fn reset_views(&self) {
        *self.views.lock().await = ViewCache::default();
    }

    pub(crate) async fn channel(
        &self,
        request: ChannelRequest,
    ) -> CommandResponse<consumers::view::ChannelView> {
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
        let cached = if read_cached_first_page && cacheable {
            self.views.lock().await.get(channel)
        } else {
            None
        };
        if let Some(data) = cached {
            return CommandResponse::Ready { data };
        }
        let result = match channel {
            consumers::view::Channel::Memos => {
                match consumers::api::memos::list(cursor, &filters).await {
                    Ok(page) => Ok(consumers::view::ChannelView::Memos {
                        connected: true,
                        memos: page.memos,
                        tags: Vec::new(),
                        next_cursor: page.next_cursor,
                    }),
                    Err(error) => Err(error.to_string()),
                }
            }
            consumers::view::Channel::Knowledge => {
                let result = match cursor {
                    Some(cursor) => consumers::api::knowledge::list(Some(cursor)).await,
                    None => consumers::api::knowledge::overview().await,
                };
                match result {
                    Ok(page) => {
                        let newspaper =
                            consumers::api::knowledge::latest_newspaper_issues(&page.documents);
                        Ok(consumers::view::ChannelView::Knowledge {
                            connected: true,
                            knowledge: page.documents,
                            newspaper,
                            next_cursor: page.cursor,
                        })
                    }
                    Err(error) => Err(error.to_string()),
                }
            }
            consumers::view::Channel::Moment => match consumers::api::moment::list(cursor).await {
                Ok(page) => Ok(consumers::view::ChannelView::Moment {
                    connected: true,
                    photos: page.photos,
                    tags: Vec::new(),
                    total: page.total,
                    next_cursor: page.next_cursor,
                }),
                Err(error) => Err(error.to_string()),
            },
        };
        match result {
            Ok(data) => {
                if cacheable {
                    self.views.lock().await.insert(channel, data.clone());
                }
                CommandResponse::Ready { data }
            }
            Err(message) => CommandResponse::Failed { message },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expires_view_cache() {
        let mut cache = ViewCache::default();
        cache.insert(
            consumers::view::Channel::Moment,
            consumers::view::ChannelView::Moment {
                connected: true,
                photos: Vec::new(),
                tags: Vec::new(),
                total: 0,
                next_cursor: Some("cursor".to_owned()),
            },
        );
        assert!(cache.get(consumers::view::Channel::Moment).is_some());
        cache.moment.as_mut().unwrap().loaded_at =
            Instant::now() - VIEW_TTL - Duration::from_secs(1);
        assert!(cache.get(consumers::view::Channel::Moment).is_none());
        cache.clear(consumers::view::Channel::Moment);
        assert!(cache.moment.is_none());
    }

    #[test]
    fn evicts_oldest_asset() {
        let mut cache = AssetCache::default();
        for index in 0..=ASSET_LIMIT {
            cache.insert(format!("img/{index}.jpg"), vec![index as u8]);
        }
        assert!(cache.get("img/0.jpg").is_none());
        assert_eq!(
            cache.get(&format!("img/{ASSET_LIMIT}.jpg")),
            Some(vec![ASSET_LIMIT as u8])
        );
        cache.clear();
        assert!(cache.data.is_empty());
        assert_eq!(cache.bytes, 0);
    }

    #[tokio::test]
    #[cfg(debug_assertions)]
    #[ignore = "requires live consumer and R2 credentials"]
    async fn bypasses_page_cache() {
        vesper_credentials::load_dev_environment().expect("development credentials should load");
        let state = CmsState::default();

        for channel in [
            consumers::view::Channel::Memos,
            consumers::view::Channel::Moment,
        ] {
            let channel_name = match channel {
                consumers::view::Channel::Memos => "memos",
                consumers::view::Channel::Moment => "moment",
                consumers::view::Channel::Knowledge => "knowledge",
            };
            let first = state
                .channel(ChannelRequest {
                    channel,
                    cursor: None,
                    filters: consumers::api::memos::ListFilters::default(),
                    read_cached_first_page: false,
                })
                .await;
            let cursor = match first {
                CommandResponse::Ready {
                    data: consumers::view::ChannelView::Memos { next_cursor, .. },
                }
                | CommandResponse::Ready {
                    data: consumers::view::ChannelView::Moment { next_cursor, .. },
                } => next_cursor.expect("the live first page should have a cursor"),
                CommandResponse::Ready { .. } => panic!("channel returned the wrong view"),
                CommandResponse::Failed { message } => {
                    panic!("{channel_name} first page failed: {message}")
                }
            };

            let second = state
                .channel(ChannelRequest {
                    channel,
                    cursor: Some(cursor),
                    filters: consumers::api::memos::ListFilters::default(),
                    read_cached_first_page: false,
                })
                .await;
            match second {
                CommandResponse::Ready {
                    data: consumers::view::ChannelView::Memos { memos, tags, .. },
                } => {
                    assert!(!memos.is_empty());
                    assert!(tags.is_empty());
                }
                CommandResponse::Ready {
                    data: consumers::view::ChannelView::Moment { photos, tags, .. },
                } => {
                    assert!(!photos.is_empty());
                    assert!(tags.is_empty());
                }
                CommandResponse::Ready { .. } => panic!("channel returned the wrong view"),
                CommandResponse::Failed { message } => {
                    panic!("{channel_name} second page failed: {message}")
                }
            }
        }
    }
}
