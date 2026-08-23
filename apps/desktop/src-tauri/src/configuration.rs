use crate::{CmsState, CommandResponse};
use tauri::Manager;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigurationStatus {
    ugos: StoredConfiguration<UgosConfiguration>,
    r2: StoredConfiguration<R2Configuration>,
    api: ApiConfiguration,
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
    CommandResponse::Ready {
        data: ConfigurationStatus {
            ugos,
            r2,
            api: ApiConfiguration {
                memos,
                moment,
                knowledge,
            },
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
