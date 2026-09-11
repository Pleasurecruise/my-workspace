use crate::print_json;
use serde_json::json;
use std::path::Path;

pub async fn run(
    action: &str,
    arguments: &[String],
    selected_date: Option<&str>,
) -> Result<(), String> {
    if action == "notion" {
        return match arguments {
            [operation] if operation == "status" => {
                match vesper_credentials::notion_calendar().map_err(|error| error.to_string())? {
                    vesper_credentials::Stored::Missing => {
                        print_json(&json!({ "configured": false, "viewUrl": null }))
                    }
                    vesper_credentials::Stored::Ready(configuration) => print_json(
                        &json!({ "configured": true, "viewUrl": configuration.view_url }),
                    ),
                }
            }
            [operation, view_url] if operation == "connect" => {
                todo_core::Store::shared()
                    .map_err(|error| error.to_string())?
                    .configure_notion(vesper_credentials::NotionCalendar {
                        view_url: view_url.clone(),
                    })
                    .await
                    .map_err(|error| error.to_string())?;
                print_json(&json!({ "configured": true }))
            }
            [operation] if operation == "disconnect" => {
                todo_core::Store::shared()
                    .map_err(|error| error.to_string())?
                    .configure_notion(vesper_credentials::NotionCalendar {
                        view_url: String::new(),
                    })
                    .await
                    .map_err(|error| error.to_string())?;
                print_json(&json!({ "configured": false }))
            }
            _ => Err("expected todo notion status | connect <view-url> | disconnect".into()),
        };
    }
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
        ("check-ins", ids) if !ids.is_empty() => print_json(
            &store
                .read_check_ins(ids.to_vec(), date)
                .await
                .map_err(|error| error.to_string())?,
        ),
        ("check-in" | "undo-check-in", [id]) => print_json(
            &store
                .set_check_in(id, date, action == "check-in")
                .await
                .map_err(|error| error.to_string())?,
        ),
        ("list", []) => print_json(
            &store
                .sync_calendar(date)
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
        ("database-path", []) => print_json(&json!({ "database": store.database_path() })),
        ("sync", []) => print_json(
            &store
                .sync_calendar(date)
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
                .create(date, &text.join(" "), None)
                .await
                .map_err(|error| error.to_string())?,
        ),
        ("update", [id, text @ ..]) if !text.is_empty() => print_json(
            &store
                .update(date, id, &text.join(" "), None)
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
