use super::text::render_telegram;
use super::{
    MemoPublication, PublicationProvider, PublishError, PublishedPost, TelegramAuthorizationStatus,
    memo_url,
};
use diesel::prelude::*;
use grammers_client::{
    Client, SenderPool, SignInError,
    client::{LoginToken, PasswordToken},
    peer::Peer,
};
use grammers_session::{
    BoxFuture, Session, SessionData,
    types::{ChannelState, DcOption, PeerId, PeerInfo, UpdateState, UpdatesState},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use vesper_credentials::Stored;

const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Deserialize, Serialize)]
struct StoredSession {
    home_dc: i32,
    dc_options: HashMap<i32, DcOption>,
    peer_infos: HashMap<PeerId, PeerInfo>,
    updates_state: UpdatesState,
}

impl Default for StoredSession {
    fn default() -> Self {
        let session = SessionData::default();
        Self {
            home_dc: session.home_dc,
            dc_options: session.dc_options,
            peer_infos: session.peer_infos,
            updates_state: session.updates_state,
        }
    }
}

struct DatabaseSession {
    path: std::path::PathBuf,
    data: Mutex<StoredSession>,
}

diesel::table! {
    telegram_session (id) { id -> Integer, data -> Text, }
}

#[derive(Debug, thiserror::Error)]
enum SessionError {
    #[error("session lock is poisoned")]
    Lock,
    #[error(transparent)]
    Database(#[from] vesper_database::Error),
    #[error("session database operation failed")]
    Query(#[from] diesel::result::Error),
    #[error("session data is invalid")]
    Codec(#[from] serde_json::Error),
}

impl DatabaseSession {
    fn open(path: &Path) -> Result<Self, SessionError> {
        let mut connection = vesper_database::open(path)?;
        let encoded = telegram_session::table
            .find(1)
            .select(telegram_session::data)
            .first::<String>(&mut connection)
            .optional()?;
        let data = match encoded {
            Some(encoded) => serde_json::from_str(&encoded)?,
            None => StoredSession::default(),
        };
        Ok(Self {
            path: path.to_owned(),
            data: Mutex::new(data),
        })
    }

    fn data(&self) -> Result<MutexGuard<'_, StoredSession>, SessionError> {
        self.data.lock().map_err(|_| SessionError::Lock)
    }

    fn persist(&self, data: &StoredSession) -> Result<(), SessionError> {
        let encoded = serde_json::to_string(data)?;
        let mut connection = vesper_database::open(&self.path)?;
        diesel::insert_into(telegram_session::table)
            .values((
                telegram_session::id.eq(1),
                telegram_session::data.eq(&encoded),
            ))
            .on_conflict(telegram_session::id)
            .do_update()
            .set(telegram_session::data.eq(&encoded))
            .execute(&mut connection)?;
        Ok(())
    }
}

impl Session for DatabaseSession {
    type Error = SessionError;

    fn home_dc_id(&self) -> Result<i32, Self::Error> {
        Ok(self.data()?.home_dc)
    }

    fn set_home_dc_id(&self, dc_id: i32) -> BoxFuture<'_, Result<(), Self::Error>> {
        Box::pin(async move {
            let mut current = self.data()?;
            let mut data = current.clone();
            data.home_dc = dc_id;
            self.persist(&data)?;
            *current = data;
            Ok(())
        })
    }

    fn dc_option(&self, dc_id: i32) -> Result<Option<DcOption>, Self::Error> {
        Ok(self.data()?.dc_options.get(&dc_id).cloned())
    }

    fn set_dc_option(&self, dc_option: &DcOption) -> BoxFuture<'_, Result<(), Self::Error>> {
        let dc_option = dc_option.clone();
        Box::pin(async move {
            let mut current = self.data()?;
            let mut data = current.clone();
            data.dc_options.insert(dc_option.id, dc_option);
            self.persist(&data)?;
            *current = data;
            Ok(())
        })
    }

    fn peer(&self, peer: PeerId) -> BoxFuture<'_, Result<Option<PeerInfo>, Self::Error>> {
        Box::pin(async move { Ok(self.data()?.peer_infos.get(&peer).cloned()) })
    }

    fn cache_peer(&self, peer: &PeerInfo) -> BoxFuture<'_, Result<(), Self::Error>> {
        let peer = peer.clone();
        Box::pin(async move {
            let mut current = self.data()?;
            let mut data = current.clone();
            data.peer_infos
                .entry(peer.id())
                .or_insert_with(|| peer.clone())
                .extend_info(&peer);
            self.persist(&data)?;
            *current = data;
            Ok(())
        })
    }

    fn updates_state(&self) -> BoxFuture<'_, Result<UpdatesState, Self::Error>> {
        Box::pin(async move { Ok(self.data()?.updates_state.clone()) })
    }

    fn set_update_state(&self, update: UpdateState) -> BoxFuture<'_, Result<(), Self::Error>> {
        Box::pin(async move {
            let mut current = self.data()?;
            let mut data = current.clone();
            match update {
                UpdateState::All(state) => data.updates_state = state,
                UpdateState::Primary { pts, date, seq } => {
                    data.updates_state.pts = pts;
                    data.updates_state.date = date;
                    data.updates_state.seq = seq;
                }
                UpdateState::Secondary { qts } => data.updates_state.qts = qts,
                UpdateState::Channel { id, pts } => {
                    data.updates_state
                        .channels
                        .retain(|channel| channel.id != id);
                    data.updates_state.channels.push(ChannelState { id, pts });
                }
            }
            self.persist(&data)?;
            *current = data;
            Ok(())
        })
    }
}

