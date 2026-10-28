use pulldown_cmark::{Event, Options, Parser, TagEnd};

const TELEGRAM_LIMIT: usize = 4_096;
const X_LIMIT: usize = 280;

pub(super) fn render_telegram(content: &str) -> String {
    truncate(&render_plain(content), TELEGRAM_LIMIT, |text| {
        text.encode_utf16().count()
    })
}

pub(super) fn render_x(content: &str) -> String {
    truncate(&render_plain(content), X_LIMIT, measure_x)
}

fn truncate(text: &str, limit: usize, measure: impl Fn(&str) -> usize) -> String {
    if measure(text) <= limit {
        return text.to_owned();
    }
    let ellipsis = "…";
    let target = limit.saturating_sub(measure(ellipsis));
    let mut end = 0;
    for (index, character) in text.char_indices() {
        let next = index + character.len_utf8();
        if measure(&text[..next]) > target {
            break;
        }
        end = next;
    }
    format!("{}{}", text[..end].trim_end(), ellipsis)
}

fn measure_x(text: &str) -> usize {
    text.chars()
        .map(|character| if character.is_ascii() { 1 } else { 2 })
        .sum()
}

fn render_plain(source: &str) -> String {
    let mut output = String::new();
    for event in Parser::new_ext(source, Options::all()) {
        match event {
            Event::Text(text)
            | Event::Code(text)
            | Event::InlineMath(text)
            | Event::DisplayMath(text)
            | Event::Html(text)
            | Event::InlineHtml(text) => output.push_str(&text),
            Event::SoftBreak | Event::HardBreak => output.push('\n'),
            Event::Rule => output.push_str("\n\n"),
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::CodeBlock
                | TagEnd::Item
                | TagEnd::TableRow,
            ) => output.push_str("\n\n"),
            _ => {}
        }
    }
    output
        .lines()
        .map(str::trim)
        .fold((String::new(), false), |(mut text, blank), line| {
            if line.is_empty() {
                if !text.is_empty() && !blank {
                    text.push('\n');
                }
                (text, true)
            } else {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(line);
                (text, false)
            }
        })
        .0
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown() {
        let text = render_telegram("# Release\n\nA **useful** [memo](https://example.com).");
        assert_eq!(text, "Release\n\nA useful memo.");
    }

    #[test]
    fn limits_telegram() {
        let text = render_telegram(&"😀".repeat(3_000));
        assert!(text.encode_utf16().count() <= TELEGRAM_LIMIT);
        assert!(text.contains('…'));
    }

    #[test]
    fn limits_x() {
        let text = render_x(&"发布内容".repeat(100));
        assert!(measure_x(&text) <= X_LIMIT);
        assert!(text.contains('…'));
    }
}
