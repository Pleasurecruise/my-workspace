use crate::{Provider, transport};
use base64::Engine;
use serde::Serialize;
use std::{collections::HashMap, sync::Mutex};
use vesper_credentials::games::Session;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginQr {
    pub id: String,
    pub image: String,
    pub expires_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LoginProgress {
    Waiting,
    Scanned,
    Expired,
    Complete,
}

#[derive(Clone)]
pub(crate) struct Pending {
    pub ticket: String,
    pub device: String,
}

pub(crate) enum Poll {
    Waiting,
    Scanned,
    Expired,
    Complete(Session),
}

struct Slot {
    id: String,
    expires_at: i64,
    pending: Option<Pending>,
}

#[derive(Default)]
pub(crate) struct Logins(Mutex<HashMap<Provider, Slot>>);

impl Logins {
    pub async fn begin(&self, provider: Provider) -> Result<LoginQr, String> {
        if provider == Provider::Steam {
            return Err("Steam uses a personal API key".to_owned());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let expires_at = transport::now() + 120;
        self.0
            .lock()
            .map_err(|_| "Login state unavailable")?
            .insert(
                provider,
                Slot {
                    id: id.clone(),
                    expires_at,
                    pending: None,
                },
            );
        let (pending, url) = match provider {
            Provider::Mihoyo => crate::mihoyo::begin().await?,
            Provider::Skland => crate::skland::begin().await?,
            Provider::Steam => unreachable!(),
        };
        let code =
            qrcode::QrCode::new(url.as_bytes()).map_err(|_| "Could not encode login QR code")?;
        let svg = code
            .render::<qrcode::render::svg::Color>()
            .min_dimensions(256, 256)
            .build();
        let image = format!(
            "data:image/svg+xml;base64,{}",
            base64::prelude::BASE64_STANDARD.encode(svg)
        );
        let mut slots = self.0.lock().map_err(|_| "Login state unavailable")?;
        let slot = slots
            .get_mut(&provider)
            .filter(|slot| slot.id == id)
            .ok_or("Login was replaced or cancelled")?;
        slot.pending = Some(pending);
        Ok(LoginQr {
            id,
            image,
            expires_at,
        })
    }

    pub async fn poll(&self, provider: Provider, id: &str) -> Result<LoginProgress, String> {
        let pending = {
            let slots = self.0.lock().map_err(|_| "Login state unavailable")?;
            let slot = slots
                .get(&provider)
                .filter(|slot| slot.id == id)
                .ok_or("Login was replaced or cancelled")?;
            if slot.expires_at <= transport::now() {
                return Ok(LoginProgress::Expired);
            }
            slot.pending.clone().ok_or("Login QR code is not ready")?
        };
        let result = match provider {
            Provider::Mihoyo => crate::mihoyo::poll(&pending).await?,
            Provider::Skland => crate::skland::poll(&pending).await?,
            Provider::Steam => return Err("Steam uses a personal API key".to_owned()),
        };
        let mut slots = self.0.lock().map_err(|_| "Login state unavailable")?;
        let slot = slots
            .get(&provider)
            .filter(|slot| slot.id == id)
            .ok_or("Login was replaced or cancelled")?;
        if slot.expires_at <= transport::now() {
            return Ok(LoginProgress::Expired);
        }
        match result {
            Poll::Waiting => Ok(LoginProgress::Waiting),
            Poll::Scanned => Ok(LoginProgress::Scanned),
            Poll::Expired => Ok(LoginProgress::Expired),
            Poll::Complete(session) => {
                vesper_credentials::games::save(&session).map_err(|error| error.to_string())?;
                slots.remove(&provider);
                Ok(LoginProgress::Complete)
            }
        }
    }

    pub async fn cancel(&self, provider: Provider, id: &str) {
        let Ok(mut slots) = self.0.lock() else {
            return;
        };
        if slots.get(&provider).is_some_and(|slot| slot.id == id) {
            slots.remove(&provider);
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/login.rs"]
mod tests;