struct Runner(tokio::task::JoinHandle<()>);

impl Drop for Runner {
    fn drop(&mut self) {
        self.0.abort();
    }
}

enum LoginStep {
    Code(LoginToken),
    Password(Box<PasswordToken>),
}

pub struct TelegramLogin {
    client: Client,
    _runner: Runner,
    step: Option<LoginStep>,
}

impl Drop for TelegramLogin {
    fn drop(&mut self) {
        self.client.disconnect();
    }
}

impl TelegramLogin {
    pub fn can_continue(&self) -> bool {
        self.step.is_some()
    }

    pub async fn complete_code(
        &mut self,
        code: &str,
    ) -> Result<TelegramAuthorizationStatus, PublishError> {
        let code = code.trim();
        if code.is_empty() || code.len() > 16 || code.chars().any(char::is_whitespace) {
            return Err(PublishError::Authorization("the login code is invalid"));
        }
        let Some(LoginStep::Code(token)) = self.step.as_ref() else {
            return Err(PublishError::Authorization("a login code is not expected"));
        };
        let result = tokio::time::timeout(TIMEOUT, self.client.sign_in(token, code))
            .await
            .map_err(|_| PublishError::Authorization("the login request timed out"))?;
        match result {
            Ok(_) => {
                self.step = None;
                Ok(TelegramAuthorizationStatus::Ready)
            }
            Err(SignInError::PasswordRequired(token)) => {
                let hint = token.hint().map(str::to_owned);
                self.step = Some(LoginStep::Password(Box::new(token)));
                Ok(TelegramAuthorizationStatus::PasswordRequired { hint })
            }
            Err(SignInError::InvalidCode) => {
                Err(PublishError::Authorization("the login code is invalid"))
            }
            Err(SignInError::SignUpRequired) => Err(PublishError::Authorization(
                "the account must be created with an official Telegram client",
            )),
            Err(SignInError::InvalidPassword(_)) | Err(SignInError::Other(_)) => Err(
                PublishError::Authorization("Telegram rejected the login request"),
            ),
        }
    }

    pub async fn complete_password(
        &mut self,
        password: &str,
    ) -> Result<TelegramAuthorizationStatus, PublishError> {
        if password.is_empty() {
            return Err(PublishError::Authorization("the 2FA password is empty"));
        }
        let Some(LoginStep::Password(token)) = self.step.take() else {
            return Err(PublishError::Authorization(
                "a 2FA password is not expected",
            ));
        };
        let result = tokio::time::timeout(
            TIMEOUT,
            self.client.check_password(*token, password.as_bytes()),
        )
        .await
        .map_err(|_| PublishError::Authorization("the 2FA request timed out"))?;
        match result {
            Ok(_) => Ok(TelegramAuthorizationStatus::Ready),
            Err(SignInError::InvalidPassword(token))
            | Err(SignInError::PasswordRequired(token)) => {
                self.step = Some(LoginStep::Password(Box::new(token)));
                Err(PublishError::Authorization("the 2FA password is invalid"))
            }
            Err(SignInError::SignUpRequired)
            | Err(SignInError::InvalidCode)
            | Err(SignInError::Other(_)) => Err(PublishError::Authorization(
                "Telegram rejected the 2FA request",
            )),
        }
    }
}

pub async fn begin_login(
    session_path: &Path,
    phone: &str,
) -> Result<(TelegramAuthorizationStatus, Option<TelegramLogin>), PublishError> {
    let credentials = match vesper_credentials::telegram()? {
        Stored::Ready(credentials) => credentials,
        Stored::Missing => return Err(PublishError::MissingCredentials("Telegram")),
    };
    let phone =
        normalize_phone(phone).ok_or(PublishError::Authorization("the phone number is invalid"))?;
    let (client, runner) = connect(session_path, credentials.api_id).await?;
    let authorized = tokio::time::timeout(TIMEOUT, client.is_authorized())
        .await
        .map_err(|_| PublishError::Authorization("the authorization check timed out"))?
        .map_err(|_| PublishError::Authorization("could not check the Telegram session"))?;
    if authorized {
        client.disconnect();
        return Ok((TelegramAuthorizationStatus::Ready, None));
    }
    let token = tokio::time::timeout(
        TIMEOUT,
        client.request_login_code(&phone, &credentials.api_hash),
    )
    .await
    .map_err(|_| PublishError::Authorization("the login-code request timed out"))?
    .map_err(|_| PublishError::Authorization("Telegram rejected the login-code request"))?;
    Ok((
        TelegramAuthorizationStatus::CodeRequired,
        Some(TelegramLogin {
            client,
            _runner: runner,
            step: Some(LoginStep::Code(token)),
        }),
    ))
}

