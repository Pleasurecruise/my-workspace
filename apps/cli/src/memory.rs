use crate::print_json;

pub async fn run(action: &str, arguments: &[String]) -> Result<(), String> {
    match (action, arguments) {
        ("graph", [flag, limit]) if flag == "--limit" => {
            let limit = limit
                .parse()
                .map_err(|error| format!("invalid graph limit {limit}: {error}"))?;
            print_json(&nowledge::graph::read(limit).await?)
        }
        ("search", [flag, limit, query @ ..]) if flag == "--limit" && !query.is_empty() => {
            let limit = limit
                .parse()
                .map_err(|error| format!("invalid search limit {limit}: {error}"))?;
            print_json(&nowledge::memories::search(&query.join(" "), limit).await?)
        }
        ("get", [id]) => print_json(&nowledge::memories::read(id).await?),
        (invalid_action, invalid_arguments) => Err(format!(
            "invalid memory arguments: {invalid_action} {}; run `vesper help`",
            invalid_arguments.join(" ")
        )),
    }
}
