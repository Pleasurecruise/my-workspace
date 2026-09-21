//! Image shortcodes backed by the bundled, owner-selected catalog.
use std::{collections::HashMap, sync::OnceLock};

use pulldown_cmark::{Event, Tag, TagEnd};
use regex::Regex;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pack {
    key: String,
    name: String,
    display: Display,
    items: Vec<Item>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Display {
    Emoji,
    Sticker,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    name: String,
    value: String,
}

fn catalog(source: &str) -> Result<HashMap<String, String>, String> {
    let packs: Vec<Pack> = serde_json::from_str(source).map_err(|error| error.to_string())?;
    let mut images = HashMap::new();
    for pack in packs {
        if pack.key.is_empty()
            || !pack.key.bytes().all(|c| c.is_ascii_alphanumeric())
            || pack.name.trim().is_empty()
        {
            return Err("Invalid emoji pack key or name".into());
        }
        for item in pack.items {
            if item.name.is_empty() || item.name.chars().any(|c| c == ':' || c.is_whitespace()) {
                return Err("Invalid emoji name".into());
            }
            let url = url::Url::parse(&item.value).map_err(|error| error.to_string())?;
            if url.scheme() != "https"
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err("Emoji images require an HTTPS URL without credentials".into());
            }
            let style = match pack.display {
                Display::Emoji => {
                    "display:inline-block;width:2rem;height:2rem;max-width:100%;margin:0 .125rem;object-fit:contain;vertical-align:middle"
                }
                Display::Sticker => {
                    "display:inline-block;width:auto;height:auto;max-width:min(6rem,100%);max-height:6rem;margin:0 .25rem;object-fit:contain;vertical-align:middle"
                }
            };
            let name = crate::embed::escape_html(&item.name);
            let src = crate::embed::escape_html(&item.value);
            let html = format!(
                r#"<img class="markdown-emoji" src="{src}" alt="[{name}]" title="{name}" loading="lazy" decoding="async" referrerpolicy="no-referrer" style="{style}" />"#
            );
            if images
                .insert(format!(":{}_{}:", pack.key, item.name), html)
                .is_some()
            {
                return Err("Duplicate emoji shortcode".into());
            }
        }
    }
    Ok(images)
}

/// Transform prose only; keep code, links, image alt text, math and generated HTML intact.
pub fn render_emojis<'a>(events: impl IntoIterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    static IMAGES: OnceLock<HashMap<String, String>> = OnceLock::new();
    let images = IMAGES.get_or_init(|| {
        catalog(include_str!("emoji-packs.json")).expect("valid bundled emoji catalog")
    });
    transform(events, images)
}

fn transform<'a>(
    events: impl IntoIterator<Item = Event<'a>>,
    images: &HashMap<String, String>,
) -> Vec<Event<'a>> {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = PATTERN
        .get_or_init(|| Regex::new(r":[a-zA-Z0-9]+_[^:\s]+:").expect("valid shortcode pattern"));
    let mut output = Vec::new();
    let mut protected = 0;
    for event in events {
        match &event {
            Event::Start(Tag::CodeBlock(_) | Tag::Link { .. } | Tag::Image { .. }) => {
                protected += 1
            }
            Event::End(TagEnd::CodeBlock | TagEnd::Link | TagEnd::Image) => protected -= 1,
            _ => {}
        }
        if let Event::Text(text) = &event
            && protected == 0
            && !images.is_empty()
        {
            let mut cursor = 0;
            for matched in pattern.find_iter(text) {
                let Some(html) = images.get(matched.as_str()) else {
                    continue;
                };
                if cursor < matched.start() {
                    output.push(Event::Text(text[cursor..matched.start()].to_owned().into()));
                }
                output.push(Event::InlineHtml(html.clone().into()));
                cursor = matched.end();
            }
            if cursor > 0 {
                if cursor < text.len() {
                    output.push(Event::Text(text[cursor..].to_owned().into()));
                }
                continue;
            }
        }
        output.push(event);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Parser, html};

    #[test]
    fn validate_catalog() {
        let images = catalog(include_str!("emoji-packs.json")).unwrap();
        assert!(images[":suzume_思考:"].contains("https://szm.de5.net/"));
        assert!(images[":suzume_期待:"].contains("max-height:6rem"));
        assert!(!images.contains_key(":daimao2_02:"));
        assert!(!images.contains_key(":denghuoju8_03:"));
    }

    #[test]
    fn shortcodes_only_replace_prose() {
        let images = catalog(r#"[{"key":"test","name":"Test","display":"sticker","items":[{"name":"开心","value":"https://example.com/test.gif?a=1&b=2"}]}]"#).unwrap();
        let source = ":test_开心: **:test_开心:** :missing_x: `:test_开心:`\n\n```\n:test_开心:\n```\n\n[:test_开心:](https://example.com/:test_开心:) ![:test_开心:](https://example.com/img.png)";
        let mut result = String::new();
        html::push_html(
            &mut result,
            transform(Parser::new(source), &images).into_iter(),
        );
        assert_eq!(result.matches("class=\"markdown-emoji\"").count(), 2);
        assert!(result.contains("a=1&amp;b=2"));
        assert!(result.contains("max-height:6rem"));
        assert!(result.contains(":missing_x:"));
        assert!(result.contains("<code>:test_开心:</code>"));
        assert!(result.contains("alt=\":test_开心:\""));
        assert!(result.contains(">:test_开心:</a>"));
    }

    #[test]
    fn catalog_rejects_unsafe_urls_and_duplicates() {
        for url in [
            "javascript:alert(1)",
            "http://example.com/x",
            "https://user:secret@example.com/x",
        ] {
            let source = format!(
                r#"[{{"key":"test","name":"Test","display":"emoji","items":[{{"name":"x","value":"{url}"}}]}}]"#
            );
            assert!(catalog(&source).is_err());
        }
        assert!(catalog(include_str!("emoji-packs.json")).is_ok());
        let pack = r#"{"key":"test","name":"Test","display":"emoji","items":[{"name":"x","value":"https://example.com/x"}]}"#;
        assert!(catalog(&format!("[{pack},{pack}]")).is_err());
    }
}
