use crate::content::Content;
use crate::{CommandError, CommandResponse};
use consumers::api::credentials::ConsumerApi;
use tauri::Manager;
use vault::Stored;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Configuration {
    ugos: Saved<ugos::credentials::Credentials>,
    r2: Saved<R2Configuration>,
    api: ApiConfiguration,
    ntfy: Saved<inbox::credentials::Credentials>,
    ntfy_dev: bool,
    codex_resets: ::todo::Subscription,
    notion_calendar: Saved<::todo::notion::configuration::Configuration>,
    app_lock: Saved<String>,
    app_lock_dev: bool,
    spotify: Saved<String>,
    qq_music: Saved<String>,
    publication: social::Configured,
}

#[derive(serde::Serialize)]
#[serde(tag = "status", content = "data", rename_all = "camelCase")]
enum Saved<T> {
    Missing,
    Ready(T),
}

impl<T> From<Stored<T>> for Saved<T> {
    fn from(stored: Stored<T>) -> Self {
        match stored {
            Stored::Missing => Self::Missing,
            Stored::Ready(value) => Self::Ready(value),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct R2Configuration {
    access_key_id: String,
    secret_access_key: String,
}

#[derive(serde::Serialize)]
struct ApiConfiguration {
    memos: Saved<String>,
    moment: Saved<String>,
    knowledge: Saved<String>,
}

#[tauri::command]
pub(crate) async fn read_configuration(app: tauri::AppHandle) -> CommandResponse<Configuration> {
    let result = async {
        let r2 = match ::cms::r2::credentials::read()? {
            Stored::Ready(credentials) => Saved::Ready(R2Configuration {
                access_key_id: credentials.access_key_id,
                secret_access_key: credentials.secret_access_key,
            }),
            Stored::Missing => Saved::Missing,
        };
        let api = ApiConfiguration {
            memos: consumers::api::credentials::read(ConsumerApi::Memos)?.into(),
            moment: consumers::api::credentials::read(ConsumerApi::Moment)?.into(),
            knowledge: consumers::api::credentials::read(ConsumerApi::Knowledge)?.into(),
        };
        let ntfy: Saved<_> = inbox::credentials::read()?.into();
        let ntfy_dev = matches!(&ntfy, Saved::Ready(credentials) if credentials.development);
        let (app_lock, app_lock_dev) = match crate::app_lock::read()? {
            Stored::Ready(password) => (Saved::Ready(password.value), password.development),
            Stored::Missing => (Saved::Missing, false),
        };
        let spotify = match music::spotify::credentials::read()? {
            Stored::Ready(credentials) => {
                Saved::Ready(credentials.web_client_id.unwrap_or_default())
            }
            Stored::Missing => Saved::Missing,
        };
        let qq_music = match music::qq::credentials::read()? {
            Stored::Ready(_) => Saved::Ready("renewable-session".to_owned()),
            Stored::Missing => Saved::Missing,
        };
        Ok::<_, CommandError>(Configuration {
            ugos: ugos::credentials::read()?.into(),
            r2,
            api,
            ntfy,
            ntfy_dev,
            codex_resets: app.state::<::todo::Store>().read_codex().await?,
            notion_calendar: ::todo::notion::configuration::read()?.into(),
            app_lock,
            app_lock_dev,
            spotify,
            qq_music,
            publication: social::configured()?,
        })
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn save_ntfy_configuration(
    configuration: inbox::credentials::Credentials,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let result = async {
        inbox::credentials::save(configuration)?;
        crate::inbox::restart(&app).await?;
        Ok::<_, CommandError>("ntfy-notifications".to_owned())
    };
    result.await.into()
}

#[tauri::command]
pub(crate) fn save_ugos_configuration(
    username: String,
    password: String,
) -> CommandResponse<String> {
    CommandResponse::from(ugos::configure(username, password).map(|()| "ugos".to_owned()))
}

#[tauri::command]
pub(crate) async fn save_r2_configuration(
    access_key_id: String,
    secret_access_key: String,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let result = async {
        ::cms::r2::configure(access_key_id, secret_access_key)?;
        app.state::<Content>().reset().await;
        Ok::<_, CommandError>("r2".to_owned())
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn save_api_configuration(
    service: ConsumerApi,
    api_key: String,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let result = async {
        consumers::api::credentials::save(service, &api_key)?;
        app.state::<Content>().reset_views().await;
        Ok::<_, CommandError>(service.name().to_owned())
    };
    result.await.into()
}

#[tauri::command]
pub(crate) async fn save_notion_calendar(
    configuration: ::todo::notion::configuration::Configuration,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let store = app.state::<::todo::Store>();
    let result = store.configure_notion(configuration).await;
    CommandResponse::from(result.map(|()| "notion-calendar".to_owned()))
}

#[tauri::command]
pub(crate) async fn save_codex_resets(
    configuration: ::todo::Subscription,
    app: tauri::AppHandle,
) -> CommandResponse<String> {
    let store = app.state::<::todo::Store>();
    let result = store.save_codex(configuration).await;
    CommandResponse::from(result.map(|()| "codex-resets".to_owned()))
}
