use crate::print_json;
use serde_json::json;

const DEFAULT_LIMIT: usize = 10;
const MAX_LIMIT: usize = 20;

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
            let store = cms_core::r2::Store::from_credentials()
                .await
                .map_err(|error| error.to_string())?;
            let mut page = cms_core::api::memos::list(&store, None)
                .await
                .map_err(|error| error.to_string())?;
            page.memos.truncate(limit);
            print_json(&json!({ "memos": page.memos, "nextCursor": page.next_cursor }))
        }
        ("search", query) if !query.is_empty() => {
            let query = query.join(" ").trim().to_lowercase();
            if query.is_empty() {
                return Err("memo search query is required".to_owned());
            }
            let store = cms_core::r2::Store::from_credentials()
                .await
                .map_err(|error| error.to_string())?;
            let result = cms_core::api::memos::search(&store, &query)
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
            let memo = cms_core::api::memos::update(
                id,
                &cms_core::api::memos::Update {
                    content: Some(content.join(" ")),
                    visibility: None,
                    tags: None,
                    pinned: None,
                    favorite: None,
                    archived: None,
                },
            )
            .await
            .map_err(|error| error.to_string())?;
            print_json(&memo)
        }
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
