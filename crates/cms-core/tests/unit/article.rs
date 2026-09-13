use super::*;

fn render_publication(source: &str) -> Result<String, PublicationError> {
    render_publication_with(source, &EmbedData::default())
}

fn compile_knowledge(source: &str) -> Result<CompiledKnowledge, EmbedError> {
    compile_knowledge_with(source, &EmbedData::default())
}

#[test]
fn highlights_code() {
    let html = render_publication("```rust\nfn main() {}\n```").unwrap();

    assert!(html.contains("class=\"highlighted-code\""));
    assert!(html.contains("<pre style=\"background-color:"));
    assert!(html.contains("<span style=\"color:"));
    assert!(html.contains("main"));
    assert!(!html.contains("language-rust"));
}

#[test]
fn renders_mermaid() {
    let html = render_publication("```mermaid\nflowchart LR\n  A[Start] --> B[End]\n```").unwrap();

    assert!(html.contains("<figure class=\"mermaid-diagram\"><svg"));
    assert!(html.contains("Start"));
    assert!(html.contains("End"));
    assert!(!html.contains("language-mermaid"));
}

#[test]
fn rejects_bad_mermaid() {
    assert!(render_publication("```mermaid\nnot-a-diagram\n```").is_err());
}

#[test]
fn renders_svg_canvases_with_distinct_profiles() {
    let html = render_publication(
        r#"```embed:architecture
align: left
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 120" role="img">
<title>Request path</title><desc>A request reaches the Rust service.</desc>
<g class="node c-teal"><rect x="10" y="20" width="120" height="60" rx="10"/><text class="th" x="24" y="54">Svelte</text></g>
<path class="arr" d="M130 50 L190 50"/>
</svg>
```

```embed:storyboard
align: right
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 120" role="img">
<title>Draft sequence</title><desc>Two hand-drawn notes connected by an arrow.</desc>
<path class="note" d="M10 15 L130 13 L132 92 L8 94 Z"/>
<path class="sketch-shadow" d="M10 15 L130 13 L132 92 L8 94 Z"/>
<path class="arrow" d="M140 55 C165 42 180 68 205 52"/>
<text class="hand title" x="24" y="46">Draft</text>
</svg>
```"#,
    )
    .unwrap();

    assert_eq!(html.matches("data-md-dialect=\"embeds\"").count(), 1);
    assert!(html.contains("svg-canvas-architecture content-embed-left"));
    assert!(html.contains("svg-canvas-storyboard content-embed-right"));
    assert!(html.contains("class=\"node c-teal\""));
    assert!(html.contains("class=\"sketch-shadow\""));
    assert!(!html.contains("language-embed"));
}

#[test]
fn upgrades_structured_diagrams_to_svg_canvases() {
    let html = render_publication(
        "```embed:architecture\nalign: wide\nflowchart LR\nClient --> API\nAPI --> Database\n```\n\n```embed:storyboard\ntitle: 发布流程\nstep: 编写 | 完成 Markdown 内容\nstep: 构建 | 编译并验证内容\nstep: 发布 | 上传生成的产物\n```",
    )
    .unwrap();

    assert!(html.contains("svg-canvas-architecture content-embed-wide"));
    assert!(html.contains("class=\"node c-teal\""));
    assert!(html.contains(">Client</text>"));
    assert!(html.contains("svg-canvas-storyboard content-embed-wide"));
    assert!(html.contains("class=\"sketch-shadow\""));
    assert!(html.contains("class=\"arrow-shadow\""));
    assert!(html.contains("class=\"arrow\""));
    assert!(html.contains(">发布流程</title>"));
}

#[test]
fn rejects_invalid_content_embeds() {
    let unknown = render_publication("```embed:video\nurl: https://example.com\n```")
        .unwrap_err()
        .to_string();
    assert!(unknown.contains("unsupported embed kind `embed:video`"));

    let bad_repository = render_publication("```embed:github\nrepo: missing-owner\n```")
        .unwrap_err()
        .to_string();
    assert!(bad_repository.contains("expected `owner/name`"));

    let bad_field = render_publication("```embed:stock\nticker: AAPL\n```")
        .unwrap_err()
        .to_string();
    assert!(bad_field.contains("does not support field `ticker`"));

    let bad_alignment = render_publication(
        "```embed:architecture\nalign: center\n<svg xmlns=\"http://www.w3.org/2000/svg\"><title>A</title><desc>B</desc></svg>\n```",
    )
    .unwrap_err()
    .to_string();
    assert!(bad_alignment.contains("expected `left`, `right`, `wide`, or `narrow`"));
}

#[test]
fn compiles_headings() {
    let output = compile_knowledge("# Overview\n\nText\n\n## Details\n\nMore").unwrap();
    assert!(output.html.contains("<h1 id=\"overview\">Overview</h1>"));
    assert!(output.html.contains("<h2 id=\"details\">Details</h2>"));
    assert_eq!(output.toc[0].id, "overview");
    assert_eq!(output.toc[1].depth, 2);
    assert_eq!(output.excerpt, "Overview Text Details More");
}

#[test]
fn aligns_metadata() {
    let output =
        compile_knowledge("# API <em>surface</em>\n\nUse <kbd>Enter</kbd> safely.").unwrap();

    assert_eq!(output.toc[0].id, "api-em-surface-em");
    assert_eq!(output.toc[0].text, "API <em>surface</em>");
    assert!(
        output
            .html
            .contains("<h1 id=\"api-em-surface-em\">API &lt;em&gt;surface&lt;/em&gt;</h1>")
    );
    assert!(
        output
            .html
            .contains("Use &lt;kbd&gt;Enter&lt;/kbd&gt; safely.")
    );
    assert_eq!(
        output.excerpt,
        "API <em>surface</em> Use <kbd>Enter</kbd> safely."
    );
}

#[test]
fn deduplicates_headings() {
    let output = compile_knowledge("# Repeat\n\n## Repeat\n\n### Repeat").unwrap();

    assert_eq!(output.toc[0].id, "repeat");
    assert_eq!(output.toc[1].id, "repeat-2");
    assert_eq!(output.toc[2].id, "repeat-3");
    assert!(output.html.contains("<h2 id=\"repeat-2\">Repeat</h2>"));
}

#[test]
fn strips_front_matter() {
    let source = "\u{feff}---  \r\ntitle: Daily\r\nsummary: Brief\r\ntags:\r\n  - newspaper\r\n  - daily\r\n---\t\r\n\r\n## Today\r\n\r\nBriefing\r\n";
    let output = compile_knowledge(source).unwrap();

    assert_eq!(knowledge_body(source), "## Today\r\n\r\nBriefing");
    assert!(!output.html.contains("title: Daily"));
    assert!(output.html.contains("<h2 id=\"today\">Today</h2>"));
    assert_eq!(output.excerpt, "Today Briefing");
}

#[test]
fn preserves_indented_body_after_front_matter() {
    let source = "---\ntitle: Example\n---\n\n    let value = 1;  \n";

    assert_eq!(knowledge_body(source), "    let value = 1;  ");
    assert!(
        compile_knowledge(source)
            .unwrap()
            .html
            .contains("<pre><code>let value = 1;")
    );
}

#[test]
fn keeps_bad_front_matter() {
    assert_eq!(knowledge_body("---\nunfinished"), "---\nunfinished");
    assert_eq!(knowledge_body("# Article"), "# Article");
}

#[test]
fn renders_knowledge() {
    let output = compile_knowledge(
        "> [!NOTE]\n> Keep the boundary explicit.\n\nRead [[target-article|the source]] and preserve $x^2$.",
    )
    .unwrap();

    assert!(
        output
            .html
            .contains("<blockquote class=\"markdown-alert-note\">")
    );
    assert!(!output.html.contains("[!NOTE]"));
    assert!(
        output
            .html
            .contains("<a href=\"/articles/target-article\">the source</a>")
    );
    assert!(
        output
            .html
            .contains("<span class=\"math math-inline\">x^2</span>")
    );
}

#[test]
fn compiles_canvas_in_knowledge_without_polluting_excerpt() {
    let output = compile_knowledge(
        "# Design\n\nExplain the flow.\n\n```embed:storyboard\nalign: left\n<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 50\"><title>Flow</title><desc>A hand-drawn flow.</desc><path class=\"scribble\" d=\"M0 25 C20 20 40 30 60 25\"/></svg>\n```",
    )
    .unwrap();

    assert!(output.html.contains("content-embed-left"));
    assert!(output.html.contains("svg-canvas-storyboard"));
    assert_eq!(output.excerpt, "Design Explain the flow.");
}

#[test]
fn plain_compilation_preserves_content_embeds_as_code() {
    let output =
        compile_knowledge_plain("# Article\n\n```embed:github\nrepo: owner/repository\n```");

    assert!(output.html.contains("language-embed:github"));
    assert!(output.html.contains("repo: owner/repository"));
    assert!(!output.html.contains("content-embed-github"));
    assert_eq!(output.excerpt, "Article repo: owner/repository");
}

#[test]
fn blocks_unsafe_destinations() {
    let output = compile_knowledge_plain(
        "[script](JaVaScRiPt:alert(1)) ![payload](data:image/svg+xml,unsafe) [mail](mailto:me@example.com) [local](/articles/one)",
    );

    assert!(!output.html.contains("javascript:"));
    assert!(!output.html.contains("data:image"));
    assert!(output.html.contains("<a href=\"#\">script</a>"));
    assert!(output.html.contains("<img src=\"\" alt=\"payload\" />"));
    assert!(output.html.contains("href=\"mailto:me@example.com\""));
    assert!(output.html.contains("href=\"/articles/one\""));
}

#[tokio::test]
async fn compiles_media_in_publication_and_knowledge() {
    let source = "# Listen\n\nBefore.\n\n```embed:media\ntype: audio\nsrc: https://example.com/clip.mp3\ncaption: A short recording\n```\n\nAfter.";
    let publication = render_publication_enriched(source).await.unwrap();
    let knowledge = compile_knowledge_enriched(source).await.unwrap();
    for html in [&publication, &knowledge.html] {
        assert!(html.contains("<audio controls preload=\"none\""));
        assert!(html.contains("<figcaption>A short recording</figcaption>"));
        assert!(html.contains("<p>Before.</p>"));
        assert!(html.contains("<p>After.</p>"));
        assert_eq!(html.matches("<style data-md-dialect").count(), 1);
    }
    assert_eq!(knowledge.toc.len(), 1);
    assert!(!knowledge.excerpt.contains("type: audio"));
    assert!(
        compile_knowledge_plain(source)
            .html
            .contains("language-embed:media")
    );
}

#[test]
fn heading_suffixes_do_not_collide_with_authored_headings() {
    let output = compile_knowledge("# Repeat\n\n# Repeat\n\n# Repeat-2\n\n# Repeat").unwrap();
    let ids: std::collections::HashSet<_> = output.toc.iter().map(|entry| &entry.id).collect();
    assert_eq!(ids.len(), output.toc.len());
}

#[tokio::test]
async fn ignores_embeds_in_knowledge_front_matter() {
    let output = compile_knowledge_enriched(
        "---\n```embed:media\ntype: invalid\nsrc: https://example.com/a\n```\n---\n\n# Visible",
    )
    .await
    .unwrap();
    assert_eq!(output.excerpt, "Visible");
}

#[tokio::test]
async fn unindexed_article_shortcuts_are_disabled_in_both_hosts() {
    let source = "# Reading\n\n- First\n\n  ```embed:article\n  id: first-article\n  title: First article\n  ```\n\n- Second\n\n  ```embed:article\n  url: https://example.com/story\n  title: External story\n  description: External summary\n  ```";
    let publication = render_publication_enriched(source).await.unwrap();
    let knowledge = compile_knowledge_enriched(source).await.unwrap();
    for html in [&publication, &knowledge.html] {
        assert!(!html.contains("href="));
        assert_eq!(html.matches("aria-disabled=\"true\"").count(), 2);
        assert_eq!(html.matches("<style data-md-dialect").count(), 1);
        assert_eq!(html.matches("<li>").count(), 2);
        assert!(!html.contains("language-embed:article"));
    }
    assert!(
        compile_knowledge_plain(source)
            .html
            .contains("language-embed:article")
    );
}

#[tokio::test]
async fn knowledge_uses_authorized_article_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert(
        "article-123".to_owned(),
        md_dialect::ArticleMetadata {
            href: None,
            title: "Resolved article title".to_owned(),
            description: "Resolved article summary".to_owned(),
        },
    );
    let compiled =
        compile_knowledge_with_articles("```embed:article\nid: article-123\n```", metadata)
            .await
            .unwrap();
    assert!(compiled.html.contains("Resolved article title"));
    assert!(compiled.html.contains("Resolved article summary"));
    assert!(compiled.html.contains("href=\"/articles/article-123\""));
}

