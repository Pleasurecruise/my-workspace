use crate::{Game, Provider};
use serde::Serialize;
use vesper_credentials::{Stored, games::accounts};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    game: Game,
    account_id: Option<String>,
}

#[derive(Serialize)]
pub struct Connections {
    providers: Vec<Provider>,
    mihoyo: Vec<String>,
    bindings: Vec<Binding>,
}

pub fn connections() -> Result<Connections, String> {
    let accounts = accounts::read().map_err(|error| error.to_string())?;
    let mut providers = Vec::new();
    if !accounts.sessions.is_empty() {
        providers.push(Provider::Mihoyo);
    }
    for provider in [Provider::Skland, Provider::Steam] {
        if matches!(
            vesper_credentials::games::read(provider).map_err(|error| error.to_string())?,
            Stored::Ready(_)
        ) {
            providers.push(provider);
        }
    }
    let bindings = [Game::Genshin, Game::StarRail, Game::Zzz]
        .into_iter()
        .map(|game| Binding {
            game,
            account_id: accounts.bindings.get(game.key()).cloned(),
        })
        .collect();
    Ok(Connections {
        providers,
        mihoyo: accounts.sessions.into_keys().collect(),
        bindings,
    })
}
