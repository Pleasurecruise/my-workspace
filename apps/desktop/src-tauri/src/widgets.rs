use diesel::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::CommandResponse;

diesel::table! {
    dashboard_widgets (id) {
        id -> Text,
        position -> Integer,
        configuration -> Text,
    }
}
diesel::table! {
    dashboard_layout (id) {
        id -> Integer,
        island_widget_id -> Nullable<Text>,
    }
}

#[cfg(test)]
#[path = "../tests/unit/game_layout.rs"]
mod game_tests;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Habit {
    id: String,
    name: String,
}

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
    Planner {
        habits: Vec<Habit>,
    },
    Spending,
    Invalid {
        configuration: String,
        error: String,
    },
    Codex,
    OpenCode,
    Claude,
    Grok,
    Copilot,
    DeepSeek,
    CherryIn,
    Quotation,
    Game {
        game: games::Game,
    },
    Steam,
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
    island_widget_id: Option<String>,
}

#[derive(Clone, Copy)]
pub(crate) enum ProviderWidget {
    Codex,
    OpenCode,
    DeepSeek,
    CherryIn,
    Github,
    Claude,
    Grok,
    Copilot,
}

impl Layout {
    fn has_ugos(&self) -> bool {
        self.widgets.iter().any(|placement| {
            matches!(
                placement.widget,
                Widget::Cpu | Widget::Memory | Widget::Storage | Widget::Network
            )
        })
    }
}

impl Default for Layout {
    fn default() -> Self {
        serde_json::from_str(include_str!("dashboard-default.json"))
            .expect("bundled Dashboard layout must be valid")
    }
}

