use crate::CommandResponse;
use tauri::Manager;

#[derive(Default)]
pub(crate) struct TelegramAuthorizationState {
    runtime: tokio::sync::Mutex<AuthorizationRuntime>,
}

#[derive(Default)]
enum AuthorizationRuntime {
    #[default]
    Idle,
    Busy,
    Pending(social::TelegramLogin),
}

impl TelegramAuthorizationState {
    pub(crate) async fn begin_operation(&self) -> Result<(), String> {
        let mut runtime = self.runtime.lock().await;
        match *runtime {
            AuthorizationRuntime::Idle => {
                *runtime = AuthorizationRuntime::Busy;
                Ok(())
            }
            AuthorizationRuntime::Busy => {
                Err("Another Telegram operation is already in progress".to_owned())
            }
            AuthorizationRuntime::Pending(_) => {
                Err("Telegram authorization is waiting for user input".to_owned())
            }
        }
    }

    pub(crate) async fn finish_operation(&self) {
        let mut runtime = self.runtime.lock().await;
        if matches!(*runtime, AuthorizationRuntime::Busy) {
            *runtime = AuthorizationRuntime::Idle;
        }
    }
}

#[tauri::command]
pub(crate) async fn read_auth(
    app: tauri::AppHandle,
) -> CommandResponse<social::TelegramAuthorizationStatus> {
    let state = app.state::<TelegramAuthorizationState>();
    if let Err(message) = state.begin_operation().await {
        return CommandResponse::Failed { message };
    }
    let session_path = match session_path(&app) {
        Ok(path) => path,
        Err(message) => {
            state.finish_operation().await;
            return CommandResponse::Failed { message };
        }
    };
    let response = match social::read_auth(&session_path).await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    };
    state.finish_operation().await;
    response
}

#[tauri::command]
pub(crate) async fn begin_auth(
    phone: String,
    app: tauri::AppHandle,
) -> CommandResponse<social::TelegramAuthorizationStatus> {
    let state = app.state::<TelegramAuthorizationState>();
    if let Err(message) = state.begin_operation().await {
        return CommandResponse::Failed { message };
    }
    let session_path = match session_path(&app) {
        Ok(path) => path,
        Err(message) => {
            *state.runtime.lock().await = AuthorizationRuntime::Idle;
            return CommandResponse::Failed { message };
        }
    };
    match social::begin_login(&session_path, &phone).await {
        Ok((data, pending)) => {
            *state.runtime.lock().await = match pending {
                Some(login) => AuthorizationRuntime::Pending(login),
                None => AuthorizationRuntime::Idle,
            };
            CommandResponse::Ready { data }
        }
        Err(error) => {
            *state.runtime.lock().await = AuthorizationRuntime::Idle;
            CommandResponse::Failed {
                message: error.to_string(),
            }
        }
    }
}

#[tauri::command]
pub(crate) async fn submit_code(
    code: String,
    app: tauri::AppHandle,
) -> CommandResponse<social::TelegramAuthorizationStatus> {
    complete(
        app.state::<TelegramAuthorizationState>().inner(),
        AuthorizationCompletion::Code(code),
    )
    .await
}

#[tauri::command]
pub(crate) async fn submit_password(
    password: String,
    app: tauri::AppHandle,
) -> CommandResponse<social::TelegramAuthorizationStatus> {
    complete(
        app.state::<TelegramAuthorizationState>().inner(),
        AuthorizationCompletion::Password(password),
    )
    .await
}

#[tauri::command]
pub(crate) async fn cancel_auth(app: tauri::AppHandle) -> CommandResponse<String> {
    let state = app.state::<TelegramAuthorizationState>();
    let mut runtime = state.runtime.lock().await;
    match *runtime {
        AuthorizationRuntime::Busy => CommandResponse::Failed {
            message: "Telegram authorization is currently processing".to_owned(),
        },
        AuthorizationRuntime::Idle | AuthorizationRuntime::Pending(_) => {
            *runtime = AuthorizationRuntime::Idle;
            CommandResponse::Ready {
                data: "telegram".to_owned(),
            }
        }
    }
}

enum AuthorizationCompletion {
    Code(String),
    Password(String),
}

async fn complete(
    state: &TelegramAuthorizationState,
    completion: AuthorizationCompletion,
) -> CommandResponse<social::TelegramAuthorizationStatus> {
    let mut login = {
        let mut runtime = state.runtime.lock().await;
        match std::mem::replace(&mut *runtime, AuthorizationRuntime::Busy) {
            AuthorizationRuntime::Pending(login) => login,
            AuthorizationRuntime::Idle => {
                *runtime = AuthorizationRuntime::Idle;
                return CommandResponse::Failed {
                    message: "Telegram authorization has not been started".to_owned(),
                };
            }
            AuthorizationRuntime::Busy => {
                *runtime = AuthorizationRuntime::Busy;
                return CommandResponse::Failed {
                    message: "Telegram authorization is already in progress".to_owned(),
                };
            }
        }
    };
    let result = match completion {
        AuthorizationCompletion::Code(code) => login.complete_code(&code).await,
        AuthorizationCompletion::Password(password) => login.complete_password(&password).await,
    };
    let can_continue = login.can_continue();
    *state.runtime.lock().await = if can_continue {
        AuthorizationRuntime::Pending(login)
    } else {
        AuthorizationRuntime::Idle
    };
    match result {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

fn session_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("telegram.session"))
        .map_err(|error| format!("could not resolve Telegram session storage: {error}"))
}
