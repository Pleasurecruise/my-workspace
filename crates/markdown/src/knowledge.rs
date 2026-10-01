use pulldown_cmark::{CowStr, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd, html};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::{CodeBlock, code_language};
use md_dialect::{ArticleMetadata, EmbedError, embed};

const CJK_PER_MINUTE: usize = 350;
const WORDS_PER_MINUTE: usize = 200;

#[derive(Debug, Serialize)]
pub struct Compiled {
    pub html: String,
    pub toc: Vec<TocEntry>,
    pub excerpt: String,
    pub stats: ReadingStats,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingStats {
    pub word_count: usize,
    pub reading_minutes: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct TocEntry {
    pub id: String,
    pub text: String,
    pub depth: u8,
}

pub async fn compile(
    source: &str,
    articles: HashMap<String, ArticleMetadata>,
) -> Result<Compiled, EmbedError> {
    let data = crate::providers::read(body(source), articles).await?;
    compile_with(source, &data)
}

/// Compiles Knowledge Markdown without resolving or interpreting content embeds.
///
/// This keeps the article readable when an optional embed provider is unavailable.
pub fn fallback(source: &str) -> Compiled {
    let source = md_dialect::normalize_embed_examples(body(source));
    let parsed: Vec<_> = Parser::new_ext(&source, options()).collect();
    let stats = reading_stats(&parsed);
    let events = parsed.into_iter().map(normalize).collect();
    assemble(events, stats)
}

pub(crate) fn compile_with(source: &str, data: &embed::Data) -> Result<Compiled, EmbedError> {
    let source = md_dialect::normalize_embed_examples(body(source));
    let parsed: Vec<_> = Parser::new_ext(&source, options()).collect();
    let stats = reading_stats(&parsed);
    let events = render_embeds(parsed, data)?;
    Ok(assemble(events, stats))
}

fn assemble(events: Vec<Event<'_>>, stats: ReadingStats) -> Compiled {
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
    let mut used_ids = HashSet::new();
    let toc: Vec<TocEntry> = headings
        .into_iter()
        .map(|(level, text)| {
            let base = heading_id(&text);
            let count = slugs.entry(base.clone()).or_insert(0);
            *count += 1;
            let mut id = base.clone();
            if *count > 1 {
                id = format!("{base}-{}", *count);
            }
            while !used_ids.insert(id.clone()) {
                *count += 1;
                id = format!("{base}-{}", *count);
            }
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
    html::push_html(&mut html, md_dialect::render_emojis(events).into_iter());
    embed::add_styles(&mut html);
    let excerpt = excerpt
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(240)
        .collect();
    Compiled {
        html,
        toc,
        excerpt,
        stats,
    }
}

fn render_embeds<'a>(
    parsed: Vec<Event<'a>>,
    data: &embed::Data,
) -> Result<Vec<Event<'a>>, EmbedError> {
    let mut events = Vec::new();
    let mut embed_block: Option<CodeBlock> = None;

    for event in parsed {
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
                let rendered = embed::render(
                    &block.language,
                    block.source.strip_suffix('\n').unwrap_or(&block.source),
                    data,
                )?
                .expect("embed namespace is recognized before buffering");
                events.push(Event::Html(CowStr::Boxed(rendered.into_boxed_str())));
            }
            event => events.push(normalize(event)),
        }
    }

    Ok(events)
}

pub fn body(source: &str) -> &str {
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

fn normalize(event: Event<'_>) -> Event<'_> {
    match crate::normalize(event, false) {
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

pub(crate) fn options() -> Options {
    crate::options() | Options::ENABLE_GFM | Options::ENABLE_MATH | Options::ENABLE_WIKILINKS
}

fn reading_stats(events: &[Event<'_>]) -> ReadingStats {
    let mut text = String::new();
    let mut excluded = 0;
    for event in events {
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::Image { .. }) => {
                excluded += 1;
                text.push(' ');
            }
            Event::End(TagEnd::CodeBlock | TagEnd::Image) => {
                excluded -= 1;
                text.push(' ');
            }
            Event::Text(value) if excluded == 0 => text.push_str(value),
            Event::SoftBreak
            | Event::HardBreak
            | Event::Code(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::Item
                | TagEnd::TableCell
                | TagEnd::TableRow,
            ) => text.push(' '),
            _ => {}
        }
    }
    let mut finder = linkify::LinkFinder::new();
    finder.kinds(&[linkify::LinkKind::Url]);
    let mut prose = String::new();
    let mut cursor = 0;
    for link in finder.links(&text) {
        prose.push_str(&text[cursor..link.start()]);
        prose.push(' ');
        cursor = link.end();
    }
    prose.push_str(&text[cursor..]);
    static WORDS: OnceLock<(regex::Regex, regex::Regex)> = OnceLock::new();
    let (cjk_pattern, word_pattern) = WORDS.get_or_init(|| {
        (
            regex::Regex::new(r"[\p{Han}\p{Hiragana}\p{Katakana}\p{Hangul}]")
                .expect("valid CJK pattern"),
            regex::Regex::new(
                r"[\p{L}\p{N}][\p{L}\p{N}\p{M}]*(?:['’-][\p{L}\p{N}][\p{L}\p{N}\p{M}]*)*",
            )
            .expect("valid word pattern"),
        )
    });
    let cjk_count = cjk_pattern.find_iter(&prose).count();
    let separated = cjk_pattern.replace_all(&prose, " ");
    let word_count = word_pattern.find_iter(&separated).count();
    let weighted_count = cjk_count * WORDS_PER_MINUTE + word_count * CJK_PER_MINUTE;
    let minute_capacity = CJK_PER_MINUTE * WORDS_PER_MINUTE;
    let reading_minutes = weighted_count.div_ceil(minute_capacity).max(1);
    ReadingStats {
        word_count: cjk_count + word_count,
        reading_minutes,
    }
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