#[tokio::test]
async fn compiles_url_list_cards_with_host_metadata_and_in_app_routes() {
    let url = "https://knowledge.you-find.me/articles/real-slug";
    let metadata = std::collections::HashMap::from([(
        url.to_owned(),
        md_dialect::ArticleMetadata {
            href: Some("/articles/real-id".to_owned()),
            title: "Actual title".to_owned(),
            description: "Actual description".to_owned(),
        },
    )]);
    let source = format!("[ordinary]({url})\n\n```embed:article\n{url}\n```");
    let compiled = compile_knowledge_with_articles(&source, metadata)
        .await
        .unwrap();
    assert!(compiled.html.contains("<strong>Actual title</strong>"));
    assert!(compiled.html.contains("Actual description"));
    assert!(compiled.html.contains("href=\"/articles/real-id\""));
    assert!(
        compiled
            .html
            .contains(&format!("href=\"{url}\">ordinary</a>"))
    );
    assert!(!compiled.html.contains("target=\"_blank\""));
}

#[tokio::test]
async fn article_loading_never_fetches_unknown_websites() {
    let source = "```embed:article\nhttps://127.0.0.1:1/private\n```\n\n```embed:article\nurl: https://example.invalid/story\n```";
    let data = md_dialect::load_embeds(source).await.unwrap();
    assert!(data.articles.is_empty());
}

