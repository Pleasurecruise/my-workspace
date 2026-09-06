use games::{Game, Runtime};

pub(super) async fn run(arguments: &[String]) -> Result<(), String> {
    let path = vesper_database::shared_path().map_err(|error| error.to_string())?;
    let runtime = Runtime::new(path);
    if let [source] = arguments
        && source == "steam"
    {
        return crate::print_json(&runtime.steam().await?);
    }
    let [action, game] = arguments else {
        return Err("expected game <notes|archive|sync> <game> or game steam".into());
    };
    let game = match game.as_str() {
        "genshin" => Game::Genshin,
        "star-rail" => Game::StarRail,
        "zzz" => Game::Zzz,
        "arknights" => Game::Arknights,
        "endfield" => Game::Endfield,
        _ => return Err("unsupported game; run vesper help".into()),
    };
    match action.as_str() {
        "notes" => crate::print_json(&runtime.notes(game, false).await.map_err(
            |error| match error {
                games::NotesError::Failed(message) => message,
                games::NotesError::VerificationRequired(_) => {
                    "Complete game verification in Desktop, then retry".into()
                }
                games::NotesError::RefreshRequired => {
                    "Refresh the game session in Desktop, then retry".into()
                }
            },
        )?),
        "archive" => crate::print_json(&runtime.summary(game, None).await?),
        "sync" => crate::print_json(&runtime.sync(game).await?),
        _ => Err("expected game notes, archive, or sync".into()),
    }
}
