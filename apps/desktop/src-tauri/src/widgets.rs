use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use tauri::Manager;

use crate::CommandResponse;

const FILE_NAME: &str = "dashboard-layout.json";

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Widget {
    Cpu,
    Memory,
    Storage,
    Network,
    LocalCpu,
    LocalMemory,
    LocalStorage,
    LocalNetwork,
    Weather {
        location: String,
    },
    Stock {
        symbol: String,
    },
    Exchange,
    ServiceStatus {
        #[serde(rename = "serviceId")]
        service_id: String,
    },
    Github,
    Calendar,
    TodoList,
    Codex,
    OpenCode,
    Claude,
    Grok,
    Copilot,
    DeepSeek,
    CherryIn,
    Quotation,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Placement {
    id: String,
    widget: Widget,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Layout {
    widgets: Vec<Placement>,
}

#[derive(Clone, Copy)]
pub(crate) enum ProviderWidget {
    Claude,
    Grok,
    Copilot,
}

impl Default for Layout {
    fn default() -> Self {
        let widgets = [
            ("cpu", Widget::Cpu),
            ("memory", Widget::Memory),
            ("storage", Widget::Storage),
            ("network", Widget::Network),
            (
                "weather-shanghai",
                Widget::Weather {
                    location: "shanghai".to_owned(),
                },
            ),
            (
                "weather-ningbo",
                Widget::Weather {
                    location: "ningbo".to_owned(),
                },
            ),
            (
                "weather-nottingham",
                Widget::Weather {
                    location: "nottingham".to_owned(),
                },
            ),
            ("github", Widget::Github),
            ("calendar", Widget::Calendar),
            ("todo-list", Widget::TodoList),
            ("codex", Widget::Codex),
            ("open-code", Widget::OpenCode),
            ("deep-seek", Widget::DeepSeek),
            ("cherry-in", Widget::CherryIn),
        ]
        .into_iter()
        .map(|(id, widget)| Placement {
            id: id.to_owned(),
            widget,
        })
        .collect();
        Self { widgets }
    }
}

impl Layout {
    fn validate(&self) -> Result<(), String> {
        let mut ids = HashSet::new();
        let mut singletons = HashSet::new();
        for placement in &self.widgets {
            if !valid_widget_id(&placement.id) {
                return Err("Dashboard widget ID contains unsupported characters".to_owned());
            }
            if !ids.insert(&placement.id) {
                return Err("Dashboard layout contains a duplicate widget ID".to_owned());
            }
            match &placement.widget {
                Widget::Stock { symbol } if !valid_stock_symbol(symbol) => {
                    return Err("Dashboard stock symbol is invalid".to_owned());
                }
                Widget::Weather { location } => {
                    let trimmed = location.trim();
                    if trimmed != location {
                        return Err("Dashboard weather location is invalid".to_owned());
                    }
                    if !(2..=120).contains(&trimmed.chars().count()) {
                        return Err("Dashboard weather location is invalid".to_owned());
                    }
                    if location.chars().any(char::is_control) {
                        return Err("Dashboard weather location is invalid".to_owned());
                    }
                }
                Widget::ServiceStatus { service_id }
                    if !crate::status::valid_service_id(service_id) =>
                {
                    return Err("Dashboard service status selection is invalid".to_owned());
                }
                _ => {}
            }
            let key = match &placement.widget {
                Widget::Cpu => "cpu".to_owned(),
                Widget::Memory => "memory".to_owned(),
                Widget::Storage => "storage".to_owned(),
                Widget::Network => "network".to_owned(),
                Widget::LocalCpu => "local-cpu".to_owned(),
                Widget::LocalMemory => "local-memory".to_owned(),
                Widget::LocalStorage => "local-storage".to_owned(),
                Widget::LocalNetwork => "local-network".to_owned(),
                Widget::Weather { location } => {
                    format!("weather-{}", location.to_lowercase())
                }
                Widget::Stock { symbol } => format!("stock-{symbol}"),
                Widget::Exchange => "exchange".to_owned(),
                Widget::ServiceStatus { service_id } => format!("service-status-{service_id}"),
                Widget::Github => "github".to_owned(),
                Widget::Calendar => "calendar".to_owned(),
                Widget::TodoList => "todo-list".to_owned(),
                Widget::Codex => "codex".to_owned(),
                Widget::OpenCode => "open-code".to_owned(),
                Widget::Claude => "claude".to_owned(),
                Widget::Grok => "grok".to_owned(),
                Widget::Copilot => "copilot".to_owned(),
                Widget::DeepSeek => "deep-seek".to_owned(),
                Widget::CherryIn => "cherry-in".to_owned(),
                Widget::Quotation => "quotation".to_owned(),
            };
            if !singletons.insert(key.clone()) {
                return Err(format!("Dashboard layout contains duplicate {key} widgets"));
            }
        }
        Ok(())
    }
}

fn valid_widget_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 80 {
        return false;
    }
    for character in id.chars() {
        match character {
            '-' | '_' => continue,
            value if value.is_ascii_alphanumeric() => continue,
            _ => return false,
        }
    }
    true
}

fn valid_stock_symbol(symbol: &str) -> bool {
    if symbol.is_empty() || symbol.len() > 12 {
        return false;
    }
    for character in symbol.chars() {
        match character {
            '.' | '-' => continue,
            value if value.is_ascii_uppercase() || value.is_ascii_digit() => continue,
            _ => return false,
        }
    }
    true
}

fn decode(bytes: &[u8]) -> Result<Layout, String> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("Dashboard layout is invalid: {error}"))?;
    migrate_provider_widgets(&mut value);
    migrate_todo_widget(&mut value);
    let layout: Layout = serde_json::from_value(value)
        .map_err(|error| format!("Dashboard layout is invalid: {error}"))?;
    layout.validate()?;
    Ok(layout)
}

