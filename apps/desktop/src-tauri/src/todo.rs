use crate::CommandResponse;
use tauri::Manager;

#[tauri::command]
pub(crate) async fn read_todos(
    date: String,
    app: tauri::AppHandle,
) -> CommandResponse<cms_core::todo::List> {
    match app.state::<cms_core::todo::Store>().list(&date).await {
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
) -> CommandResponse<cms_core::todo::List> {
    match app
        .state::<cms_core::todo::Store>()
        .create(&date, &text)
        .await
    {
        Ok(data) => CommandResponse::Ready { data },
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
) -> CommandResponse<cms_core::todo::List> {
    match app
        .state::<cms_core::todo::Store>()
        .set_completed(&date, &id, completed)
        .await
    {
        Ok(data) => CommandResponse::Ready { data },
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
) -> CommandResponse<cms_core::todo::List> {
    match app
        .state::<cms_core::todo::Store>()
        .delete(&date, &id)
        .await
    {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}
