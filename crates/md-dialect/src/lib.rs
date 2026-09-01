use pulldown_cmark::{
    CodeBlockKind, CowStr, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd, html,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::OnceLock;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

mod embed;

pub use embed::EmbedError;

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
    #[error("could not compile content embed: {0}")]
    Embed(#[from] EmbedError),
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

pub fn render_publication(source: &str) -> Result<String, PublicationError> {
    render_publication_with(source, &embed::Data::default())
}

pub async fn render_publication_enriched(source: &str) -> Result<String, PublicationError> {
    let data = embed::load(source).await?;
    render_publication_with(source, &data)
}

fn render_publication_with(source: &str, data: &embed::Data) -> Result<String, PublicationError> {
    let mut events = Vec::new();
    let mut code_block: Option<CodeBlock> = None;

    for event in Parser::new_ext(source, common_options()) {
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
                let rendered =
                    if let Some(rendered) = embed::render(&block.language, &block.source, data)? {
                        rendered
                    } else if block.language == "mermaid" {
                        let svg = mermaid_svg::render(&block.source)?;
                        format!("<figure class=\"mermaid-diagram\">{svg}</figure>\n")
                    } else {
                        highlight_code(&block.source, &block.language)?
                    };
                events.push(Event::Html(CowStr::Boxed(rendered.into_boxed_str())));
            }
            event if code_block.is_none() => events.push(normalize(event)),
            _ => {}
        }
    }

    let mut output = String::new();
    html::push_html(&mut output, events.into_iter());
    embed::add_styles(&mut output);
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

pub fn compile_knowledge(source: &str) -> Result<CompiledKnowledge, EmbedError> {
    compile_knowledge_with(source, &embed::Data::default())
}

pub async fn compile_knowledge_enriched(source: &str) -> Result<CompiledKnowledge, EmbedError> {
    let data = embed::load(source).await?;
    compile_knowledge_with(source, &data)
}

/// Compiles Knowledge Markdown without resolving or interpreting content embeds.
///
/// This keeps the article readable when an optional embed provider is unavailable.
pub fn compile_knowledge_plain(source: &str) -> CompiledKnowledge {
    let source = knowledge_body(source);
    let events = Parser::new_ext(source, knowledge_options())
        .map(normalize_knowledge)
        .collect();
    compile_knowledge_events(events)
}

fn compile_knowledge_with(
    source: &str,
    data: &embed::Data,
) -> Result<CompiledKnowledge, EmbedError> {
    let source = knowledge_body(source);
    let events = knowledge_events(source, data)?;
    Ok(compile_knowledge_events(events))
}

fn compile_knowledge_events(events: Vec<Event<'_>>) -> CompiledKnowledge {
    let mut heading_text: Option<String> = None;
    let mut headings: Vec<(HeadingLevel, String)> = Vec::new();
    let mut excerpt = String::new();

    for event in &events {
        match event {
            Event::Start(Tag::Heading { .. }) => heading_text = Some(String::new()),
            Event::Text(text)
            | Event::Code(text)
            | Event::InlineMath(text)
            | Event::DisplayMath(text) => {
                if let Some(current_heading) = &mut heading_text {
                    current_heading.push_str(text);
                }
                excerpt.push_str(text);
            }
            Event::End(TagEnd::Heading(level)) => {
                if let Some(text) = heading_text.take() {
                    headings.push((*level, text));
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
    let events = events.into_iter().map(|event| match event {
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
    html::push_html(&mut html, events);
    embed::add_styles(&mut html);
    let excerpt = excerpt
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(240)
        .collect();
    CompiledKnowledge { html, toc, excerpt }
}

fn knowledge_events<'a>(source: &'a str, data: &embed::Data) -> Result<Vec<Event<'a>>, EmbedError> {
    let mut events = Vec::new();
    let mut embed_block: Option<CodeBlock> = None;

    for event in Parser::new_ext(source, knowledge_options()) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = code_language(&kind);
                if language.starts_with("embed:") {
                    embed_block = Some(CodeBlock {
                        language,
                        source: String::new(),
                    });
                } else {
                    events.push(Event::Start(Tag::CodeBlock(kind)));
                }
            }
            Event::Text(text) if embed_block.is_some() => {
                if let Some(block) = &mut embed_block {
                    block.source.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) if embed_block.is_some() => {
                let block = embed_block.take().expect("embed block start precedes end");
                let rendered = embed::render(&block.language, &block.source, data)?
                    .expect("embed namespace is recognized before buffering");
                events.push(Event::Html(CowStr::Boxed(rendered.into_boxed_str())));
            }
            event => events.push(normalize_knowledge(event)),
        }
    }

    Ok(events)
}

fn code_language(kind: &CodeBlockKind<'_>) -> String {
    match kind {
        CodeBlockKind::Indented => String::new(),
        CodeBlockKind::Fenced(info) => info
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase(),
    }
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
    match normalize(event) {
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

fn normalize(event: Event<'_>) -> Event<'_> {
    match event {
        Event::Html(text) | Event::InlineHtml(text) => Event::Text(text),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) if !safe_destination(&dest_url, true) => Event::Start(Tag::Link {
            link_type,
            dest_url: "#".into(),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) if !safe_destination(&dest_url, false) => Event::Start(Tag::Image {
            link_type,
            dest_url: "".into(),
            title,
            id,
        }),
        event => event,
    }
}

fn safe_destination(destination: &str, allow_mailto: bool) -> bool {
    let Some(colon) = destination.find(':') else {
        return true;
    };
    if destination
        .find(['/', '?', '#'])
        .is_some_and(|delimiter| delimiter < colon)
    {
        return true;
    }

    destination[..colon].eq_ignore_ascii_case("http")
        || destination[..colon].eq_ignore_ascii_case("https")
        || (allow_mailto && destination[..colon].eq_ignore_ascii_case("mailto"))
}

fn common_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

fn knowledge_options() -> Options {
    common_options() | Options::ENABLE_GFM | Options::ENABLE_MATH | Options::ENABLE_WIKILINKS
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
mod tests;
