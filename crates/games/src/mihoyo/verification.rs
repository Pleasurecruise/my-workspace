// Adapted from Snap.Hutao.Remastered's CardClient / GeetestService.
// Copyright (c) 2022-2025 DGP Studio; (c) 2026 Snap.HutaoRemasteringProject.
// Licensed under MIT. Star Rail uses the official RPG toolcomsrv protocol.
use super::{Envelope, record::RecordSession};
use crate::{Game, Runtime, transport};
use serde::{Deserialize, Serialize};
use vesper_credentials::games::Session;

const REGISTER: &str = "https://api-takumi-record.mihoyo.com/game_record/app/card/wapi/createVerification?is_high=true";
const VERIFY: &str =
    "https://api-takumi-record.mihoyo.com/game_record/app/card/wapi/verifyVerification";
const RAIL_REGISTER: &str = "https://api-takumi.mihoyo.com/event/toolcomsrv/risk/createGeetest?app_key=hkrpg_game_record&is_high=true";
const RAIL_VERIFY: &str = "https://api-takumi.mihoyo.com/event/toolcomsrv/risk/verifyGeetest";

pub struct Captcha {
    pub id: String,
    pub gt: String,
    pub challenge: String,
    pub new_captcha: Option<bool>,
    pub success: Option<u8>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptchaSolution {
    geetest_challenge: String,
    geetest_validate: String,
    geetest_seccode: String,
}

#[derive(Deserialize)]
struct Registration {
    gt: String,
    challenge: String,
    new_captcha: Option<bool>,
    success: Option<u8>,
}
#[derive(Deserialize)]
struct Verified {
    challenge: String,
}
#[derive(Clone)]
pub(crate) struct Pending {
    game: Game,
    record: RecordSession,
    expires_at: i64,
    trace: Option<String>,
}

fn context(game: Game) -> Result<(&'static str, &'static str), String> {
    match game {
        Game::Genshin => Ok(("2", super::notes_endpoint(game)?)),
        Game::StarRail => Ok(("6", super::notes_endpoint(game)?)),
        _ => Err("This game does not support this verification flow.".into()),
    }
}

fn card_request(
    record: &RecordSession,
    game: Game,
    body: Option<&str>,
    trace: Option<&str>,
) -> Result<reqwest::RequestBuilder, String> {
    let (id, path) = context(game)?;
    let endpoint = match (game, body.is_some()) {
        (Game::StarRail, false) => RAIL_REGISTER,
        (Game::StarRail, true) => RAIL_VERIFY,
        (_, false) => REGISTER,
        (_, true) => VERIFY,
    };
    let url = reqwest::Url::parse(endpoint).map_err(|_| "Invalid verification endpoint")?;
    let mut request = record
        .request(game, url, body)?
        .header("x-rpc-challenge_game", id)
        .header("x-rpc-challenge_path", path);
    if let Some(trace) = trace {
        request = request.header("x-rpc-challenge_trace", trace);
    }
    Ok(request)
}

async fn request<T: serde::de::DeserializeOwned>(
    record: &RecordSession,
    game: Game,
    body: Option<&str>,
    trace: Option<&str>,
    diagnostic_path: &std::path::Path,
) -> Result<T, String> {
    let response: Envelope = transport::json(card_request(record, game, body, trace)?).await?;
    let diagnostic = Diagnostic {
        game,
        stage: if body.is_some() { "submit" } else { "register" },
        retcode: response.retcode,
        has_trace: trace.is_some(),
        timestamp: transport::now(),
    };
    if diagnostic.save(diagnostic_path).await.is_err() {
        tracing::warn!("Could not save miHoYo verification diagnostics");
    }
    if response.retcode != 0 {
        tracing::warn!(
            game = game.key(),
            stage = if body.is_some() { "submit" } else { "register" },
            retcode = response.retcode,
            has_trace = trace.is_some(),
            "miHoYo verification rejected"
        );
    }
    response.verification()
}

impl Envelope {
    fn verification<T: serde::de::DeserializeOwned>(self) -> Result<T, String> {
        if self.retcode == 30001 {
            return Err("miHoYo reports that this request needs no captcha, but did not authorize game-record access. Close the window and manually refresh the card to check access.".into());
        }
        self.decode()
    }
}

// Deliberately excludes account identity, provider messages, cookies and captcha values.
#[derive(Clone, Serialize)]
struct Diagnostic {
    game: Game,
    stage: &'static str,
    retcode: i64,
    has_trace: bool,
    timestamp: i64,
}

diesel::table! {
    game_diagnostic (id) {
        id -> Integer, game -> Text, stage -> Text, retcode -> BigInt,
        has_trace -> Bool, timestamp -> BigInt,
    }
}

impl Diagnostic {
    async fn save(&self, path: &std::path::Path) -> Result<(), String> {
        use diesel::prelude::*;
        let path = path.to_owned();
        let diagnostic = self.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = vesper_database::open(&path).map_err(|error| error.to_string())?;
            let values = (
                game_diagnostic::game.eq(diagnostic.game.key()),
                game_diagnostic::stage.eq(diagnostic.stage),
                game_diagnostic::retcode.eq(diagnostic.retcode),
                game_diagnostic::has_trace.eq(diagnostic.has_trace),
                game_diagnostic::timestamp.eq(diagnostic.timestamp),
            );
            diesel::insert_into(game_diagnostic::table)
                .values((game_diagnostic::id.eq(1), values))
                .on_conflict(game_diagnostic::id)
                .do_update()
                .set(values)
                .execute(&mut connection)
                .map_err(|error| error.to_string())?;
            Ok(())
        })
        .await
        .map_err(|error| error.to_string())?
    }
}

