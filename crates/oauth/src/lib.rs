use std::time::Duration;

use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
    basic::{BasicClient, BasicTokenResponse},
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

type Client = BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;
const MAX_CALLBACK_LINE_BYTES: usize = 8 * 1024;
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("OAuth configuration is invalid")]
    Configuration,
    #[error("OAuth callback failed: {0}")]
    Callback(&'static str),
    #[error("OAuth request failed")]
    Request(#[source] reqwest::Error),
    #[error("OAuth provider returned {0}")]
    Status(reqwest::StatusCode),
    #[error("OAuth provider returned invalid authorization data")]
    Protocol,
}

// Neither authorization state nor token data implements Debug: both contain secrets.
pub struct Authorization {
    pub url: String,
    client: Client,
    verifier: PkceCodeVerifier,
    state: CsrfToken,
    listener: TcpListener,
    path: String,
}

pub struct Token {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

pub async fn authorize(
    client_id: &str,
    authorize_url: &str,
    token_url: &str,
    redirect_uri: &str,
    scopes: &str,
) -> Result<Authorization, Error> {
    let redirect = RedirectUrl::new(redirect_uri.to_owned()).map_err(|_| Error::Configuration)?;
    if redirect.url().scheme() != "http" || redirect.url().host_str() != Some("127.0.0.1") {
        return Err(Error::Configuration);
    }
    let port = redirect.url().port().ok_or(Error::Configuration)?;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|_| Error::Callback("could not listen on the local port"))?;
    let path = redirect.url().path().to_owned();
    let client = client(client_id, token_url)?
        .set_auth_uri(AuthUrl::new(authorize_url.to_owned()).map_err(|_| Error::Configuration)?)
        .set_redirect_uri(redirect);
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(
            scopes
                .split_whitespace()
                .map(|scope| Scope::new(scope.to_owned())),
        )
        .set_pkce_challenge(challenge)
        .url();
    Ok(Authorization {
        url: url.into(),
        client,
        verifier,
        state,
        listener,
        path,
    })
}

fn client(
    client_id: &str,
    token_url: &str,
) -> Result<
    BasicClient<EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    Error,
> {
    Ok(BasicClient::new(ClientId::new(client_id.to_owned()))
        .set_token_uri(TokenUrl::new(token_url.to_owned()).map_err(|_| Error::Configuration)?))
}

pub async fn authenticate(authorization: Authorization) -> Result<Token, Error> {
    let code = tokio::time::timeout(CALLBACK_TIMEOUT, wait_for_code(&authorization))
        .await
        .map_err(|_| Error::Callback("authorization timed out"))?;
    let token = authorization
        .client
        .exchange_code(AuthorizationCode::new(code?))
        .set_pkce_verifier(authorization.verifier)
        .request_async(&send)
        .await
        .map_err(token_error)?;
    project_token(token)
}

pub async fn refresh(
    client_id: &str,
    token_url: &str,
    refresh_token: &str,
) -> Result<Token, Error> {
    let token = client(client_id, token_url)?
        .exchange_refresh_token(&RefreshToken::new(refresh_token.to_owned()))
        .request_async(&send)
        .await
        .map_err(token_error)?;
    project_token(token)
}

fn project_token(token: BasicTokenResponse) -> Result<Token, Error> {
    let expires_in = token.expires_in().ok_or(Error::Protocol)?.as_secs();
    if token.access_token().secret().is_empty()
        || token
            .refresh_token()
            .is_some_and(|token| token.secret().is_empty())
    {
        return Err(Error::Protocol);
    }
    Ok(Token {
        access_token: token.access_token().secret().clone(),
        refresh_token: token.refresh_token().map(|token| token.secret().clone()),
        expires_in,
    })
}

fn token_error(
    error: oauth2::RequestTokenError<Error, oauth2::basic::BasicErrorResponse>,
) -> Error {
    match error {
        oauth2::RequestTokenError::Request(error) => error,
        // Parse and server errors can include credentials or arbitrary response bodies.
        _ => Error::Protocol,
    }
}

async fn send(request: oauth2::HttpRequest) -> Result<oauth2::HttpResponse, Error> {
    // Adapt oauth2's HTTP types to the workspace's reqwest version and proxy policy.
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| Error::Request(error.without_url()))?;
    let request =
        reqwest::Request::try_from(request).map_err(|error| Error::Request(error.without_url()))?;
    let response = client
        .execute(request)
        .await
        .map_err(|error| Error::Request(error.without_url()))?;
    if !response.status().is_success() {
        return Err(Error::Status(response.status()));
    }
    let mut result = oauth2::HttpResponse::new(Vec::new());
    *result.status_mut() = response.status();
    *result.headers_mut() = response.headers().clone();
    *result.body_mut() = response
        .bytes()
        .await
        .map_err(|error| Error::Request(error.without_url()))?
        .to_vec();
    Ok(result)
}

