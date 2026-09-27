use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command, arg};
use std::ffi::OsString;

pub(super) fn parse(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<Vec<String>, clap::Error> {
    let mut arguments: Vec<_> = arguments.into_iter().collect();
    if arguments.len() == 1 {
        arguments.push("--help".into());
    }
    let mut command = command();
    let matches = command.try_get_matches_from_mut(arguments)?;
    let mut normalized = Vec::new();
    normalize(&command, &matches, &mut normalized);
    Ok(normalized)
}

fn normalize(command: &Command, matches: &ArgMatches, output: &mut Vec<String>) {
    if let Some((name, child_matches)) = matches.subcommand() {
        output.push(name.to_owned());
        if matches!(name, "todo" | "ledger")
            && let Some(date) = child_matches.get_one::<String>("date")
        {
            output.extend(["--date".to_owned(), date.clone()]);
        }
        let child = command.find_subcommand(name).expect("parsed subcommand");
        normalize(child, child_matches, output);
        return;
    }
    let domain = output.first().map(String::as_str);
    if command.get_name() == "list" && matches!(domain, Some("memo" | "knowledge" | "photo")) {
        let mut filters = serde_json::Map::new();
        for (argument, field) in [
            ("cursor", "cursor"),
            ("search", "search"),
            ("visibility", "visibility"),
            ("from", "fromDate"),
            ("to", "toDate"),
        ] {
            if let Ok(Some(value)) = matches.try_get_one::<String>(argument) {
                filters.insert(field.into(), value.clone().into());
            }
        }
        if let Some(limit) = matches.get_one::<u64>("limit") {
            filters.insert("limit".into(), (*limit).into());
        }
        if let Some(tags) = matches.get_many::<String>("tag") {
            filters.insert("tags".into(), tags.cloned().collect::<Vec<_>>().into());
        }
        for (argument, field) in [
            ("archived", "archivedOnly"),
            ("favorites", "favoritesOnly"),
            ("updated", "sortByUpdated"),
        ] {
            if matches.try_get_one::<bool>(argument).ok().flatten() == Some(&true) {
                filters.insert(field.into(), true.into());
            }
        }
        let action = if domain == Some("photo") {
            "query"
        } else {
            "page"
        };
        output.pop();
        output.extend([
            action.to_owned(),
            serde_json::Value::Object(filters).to_string(),
        ]);
        return;
    }
    if domain == Some("photo") && command.get_name() == "upload" {
        if let Some(path) = matches.get_one::<String>("metadata") {
            output.extend(["--file".to_owned(), path.clone()]);
        } else {
            let metadata = serde_json::json!({
                "title": matches.get_one::<String>("title"),
                "description": matches.get_one::<String>("description"),
                "tags": matches.get_many::<String>("tag").into_iter().flatten().collect::<Vec<_>>(),
                "date": matches.get_one::<String>("date"),
            });
            output.push(metadata.to_string());
        }
        output.push(
            matches
                .get_one::<String>("SOURCE_IMAGE")
                .expect("required image")
                .clone(),
        );
        return;
    }
    if domain == Some("photo") && output.get(1).is_some_and(|value| value == "object") {
        output.remove(1);
        output[1] = format!("object-{}", command.get_name());
    }
    for argument in command.get_positionals() {
        let id = argument.get_id().as_str();
        if id == "input" {
            if let Some(path) = matches.get_one::<String>("file") {
                output.extend(["--file".to_owned(), path.clone()]);
                continue;
            }
            if matches.get_flag("stdin") {
                output.push("--stdin".to_owned());
                continue;
            }
            if let Some(values) = matches.get_many::<String>(id) {
                let content = values.cloned().collect::<Vec<_>>().join(" ");
                if content.starts_with("--") {
                    output.push("--".to_owned());
                }
                output.push(content);
            }
        } else if let Some(values) = matches.get_many::<String>(id) {
            output.extend(values.cloned());
        }
    }
    if command.get_name() == "publish" && matches.get_flag("live") {
        output.push("--live".to_owned());
    }
}

fn content(command: Command) -> Command {
    command
        .arg(
            Arg::new("input")
                .value_name("INPUT")
                .num_args(1..)
                .help("Inline Markdown or JSON; use -- before content starting with a dash"),
        )
        .arg(arg!(--file <PATH> "Read UTF-8 content from a file"))
        .arg(arg!(--stdin "Read UTF-8 content from standard input"))
        .group(
            ArgGroup::new("content")
                .args(["input", "file", "stdin"])
                .required(true),
        )
}

fn command() -> Command {
    let memo = Command::new("memo")
        .about("Read and manage Memos")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("get")
                .about("Read one memo")
                .args([arg!(<ID>)]),
        )
        .subcommand(Command::new("tags").about("List tags and counts"))
        .subcommand(
            Command::new("list").about("List and filter memos").args([
                arg!(--limit <COUNT> "Maximum results (1–25)")
                    .value_parser(clap::value_parser!(u64).range(1..=25))
                    .default_value("10"),
                arg!(--cursor <CURSOR> "Continue from nextCursor"),
                arg!(--tag <TAG> "Filter by tag; repeat for multiple tags")
                    .action(ArgAction::Append),
                arg!(--search <TEXT> "Search memo content"),
                arg!(--archived "Show archived memos").conflicts_with("favorites"),
                arg!(--favorites "Show favorites"),
                arg!(--updated "Sort by update time"),
            ]),
        )
        .subcommand(content(
            Command::new("page").about("List a filtered page using JSON"),
        ))
        .subcommand(
            Command::new("search")
                .about("Search memo content")
                .args([arg!(<QUERY> ...)]),
        )
        .subcommand(content(
            Command::new("create").about("Create a private memo from Markdown"),
        ))
        .subcommand(
            Command::new("import-x")
                .about("Import an X post as a favorite")
                .args([
                    arg!(<URL>),
                    arg!([VISIBILITY]).value_parser(["public", "private"]),
                ]),
        )
        .subcommand(content(
            Command::new("update")
                .about("Replace a memo's Markdown")
                .args([arg!(<ID>)]),
        ))
        .subcommand(content(
            Command::new("patch")
                .about("Update memo fields using JSON")
                .args([arg!(<ID>)]),
        ))
        .subcommand(
            Command::new("visibility")
                .about("Set memo visibility")
                .args([
                    arg!(<ID>),
                    arg!(<VISIBILITY>).value_parser(["public", "private"]),
                ]),
        )
        .subcommands(
            [
                ("pin", "Pin a memo"),
                ("unpin", "Unpin a memo"),
                ("favorite", "Favorite a memo"),
                ("unfavorite", "Unfavorite a memo"),
                ("archive", "Archive a memo"),
                ("restore", "Restore an archived memo"),
                ("delete", "Permanently delete a memo"),
            ]
            .map(|(name, about)| Command::new(name).about(about).args([arg!(<ID>)])),
        );

    let knowledge = Command::new("knowledge")
        .about("Read and manage Knowledge articles")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("list")
                .about("List and filter article summaries")
                .args([
                    arg!(--limit <COUNT> "Maximum results (1–100)")
                        .value_parser(clap::value_parser!(u64).range(1..=100))
                        .default_value("20"),
                    arg!(--cursor <CURSOR> "Continue from cursor"),
                    arg!(--tag <TAG> "Filter by tag; repeat for multiple tags")
                        .action(ArgAction::Append),
                    arg!(--visibility <VISIBILITY> "Filter visibility")
                        .value_parser(["public", "private"]),
                ]),
        )
        .subcommand(
            Command::new("get")
                .about("Read an article by UUID or URL")
                .args([arg!(<ID_OR_URL>)]),
        )
        .subcommand(content(
            Command::new("page").about("List summaries using JSON filters"),
        ))
        .subcommand(content(
            Command::new("create").about("Create an article using JSON"),
        ))
        .subcommands(
            [
                (
                    "update-draft",
                    "Update draft using JSON with expectedHash and expectedUpdatedAt",
                ),
                (
                    "update-documents",
                    "Update documents using JSON with expectedHash and expectedUpdatedAt",
                ),
                (
                    "visibility",
                    "Set visibility using JSON with expectedHash and expectedUpdatedAt",
                ),
            ]
            .map(|(name, about)| content(Command::new(name).about(about).args([arg!(<ID>)]))),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete an unchanged article")
                .args([
                    arg!(<ID>),
                    arg!(<EXPECTED_HASH>),
                    arg!(<EXPECTED_UPDATED_AT>),
                ]),
        );

    let photo = Command::new("photo")
        .about("Upload and manage photos in the Moment gallery")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("get")
                .about("Read one photo")
                .args([arg!(<ID>)]),
        )
        .subcommand(Command::new("list").about("List and filter gallery photos").args([
            arg!(--limit <COUNT> "Maximum results (1–100)").value_parser(clap::value_parser!(u64).range(1..=100)).default_value("20"),
            arg!(--tag <TAG> "Filter by tag; repeat for multiple tags").action(ArgAction::Append),
            arg!(--from <YYYY_MM_DD> "First capture date"),
            arg!(--to <YYYY_MM_DD> "Last capture date"),
            arg!(--search <TEXT> "Search photos").conflicts_with_all(["tag", "from", "to"]),
        ]))
        .subcommand(Command::new("tags").about("List photo tags"))
        .subcommand(
            Command::new("search")
                .about("Search photos")
                .args([arg!(<QUERY> ...)]),
        )
        .subcommand(content(
            Command::new("query").about("Query photos using JSON filters"),
        ))
        .subcommand(content(
            Command::new("register").about("Register existing image objects using JSON"),
        ))
        .subcommand(content(
            Command::new("update")
                .about("Update photo metadata using JSON")
                .args([arg!(<ID>)]),
        ))
        .subcommand(
            Command::new("upload")
                .about("Process an image, upload it, and add it to the gallery")
                .arg(arg!(<SOURCE_IMAGE> "Local PNG, JPEG, WebP, AVIF, or HEIC image"))
                .args([
                    arg!(--title <TITLE> "Photo title (default: file name)"),
                    arg!(--description <TEXT> "Photo description"),
                    arg!(--tag <TAG> "Photo tag; repeat for multiple tags").action(ArgAction::Append),
                    arg!(--date <DATE> "Capture time (default: image EXIF)"),
                    arg!(--metadata <PATH> "Read Upload metadata JSON from a file")
                        .conflicts_with_all(["title", "description", "tag", "date"]),
                ])
                .after_help("Example: vesper photo upload photo.heic --title 'Weekend walk' --tag travel\nEXIF supplies capture time and location unless metadata overrides them."),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a photo through the consumer API")
                .args([arg!(<ID>)]),
        )
        .subcommand(Command::new("object")
            .about("Manage raw R2 objects; uploads do not add photos to the gallery")
            .subcommand_required(true)
            .subcommands([
                Command::new("put").about("Upload an object without registering a photo")
                    .args([arg!(<R2_KEY>), arg!(<LOCAL_PATH>)]),
                Command::new("get").about("Download an object")
                    .args([arg!(<R2_KEY>), arg!(<LOCAL_PATH>)]),
                Command::new("delete").about("Delete a raw object; verify it is unreferenced first")
                    .args([arg!(<R2_KEY>)]),
            ]));

    let date = arg!(--date <YYYY_MM_DD> "Select a calendar day (default: today)").global(true);
    let todo = Command::new("todo")
        .about("Manage tasks, habits, and calendars")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(date.clone())
        .subcommands(
            [
                ("list", "Synchronize calendars and list tasks"),
                ("sync", "Synchronize configured calendars"),
                ("sync-ics", "Synchronize ICS calendars"),
                ("database-path", "Print the shared database path"),
                ("schedule-path", "Print the managed ICS directory"),
            ]
            .map(|(name, about)| Command::new(name).about(about)),
        )
        .subcommands(
            [
                ("get", "Read one task"),
                ("complete", "Complete a task"),
                ("reopen", "Reopen a task"),
                ("delete", "Delete a task"),
                ("check-in", "Check in a habit"),
                ("undo-check-in", "Undo a habit check-in"),
            ]
            .map(|(name, about)| Command::new(name).about(about).args([arg!(<ID>)])),
        )
        .subcommand(
            Command::new("create")
                .about("Create a task")
                .args([arg!(<TEXT> ...)]),
        )
        .subcommand(
            Command::new("update")
                .about("Update a task title")
                .args([arg!(<ID>), arg!(<TEXT> ...)]),
        )
        .subcommand(
            Command::new("check-ins")
                .about("Read habit states and history")
                .args([arg!(<HABIT_ID> ...)]),
        )
        .subcommand(
            Command::new("import-ics")
                .about("Import ICS schedules")
                .args([arg!(<PATH> ...)]),
        )
        .subcommand(
            Command::new("notion")
                .about("Configure the Notion calendar")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(Command::new("status").about("Read connection status"))
                .subcommand(
                    Command::new("connect")
                        .about("Connect a calendar view using the existing ntn login")
                        .args([arg!(<VIEW_URL>)]),
                )
                .subcommand(Command::new("disconnect").about("Disconnect the calendar view")),
        );

    let ledger = Command::new("ledger")
        .about("Manage local GBP expenses")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(date)
        .subcommand(Command::new("list").about("Read expenses and monthly totals"))
        .subcommand(Command::new("create").about("Create an expense").args([
            arg!(<AMOUNT>).allow_negative_numbers(true),
            arg!(<CATEGORY>),
            arg!([DESCRIPTION]),
        ]))
        .subcommand(Command::new("update").about("Update an expense").args([
            arg!(<ID>),
            arg!(<AMOUNT>).allow_negative_numbers(true),
            arg!(<CATEGORY>),
            arg!([DESCRIPTION]),
        ]))
        .subcommand(
            Command::new("delete")
                .about("Delete an expense")
                .args([arg!(<ID>)]),
        );

    let game = Command::new("game")
        .about("Read game activity and synchronize pull archives")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("steam").about("Read Steam activity"))
        .subcommands(
            [
                ("notes", "Read daily notes"),
                ("archive", "Read pull archive summary"),
                ("sync", "Synchronize pull history"),
            ]
            .map(|(name, about)| {
                Command::new(name)
                    .about(about)
                    .args([arg!(<GAME>).value_parser([
                        "genshin",
                        "star-rail",
                        "zzz",
                        "arknights",
                        "endfield",
                    ])])
            }),
        );

    let status = Command::new("status")
        .about("Read provider status; omit a source to read all usage sources")
        .subcommands(
            [
                ("ugos", "UGOS task manager"),
                ("claude", "Claude usage"),
                ("codex", "Codex usage"),
                ("copilot", "Copilot usage"),
                ("grok", "Grok usage"),
                ("opencode", "OpenCode usage"),
                ("deepseek", "DeepSeek balance"),
                ("cherryin", "CherryIN balance"),
                ("tokenflux", "TokenFlux usage"),
                ("dimagent", "DimAgent usage"),
                ("exchange", "Exchange rates"),
                ("quotation", "Quotation"),
                ("service-catalog", "Available service IDs"),
            ]
            .map(|(name, about)| Command::new(name).about(about)),
        )
        .subcommand(
            Command::new("weather")
                .about("Read weather")
                .args([arg!([LOCATION] ...)]),
        )
        .subcommand(
            Command::new("astronomy")
                .about("Read astronomy")
                .args([arg!([LOCATION] ...)]),
        )
        .subcommand(
            Command::new("stocks")
                .about("Read stock quotes")
                .args([arg!(<SYMBOL> ...)]),
        )
        .subcommand(
            Command::new("services")
                .about("Read service status")
                .args([arg!([SERVICE_ID] ...)]),
        )
        .subcommand(
            Command::new("github")
                .about("Read GitHub activity or repository details")
                .args([arg!([OWNER_REPOSITORY])]),
        );

    Command::new("vesper")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Vesper content, tasks, and provider tools")
        .subcommand_required(true)
        .subcommand(Command::new("build").about("Validate local content using temporary output"))
        .subcommand(Command::new("publish").about("Build and preview publication to R2")
            .arg(Arg::new("live").long("live").action(ArgAction::SetTrue).help("Upload the build to R2")))
        .subcommands([memo, knowledge, photo, todo, ledger, game, status])
        .after_help("Use 'vesper <command> --help' for details and 'vesper help <command>' to explore commands.")
}

#[cfg(test)]
#[path = "../tests/unit/arguments.rs"]
mod tests;
