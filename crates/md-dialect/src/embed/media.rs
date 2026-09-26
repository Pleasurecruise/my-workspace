use super::{EmbedError, align, escape_html, is_web_url, reject_unknown, required};
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use std::collections::HashMap;
use url::{ParseError, Url};

const PATH_ENCODING: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'{')
    .add(b'}');

pub(super) struct Source {
    pub(super) url: String,
    pub(super) local: bool,
}

pub(super) struct Media<'a> {
    kind: &'a str,
    pub(super) src: Source,
    pub(super) poster: Option<Source>,
    title: &'a str,
    caption: Option<&'a str>,
    align: &'a str,
}

pub(super) fn parse<'a>(mut fields: HashMap<&str, &'a str>) -> Result<Media<'a>, EmbedError> {
    reject_unknown(
        "embed:media",
        &fields,
        &["type", "src", "poster", "title", "caption", "align"],
    )?;
    let kind = required(&mut fields, "media", "type")?;
    if !matches!(kind, "audio" | "video") {
        return Err(EmbedError::InvalidMedia {
            field: "type",
            message: "expected `audio` or `video`",
        });
    }
    let src = parse_source(required(&mut fields, "media", "src")?, "src")?;
    let poster = fields
        .remove("poster")
        .map(|value| parse_source(value, "poster"))
        .transpose()?;
    if kind == "audio" && poster.is_some() {
        return Err(EmbedError::InvalidMedia {
            field: "poster",
            message: "only video supports a poster",
        });
    }
    let align = align(&mut fields)?;
    Ok(Media {
        kind,
        src,
        poster,
        align,
        title: fields.remove("title").unwrap_or(if kind == "audio" {
            "Audio player"
        } else {
            "Video player"
        }),
        caption: fields.remove("caption"),
    })
}

fn parse_source(value: &str, field: &'static str) -> Result<Source, EmbedError> {
    let invalid = || EmbedError::InvalidMedia {
        field,
        message: "expected an HTTP(S) URL without credentials or a document-relative asset path",
    };
    if value.is_empty() || value.chars().any(char::is_control) || value.contains('\\') {
        return Err(invalid());
    }
    match Url::parse(value) {
        Ok(mut url) => {
            if !is_web_url(&url) {
                return Err(invalid());
            }
            // GitHub's file viewer is HTML. Keep the full ref/path suffix when requesting bytes.
            if url.host_str() == Some("github.com") && url.port().is_none() {
                let segments: Vec<_> = url.path().split('/').collect();
                if segments.len() >= 6
                    && segments[3] == "blob"
                    && segments[1..3].iter().all(|segment| !segment.is_empty())
                    && segments[4..].iter().all(|segment| !segment.is_empty())
                {
                    let path = format!(
                        "/{}/{}/{}",
                        segments[1],
                        segments[2],
                        segments[4..].join("/")
                    );
                    url.set_host(Some("raw.githubusercontent.com"))
                        .map_err(|_| invalid())?;
                    url.set_path(&path);
                    url.set_query(None);
                }
            }
            Ok(Source {
                url: url.to_string(),
                local: false,
            })
        }
        Err(ParseError::RelativeUrlWithoutBase) => {
            if value.starts_with('/') || value.starts_with('~') || value.contains(['?', '#']) {
                return Err(invalid());
            }
            Ok(Source {
                url: utf8_percent_encode(value, PATH_ENCODING).to_string(),
                local: true,
            })
        }
        Err(_) => Err(invalid()),
    }
}

pub(super) fn render(fields: HashMap<&str, &str>) -> Result<String, EmbedError> {
    let media = parse(fields)?;
    let preview = media.kind == "video" && media.poster.is_none();
    // An opening-time fragment requests a decoded preview frame in WebKit as well as metadata.
    // Preserve authored fragments, including an explicit playback start time.
    let src = if preview && !media.src.url.contains('#') {
        format!("{}#t=0.001", media.src.url)
    } else {
        media.src.url.clone()
    };
    let preload = if preview { "metadata" } else { "none" };
    let poster = media
        .poster
        .map(|source| format!(" poster=\"{}\"", escape_html(&source.url)))
        .unwrap_or_default();
    let caption = media
        .caption
        .map(|text| format!("<figcaption>{}</figcaption>", escape_html(text)))
        .unwrap_or_default();
    let inline = if media.kind == "video" {
        " playsinline"
    } else {
        ""
    };
    Ok(format!(
        concat!(
            "<figure class=\"content-embed content-embed-media content-embed-{align}\">",
            "<{kind} controls preload=\"{preload}\"{inline}{poster} src=\"{src}\" aria-label=\"{title}\">",
            "Your browser does not support this media. <a href=\"{href}\" target=\"_blank\" rel=\"noopener noreferrer\">Open media</a>",
            "</{kind}>{caption}</figure>\n"
        ),
        inline = inline,
        poster = poster,
        caption = caption,
        kind = media.kind,
        align = media.align,
        src = escape_html(&src),
        href = escape_html(&media.src.url),
        preload = preload,
        title = escape_html(media.title),
    ))
}
