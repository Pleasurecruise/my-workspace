use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum Source<T> {
    Ready { data: T },
    Failed { message: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    task_manager: Source<ugos::TaskManagerSnapshot>,
    claude: Source<useage::claude::ClaudeUsage>,
    codex: Source<useage::codex::CodexUsage>,
    copilot: Source<useage::copilot::CopilotUsage>,
    grok: Source<useage::grok::GrokUsage>,
    open_code: Source<useage::opencode::OpenCodeUsage>,
    deep_seek: Source<useage::deepseek::DeepSeekBalance>,
    cherry_in: Source<useage::cherryin::CherryInBalance>,
}

pub(super) async fn run(sources: &[String]) -> Result<(), String> {
    if let [source] = sources {
        return match source.as_str() {
            "ugos" => super::print_json(
                &ugos::task_manager()
                    .await
                    .map_err(|error| error.to_string())?,
            ),
            "claude" => super::print_json(&useage::claude::read().await?),
            "codex" => super::print_json(&useage::codex::read().await?),
            "copilot" => super::print_json(&useage::copilot::read().await?),
            "grok" => super::print_json(&useage::grok::read().await?),
            "opencode" => super::print_json(&useage::opencode::read().await?),
            "deepseek" => super::print_json(&useage::deepseek::read().await?),
            "cherryin" => super::print_json(&useage::cherryin::read().await?),
            _ => Err(format!(
                "unknown status source: {source}; run `vesper help`"
            )),
        };
    }
    if !sources.is_empty() {
        return Err("status accepts at most one source".to_owned());
    }

    let (task_manager, claude, codex, copilot, grok, open_code, deep_seek, cherry_in) = tokio::join!(
        ugos::task_manager(),
        useage::claude::read(),
        useage::codex::read(),
        useage::copilot::read(),
        useage::grok::read(),
        useage::opencode::read(),
        useage::deepseek::read(),
        useage::cherryin::read(),
    );
    super::print_json(&Report {
        task_manager: match task_manager {
            Ok(data) => Source::Ready { data },
            Err(error) => Source::Failed {
                message: error.to_string(),
            },
        },
        claude: match claude {
            Ok(data) => Source::Ready { data },
            Err(message) => Source::Failed { message },
        },
        codex: match codex {
            Ok(data) => Source::Ready { data },
            Err(message) => Source::Failed { message },
        },
        copilot: match copilot {
            Ok(data) => Source::Ready { data },
            Err(message) => Source::Failed { message },
        },
        grok: match grok {
            Ok(data) => Source::Ready { data },
            Err(message) => Source::Failed { message },
        },
        open_code: match open_code {
            Ok(data) => Source::Ready { data },
            Err(message) => Source::Failed { message },
        },
        deep_seek: match deep_seek {
            Ok(data) => Source::Ready { data },
            Err(message) => Source::Failed { message },
        },
        cherry_in: match cherry_in {
            Ok(data) => Source::Ready { data },
            Err(message) => Source::Failed { message },
        },
    })
}
