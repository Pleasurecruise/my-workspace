use crate::CommandResponse;
use tauri::{Emitter, Manager};

#[tauri::command]
pub(crate) async fn read_todos(
    date: String,
    app: tauri::AppHandle,
) -> CommandResponse<todo_core::List> {
    match app.state::<todo_core::Store>().sync_calendar(&date).await {
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
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<todo_core::List> {
    match app.state::<todo_core::Store>().create(&date, &text).await {
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
