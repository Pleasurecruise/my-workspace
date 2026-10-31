use crate::present;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap, HashMap};

pub const MAX_NODES: u32 = 1000;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub communities: Vec<Community>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    Memory,
    Entity,
    Thread,
    Skill,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub title: String,
    pub summary: String,
    pub category: Option<String>,
    pub source: Option<String>,
    pub updated_at: Option<String>,
    pub recorded_at: Option<Timestamp>,
    pub degree: usize,
    pub core: usize,
    pub influence: f64,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub reason: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Community {
    pub id: String,
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Deserialize)]
struct GraphWire {
    nodes: Vec<NodeWire>,
    edges: Vec<EdgeWire>,
    #[serde(default)]
    community_hulls: Vec<CommunityWire>,
}

#[derive(Deserialize)]
struct NodeWire {
    id: String,
    label: String,
    node_type: String,
    #[serde(default)]
    importance: Option<f64>,
    #[serde(default)]
    metadata: NodeMetadata,
}

#[derive(Default, Deserialize)]
struct NodeMetadata {
    title: Option<String>,
    description: Option<String>,
    content: Option<String>,
    entity_type: Option<String>,
    kind: Option<String>,
    source: Option<String>,
    updated_at: Option<String>,
    created_at: Option<String>,
}

#[derive(Deserialize)]
struct EdgeWire {
    source: String,
    target: String,
    edge_type: String,
    #[serde(default)]
    metadata: EdgeMetadata,
}

#[derive(Default, Deserialize)]
struct EdgeMetadata {
    reason: Option<String>,
}

#[derive(Deserialize)]
struct CommunityWire {
    community_id: String,
    community_name: String,
    member_nodes: Vec<String>,
}

fn ranks(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|left, right| values[*left].total_cmp(&values[*right]));
    let mut ranks = vec![0.5; values.len()];
    let mut start = 0;
    while start < order.len() {
        let mut end = start;
        while end + 1 < order.len() && values[order[end + 1]] == values[order[start]] {
            end += 1;
        }
        if order.len() > 1 {
            let rank = (start + end) as f64 / 2.0 / (order.len() - 1) as f64;
            for index in &order[start..=end] {
                ranks[*index] = rank;
            }
        }
        start = end + 1;
    }
    ranks
}

fn cores(neighbours: &[BTreeSet<usize>]) -> Vec<usize> {
    let mut remaining: Vec<usize> = neighbours.iter().map(BTreeSet::len).collect();
    let mut queue: BinaryHeap<Reverse<(usize, usize)>> = remaining
        .iter()
        .enumerate()
        .map(|(index, degree)| Reverse((*degree, index)))
        .collect();
    let mut removed = vec![false; neighbours.len()];
    let mut cores = vec![0; neighbours.len()];
    let mut level = 0;
    while let Some(Reverse((degree, index))) = queue.pop() {
        if removed[index] || degree != remaining[index] {
            continue;
        }
        level = level.max(degree);
        cores[index] = level;
        removed[index] = true;
        for neighbour in &neighbours[index] {
            if !removed[*neighbour] {
                remaining[*neighbour] -= 1;
                queue.push(Reverse((remaining[*neighbour], *neighbour)));
            }
        }
    }
    cores
}

fn project(wire: GraphWire) -> Graph {
    let mut importance = Vec::new();
    let mut nodes: Vec<Node> = wire
        .nodes
        .into_iter()
        .filter_map(|node| {
            let kind = match node.node_type.as_str() {
                "memory" => NodeKind::Memory,
                "entity" => NodeKind::Entity,
                "thread" => NodeKind::Thread,
                "skill" => NodeKind::Skill,
                _ => return None,
            };
            let metadata = node.metadata;
            let created_at = present(metadata.created_at);
            let updated_at = present(metadata.updated_at);
            importance.push(node.importance.unwrap_or_default());
            Some(Node {
                title: present(metadata.title).unwrap_or(node.label),
                summary: present(metadata.description)
                    .or(present(metadata.content))
                    .unwrap_or_default(),
                category: present(metadata.entity_type).or(present(metadata.kind)),
                source: present(metadata.source),
                recorded_at: created_at
                    .as_ref()
                    .or(updated_at.as_ref())
                    .and_then(|value| value.parse().ok()),
                updated_at: updated_at.or(created_at),
                degree: 0,
                core: 0,
                influence: 0.0,
                id: node.id,
                kind,
            })
        })
        .collect();
    let positions: HashMap<String, usize> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();
    let mut neighbours = vec![BTreeSet::new(); nodes.len()];
    let edges: Vec<Edge> = wire
        .edges
        .into_iter()
        .filter_map(|edge| {
            let source = *positions.get(&edge.source)?;
            let target = *positions.get(&edge.target)?;
            if source == target {
                return None;
            }
            neighbours[source].insert(target);
            neighbours[target].insert(source);
            Some(Edge {
                source: edge.source,
                target: edge.target,
                relation: edge.edge_type,
                reason: present(edge.metadata.reason),
            })
        })
        .collect();

    let degrees: Vec<f64> = neighbours.iter().map(|set| set.len() as f64).collect();
    let connectivity = ranks(&degrees);
    let mut prominence = vec![0.5; nodes.len()];
    for kind in [
        NodeKind::Memory,
        NodeKind::Entity,
        NodeKind::Thread,
        NodeKind::Skill,
    ] {
        let members: Vec<usize> = (0..nodes.len())
            .filter(|index| nodes[*index].kind == kind)
            .collect();
        let values: Vec<f64> = members.iter().map(|index| importance[*index]).collect();
        for (index, rank) in members.iter().zip(ranks(&values)) {
            prominence[*index] = rank;
        }
    }
    // Entities carry no timestamps, so they take the latest time of the records that mention them.
    let inherited: Vec<Option<Timestamp>> = neighbours
        .iter()
        .map(|set| {
            set.iter()
                .filter_map(|index| nodes[*index].recorded_at)
                .max()
        })
        .collect();
    for (index, (node, core)) in nodes.iter_mut().zip(cores(&neighbours)).enumerate() {
        node.degree = neighbours[index].len();
        node.core = core;
        node.influence = (prominence[index] + connectivity[index]) / 2.0;
        if node.recorded_at.is_none() {
            node.recorded_at = inherited[index];
        }
    }

    let communities = wire
        .community_hulls
        .into_iter()
        .map(|community| Community {
            members: community
                .member_nodes
                .into_iter()
                .filter(|member| positions.contains_key(member))
                .collect(),
            id: community.community_id,
            name: community.community_name,
        })
        .filter(|community| !community.members.is_empty())
        .collect();
    Graph {
        nodes,
        edges,
        communities,
    }
}

