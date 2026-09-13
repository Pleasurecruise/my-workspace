use super::*;

#[test]
fn renders_repository_data() {
    let mut data = Data::default();
    data.repositories.insert(
        "canmi21/seam".to_owned(),
        quotes::github::RepositorySnapshot {
            full_name: "canmi21/seam".to_owned(),
            description: "A typed seam".to_owned(),
            owner_avatar_url: "https://avatars.example/canmi21".to_owned(),
            language: "Rust".to_owned(),
            stars: 21,
            forks: 3,
            open_issues: 2,
            default_branch: "main".to_owned(),
            updated_at: "2026-09-01T00:00:00Z".to_owned(),
            url: "https://github.com/canmi21/seam".to_owned(),
        },
    );
    let html = render(GITHUB, "repo: canmi21/seam\nalign: left", &data)
        .expect("valid repository embed")
        .expect("registered embed");
    assert!(html.contains("content-embed-left"));
    assert!(html.contains(">21</span>"));
    assert!(html.contains(">2</span>"));
}

#[test]
fn renders_stock_data_as_a_smooth_month_chart() {
    let mut data = Data::default();
    data.stocks.insert(
        "AAPL".to_owned(),
        quotes::stocks::StockSeries {
            symbol: "AAPL".to_owned(),
            name: "Apple Inc.".to_owned(),
            currency: "USD".to_owned(),
            exchange: "NMS".to_owned(),
            price: 231.4,
            change: 2.1,
            change_percent: 0.92,
            points: vec![
                quotes::stocks::StockPoint {
                    timestamp: 1,
                    close: 220.0,
                },
                quotes::stocks::StockPoint {
                    timestamp: 2,
                    close: 226.0,
                },
                quotes::stocks::StockPoint {
                    timestamp: 3,
                    close: 231.4,
                },
            ],
        },
    );
    let html = render(STOCK, "code: AAPL\nalign: right", &data)
        .expect("valid stock embed")
        .expect("registered embed");
    assert!(html.contains("content-embed-right content-embed-up"));
    assert!(html.contains("class=\"content-stock-area\""));
    assert!(html.contains(" C"));
    assert!(html.contains("<circle class=\"content-stock-end\""));
}

#[test]
fn renders_link_metadata_as_text_and_keeps_ordinary_links_unhandled() {
    let mut data = Data::default();
    data.links.insert(
        "https://example.com".to_owned(),
        quotes::opengraph::Metadata {
            url: "https://example.com/?a=1&b=2".to_owned(),
            title: "<script>alert(1)</script>".to_owned(),
            description: "A & B".to_owned(),
            site_name: "Example".to_owned(),
            image: None,
        },
    );
    let html = render(LINK, "url: https://example.com\nalign: left", &data)
        .unwrap()
        .unwrap();
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(html.contains("A &amp; B"));
    assert!(html.contains("href=\"https://example.com/?a=1&amp;b=2\""));
    assert!(!html.contains("<img"));
    assert!(
        render("text", "https://example.com", &data)
            .unwrap()
            .is_none()
    );
    assert!(render(LINK, "url: https://example.com\nimage: injected", &data).is_err());
}

#[test]
fn renders_audio_and_video_without_provider_data() {
    let data = Data::default();
    let audio = render(
        MEDIA,
        "type: audio\nsrc: ./media/片段 one.mp3\ntitle: A & B\ncaption: <script>literal</script>",
        &data,
    )
    .unwrap()
    .unwrap();
    assert!(audio.contains("<audio controls preload=\"none\""));
    assert!(audio.contains("src=\"./media/%E7%89%87%E6%AE%B5%20one.mp3\""));
    assert!(audio.contains("aria-label=\"A &amp; B\""));
    assert!(audio.contains("<figcaption>&lt;script&gt;literal&lt;/script&gt;</figcaption>"));
    assert!(!audio.contains("autoplay"));
    let video = render(MEDIA, "type: video\nsrc: https://example.com/watch.mp4?a=1&b=2#t=5\nposter: ../media/cover.jpg\nalign: left", &data).unwrap().unwrap();
    assert!(video.contains("content-embed-left"));
    assert!(
        video
            .contains("<video controls preload=\"none\" playsinline poster=\"../media/cover.jpg\"")
    );
    assert!(video.contains("watch.mp4?a=1&amp;b=2#t=5"));
}

