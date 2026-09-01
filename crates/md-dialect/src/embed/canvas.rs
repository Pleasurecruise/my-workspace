use super::{EmbedError, unquote};
use std::io::Cursor;

pub(super) fn render(kind: &'static str, source: &str) -> Result<String, EmbedError> {
    let (align, svg) = match source.lines().next() {
        Some(line) if line.trim_start().starts_with("align:") => {
            let Some((_, value)) = line.split_once(':') else {
                return Err(EmbedError::InvalidLine {
                    kind: format!("embed:{kind}"),
                    line: 1,
                });
            };
            let align = unquote(value.trim());
            match align {
                "left" | "right" | "wide" => {}
                value => return Err(EmbedError::InvalidAlignment(value.to_owned())),
            }
            (align, source[line.len()..].trim_start_matches(['\r', '\n']))
        }
        _ => ("wide", source),
    };
    let svg = svg.trim();
    if !svg.starts_with("<svg") {
        return Err(EmbedError::InvalidCanvas {
            kind,
            message: "expected one `<svg>` document after the optional `align:` line".to_owned(),
        });
    }

    let mut sanitized = Vec::new();
    svg_hush::Filter::new()
        .filter(&mut Cursor::new(svg.as_bytes()), &mut sanitized)
        .map_err(|error| EmbedError::InvalidCanvas {
            kind,
            message: error.to_string(),
        })?;
    let sanitized = String::from_utf8(sanitized).map_err(|error| EmbedError::InvalidCanvas {
        kind,
        message: error.to_string(),
    })?;
    let Some(start) = sanitized.find("<svg") else {
        return Err(EmbedError::InvalidCanvas {
            kind,
            message: "the sanitizer did not return an SVG document".to_owned(),
        });
    };
    let svg = &sanitized[start..];
    if !svg.contains("<title") {
        return Err(EmbedError::InvalidCanvas {
            kind,
            message: "the SVG must contain both `<title>` and `<desc>`".to_owned(),
        });
    }
    if !svg.contains("<desc") {
        return Err(EmbedError::InvalidCanvas {
            kind,
            message: "the SVG must contain both `<title>` and `<desc>`".to_owned(),
        });
    }

    Ok(format!(
        "<figure class=\"content-embed svg-canvas svg-canvas-{kind} content-embed-{align}\">{svg}</figure>\n"
    ))
}