#[test]
fn reading_statistics_count_prose_without_markdown_configuration() {
    let source = "---\ntitle: metadata not counted\n---\n# 标题 Hello\n\n正文 **world** [链接](https://example.com/long/path)\n\nhttps://example.com/bare/url\n\n![Image description](https://example.com/image.png)\n\n`inline code` $x+y$\n\n```rust\nlet code = 123;\n```\n\n```embed:media\ntype: audio\nsrc: https://example.com/audio.mp3\ncaption: hidden configuration\n```";
    let compiled = compile_knowledge_plain(source);
    assert_eq!(compiled.stats.word_count, 8);
    assert_eq!(compiled.stats.reading_minutes, 1);
    let plain = compile_knowledge_plain("co**op**erate café naïve don't 中文 e\u{301}cole");
    assert_eq!(plain.stats.word_count, 7);
}

#[tokio::test]
async fn reading_statistics_survive_embed_fallback() {
    let source = "# Example\n\n中文 text\n\n```embed:media\ntype: audio\nsrc: https://example.com/audio.mp3\n```";
    let enriched = compile_knowledge_enriched(source).await.unwrap();
    let plain = compile_knowledge_plain(source);
    assert_eq!(enriched.stats.word_count, 4);
    assert_eq!(plain.stats.word_count, enriched.stats.word_count);
    assert_eq!(plain.stats.reading_minutes, enriched.stats.reading_minutes);
}

