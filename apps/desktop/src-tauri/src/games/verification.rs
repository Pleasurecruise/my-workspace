use games::{BridgeMessage, Game, VerificationPage};
use tauri::{
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent, webview::Cookie,
};

const BRIDGE: &str = r#"
if (window === window.top && ["https://webstatic.mihoyo.com", "https://act.mihoyo.com"].includes(location.origin)) {
    const messages = [];
    let sending = false;
    const send = () => {
        if (sending || messages.length === 0) return;
        sending = true;
        location.href = "vesper-game-bridge://message?data=" + encodeURIComponent(messages[0]);
    };
    window.__vesperBridgeAck = () => { messages.shift(); sending = false; send(); };
    window.MiHoYoJSInterface = {
        postMessage: (message) => { messages.push(message); send(); },
        closePage: () => { messages.push(JSON.stringify({method: "closePage"})); send(); }
    };
}
"#;

fn allowed(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.host_str().is_some_and(|host| {
            matches!(
                host,
                "webstatic.mihoyo.com" | "act.mihoyo.com" | "geetest.com"
            ) || host.ends_with(".geetest.com")
        })
}

fn session_installed(
    expected: &std::collections::BTreeMap<String, String>,
    installed: &[Cookie<'_>],
) -> bool {
    expected.iter().all(|(name, value)| {
        installed.iter().any(|cookie| {
            cookie.name() == name
                && cookie.value() == value
                && cookie
                    .domain()
                    .is_some_and(|domain| domain.trim_start_matches('.') == "mihoyo.com")
                && cookie.path() == Some("/")
                && cookie.secure() == Some(true)
        })
    })
}

pub(super) fn open(app: &AppHandle, game: Game, page: VerificationPage) -> Result<(), String> {
    let label = "game-verification";
    if let Some(window) = app.get_webview_window(label) {
        window
            .set_focus()
            .map_err(|_| "Could not focus the verification window")?;
        return Err("Finish or close the existing game-record verification window first.".into());
    }
    let target = reqwest::Url::parse(page.url).map_err(|_| "Invalid game-record page")?;
    let cookies = page.cookies().clone();
    let handle = app.clone();
    let window = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::External(reqwest::Url::parse("about:blank").map_err(|_| "Invalid blank page")?),
    )
    .title("Miyoushe — complete verification, then close this window")
    .inner_size(480.0, 760.0)
    .min_inner_size(360.0, 480.0)
    .center()
    .incognito(true)
    .user_agent(games::RECORD_USER_AGENT)
    .initialization_script(BRIDGE)
    .on_navigation(move |url| {
        if url.as_str() == "about:blank" {
            return true;
        }
        if url.scheme() != "vesper-game-bridge" {
            return allowed(url);
        }
        let Some(window) = handle.get_webview_window(label) else {
            return false;
        };
        let result = (|| -> Result<(), String> {
            let encoded = url
                .query_pairs()
                .find(|(key, _)| key == "data")
                .map(|(_, value)| value.into_owned())
                .ok_or("Missing verification message")?;
            if encoded.len() > 64 * 1024 {
                return Err("Verification message is too large".into());
            }
            let message: BridgeMessage =
                serde_json::from_str(&encoded).map_err(|_| "Invalid verification message")?;
            match message.method.as_str() {
                "login" => {
                    window.close().map_err(|_| "Could not close verification")?;
                    super::emit(&handle, "game-login-required", game)
                        .map_err(|_| "Could not open game login")?;
                    if let Some(main) = handle.get_webview_window("main") {
                        main.set_focus().map_err(|_| "Could not focus the main window")?;
                    }
                }
                "closePage" => window.close().map_err(|_| "Could not close verification")?,
                "pushPage" => {
                    let destination = message
                        .payload
                        .as_ref()
                        .and_then(|payload| payload.page.as_ref())
                        .ok_or("Missing game-record destination")?;
                    let destination =
                        reqwest::Url::parse(&destination.replace("rolePageAccessNotAllowed=&", ""))
                            .map_err(|_| "Invalid game-record destination")?;
                    if !allowed(&destination) {
                        return Err("Unsupported game-record destination".into());
                    }
                    window
                        .navigate(destination)
                        .map_err(|_| "Could not open game-record page")?;
                }
                _ => {
                    if let Some(callback) = &message.callback {
                        let response = page.respond(&message)?;
                        let callback = serde_json::to_string(callback)
                            .map_err(|_| "Invalid verification callback")?;
                        let script = match response {
                            Some(response) => {
                                let response = serde_json::to_string(&response)
                                    .map_err(|_| "Invalid verification response")?;
                                    format!("if (location.origin === 'https://webstatic.mihoyo.com' || location.origin === 'https://act.mihoyo.com') window.mhyWebBridge({callback}, {response});")
                            }
                            None => format!("if (location.origin === 'https://webstatic.mihoyo.com' || location.origin === 'https://act.mihoyo.com') window.mhyWebBridge({callback});"),
                        };
                        window
                            .eval(script)
                            .map_err(|_| "Could not deliver verification response")?;
                    }
                }
            }
            Ok(())
        })();
        if let Err(message) = result
            && handle
                .emit_to(
                    "main",
                    "game-verification-error",
                    serde_json::json!({"game":game,"message":message}),
                )
                .is_err()
        {
            tracing::warn!("Could not deliver the game verification error");
        }
        if window.eval("window.__vesperBridgeAck?.();").is_err() {
            tracing::warn!("Could not acknowledge the game verification message");
        }
        false
    })
    .build()
    .map_err(|_| "Could not open the official game-record window")?;
    for (name, value) in &cookies {
        let cookie = Cookie::build((name.clone(), value.clone()))
            .domain(".mihoyo.com")
            .path("/")
            .secure(true)
            .build();
        if window.set_cookie(cookie).is_err() {
            if window.close().is_err() {
                tracing::warn!("Could not close the failed game verification window");
            }
            return Err("Could not prepare the official game-record session".into());
        }
    }
    // Read the whole isolated store: Wry's macOS cookies_for_url uses exact
    // domain equality and incorrectly omits our .mihoyo.com parent-domain cookies.
    let installed = window.cookies();
    if !installed
        .as_ref()
        .is_ok_and(|installed| session_installed(&cookies, installed))
    {
        if window.close().is_err() {
            tracing::warn!("Could not close the failed game verification window");
        }
        return Err("Could not transfer your login to the official record window. Please reopen verification.".into());
    }
    let handle = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed)
            && super::emit(&handle, "game-verification-closed", game).is_err()
        {
            tracing::warn!("Could not notify the main window that verification closed");
        }
    });
    if window.navigate(target).is_err() {
        if window.close().is_err() {
            tracing::warn!("Could not close the failed game verification window");
        }
        return Err("Could not load the official game-record page".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/game_verification.rs"]
mod tests;
