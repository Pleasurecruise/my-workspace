use super::{EmbedError, canvas, escape_html, unquote};

pub(super) fn render(source: &str) -> Result<String, EmbedError> {
    let board = match source.lines().next() {
        Some(line) if line.trim_start().starts_with("align:") => {
            source[line.len()..].trim_start_matches(['\r', '\n'])
        }
        _ => source,
    };
    if board.trim_start().starts_with("<svg") {
        return canvas::render("storyboard", source);
    }

    let mut title = None;
    let mut align = "wide";
    let mut steps = Vec::new();
    for (index, raw) in source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
    {
        let Some((field, value)) = raw.split_once(':') else {
            return Err(EmbedError::InvalidLine {
                kind: "embed:storyboard".to_owned(),
                line: index + 1,
            });
        };
        match field.trim() {
            "align" => {
                align = unquote(value.trim());
                match align {
                    "left" | "right" | "wide" => {}
                    value => return Err(EmbedError::InvalidAlignment(value.to_owned())),
                }
            }
            "title" if title.is_none() => title = Some(value.trim()),
            "step" => {
                let Some((heading, body)) = value.split_once('|') else {
                    return Err(EmbedError::InvalidCanvas {
                        kind: "storyboard",
                        message: "each `step:` must use `heading | description`".to_owned(),
                    });
                };
                steps.push((heading.trim(), body.trim()));
            }
            _ => {
                return Err(EmbedError::InvalidCanvas {
                    kind: "storyboard",
                    message: format!("unsupported storyboard field `{}`", field.trim()),
                });
            }
        }
    }
    let Some(title) = title else {
        return Err(EmbedError::InvalidCanvas {
            kind: "storyboard",
            message: "the storyboard requires one `title:`".to_owned(),
        });
    };
    if steps.len() < 2 {
        return Err(EmbedError::InvalidCanvas {
            kind: "storyboard",
            message: "the storyboard requires between two and six `step:` lines".to_owned(),
        });
    }
    if steps.len() > 6 {
        return Err(EmbedError::InvalidCanvas {
            kind: "storyboard",
            message: "the storyboard requires between two and six `step:` lines".to_owned(),
        });
    }
    for (heading, body) in &steps {
        if heading.is_empty() {
            return Err(EmbedError::InvalidCanvas {
                kind: "storyboard",
                message: "storyboard headings cannot be empty".to_owned(),
            });
        }
        if body.is_empty() {
            return Err(EmbedError::InvalidCanvas {
                kind: "storyboard",
                message: "storyboard descriptions cannot be empty".to_owned(),
            });
        }
    }

    let width = 48 + steps.len() * 174 + steps.len().saturating_sub(1) * 54;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} 250\" role=\"img\"><title>{}</title><desc>An Excalidraw-style sequence of {} notes.</desc>",
        escape_html(title),
        steps.len(),
    );
    for (index, (heading, body)) in steps.iter().enumerate() {
        let x = 24 + index * 228;
        let top = if index % 2 == 0 { 46 } else { 52 };
        let color = ["fill-blue", "fill-violet", "fill-green", "fill-orange"][index % 4];
        svg.push_str(&format!(
            "<g class=\"{color}\"><path class=\"note\" d=\"M{x} {top} C{} {} {} {} {} {} L{} {} C{} {} {} {} {} {} Z\"/><path class=\"sketch-shadow\" d=\"M{} {} C{} {} {} {} {} {} L{} {} C{} {} {} {} {} {} Z\"/><text class=\"hand step\" x=\"{}\" y=\"{}\">{:02}</text><text class=\"hand title\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text><text class=\"hand caption\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text></g>",
            x + 42, top - 5, x + 132, top + 5, x + 174, top + 1,
            x + 172, top + 130, x + 132, top + 137, x + 84, top + 126, x - 3, top + 130,
            x + 2, top + 2, x + 44, top - 1, x + 130, top + 3, x + 172, top + 4,
            x + 169, top + 128, x + 132, top + 134, x + 86, top + 132, x - 1, top + 132,
            x + 14, top + 22, index + 1, x + 87, top + 57, escape_html(heading),
            x + 87, top + 84, escape_html(body),
        ));
        if index + 1 < steps.len() {
            let start = x + 184;
            let end = x + 218;
            svg.push_str(&format!(
                "<path class=\"arrow-shadow\" d=\"M{start} {} C{} {} {} {} {end} {} M{} {} L{end} {} L{} {}\"/><path class=\"arrow\" d=\"M{start} {} C{} {} {} {} {end} {} M{} {} L{end} {} L{} {}\"/>",
                top + 73, start + 11, top + 57, end - 9, top + 84, top + 71,
                end - 9, top + 62, top + 71, end - 9, top + 80,
                top + 71, start + 10, top + 54, end - 10, top + 86, top + 69,
                end - 10, top + 60, top + 69, end - 10, top + 78,
            ));
        }
    }
    svg.push_str("</svg>");
    canvas::render("storyboard", &format!("align: {align}\n{svg}"))
}
