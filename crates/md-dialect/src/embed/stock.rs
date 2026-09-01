use super::{Data, EmbedError, escape_html, reject_unknown, required};
use std::collections::HashMap;

pub(super) fn render(mut fields: HashMap<&str, &str>, data: &Data) -> Result<String, EmbedError> {
    reject_unknown("embed:stock", &fields, &["code", "align"])?;
    let code = required(&mut fields, "stock", "code")?.to_ascii_uppercase();
    if !valid(&code) {
        return Err(EmbedError::InvalidStockCode(code));
    }
    let align = fields.remove("align").unwrap_or("wide");
    match align {
        "left" | "right" | "wide" => {}
        value => return Err(EmbedError::InvalidAlignment(value.to_owned())),
    }
    let stock = match data.stocks.get(&code) {
        Some(stock) => stock,
        None => {
            return Err(EmbedError::MissingData {
                kind: "stock",
                id: code.clone(),
            });
        }
    };
    let trend = if stock.change >= 0.0 { "up" } else { "down" };
    let sign = if stock.change >= 0.0 { "+" } else { "" };
    let (chart, area, end_x, end_y) = chart(&stock.points);
    let gradient = code
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    Ok(format!(
        "<aside class=\"content-embed content-embed-stock content-embed-{align} content-embed-{trend}\" data-stock-code=\"{code}\" aria-label=\"Stock {code}\"><span class=\"content-stock-quote\"><span class=\"content-embed-label\"><svg class=\"content-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M12 16v5\"/><path d=\"M16 14.639V21\"/><path d=\"M20 10.656V21\"/><path d=\"m22 3-8.646 8.646a.5.5 0 0 1-.708 0L9.354 8.354a.5.5 0 0 0-.707 0L2 15\"/><path d=\"M4 18.463V21\"/><path d=\"M8 14.656V21\"/></svg>{exchange} · {currency}</span><strong>{code}</strong><span class=\"content-stock-name\">{name}</span><span class=\"content-stock-price\">{price:.2}</span><span class=\"content-stock-change\"><svg class=\"content-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M16 7h6v6\"/><path d=\"m22 7-8.5 8.5-5-5L2 17\"/></svg>{sign}{change:.2} · {sign}{percent:.2}%</span></span><span class=\"content-stock-visual\"><svg class=\"content-stock-chart\" viewBox=\"0 0 320 112\" preserveAspectRatio=\"none\" role=\"img\" aria-label=\"One month price trend\"><defs><linearGradient id=\"stock-gradient-{gradient}\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\"><stop offset=\"0\" stop-color=\"currentColor\" stop-opacity=\".2\"/><stop offset=\"1\" stop-color=\"currentColor\" stop-opacity=\"0\"/></linearGradient></defs><path class=\"content-stock-area\" d=\"{area}\" fill=\"url(#stock-gradient-{gradient})\"/><path class=\"content-stock-line\" d=\"{chart}\"/><circle class=\"content-stock-end\" cx=\"{end_x:.1}\" cy=\"{end_y:.1}\" r=\"3\"/></svg><span class=\"content-stock-range\"><span>1M</span><span>{first:.2}</span><span>{last:.2}</span></span></span></aside>\n",
        exchange = escape_html(&stock.exchange),
        currency = escape_html(&stock.currency),
        name = escape_html(&stock.name),
        price = stock.price,
        change = stock.change,
        percent = stock.change_percent,
        first = stock
            .points
            .first()
            .map_or(stock.price, |point| point.close),
        last = stock.points.last().map_or(stock.price, |point| point.close),
    ))
}

fn chart(points: &[quotes::stocks::StockPoint]) -> (String, String, f64, f64) {
    let min = points
        .iter()
        .map(|point| point.close)
        .fold(f64::INFINITY, f64::min);
    let max = points
        .iter()
        .map(|point| point.close)
        .fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(f64::EPSILON);
    let last = points.len().saturating_sub(1).max(1) as f64;
    let points = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let x = index as f64 / last * 320.0;
            let y = 96.0 - (point.close - min) / range * 80.0;
            (x, y)
        })
        .collect::<Vec<_>>();
    let Some(&(start_x, start_y)) = points.first() else {
        return (String::new(), String::new(), 0.0, 0.0);
    };
    let mut line = format!("M{start_x:.1} {start_y:.1}");
    for index in 0..points.len().saturating_sub(1) {
        let previous = points[index.saturating_sub(1)];
        let current = points[index];
        let next = points[index + 1];
        let following = points.get(index + 2).copied().unwrap_or(next);
        let control_1 = (
            current.0 + (next.0 - previous.0) / 6.0,
            current.1 + (next.1 - previous.1) / 6.0,
        );
        let control_2 = (
            next.0 - (following.0 - current.0) / 6.0,
            next.1 - (following.1 - current.1) / 6.0,
        );
        line.push_str(&format!(
            " C{:.1} {:.1},{:.1} {:.1},{:.1} {:.1}",
            control_1.0, control_1.1, control_2.0, control_2.1, next.0, next.1
        ));
    }
    let (end_x, end_y) = points.last().copied().unwrap_or((start_x, start_y));
    let area = format!("{line} L{end_x:.1} 112 L{start_x:.1} 112 Z");
    (line, area, end_x, end_y)
}

pub(super) fn valid(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.len() > 20 {
        return false;
    }
    value.bytes().all(|byte| {
        matches!(
            byte,
            b'.' | b'-' | b'^' | b'=' | b':' | b'A'..=b'Z' | b'0'..=b'9'
        )
    })
}
