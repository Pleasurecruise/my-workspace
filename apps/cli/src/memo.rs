use crate::print_json;
use cms_core::api::memos::{Update, Visibility};
use serde::Deserialize;
use serde_json::json;

const DEFAULT_LIMIT: usize = 10;
const MAX_LIMIT: usize = cms_core::api::memos::PAGE_SIZE;

enum MemoFlag {
    Pinned,
    Favorite,
    Archived,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageInput {
    cursor: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    sort_by_updated: bool,
    #[serde(default)]
    archived_only: bool,
    #[serde(default)]
    favorites_only: bool,
}

pub async fn run(action: &str, arguments: &[String]) -> Result<(), String> {
    match (action, arguments) {
        ("tags", []) => {
            let tags = cms_core::api::memos::tags()
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "tags": tags }))
        }
        ("list", limits) if limits.len() <= 1 => {
            let limit = match limits {
                [] => DEFAULT_LIMIT,
                [limit] => limit
                    .parse()
                    .map_err(|error| format!("invalid memo limit: {error}"))?,
                invalid_limits => {
                    return Err(format!(
                        "memo list accepts at most one limit, received {}",
                        invalid_limits.len()
                    ));
                }
            };
            if !(1..=MAX_LIMIT).contains(&limit) {
                return Err(format!("memo limit must be between 1 and {MAX_LIMIT}"));
            }
            let page = cms_core::api::memos::list(
                None,
                &cms_core::api::memos::ListFilters {
                    limit: Some(limit),
                    ..cms_core::api::memos::ListFilters::default()
                },
            )
            .await
            .map_err(|error| error.to_string())?;
            print_json(&json!({ "memos": page.memos, "nextCursor": page.next_cursor }))
        }
        ("page", [input]) => {
            let input: PageInput = serde_json::from_str(input)
                .map_err(|error| format!("invalid memo page JSON: {error}"))?;
            if input.archived_only && input.favorites_only {
                return Err(
                    "memo page cannot request archivedOnly and favoritesOnly together".to_owned(),
                );
            }
            if input
                .limit
                .is_some_and(|limit| !(1..=MAX_LIMIT).contains(&limit))
            {
                return Err(format!("memo page limit must be between 1 and {MAX_LIMIT}"));
            }
            let page = cms_core::api::memos::list(
                input.cursor,
                &cms_core::api::memos::ListFilters {
                    limit: input.limit,
                    search: input.search,
                    tags: input.tags,
                    sort_by_updated: input.sort_by_updated,
                    archived_only: input.archived_only,
                    favorites_only: input.favorites_only,
                },
            )
            .await
            .map_err(|error| error.to_string())?;
            print_json(&page)
        }
        ("search", query) if !query.is_empty() => {
            let query = query.join(" ").trim().to_lowercase();
            if query.is_empty() {
                return Err("memo search query is required".to_owned());
            }
            let result = cms_core::api::memos::search(&query)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&result)
        }
        ("create", content) if !content.is_empty() => {
            let memo = cms_core::api::memos::create(
                &content.join(" "),
                cms_core::api::memos::Visibility::Private,
            )
            .await
            .map_err(|error| error.to_string())?;
            print_json(&memo)
        }
        ("update", [id, content @ ..]) if !content.is_empty() => {
            update(
                id,
                Update {
                    content: Some(content.join(" ")),
                    visibility: None,
                    tags: None,
                    pinned: None,
                    favorite: None,
                    archived: None,
                },
            )
            .await
        }
        ("patch", [id, input]) => {
            let input: Update = serde_json::from_str(input)
                .map_err(|error| format!("invalid memo patch JSON: {error}"))?;
            if input.content.is_none()
                && input.visibility.is_none()
                && input.tags.is_none()
                && input.pinned.is_none()
                && input.favorite.is_none()
                && input.archived.is_none()
            {
                return Err("memo patch must set at least one field".to_owned());
            }
            update(id, input).await
        }
        ("visibility", [id, visibility]) => {
            let visibility = match visibility.as_str() {
                "public" => Visibility::Public,
                "private" => Visibility::Private,
                value => return Err(format!("invalid memo visibility: {value}")),
            };
            update(
                id,
                Update {
                    visibility: Some(visibility),
                    ..Update::default()
                },
            )
            .await
        }
        ("pin", [id]) => update_flag(id, MemoFlag::Pinned, true).await,
        ("unpin", [id]) => update_flag(id, MemoFlag::Pinned, false).await,
        ("favorite", [id]) => update_flag(id, MemoFlag::Favorite, true).await,
        ("unfavorite", [id]) => update_flag(id, MemoFlag::Favorite, false).await,
        ("archive", [id]) => update_flag(id, MemoFlag::Archived, true).await,
        ("restore", [id]) => update_flag(id, MemoFlag::Archived, false).await,
        ("delete", [id]) => {
            cms_core::api::memos::delete(id)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "id": id, "deleted": true }))
        }
        (invalid_action, invalid_arguments) => Err(format!(
            "invalid memo arguments: {action} {}; run `vesper help`",
            invalid_arguments.join(" "),
            action = invalid_action
        )),
    }
}

async fn update(id: &str, input: Update) -> Result<(), String> {
    let memo = cms_core::api::memos::update(id, &input)
        .await
        .map_err(|error| error.to_string())?;
    print_json(&memo)
}

async fn update_flag(id: &str, flag: MemoFlag, value: bool) -> Result<(), String> {
    let input = match flag {
        MemoFlag::Pinned => Update {
            pinned: Some(value),
            ..Update::default()
        },
        MemoFlag::Favorite => Update {
            favorite: Some(value),
            ..Update::default()
        },
        MemoFlag::Archived => Update {
            archived: Some(value),
            ..Update::default()
        },
    };
    update(id, input).await
}

#[cfg(test)]
mod tests {
    use super::run;

    #[tokio::test]
    async fn rejects_an_empty_patch_before_calling_the_api() {
        let error = run("patch", &["memo-id".to_owned(), "{}".to_owned()])
            .await
            .expect_err("an empty patch should fail");

        assert_eq!(error, "memo patch must set at least one field");
    }

    #[tokio::test]
    async fn rejects_an_invalid_visibility_before_calling_the_api() {
        let error = run("visibility", &["memo-id".to_owned(), "friends".to_owned()])
            .await
            .expect_err("an unsupported visibility should fail");

        assert_eq!(error, "invalid memo visibility: friends");
    }

    #[tokio::test]
    async fn rejects_conflicting_page_filters_before_calling_the_api() {
        let error = run(
            "page",
            &[r#"{"archivedOnly":true,"favoritesOnly":true}"#.to_owned()],
        )
        .await
        .expect_err("mutually exclusive filters should fail");

        assert_eq!(
            error,
            "memo page cannot request archivedOnly and favoritesOnly together"
        );
    }
}
