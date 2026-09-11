use crate::CommandResponse;
use tauri::{Emitter, Manager};

#[tauri::command]
pub(crate) async fn read_expenses(
    date: String,
    app: tauri::AppHandle,
) -> CommandResponse<ledger::Snapshot> {
    match app.state::<ledger::Store>().read(&date).await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn create_expense(
    date: String,
    amount: String,
    category: String,
    description: Option<String>,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<ledger::Snapshot> {
    changed(
        app.state::<ledger::Store>()
            .create(&date, &amount, &category, description.as_deref())
            .await,
        &app,
        &window,
    )
}

#[tauri::command]
pub(crate) async fn update_expense(
    date: String,
    id: String,
    amount: String,
    category: String,
    description: Option<String>,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<ledger::Snapshot> {
    changed(
        app.state::<ledger::Store>()
            .update(
                &date,
                &id,
                &amount,
                &category,
                Some(description.as_deref().unwrap_or("")),
            )
            .await,
        &app,
        &window,
    )
}

#[tauri::command]
pub(crate) async fn delete_expense(
    date: String,
    id: String,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<ledger::Snapshot> {
    changed(
        app.state::<ledger::Store>().delete(&date, &id).await,
        &app,
        &window,
    )
}

fn changed(
    result: Result<ledger::Snapshot, ledger::Error>,
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
) -> CommandResponse<ledger::Snapshot> {
    match result {
        Ok(data) => {
            let _ = app.emit_filter("expenses-updated", &data.date, |target| match target {
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
