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
    pub affected_components: Vec<AffectedComponent>,
    pub active_incidents: usize,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedComponent {
    pub name: String,
    pub status: Status,
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
    #[serde(default)]
    id: String,
    name: String,
    status: String,
    #[serde(default)]
    group: bool,
}

#[derive(Deserialize)]
struct Incident {
    status: String,
    #[serde(default)]
    components: Vec<IncidentComponent>,
}

#[derive(Deserialize)]
struct IncidentComponent {
    id: String,
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
    let active_incidents = summary
        .incidents
        .iter()
        .filter(|incident| incident.status != "resolved" && incident.status != "completed")
        .filter(|incident| {
            service.component_filter.is_none()
                || incident.components.iter().any(|affected| {
                    components
                        .iter()
                        .any(|component| !component.id.is_empty() && component.id == affected.id)
                })
        })
        .count();
    let affected_components = components
        .into_iter()
        .filter_map(|component| {
            let status = status(&component.status);
            (status != Status::Operational).then_some(AffectedComponent {
                name: component.name,
                status,
            })
        })
        .collect();

    Ok(ServiceStatus {
        service_id: service.id.to_owned(),
        name: service.name.to_owned(),
        status: current_status,
        operational_percent: operational_components as f64 / total_components as f64 * 100.0,
        operational_components,
        total_components,
        affected_components,
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
        assert_eq!(projected.affected_components.len(), 1);
        assert_eq!(projected.affected_components[0].name, "Actions");
        assert_eq!(
            projected.affected_components[0].status,
            Status::PartialOutage
        );
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
        assert_eq!(projected.affected_components.len(), 1);
        assert_eq!(projected.affected_components[0].name, "Codex API");
        assert_eq!(
            projected.affected_components[0].status,
            Status::DegradedPerformance
        );
    }

    #[test]
    fn includes_maintenance_and_unknown_services() {
        let projected = project(
            service("deepseek").expect("known service"),
            summary(serde_json::json!([
                { "name": "API", "status": "operational" },
                { "name": "Chat", "status": "under_maintenance" },
                { "name": "Login", "status": "unexpected_status" },
                { "name": "Products", "status": "major_outage", "group": true }
            ])),
        )
        .expect("valid status projection");

        let serialized = serde_json::to_value(projected).expect("serializable status");
        assert_eq!(
            serialized["affectedComponents"],
            serde_json::json!([
                { "name": "Chat", "status": "underMaintenance" },
                { "name": "Login", "status": "unknown" }
            ])
        );
    }

    #[test]
    fn healthy_services_have_no_affected_components() {
        let projected = project(
            service("github").expect("known service"),
            summary(serde_json::json!([{ "name": "API", "status": "operational" }])),
        )
        .expect("valid status projection");

        assert!(projected.affected_components.is_empty());
        assert_eq!(projected.status, Status::Operational);
    }

    #[test]
    fn rejects_missing_service_components() {
        for components in [
            serde_json::json!([]),
            serde_json::json!([{ "name": "Codex", "status": "operational", "group": true }]),
            serde_json::json!([{ "name": "ChatGPT", "status": "operational" }]),
        ] {
            let result = project(
                service("codex").expect("known service"),
                summary(components),
            );
            assert_eq!(
                result.unwrap_err(),
                "Codex status returned no matching components"
            );
        }
    }

    #[test]
    fn unrelated_outages_do_not_degrade_codex() {
        let projected = project(
            service("codex").expect("known service"),
            summary(serde_json::json!([
                { "name": "ChatGPT", "status": "major_outage" },
                { "name": "CODEX API", "status": "operational" }
            ])),
        )
        .expect("valid status projection");

        assert_eq!(projected.status, Status::Operational);
        assert_eq!(projected.operational_percent, 100.0);
        assert!(projected.affected_components.is_empty());
        assert_eq!(projected.active_incidents, 0);
    }

    #[test]
    fn counts_only_codex_incidents() {
        for component_status in ["operational", "degraded_performance"] {
            let summary = serde_json::from_value(serde_json::json!({
                "page": { "updated_at": "2026-09-01T08:00:00Z" },
                "components": [
                    { "id": "codex-api", "name": "Codex API", "status": component_status },
                    { "id": "chatgpt", "name": "ChatGPT", "status": "major_outage" }
                ],
                "incidents": [
                    { "status": "monitoring", "components": [{ "id": "codex-api" }] },
                    { "status": "investigating", "components": [{ "id": "chatgpt" }] },
                    { "status": "resolved", "components": [{ "id": "codex-api" }] },
                    { "status": "completed", "components": [{ "id": "codex-api" }] },
                    { "status": "investigating", "components": [] }
                ]
            }))
            .expect("valid incident summary");
            let projected = project(service("codex").expect("known service"), summary)
                .expect("valid status projection");
            assert_eq!(projected.active_incidents, 1);
        }
    }

    #[test]
    fn validates_catalog_ids() {
        assert!(valid_service_id("deepseek"));
        assert!(!valid_service_id("https://example.com/status"));
    }
}