fn valid_field(value: &str) -> bool {
    !value.is_empty() && value.len() <= 2048 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

impl CaptchaSolution {
    fn body(&self, game: Game) -> Result<String, String> {
        #[derive(Serialize)]
        struct RailProof<'a> {
            #[serde(flatten)]
            proof: &'a CaptchaSolution,
            app_key: &'static str,
        }
        let body = if game == Game::StarRail {
            serde_json::to_string(&RailProof {
                proof: self,
                app_key: "hkrpg_game_record",
            })
        } else {
            serde_json::to_string(self)
        };
        body.map_err(|_| "Could not encode the verification result.".into())
    }

    fn validate(&self) -> Result<(), String> {
        if !valid_field(&self.geetest_challenge)
            || !valid_field(&self.geetest_validate)
            || self.geetest_seccode != format!("{}|jordan", self.geetest_validate)
        {
            return Err(
                "The verification result is incomplete. Close this window and verify again.".into(),
            );
        }
        Ok(())
    }
}

impl Runtime {
    async fn mark_verified(&self, record: &RecordSession, game: Game) {
        if let Session::Mihoyo {
            account_id, stoken, ..
        } = &record.session
        {
            self.notes
                .map_error(&(game, account_id.clone(), stoken.clone()), |error| {
                    if matches!(error, crate::NotesError::VerificationRequired(_)) {
                        crate::NotesError::RefreshRequired
                    } else {
                        error
                    }
                })
                .await;
        }
    }

    pub async fn begin_verification(&self, game: Game) -> Result<Captcha, String> {
        context(game)?;
        let _guard = self.mihoyo.lock().await;
        {
            let mut pending = self.verification.lock().await;
            pending.retain(|_, entry| entry.expires_at > transport::now());
            if !pending.is_empty() {
                return Err("Complete or close the current verification window first.".into());
            }
        }
        let session = transport::game(game)?;
        if !matches!(session, Session::Mihoyo { .. }) {
            return Err("Connect your miHoYo account first.".into());
        }
        let record = self.load_record(&session).await?;
        let trace = record
            .verification_traces
            .lock()
            .await
            .get(game.key())
            .cloned();
        let registration: Registration =
            request(&record, game, None, trace.as_deref(), &self.archive)
                .await
                .map_err(|error| format!("Could not start verification: {error}"))?;
        if !valid_field(&registration.gt) || !valid_field(&registration.challenge) {
            return Err(
                "miHoYo returned an invalid challenge. Please try again manually later.".into(),
            );
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.verification.lock().await.insert(
            id.clone(),
            Pending {
                game,
                record,
                expires_at: transport::now() + 600,
                trace,
            },
        );
        Ok(Captcha {
            id,
            gt: registration.gt,
            challenge: registration.challenge,
            new_captcha: registration.new_captcha,
            success: registration.success,
        })
    }

    pub async fn complete_verification(
        &self,
        id: &str,
        solution: CaptchaSolution,
    ) -> Result<(), String> {
        solution.validate()?;
        let _guard = self.mihoyo.lock().await;
        let pending = self
            .verification
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or("Verification was cancelled or has ended.")?;
        if pending.expires_at <= transport::now() {
            return Err("The challenge has expired. Reopen the verification window.".into());
        }
        let session = transport::game(pending.game)?;
        if !pending.record.matches(&session) {
            return Err("Your login has changed. Please verify again.".into());
        }
        let body = solution.body(pending.game)?;
        let verified: Verified = request(
            &pending.record,
            pending.game,
            Some(&body),
            pending.trace.as_deref(),
            &self.archive,
        )
        .await
        .map_err(|error| format!("Verification failed: {error}"))?;
        self.finish_verification(id, verified.challenge).await
    }

    async fn finish_verification(&self, id: &str, challenge: String) -> Result<(), String> {
        if !valid_field(&challenge) {
            return Err("miHoYo did not confirm verification. Please verify again.".into());
        }
        let mut active = self.verification.lock().await;
        let pending = active
            .remove(id)
            .ok_or("The verification window was closed. The result was discarded.")?;
        if pending.expires_at <= transport::now() {
            return Err("The challenge has expired. Please verify again.".into());
        }
        let mut challenges = pending.record.challenges.lock().await;
        challenges.insert(pending.game.key().into(), challenge);
        drop(challenges);
        self.mark_verified(&pending.record, pending.game).await;
        Ok(())
    }

    pub async fn cancel_verification(&self, id: &str) {
        self.verification.lock().await.remove(id);
    }
}

#[cfg(test)]
#[path = "../../tests/unit/mihoyo/verification.rs"]
mod tests;
