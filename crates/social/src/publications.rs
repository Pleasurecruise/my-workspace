use super::{PublicationProvider, PublishError, PublishedPost};
use diesel::prelude::*;
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Publication {
    pub memo_id: String,
    pub provider: PublicationProvider,
}

diesel::table! {
    memo_publications (memo_id, provider) {
        memo_id -> Text,
        provider -> Text,
        external_id -> Text,
        url -> Nullable<Text>,
        published_at -> BigInt,
    }
}

pub fn list(path: &Path) -> Result<Vec<Publication>, PublishError> {
    let mut connection =
        database::open(path).map_err(|_| PublishError::Storage("could not open local storage"))?;
    let rows = memo_publications::table
        .select((memo_publications::memo_id, memo_publications::provider))
        .load::<(String, String)>(&mut connection)
        .map_err(|_| PublishError::Storage("could not read publications"))?;
    rows.into_iter()
        .map(|(memo_id, provider)| {
            let provider = match provider.as_str() {
                "telegram" => PublicationProvider::Telegram,
                "x" => PublicationProvider::X,
                _ => return Err(PublishError::Storage("a stored provider is unknown")),
            };
            Ok(Publication { memo_id, provider })
        })
        .collect()
}

pub(crate) fn exists(
    path: &Path,
    memo_id: &str,
    provider: PublicationProvider,
) -> Result<bool, PublishError> {
    let mut connection =
        database::open(path).map_err(|_| PublishError::Storage("could not open local storage"))?;
    diesel::select(diesel::dsl::exists(
        memo_publications::table.find((memo_id, provider.name())),
    ))
    .get_result(&mut connection)
    .map_err(|_| PublishError::Storage("could not read publications"))
}

pub(crate) fn record(path: &Path, memo_id: &str, post: &PublishedPost) -> Result<(), PublishError> {
    let published_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| PublishError::Storage("the system clock is invalid"))?
        .as_secs()
        .cast_signed();
    let mut connection =
        database::open(path).map_err(|_| PublishError::Storage("could not open local storage"))?;
    diesel::insert_into(memo_publications::table)
        .values((
            memo_publications::memo_id.eq(memo_id),
            memo_publications::provider.eq(post.provider.name()),
            memo_publications::external_id.eq(&post.external_id),
            memo_publications::url.eq(post.url.as_deref()),
            memo_publications::published_at.eq(published_at),
        ))
        .on_conflict_do_nothing()
        .execute(&mut connection)
        .map_err(|_| PublishError::Storage("the post was sent but could not be recorded"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records() {
        let directory =
            std::env::temp_dir().join(format!("vesper-publication-{}", uuid::Uuid::new_v4()));
        let path = directory.join(database::FILE_NAME);
        let post = PublishedPost {
            provider: PublicationProvider::Telegram,
            external_id: "7".to_owned(),
            url: None,
        };
        assert!(!exists(&path, "memo-1", PublicationProvider::Telegram).unwrap());
        record(&path, "memo-1", &post).unwrap();
        record(&path, "memo-1", &post).unwrap();

        assert!(exists(&path, "memo-1", PublicationProvider::Telegram).unwrap());
        assert!(!exists(&path, "memo-1", PublicationProvider::X).unwrap());
        let stored = list(&path).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].provider, PublicationProvider::Telegram);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
