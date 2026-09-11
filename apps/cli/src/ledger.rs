use crate::print_json;

pub async fn run(
    action: &str,
    arguments: &[String],
    selected_date: Option<&str>,
) -> Result<(), String> {
    let date = match selected_date {
        Some(date) => date.to_owned(),
        None => time::OffsetDateTime::now_local()
            .map_err(|error| error.to_string())?
            .date()
            .to_string(),
    };
    let store =
        ledger::Store::new(vesper_database::shared_path().map_err(|error| error.to_string())?);
    let snapshot = execute(&store, &date, action, arguments).await?;
    print_json(&snapshot)
}

async fn execute(
    store: &ledger::Store,
    date: &str,
    action: &str,
    arguments: &[String],
) -> Result<ledger::Snapshot, String> {
    let result = match (action, arguments) {
        ("list", []) => store.read(date).await,
        ("create", [amount, category]) => store.create(date, amount, category).await,
        ("update", [id, amount, category]) => store.update(date, id, amount, category).await,
        ("delete", [id]) => store.delete(date, id).await,
        _ => return Err("expected ledger [--date YYYY-MM-DD] list | create <amount> <category> | update <id> <amount> <category> | delete <id>".into()),
    };
    result.map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "../tests/unit/ledger.rs"]
mod tests;
