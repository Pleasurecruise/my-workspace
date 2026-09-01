use super::{EmbedError, canvas, escape_html};

pub(super) fn render(source: &str) -> Result<String, EmbedError> {
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

    let width = 48 + nodes.len() * 160 + nodes.len().saturating_sub(1) * 64;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} 210\" role=\"img\"><title>Architecture flow</title><desc>A left-to-right system architecture diagram.</desc>"
    );
    for (index, (_, label)) in nodes.iter().enumerate() {
        let x = 24 + index * 224;
        let color = [
            "c-teal", "c-purple", "c-coral", "c-blue", "c-green", "c-amber",
        ][index % 6];
        svg.push_str(&format!(
            "<g class=\"node {color}\"><rect x=\"{x}\" y=\"61\" width=\"160\" height=\"88\" rx=\"12\"/><text class=\"th\" x=\"{}\" y=\"105\">{}</text><text class=\"ts\" x=\"{}\" y=\"126\">system boundary</text></g>",
            x + 20,
            escape_html(label),
            x + 20,
        ));
    }
    for (from, to) in edges {
        let from_i = nodes
            .iter()
            .position(|(id, _)| id == &from)
            .expect("flowchart edge source is registered");
        let to_i = nodes
            .iter()
            .position(|(id, _)| id == &to)
            .expect("flowchart edge target is registered");
        let start = 184 + from_i * 224;
        let end = 24 + to_i * 224;
        let middle = (start + end) / 2;
        svg.push_str(&format!(
            "<path class=\"arr\" d=\"M{start} 105 C{middle} 105 {middle} 105 {end} 105\"/><path class=\"arr\" d=\"M{} 99 L{end} 105 L{} 111\"/>",
            end.saturating_sub(8),
            end.saturating_sub(8),
        ));
    }
    svg.push_str("</svg>");
    let align = source
        .lines()
        .next()
        .filter(|line| line.trim_start().starts_with("align:"));
    match align {
        Some(line) => canvas::render("architecture", &format!("{}\n{svg}", line.trim())),
        None => canvas::render("architecture", &svg),
    }
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
        if id.is_empty() {
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
    if value.is_empty() {
        return Err(EmbedError::InvalidCanvas {
            kind: "architecture",
            message: "flowchart node IDs cannot be empty".to_owned(),
        });
    }
    Ok((value.to_owned(), value.to_owned()))
}
