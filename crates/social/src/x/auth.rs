use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::PublishError;

const AUTHORIZE_ENDPOINT: &str = "https://x.com/i/oauth2/authorize";
const TOKEN_ENDPOINT: &str = "https://api.x.com/2/oauth2/token";
const REDIRECT_URI: &str = "http://127.0.0.1:8792/callback";
const REDIRECT_PATH: &str = "/callback";
const SCOPES: &str = "tweet.read tweet.write users.read offline.access";
const CLIENT_ID: Option<&str> = option_env!("VESPER_X_CLIENT_ID");
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MAX_CALLBACK_LINE_BYTES: usize = 8 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct Authorization {
    pub url: String,
    client_id: String,
    verifier: String,
    state: String,
    listener: TcpListener,
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

pub async fn authorization() -> Result<Authorization, PublishError> {
    let client_id = CLIENT_ID
        .filter(|client_id| !client_id.is_empty() && client_id.len() <= 256)
        .ok_or(PublishError::XAuthorization(
            "the X OAuth Client ID is not configured in this build",
        ))?;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 8792))
        .await
        .map_err(|_| PublishError::XAuthorization("could not listen for the local callback"))?;
    let verifier = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = uuid::Uuid::new_v4().simple().to_string();
    let url = reqwest::Url::parse_with_params(
        AUTHORIZE_ENDPOINT,
        [
            ("response_type", "code"),
            ("client_id", client_id),
            ("redirect_uri", REDIRECT_URI),
            ("scope", SCOPES),
            ("state", state.as_str()),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ],
    )
    .map_err(|_| PublishError::XAuthorization("could not build the authorization URL"))?;
    Ok(Authorization {
        url: url.into(),
        client_id: client_id.to_owned(),
        verifier,
        state,
        listener,
    })
}

pub async fn authenticate(
    authorization: Authorization,
) -> Result<vesper_credentials::XCredentials, PublishError> {
    let client_id = authorization.client_id.clone();
    let verifier = authorization.verifier.clone();
    let code = wait_for_code(authorization).await?;
    let response = client()?
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("code", code.as_str()),
            ("grant_type", "authorization_code"),
            ("client_id", client_id.as_str()),
            ("redirect_uri", REDIRECT_URI),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|_| PublishError::Request("X authorization"))?;
    if !response.status().is_success() {
        return Err(PublishError::Status {
            provider: "X authorization",
            status: response.status(),
        });
    }
    let token: TokenResponse = response
        .json()
        .await
        .map_err(|_| PublishError::Protocol("X authorization"))?;
    let refresh_token = token.refresh_token.ok_or(PublishError::XAuthorization(
        "X did not return a refresh token",
    ))?;
    Ok(vesper_credentials::XCredentials {
        client_id,
        access_token: token.access_token,
        refresh_token,
        expires_at: now().saturating_add(token.expires_in),
    })
}

pub async fn refresh(
    credentials: &vesper_credentials::XCredentials,
) -> Result<vesper_credentials::XCredentials, PublishError> {
    let response = client()?
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("refresh_token", credentials.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
            ("client_id", credentials.client_id.as_str()),
        ])
        .send()
        .await
        .map_err(|_| PublishError::Request("X authorization"))?;
    if !response.status().is_success() {
        return Err(PublishError::Status {
            provider: "X authorization",
            status: response.status(),
        });
    }
    let token: TokenResponse = response
        .json()
        .await
        .map_err(|_| PublishError::Protocol("X authorization"))?;
    Ok(vesper_credentials::XCredentials {
        client_id: credentials.client_id.clone(),
        access_token: token.access_token,
        refresh_token: token
            .refresh_token
            .unwrap_or_else(|| credentials.refresh_token.clone()),
        expires_at: now().saturating_add(token.expires_in),
    })
}

pub fn expires_soon(credentials: &vesper_credentials::XCredentials) -> bool {
    credentials.expires_at <= now().saturating_add(60)
}

fn client() -> Result<reqwest::Client, PublishError> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|_| PublishError::Request("X authorization"))
}

async fn wait_for_code(authorization: Authorization) -> Result<String, PublishError> {
    let callback = async {
        loop {
            let (mut stream, _) = authorization
                .listener
                .accept()
                .await
                .map_err(|_| PublishError::XAuthorization("the local callback failed"))?;
            let mut line = Vec::new();
            {
                let mut reader =
                    BufReader::new(&mut stream).take((MAX_CALLBACK_LINE_BYTES + 1) as u64);
                reader
                    .read_until(b'\n', &mut line)
                    .await
                    .map_err(|_| PublishError::XAuthorization("the local callback was invalid"))?;
            }
            let line = if line.len() <= MAX_CALLBACK_LINE_BYTES && line.ends_with(b"\n") {
                std::str::from_utf8(&line).unwrap_or_default()
            } else {
                ""
            };
            let result = parse_callback(line, &authorization.state);
            let (status, message) = match &result {
                Ok(_) => ("200 OK", "X is connected. You can close this tab."),
                Err(_) => (
                    "400 Bad Request",
                    "X authorization could not be completed. Return to Vesper and try again.",
                ),
            };
            let body = format!(
                "<!doctype html><meta charset=\"utf-8\"><title>Vesper X</title><p>{message}</p>"
            );
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            if callback_url(line).is_some_and(|url| url.path() == REDIRECT_PATH) {
                return result;
            }
        }
    };
    tokio::time::timeout(CALLBACK_TIMEOUT, callback)
        .await
        .map_err(|_| PublishError::XAuthorization("authorization timed out"))?
}

fn parse_callback(line: &str, expected_state: &str) -> Result<String, PublishError> {
    let url = callback_url(line).ok_or(PublishError::XAuthorization("the callback was invalid"))?;
    if url.path() != REDIRECT_PATH {
        return Err(PublishError::XAuthorization(
            "the callback path was invalid",
        ));
    }
    let mut code = None;
    let mut state = None;
    let mut denied = false;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            "error" => denied = true,
            _ => {}
        }
    }
    if denied {
        return Err(PublishError::XAuthorization("access was denied"));
    }
    if state.as_deref() != Some(expected_state) {
        return Err(PublishError::XAuthorization(
            "the callback state did not match",
        ));
    }
    code.ok_or(PublishError::XAuthorization(
        "the callback did not include a code",
    ))
}

fn callback_url(line: &str) -> Option<reqwest::Url> {
    let target = line.split_whitespace().nth(1)?;
    reqwest::Url::parse(&format!("http://127.0.0.1{target}")).ok()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::parse_callback;

    #[test]
    fn validates_callback_state() {
        let line = "GET /callback?code=ready&state=expected HTTP/1.1";
        assert_eq!(parse_callback(line, "expected").unwrap(), "ready");
        assert!(parse_callback(line, "different").is_err());
        assert!(
            parse_callback(
                "GET /callback?error=denied&state=expected HTTP/1.1",
                "expected"
            )
            .is_err()
        );
    }
}