pub async fn read(limit: u32) -> Result<Graph, String> {
    if !(1..=MAX_NODES).contains(&limit) {
        return Err(format!("Graph limit must be between 1 and {MAX_NODES}"));
    }
    let wire = crate::get("/graph/overview", &[("limit", limit.to_string())]).await?;
    Ok(project(wire))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn projection() {
        let wire: GraphWire = serde_json::from_value(json!({
            "nodes": [
                {"id": "m", "label": "short", "node_type": "memory", "importance": 0.8, "community": null,
                 "metadata": {"title": "Memory title", "description": " ", "content": "Body", "source": "cli", "created_at": "2026-10-01T00:00:00Z"}},
                {"id": "e", "label": "Rust", "node_type": "entity", "importance": 0.1, "community": "4",
                 "metadata": {"title": "", "description": "Language", "entity_type": "TECHNOLOGY"}},
                {"id": "x", "label": "unknown", "node_type": "label", "metadata": {}}
            ],
            "edges": [
                {"source": "m", "target": "e", "edge_type": "MENTIONS", "metadata": {"reason": "uses Rust"}},
                {"source": "m", "target": "x", "edge_type": "RELATES_TO", "metadata": {}},
                {"source": "m", "target": "m", "edge_type": "EVOLVES", "metadata": {}}
            ],
            "community_hulls": [
                {"community_id": "4", "community_name": "Rust +1", "member_nodes": ["e", "x"]},
                {"community_id": "5", "community_name": "Gone", "member_nodes": ["x"]}
            ]
        }))
        .unwrap();
        let graph = project(wire);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.nodes[0].degree, 1);
        assert_eq!(
            graph.nodes[1].recorded_at,
            Some("2026-10-01T00:00:00Z".parse().unwrap())
        );
        assert_eq!(graph.nodes[0].title, "Memory title");
        assert_eq!(graph.nodes[0].summary, "Body");
        assert_eq!(
            graph.nodes[0].updated_at.as_deref(),
            Some("2026-10-01T00:00:00Z")
        );
        assert_eq!(graph.nodes[1].title, "Rust");
        assert_eq!(graph.nodes[1].category.as_deref(), Some("TECHNOLOGY"));
        assert_eq!(
            graph.edges,
            vec![Edge {
                source: "m".into(),
                target: "e".into(),
                relation: "MENTIONS".into(),
                reason: Some("uses Rust".into()),
            }]
        );
        assert_eq!(
            graph.communities,
            vec![Community {
                id: "4".into(),
                name: "Rust +1".into(),
                members: vec!["e".into()],
            }]
        );
    }

    #[test]
    fn structure() {
        let edge = |source: usize, target: usize| json!({"source": format!("n{source}"), "target": format!("n{target}"), "edge_type": "RELATES_TO", "metadata": {}});
        let wire: GraphWire = serde_json::from_value(json!({
            "nodes": (0..6).map(|index| json!({
                "id": format!("n{index}"), "label": "node", "node_type": "entity",
                "importance": f64::from(index), "metadata": {}
            })).collect::<Vec<_>>(),
            "edges": [edge(0, 1), edge(1, 2), edge(2, 0), edge(0, 1), edge(2, 3), edge(3, 4)]
        }))
        .unwrap();
        let graph = project(wire);
        let cores: Vec<usize> = graph.nodes.iter().map(|node| node.core).collect();

        assert_eq!(cores, vec![2, 2, 2, 1, 1, 0]);
        assert_eq!(graph.nodes[0].degree, 2);
        assert!(graph.nodes[2].influence > graph.nodes[5].influence);
        assert!(graph.nodes[5].recorded_at.is_none());
    }

    #[test]
    fn tied_ranks() {
        assert_eq!(ranks(&[1.0, 1.0, 1.0]), vec![0.5, 0.5, 0.5]);
        assert_eq!(ranks(&[3.0, 1.0, 2.0]), vec![1.0, 0.0, 0.5]);
        assert_eq!(ranks(&[4.0]), vec![0.5]);
    }

    #[tokio::test]
    #[ignore = "requires configured Nowledge Mem credentials"]
    async fn live_graph() {
        let graph = read(50).await.expect("graph should be readable");

        assert!(!graph.nodes.is_empty());
    }
}
