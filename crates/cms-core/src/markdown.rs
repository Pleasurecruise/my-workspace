use linkify::{LinkFinder, LinkKind};
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd, html};

pub fn render(source: &str) -> String {
    let parser = Parser::new_ext(source, options()).map(|event| normalize(event, false));
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
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

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

#[cfg(test)]
#[path = "../tests/unit/markdown.rs"]
mod tests;
