use crate::CommandResponse;
use tauri::{Emitter, Manager};

#[tauri::command]
pub(crate) async fn read_todos(
    date: String,
    refresh: Option<bool>,
    app: tauri::AppHandle,
) -> CommandResponse<todo_core::List> {
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
pub(crate) async fn read_check_in(
    id: String,
    app: tauri::AppHandle,
) -> CommandResponse<todo_core::CheckIn> {
    match app.state::<todo_core::Store>().read_check_in(&id).await {
        Ok(data) => CommandResponse::Ready { data },
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
    app: tauri::AppHandle,
) -> CommandResponse<Vec<todo_core::CheckIn>> {
    match app.state::<todo_core::Store>().read_check_ins(ids).await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}
