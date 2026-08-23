use crate::print_json;
use serde_json::json;

pub async fn run(action: &str, arguments: &[String]) -> Result<(), String> {
    let store = cms_core::todo::Store::shared().map_err(|error| error.to_string())?;
    let date = cms_core::todo::current_date().map_err(|error| error.to_string())?;
    run_with_store(&store, &date, action, arguments).await
}

async fn run_with_store(
    store: &cms_core::todo::Store,
    date: &str,
    action: &str,
    arguments: &[String],
) -> Result<(), String> {
    match (action, arguments) {
        ("list", []) => print_json(&store.list(date).await.map_err(|error| error.to_string())?),
        ("get", [id]) => print_json(
            &store
                .get(date, id)
                .await
                .map_err(|error| error.to_string())?,
        ),
        ("create", text) if !text.is_empty() => print_json(
            &store
                .create(date, &text.join(" "))
                .await
                .map_err(|error| error.to_string())?,
        ),
        ("update", [id, text @ ..]) if !text.is_empty() => print_json(
            &store
                .update(date, id, &text.join(" "))
                .await
                .map_err(|error| error.to_string())?,
        ),
        ("complete", [id]) => print_json(
            &store
                .set_completed(date, id, true)
                .await
                .map_err(|error| error.to_string())?,
        ),
        ("reopen", [id]) => print_json(
            &store
                .set_completed(date, id, false)
                .await
                .map_err(|error| error.to_string())?,
        ),
        ("delete", [id]) => {
            let todos = store
                .delete(date, id)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "id": id, "deleted": true, "todos": todos }))
        }
        (invalid_action, invalid_arguments) => Err(format!(
            "invalid todo arguments: {action} {}; run `vesper help`",
            invalid_arguments.join(" "),
            action = invalid_action
        )),
    }
}

#[cfg(test)]
#[path = "../tests/unit/todo.rs"]
mod tests;
