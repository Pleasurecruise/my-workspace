use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::{Error, Result};

const AUTHORIZE_ENDPOINT: &str = "https://accounts.spotify.com/authorize";
const TOKEN_ENDPOINT: &str = "https://accounts.spotify.com/api/token";
const REDIRECT_PATH: &str = "/login";
pub(crate) const WEB_CLIENT_ID: &str = "d420a117a32841c2b3474932e49fb54b";
pub(crate) const PLAYBACK_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const WEB_SCOPES: &str = "user-library-read user-read-private";
const PLAYBACK_SCOPES: &str = "app-remote-control streaming user-modify-playback-state user-read-currently-playing user-read-playback-state user-read-private";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MAX_CALLBACK_LINE_BYTES: usize = 8 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct Authorization {
    pub url: String,
    client_id: &'static str,
    redirect_uri: String,
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

pub(crate) struct AccessToken {
    pub value: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
}

pub struct GrantToken {
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn web_authorization() -> Result<Authorization> {
    authorization(WEB_CLIENT_ID, 8989, WEB_SCOPES).await
}

pub async fn playback_authorization() -> Result<Authorization> {
    authorization(PLAYBACK_CLIENT_ID, 8898, PLAYBACK_SCOPES).await
}

async fn authorization(
    client_id: &'static str,
    port: u16,
    scopes: &'static str,
) -> Result<Authorization> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|error| {
            Error::Authentication(format!("could not listen for the callback: {error}"))
        })?;
    let redirect_uri = format!("http://127.0.0.1:{port}{REDIRECT_PATH}");
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
            ("client_id", client_id),
            ("response_type", "code"),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_challenge_method", "S256"),
            ("code_challenge", challenge.as_str()),
            ("state", state.as_str()),
            ("scope", scopes),
        ],
    )
    .map_err(|error| Error::Authentication(error.to_string()))?;
    Ok(Authorization {
        url: url.into(),
        client_id,
        redirect_uri,
        verifier,
        state,
        listener,
    })
}

pub async fn authenticate(authorization: Authorization) -> Result<GrantToken> {
    let client_id = authorization.client_id;
    let redirect_uri = authorization.redirect_uri.clone();
    let verifier = authorization.verifier.clone();
    let code = wait_for_code(authorization).await?;
    let response = http_client()?
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(Error::Status {
            operation: "sign in",
            status: response.status(),
        });
    }
    let token: TokenResponse = response.json().await?;
    let refresh_token = token.refresh_token.ok_or_else(|| {
        Error::Authentication("Spotify did not return a refresh token".to_owned())
    })?;
    Ok(GrantToken {
        access_token: token.access_token,
        refresh_token,
    })
}

pub(crate) async fn refresh(client_id: &str, refresh_token: &str) -> Result<AccessToken> {
    let response = http_client()?
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(Error::Status {
            operation: "refresh access",
            status: response.status(),
        });
    }
    let token: TokenResponse = response.json().await?;
    Ok(AccessToken {
        value: token.access_token,
        expires_in: token.expires_in,
        refresh_token: token.refresh_token,
    })
}

fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(Error::from)
}

async fn wait_for_code(authorization: Authorization) -> Result<String> {
    let callback = async {
        loop {
            let (mut stream, _) = authorization
                .listener
                .accept()
                .await
                .map_err(|error| Error::Authentication(error.to_string()))?;
            let mut line = Vec::new();
            {
                let mut reader =
                    BufReader::new(&mut stream).take((MAX_CALLBACK_LINE_BYTES + 1) as u64);
                reader
                    .read_until(b'\n', &mut line)
                    .await
                    .map_err(|error| Error::Authentication(error.to_string()))?;
            }
            let line = if line.len() <= MAX_CALLBACK_LINE_BYTES && line.ends_with(b"\n") {
                std::str::from_utf8(&line).unwrap_or_default()
            } else {
                ""
            };
            let result = parse_callback(line, &authorization.state);
            let (status, message) = match &result {
                Ok(_) => ("200 OK", "Spotify is connected. You can close this tab."),
                Err(_) => (
                    "400 Bad Request",
                    "Spotify sign-in could not be completed. Return to Vesper and try again.",
                ),
            };
            let body = format!(
                "<!doctype html><meta charset=\"utf-8\"><title>Vesper Music</title><p>{message}</p>"
            );
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            if callback_url(line).is_ok_and(|url| url.path() == REDIRECT_PATH) {
                return result;
            }
        }
    };
    tokio::time::timeout(CALLBACK_TIMEOUT, callback)
        .await
        .map_err(|_| Error::Authentication("sign-in timed out".to_owned()))?
}

fn parse_callback(line: &str, expected_state: &str) -> Result<String> {
    let url = callback_url(line)?;
    if url.path() != REDIRECT_PATH {
        return Err(Error::Authentication("unexpected callback path".to_owned()));
    }
    let mut code = None;
    let mut state = None;
    let mut denied = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            "error" => denied = Some(value.into_owned()),
            _ => {}
        }
    }
    if let Some(denied) = denied {
        return Err(Error::Authentication(format!(
            "Spotify refused access: {denied}"
        )));
    }
    if state.as_deref() != Some(expected_state) {
        return Err(Error::Authentication(
            "callback state did not match".to_owned(),
        ));
    }
    code.ok_or_else(|| Error::Authentication("callback did not include a code".to_owned()))
}

fn callback_url(line: &str) -> Result<reqwest::Url> {
    let target = line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| Error::Authentication("invalid callback".to_owned()))?;
    reqwest::Url::parse(&format!("http://localhost{target}"))
        .map_err(|error| Error::Authentication(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{WEB_CLIENT_ID, parse_callback};

    #[test]
    fn callback_requires_matching_state() {
        let line = "GET /login?code=ready&state=expected HTTP/1.1";
        assert_eq!(parse_callback(line, "expected").unwrap(), "ready");
        assert!(parse_callback(line, "different").is_err());
    }

    #[test]
    fn callback_reports_denied_access() {
        let line = "GET /login?error=access_denied&state=expected HTTP/1.1";
        assert!(parse_callback(line, "expected").is_err());
    }

    #[test]
    fn shared_web_client_is_stable() {
        assert_eq!(WEB_CLIENT_ID.len(), 32);
    }
}
