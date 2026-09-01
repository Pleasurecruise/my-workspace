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

pub(super) async fn run() -> Result<(), String> {
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
