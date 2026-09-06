use games::{Captcha, CaptchaSolution, Game, Runtime};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

fn page(captcha: &Captcha) -> Result<reqwest::Url, String> {
    let registration =
        serde_json::to_string(&serde_json::json!({"gt":captcha.gt,"challenge":captcha.challenge,"new_captcha":captcha.new_captcha,"success":captcha.success}))
            .map_err(|_| "Could not prepare the verification window.")?
            .replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('&', "\\u0026");
    let html = include_str!("captcha.html").replace("__VESPER_CAPTCHA__", &registration);
    let encoded = percent_encoding::utf8_percent_encode(&html, percent_encoding::NON_ALPHANUMERIC);
    reqwest::Url::parse(&format!("data:text/html;charset=utf-8,{encoded}"))
        .map_err(|_| "Could not prepare the verification page.".into())
}

fn solution(url: &reqwest::Url) -> Result<CaptchaSolution, String> {
    if url.scheme() != "vesper-captcha" || url.host_str() != Some("result") {
        return Err("Invalid verification callback.".into());
    }
    let encoded = url
        .query_pairs()
        .find(|(key, _)| key == "data")
        .map(|(_, value)| value.into_owned())
        .ok_or("Verification result is missing.")?;
    if encoded.len() > 16 * 1024 {
        return Err("Verification result exceeds the size limit.".into());
    }
    serde_json::from_str(&encoded).map_err(|_| "Invalid verification result format.".into())
}

pub(super) fn open(app: &AppHandle, game: Game, captcha: Captcha) -> Result<(), String> {
    let target = page(&captcha)?;
    let origin = target.clone();
    let handle = app.clone();
    let id = captcha.id.clone();
    let submitted = Arc::new(AtomicBool::new(false));
    let window = WebviewWindowBuilder::new(app, "game-verification", WebviewUrl::External(target))
        .title("miHoYo security verification")
        .inner_size(420.0, 500.0)
        .min_inner_size(360.0, 420.0)
        .center()
        .incognito(true)
        .on_navigation(move |url| {
            if url == &origin || url.as_str() == "about:blank" {
                return true;
            }
            if url.scheme() != "vesper-captcha" {
                return url.scheme() == "https"
                    && url.host_str().is_some_and(|host| {
                        host == "geetest.com" || host.ends_with(".geetest.com")
                    });
            }
            if submitted.swap(true, Ordering::SeqCst) {
                return false;
            }
            let result = solution(url);
            let Some(window) = handle.get_webview_window("game-verification") else {
                return false;
            };
            let app = handle.clone();
            let id = id.clone();
            tauri::async_runtime::spawn(async move {
                let result = match result {
                    Ok(solution) => {
                        app.state::<Runtime>()
                            .complete_verification(&id, solution)
                            .await
                    }
                    Err(message) => Err(message),
                };
                if result.is_ok() {
                    let result = games::NotesResponse::from(Err(games::NotesError::RefreshRequired));
                    let _ = app.emit_to("main", "game-notes-updated", serde_json::json!({"game":game,"result":result}));
                }
                let text = match result {
                    Ok(()) => {
                        "Verification accepted. Close this window, then use the refresh icon to check game-record access.".to_owned()
                    }
                    Err(message) => message,
                };
                if let Ok(text) = serde_json::to_string(&text) {
                    let _ = window.eval(format!(
                        "if (location.protocol === 'data:') window.showVerificationStatus({text});"
                    ));
                }
            });
            false
        })
        .build()
        .map_err(|_| "Could not create the verification window.")?;
    let handle = app.clone();
    let id = captcha.id;
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            let app = handle.clone();
            let id = id.clone();
            tauri::async_runtime::spawn(async move {
                app.state::<Runtime>().cancel_verification(&id).await;
                let _ = app.emit_to("main", "game-verification-closed", game);
            });
        }
    });
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/game_captcha.rs"]
mod tests;
