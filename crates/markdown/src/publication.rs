use pulldown_cmark::{CowStr, Event, Parser, Tag, TagEnd, html};
use std::collections::HashMap;
use std::sync::OnceLock;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

use crate::{CodeBlock, code_language};
use md_dialect::{EmbedError, embed};

const CODE_THEME: &str = "InspiredGitHub";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not highlight code block: {0}")]
    Highlight(#[from] syntect::Error),
    #[error("could not render Mermaid diagram: {0}")]
    Mermaid(#[from] mermaid_svg::RenderError),
    #[error("could not compile content embed: {0}")]
    Embed(#[from] EmbedError),
}

pub async fn render(source: &str) -> Result<String, Error> {
    let data = crate::providers::read(source, HashMap::new()).await?;
    render_with(source, &data)
}

pub(crate) fn render_with(source: &str, data: &embed::Data) -> Result<String, Error> {
    let source = md_dialect::normalize_embed_examples(source);
    let mut events = Vec::new();
    let mut code_block: Option<CodeBlock> = None;

    for event in Parser::new_ext(&source, crate::options()) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = code_language(&kind);
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
                let rendered = if let Some(rendered) = embed::render(
                    &block.language,
                    block.source.strip_suffix('\n').unwrap_or(&block.source),
                    data,
                )? {
                    rendered
                } else if block.language == "mermaid" {
                    let svg = mermaid_svg::render(&block.source)?;
                    format!("<figure class=\"mermaid-diagram\">{svg}</figure>\n")
                } else {
                    highlight_code(&block.source, &block.language)?
                };
                events.push(Event::Html(CowStr::Boxed(rendered.into_boxed_str())));
            }
            event if code_block.is_none() => events.push(crate::normalize(event, false)),
            _ => {}
        }
    }

    let mut output = String::new();
    html::push_html(&mut output, md_dialect::render_emojis(events).into_iter());
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