#[test]
fn rejects_invalid_media_fields_and_sources() {
    let data = Data::default();
    for fields in [
        "type: image\nsrc: photo.png",
        "type: audio",
        "src: media.mp4",
        "type: audio\nsrc: a.mp3\nposter: cover.jpg",
        "type: video\nsrc: a.mp4\nautoplay: true",
        "type: video\nsrc: a.mp4\nsrc: b.mp4",
        "type: video\nsrc: a.mp4\nalign: center",
    ] {
        assert!(render(MEDIA, fields, &data).is_err(), "{fields}");
    }
    for src in [
        "javascript:alert(1)",
        "data:audio/mpeg;base64,abcd",
        "file:///tmp/a.mp3",
        "/Users/name/a.mp3",
        "//example.com/a.mp3",
        "C:\\media\\a.mp3",
        "https://user:secret@example.com/a.mp3",
        "~/Music/a.mp3",
        "a.mp3#fragment",
        "https://",
        "https://example.com/\u{0000}a.mp3",
    ] {
        assert!(
            render(MEDIA, &format!("type: audio\nsrc: {src}"), &data).is_err(),
            "{src}"
        );
        assert!(
            render(
                MEDIA,
                &format!("type: video\nsrc: a.mp4\nposter: {src}"),
                &data
            )
            .is_err(),
            "poster: {src}"
        );
    }
}

#[test]
fn collects_only_local_media_assets() {
    let source = "```embed:media\ntype: audio\nsrc: ./audio.mp3\n```\n\n```EMBED:MEDIA\ntype: video\nsrc: https://example.com/video.mp4\nposter: ./cover.jpg\n```\n\n```embed:media\ntype: audio\nsrc: ./audio.mp3\n```\n\n```text\ntype: audio\nsrc: ignored.mp3\n```";
    assert_eq!(
        collect_media_paths(source).unwrap(),
        vec!["./audio.mp3", "./cover.jpg"]
    );
}

#[test]
fn previews_video_frames_without_overriding_posters_or_start_times() {
    let data = Data::default();
    let preview = render(MEDIA, "type: video\nsrc: ./video.mp4", &data)
        .unwrap()
        .unwrap();
    assert!(preview.contains("src=\"./video.mp4#t=0.001\""));
    assert!(preview.contains("controls preload=\"metadata\" playsinline"));
    assert!(preview.contains("href=\"./video.mp4\""));
    assert!(!preview.contains("autoplay"));
    let poster = render(
        MEDIA,
        "type: video\nsrc: ./video.mp4\nposter: ./cover.jpg",
        &data,
    )
    .unwrap()
    .unwrap();
    assert!(poster.contains("src=\"./video.mp4\""));
    assert!(poster.contains("poster=\"./cover.jpg\""));
    assert!(poster.contains("preload=\"none\""));
    let timed = render(
        MEDIA,
        "type: video\nsrc: https://example.com/demo.mp4#t=5,10",
        &data,
    )
    .unwrap()
    .unwrap();
    assert!(timed.contains("src=\"https://example.com/demo.mp4#t=5,10\""));
    assert!(!timed.contains("0.001"));
}

#[test]
fn github_media_file_pages_resolve_to_bytes_without_rewriting_other_hosts() {
    let data = Default::default();
    for (source, expected) in [
        (
            "https://github.com/Pleasurecruise/pleasure1234/blob/main/public/cat.mp3",
            "https://raw.githubusercontent.com/Pleasurecruise/pleasure1234/main/public/cat.mp3",
        ),
        (
            "https://github.com/a/b/blob/feature/audio/my%20clip.mp3?raw=true#t=5",
            "https://raw.githubusercontent.com/a/b/feature/audio/my%20clip.mp3#t=5",
        ),
        (
            "https://github.com/a/b/releases/download/v1/song.mp3",
            "https://github.com/a/b/releases/download/v1/song.mp3",
        ),
        (
            "https://github.com.example.com/a/b/blob/main/song.mp3",
            "https://github.com.example.com/a/b/blob/main/song.mp3",
        ),
        (
            "https://github.com/a/b/blob/main",
            "https://github.com/a/b/blob/main",
        ),
    ] {
        let html = render(MEDIA, &format!("type: audio\nsrc: {source}"), &data)
            .unwrap()
            .unwrap();
        assert!(html.contains(&format!("src=\"{expected}\"")), "{html}");
    }
}