#[test]
fn reading_time_uses_mixed_language_rates() {
    let source = format!("{} {}", "字".repeat(350), "word ".repeat(200));
    let compiled = compile_knowledge_plain(&source);
    assert_eq!(compiled.stats.word_count, 550);
    assert_eq!(compiled.stats.reading_minutes, 2);
    let minute = format!("{} {}", "字".repeat(175), "word ".repeat(100));
    assert_eq!(compile_knowledge_plain(&minute).stats.reading_minutes, 1);
    let longer = format!("字{minute}");
    assert_eq!(compile_knowledge_plain(&longer).stats.reading_minutes, 2);
    assert_eq!(compile_knowledge_plain("").stats.word_count, 0);
}

#[tokio::test]
async fn document_embeds_keep_fence_newlines_consistent() {
    let patch = "title: Changes\n---\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-<old>\n+<new>";
    let source = format!(
        "~~~embed:quote\nauthor: A & B\nurl: https://example.com\n---\nFirst\n\n<script>plain text</script>\n\n~~~\n\n~~~embed:diff\n{patch}\n~~~"
    );
    let knowledge = super::compile_knowledge_enriched(&source).await.unwrap();
    let publication = super::render_publication_enriched(&source).await.unwrap();
    for html in [&knowledge.html, &publication] {
        assert!(html.contains("First\n\n&lt;script&gt;plain text&lt;/script&gt;\n</p>"));
        assert!(html.contains("class=\"diff-remove\">-&lt;old&gt;\n</span>"));
        assert!(html.contains("class=\"diff-add\">+&lt;new&gt;\n</span>"));
    }
    let invalid = format!("~~~embed:diff\n{patch}\n\n~~~");
    assert!(super::compile_knowledge_enriched(&invalid).await.is_err());
    assert!(super::render_publication_enriched(&invalid).await.is_err());
}

#[tokio::test]
async fn bare_embed_examples_do_not_swallow_following_live_blocks() {
    let source = "```\n```embed:github\nrepo: example/source-only\n```\n```\n\n```embed:annotation\nmark: text\nnote: note\n---\nLive text.\n```";
    let knowledge = compile_knowledge_enriched(source).await.unwrap();
    assert!(knowledge.html.contains("language-markdown"));
    assert!(knowledge.html.contains("```embed:github"));
    assert!(knowledge.html.contains("Live <mark>text</mark>."));
    let publication = render_publication_enriched(source).await.unwrap();
    assert!(publication.contains("Live <mark>text</mark>."));
    assert!(
        md_dialect::article_urls(
            "```\n```embed:article\nhttps://knowledge.you-find.me/articles/example\n```\n```"
        )
        .unwrap()
        .is_empty()
    );
}
