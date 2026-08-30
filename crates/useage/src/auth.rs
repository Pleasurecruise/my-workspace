use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize)]
struct AuthEntry {
    #[serde(rename = "type")]
    kind: String,
    key: Option<String>,
}

#[derive(Deserialize)]
struct ModelsFile {
    providers: HashMap<String, ModelProvider>,
}

#[derive(Deserialize)]
struct ModelProvider {
    #[serde(rename = "apiKey")]
    api_key: Option<String>,
}

pub(crate) async fn api_key(provider_id: &str) -> Result<String, String> {
    let agent_directory = std::env::var_os("PI_CODING_AGENT_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".pi/agent")))
        .ok_or_else(|| "Could not locate the pi agent directory".to_owned())?;

    let auth_path = agent_directory.join("auth.json");
    match tokio::fs::read_to_string(&auth_path).await {
        Ok(content) => {
            let entries: HashMap<String, AuthEntry> = serde_json::from_str(&content)
                .map_err(|error| format!("Could not parse {}: {error}", auth_path.display()))?;
            if let Some(entry) = find_provider(&entries, provider_id) {
                if entry.kind != "api_key" {
                    return Err(format!("{provider_id} is not an API key in pi auth.json"));
                }
                return entry
                    .key
                    .clone()
                    .filter(|key| !key.trim().is_empty())
                    .ok_or_else(|| {
                        format!("{provider_id} does not contain a key in pi auth.json")
                    });
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!("Could not read {}: {error}", auth_path.display()));
        }
    }

    let models_path = agent_directory.join("models.json");
    let content = tokio::fs::read_to_string(&models_path)
        .await
        .map_err(|error| format!("Could not read {}: {error}", models_path.display()))?;
    let models: ModelsFile = serde_json::from_str(&content)
        .map_err(|error| format!("Could not parse {}: {error}", models_path.display()))?;
    let provider = find_provider(&models.providers, provider_id)
        .ok_or_else(|| format!("{provider_id} is not configured in pi"))?;
    provider
        .api_key
        .clone()
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| format!("{provider_id} does not contain an apiKey in pi models.json"))
}

fn find_provider<'a, T>(providers: &'a HashMap<String, T>, id: &str) -> Option<&'a T> {
    providers
        .iter()
        .find_map(|(name, provider)| name.eq_ignore_ascii_case(id).then_some(provider))
}
