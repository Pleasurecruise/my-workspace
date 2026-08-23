use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct CompiledKnowledge {
    pub html: String,
    pub toc: Vec<TocEntry>,
    pub excerpt: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TocEntry {
    pub id: String,
    pub text: String,
    pub depth: u8,
}

pub fn render(source: &str) -> String {
    let parser = Parser::new_ext(source, options()).map(|event| normalize(event, false));
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

pub fn render_memo(source: &str) -> String {
    let parser = Parser::new_ext(source, options()).map(|event| normalize(event, true));
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

pub fn compile_knowledge(source: &str) -> CompiledKnowledge {
    let mut heading_text: Option<String> = None;
    let mut headings: Vec<(HeadingLevel, String)> = Vec::new();
    let mut excerpt = String::new();

    for event in Parser::new_ext(source, options()).map(|event| normalize(event, false)) {
        match event {
            Event::Start(Tag::Heading { .. }) => heading_text = Some(String::new()),
            Event::Text(text) | Event::Code(text) => {
                if let Some(current_heading) = &mut heading_text {
                    current_heading.push_str(&text);
                }
                excerpt.push_str(&text);
            }
            Event::End(TagEnd::Heading(level)) => {
                if let Some(text) = heading_text.take() {
                    headings.push((level, text));
                }
                excerpt.push(' ');
            }
            Event::SoftBreak | Event::HardBreak => excerpt.push(' '),
            Event::End(
                TagEnd::Paragraph
                | TagEnd::CodeBlock
                | TagEnd::Item
                | TagEnd::TableCell
                | TagEnd::TableRow,
            ) => excerpt.push(' '),
            _ => {}
        }
    }

    let mut slugs: HashMap<String, usize> = HashMap::new();
    let toc: Vec<TocEntry> = headings
        .into_iter()
        .map(|(level, text)| {
            let base = heading_id(&text);
            let count = slugs.entry(base.clone()).or_insert(0);
            *count += 1;
            let id = if *count == 1 {
                base
            } else {
                format!("{base}-{}", *count)
            };
            TocEntry {
                id,
                text,
                depth: match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                },
            }
        })
        .collect();
    let mut heading_index = 0;
    let events = Parser::new_ext(source, options())
        .map(|event| normalize(event, false))
        .map(|event| match event {
            Event::Start(Tag::Heading {
                level,
                classes,
                attrs,
                ..
            }) => {
                let id = toc[heading_index].id.clone().into();
                heading_index += 1;
                Event::Start(Tag::Heading {
                    level,
                    id: Some(id),
                    classes,
                    attrs,
                })
            }
            event => event,
        });
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events);
    let excerpt = excerpt
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(240)
        .collect();
    CompiledKnowledge { html, toc, excerpt }
}

fn normalize<'a>(event: Event<'a>, hard_breaks: bool) -> Event<'a> {
    match event {
        Event::Html(text) | Event::InlineHtml(text) => Event::Text(text),
        Event::SoftBreak if hard_breaks => Event::HardBreak,
        event => event,
    }
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

fn heading_id(text: &str) -> String {
    let value = text
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let value = value
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<&str>>()
        .join("-");
    if value.is_empty() {
        "section".to_owned()
    } else {
        value
    }
}

#[cfg(test)]
#[path = "../tests/unit/markdown.rs"]
mod tests;
