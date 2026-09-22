use std::{error::Error, process::ExitCode};

use tracing_subscriber::EnvFilter;

mod arguments;
mod game;
mod knowledge;
mod ledger;
mod memo;
mod moment;
mod status;
mod todo;

fn main() -> ExitCode {
    let arguments = match arguments::parse(std::env::args_os()) {
        Ok(arguments) => arguments,
        Err(error) => {
            let code = error.exit_code() as u8;
            if error.print().is_err() {
                return ExitCode::FAILURE;
            }
            return ExitCode::from(code);
        }
    };
    execute(arguments)
}

#[tokio::main]
async fn execute(arguments: Vec<String>) -> ExitCode {
    #[cfg(debug_assertions)]
    if let Err(error) = vesper_credentials::load_dev_environment() {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }
    if let Err(error) = init_logging() {
        eprintln!("error: failed to initialize logging: {error}");
        return ExitCode::FAILURE;
    }
    match run(arguments.into_iter()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn init_logging() -> Result<(), Box<dyn Error + Send + Sync>> {
    let filter = match std::env::var("RUST_LOG") {
        Ok(value) => EnvFilter::try_new(value)?,
        Err(std::env::VarError::NotPresent) => EnvFilter::new("info"),
        Err(error) => return Err(Box::new(error)),
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()?;

    Ok(())
}

async fn run(arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let arguments: Vec<String> = arguments.collect();
    let repository = std::env::current_dir().map_err(|error| error.to_string())?;

    match arguments.as_slice() {
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
        [command, sources @ ..] if command == "status" => status::run(sources).await,
        [command, arguments @ ..] if command == "game" => game::run(arguments).await,
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
        [domain, flag, date, action, rest @ ..] if domain == "ledger" && flag == "--date" => {
            ledger::run(action, rest, Some(date)).await
        }
        [domain, action, rest @ ..] if domain == "ledger" => ledger::run(action, rest, None).await,
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

async fn read_input(arguments: &[String]) -> Result<String, String> {
    match arguments {
        [flag, path] if flag == "--file" => tokio::fs::read_to_string(path)
            .await
            .map_err(|error| format!("could not read input file {path}: {error}")),
        [flag] if flag == "--stdin" => {
            use tokio::io::AsyncReadExt;
            let mut content = String::new();
            tokio::io::stdin()
                .read_to_string(&mut content)
                .await
                .map_err(|error| format!("could not read standard input: {error}"))?;
            Ok(content)
        }
        [separator, content @ ..] if separator == "--" && !content.is_empty() => {
            Ok(content.join(" "))
        }
        [first, ..] if !first.starts_with("--") => Ok(arguments.join(" ")),
        _ => Err("expected content, --file <path>, or --stdin".to_owned()),
    }
}

#[cfg(test)]
#[path = "../tests/unit/cli.rs"]
mod tests;