impl Layout {
    fn validate(&self) -> Result<(), String> {
        if self
            .island_widget_id
            .as_ref()
            .is_some_and(|id| !self.widgets.iter().any(|placement| &placement.id == id))
        {
            return Err("Dynamic Island selection must reference a saved widget".to_owned());
        }
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
                Widget::Planner { habits } => {
                    let mut names = HashSet::new();
                    let mut habit_ids = HashSet::new();
                    for habit in habits {
                        if !valid_widget_id(&habit.id)
                            || !habit_ids.insert(&habit.id)
                            || habit.name.trim() != habit.name
                            || habit.name.chars().any(char::is_control)
                            || !(1..=120).contains(&habit.name.chars().count())
                            || !names.insert(habit.name.to_lowercase())
                        {
                            return Err("Habit names and IDs must be valid and unique".into());
                        }
                    }
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
                    if !quotes::status::valid_service_id(service_id) =>
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
                Widget::Planner { .. } => "planner".to_owned(),
                Widget::Spending => "spending".to_owned(),
                Widget::Codex => "codex".to_owned(),
                Widget::OpenCode => "open-code".to_owned(),
                Widget::Claude => "claude".to_owned(),
                Widget::Grok => "grok".to_owned(),
                Widget::Copilot => "copilot".to_owned(),
                Widget::DeepSeek => "deep-seek".to_owned(),
                Widget::CherryIn => "cherry-in".to_owned(),
                Widget::Quotation => "quotation".to_owned(),
                Widget::Game { game } => format!("game-{}", game.key()),
                Widget::Steam => "steam".to_owned(),
                Widget::Invalid { .. } => format!("invalid-{}", placement.id),
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

#[cfg(test)]
fn decode(bytes: &[u8]) -> Result<Layout, String> {
    let layout: Layout = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    layout.validate()?;
    Ok(layout)
}

fn parse_widget(configuration: &str) -> Result<Widget, String> {
    let value: serde_json::Value =
        serde_json::from_str(configuration).map_err(|error| error.to_string())?;
    let widget: Widget =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    if matches!(widget, Widget::Invalid { .. }) {
        return Err("Reserved widget kind".into());
    }
    // Serde's internally tagged unit variants otherwise ignore additional fields.
    let encoded = serde_json::to_value(&widget).map_err(|error| error.to_string())?;
    if let Some(fields) = value.as_object() {
        for key in fields.keys() {
            if encoded.get(key).is_none() {
                return Err(format!("Unknown widget field: {key}"));
            }
        }
    }
    Ok(widget)
}

fn read(path: &Path) -> Result<Layout, String> {
    let mut connection = vesper_database::open(path).map_err(|error| error.to_string())?;
    connection
        .transaction::<_, diesel::result::Error, _>(|connection| {
            let selected = dashboard_layout::table
                .select(dashboard_layout::island_widget_id)
                .first::<Option<String>>(connection)
                .optional()?;
            let records = dashboard_widgets::table
                .order(dashboard_widgets::position.asc())
                .select((dashboard_widgets::id, dashboard_widgets::configuration))
                .load::<(String, String)>(connection)?;
            Ok((selected, records))
        })
        .map_err(|error| format!("Could not read Dashboard layout: {error}"))
        .and_then(|(selected, mut records)| {
            let Some(island_widget_id) = selected else {
                if !records.is_empty() {
                    return Err("Dashboard layout is missing its selection record".to_owned());
                }
                return Ok(Layout::default());
            };
            let mut layout = Layout {
                widgets: Vec::new(),
                island_widget_id: None,
            };
            let positions: std::collections::HashMap<_, _> = records
                .iter()
                .enumerate()
                .map(|(position, (id, _))| (id.clone(), position))
                .collect();
            records.sort_by(|a, b| a.0.cmp(&b.0));
            for (id, configuration) in records {
                let widget = match parse_widget(&configuration) {
                    Ok(widget) => widget,
                    Err(error) => Widget::Invalid {
                        configuration: configuration.clone(),
                        error,
                    },
                };
                let position = layout.widgets.len();
                layout.widgets.push(Placement { id, widget });
                if let Err(error) = layout.validate() {
                    layout.widgets[position].widget = Widget::Invalid {
                        configuration,
                        error,
                    };
                }
            }
            layout
                .widgets
                .sort_by_key(|placement| positions[&placement.id]);
            layout.island_widget_id = island_widget_id;
            layout.validate()?;
            Ok(layout)
        })
}

fn write(path: &Path, layout: &Layout) -> Result<(), String> {
    layout.validate()?;
    let records = layout
        .widgets
        .iter()
        .enumerate()
        .map(|(position, placement)| {
            let position =
                i32::try_from(position).map_err(|_| "Too many Dashboard widgets".to_owned())?;
            let configuration = match &placement.widget {
                Widget::Invalid { configuration, .. } => configuration.clone(),
                widget => serde_json::to_string(widget).map_err(|error| error.to_string())?,
            };
            Ok((&placement.id, position, configuration))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut connection = vesper_database::open(path).map_err(|error| error.to_string())?;
    connection
        .immediate_transaction::<_, diesel::result::Error, _>(|connection| {
            diesel::delete(dashboard_layout::table).execute(connection)?;
            diesel::delete(dashboard_widgets::table).execute(connection)?;
            for (id, position, configuration) in &records {
                diesel::insert_into(dashboard_widgets::table)
                    .values((
                        dashboard_widgets::id.eq(id),
                        dashboard_widgets::position.eq(position),
                        dashboard_widgets::configuration.eq(configuration),
                    ))
                    .execute(connection)?;
            }
            diesel::insert_into(dashboard_layout::table)
                .values((
                    dashboard_layout::id.eq(1),
                    dashboard_layout::island_widget_id.eq(&layout.island_widget_id),
                ))
                .execute(connection)?;
            Ok(())
        })
        .map_err(|error| format!("Could not save Dashboard layout: {error}"))
}

fn path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|directory| directory.join(vesper_database::FILE_NAME))
        .map_err(|error| format!("Could not resolve Dashboard database: {error}"))
}

pub(crate) fn island_widget(app: &tauri::AppHandle) -> Result<Option<Widget>, String> {
    let layout = path(app).and_then(|path| read(&path))?;
    Ok(layout
        .widgets
        .into_iter()
        .find(|placement| Some(&placement.id) == layout.island_widget_id.as_ref())
        .map(|placement| placement.widget))
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

pub(crate) fn games(app: &tauri::AppHandle) -> Result<(Vec<games::Game>, bool), String> {
    let layout = path(app).and_then(|path| read(&path))?;
    let mut selected = Vec::new();
    let mut steam = false;
    for placement in layout.widgets {
        match placement.widget {
            Widget::Game { game } => selected.push(game),
            Widget::Steam => steam = true,
            _ => {}
        }
    }
    Ok((selected, steam))
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

pub(crate) fn has_ugos(app: &tauri::AppHandle) -> Result<bool, String> {
    let layout = path(app).and_then(|path| read(&path))?;
    Ok(layout.has_ugos())
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
    Ok(layout.has_provider(provider))
}

impl Layout {
    fn has_provider(&self, provider: ProviderWidget) -> bool {
        self.widgets.iter().any(|placement| {
            matches!(
                (provider, &placement.widget),
                (ProviderWidget::Codex, Widget::Codex)
                    | (ProviderWidget::OpenCode, Widget::OpenCode)
                    | (ProviderWidget::DeepSeek, Widget::DeepSeek)
                    | (ProviderWidget::CherryIn, Widget::CherryIn)
                    | (ProviderWidget::Github, Widget::Github)
                    | (ProviderWidget::Claude, Widget::Claude)
                    | (ProviderWidget::Grok, Widget::Grok)
                    | (ProviderWidget::Copilot, Widget::Copilot)
            )
        })
    }
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
        Ok(()) => {
            crate::island::sync(&app);
            CommandResponse::Ready { data: () }
        }
        Err(message) => CommandResponse::Failed { message },
    }
}

#[tauri::command]
pub(crate) fn reset_layout(app: tauri::AppHandle) -> CommandResponse<Layout> {
    let layout = Layout::default();
    match path(&app).and_then(|path| write(&path, &layout)) {
        Ok(()) => {
            crate::island::sync(&app);
            CommandResponse::Ready { data: layout }
        }
        Err(message) => CommandResponse::Failed { message },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_habits() {
        let mut layout = Layout {
            widgets: vec![Placement {
                id: "planner".into(),
                widget: Widget::Planner {
                    habits: vec![
                        Habit {
                            id: "read".into(),
                            name: "Read".into(),
                        },
                        Habit {
                            id: "walk".into(),
                            name: "Walk".into(),
                        },
                    ],
                },
            }],
            island_widget_id: Some("planner".into()),
        };
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(vesper_database::FILE_NAME);
        write(&path, &layout).unwrap();
        assert_eq!(
            serde_json::to_value(read(&path).unwrap()).unwrap(),
            serde_json::to_value(&layout).unwrap()
        );
        let Widget::Planner { habits } = &mut layout.widgets[0].widget else {
            panic!("Expected Planner")
        };
        habits[1].name = "READ".into();
        assert!(layout.validate().is_err());
        for name in ["", "  Read", "Read\nmore"] {
            let Widget::Planner { habits } = &mut layout.widgets[0].widget else {
                panic!("Expected Planner")
            };
            habits[1].name = name.into();
            assert!(layout.validate().is_err());
        }
    }

    #[test]
    fn spending_is_a_persisted_singleton_that_can_be_pinned() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(vesper_database::FILE_NAME);
        let mut layout = Layout {
            widgets: vec![Placement {
                id: "spending".into(),
                widget: Widget::Spending,
            }],
            island_widget_id: Some("spending".into()),
        };
        write(&path, &layout).unwrap();
        let restored = read(&path).unwrap();
        assert!(matches!(restored.widgets[0].widget, Widget::Spending));
        assert_eq!(restored.island_widget_id.as_deref(), Some("spending"));
        layout.widgets.push(Placement {
            id: "spending-other".into(),
            widget: Widget::Spending,
        });
        assert!(layout.validate().is_err());
        assert!(matches!(
            read(&path).unwrap().widgets[0].widget,
            Widget::Spending
        ));
    }

    #[test]
    fn every_provider_follows_saved_widget_presence() {
        for (provider, widget) in [
            (ProviderWidget::Codex, Widget::Codex),
            (ProviderWidget::OpenCode, Widget::OpenCode),
            (ProviderWidget::Claude, Widget::Claude),
            (ProviderWidget::Grok, Widget::Grok),
            (ProviderWidget::Copilot, Widget::Copilot),
            (ProviderWidget::DeepSeek, Widget::DeepSeek),
            (ProviderWidget::CherryIn, Widget::CherryIn),
            (ProviderWidget::Github, Widget::Github),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join(vesper_database::FILE_NAME);
            let mut layout = Layout {
                widgets: vec![Placement {
                    id: "provider".to_owned(),
                    widget,
                }],
                island_widget_id: None,
            };
            write(&path, &layout).unwrap();
            assert!(read(&path).unwrap().has_provider(provider));
            layout.widgets.clear();
            write(&path, &layout).unwrap();
            assert!(!read(&path).unwrap().has_provider(provider));
        }
    }

    #[test]
    fn replaces_layout_and_preserves_it_when_validation_fails() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(vesper_database::FILE_NAME);
        let mut layout = Layout::default();
        layout.widgets.reverse();
        write(&path, &layout).unwrap();
        let saved = serde_json::to_value(read(&path).unwrap()).unwrap();
        assert_eq!(saved, serde_json::to_value(&layout).unwrap());
        layout.island_widget_id = Some("absent".to_owned());
        assert!(write(&path, &layout).is_err());
        assert_eq!(serde_json::to_value(read(&path).unwrap()).unwrap(), saved);
        write(
            &path,
            &Layout {
                widgets: vec![],
                island_widget_id: None,
            },
        )
        .unwrap();
        assert!(read(&path).unwrap().widgets.is_empty());
    }

    #[test]
    fn database_failure_rolls_back_the_complete_layout() {
        use diesel::connection::SimpleConnection;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(vesper_database::FILE_NAME);
        write(&path, &Layout::default()).unwrap();
        let saved = serde_json::to_value(read(&path).unwrap()).unwrap();
        let mut connection = vesper_database::open(&path).unwrap();
        connection.batch_execute("CREATE TRIGGER reject_layout BEFORE INSERT ON dashboard_layout BEGIN SELECT RAISE(ABORT, 'test write failure'); END;").unwrap();
        assert!(
            write(
                &path,
                &Layout {
                    widgets: vec![],
                    island_widget_id: None
                }
            )
            .is_err()
        );
        assert_eq!(serde_json::to_value(read(&path).unwrap()).unwrap(), saved);
    }

    #[test]
    fn duplicate_order() {
        use diesel::connection::SimpleConnection;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(vesper_database::FILE_NAME);
        let mut connection = vesper_database::open(&path).unwrap();
        connection
            .batch_execute(
                r#"
            INSERT INTO dashboard_widgets VALUES
              ('planner-a', 0, '{"kind":"planner","habits":[{"id":"read","name":"Read"}]}'),
              ('planner-b', 1, '{"kind":"planner","habits":[{"id":"walk","name":"Walk"}]}');
            INSERT INTO dashboard_layout VALUES (1, 'planner-a');
        "#,
            )
            .unwrap();
        let mut layout = read(&path).unwrap();
        assert!(matches!(layout.widgets[1].widget, Widget::Invalid { .. }));
        layout.widgets.reverse();
        write(&path, &layout).unwrap();
        assert_eq!(
            serde_json::to_value(read(&path).unwrap()).unwrap(),
            serde_json::to_value(&layout).unwrap(),
        );
    }

    #[test]
    fn isolates_invalid_widgets() {
        for configuration in [
            "{",
            r#"{"kind":"calendar"}"#,
            r#"{"kind":"spending","extra":true}"#,
            r#"{"kind":"stock","symbol":""}"#,
        ] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join(vesper_database::FILE_NAME);
            let initial = Layout {
                widgets: vec![
                    Placement {
                        id: "planner".into(),
                        widget: Widget::Planner { habits: vec![] },
                    },
                    Placement {
                        id: "spending".into(),
                        widget: Widget::Spending,
                    },
                ],
                island_widget_id: Some("planner".into()),
            };
            write(&path, &initial).unwrap();
            let mut connection = vesper_database::open(&path).unwrap();
            diesel::update(dashboard_widgets::table.find("planner"))
                .set(dashboard_widgets::configuration.eq(configuration))
                .execute(&mut connection)
                .unwrap();
            let mut restored = read(&path).unwrap();
            assert_eq!(restored.widgets.len(), initial.widgets.len());
            assert_eq!(restored.island_widget_id.as_deref(), Some("planner"));
            let broken = restored.widgets.iter().find(|p| p.id == "planner").unwrap();
            assert!(
                matches!(&broken.widget, Widget::Invalid { configuration: raw, error } if raw == configuration && !error.is_empty()),
                "{configuration}: {:?}",
                broken.widget
            );
            assert!(
                restored
                    .widgets
                    .iter()
                    .any(|p| matches!(p.widget, Widget::Spending))
            );
            restored.widgets.reverse();
            write(&path, &restored).unwrap();
            let value: String = dashboard_widgets::table
                .find("planner")
                .select(dashboard_widgets::configuration)
                .first(&mut connection)
                .unwrap();
            assert_eq!(value, configuration);
            assert_eq!(
                serde_json::to_value(read(&path).unwrap()).unwrap(),
                serde_json::to_value(&restored).unwrap()
            );
            restored.widgets.retain(|p| p.id != "planner");
            restored.island_widget_id = None;
            write(&path, &restored).unwrap();
            assert!(
                read(&path)
                    .unwrap()
                    .widgets
                    .iter()
                    .all(|p| !matches!(p.widget, Widget::Invalid { .. }))
            );
        }
    }

    #[test]
    fn island_selection_must_reference_a_saved_widget() {
        let mut layout = Layout::default();
        assert!(layout.widgets.iter().any(|placement| Some(&placement.id)
            == layout.island_widget_id.as_ref()
            && matches!(placement.widget, Widget::Planner { .. })));
        layout.island_widget_id = Some("absent".to_owned());
        assert!(layout.validate().is_err());
        layout.island_widget_id = None;
        layout.validate().unwrap();
    }

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
    fn default_layout_is_valid() {
        Layout::default().validate().unwrap();
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
    fn rejects_old_widgets() {
        for kind in [
            "usage", "quota", "balance", "todo", "calendar", "todoList", "checkIn",
        ] {
            let bytes = serde_json::to_vec(&serde_json::json!({
                "widgets": [{ "id": "old", "widget": { "kind": kind } }]
            }))
            .unwrap();
            assert!(decode(&bytes).is_err(), "unsupported widget {kind}");
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/ugos_layout.rs"]
mod ugos_tests;
