use crate::present;
use serde::{Deserialize, Serialize};

pub const MAX_RESULTS: u32 = 50;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub id: String,
    pub title: String,
    pub content: String,
    pub kind: Option<String>,
    pub state: Option<String>,
    pub source: Option<String>,
    pub importance: f64,
    pub created_at: String,
    pub relevance: Option<f64>,
}

#[derive(Deserialize)]
struct SearchWire {
    memories: Vec<MemoryWire>,
}

#[derive(Deserialize)]
struct MemoryWire {
    id: String,
    title: String,
    content: String,
    unit_type: Option<String>,
    lifecycle_state: Option<String>,
    source: Option<String>,
    importance: f64,
    created_at: String,
    relevance_score: Option<f64>,
}

impl From<MemoryWire> for Memory {
    fn from(wire: MemoryWire) -> Self {
        Self {
            id: wire.id,
            title: wire.title,
            content: wire.content,
            kind: present(wire.unit_type),
            state: present(wire.lifecycle_state),
            source: present(wire.source),
            importance: wire.importance,
            created_at: wire.created_at,
            relevance: wire.relevance_score,
        }
    }
}

pub async fn search(query: &str, limit: u32) -> Result<Vec<Memory>, String> {
    if query.trim().is_empty() {
        return Err("Memory search query cannot be empty".to_owned());
    }
    if !(1..=MAX_RESULTS).contains(&limit) {
        return Err(format!(
            "Memory search limit must be between 1 and {MAX_RESULTS}"
        ));
    }
    let wire: SearchWire = crate::get(
        "/memories/search",
        &[("q", query.trim().to_owned()), ("limit", limit.to_string())],
    )
    .await?;
    Ok(wire.memories.into_iter().map(Memory::from).collect())
}

pub async fn read(id: &str) -> Result<Memory, String> {
    let valid = !id.is_empty()
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'));
    if !valid {
        return Err(format!("Invalid Nowledge Mem memory ID: {id}"));
    }
    let wire: MemoryWire = crate::get(&format!("/memories/{id}"), &[]).await?;
    Ok(wire.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn search_projection() {
        let wire: SearchWire = serde_json::from_value(json!({
            "memories": [{
                "id": "m", "title": "Rule", "content": "Body", "unit_type": "preference",
                "lifecycle_state": "active", "source": " ", "importance": 0.8,
                "created_at": "2026-10-04T16:02:02+00:00", "relevance_score": 0.99,
                "metadata": {}, "label_ids": []
            }],
            "pagination": {}
        }))
        .unwrap();
        let memories: Vec<Memory> = wire.memories.into_iter().map(Memory::from).collect();

        assert_eq!(memories[0].kind.as_deref(), Some("preference"));
        assert_eq!(memories[0].source, None);
        assert_eq!(memories[0].relevance, Some(0.99));
    }
}
