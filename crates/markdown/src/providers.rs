use futures_util::stream::{self, StreamExt, TryStreamExt};
use md_dialect::{ArticleMetadata, EmbedError, embed};
use std::collections::HashMap;

const CONCURRENCY: usize = 4;

pub(crate) async fn read(
    source: &str,
    articles: HashMap<String, ArticleMetadata>,
) -> Result<embed::Data, EmbedError> {
    let references = embed::references(source)?;
    let mut data = embed::Data {
        articles,
        ..embed::Data::default()
    };
    data.repositories = stream::iter(references.repositories.into_iter().map(|repo| async move {
        let snapshot = github::read_repository(&repo).await?;
        Ok::<_, String>((repo, snapshot))
    }))
    .buffer_unordered(CONCURRENCY)
    .try_collect()
    .await
    .map_err(EmbedError::Data)?;
    data.links = stream::iter(references.links.into_iter().map(|url| async move {
        let metadata = link_preview::read(&url).await?;
        Ok::<_, String>((url, metadata))
    }))
    .buffer_unordered(CONCURRENCY)
    .try_collect()
    .await
    .map_err(EmbedError::Data)?;
    data.tweets = stream::iter(references.tweets.into_iter().map(|url| async move {
        let post = link_preview::twitter::read(&url).await;
        (url, post)
    }))
    .buffer_unordered(CONCURRENCY)
    .collect()
    .await;
    if !references.stocks.is_empty() {
        let report = market_data::stocks::read(references.stocks.into_iter().collect())
            .await
            .map_err(EmbedError::Data)?;
        if let Some(failure) = report.failures.into_iter().next() {
            return Err(EmbedError::Data(failure.message));
        }
        for stock in report.stocks {
            data.stocks.insert(stock.symbol.clone(), stock);
        }
    }
    Ok(data)
}
