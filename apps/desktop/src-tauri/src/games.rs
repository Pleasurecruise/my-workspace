use crate::CommandResponse;
use games::session::{self, Session};
use games::{Connections, Game, LoginProgress, LoginQr, NotesResponse, Provider, Runtime, archive};
use tauri::{AppHandle, Emitter, Manager};
use vault::Stored;
mod captcha;
mod verification;

// Verification webviews host third-party pages; game state belongs only to our UI windows.
fn emit<T: serde::Serialize + Clone>(
    app: &AppHandle,
    event: &str,
    payload: T,
) -> tauri::Result<()> {
    app.emit_filter(event, payload, |target| {
        matches!(target,
        tauri::EventTarget::WebviewWindow { label } if label == "main" || label == "island")
    })
}

fn notify_accounts_changed(app: &AppHandle) {
    if emit(app, "game-accounts-changed", ()).is_err() {
        tracing::warn!("Could not notify the main window of game account changes");
    }
}

// Stored values are returned only to the trusted Settings form.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SteamSettings {
    api_key: String,
    steam_id: String,
}

#[tauri::command]
pub(crate) fn read_steam_settings() -> CommandResponse<Option<SteamSettings>> {
    let settings = match session::read(Provider::Steam) {
        Ok(Stored::Ready(Session::Steam { api_key, steam_id })) => {
            Ok(Some(SteamSettings { api_key, steam_id }))
        }
        Ok(Stored::Missing) => Ok(None),
        Ok(_) => Err("Invalid Steam configuration".to_owned()),
        Err(error) => Err(error.to_string()),
    };
    settings.into()
}

#[tauri::command]
pub(crate) fn read_game_connections() -> CommandResponse<Connections> {
    games::connections().into()
}

#[tauri::command]
pub(crate) async fn select_game_account(
    game: Game,
    id: Option<String>,
    app: AppHandle,
) -> CommandResponse<()> {
    if app.get_webview_window("game-verification").is_some() {
        return CommandResponse::Failed {
            message: "Close game verification before changing accounts.".into(),
        };
    }
    let result = app
        .state::<Runtime>()
        .select_account(game, id.as_deref())
        .await;
    if result.is_ok() {
        notify_accounts_changed(&app);
    }
    result.into()
}

#[tauri::command]
pub(crate) async fn remove_game_account(id: String, app: AppHandle) -> CommandResponse<()> {
    if app.get_webview_window("game-verification").is_some() {
        return CommandResponse::Failed {
            message: "Close game verification before removing accounts.".into(),
        };
    }
    let result = app.state::<Runtime>().remove_account(&id).await;
    if result.is_ok() {
        notify_accounts_changed(&app);
    }
    result.into()
}

#[tauri::command]
pub(crate) async fn begin_game_login(
    provider: Provider,
    app: AppHandle,
) -> CommandResponse<LoginQr> {
    let runtime = app.state::<Runtime>();
    runtime.begin_login(provider).await.into()
}

#[tauri::command]
pub(crate) async fn poll_game_login(
    provider: Provider,
    id: String,
    app: AppHandle,
) -> CommandResponse<LoginProgress> {
    let runtime = app.state::<Runtime>();
    let result = runtime.poll_login(provider, &id).await;
    if matches!(result, Ok(LoginProgress::Complete)) {
        notify_accounts_changed(&app);
    }
    result.into()
}

#[tauri::command]
pub(crate) async fn cancel_game_login(provider: Provider, id: String, app: AppHandle) {
    app.state::<Runtime>().cancel_login(provider, &id).await;
}

#[tauri::command]
pub(crate) async fn save_steam_connection(
    api_key: String,
    steam_id: String,
) -> CommandResponse<()> {
    games::save_steam(api_key, steam_id).await.into()
}

#[tauri::command]
pub(crate) async fn read_game_notes(
    game: Game,
    refresh: Option<bool>,
    app: AppHandle,
) -> NotesResponse {
    let runtime = app.state::<Runtime>();
    runtime.notes(game, refresh.unwrap_or(false)).await.into()
}

#[tauri::command]
pub(crate) async fn verify_game(game: Game, app: AppHandle) -> CommandResponse<bool> {
    let result = async {
        if let Some(window) = app.get_webview_window("game-verification") {
            window
                .set_focus()
                .map_err(|_| "Could not focus the verification window.")?;
            return Err("Complete or close the current verification window first.".into());
        }
        if matches!(game, Game::Genshin | Game::StarRail) {
            let challenge = app.state::<Runtime>().begin_verification(game).await?;
            let id = challenge.id.clone();
            let result = captcha::open(&app, game, challenge);
            if result.is_err() {
                app.state::<Runtime>().cancel_verification(&id).await;
            }
            result.map(|()| true)
        } else {
            let page = app.state::<Runtime>().record_page(game).await?;
            verification::open(&app, game, page).map(|()| true)
        }
    }
    .await;
    result.into()
}

#[tauri::command]
pub(crate) async fn read_steam_games(app: AppHandle) -> CommandResponse<games::steam::Snapshot> {
    let runtime = app.state::<Runtime>();
    runtime.steam().await.into()
}

#[tauri::command]
pub(crate) async fn read_gacha_archive(
    game: Game,
    uid: Option<String>,
    app: AppHandle,
) -> CommandResponse<archive::Summary> {
    let runtime = app.state::<Runtime>();
    runtime.summary(game, uid).await.into()
}

#[tauri::command]
pub(crate) async fn sync_gacha_archive(
    game: Game,
    app: AppHandle,
) -> CommandResponse<archive::Summary> {
    let runtime = app.state::<Runtime>();
    runtime.sync(game).await.into()
}

#[derive(Clone, serde::Serialize)]
struct NotesEvent {
    game: Game,
    result: NotesResponse,
}

pub(crate) async fn refresh(app: &tauri::AppHandle, force: bool) -> Result<(), String> {
    let (selected, steam) = crate::dashboard::layout()?.games();
    let mut tasks = tokio::task::JoinSet::new();
    for game in selected {
        let app = app.clone();
        tasks.spawn(async move {
            let result = read_game_notes(game, Some(force), app.clone()).await;
            if emit(&app, "game-notes-updated", NotesEvent { game, result }).is_err() {
                tracing::warn!("Could not publish game notes");
            }
        });
    }
    if steam {
        let app = app.clone();
        tasks.spawn(async move {
            let result = read_steam_games(app.clone()).await;
            if emit(&app, "steam-games-updated", result).is_err() {
                tracing::warn!("Could not publish Steam activity");
            }
        });
    }
    while let Some(result) = tasks.join_next().await {
        result.map_err(|_| "Game refresh worker failed")?;
    }
    Ok(())
}