#[test]
fn article_links_require_index_metadata_even_with_manual_titles() {
    for source in [
        "id: article-123\ntitle: Custom",
        "url: https://example.com/story\ntitle: Custom\ndescription: Summary",
    ] {
        let html = render(ARTICLE, source, &Data::default()).unwrap().unwrap();
        assert!(html.contains("aria-disabled=\"true\""));
        assert!(!html.contains("href="));
        assert!(!html.contains("Custom"));
    }
}

#[test]
fn article_links_reject_ambiguous_and_unsafe_targets() {
    for fields in [
        "id: one\nurl: https://example.com",
        "id: ../other",
        "id: one?secret",
        "id: one%2Ftwo",
        "id: .",
        "url: javascript:alert(1)",
        "url: //example.com",
        "url: https://user:password@example.com",
        "url: /articles/one",
        "id: one\nextra: ignored",
        "id: one\nid: two",
        "description: Missing target",
    ] {
        assert!(
            render(
                "embed:article",
                &format!("title: Story\n{fields}"),
                &Data::default()
            )
            .is_err(),
            "{fields}"
        );
    }
}

#[test]
fn article_shortcuts_read_metadata_and_allow_independent_overrides() {
    let mut data = Data::default();
    data.articles.insert(
        "article-123".to_owned(),
        ArticleMetadata {
            href: None,
            title: "Automatic <title>".to_owned(),
            description: "Automatic & summary".to_owned(),
        },
    );
    let html = render(ARTICLE, "id: article-123", &data).unwrap().unwrap();
    assert!(html.contains("Automatic &lt;title&gt;"));
    assert!(html.contains("Automatic &amp; summary"));
    let html = render(ARTICLE, "id: article-123\ntitle: Custom", &data)
        .unwrap()
        .unwrap();
    assert!(html.contains("<strong>Custom</strong>"));
    assert!(html.contains("Automatic &amp; summary"));
    let html = render(
        ARTICLE,
        "id: article-123\ndescription: Custom summary",
        &data,
    )
    .unwrap()
    .unwrap();
    assert!(html.contains("Automatic &lt;title&gt;"));
    assert!(html.contains("Custom summary"));
    assert_eq!(
        article_ids(
            "```embed:article\nid: article-123\n```\n\n```embed:article\nid: article-123\n```"
        )
        .unwrap(),
        ["article-123"]
    );
}

#[test]
fn unresolved_articles_are_not_clickable() {
    for fields in ["id: article-123", "url: https://example.com/story"] {
        let html = render(ARTICLE, fields, &Data::default()).unwrap().unwrap();
        assert!(html.contains("<div class="));
        assert!(html.contains("Article not found in the article list"));
    }
}

#[test]
fn article_url_lists_render_metadata_cards_and_preserve_order() {
    let url = "https://knowledge.you-find.me/articles/real-slug";
    let mut data = Data::default();
    data.articles.insert(
        url.to_owned(),
        ArticleMetadata {
            href: Some("/articles/article-id".to_owned()),
            title: "Real <title>".to_owned(),
            description: "Real & description".to_owned(),
        },
    );
    let source = format!("{url}\n- https://example.com/unavailable\n{url}");
    let html = render(ARTICLE, &source, &data).unwrap().unwrap();
    assert_eq!(html.matches("<li>").count(), 3);
    assert_eq!(html.matches("Real &lt;title&gt;").count(), 2);
    assert!(html.contains("Real &amp; description"));
    assert!(html.contains("href=\"/articles/article-id\""));
    assert!(!html.contains("<strong>https://"));
    assert!(html.contains("Article not found in the article list"));
    assert!(render("text", &source, &data).unwrap().is_none());
    let document =
        format!("[ordinary](https://example.com/ordinary)\n\n```embed:article\n{source}\n```");
    assert_eq!(
        article_urls(&document).unwrap(),
        [url, "https://example.com/unavailable"]
    );
    assert!(article_ids(&document).unwrap().is_empty());
}

