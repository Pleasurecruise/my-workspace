mod accounts;
pub mod archive;
mod cache;
mod login;
mod mihoyo;
mod skland;
mod steam;
mod transport;

pub use accounts::{Connections, connections};
pub use login::{LoginProgress, LoginQr};
pub use mihoyo::rail_gacha::Report as StarRailReport;
pub use mihoyo::record::{
    BridgeMessage, BridgeResult, USER_AGENT as RECORD_USER_AGENT, VerificationPage,
};
pub use mihoyo::verification::{Captcha, CaptchaSolution};
use serde::{Deserialize, Serialize};
pub use vesper_credentials::games::Provider;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Game {
    Genshin,
    StarRail,
    Zzz,
    Arknights,
    Endfield,
}

impl Game {
    pub fn key(self) -> &'static str {
        match self {
            Self::Genshin => "genshin",
            Self::StarRail => "starRail",
            Self::Zzz => "zzz",
            Self::Arknights => "arknights",
            Self::Endfield => "endfield",
        }
    }
    pub fn provider(self) -> Provider {
        match self {
            Self::Genshin | Self::StarRail | Self::Zzz => Provider::Mihoyo,
            Self::Arknights | Self::Endfield => Provider::Skland,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Account {
    pub game: Game,
    pub uid: String,
    pub region: String,
    pub name: String,
    pub role_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Meter {
    pub label: String,
    pub current: u64,
    pub max: u64,
    pub full_at: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub label: String,
    pub value: String,
    pub progress: Option<TaskProgress>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TaskProgress {
    pub current: u64,
    pub total: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notes {
    pub account: Account,
    pub sampled_at: i64,
    pub meters: Vec<Meter>,
    pub tasks: Vec<Task>,
}

#[derive(Clone, Debug)]
pub enum NotesError {
    VerificationRequired(i64),
    RefreshRequired,
    Failed(String),
}

impl From<String> for NotesError {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

impl From<&str> for NotesError {
    fn from(message: &str) -> Self {
        Self::Failed(message.to_owned())
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum NotesResponse {
    Ready { data: Notes },
    VerificationRequired { code: i64, message: String },
    RefreshRequired { message: String },
    Failed { message: String },
}

impl From<Result<Notes, NotesError>> for NotesResponse {
    fn from(result: Result<Notes, NotesError>) -> Self {
        match result {
            Ok(data) => Self::Ready { data },
            Err(NotesError::VerificationRequired(code)) => Self::VerificationRequired {
                code,
                message: mihoyo::VERIFICATION_MESSAGE.to_owned(),
            },
            Err(NotesError::Failed(message)) => Self::Failed { message },
            Err(NotesError::RefreshRequired) => Self::RefreshRequired {
                message: "Verification accepted. Use the refresh icon to check game-record access."
                    .into(),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pull {
    pub id: String,
    pub pool: String,
    pub pool_name: String,
    pub item_id: Option<String>,
    pub name: String,
    pub rarity: u8,
    pub time: String,
    pub is_free: Option<bool>,
    pub is_new: Option<bool>,
}

pub struct Runtime {
    mihoyo: tokio::sync::Mutex<()>,
    skland: tokio::sync::Mutex<()>,
    steam: tokio::sync::Mutex<()>,
    login: login::Logins,
    record: tokio::sync::Mutex<std::collections::HashMap<String, mihoyo::record::RecordSession>>,
    notes: cache::Cache<(Game, String, String), Notes, NotesError>,
    verification:
        tokio::sync::Mutex<std::collections::HashMap<String, mihoyo::verification::Pending>>,
    archive: std::path::PathBuf,
}

impl Runtime {
    pub fn new(archive: std::path::PathBuf) -> Self {
        Self {
            mihoyo: Default::default(),
            skland: Default::default(),
            steam: Default::default(),
            login: Default::default(),
            record: Default::default(),
            notes: Default::default(),
            verification: Default::default(),
            archive,
        }
    }
    fn operation(&self, provider: Provider) -> &tokio::sync::Mutex<()> {
        match provider {
            Provider::Mihoyo => &self.mihoyo,
            Provider::Skland => &self.skland,
            Provider::Steam => &self.steam,
        }
    }
    pub async fn begin_login(&self, provider: Provider) -> Result<LoginQr, String> {
        self.login.begin(provider).await
    }
    pub async fn poll_login(&self, provider: Provider, id: &str) -> Result<LoginProgress, String> {
        let _guard = self.operation(provider).lock().await;
        self.login.poll(provider, id).await
    }
    pub async fn cancel_login(&self, provider: Provider, id: &str) {
        self.login.cancel(provider, id).await;
    }
    pub async fn select_account(&self, game: Game, id: Option<&str>) -> Result<(), String> {
        if game.provider() != Provider::Mihoyo {
            return Err("Only miHoYo supports account selection".into());
        }
        let _guard = self
            .mihoyo
            .try_lock()
            .map_err(|_| "miHoYo is busy. Try again shortly.")?;
        vesper_credentials::games::accounts::update(|accounts| accounts.select(game.key(), id))
            .map_err(|error| error.to_string())
    }
    pub async fn remove_account(&self, id: &str) -> Result<(), String> {
        let _guard = self
            .mihoyo
            .try_lock()
            .map_err(|_| "miHoYo is busy. Try again shortly.")?;
        vesper_credentials::games::accounts::update(|accounts| accounts.remove(id))
            .map_err(|error| error.to_string())?;
        self.record.lock().await.remove(id);
        Ok(())
    }
    pub async fn notes(&self, game: Game, refresh: bool) -> Result<Notes, NotesError> {
        let _guard = self.operation(game.provider()).lock().await;
        let session = transport::game(game)?;
        self.read_notes(&session, game, refresh).await
    }
    async fn read_notes(
        &self,
        session: &vesper_credentials::games::Session,
        game: Game,
        refresh: bool,
    ) -> Result<Notes, NotesError> {
        use vesper_credentials::games::Session;
        let (id, token) = match session {
            Session::Mihoyo {
                account_id, stoken, ..
            } => (account_id, stoken),
            Session::Skland { user_id, cred, .. } => (user_id, cred),
            Session::Steam { .. } => return Err("Steam does not provide daily notes".into()),
        };
        self.notes
            .read((game, id.clone(), token.clone()), refresh, async {
                match game.provider() {
                    Provider::Mihoyo => {
                        let Session::Mihoyo { account_id, .. } = session else {
                            return Err("Invalid miHoYo account".into());
                        };
                        let mut records = self.record.lock().await;
                        if records
                            .get(account_id)
                            .is_none_or(|record| !record.matches(session))
                        {
                            records.insert(
                                account_id.clone(),
                                mihoyo::record::RecordSession::create(session).await?,
                            );
                        }
                        let record = records
                            .get(account_id)
                            .ok_or("Game-record session unavailable")?;
                        mihoyo::notes(record, game).await
                    }
                    Provider::Skland => skland::notes(session, game)
                        .await
                        .map_err(NotesError::Failed),
                    Provider::Steam => unreachable!(),
                }
            })
            .await
    }
    pub async fn record_page(&self, game: Game) -> Result<VerificationPage, String> {
        if game.provider() != Provider::Mihoyo {
            return Err("This game does not use Miyoushe verification".into());
        }
        let _guard = self.operation(Provider::Mihoyo).lock().await;
        let session = transport::game(game)?;
        let vesper_credentials::games::Session::Mihoyo { account_id, .. } = &session else {
            return Err("Invalid miHoYo account".into());
        };
        // Opening verification is explicit: renew its derived CookieToken instead of
        // handing a possibly rejected in-memory token to the official page.
        let record = mihoyo::record::RecordSession::create(&session).await?;
        let mut records = self.record.lock().await;
        records.insert(account_id.clone(), record);
        let record = records
            .get(account_id)
            .ok_or("Game-record session unavailable")?;
        record.page(game).await
    }
    pub async fn steam(&self) -> Result<steam::Snapshot, String> {
        let _guard = self.steam.lock().await;
        steam::read(&transport::session(Provider::Steam)?).await
    }

    pub async fn sync(&self, game: Game) -> Result<archive::Summary, String> {
        enum Download {
            Records(Account, Vec<Pull>),
            Official(Account, StarRailReport),
        }
        let _guard = self
            .operation(game.provider())
            .try_lock()
            .map_err(|_| "This provider is busy. Try again shortly.".to_owned())?;
        let session = transport::game(game)?;
        let result = tokio::time::timeout(std::time::Duration::from_secs(300), async {
            match game.provider() {
                Provider::Mihoyo => {
                    let vesper_credentials::games::Session::Mihoyo { account_id, .. } = &session
                    else {
                        return Err("Invalid miHoYo account".into());
                    };
                    let mut records = self.record.lock().await;
                    if records
                        .get(account_id)
                        .is_none_or(|record| !record.matches(&session))
                    {
                        records.insert(
                            account_id.clone(),
                            mihoyo::record::RecordSession::create(&session)
                                .await
                                .map_err(|error| format!("Pull history session: {error}"))?,
                        );
                    }
                    let record = records
                        .get(account_id)
                        .ok_or("Game-record session unavailable")?
                        .clone();
                    drop(records);
                    if game == Game::StarRail {
                        let (account, report) = mihoyo::rail_gacha::read(&record).await?;
                        Ok(Download::Official(account, report))
                    } else {
                        mihoyo::pulls(&record, game)
                            .await
                            .map(|(account, pulls)| Download::Records(account, pulls))
                    }
                }
                Provider::Skland => skland::pulls(&session, game)
                    .await
                    .map(|(account, pulls)| Download::Records(account, pulls)),
                Provider::Steam => unreachable!(),
            }
        })
        .await
        .map_err(|_| {
            "Pull history sync timed out; the previous archive is unchanged.".to_owned()
        })?;
        let download = result?;
        let path = self.archive.clone();
        tokio::task::spawn_blocking(move || match download {
            Download::Official(account, report) => archive::save_official(&path, &account, report),
            Download::Records(account, pulls) => archive::merge(&path, &account, &pulls),
        })
        .await
        .map_err(|_| "Archive worker failed".to_owned())?
    }
    pub async fn summary(
        &self,
        game: Game,
        uid: Option<String>,
    ) -> Result<archive::Summary, String> {
        let path = self.archive.clone();
        tokio::task::spawn_blocking(move || archive::summary(&path, game, uid.as_deref()))
            .await
            .map_err(|_| "Archive worker failed".to_owned())?
    }
}

pub use steam::Snapshot as SteamSnapshot;

pub async fn save_steam(api_key: String, steam_id: String) -> Result<(), String> {
    let session = vesper_credentials::games::Session::Steam {
        api_key: api_key.trim().to_owned(),
        steam_id: steam_id.trim().to_owned(),
    };
    steam::read(&session).await?;
    vesper_credentials::games::save(&session).map_err(|error| error.to_string())
}
