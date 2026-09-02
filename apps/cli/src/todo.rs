use crate::print_json;
use serde_json::json;
use std::path::Path;

pub async fn run(
    action: &str,
    arguments: &[String],
    selected_date: Option<&str>,
) -> Result<(), String> {
    let store = todo_core::Store::shared().map_err(|error| error.to_string())?;
    let date = match selected_date {
        Some(date) => {
            todo_core::validate_date(date).map_err(|error| error.to_string())?;
            date.to_owned()
        }
        None => todo_core::current_date().map_err(|error| error.to_string())?,
    };
    run_with_store(&store, &date, action, arguments).await
}

async fn run_with_store(
    store: &todo_core::Store,
    date: &str,
    action: &str,
    arguments: &[String],
) -> Result<(), String> {
    match (action, arguments) {
        ("list", []) => print_json(
            &store
                .sync_schedule(date)
                .await
                .map_err(|error| error.to_string())?,
        ),
        ("schedule-path", []) => print_json(&json!({ "directory": store.schedule_directory() })),
        ("sync-ics", []) => print_json(
            &store
                .sync_schedule(date)
                .await
                .map_err(|error| error.to_string())?,
        ),
        ("import-ics", sources) if !sources.is_empty() => {
            let sources: Vec<_> = sources
                .iter()
                .map(|source| Path::new(source).to_owned())
                .collect();
            let installed = store
                .import_schedules(&sources)
                .await
                .map_err(|error| error.to_string())?;
            let todos = store
                .sync_schedule(date)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({
                "directory": store.schedule_directory(),
                "installed": installed,
                "todos": todos,
            }))
        }
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
