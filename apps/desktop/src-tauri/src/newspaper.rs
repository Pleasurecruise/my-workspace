use crate::CommandResponse;
use tauri::Manager;

#[tauri::command]
pub(crate) async fn read_newspaper(app: tauri::AppHandle) -> CommandResponse<newspaper::Daily> {
    app.state::<newspaper::Reader>().read().await.into()
}