fn migrate_todo_widget(value: &mut serde_json::Value) {
    let Some(widgets) = value
        .get_mut("widgets")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    let mut migrated = Vec::with_capacity(widgets.len() + 1);
    for mut placement in widgets.drain(..) {
        let is_legacy_todo = placement
            .get("widget")
            .and_then(|widget| widget.get("kind"))
            .and_then(serde_json::Value::as_str)
            == Some("todo");
        if !is_legacy_todo {
            migrated.push(placement);
            continue;
        }
        let id = placement
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("todo")
            .to_owned();
        if let Some(widget) = placement
            .get_mut("widget")
            .and_then(serde_json::Value::as_object_mut)
        {
            widget.insert("kind".to_owned(), serde_json::Value::from("calendar"));
        }
        migrated.push(placement);
        migrated.push(serde_json::json!({
            "id": format!("{id}-list"),
            "widget": { "kind": "todoList" }
        }));
    }
    *widgets = migrated;
}

fn migrate_provider_widgets(value: &mut serde_json::Value) {
    let Some(widgets) = value
        .get_mut("widgets")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    let mut migrated = Vec::with_capacity(widgets.len() + 3);
    for mut placement in widgets.drain(..) {
        let legacy_kind = placement
            .get("widget")
            .and_then(|widget| widget.get("kind"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        if !matches!(legacy_kind.as_deref(), Some("usage" | "quota" | "balance")) {
            migrated.push(placement);
            continue;
        }
        let id = placement
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("provider")
            .to_owned();
        let primary_kind = if legacy_kind.as_deref() == Some("balance") {
            "deepSeek"
        } else {
            "codex"
        };
        if let Some(kind) = placement
            .get_mut("widget")
            .and_then(serde_json::Value::as_object_mut)
        {
            kind.insert("kind".to_owned(), serde_json::Value::from(primary_kind));
        }
        migrated.push(placement);
        if legacy_kind.as_deref() != Some("balance") {
            migrated.push(serde_json::json!({
                "id": format!("{id}-open-code"),
                "widget": { "kind": "openCode" }
            }));
        }
        if legacy_kind.as_deref() == Some("usage") {
            migrated.push(serde_json::json!({
                "id": format!("{id}-deep-seek"),
                "widget": { "kind": "deepSeek" }
            }));
        }
        if legacy_kind.as_deref() != Some("quota") {
            migrated.push(serde_json::json!({
                "id": format!("{id}-cherry-in"),
                "widget": { "kind": "cherryIn" }
            }));
        }
    }
    *widgets = migrated;
}

fn read(path: &Path) -> Result<Layout, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Layout::default()),
        Err(error) => return Err(format!("Could not read Dashboard layout: {error}")),
    };
    decode(&bytes)
}

fn write(path: &Path, layout: &Layout) -> Result<(), String> {
    layout.validate()?;
    let parent = path
        .parent()
        .ok_or_else(|| "Dashboard layout path has no parent directory".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create Dashboard layout directory: {error}"))?;
    let bytes = serde_json::to_vec_pretty(layout)
        .map_err(|error| format!("Could not encode Dashboard layout: {error}"))?;
    fs::write(path, bytes).map_err(|error| format!("Could not save Dashboard layout: {error}"))
}

fn path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join(FILE_NAME))
        .map_err(|error| format!("Could not resolve Dashboard layout directory: {error}"))
}