#[test]
fn article_url_lists_reject_mixed_and_unsafe_entries() {
    for source in [
        "",
        "javascript:alert(1)",
        "https://user:pass@example.com",
        "https://example.com prose",
        "https://example.com\nid: mixed",
    ] {
        assert!(
            render(ARTICLE, source, &Data::default()).is_err(),
            "{source}"
        );
    }
    assert!(
        render(
            ARTICLE,
            &vec!["https://example.com"; 51].join("\n"),
            &Data::default()
        )
        .is_err()
    );
}

#[test]
fn aligned_articles_keep_metadata_targets_and_list_collection() {
    let mut data = Data::default();
    data.articles.insert(
        "article-123".to_owned(),
        ArticleMetadata {
            href: Some("/articles/article-123".to_owned()),
            title: "Known article".to_owned(),
            description: "Summary".to_owned(),
        },
    );
    for align in ["left", "right", "wide", "narrow"] {
        let source = format!("align: {align}\nid: article-123");
        let html = render(ARTICLE, &source, &data).unwrap().unwrap();
        assert!(html.contains(&format!("content-embed-{align}\"")));
        assert!(html.contains("href=\"/articles/article-123\""));
        assert_eq!(
            article_ids(&format!("```embed:article\n{source}\n```")).unwrap(),
            ["article-123"]
        );
        let source =
            format!("align: \"{align}\"\nhttps://example.com/story\n- https://example.com/second");
        let html = render(ARTICLE, &source, &data).unwrap().unwrap();
        assert!(html.starts_with(&format!(
            "<ul class=\"content-article-list content-embed-{align}\">"
        )));
        assert_eq!(html.matches("<li>").count(), 2);
        let document = format!("```embed:article\n{source}\n```");
        assert_eq!(
            article_urls(&document).unwrap(),
            ["https://example.com/story", "https://example.com/second"]
        );
        assert!(article_ids(&document).unwrap().is_empty());
        for kind in ["audio", "video"] {
            let html = render(
                MEDIA,
                &format!("type: {kind}\nsrc: https://example.com/media\nalign: {align}"),
                &data,
            )
            .unwrap()
            .unwrap();
            assert!(html.contains(&format!("content-embed-{align}")));
        }
    }
    for source in [
        "align: narrow",
        "align: center\nhttps://example.com",
        "align: narrow\nalign: left\nhttps://example.com",
        "id: article-123\nalign: center",
    ] {
        assert!(render(ARTICLE, source, &data).is_err(), "{source}");
    }
}

#[test]
fn embed_styles_never_escape_into_visible_document_text() {
    let mut html = render(ARTICLE, "id: article-123\nalign: narrow", &Data::default())
        .unwrap()
        .unwrap();
    let body = html.clone();
    add_styles(&mut html);
    let (styles, after) = html.split_once("</style>").unwrap();
    assert!(styles.contains(".content-embed.content-embed-narrow"));
    assert_eq!(after.trim(), body.trim());
}

#[test]
fn manual_article_metadata_still_requires_index_resolution() {
    let source = "```embed:article\nid: article-123\ntitle: Custom\ndescription: Summary\n```";
    assert_eq!(article_ids(source).unwrap(), ["article-123"]);
}

#[test]
fn discovers_article_fences_inside_footnotes() {
    let source = "Text[^note]\n\n[^note]:\n    ```embed:article\n    https://knowledge.you-find.me/articles/example\n    ```\n";
    assert_eq!(
        super::article_urls(source).unwrap(),
        ["https://knowledge.you-find.me/articles/example"]
    );
}

#[test]
fn discovers_media_assets_inside_footnotes() {
    let source = "Text[^note]\n\n[^note]:\n    ```embed:media\n    type: audio\n    src: ../recording.mp3\n    ```\n";
    assert_eq!(
        super::collect_media_paths(source).unwrap(),
        ["../recording.mp3"]
    );
}

