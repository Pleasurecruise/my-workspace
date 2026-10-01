pub mod knowledge;
mod providers;
pub mod publication;

pub use md_dialect::{ArticleMetadata, article_ids, article_urls};

use linkify::{LinkFinder, LinkKind};
use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd, html};

/// UTF-16 source ranges for syntax displayed as editable source blocks in Milkdown.
#[derive(Debug, serde::Serialize)]
pub struct MarkdownSpan {
    start: usize,
    end: usize,
}

pub fn source_spans(source: &str) -> Vec<MarkdownSpan> {
    let options = knowledge::options() | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    let mut protected = 0;
    let mut ranges: Vec<_> = Parser::new_ext(source, options)
        .into_offset_iter()
        .filter_map(|(event, range)| {
            match &event {
                Event::Start(Tag::CodeBlock(_) | Tag::Link { .. } | Tag::Image { .. }) => {
                    protected += 1
                }
                Event::End(TagEnd::CodeBlock | TagEnd::Link | TagEnd::Image) => protected -= 1,
                _ => {}
            }
            let shortcode = if let Event::Text(text) = &event
                && protected == 0
            {
                md_dialect::render_emojis([Event::Text(text.clone())])
                    .iter()
                    .any(|event| matches!(event, Event::InlineHtml(_)))
            } else {
                false
            };
            (shortcode
                || matches!(
                    event,
                    Event::InlineMath(_)
                        | Event::DisplayMath(_)
                        | Event::Html(_)
                        | Event::InlineHtml(_)
                        | Event::Start(Tag::CodeBlock(_) | Tag::MetadataBlock(_))
                        | Event::Start(Tag::Link {
                            link_type: LinkType::WikiLink { .. },
                            ..
                        })
                ))
            .then_some(range)
        })
        .collect();
    ranges.sort_by_key(|range| range.start);
    let mut byte_offset = 0;
    let mut utf16_offset = 0;
    ranges
        .into_iter()
        .map(|range| {
            utf16_offset += source[byte_offset..range.start].encode_utf16().count();
            byte_offset = range.start;
            MarkdownSpan {
                start: utf16_offset,
                end: utf16_offset + source[range].encode_utf16().count(),
            }
        })
        .collect()
}

/// Retain document-scoped references when compiling an editable block on its own.
pub fn fragment(source: &str, context: &str) -> String {
    let parser = Parser::new_ext(context, knowledge::options());
    let mut definitions: Vec<_> = parser
        .reference_definitions()
        .iter()
        .map(|(_, definition)| definition.span.clone())
        .collect();
    let footnotes: std::collections::HashMap<_, _> = parser
        .into_offset_iter()
        .filter_map(|(event, range)| {
            if let Event::Start(Tag::FootnoteDefinition(label)) = event {
                Some((label.to_string(), range))
            } else {
                None
            }
        })
        .collect();
    definitions.sort_by_key(|range| range.start);
    let mut fragment = source.to_owned();
    for range in definitions {
        fragment.push_str("\n\n");
        fragment.push_str(&context[range]);
    }
    let mut included = std::collections::HashSet::new();
    let mut pending = vec![source];
    while let Some(part) = pending.pop() {
        let document = format!("{part}\n\n{context}");
        for (event, range) in Parser::new_ext(&document, knowledge::options()).into_offset_iter() {
            if range.start >= part.len() {
                break;
            }
            if let Event::FootnoteReference(label) = event
                && let Some(range) = footnotes.get(label.as_ref())
                && included.insert(label.to_string())
            {
                let definition = &context[range.clone()];
                fragment.push_str("\n\n");
                fragment.push_str(definition);
                pending.push(definition);
            }
        }
    }
    fragment
}

