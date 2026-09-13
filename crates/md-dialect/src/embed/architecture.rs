use super::{EmbedError, canvas, escape_html};

pub(super) fn render(source: &str) -> Result<String, EmbedError> {
    let source = source.trim();
    let diagram = match source.lines().next() {
        Some(line) if line.trim_start().starts_with("align:") => {
            source[line.len()..].trim_start_matches(['\r', '\n'])
        }
        _ => source,
    };
    if diagram.trim_start().starts_with("<svg") {
        return canvas::render("architecture", source);
    }

    let mut lines = diagram.lines().filter(|line| !line.trim().is_empty());
    match lines.next().map(str::trim) {
        Some("flowchart LR" | "graph LR") => {}
        _ => {
            return Err(EmbedError::InvalidCanvas {
                kind: "architecture",
                message: "expected an SVG canvas or a `flowchart LR` diagram".to_owned(),
            });
        }
    }
    let mut nodes: Vec<(String, String)> = Vec::new();
    let mut edges = Vec::new();
    for line in lines {
        let Some((from, to)) = line.trim().split_once("-->") else {
            return Err(EmbedError::InvalidCanvas {
                kind: "architecture",
                message: format!("unsupported flowchart edge `{}`", line.trim()),
            });
        };
        if to.contains("-->") {
            return Err(EmbedError::InvalidCanvas {
                kind: "architecture",
                message: "each edge requires exactly two endpoints".to_owned(),
            });
        }
        let from = node(from.trim())?;
        let to = node(to.trim())?;
        for item in [&from, &to] {
            if nodes.iter().all(|(id, _)| id != &item.0) {
                nodes.push(item.clone());
            }
        }
        edges.push((from.0, to.0));
    }
    if nodes.len() < 2 {
        return Err(EmbedError::InvalidCanvas {
            kind: "architecture",
            message: "the flowchart must contain at least one edge".to_owned(),
        });
    }

    let mut incoming = vec![0; nodes.len()];
    let edges: Vec<_> = edges
        .into_iter()
        .map(|(from, to)| {
            (
                nodes
                    .iter()
                    .position(|node| node.0 == from)
                    .expect("registered source"),
                nodes
                    .iter()
                    .position(|node| node.0 == to)
                    .expect("registered target"),
            )
        })
        .collect();
    for &(_, to) in &edges {
        incoming[to] += 1;
    }
    let mut levels = vec![0; nodes.len()];
    let mut queue: std::collections::VecDeque<_> = (0..nodes.len())
        .filter(|&index| incoming[index] == 0)
        .collect();
    let mut visited = vec![false; nodes.len()];
    while let Some(from) = queue.pop_front() {
        visited[from] = true;
        for &(_, to) in edges.iter().filter(|&&(source, _)| source == from) {
            levels[to] = levels[to].max(levels[from] + 1);
            incoming[to] -= 1;
            if incoming[to] == 0 {
                queue.push_back(to);
            }
        }
    }
    let mut next = (0..nodes.len())
        .filter(|&i| visited[i])
        .map(|i| levels[i] + 1)
        .max()
        .unwrap_or(0);
    for index in 0..nodes.len() {
        if !visited[index] {
            levels[index] = next;
            next += 1;
        }
    }
    let mut groups = vec![Vec::new(); levels.iter().max().copied().unwrap_or(0) + 1];
    for (index, &level) in levels.iter().enumerate() {
        groups[level].push(index);
    }
    let align = source
        .lines()
        .next()
        .filter(|line| line.trim_start().starts_with("align:"));
    let render = |compact| {
        let svg = render_diagram(&nodes, &edges, &levels, &groups, compact);
        let source = match align {
            Some(line) => format!("{}\n{svg}", line.trim()),
            None => svg,
        };
        canvas::render("architecture", &source)
    };
    let wide = render(false)?;
    let compact = render(true)?;
    let alignment = align
        .and_then(|line| line.split_once(':'))
        .map(|(_, value)| super::unquote(value.trim()))
        .unwrap_or("wide");
    Ok(format!(
        "<div class=\"architecture-flow content-embed-{alignment}\"><div class=\"architecture-wide\">{wide}</div><div class=\"architecture-compact\">{compact}</div></div>\n"
    ))
}

