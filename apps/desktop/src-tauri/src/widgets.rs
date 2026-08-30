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
    Weather { location: String },
    Stock { symbol: String },
    Github,
    Todo,
    Usage,
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
            ("todo", Widget::Todo),
            ("usage", Widget::Usage),
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
                    if trimmed != location
                        || !(2..=120).contains(&trimmed.chars().count())
                        || location.chars().any(char::is_control)
                    {
                        return Err("Dashboard weather location is invalid".to_owned());
                    }
                }
                _ => {}
            }
            let key = match &placement.widget {
                Widget::Cpu => "cpu".to_owned(),
                Widget::Memory => "memory".to_owned(),
                Widget::Storage => "storage".to_owned(),
                Widget::Network => "network".to_owned(),
                Widget::Weather { location } => {
                    format!("weather-{}", location.to_lowercase())
                }
                Widget::Stock { symbol } => format!("stock-{symbol}"),
                Widget::Github => "github".to_owned(),
                Widget::Todo => "todo".to_owned(),
                Widget::Usage => "usage".to_owned(),
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
    let layout: Layout = serde_json::from_slice(bytes)
        .map_err(|error| format!("Dashboard layout is invalid: {error}"))?;
    layout.validate()?;
    Ok(layout)
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
    fn rejects_extra_field() {
        let json = br#"{"revision":1,"widgets":[]}"#;

        assert!(decode(json).is_err());
    }
}
