use super::Content;
use crate::{CommandError, CommandResponse};
use consumers::api::memos::{self, MemoView, TagCount, Update, Visibility};
use consumers::view::Channel;
use tauri::Manager;

async fn invalidate(app: &tauri::AppHandle) {
    app.state::<Content>().invalidate_view(Channel::Memos).await;
}

#[tauri::command]
pub(crate) async fn read_memo_tags() -> CommandResponse<Vec<TagCount>> {
    memos::tags().await.into()
}

#[tauri::command]
pub(crate) async fn create_memo(
    content: String,
    visibility: Visibility,
    app: tauri::AppHandle,
) -> CommandResponse<MemoView> {
    let result = async {
        let memo = memos::create(&content, visibility).await?;
        invalidate(&app).await;
        Ok::<_, CommandError>(memo)
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn import_x_memo(
    url: String,
    visibility: Visibility,
    app: tauri::AppHandle,
) -> CommandResponse<MemoView> {
    let result = async {
        let memo = memos::import_x(&url, visibility).await?;
        invalidate(&app).await;
        Ok::<_, CommandError>(memo)
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn update_memo(
    id: String,
    input: Update,
    app: tauri::AppHandle,
) -> CommandResponse<MemoView> {
    let result = async {
        let memo = memos::update(&id, &input).await?;
        invalidate(&app).await;
        Ok::<_, CommandError>(memo)
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn delete_memo(id: String, app: tauri::AppHandle) -> CommandResponse<String> {
    let result = async {
        memos::delete(&id).await?;
        invalidate(&app).await;
        Ok::<_, CommandError>(id)
    };
    result.await.into()
}
