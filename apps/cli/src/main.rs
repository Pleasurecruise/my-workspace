use std::process::ExitCode;

mod knowledge;
mod memo;
mod moment;
mod status;
mod todo;

#[tokio::main]
async fn main() -> ExitCode {
    #[cfg(debug_assertions)]
    if let Err(error) = vesper_credentials::load_dev_environment() {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }
    if let Err(error) = my_workspace_logger::init() {
        eprintln!("error: failed to initialize logging: {error}");
        return ExitCode::FAILURE;
    }
    match run(std::env::args().skip(1)).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let arguments: Vec<String> = arguments.collect();
    let repository = std::env::current_dir().map_err(|error| error.to_string())?;

    match arguments.as_slice() {
        [] => {
            print_help();
            Ok(())
        }
        [command] if command == "help" || command == "--help" || command == "-h" => {
            print_help();
            Ok(())
        }
        [command] if command == "build" => {
            let output = cms_core::build::build(&repository)
                .await
                .map_err(|error| error.to_string())?;
            let report = output.report();
            println!(
                "validated {} Markdown file(s) and {} asset(s); temporary output removed",
                report.markdown_files, report.copied_files
            );
            Ok(())
        }
        [command] if command == "publish" => publish(&repository, false).await,
        [command] if command == "status" => status::run().await,
        [command, flag] if command == "publish" && flag == "--live" => {
            publish(&repository, true).await
        }
        [domain, action, rest @ ..] if domain == "memo" => memo::run(action, rest).await,
        [domain, action, rest @ ..] if domain == "knowledge" => knowledge::run(action, rest).await,
        [domain, action, rest @ ..] if domain == "moment" => moment::run(action, rest).await,
        [domain, flag, date, action, rest @ ..] if domain == "todo" && flag == "--date" => {
            todo::run(action, rest, Some(date)).await
        }
        [domain, action, rest @ ..] if domain == "todo" => todo::run(action, rest, None).await,
        invalid_arguments => Err(format!(
            "invalid arguments: {}; run `vesper help`",
            invalid_arguments.join(" ")
        )),
    }
}

async fn publish(repository: &std::path::Path, live: bool) -> Result<(), String> {
    let output = cms_core::build::build(repository)
        .await
        .map_err(|error| error.to_string())?;
    let report = cms_core::publish::publish(output.directory(), live)
        .await
        .map_err(|error| error.to_string())?;
    println!(
        "{} {} object(s) from {} to {}/{}",
        if report.live {
            "published"
        } else {
            "previewed"
        },
        report.objects,
        report.source.display(),
        report.bucket,
        report.prefix
    );
    Ok(())
}

fn print_json(value: &impl serde::Serialize) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn print_help() {
    println!(
        "vesper\n\n\
         Commands:\n  \
         build             validate content using temporary output\n  \
         publish           compile, then preview the SDK upload\n  \
         publish --live    compile, upload with the SDK, then remove temporary output\n  \
         status            read UGOS and AI provider status as JSON\n  \
         memo tags                   list memo tags with counts\n  \
         memo list [limit]            list newest memos as JSON\n  \
         memo page <json>             list a filtered or paginated memo page\n  \
         memo search <query>          search memo bodies as JSON\n  \
         memo create <markdown>       create a private memo\n  \
         memo import-x <url> [public|private]  import an X post as a favorite\n  \
         memo update <id> <markdown>  replace a memo body\n  \
         memo patch <id> <json>       update typed memo fields\n  \
         memo visibility <id> <public|private>  change memo visibility\n  \
         memo pin <id>                 pin a memo\n  \
         memo unpin <id>               unpin a memo\n  \
         memo favorite <id>            favorite a memo\n  \
         memo unfavorite <id>          remove a memo from favorites\n  \
         memo archive <id>             archive a memo\n  \
         memo restore <id>             restore an archived memo\n  \
         memo delete <id>             permanently delete a memo\n  \
         knowledge list [cursor]       list Knowledge articles\n  \
         knowledge get <id>            read one Knowledge article\n  \
         knowledge create <json>       create an article from a typed JSON payload\n  \
         knowledge update-draft <id> <json>      update draft fields with expectedHash\n  \
         knowledge update-documents <id> <json>  update editions with expectedHash\n  \
         knowledge visibility <id> <json>        update visibility with expectedHash\n  \
         knowledge delete <id> <hash>  delete an unchanged article\n  \
         moment tags                   list Moment tags\n  \
         moment list [cursor]          list photos\n  \
         moment search <query>         search photo metadata\n  \
         moment create <json>          register uploaded R2 image keys and metadata\n  \
         moment upload-photo <json> <source>  prepare and upload PNG, JPEG, WebP, AVIF, or HEIC\n  \
         moment update <id> <json>     update photo metadata\n  \
         moment delete <id>            delete photo metadata\n  \
         moment upload <key> <path>     upload an original or thumbnail through the R2 SDK\n  \
         moment download <key> <path>   download an image through the R2 SDK\n  \
         moment remove-object <key>     remove an orphaned image object from R2\n  \
         todo --date <YYYY-MM-DD> <action> [...]  operate on another calendar day\n  \
         todo list                      list today's Todos as JSON\n  \
         todo get <id>                  read one Todo as JSON\n  \
         todo create <text>             create a Todo\n  \
         todo update <id> <text>        replace a Todo's text\n  \
         todo complete <id>             mark a Todo complete\n  \
         todo reopen <id>               mark a Todo incomplete\n  \
         todo delete <id>               delete a Todo"
    );
}

#[cfg(test)]
#[path = "../tests/unit/cli.rs"]
mod tests;