pub(crate) fn stock_symbols(app: &tauri::AppHandle) -> Result<Vec<String>, String> {
    let layout = path(app).and_then(|path| read(&path))?;
    let mut symbols = Vec::new();
    for placement in layout.widgets {
        if let Widget::Stock { symbol } = placement.widget {
            symbols.push(symbol);
        }
    }
    Ok(symbols)
}

pub(crate) fn weather_locations(app: &tauri::AppHandle) -> Result<Vec<String>, String> {
    let layout = path(app).and_then(|path| read(&path))?;
    let mut locations = Vec::new();
    for placement in layout.widgets {
        if let Widget::Weather { location } = placement.widget {
            locations.push(location);
        }
    }
    Ok(locations)
}

pub(crate) fn service_status_ids(app: &tauri::AppHandle) -> Result<Vec<String>, String> {
    let layout = path(app).and_then(|path| read(&path))?;
    let mut service_ids = Vec::new();
    for placement in layout.widgets {
        if let Widget::ServiceStatus { service_id } = placement.widget {
            service_ids.push(service_id);
        }
    }
    Ok(service_ids)
}

pub(crate) fn has_exchange(app: &tauri::AppHandle) -> Result<bool, String> {
    let layout = path(app).and_then(|path| read(&path))?;
    Ok(layout
        .widgets
        .iter()
        .any(|placement| matches!(placement.widget, Widget::Exchange)))
}

pub(crate) fn has_quotation(app: &tauri::AppHandle) -> Result<bool, String> {
    let layout = path(app).and_then(|path| read(&path))?;
    Ok(layout
        .widgets
        .iter()
        .any(|placement| matches!(placement.widget, Widget::Quotation)))
}

pub(crate) fn has_device_telemetry(app: &tauri::AppHandle) -> Result<bool, String> {
    let layout = path(app).and_then(|path| read(&path))?;
    Ok(layout.widgets.iter().any(|placement| {
        matches!(
            placement.widget,
            Widget::LocalCpu | Widget::LocalMemory | Widget::LocalStorage | Widget::LocalNetwork
        )
    }))
}

pub(crate) fn has_provider(
    app: &tauri::AppHandle,
    provider: ProviderWidget,
) -> Result<bool, String> {
    let layout = path(app).and_then(|path| read(&path))?;
    Ok(layout.widgets.iter().any(|placement| {
        matches!(
            (provider, &placement.widget),
            (ProviderWidget::Claude, Widget::Claude)
                | (ProviderWidget::Grok, Widget::Grok)
                | (ProviderWidget::Copilot, Widget::Copilot)
        )
    }))
}

#[tauri::command]
pub(crate) fn read_layout(app: tauri::AppHandle) -> CommandResponse<Layout> {
    match path(&app).and_then(|path| read(&path)) {
        Ok(data) => CommandResponse::Ready { data },
        Err(message) => CommandResponse::Failed { message },
    }
}

#[tauri::command]
pub(crate) fn save_layout(layout: Layout, app: tauri::AppHandle) -> CommandResponse<()> {
    match path(&app).and_then(|path| write(&path, &layout)) {
        Ok(()) => CommandResponse::Ready { data: () },
        Err(message) => CommandResponse::Failed { message },
    }
}

