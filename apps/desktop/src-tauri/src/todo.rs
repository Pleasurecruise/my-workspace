use crate::CommandResponse;
use tauri::{Emitter, Manager};

pub(crate) async fn roll_over(app: &tauri::AppHandle, date: &str) -> Result<(), todo_core::Error> {
    let changed = app.state::<todo_core::Store>().roll_over(date).await?;
    for date in changed {
        if let Err(error) = app.emit("todo-updated", &date) {
            tracing::warn!(%error, "failed to notify Todo rollover");
        }
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn read_todos(
    date: String,
    refresh: Option<bool>,
    app: tauri::AppHandle,
) -> CommandResponse<todo_core::List> {
    let today = match todo_core::current_date() {
        Ok(date) => date,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    if let Err(error) = roll_over(&app, &today).await {
        return CommandResponse::Failed {
            message: error.to_string(),
        };
    }
    match app
        .state::<todo_core::Store>()
        .read_calendar(&date, refresh.unwrap_or(false))
        .await
    {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn add_todo(
    date: String,
    text: String,
    description: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<todo_core::List> {
    match app
        .state::<todo_core::Store>()
        .create(&date, &text, Some(&description))
        .await
    {
        Ok(data) => {
            let _ = app.emit_filter("todo-updated", &data.date, |target| match target {
                tauri::EventTarget::WebviewWindow { label } => label != window.label(),
                _ => false,
            });
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn set_todo_completed(
    date: String,
    id: String,
    completed: bool,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<todo_core::List> {
    match app
        .state::<todo_core::Store>()
        .set_completed(&date, &id, completed)
        .await
    {
        Ok(data) => {
            let _ = app.emit_filter("todo-updated", &data.date, |target| match target {
                tauri::EventTarget::WebviewWindow { label } => label != window.label(),
                _ => false,
            });
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn set_todo_rollover(
    date: String,
    id: String,
    rollover: bool,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<todo_core::List> {
    match app
        .state::<todo_core::Store>()
        .set_rollover(&date, &id, rollover)
        .await
    {
        Ok(data) => {
            let _ = app.emit_filter("todo-updated", &data.date, |target| match target {
                tauri::EventTarget::WebviewWindow { label } => label != window.label(),
                _ => false,
            });
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn reorder_todos(
    date: String,
    ids: Vec<String>,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<todo_core::List> {
    match app.state::<todo_core::Store>().reorder(&date, ids).await {
        Ok(data) => {
            let _ = app.emit_filter("todo-updated", &data.date, |target| match target {
                tauri::EventTarget::WebviewWindow { label } => label != window.label(),
                _ => false,
            });
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn delete_todo(
    date: String,
    id: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<todo_core::List> {
    match app.state::<todo_core::Store>().delete(&date, &id).await {
        Ok(data) => {
            let _ = app.emit_filter("todo-updated", &data.date, |target| match target {
                tauri::EventTarget::WebviewWindow { label } => label != window.label(),
                _ => false,
            });
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn update_todo(
    date: String,
    id: String,
    text: String,
    description: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<todo_core::List> {
    match app
        .state::<todo_core::Store>()
        .update(&date, &id, &text, Some(&description))
        .await
    {
        Ok(data) => {
            let _ = app.emit_filter("todo-updated", &data.date, |target| match target {
                tauri::EventTarget::WebviewWindow { label } => label != window.label(),
                _ => false,
            });
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn set_check_in(
    id: String,
    date: String,
    completed: bool,
    app: tauri::AppHandle,
) -> CommandResponse<todo_core::CheckIn> {
    match app
        .state::<todo_core::Store>()
        .set_check_in(&id, &date, completed)
        .await
    {
        Ok(data) => {
            let _ = app.emit("check-in-updated", &id);
            CommandResponse::Ready { data }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn read_check_ins(
    ids: Vec<String>,
    date: String,
    app: tauri::AppHandle,
) -> CommandResponse<Vec<todo_core::CheckIn>> {
    match app
        .state::<todo_core::Store>()
        .read_check_ins(ids, &date)
        .await
    {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) fn read_planner_date() -> CommandResponse<String> {
    match todo_core::current_date() {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}
