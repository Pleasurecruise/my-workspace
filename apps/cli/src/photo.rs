use crate::print_json;
use consumers::api::moment::{Create, Update, Upload};
use serde_json::json;
use std::path::Path;

pub async fn run(action: &str, arguments: &[String]) -> Result<(), String> {
    match (action, arguments) {
        ("get", [id]) => {
            let photo = consumers::api::moment::get(id)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&photo)
        }
        ("query", input) => {
            let input = crate::read_input(input).await?;
            let query = serde_json::from_str(&input)
                .map_err(|error| format!("invalid photo query: {error}"))?;
            let photos = consumers::api::moment::query(&query)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "photos": photos }))
        }
        ("tags", []) => {
            let tags = consumers::api::moment::tags()
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "tags": tags }))
        }
        ("search", query) if !query.is_empty() => {
            let query = query.join(" ");
            let photos = consumers::api::moment::search(&query)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "photos": photos }))
        }
        ("register", input) => {
            let input = crate::read_input(input).await?;
            let input: Create = serde_json::from_str(&input)
                .map_err(|error| format!("invalid photo create JSON: {error}"))?;
            let photo = consumers::api::moment::create(&input)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&photo)
        }
        ("upload", arguments) => {
            let (input, source_path) = match arguments {
                [flag, _, source] if flag == "--file" => (&arguments[..2], source),
                [input, source] if !input.starts_with("--") => (&arguments[..1], source),
                _ => {
                    return Err(
                        "expected photo upload <source-image> [--metadata <path>]; run `vesper help`"
                            .to_owned(),
                    );
                }
            };
            let input = crate::read_input(input).await?;
            let mut metadata: serde_json::Value = serde_json::from_str(&input)
                .map_err(|error| format!("invalid photo upload JSON: {error}"))?;
            if metadata.get("title").is_none_or(serde_json::Value::is_null) {
                let title = Path::new(source_path)
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| "photo title is required for this file name".to_owned())?;
                let fields = metadata
                    .as_object_mut()
                    .ok_or_else(|| "photo metadata must be a JSON object".to_owned())?;
                fields.insert("title".into(), title.into());
            }
            let input: Upload = serde_json::from_value(metadata)
                .map_err(|error| format!("invalid photo upload JSON: {error}"))?;
            input.validate().map_err(|error| error.to_string())?;
            let source = tokio::fs::read(source_path).await.map_err(|error| {
                format!("could not read photo source image {source_path}: {error}")
            })?;
            let store = cms::r2::Store::from_credentials()
                .await
                .map_err(|error| error.to_string())?;
            let photo = consumers::api::moment::upload(
                &store,
                input,
                source,
                consumers::api::moment::MetadataPolicy::SourceDefaults,
            )
            .await
            .map_err(|error| error.to_string())?;
            print_json(&photo)
        }
        ("update", [id, input @ ..]) => {
            let input = crate::read_input(input).await?;
            let input: Update = serde_json::from_str(&input)
                .map_err(|error| format!("invalid photo update JSON: {error}"))?;
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
        ("object-put", [key, path]) => {
            let store = cms::r2::Store::from_credentials()
                .await
                .map_err(|error| error.to_string())?;
            store
                .put_file(key, Path::new(path))
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "key": key, "uploaded": true }))
        }
        ("object-get", [key, path]) => {
            let store = cms::r2::Store::from_credentials()
                .await
                .map_err(|error| error.to_string())?;
            let bytes = store.get(key).await.map_err(|error| error.to_string())?;
            tokio::fs::write(path, bytes)
                .await
                .map_err(|error| format!("could not write photo image {path}: {error}"))?;
            print_json(&json!({ "key": key, "path": path, "downloaded": true }))
        }
        ("object-delete", [key]) => {
            let store = cms::r2::Store::from_credentials()
                .await
                .map_err(|error| error.to_string())?;
            store.delete(key).await.map_err(|error| error.to_string())?;
            print_json(&json!({ "key": key, "removed": true }))
        }
        (invalid_action, invalid_arguments) => Err(format!(
            "invalid photo arguments: {action} {}; run `vesper help`",
            invalid_arguments.join(" "),
            action = invalid_action
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    #[tokio::test]
    async fn rejects_invalid_upload_metadata() {
        let error = run("upload", &["not-json".to_owned(), "source.heic".to_owned()])
            .await
            .expect_err("invalid upload JSON should fail");

        assert!(error.starts_with("invalid photo upload JSON:"));
        for metadata in [
            serde_json::json!({"title": ""}),
            serde_json::json!({"title": "Photo", "tags": [""]}),
            serde_json::json!({"title": "Photo", "geo": {"lat": 91, "lng": 0}}),
        ] {
            let error = run(
                "upload",
                &[metadata.to_string(), "missing-image.png".into()],
            )
            .await
            .unwrap_err();
            assert_eq!(
                error,
                "consumer API returned invalid data: photo upload metadata is invalid"
            );
        }
    }
}
