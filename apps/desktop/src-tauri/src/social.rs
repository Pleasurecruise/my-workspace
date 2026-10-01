use crate::{CommandError, CommandResponse};
use consumers::api::memos::{self, Visibility};
use social::telegram::Authorization;
use social::{MemoPublication, PublicationVisibility, PublishedPost};
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

#[derive(Default)]
pub(crate) struct Social {
    telegram: Mutex<Telegram>,
    x: Mutex<()>,
}

#[derive(Default)]
enum Telegram {
    #[default]
    Idle,
    Busy,
    Pending(social::telegram::Login),
}

impl Social {
    async fn begin(&self) -> Result<(), CommandError> {
        let mut telegram = self.telegram.lock().await;
        match *telegram {
            Telegram::Idle => {
                *telegram = Telegram::Busy;
                Ok(())
            }
            Telegram::Busy => Err("Another Telegram operation is already in progress".into()),
            Telegram::Pending(_) => Err("Telegram authorization is waiting for user input".into()),
        }
    }

    async fn finish(&self, next: Telegram) {
        let mut telegram = self.telegram.lock().await;
        if matches!(*telegram, Telegram::Busy) {
            *telegram = next;
        }
    }
}

async fn publication(id: &str) -> Result<MemoPublication, CommandError> {
    let memo = memos::read(id).await?.memo;
    Ok(MemoPublication {
        id: memo.id,
        content: memo.content,
        visibility: match memo.visibility {
            Visibility::Public => PublicationVisibility::Public,
            Visibility::Private => PublicationVisibility::Private,
        },
    })
}

#[tauri::command]
pub(crate) async fn publish_telegram(
    id: String,
    app: tauri::AppHandle,
) -> CommandResponse<PublishedPost> {
    let state = app.state::<Social>();
    let result = async {
        let memo = publication(&id).await?;
        state.begin().await?;
        let published = match database::path() {
            Ok(path) => social::telegram::publish(&memo, &path)
                .await
                .map_err(CommandError::from),
            Err(error) => Err(error.into()),
        };
        state.finish(Telegram::Idle).await;
        published
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn publish_x(id: String, app: tauri::AppHandle) -> CommandResponse<PublishedPost> {
    let state = app.state::<Social>();
    let _operation = state.x.lock().await;
    let result = async {
        let memo = publication(&id).await?;
        Ok::<_, CommandError>(social::x::publish(&memo).await?)
    };
    result.await.into()
}

#[tauri::command]
pub(crate) fn save_telegram(
    credentials: social::telegram::credentials::Credentials,
) -> CommandResponse<String> {
    CommandResponse::from(
        social::telegram::credentials::save(credentials).map(|()| "telegram".to_owned()),
    )
}

#[tauri::command]
pub(crate) async fn connect_x(app: tauri::AppHandle) -> CommandResponse<String> {
    let state = app.state::<Social>();
    let _operation = state.x.lock().await;
    let result = async {
        let authorization = social::x::authorization().await?;
        app.opener()
            .open_url(&authorization.url, None::<String>)
            .map_err(|error| format!("Could not open X authorization: {error}"))?;
        let credentials = social::x::authenticate(authorization).await?;
        social::x::credentials::save(credentials)?;
        Ok::<_, CommandError>("x".to_owned())
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn read_auth(app: tauri::AppHandle) -> CommandResponse<Authorization> {
    let state = app.state::<Social>();
    let result = async {
        state.begin().await?;
        let status = match database::path() {
            Ok(path) => social::telegram::read_auth(&path)
                .await
                .map_err(CommandError::from),
            Err(error) => Err(error.into()),
        };
        state.finish(Telegram::Idle).await;
        status
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn begin_auth(
    phone: String,
    app: tauri::AppHandle,
) -> CommandResponse<Authorization> {
    let state = app.state::<Social>();
    let result = async {
        state.begin().await?;
        let login = match database::path() {
            Ok(path) => social::telegram::begin_login(&path, &phone)
                .await
                .map_err(CommandError::from),
            Err(error) => Err(error.into()),
        };
        match login {
            Ok((status, pending)) => {
                state
                    .finish(pending.map_or(Telegram::Idle, Telegram::Pending))
                    .await;
                Ok(status)
            }
            Err(error) => {
                state.finish(Telegram::Idle).await;
                Err(error)
            }
        }
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn submit_code(
    code: String,
    app: tauri::AppHandle,
) -> CommandResponse<Authorization> {
    complete(app.state::<Social>().inner(), Completion::Code(code)).await
}

#[tauri::command]
pub(crate) async fn submit_password(
    password: String,
    app: tauri::AppHandle,
) -> CommandResponse<Authorization> {
    complete(
        app.state::<Social>().inner(),
        Completion::Password(password),
    )
    .await
}

#[tauri::command]
pub(crate) async fn cancel_auth(app: tauri::AppHandle) -> CommandResponse<String> {
    let state = app.state::<Social>();
    let mut telegram = state.telegram.lock().await;
    if matches!(*telegram, Telegram::Busy) {
        return CommandResponse::Failed {
            message: "Telegram authorization is currently processing".to_owned(),
        };
    }
    *telegram = Telegram::Idle;
    CommandResponse::Ready {
        data: "telegram".to_owned(),
    }
}

enum Completion {
    Code(String),
    Password(String),
}

async fn complete(state: &Social, completion: Completion) -> CommandResponse<Authorization> {
    let mut login = {
        let mut telegram = state.telegram.lock().await;
        match std::mem::replace(&mut *telegram, Telegram::Busy) {
            Telegram::Pending(login) => login,
            Telegram::Idle => {
                *telegram = Telegram::Idle;
                return CommandResponse::Failed {
                    message: "Telegram authorization has not been started".to_owned(),
                };
            }
            Telegram::Busy => {
                return CommandResponse::Failed {
                    message: "Telegram authorization is already in progress".to_owned(),
                };
            }
        }
    };
    let result = match completion {
        Completion::Code(code) => login.complete_code(&code).await,
        Completion::Password(password) => login.complete_password(&password).await,
    };
    let next = if login.can_continue() {
        Telegram::Pending(login)
    } else {
        Telegram::Idle
    };
    *state.telegram.lock().await = next;
    result.into()
}
