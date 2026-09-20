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

fn content(command: Command, multiple: bool) -> Command {
    command
        .arg(
            Arg::new("input")
                .value_name("INPUT")
                .num_args(if multiple {
                    clap::builder::ValueRange::from(1..)
                } else {
                    1.into()
                })
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
            Command::new("list")
                .about("List newest memos (default 10, maximum 25)")
                .args([arg!([LIMIT])]),
        )
        .subcommand(content(
            Command::new("page").about("List a filtered page using JSON"),
            true,
        ))
        .subcommand(
            Command::new("search")
                .about("Search memo content")
                .args([arg!(<QUERY> ...)]),
        )
        .subcommand(content(
            Command::new("create").about("Create a private memo from Markdown"),
            true,
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
            true,
        ))
        .subcommand(content(
            Command::new("patch")
                .about("Update memo fields using JSON")
                .args([arg!(<ID>)]),
            true,
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
                .about("List article summaries")
                .args([arg!([CURSOR])]),
        )
        .subcommand(
            Command::new("get")
                .about("Read an article by UUID or URL")
                .args([arg!(<ID_OR_URL>)]),
        )
        .subcommand(content(
            Command::new("page").about("List summaries using JSON filters"),
            true,
        ))
        .subcommand(content(
            Command::new("create").about("Create an article using JSON"),
            true,
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
            .map(|(name, about)| content(Command::new(name).about(about).args([arg!(<ID>)]), true)),
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

    let moment = Command::new("moment")
        .about("Read and manage Moment photos")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("get")
                .about("Read one photo")
                .args([arg!(<ID>)]),
        )
        .subcommand(Command::new("list").about("List the latest 100 photos"))
        .subcommand(Command::new("tags").about("List photo tags"))
        .subcommand(
            Command::new("search")
                .about("Search photos")
                .args([arg!(<QUERY> ...)]),
        )
        .subcommand(content(
            Command::new("query").about("Query photos using JSON filters"),
            true,
        ))
        .subcommand(content(
            Command::new("create").about("Register uploaded image keys using JSON"),
            true,
        ))
        .subcommand(content(
            Command::new("update")
                .about("Update photo metadata using JSON")
                .args([arg!(<ID>)]),
            true,
        ))
        .subcommand(
            content(
                Command::new("upload-photo").about("Process, upload, and register a photo"),
                false,
            )
            .arg(arg!(<SOURCE_IMAGE>))
            .allow_missing_positional(true),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a photo through the consumer API")
                .args([arg!(<ID>)]),
        )
        .subcommand(
            Command::new("upload")
                .about("Upload an original or thumbnail to R2")
                .args([arg!(<R2_KEY>), arg!(<LOCAL_PATH>)]),
        )
        .subcommand(
            Command::new("download")
                .about("Download an R2 image")
                .args([arg!(<R2_KEY>), arg!(<LOCAL_PATH>)]),
        )
        .subcommand(
            Command::new("remove-object")
                .about("Remove a verified orphan from R2")
                .args([arg!(<R2_KEY>)]),
        );

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
        .subcommands([memo, knowledge, moment, todo, ledger, game, status])
        .after_help("Use 'vesper <command> --help' for details and 'vesper help <command>' to explore commands.")
}

#[cfg(test)]
#[path = "../tests/unit/arguments.rs"]
mod tests;
