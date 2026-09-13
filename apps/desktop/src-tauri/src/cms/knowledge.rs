use consumers::api::knowledge::Document;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

const ARTICLE_TTL: Duration = Duration::from_secs(30);
const ARTICLE_LIMIT: usize = 16;

struct CachedArticle {
    document: Document,
    loaded_at: Instant,
}

#[derive(Default)]
struct Cache {
    revision: u64,
    articles: HashMap<String, CachedArticle>,
    requests: HashMap<String, Weak<Mutex<()>>>,
}

pub(crate) struct KnowledgeReader {
    cache: Mutex<Cache>,
    reads: Semaphore,
    prefetches: Semaphore,
}

impl Default for KnowledgeReader {
    fn default() -> Self {
        Self {
            cache: Mutex::new(Cache::default()),
            reads: Semaphore::new(6),
            prefetches: Semaphore::new(2),
        }
    }
}

impl KnowledgeReader {
    pub(crate) async fn clear(&self) {
        let mut cache = self.cache.lock().await;
        cache.revision += 1;
        cache.articles.clear();
        cache.requests.clear();
    }

    pub(crate) async fn read(
        &self,
        id: &str,
        expected_hash: Option<&str>,
        prefetch: bool,
    ) -> Result<Document, String> {
        self.read_with(id, expected_hash, prefetch, async {
            let article = consumers::api::knowledge::get(id)
                .await
                .map_err(|error| error.to_string())?;
            consumers::api::knowledge::project_article(article)
                .await
                .map_err(|error| error.to_string())
        })
        .await
    }

    async fn read_with(
        &self,
        id: &str,
        expected_hash: Option<&str>,
        prefetch: bool,
        fetch: impl std::future::Future<Output = Result<Document, String>>,
    ) -> Result<Document, String> {
        let started = Instant::now();
        let (revision, request) =
            {
                let mut cache = self.cache.lock().await;
                cache
                    .articles
                    .retain(|_, item| item.loaded_at.elapsed() < ARTICLE_TTL);
                if let Some(item) = cache.articles.get(id).filter(|item| {
                    expected_hash.is_none_or(|hash| hash == item.document.content_hash)
                }) {
                    return Ok(item.document.clone());
                }
                cache
                    .requests
                    .retain(|_, request| request.strong_count() > 0);
                let request = match cache.requests.get(id).and_then(Weak::upgrade) {
                    Some(request) => request,
                    None => {
                        let request = Arc::new(Mutex::new(()));
                        cache
                            .requests
                            .insert(id.to_owned(), Arc::downgrade(&request));
                        request
                    }
                };
                (cache.revision, request)
            };
        let _request = request.lock().await;
        {
            let cache = self.cache.lock().await;
            if revision != cache.revision {
                return Err("Knowledge changed; open the article again.".to_owned());
            }
            if let Some(item) = cache.articles.get(id) {
                let current = expected_hash.is_none_or(|hash| hash == item.document.content_hash)
                    || item.loaded_at >= started;
                if current && item.loaded_at.elapsed() < ARTICLE_TTL {
                    return Ok(item.document.clone());
                }
            }
        }
        let _prefetch = if prefetch {
            Some(
                self.prefetches
                    .try_acquire()
                    .map_err(|_| "Prefetch capacity reached".to_owned())?,
            )
        } else {
            None
        };
        let _permit = self
            .reads
            .acquire()
            .await
            .map_err(|_| "Article reader unavailable".to_owned())?;
        let document = fetch.await?;
        let mut cache = self.cache.lock().await;
        if revision != cache.revision {
            return Err("Knowledge changed; open the article again.".to_owned());
        }
        cache.articles.insert(
            id.to_owned(),
            CachedArticle {
                document: document.clone(),
                loaded_at: Instant::now(),
            },
        );
        while cache.articles.len() > ARTICLE_LIMIT {
            let oldest = cache
                .articles
                .iter()
                .min_by_key(|(_, item)| item.loaded_at)
                .map(|(id, _)| id.clone());
            if let Some(oldest) = oldest {
                cache.articles.remove(&oldest);
            }
        }
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use consumers::api::knowledge::Visibility;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn document(hash: &str) -> Document {
        Document {
            id: "article".into(),
            slug: "article".into(),
            title: "Title".into(),
            summary: "Summary".into(),
            tags: vec![],
            visibility: Visibility::Private,
            content_hash: hash.into(),
            created_at: String::new(),
            updated_at: String::new(),
            newspaper_edition: None,
            source: "Body".into(),
            html: "<p>Body</p>".into(),
            toc: vec![],
            stats: cms_core::markdown::ReadingStats {
                word_count: 1,
                reading_minutes: 1,
            },
        }
    }

    #[tokio::test]
    async fn prefetch_and_click_share_one_fetch_then_hash_changes_refresh() {
        let reader = KnowledgeReader::default();
        let calls = AtomicUsize::new(0);
        let fetch = || async {
            calls.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
            Ok(document("v1"))
        };
        let (preview, click) = tokio::join!(
            reader.read_with("article", Some("v1"), true, fetch()),
            reader.read_with("article", Some("v1"), false, fetch()),
        );
        assert_eq!(preview.unwrap().content_hash, click.unwrap().content_hash);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        reader
            .read_with("article", Some("v1"), false, async {
                panic!("warm click fetched again")
            })
            .await
            .unwrap();
        let updated = reader
            .read_with("article", Some("v2"), false, async { Ok(document("v2")) })
            .await
            .unwrap();
        assert_eq!(updated.content_hash, "v2");
    }

    #[tokio::test]
    async fn invalidation_discards_inflight_results_and_failures_retry() {
        let reader = KnowledgeReader::default();
        let result = reader
            .read_with("article", None, false, async {
                reader.clear().await;
                Ok(document("old"))
            })
            .await;
        assert!(result.is_err());
        assert!(reader.cache.lock().await.articles.is_empty());
        assert!(
            reader
                .read_with("article", None, false, async { Err("offline".into()) })
                .await
                .is_err()
        );
        assert!(
            reader
                .read_with("article", None, false, async { Ok(document("new")) })
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn cache_is_bounded_and_expired_entries_reload() {
        let reader = KnowledgeReader::default();
        for index in 0..20 {
            reader
                .read_with(&index.to_string(), None, true, async { Ok(document("v1")) })
                .await
                .unwrap();
        }
        {
            let mut cache = reader.cache.lock().await;
            assert_eq!(cache.articles.len(), ARTICLE_LIMIT);
            assert!(!cache.articles.contains_key("0"));
            cache.articles.get_mut("19").unwrap().loaded_at = Instant::now() - ARTICLE_TTL;
        }
        assert_eq!(
            reader
                .read_with("19", None, false, async { Ok(document("v2")) })
                .await
                .unwrap()
                .content_hash,
            "v2"
        );
    }

    #[tokio::test]
    async fn saturated_prefetch_does_not_block_foreground() {
        let reader = KnowledgeReader::default();
        let _capacity = reader.prefetches.acquire_many(2).await.unwrap();
        assert!(
            reader
                .read_with("preview", None, true, async {
                    panic!("speculative fetch exceeded limit")
                })
                .await
                .is_err()
        );
        reader
            .read_with("article", None, false, async { Ok(document("v1")) })
            .await
            .unwrap();
    }
}