pub async fn read_auth(session_path: &Path) -> Result<TelegramAuthorizationStatus, PublishError> {
    let credentials = match vesper_credentials::telegram()? {
        Stored::Ready(credentials) => credentials,
        Stored::Missing => return Err(PublishError::MissingCredentials("Telegram")),
    };
    let mut connection = vesper_database::open(session_path)
        .map_err(|_| PublishError::Session("could not open local storage"))?;
    let exists = diesel::select(diesel::dsl::exists(telegram_session::table.find(1)))
        .get_result::<bool>(&mut connection)
        .map_err(|_| PublishError::Session("could not read local session"))?;
    if !exists {
        return Ok(TelegramAuthorizationStatus::Disconnected);
    }
    let (client, _runner) = connect(session_path, credentials.api_id).await?;
    let authorized = tokio::time::timeout(TIMEOUT, client.is_authorized())
        .await
        .map_err(|_| PublishError::Authorization("the authorization check timed out"))?
        .map_err(|_| PublishError::Authorization("could not check the Telegram session"))?;
    client.disconnect();
    Ok(if authorized {
        TelegramAuthorizationStatus::Ready
    } else {
        TelegramAuthorizationStatus::Disconnected
    })
}

pub async fn publish(
    memo: &MemoPublication,
    session_path: &Path,
) -> Result<PublishedPost, PublishError> {
    let memo_url = memo_url(memo)?;
    let credentials = match vesper_credentials::telegram()? {
        Stored::Ready(credentials) => credentials,
        Stored::Missing => return Err(PublishError::MissingCredentials("Telegram")),
    };
    let text = render_telegram(&memo.content, &memo_url);
    let (client, _runner) = connect(session_path, credentials.api_id).await?;
    let authorized = tokio::time::timeout(TIMEOUT, client.is_authorized())
        .await
        .map_err(|_| PublishError::Request("Telegram"))?
        .map_err(|_| PublishError::Request("Telegram"))?;
    if !authorized {
        client.disconnect();
        return Err(PublishError::Session("the user account is not authorized"));
    }
    let peer = tokio::time::timeout(
        TIMEOUT,
        client.resolve_username(&credentials.channel_username),
    )
    .await
    .map_err(|_| PublishError::Request("Telegram"))?
    .map_err(|_| PublishError::Request("Telegram"))?
    .ok_or(PublishError::Session(
        "the configured channel was not found",
    ))?;
    let Peer::Channel(channel) = peer else {
        client.disconnect();
        return Err(PublishError::Session(
            "the configured username is not a broadcast channel",
        ));
    };
    let channel_ref = tokio::time::timeout(TIMEOUT, channel.to_ref())
        .await
        .map_err(|_| PublishError::Request("Telegram"))?
        .map_err(|_| PublishError::Request("Telegram"))?
        .ok_or(PublishError::Session(
            "the configured channel reference is unavailable",
        ))?;
    let message = tokio::time::timeout(TIMEOUT, client.send_message(channel_ref, text))
        .await
        .map_err(|_| PublishError::Request("Telegram"))?
        .map_err(|_| PublishError::Request("Telegram"))?;
    client.disconnect();
    Ok(PublishedPost {
        provider: PublicationProvider::Telegram,
        external_id: message.id().to_string(),
        url: Some(format!(
            "https://t.me/{}/{}",
            credentials.channel_username,
            message.id()
        )),
    })
}

fn normalize_phone(phone: &str) -> Option<String> {
    let phone = phone.trim();
    if phone.is_empty()
        || phone.chars().enumerate().any(|(index, character)| {
            !(character.is_ascii_digit()
                || character.is_ascii_whitespace()
                || matches!(character, '-' | '(' | ')')
                || (character == '+' && index == 0))
        })
    {
        return None;
    }
    let digits: String = phone
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect();
    (7..=15)
        .contains(&digits.len())
        .then(|| format!("+{digits}"))
}

async fn connect(session_path: &Path, api_id: i32) -> Result<(Client, Runner), PublishError> {
    if let Some(parent) = session_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|_| PublishError::Session("could not create local storage"))?;
    }
    let session = Arc::new(
        DatabaseSession::open(session_path)
            .map_err(|_| PublishError::Session("could not open local storage"))?,
    );
    let SenderPool { runner, handle, .. } = SenderPool::new(session, api_id);
    Ok((Client::new(handle), Runner(tokio::spawn(runner.run()))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_phone() {
        assert_eq!(
            normalize_phone("+86 138-0013-8000"),
            Some("+8613800138000".to_owned())
        );
        assert_eq!(normalize_phone("account13800138000"), None);
        assert_eq!(normalize_phone("123"), None);
    }

    #[tokio::test]
    async fn protects_session() {
        let directory =
            std::env::temp_dir().join(format!("vesper-session-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join(vesper_database::FILE_NAME);
        let session = DatabaseSession::open(&path).unwrap();
        session.set_home_dc_id(4).await.unwrap();
        drop(session);

        let restored = DatabaseSession::open(&path).unwrap();
        assert_eq!(restored.home_dc_id().unwrap(), 4);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
