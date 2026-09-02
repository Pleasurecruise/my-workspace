use crate::print_json;
use consumers::api::knowledge::{Create, DocumentUpdate, DraftUpdate, VisibilityUpdate};
use serde_json::json;

pub async fn run(action: &str, arguments: &[String]) -> Result<(), String> {
    match (action, arguments) {
        ("list", []) => {
            let page = consumers::api::knowledge::list(None)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&page)
        }
        ("list", [cursor]) => {
            let page = consumers::api::knowledge::list(Some(cursor.to_owned()))
                .await
                .map_err(|error| error.to_string())?;
            print_json(&page)
        }
        ("get", [id]) => {
            let article = consumers::api::knowledge::get(id)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&article)
        }
        ("create", [input]) => {
            let input: Create = serde_json::from_str(input)
                .map_err(|error| format!("invalid knowledge create JSON: {error}"))?;
            let article = consumers::api::knowledge::create(&input)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&article)
        }
        ("update-draft", [id, input]) => {
            let input: DraftUpdate = serde_json::from_str(input)
                .map_err(|error| format!("invalid knowledge draft JSON: {error}"))?;
            let article = consumers::api::knowledge::update_draft(id, &input)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&article)
        }
        ("update-documents", [id, input]) => {
            let input: DocumentUpdate = serde_json::from_str(input)
                .map_err(|error| format!("invalid knowledge documents JSON: {error}"))?;
            let article = consumers::api::knowledge::update_documents(id, &input)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&article)
        }
        ("visibility", [id, input]) => {
            let input: VisibilityUpdate = serde_json::from_str(input)
                .map_err(|error| format!("invalid knowledge visibility JSON: {error}"))?;
            let article = consumers::api::knowledge::set_visibility(id, &input)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&article)
        }
        ("delete", [id, expected_hash]) => {
            consumers::api::knowledge::delete(id, expected_hash)
                .await
                .map_err(|error| error.to_string())?;
            print_json(&json!({ "id": id, "deleted": true }))
        }
        (invalid_action, invalid_arguments) => Err(format!(
            "invalid knowledge arguments: {action} {}; run `vesper help`",
            invalid_arguments.join(" "),
            action = invalid_action
        )),
    }
}
