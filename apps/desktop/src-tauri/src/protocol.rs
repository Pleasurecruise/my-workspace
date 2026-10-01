use crate::content::Content;
use crate::music::Music;
use tauri::http::{HeaderValue, Method, Request, Response, StatusCode, header};
use tauri::{Manager, UriSchemeContext, UriSchemeResponder, Wry};

type Context<'a> = UriSchemeContext<'a, Wry>;

fn empty(status: StatusCode) -> Response<Vec<u8>> {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = status;
    response
}

fn image(content_type: &str, cache: &'static str, bytes: Vec<u8>) -> Response<Vec<u8>> {
    let mut response = Response::new(bytes);
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    response
}

fn key(context: &Context<'_>, request: &Request<Vec<u8>>) -> Result<String, StatusCode> {
    if context.webview_label() != "main" {
        return Err(StatusCode::FORBIDDEN);
    }
    if request.method() != Method::GET {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    percent_encoding::percent_decode_str(request.uri().path().trim_start_matches('/'))
        .decode_utf8()
        .map(|key| key.into_owned())
        .map_err(|_| StatusCode::BAD_REQUEST)
}

fn image_type(key: &str) -> Option<&'static str> {
    let (_, extension) = key.rsplit_once('.')?;
    match extension.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "avif" => Some("image/avif"),
        _ => None,
    }
}

pub(crate) fn asset(
    context: Context<'_>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let key = match key(&context, &request) {
        Ok(key) => key,
        Err(status) => return responder.respond(empty(status)),
    };
    let Some(content_type) = image_type(&key) else {
        return responder.respond(empty(StatusCode::BAD_REQUEST));
    };
    let app = context.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        let response = match app.state::<Content>().asset(&key).await {
            Ok(data) => image(content_type, "no-store", data.as_ref().clone()),
            Err(error) => {
                tracing::warn!(%error, %key, "could not serve a Moment image");
                empty(StatusCode::NOT_FOUND)
            }
        };
        responder.respond(response);
    });
}

pub(crate) fn music_cover(
    context: Context<'_>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let key = match key(&context, &request) {
        Ok(key) => key,
        Err(status) => return responder.respond(empty(status)),
    };
    let app = context.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        let response = match app.state::<Music>().cover(&key).await {
            Ok(cover) => image(&cover.content_type, "private, max-age=86400", cover.bytes),
            Err(error) => {
                tracing::warn!(%error, %key, "could not serve a music album cover");
                empty(StatusCode::NOT_FOUND)
            }
        };
        responder.respond(response);
    });
}
