use serde::de::DeserializeOwned;
use vesper_credentials::{
    Stored,
    games::{Provider, Session},
};

pub(crate) fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .connect_timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not initialize game HTTP client".to_owned())
}

pub(crate) async fn json<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> Result<T, String> {
    json_with_headers(request).await.map(|(body, _)| body)
}

pub(crate) async fn json_with_headers<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> Result<(T, reqwest::header::HeaderMap), String> {
    let mut response = request
        .send()
        .await
        .map_err(|_| "Could not reach the game service. Try again later.".to_owned())?;
    if !response.status().is_success() {
        return Err(format!(
            "Game service returned HTTP {}",
            response.status().as_u16()
        ));
    }
    let headers = response.headers().clone();
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Game response was interrupted".to_owned())?
    {
        if body.len() + chunk.len() > 8 * 1024 * 1024 {
            return Err("Game response exceeds the size limit".to_owned());
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body)
        .map(|body| (body, headers))
        .map_err(|_| {
            "The game service returned an unsupported response. Please update Vesper.".to_owned()
        })
}

pub(crate) fn session(provider: Provider) -> Result<Session, String> {
    match vesper_credentials::games::read(provider).map_err(|error| error.to_string())? {
        Stored::Ready(session) => Ok(session),
        Stored::Missing => Err("Connect this game provider in Settings first.".to_owned()),
    }
}

pub(crate) fn game(game: crate::Game) -> Result<Session, String> {
    if game.provider() != Provider::Mihoyo {
        return session(game.provider());
    }
    let mut accounts =
        vesper_credentials::games::accounts::read().map_err(|error| error.to_string())?;
    let id = accounts
        .bindings
        .get(game.key())
        .ok_or("Choose a miHoYo account for this game in Settings.")?;
    accounts
        .sessions
        .remove(id)
        .ok_or_else(|| "The selected miHoYo account is no longer connected.".into())
}

pub(crate) fn now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}