#[tauri::command]
pub(crate) fn reset_layout(app: tauri::AppHandle) -> CommandResponse<Layout> {
    let layout = Layout::default();
    match path(&app).and_then(|path| write(&path, &layout)) {
        Ok(()) => CommandResponse::Ready { data: layout },
        Err(message) => CommandResponse::Failed { message },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_kind() {
        let json = br#"{"widgets":[{"id":"bad","widget":{"kind":"unsupported"}}]}"#;
        assert!(decode(json).is_err());
    }

    #[test]
    fn rejects_duplicates() {
        let json = br#"{"widgets":[{"id":"cpu-1","widget":{"kind":"cpu"}},{"id":"cpu-2","widget":{"kind":"cpu"}}]}"#;
        assert!(decode(json).is_err());
    }

    #[test]
    fn supports_weather() {
        let json = br#"{"widgets":[{"id":"weather-shanghai","widget":{"kind":"weather","location":"shanghai"}},{"id":"weather-ningbo","widget":{"kind":"weather","location":"ningbo"}}]}"#;

        assert!(decode(json).is_ok());
    }

    #[test]
    fn supports_custom_location() {
        let json = br#"{"widgets":[{"id":"weather-custom","widget":{"kind":"weather","location":"Hangzhou, China"}}]}"#;

        assert!(decode(json).is_ok());
    }

    #[test]
    fn rejects_noncanonical_location() {
        let json = br#"{"widgets":[{"id":"weather-custom","widget":{"kind":"weather","location":" Hangzhou "}}]}"#;

        assert!(decode(json).is_err());
    }

    #[test]
    fn supports_known_service_status() {
        let json = br#"{"widgets":[{"id":"service-status-codex","widget":{"kind":"serviceStatus","serviceId":"codex"}}]}"#;

        assert!(decode(json).is_ok());
    }

    #[test]
    fn supports_one_exchange_widget() {
        let json = br#"{"widgets":[{"id":"exchange","widget":{"kind":"exchange"}}]}"#;

        assert!(decode(json).is_ok());

        let duplicates = br#"{"widgets":[{"id":"exchange-1","widget":{"kind":"exchange"}},{"id":"exchange-2","widget":{"kind":"exchange"}}]}"#;
        assert!(decode(duplicates).is_err());
    }

    #[test]
    fn supports_current_device_widgets_as_singletons() {
        let json = br#"{"widgets":[{"id":"local-cpu","widget":{"kind":"localCpu"}},{"id":"local-memory","widget":{"kind":"localMemory"}},{"id":"local-storage","widget":{"kind":"localStorage"}},{"id":"local-network","widget":{"kind":"localNetwork"}}]}"#;
        assert!(decode(json).is_ok());

        let duplicate = br#"{"widgets":[{"id":"local-cpu-1","widget":{"kind":"localCpu"}},{"id":"local-cpu-2","widget":{"kind":"localCpu"}}]}"#;
        assert!(decode(duplicate).is_err());
    }

    #[test]
    fn default_layout_omits_exchange_widget() {
        assert!(
            !Layout::default()
                .widgets
                .iter()
                .any(|placement| matches!(placement.widget, Widget::Exchange))
        );
    }

    #[test]
    fn rejects_unknown_service_status() {
        let json = br#"{"widgets":[{"id":"service-status-other","widget":{"kind":"serviceStatus","serviceId":"other"}}]}"#;

        assert!(decode(json).is_err());
    }

    #[test]
    fn rejects_extra_field() {
        let json = br#"{"revision":1,"widgets":[]}"#;

        assert!(decode(json).is_err());
    }

    #[test]
    fn migrates_combined_usage_widget_to_provider_widgets() {
        let json = br#"{"widgets":[{"id":"usage","widget":{"kind":"usage"}}]}"#;
        let layout = decode(json).expect("legacy usage layout");

        assert_eq!(layout.widgets.len(), 4);
        assert!(matches!(layout.widgets[0].widget, Widget::Codex));
        assert!(matches!(layout.widgets[1].widget, Widget::OpenCode));
        assert!(matches!(layout.widgets[2].widget, Widget::DeepSeek));
        assert!(matches!(layout.widgets[3].widget, Widget::CherryIn));
    }

    #[test]
    fn migrates_intermediate_quota_and_balance_widgets() {
        let json = br#"{"widgets":[{"id":"quota","widget":{"kind":"quota"}},{"id":"balance","widget":{"kind":"balance"}}]}"#;
        let layout = decode(json).expect("intermediate provider layout");

        assert_eq!(layout.widgets.len(), 4);
        assert!(matches!(layout.widgets[0].widget, Widget::Codex));
        assert!(matches!(layout.widgets[1].widget, Widget::OpenCode));
        assert!(matches!(layout.widgets[2].widget, Widget::DeepSeek));
        assert!(matches!(layout.widgets[3].widget, Widget::CherryIn));
    }

    #[test]
    fn migrates_combined_todo_widget_to_calendar_and_list() {
        let json = br#"{"widgets":[{"id":"todo","widget":{"kind":"todo"}}]}"#;
        let layout = decode(json).expect("legacy Todo layout");

        assert_eq!(layout.widgets.len(), 2);
        assert!(matches!(layout.widgets[0].widget, Widget::Calendar));
        assert!(matches!(layout.widgets[1].widget, Widget::TodoList));
    }
}
