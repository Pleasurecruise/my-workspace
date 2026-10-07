use super::Content;
use crate::{CommandError, CommandResponse};
use consumers::api::moment::{self, MetadataPolicy, Photo, PhotoMetadata, Update, Upload};
use consumers::view::Channel;
use tauri::Manager;

#[tauri::command]
pub(crate) async fn read_moment_tags() -> CommandResponse<Vec<String>> {
    moment::tags().await.into()
}

#[tauri::command]
pub(crate) async fn read_photo_metadata(source: Vec<u8>) -> CommandResponse<PhotoMetadata> {
    moment::read_metadata(source).await.into()
}

#[tauri::command]
pub(crate) async fn create_photo(
    input: Upload,
    source: Vec<u8>,
    app: tauri::AppHandle,
) -> CommandResponse<Photo> {
    let state = app.state::<Content>();
    let result = async {
        input.validate()?;
        let repository = state.repository().await?;
        let photo =
            moment::upload(repository.store(), input, source, MetadataPolicy::Reviewed).await?;
        state.invalidate_view(Channel::Moment).await;
        Ok::<_, CommandError>(photo)
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn update_photo(
    id: String,
    input: Update,
    app: tauri::AppHandle,
) -> CommandResponse<Photo> {
    let result = async {
        let photo = moment::update(&id, &input).await?;
        app.state::<Content>()
            .invalidate_view(Channel::Moment)
            .await;
        Ok::<_, CommandError>(photo)
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn delete_photo(id: String, app: tauri::AppHandle) -> CommandResponse<String> {
    let state = app.state::<Content>();
    let result = async {
        moment::delete(&id).await?;
        state.invalidate_view(Channel::Moment).await;
        Ok::<_, CommandError>(id)
    };
    result.await.into()
}
