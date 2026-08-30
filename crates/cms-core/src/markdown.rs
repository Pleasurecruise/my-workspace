use linkify::{LinkFinder, LinkKind};
use pulldown_cmark::{
    CodeBlockKind, CowStr, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd, html,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::OnceLock;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

const CODE_THEME: &str = "InspiredGitHub";

struct CodeBlock {
    language: String,
    source: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PublicationError {
    #[error("could not highlight code block: {0}")]
    Highlight(#[from] syntect::Error),
    #[error("could not render Mermaid diagram: {0}")]
    Mermaid(#[from] mermaid_svg::RenderError),
}

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

pub fn render_publication(source: &str) -> Result<String, PublicationError> {
    let mut events = Vec::new();
    let mut code_block: Option<CodeBlock> = None;

    for event in Parser::new_ext(source, options()) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Indented => String::new(),
                    CodeBlockKind::Fenced(info) => info
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_ascii_lowercase(),
                };
                code_block = Some(CodeBlock {
                    language,
                    source: String::new(),
                });
            }
            Event::Text(text) if code_block.is_some() => {
                if let Some(block) = &mut code_block {
                    block.source.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                let block = code_block.take().expect("code block start precedes end");
                let rendered = if block.language == "mermaid" {
                    let svg = mermaid_svg::render(&block.source)?;
                    format!("<figure class=\"mermaid-diagram\">{svg}</figure>\n")
                } else {
                    highlight_code(&block.source, &block.language)?
                };
                events.push(Event::Html(CowStr::Boxed(rendered.into_boxed_str())));
            }
            event if code_block.is_none() => events.push(normalize(event, false)),
            _ => {}
        }
    }

    let mut output = String::new();
    html::push_html(&mut output, events.into_iter());
    Ok(output)
}

fn highlight_code(source: &str, language: &str) -> Result<String, syntect::Error> {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    static THEMES: OnceLock<ThemeSet> = OnceLock::new();

    let syntaxes = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
    let syntax = syntaxes
        .find_syntax_by_token(language)
        .or_else(|| syntaxes.find_syntax_by_extension(language))
        .or_else(|| syntaxes.find_syntax_by_name(language))
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
    let themes = THEMES.get_or_init(ThemeSet::load_defaults);
    let html = highlighted_html_for_string(source, syntaxes, syntax, &themes.themes[CODE_THEME])?;
    Ok(format!("<div class=\"highlighted-code\">{html}</div>\n"))
}

pub fn render_memo(source: &str) -> String {
    let mut protected_depth = 0;
    let mut events = Vec::new();
    for event in Parser::new_ext(source, options()).map(|event| normalize(event, true)) {
        if matches!(
            &event,
            Event::Start(Tag::CodeBlock(_) | Tag::Link { .. } | Tag::Image { .. })
        ) {
            protected_depth += 1;
        }
        let protected_end = matches!(
            &event,
            Event::End(TagEnd::CodeBlock | TagEnd::Link | TagEnd::Image)
        );
        match event {
            Event::Text(text) if protected_depth == 0 => events.extend(autolink_text(&text)),
            event => events.push(event),
        }
        if protected_end {
            protected_depth -= 1;
        }
    }
    let mut output = String::new();
    html::push_html(&mut output, events.into_iter());
    output
}

fn autolink_text(text: &str) -> Vec<Event<'static>> {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    let mut events = Vec::new();
    let mut cursor = 0;
    for link in finder.links(text) {
        if cursor < link.start() {
            events.push(Event::Text(text[cursor..link.start()].to_owned().into()));
        }
        let url = link.as_str().to_owned();
        events.push(Event::Start(Tag::Link {
            link_type: LinkType::Autolink,
            dest_url: url.clone().into(),
            title: "".into(),
            id: "".into(),
        }));
        events.push(Event::Text(url.into()));
        events.push(Event::End(TagEnd::Link));
        cursor = link.end();
    }
    if cursor < text.len() {
        events.push(Event::Text(text[cursor..].to_owned().into()));
    }
    events
}

pub fn compile_knowledge(source: &str) -> CompiledKnowledge {
    let source = knowledge_body(source);
    let mut heading_text: Option<String> = None;
    let mut headings: Vec<(HeadingLevel, String)> = Vec::new();
    let mut excerpt = String::new();

    for event in
        Parser::new_ext(source, knowledge_options()).map(|event| normalize_knowledge(event))
    {
        match event {
            Event::Start(Tag::Heading { .. }) => heading_text = Some(String::new()),
            Event::Text(text)
            | Event::Code(text)
            | Event::InlineMath(text)
            | Event::DisplayMath(text) => {
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
    let events = Parser::new_ext(source, knowledge_options())
        .map(normalize_knowledge)
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

pub fn knowledge_body(source: &str) -> &str {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut lines = source.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return source;
    };
    if first.trim_end_matches([' ', '\t', '\r', '\n']) != "---" {
        return source;
    }

    let mut offset = first.len();
    for line in lines {
        offset += line.len();
        if line.trim_end_matches([' ', '\t', '\r', '\n']) == "---" {
            return source[offset..].trim_matches(['\r', '\n']);
        }
    }
    source
}

fn normalize_knowledge(event: Event<'_>) -> Event<'_> {
    match normalize(event, false) {
        Event::Start(Tag::Link {
            link_type: link_type @ LinkType::WikiLink { .. },
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: format!("/articles/{dest_url}").into(),
            title,
            id,
        }),
        event => event,
    }
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

fn knowledge_options() -> Options {
    options() | Options::ENABLE_GFM | Options::ENABLE_MATH | Options::ENABLE_WIKILINKS
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
