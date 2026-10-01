use diesel::prelude::*;
use std::collections::HashSet;
use std::path::Path;

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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Habit {
    id: String,
    name: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Widget {
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
    CodexClaude,
    Grok,
    Copilot,
    DeepSeek,
    CherryIn,
    TokenFlux,
    DimAgent,
    Quotation,
    Game {
        game: games::Game,
    },
    Steam,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Placement {
    id: String,
    widget: Widget,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Layout {
    widgets: Vec<Placement>,
    island_widget_id: Option<String>,
}

#[derive(Clone, Copy)]
pub enum Provider {
    Codex,
    OpenCode,
    DeepSeek,
    CherryIn,
    Github,
    Claude,
    Grok,
    Copilot,
    TokenFlux,
    DimAgent,
}

impl Layout {
    pub fn has_ugos(&self) -> bool {
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
        // Dynamic Island is still too ugly; keep islandWidgetId null in the default layout for now.
        serde_json::from_str(include_str!("default.json"))
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
                    if !service_status::is_known_service(service_id) =>
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
                Widget::CodexClaude => "codex-claude".to_owned(),
                Widget::Grok => "grok".to_owned(),
                Widget::Copilot => "copilot".to_owned(),
                Widget::DeepSeek => "deep-seek".to_owned(),
                Widget::CherryIn => "cherry-in".to_owned(),
                Widget::TokenFlux => "token-flux".to_owned(),
                Widget::DimAgent => "dim-agent".to_owned(),
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

pub fn read(path: &Path) -> Result<Layout, String> {
    let mut connection = database::open(path).map_err(|error| error.to_string())?;
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

pub fn write(path: &Path, layout: &Layout) -> Result<(), String> {
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
    let mut connection = database::open(path).map_err(|error| error.to_string())?;
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

impl Layout {
    pub fn island(&self) -> Option<&Widget> {
        self.widgets
            .iter()
            .find(|placement| Some(&placement.id) == self.island_widget_id.as_ref())
            .map(|placement| &placement.widget)
    }

    pub fn stock_symbols(&self) -> Vec<String> {
        self.widgets
            .iter()
            .filter_map(|placement| match &placement.widget {
                Widget::Stock { symbol } => Some(symbol.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn weather_locations(&self) -> Vec<String> {
        self.widgets
            .iter()
            .filter_map(|placement| match &placement.widget {
                Widget::Weather { location } => Some(location.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn service_ids(&self) -> Vec<String> {
        self.widgets
            .iter()
            .filter_map(|placement| match &placement.widget {
                Widget::ServiceStatus { service_id } => Some(service_id.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn games(&self) -> (Vec<games::Game>, bool) {
        let mut selected = Vec::new();
        let mut steam = false;
        for placement in &self.widgets {
            match &placement.widget {
                Widget::Game { game } => selected.push(*game),
                Widget::Steam => steam = true,
                _ => {}
            }
        }
        (selected, steam)
    }

    pub fn has_exchange(&self) -> bool {
        self.contains(|widget| matches!(widget, Widget::Exchange))
    }

    pub fn has_quotation(&self) -> bool {
        self.contains(|widget| matches!(widget, Widget::Quotation))
    }

    pub fn has_device_telemetry(&self) -> bool {
        self.contains(|widget| {
            matches!(
                widget,
                Widget::LocalCpu
                    | Widget::LocalMemory
                    | Widget::LocalStorage
                    | Widget::LocalNetwork
            )
        })
    }

    pub fn has_provider(&self, provider: Provider) -> bool {
        self.contains(|widget| {
            matches!(
                (provider, widget),
                (Provider::Codex, Widget::Codex)
                    | (Provider::OpenCode, Widget::OpenCode)
                    | (Provider::DeepSeek, Widget::DeepSeek)
                    | (Provider::CherryIn, Widget::CherryIn)
                    | (Provider::Github, Widget::Github)
                    | (Provider::Claude, Widget::Claude)
                    | (Provider::Codex, Widget::CodexClaude)
                    | (Provider::Claude, Widget::CodexClaude)
                    | (Provider::Grok, Widget::Grok)
                    | (Provider::Copilot, Widget::Copilot)
                    | (Provider::TokenFlux, Widget::TokenFlux)
                    | (Provider::DimAgent, Widget::DimAgent)
            )
        })
    }

    fn contains(&self, predicate: impl Fn(&Widget) -> bool) -> bool {
        self.widgets
            .iter()
            .any(|placement| predicate(&placement.widget))
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/ugos.rs"]
mod ugos_tests;