/// Compare parsed content before allowing a rich editor to normalize Markdown syntax.
/// Raw HTML and custom fence bodies remain part of the comparison.
pub fn equivalent(source: &str, candidate: &str) -> bool {
    let source = source.replace("\r\n", "\n");
    let candidate = candidate.replace("\r\n", "\n");
    // Compare formatting on content, not the nesting order chosen by a serializer.
    // Milkdown can split a link around bold text and put the separating space outside it.
    #[derive(PartialEq)]
    enum Content<'a> {
        Text(String, Vec<Tag<'a>>),
        Event(Event<'a>, Vec<Tag<'a>>),
    }
    let contents = |text| {
        let mut output = Vec::new();
        let mut active: Vec<(Tag<'_>, bool)> = Vec::new();
        for event in Parser::new_ext(text, knowledge::options()) {
            match event {
                Event::Start(tag @ (Tag::Emphasis | Tag::Strong | Tag::Strikethrough)) => {
                    active.push((tag, false));
                }
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) => {
                    let wiki = matches!(link_type, LinkType::WikiLink { .. });
                    active.push((
                        Tag::Link {
                            link_type: if wiki { link_type } else { LinkType::Inline },
                            dest_url,
                            title,
                            id: if wiki { id } else { "".into() },
                        },
                        false,
                    ));
                }
                Event::End(
                    TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link,
                ) => {
                    if let Some((tag @ Tag::Link { .. }, false)) = active.pop() {
                        // Empty links still carry a destination which must not disappear.
                        output.push(Content::Event(Event::Start(tag), Vec::new()));
                    }
                }
                event => {
                    let mut marks: Vec<_> = active.iter().map(|(tag, _)| tag.clone()).collect();
                    marks.sort_by_key(Tag::to_end);
                    if let Event::Text(value) = event {
                        for part in value.split_inclusive(char::is_whitespace) {
                            let content = part.trim_end_matches(char::is_whitespace);
                            for value in [content, &part[content.len()..]] {
                                if value.is_empty() {
                                    continue;
                                }
                                let formatting = if value == content {
                                    marks.clone()
                                } else {
                                    Vec::new()
                                };
                                if !formatting.is_empty() {
                                    for (_, seen) in &mut active {
                                        *seen = true;
                                    }
                                }
                                if let Some(Content::Text(previous, previous_marks)) =
                                    output.last_mut()
                                    && *previous_marks == formatting
                                {
                                    previous.push_str(value);
                                } else {
                                    output.push(Content::Text(value.to_owned(), formatting));
                                }
                            }
                        }
                    } else {
                        for (_, seen) in &mut active {
                            *seen = true;
                        }
                        let event = match event {
                            Event::Start(Tag::Image {
                                dest_url, title, ..
                            }) => Event::Start(Tag::Image {
                                link_type: LinkType::Inline,
                                dest_url,
                                title,
                                id: "".into(),
                            }),
                            event => event,
                        };
                        output.push(Content::Event(event, marks));
                    }
                }
            }
        }
        output
    };
    contents(&source) == contents(&candidate)
}

pub fn render(source: &str) -> String {
    let parser = Parser::new_ext(source, options()).map(|event| normalize(event, false));
    let mut output = String::new();
    html::push_html(&mut output, md_dialect::render_emojis(parser).into_iter());
    output
}

pub fn render_memo(source: &str) -> String {
    let mut list_depth = 0;
    let mut protected_depth = 0;
    let mut separated = false;
    let mut output = String::new();
    let mut cursor = 0;
    for (event, range) in Parser::new_ext(source, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth -= 1,
            Event::Start(Tag::Item) => separated = false,
            Event::Start(
                Tag::BlockQuote(_)
                | Tag::Link { .. }
                | Tag::Image { .. }
                | Tag::Strong
                | Tag::Emphasis
                | Tag::Strikethrough,
            ) => {
                protected_depth += 1;
            }
            Event::End(
                TagEnd::BlockQuote(_)
                | TagEnd::Link
                | TagEnd::Image
                | TagEnd::Strong
                | TagEnd::Emphasis
                | TagEnd::Strikethrough,
            ) => {
                protected_depth -= 1;
            }
            Event::SoftBreak
                if list_depth > 0
                    && protected_depth == 0
                    && !separated
                    && source[range.end..]
                        .chars()
                        .next()
                        .is_some_and(|character| !character.is_whitespace()) =>
            {
                output.push_str(&source[cursor..range.end]);
                output.push('\n');
                cursor = range.end;
                separated = true;
            }
            _ => {}
        }
    }
    let source = if cursor == 0 {
        std::borrow::Cow::Borrowed(source)
    } else {
        output.push_str(&source[cursor..]);
        std::borrow::Cow::Owned(output)
    };
    let mut protected_depth = 0;
    let mut events = Vec::new();
    for event in Parser::new_ext(&source, options()).map(|event| normalize(event, true)) {
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
    html::push_html(&mut output, md_dialect::render_emojis(events).into_iter());
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

fn normalize<'a>(event: Event<'a>, hard_breaks: bool) -> Event<'a> {
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
        Event::SoftBreak if hard_breaks => Event::HardBreak,
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

struct CodeBlock {
    language: String,
    source: String,
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

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/compile.rs"]
mod compile_tests;
