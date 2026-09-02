use crate::print_json;
use consumers::api::moment::{Create, Update, Upload};
use serde_json::json;
use std::path::Path;

pub async fn run(action: &str, arguments: &[String]) -> Result<(), String> {
    match (action, arguments) {
        ("tags", []) => {
            let tags = consumers::api::moment::tags()
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "tags": tags }))
        }
        ("list", []) => {
            let page = consumers::api::moment::list(None)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&page)
        }
        ("list", [cursor]) => {
            let page = consumers::api::moment::list(Some(cursor.to_owned()))
                .await
                .map_err(|error| error.to_string())?;
            print_json(&page)
        }
        ("search", query) if !query.is_empty() => {
            let query = query.join(" ");
            let photos = consumers::api::moment::search(&query)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "photos": photos }))
        }
        ("create", [input]) => {
            let input: Create = serde_json::from_str(input)
                .map_err(|error| format!("invalid moment create JSON: {error}"))?;
            let photo = consumers::api::moment::create(&input)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&photo)
        }
        ("upload-photo", [input, source_path]) => {
            let input: Upload = serde_json::from_str(input)
                .map_err(|error| format!("invalid Moment upload JSON: {error}"))?;
            let source = tokio::fs::read(source_path).await.map_err(|error| {
                format!("could not read Moment source image {source_path}: {error}")
            })?;
            let store = cms_core::r2::Store::from_credentials()
                .await
                .map_err(|error| error.to_string())?;
            let photo = consumers::api::moment::upload(&store, input, source)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&photo)
        }
        ("update", [id, input]) => {
            let input: Update = serde_json::from_str(input)
                .map_err(|error| format!("invalid moment update JSON: {error}"))?;
            let photo = consumers::api::moment::update(id, &input)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&photo)
        }
        ("delete", [id]) => {
            consumers::api::moment::delete(id)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "id": id, "deleted": true }))
        }
        ("upload", [key, path]) => {
            let store = cms_core::r2::Store::from_credentials()
                .await
                .map_err(|error| error.to_string())?;
            store
                .put_file(key, Path::new(path))
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "key": key, "uploaded": true }))
        }
        ("download", [key, path]) => {
            let store = cms_core::r2::Store::from_credentials()
                .await
                .map_err(|error| error.to_string())?;
            let bytes = store.get(key).await.map_err(|error| error.to_string())?;
            tokio::fs::write(path, bytes)
                .await
                .map_err(|error| format!("could not write Moment image {path}: {error}"))?;
            print_json(&json!({ "key": key, "path": path, "downloaded": true }))
        }
        ("remove-object", [key]) => {
            let store = cms_core::r2::Store::from_credentials()
                .await
                .map_err(|error| error.to_string())?;
            store.delete(key).await.map_err(|error| error.to_string())?;
            print_json(&json!({ "key": key, "removed": true }))
        }
        (invalid_action, invalid_arguments) => Err(format!(
            "invalid moment arguments: {action} {}; run `vesper help`",
            invalid_arguments.join(" "),
            action = invalid_action
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    #[tokio::test]
    async fn rejects_bad_upload_json() {
        let error = run(
            "upload-photo",
            &["not-json".to_owned(), "source.heic".to_owned()],
        )
        .await
        .expect_err("invalid upload JSON should fail");

        assert!(error.starts_with("invalid Moment upload JSON:"));
    }
}
