use super::{compile_knowledge, knowledge_body, render, render_memo, render_publication};

#[test]
fn renders_extensions() {
    let html = render("# Title\n\n~~old~~\n\n| A | B |\n| - | - |\n| 1 | 2 |");
    assert!(html.contains("<h1>Title</h1>"));
    assert!(html.contains("<del>old</del>"));
    assert!(html.contains("<table>"));
}

#[test]
fn renders_memo_breaks() {
    assert_eq!(render_memo("first\nsecond"), "<p>first<br />\nsecond</p>\n");
}

#[test]
fn autolinks_memo_text() {
    let html = render_memo(
        "Open https://memos.you-find.me/memo/20260817T054032Z-73496a1d.\n\n`https://example.com/code`\n\n[site](https://example.com)",
    );

    assert!(html.contains(
        "<a href=\"https://memos.you-find.me/memo/20260817T054032Z-73496a1d\">https://memos.you-find.me/memo/20260817T054032Z-73496a1d</a>."
    ));
    assert!(html.contains("<code>https://example.com/code</code>"));
    assert!(html.contains("<a href=\"https://example.com\">site</a>"));
}

#[test]
fn escapes_raw_html() {
    let source = "before <script>alert('x')</script> after\n\n<div>block</div>";
    let html = render(source);

    assert!(html.contains("&lt;script&gt;alert('x')&lt;/script&gt;"));
    assert!(html.contains("&lt;div&gt;block&lt;/div&gt;"));
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<div>block</div>"));
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
fn compiles_headings() {
    let output = compile_knowledge("# Overview\n\nText\n\n## Details\n\nMore");
    assert!(output.html.contains("<h1 id=\"overview\">Overview</h1>"));
    assert!(output.html.contains("<h2 id=\"details\">Details</h2>"));
    assert_eq!(output.toc[0].id, "overview");
    assert_eq!(output.toc[1].depth, 2);
    assert_eq!(output.excerpt, "Overview Text Details More");
}

#[test]
fn aligns_metadata() {
    let output = compile_knowledge("# API <em>surface</em>\n\nUse <kbd>Enter</kbd> safely.");

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
    let output = compile_knowledge("# Repeat\n\n## Repeat\n\n### Repeat");

    assert_eq!(output.toc[0].id, "repeat");
    assert_eq!(output.toc[1].id, "repeat-2");
    assert_eq!(output.toc[2].id, "repeat-3");
    assert!(output.html.contains("<h2 id=\"repeat-2\">Repeat</h2>"));
}

#[test]
fn strips_front_matter() {
    let source = "\u{feff}---  \r\ntitle: Daily\r\nsummary: Brief\r\ntags:\r\n  - newspaper\r\n  - daily\r\n---\t\r\n\r\n## Today\r\n\r\nBriefing\r\n";
    let output = compile_knowledge(source);

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
    );

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
