use crate::{CommandResponse, github, weather};

#[tauri::command]
pub(crate) async fn read_task_manager() -> CommandResponse<ugos::TaskManagerSnapshot> {
    match ugos::task_manager().await {
        Ok(data) => {
            tracing::info!("loaded UGOS Task Manager");
            CommandResponse::Ready { data }
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to load UGOS Task Manager");
            CommandResponse::Failed {
                message: error.to_string(),
            }
        }
    }
}

#[tauri::command]
pub(crate) async fn read_codex_usage() -> CommandResponse<useage::codex::CodexUsage> {
    match useage::codex::read().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => {
            tracing::warn!(error = %error, "failed to load Codex usage");
            CommandResponse::Failed { message: error }
        }
    }
}

#[tauri::command]
pub(crate) async fn read_opencode_usage() -> CommandResponse<useage::opencode::OpenCodeUsage> {
    match useage::opencode::read().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => {
            tracing::warn!(error = %error, "failed to load OpenCode Go usage");
            CommandResponse::Failed { message: error }
        }
    }
}

#[tauri::command]
pub(crate) async fn read_deepseek_balance() -> CommandResponse<useage::deepseek::DeepSeekBalance> {
    match useage::deepseek::read().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => {
            tracing::warn!(error = %error, "failed to load DeepSeek balance");
            CommandResponse::Failed { message: error }
        }
    }
}

#[tauri::command]
pub(crate) async fn read_cherryin_balance() -> CommandResponse<useage::cherryin::CherryInBalance> {
    match useage::cherryin::read().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(error) => {
            tracing::warn!(error = %error, "failed to load CherryIN balance");
            CommandResponse::Failed { message: error }
        }
    }
}

#[tauri::command]
pub(crate) async fn read_weather() -> CommandResponse<weather::WeatherReport> {
    match weather::read().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(message) => {
            tracing::warn!(error = %message, "failed to load weather");
            CommandResponse::Failed { message }
        }
    }
}

#[tauri::command]
pub(crate) async fn read_github() -> CommandResponse<github::GithubSnapshot> {
    match github::read().await {
        Ok(data) => CommandResponse::Ready { data },
        Err(message) => {
            tracing::warn!(error = %message, "failed to load GitHub activity");
            CommandResponse::Failed { message }
        }
    }
}