#[test]
fn rejects_invalid_fences_before_provider_reads() {
    use futures_util::FutureExt;
    for source in [
        "```embed:github\nrepo: owner/repo\nalign: invalid\n```",
        "```embed:github\nrepo: owner/repo\nextra: invalid\n```",
        "```embed:stock\ncode: MSFT\nextra: invalid\n```",
        "```embed:stock\ncode: MSFT\nalign: invalid\n```",
        "```embed:unknown\n```",
    ] {
        let result = super::load(source)
            .now_or_never()
            .expect("validation must finish before I/O");
        assert!(matches!(
            result,
            Err(EmbedError::InvalidAlignment(_))
                | Err(EmbedError::UnknownField { .. })
                | Err(EmbedError::UnsupportedKind(_))
        ));
    }
    let source =
        "```embed:github\nrepo: owner/repo\n```\n\n```embed:architecture\nnot a diagram\n```";
    assert!(
        super::load(source)
            .now_or_never()
            .expect("validate the whole document first")
            .is_err()
    );
}

#[test]
fn document_embed_contract_matches_knowledge() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/document-embeds.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let kind = case["kind"].as_str().unwrap();
        let source = case["source"].as_str().unwrap();
        let valid = case["valid"].as_bool().unwrap();
        let rendered = super::render(kind, source, &super::Data::default());
        assert_eq!(rendered.is_ok(), valid, "{kind}: {source}: {rendered:?}");
        if valid {
            let html = rendered.unwrap().unwrap();
            assert!(!html.contains("<script>"));
            assert!(html.contains("content-embed-"));
        }
    }
}

#[test]
fn quote_and_diff_preserve_plain_text() {
    let quote = super::render(
        "embed:quote",
        "author: A & B\nurl: https://example.com\n---\nFirst\n\n<script>text</script>",
        &super::Data::default(),
    )
    .unwrap()
    .unwrap();
    assert!(quote.contains("First\n\n&lt;script&gt;text&lt;/script&gt;"));
    assert!(quote.contains("cite=\"https://example.com/\""));
    let diff = super::render(
        "embed:diff",
        "title: Markers\n---\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n--- old\n+++ new",
        &super::Data::default(),
    )
    .unwrap()
    .unwrap();
    assert!(diff.contains("class=\"diff-remove\">--- old"));
    assert!(diff.contains("class=\"diff-add\">+++ new"));
}

#[test]
fn annotation_preserves_text_and_rejects_ambiguous_marks() {
    let source = "mark: 内容优先\nnote: <说明>\ncolor: red\nurl: https://example.com/source\n---\n我的博客坚持内容优先。";
    let html = render("embed:annotation", source, &Data::default())
        .unwrap()
        .unwrap();
    assert!(html.contains("我的博客坚持<mark>内容优先</mark>。"));
    assert!(html.contains("&lt;说明&gt;</a>"));
    for source in [
        "mark: aa\nnote: note\n---\naaa",
        "mark: missing\nnote: note\n---\ntext",
        "mark: a\nnote: note\ncolor: pink\n---\na",
        "mark: a\nnote: note\nurl: javascript:alert(1)\n---\na",
        "mark: a\nnote: note\nurl: https://user:pass@example.com\n---\na",
        "mark: a\nnote: note\nnote: duplicate\n---\na",
    ] {
        assert!(
            render("embed:annotation", source, &Data::default()).is_err(),
            "{source}"
        );
    }
}

#[test]
fn semantic_canvases_validate_like_knowledge() {
    for source in [
        "flowchart LR\na --> b --> c",
        "flowchart LR\na[x[y]] --> b",
        "flowchart LR\na] --> b",
    ] {
        assert!(render("embed:architecture", source, &Data::default()).is_err());
    }
    for source in [
        "title: t\nalign: wide\nalign: left\nstep: a | b\nstep: c | d",
        "title: t\nstep: a |\nstep: c | d",
        "title: \nstep: a | b\nstep: c | d",
    ] {
        assert!(render("embed:storyboard", source, &Data::default()).is_err());
    }
    let source = "title: t\n".to_owned() + &"step: a | b\n".repeat(7);
    assert!(render("embed:storyboard", &source, &Data::default()).is_err());
    let html = render(
        "embed:storyboard",
        "title: \"Board\"\nstep: \"One | First\"\nstep: Two | Second",
        &Data::default(),
    )
    .unwrap()
    .unwrap();
    assert!(!html.contains("&quot;"));
}
