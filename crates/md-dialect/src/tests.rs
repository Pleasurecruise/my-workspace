use super::{compile_knowledge, compile_knowledge_plain, knowledge_body, render_publication};

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
    assert!(bad_alignment.contains("expected `left`, `right`, or `wide`"));
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
