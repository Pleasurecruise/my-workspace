use crate::UgosError;
use crate::auth;
use crate::tls::{self, CertFingerprint};
use crate::types::ApiResponse;
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub(crate) struct Client {
    http: reqwest::Client,
    base_url: String,
    token: String,
    client_id: String,
    client_version: String,
}

impl Client {
    pub(crate) async fn connect(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        fingerprint: CertFingerprint,
    ) -> Result<Self, UgosError> {
        let http = tls::http_client(fingerprint)?;
        let base_url = format!("https://{host}:{port}/ugreen");
        let desktop = http
            .get(format!("https://{host}:{port}/desktop/?os=ugospro"))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let client_version = parse_client_version(&desktop)?;
        let device_id = fingerprint.to_hex()[..32].to_owned();
        let mut client_id = uuid::Uuid::new_v4().to_string();
        client_id.truncate(client_id.len() - 12);
        client_id.push_str("WEB");
        let token = auth::login(
            &http,
            &base_url,
            username,
            password,
            &client_id,
            &device_id,
            &client_version,
        )
        .await?;
        Ok(Self {
            http,
            base_url,
            token,
            client_id,
            client_version,
        })
    }

    pub(crate) async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, UgosError> {
        let body = self
            .http
            .get(format!("{}/v1/{path}", self.base_url))
            .query(&[("token", self.token.as_str())])
            .header("Accept", "application/json, text/plain, */*")
            .header("UG-Agent", "PC/WEB")
            .header("Client-Id", &self.client_id)
            .header("Client-Version", &self.client_version)
            .header("Cache-Control", "no-cache")
            .header("X-Specify-Language", "en-US")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        ApiResponse::decode(&body, path)
    }
}

fn parse_client_version(desktop: &str) -> Result<String, UgosError> {
    let version_source = desktop
        .split_once("window.clientNumberVersion=")
        .map(|(_, source)| source)
        .ok_or_else(|| UgosError::Decode {
            endpoint: "desktop".to_owned(),
            message: "clientNumberVersion is missing".to_owned(),
        })?;
    let version: String = version_source
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if version.is_empty() {
        return Err(UgosError::Decode {
            endpoint: "desktop".to_owned(),
            message: "clientNumberVersion is empty".to_owned(),
        });
    }
    Ok(version)
}

#[cfg(test)]
#[path = "../tests/unit/client.rs"]
mod tests;
