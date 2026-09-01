use std::time::Duration;

use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};

const CONCURRENCY: usize = 3;

#[derive(Clone, Copy)]
struct Service {
    id: &'static str,
    name: &'static str,
    keywords: &'static str,
    endpoint: &'static str,
    component_filter: Option<&'static str>,
}

const SERVICES: [Service; 3] = [
    Service {
        id: "github",
        name: "GitHub",
        keywords: "github status code hosting 代码托管",
        endpoint: "https://www.githubstatus.com/api/v2/summary.json",
        component_filter: None,
    },
    Service {
        id: "codex",
        name: "Codex",
        keywords: "openai codex status",
        endpoint: "https://status.openai.com/api/v2/summary.json",
        component_filter: Some("codex"),
    },
    Service {
        id: "deepseek",
        name: "DeepSeek",
        keywords: "deepseek status 深度求索",
        endpoint: "https://status.deepseek.com/api/v2/summary.json",
        component_filter: None,
    },
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceCatalogEntry {
    id: &'static str,
    name: &'static str,
    keywords: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatusReport {
    pub services: Vec<ServiceStatus>,
    pub failures: Vec<ServiceStatusFailure>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatusFailure {
    pub service_id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub service_id: String,
    pub name: String,
    pub status: Status,
    pub operational_percent: f64,
    pub operational_components: usize,
    pub total_components: usize,
    pub active_incidents: usize,
    pub updated_at: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Operational,
    UnderMaintenance,
    DegradedPerformance,
    PartialOutage,
    MajorOutage,
    Unknown,
}

#[derive(Deserialize)]
struct Summary {
    page: Page,
    #[serde(default)]
    components: Vec<Component>,
    #[serde(default)]
    incidents: Vec<Incident>,
}

#[derive(Deserialize)]
struct Page {
    updated_at: String,
}

#[derive(Deserialize)]
struct Component {
    name: String,
    status: String,
    #[serde(default)]
    group: bool,
}

#[derive(Deserialize)]
struct Incident {
    status: String,
}

pub fn valid_service_id(id: &str) -> bool {
    SERVICES.iter().any(|service| service.id == id)
}

pub fn catalog() -> Vec<ServiceCatalogEntry> {
    SERVICES
        .iter()
        .map(|service| ServiceCatalogEntry {
            id: service.id,
            name: service.name,
            keywords: service.keywords,
        })
        .collect()
}

fn service(id: &str) -> Option<Service> {
    SERVICES.iter().copied().find(|service| service.id == id)
}

fn status(value: &str) -> Status {
    match value {
        "operational" => Status::Operational,
        "under_maintenance" => Status::UnderMaintenance,
        "degraded_performance" => Status::DegradedPerformance,
        "partial_outage" => Status::PartialOutage,
        "major_outage" => Status::MajorOutage,
        _ => Status::Unknown,
    }
}

fn severity(value: Status) -> u8 {
    match value {
        Status::Operational => 0,
        Status::UnderMaintenance => 1,
        Status::DegradedPerformance => 2,
        Status::PartialOutage => 3,
        Status::MajorOutage => 4,
        Status::Unknown => 5,
    }
}

fn project(service: Service, summary: Summary) -> Result<ServiceStatus, String> {
    let components: Vec<_> = summary
        .components
        .into_iter()
        .filter(|component| !component.group)
        .filter(|component| {
            service
                .component_filter
                .is_none_or(|filter| component.name.to_ascii_lowercase().contains(filter))
        })
        .collect();
    if components.is_empty() {
        return Err(format!(
            "{} status returned no matching components",
            service.name
        ));
    }
    let total_components = components.len();
    let operational_components = components
        .iter()
        .filter(|component| status(&component.status) == Status::Operational)
        .count();
    let current_status = components
        .iter()
        .map(|component| status(&component.status))
        .max_by_key(|value| severity(*value))
        .unwrap_or(Status::Unknown);
    let unresolved_incidents = summary
        .incidents
        .iter()
        .filter(|incident| incident.status != "resolved" && incident.status != "completed")
        .count();
    let active_incidents =
        if service.component_filter.is_some() && current_status == Status::Operational {
            0
        } else {
            unresolved_incidents
        };

    Ok(ServiceStatus {
        service_id: service.id.to_owned(),
        name: service.name.to_owned(),
        status: current_status,
        operational_percent: operational_components as f64 / total_components as f64 * 100.0,
        operational_components,
        total_components,
        active_incidents,
        updated_at: summary.page.updated_at,
    })
}

async fn request(client: &reqwest::Client, service: Service) -> Result<ServiceStatus, String> {
    let response = client
        .get(service.endpoint)
        .send()
        .await
        .map_err(|error| format!("Could not query {} status: {error}", service.name))?;
    if !response.status().is_success() {
        return Err(format!(
            "{} status request failed: HTTP {}",
            service.name,
            response.status()
        ));
    }
    let summary = response.json().await.map_err(|error| {
        format!(
            "{} status returned an unsupported payload: {error}",
            service.name
        )
    })?;
    project(service, summary)
}

pub async fn read(service_ids: Vec<String>) -> Result<ServiceStatusReport, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Vesper/0.1 service status dashboard")
        .build()
        .map_err(|error| format!("Could not create service status client: {error}"))?;
    let results = stream::iter(service_ids.into_iter().map(|service_id| {
        let selected = service(&service_id);
        let client = &client;
        async move {
            let service = selected.ok_or_else(|| ServiceStatusFailure {
                service_id: service_id.clone(),
                message: "Dashboard service status selection is invalid".to_owned(),
            })?;
            request(client, service)
                .await
                .map_err(|message| ServiceStatusFailure {
                    service_id,
                    message,
                })
        }
    }))
    .buffered(CONCURRENCY)
    .collect::<Vec<_>>()
    .await;
    let mut services = Vec::with_capacity(results.len());
    let mut failures = Vec::new();
    for result in results {
        match result {
            Ok(service) => services.push(service),
            Err(failure) => failures.push(failure),
        }
    }
    Ok(ServiceStatusReport { services, failures })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(components: serde_json::Value) -> Summary {
        serde_json::from_value(serde_json::json!({
            "page": { "updated_at": "2026-09-01T08:00:00Z" },
            "components": components,
            "incidents": [
                { "status": "resolved" },
                { "status": "monitoring" }
            ]
        }))
        .expect("valid status summary")
    }

    #[test]
    fn projects_component_health() {
        let projected = project(
            service("github").expect("known service"),
            summary(serde_json::json!([
                { "name": "API", "status": "operational" },
                { "name": "Actions", "status": "partial_outage" },
                { "name": "Products", "status": "operational", "group": true }
            ])),
        )
        .expect("valid status projection");

        assert_eq!(projected.status, Status::PartialOutage);
        assert_eq!(projected.operational_components, 1);
        assert_eq!(projected.total_components, 2);
        assert_eq!(projected.operational_percent, 50.0);
        assert_eq!(projected.active_incidents, 1);
    }

    #[test]
    fn selects_only_codex_components() {
        let projected = project(
            service("codex").expect("known service"),
            summary(serde_json::json!([
                { "name": "ChatGPT", "status": "major_outage" },
                { "name": "Codex Web", "status": "operational" },
                { "name": "Codex API", "status": "degraded_performance" }
            ])),
        )
        .expect("valid Codex projection");

        assert_eq!(projected.status, Status::DegradedPerformance);
        assert_eq!(projected.total_components, 2);
    }

    #[test]
    fn validates_catalog_ids() {
        assert!(valid_service_id("deepseek"));
        assert!(!valid_service_id("https://example.com/status"));
    }
}