async fn wait_for_code(authorization: &Authorization) -> Result<String, Error> {
    loop {
        let (mut stream, _) = authorization
            .listener
            .accept()
            .await
            .map_err(|_| Error::Callback("could not accept the local connection"))?;
        let mut line = Vec::new();
        let read = async {
            BufReader::new(&mut stream)
                .take((MAX_CALLBACK_LINE_BYTES + 1) as u64)
                .read_until(b'\n', &mut line)
                .await
        };
        if !matches!(
            tokio::time::timeout(Duration::from_secs(10), read).await,
            Ok(Ok(_))
        ) {
            continue;
        }
        let url = if line.len() <= MAX_CALLBACK_LINE_BYTES && line.ends_with(b"\n") {
            callback_url(&line)
        } else {
            Err(Error::Callback("request line is too long or incomplete"))
        };
        let is_callback = url
            .as_ref()
            .is_ok_and(|url| url.path() == authorization.path);
        let result = url.and_then(|url| {
            parse_callback(&url, &authorization.path, authorization.state.secret())
        });
        let (status, message) = if result.is_ok() {
            (
                "200 OK",
                "Authorization received. Return to Vesper to finish connecting.",
            )
        } else {
            (
                "400 Bad Request",
                "Authorization could not be completed. Return to Vesper and try again.",
            )
        };
        let body =
            format!("<!doctype html><meta charset=\"utf-8\"><title>Vesper</title><p>{message}</p>");
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = tokio::time::timeout(
            Duration::from_secs(10),
            stream.write_all(response.as_bytes()),
        )
        .await;
        if is_callback {
            return result;
        }
    }
}

fn callback_url(line: &[u8]) -> Result<reqwest::Url, Error> {
    let mut headers = [];
    let mut request = httparse::Request::new(&mut headers);
    request
        .parse(line)
        .map_err(|_| Error::Callback("invalid HTTP request"))?;
    if request.method != Some("GET") || request.version.is_none() {
        return Err(Error::Callback("expected an HTTP GET request"));
    }
    let target = request
        .path
        .filter(|path| path.starts_with('/') && !path.starts_with("//"))
        .ok_or(Error::Callback("invalid callback path"))?;
    reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| Error::Callback("invalid callback URL"))
}

fn parse_callback(url: &reqwest::Url, path: &str, expected_state: &str) -> Result<String, Error> {
    if url.path() != path {
        return Err(Error::Callback("unexpected callback path"));
    }
    let mut code = None;
    let mut state = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" if code.is_none() => code = Some(value.into_owned()),
            "state" if state.is_none() => state = Some(value.into_owned()),
            "error" => return Err(Error::Callback("access was denied")),
            "code" | "state" => return Err(Error::Callback("duplicate callback parameter")),
            _ => {}
        }
    }
    if state.as_deref() != Some(expected_state) {
        return Err(Error::Callback("callback state did not match"));
    }
    code.filter(|code| !code.is_empty())
        .ok_or(Error::Callback("callback did not include a code"))
}

#[cfg(test)]
#[path = "../tests/unit/oauth.rs"]
mod tests;