fn render_diagram(
    nodes: &[(String, String)],
    edges: &[(usize, usize)],
    levels: &[usize],
    groups: &[Vec<usize>],
    compact: bool,
) -> String {
    let mut positions = vec![(0, 0); nodes.len()];
    let (width, height) = if compact {
        let mut rows = 0;
        for group in groups {
            for (column, &index) in group.iter().enumerate() {
                let x = if group.len() % 2 == 1 && column + 1 == group.len() {
                    120
                } else {
                    32 + column % 2 * 176
                };
                positions[index] = (x, 24 + (rows + column / 2) * 120);
            }
            rows += group.len().div_ceil(2);
        }
        (400, rows * 120 + 8)
    } else {
        let rows = groups.iter().map(Vec::len).max().unwrap_or(1);
        let top = 24
            + edges
                .iter()
                .filter(|&&(from, to)| levels[to] != levels[from] + 1)
                .count()
                * 14;
        for (column, group) in groups.iter().enumerate() {
            for (row, &index) in group.iter().enumerate() {
                positions[index] = (
                    24 + column * 224,
                    top + (rows - group.len()) * 56 + row * 112,
                );
            }
        }
        (groups.len() * 224 + 8, top + rows * 112)
    };
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\" role=\"img\"><title>Architecture flow</title><desc>System boundaries grouped by dependency.</desc>"
    );
    let mut rail = 12;
    for &(from, to) in edges {
        let (x, y) = positions[from];
        let (end_x, end_y) = positions[to];
        let path = if compact {
            let x = x + 80;
            let y = y + 80;
            let end_x = end_x + 80;
            let path = if end_y == y + 40 {
                format!(
                    "M{x} {y} C{x} {} {end_x} {} {end_x} {end_y}",
                    y + 20,
                    end_y - 20
                )
            } else {
                let outer = 8 + (rail % 5) * 4;
                rail += 1;
                format!(
                    "M{x} {y} L{x} {} L{outer} {} L{outer} {} L{end_x} {} L{end_x} {end_y}",
                    y + 16,
                    y + 16,
                    end_y - 16,
                    end_y - 16
                )
            };
            format!(
                "{path} M{} {} L{end_x} {end_y} L{} {}",
                end_x - 6,
                end_y - 8,
                end_x + 6,
                end_y - 8
            )
        } else {
            let x = x + 160;
            let y = y + 40;
            let end_y = end_y + 40;
            let path = if levels[to] == levels[from] + 1 {
                format!(
                    "M{x} {y} C{} {y} {} {end_y} {end_x} {end_y}",
                    x + 32,
                    x + 32
                )
            } else {
                let path = format!(
                    "M{x} {y} L{} {y} L{} {rail} L{} {rail} L{} {end_y} L{end_x} {end_y}",
                    x + 20,
                    x + 20,
                    end_x - 20,
                    end_x - 20
                );
                rail += 14;
                path
            };
            format!(
                "{path} M{} {} L{end_x} {end_y} L{} {}",
                end_x - 8,
                end_y - 6,
                end_x - 8,
                end_y + 6
            )
        };
        svg.push_str(&format!("<path class=\"arr\" d=\"{path}\"/>"));
    }
    for (index, (_, label)) in nodes.iter().enumerate() {
        let (x, y) = positions[index];
        let color = [
            "c-teal", "c-purple", "c-coral", "c-blue", "c-green", "c-amber",
        ][levels[index] % 6];
        svg.push_str(&format!("<g class=\"node {color}\"><rect x=\"{x}\" y=\"{y}\" width=\"160\" height=\"80\" rx=\"12\"/><text class=\"th\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text></g>", x + 80, y + 45, escape_html(label)));
    }
    svg.push_str("</svg>");
    svg
}

fn node(value: &str) -> Result<(String, String), EmbedError> {
    if let Some(open) = value.find('[') {
        let Some(label) = value.strip_suffix(']') else {
            return Err(EmbedError::InvalidCanvas {
                kind: "architecture",
                message: format!("invalid flowchart node `{value}`"),
            });
        };
        let id = value[..open].trim();
        let label = label[open + 1..].trim();
        if id.is_empty() || id.contains(['[', ']']) || label.contains(['[', ']']) {
            return Err(EmbedError::InvalidCanvas {
                kind: "architecture",
                message: format!("invalid flowchart node `{value}`"),
            });
        }
        if label.is_empty() {
            return Err(EmbedError::InvalidCanvas {
                kind: "architecture",
                message: format!("invalid flowchart node `{value}`"),
            });
        }
        return Ok((id.to_owned(), label.to_owned()));
    }
    if value.is_empty() || value.contains(['[', ']']) {
        return Err(EmbedError::InvalidCanvas {
            kind: "architecture",
            message: "flowchart node IDs cannot be empty".to_owned(),
        });
    }
    Ok((value.to_owned(), value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branching_article_flow_uses_dependency_columns_and_a_tall_compact_layout() {
        let html = render("flowchart LR\nBrowser[Browser] --> API[Article API]\nVesper[Vesper CLI] --> API[Article API]\nAPI --> Service[Validate and save]\nService --> D1[Metadata]\nService --> R2[Markdown]\nService --> KV[Cache]\nService --> Search[Index]").unwrap();
        assert!(html.contains("viewBox=\"0 0 904 472\""));
        assert!(html.contains("viewBox=\"0 0 400 608\""));
        assert!(!html.contains("system boundary"));
        assert_eq!(html.matches("<title>Architecture flow</title>").count(), 2);
        for label in [
            "Browser",
            "Vesper CLI",
            "Article API",
            "Validate and save",
            "Metadata",
            "Markdown",
            "Cache",
            "Index",
        ] {
            assert_eq!(html.matches(&format!(">{label}</text>")).count(), 2);
        }
    }

    #[test]
    fn cycles_and_self_edges_have_bounded_finite_layouts() {
        for source in [
            "flowchart LR\na --> b\nb --> a",
            "flowchart LR\na --> a\na --> b",
            "flowchart LR\na --> b\nb --> c\nc --> b",
            "flowchart LR\na --> b\na --> c\nc --> d\nb --> d\na --> d",
        ] {
            let html = render(source).unwrap();
            assert!(html.contains("architecture-compact"));
            assert!(!html.contains("NaN"));
            assert!(!html.contains("inf"));
        }
    }

    #[test]
    fn authored_svg_keeps_its_own_geometry() {
        let html = render("align: narrow\n<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 680 230\"><title>Custom</title><desc>Authored layout</desc></svg>").unwrap();
        assert!(!html.contains("architecture-flow"));
        assert!(html.contains("viewBox=\"0 0 680 230\""));
        assert!(html.contains("content-embed-narrow"));
    }
}
