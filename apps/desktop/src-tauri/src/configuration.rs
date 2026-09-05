use crate::CommandResponse;
use crate::cms::CmsState;
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

#[derive(Default)]
pub(crate) struct PublicationState {
    pub(crate) x_operation: tokio::sync::Mutex<()>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigurationStatus {
    ugos: StoredConfiguration<UgosConfiguration>,
    r2: StoredConfiguration<R2Configuration>,
    api: ApiConfiguration,
    ntfy: StoredConfiguration<vesper_credentials::NtfyConfig>,
    ntfy_dev: bool,
    app_lock: StoredConfiguration<String>,
    app_lock_dev: bool,
    spotify: StoredConfiguration<String>,
    qq_music: StoredConfiguration<String>,
    publication: social::PublicationConfigurationStatus,
}

#[derive(Clone, serde::Serialize)]
#[serde(tag = "status", content = "data", rename_all = "camelCase")]
enum StoredConfiguration<T> {
    Missing,
    Ready(T),
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UgosConfiguration {
    username: String,
    password: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct R2Configuration {
    access_key_id: String,
    secret_access_key: String,
}

#[derive(Clone, serde::Serialize)]
struct ApiConfiguration {
    memos: StoredConfiguration<String>,
    moment: StoredConfiguration<String>,
    knowledge: StoredConfiguration<String>,
}

#[tauri::command]
pub(crate) fn read_configuration() -> CommandResponse<ConfigurationStatus> {
    let ugos = match vesper_credentials::ugos() {
        Ok(vesper_credentials::Stored::Ready(credentials)) => {
            StoredConfiguration::Ready(UgosConfiguration {
                username: credentials.username,
                password: credentials.password,
            })
        }
        Ok(vesper_credentials::Stored::Missing) => StoredConfiguration::Missing,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let r2 = match vesper_credentials::r2() {
        Ok(vesper_credentials::Stored::Ready(credentials)) => {
            StoredConfiguration::Ready(R2Configuration {
                access_key_id: credentials.access_key_id,
                secret_access_key: credentials.secret_access_key,
            })
        }
        Ok(vesper_credentials::Stored::Missing) => StoredConfiguration::Missing,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let memos = match read_consumer_api(vesper_credentials::ConsumerApi::Memos) {
        Ok(configuration) => configuration,
        Err(message) => return CommandResponse::Failed { message },
    };
    let moment = match read_consumer_api(vesper_credentials::ConsumerApi::Moment) {
        Ok(configuration) => configuration,
        Err(message) => return CommandResponse::Failed { message },
    };
    let knowledge = match read_consumer_api(vesper_credentials::ConsumerApi::Knowledge) {
        Ok(configuration) => configuration,
        Err(message) => return CommandResponse::Failed { message },
    };
    let (ntfy, ntfy_dev) = match vesper_credentials::ntfy() {
        Ok(vesper_credentials::Stored::Ready(configuration)) => {
            let development = configuration.development;
            (StoredConfiguration::Ready(configuration), development)
        }
        Ok(vesper_credentials::Stored::Missing) => (StoredConfiguration::Missing, false),
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let (app_lock, app_lock_dev) = match vesper_credentials::app_lock() {
        Ok(vesper_credentials::Stored::Ready(app_lock)) => (
            StoredConfiguration::Ready(app_lock.password),
            app_lock.development,
        ),
        Ok(vesper_credentials::Stored::Missing) => (StoredConfiguration::Missing, false),
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let spotify = match vesper_credentials::spotify() {
        Ok(vesper_credentials::Stored::Ready(_)) => {
            StoredConfiguration::Ready("shared-web-and-local-playback".to_owned())
        }
        Ok(vesper_credentials::Stored::Missing) => StoredConfiguration::Missing,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let qq_music = match vesper_credentials::qq_music() {
        Ok(vesper_credentials::Stored::Ready(_)) => {
            StoredConfiguration::Ready("renewable-session".to_owned())
        }
        Ok(vesper_credentials::Stored::Missing) => StoredConfiguration::Missing,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let publication = match social::read_config() {
        Ok(publication) => publication,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    CommandResponse::Ready {
        data: ConfigurationStatus {
            ugos,
            r2,
            api: ApiConfiguration {
                memos,
                moment,
                knowledge,
            },
            ntfy,
            ntfy_dev,
            app_lock,
            app_lock_dev,
            spotify,
            qq_music,
            publication,
        },
    }
}

#[tauri::command]
pub(crate) async fn save_ntfy_configuration(
    configuration: vesper_credentials::NtfyConfig,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    match vesper_credentials::save_ntfy(configuration) {
        Ok(()) => match crate::notifications::restart(app).await {
            Ok(()) => CommandResponse::Ready {
                data: "ntfy-notifications".to_owned(),
            },
            Err(message) => CommandResponse::Failed { message },
        },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

fn read_consumer_api(
    api: vesper_credentials::ConsumerApi,
) -> Result<StoredConfiguration<String>, String> {
    match vesper_credentials::consumer_api(api) {
        Ok(vesper_credentials::Stored::Ready(api_key)) => Ok(StoredConfiguration::Ready(api_key)),
        Ok(vesper_credentials::Stored::Missing) => Ok(StoredConfiguration::Missing),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
pub(crate) fn save_ugos_configuration(
    username: String,
    password: String,
) -> CommandResponse<String> {
    match ugos::configure(username, password) {
        Ok(()) => CommandResponse::Ready {
            data: "ugos".to_owned(),
        },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn save_r2_configuration(
    access_key_id: String,
    secret_access_key: String,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    match cms_core::r2::configure(access_key_id, secret_access_key) {
        Ok(()) => {
            app.state::<CmsState>().reset().await;
            CommandResponse::Ready {
                data: "r2".to_owned(),
            }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn save_api_configuration(
    service: vesper_credentials::ConsumerApi,
    api_key: String,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    match vesper_credentials::save_consumer_api(service, &api_key) {
        Ok(()) => {
            app.state::<CmsState>().reset_views().await;
            CommandResponse::Ready {
                data: service.name().to_owned(),
            }
        }
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) fn read_publication() -> CommandResponse<social::PublicationConfigurationStatus> {
    match social::read_config() {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) fn save_telegram(
    credentials: vesper_credentials::TelegramCredentials,
) -> CommandResponse<String> {
    match vesper_credentials::save_telegram(credentials) {
        Ok(()) => CommandResponse::Ready {
            data: "telegram".to_owned(),
        },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) async fn connect_x(app: tauri::AppHandle) -> CommandResponse<String> {
    let state = app.state::<PublicationState>();
    let _operation = state.x_operation.lock().await;
    let authorization = match social::x_authorization().await {
        Ok(authorization) => authorization,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    if let Err(error) = app.opener().open_url(&authorization.url, None::<String>) {
        return CommandResponse::Failed {
            message: format!("Could not open X authorization: {error}"),
        };
    }
    let credentials = match social::authenticate_x(authorization).await {
        Ok(credentials) => credentials,
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    match vesper_credentials::save_x(credentials) {
        Ok(()) => CommandResponse::Ready {
            data: "x".to_owned(),
        },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) fn save_app_lock(password: String) -> CommandResponse<String> {
    match vesper_credentials::save_app_lock(&password) {
        Ok(()) => CommandResponse::Ready {
            data: "app-lock".to_owned(),
        },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) fn remove_app_lock() -> CommandResponse<String> {
    match vesper_credentials::delete_app_lock() {
        Ok(()) => CommandResponse::Ready {
            data: "app-lock".to_owned(),
        },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

#[tauri::command]
pub(crate) fn unlock_app(
    state: tauri::State<'_, AppLockState>,
    password: String,
) -> CommandResponse<String> {
    let stored = match vesper_credentials::app_lock() {
        Ok(vesper_credentials::Stored::Ready(app_lock)) => app_lock.password,
        Ok(vesper_credentials::Stored::Missing) => {
            return CommandResponse::Failed {
                message: "App Lock is not configured.".to_owned(),
            };
        }
        Err(error) => {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    if !state.unlock(&stored, &password) {
        return CommandResponse::Failed {
            message: "Incorrect password.".to_owned(),
        };
    }
    CommandResponse::Ready {
        data: "app-lock".to_owned(),
    }
}

#[derive(Default)]
pub(crate) struct AppLockState(std::sync::atomic::AtomicBool);

impl AppLockState {
    fn unlock(&self, stored: &str, supplied: &str) -> bool {
        if !passwords_match(stored, supplied) {
            return false;
        }
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
        true
    }

    pub(crate) fn locked(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[tauri::command]
pub(crate) fn read_app_lock(state: tauri::State<'_, AppLockState>) -> bool {
    state.locked()
}

#[tauri::command]
pub(crate) fn lock_app(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppLockState>,
) -> CommandResponse<()> {
    use tauri::Manager;
    match vesper_credentials::app_lock() {
        Ok(vesper_credentials::Stored::Ready(_)) => {
            state.0.store(true, std::sync::atomic::Ordering::SeqCst);
            if let Some(webview) = app.get_webview_window("main") {
                webview.close_devtools();
            }
            CommandResponse::Ready { data: () }
        }
        Ok(vesper_credentials::Stored::Missing) => CommandResponse::Failed {
            message: "App Lock is not configured.".to_owned(),
        },
        Err(error) => CommandResponse::Failed {
            message: error.to_string(),
        },
    }
}

fn passwords_match(stored: &str, supplied: &str) -> bool {
    if stored.len() != supplied.len() {
        return false;
    }
    stored
        .as_bytes()
        .iter()
        .zip(supplied.as_bytes())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::{AppLockState, passwords_match};

    #[test]
    fn locked_session_requires_valid_password_to_clear() {
        let state = AppLockState::default();
        state.0.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(state.locked());
        assert!(state.locked());
        assert!(!state.unlock("correct horse", "correct house"));
        assert!(state.locked());
        assert!(state.unlock("correct horse", "correct horse"));
        assert!(!state.locked());
    }

    #[test]
    fn compares_full_passwords() {
        assert!(passwords_match("correct horse", "correct horse"));
        assert!(!passwords_match("correct horse", "correct"));
        assert!(!passwords_match("correct horse", "correct house"));
    }
}
