use super::{compile_knowledge, knowledge_body, render, render_memo};

#[test]
fn renders_common_extensions() {
    let html = render("# Title\n\n~~old~~\n\n| A | B |\n| - | - |\n| 1 | 2 |");
    assert!(html.contains("<h1>Title</h1>"));
    assert!(html.contains("<del>old</del>"));
    assert!(html.contains("<table>"));
}

#[test]
fn renders_memo_soft_breaks_as_line_breaks() {
    assert_eq!(render_memo("first\nsecond"), "<p>first<br />\nsecond</p>\n");
}

#[test]
fn autolinks_bare_memo_urls_without_touching_code_or_existing_links() {
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
fn renders_raw_html_as_literal_text() {
    let source = "before <script>alert('x')</script> after\n\n<div>block</div>";
    let html = render(source);

    assert!(html.contains("&lt;script&gt;alert('x')&lt;/script&gt;"));
    assert!(html.contains("&lt;div&gt;block&lt;/div&gt;"));
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<div>block</div>"));
}

#[test]
fn compiles_headings_for_the_knowledge_reader() {
    let output = compile_knowledge("# Overview\n\nText\n\n## Details\n\nMore");
    assert!(output.html.contains("<h1 id=\"overview\">Overview</h1>"));
    assert!(output.html.contains("<h2 id=\"details\">Details</h2>"));
    assert_eq!(output.toc[0].id, "overview");
    assert_eq!(output.toc[1].depth, 2);
    assert_eq!(output.excerpt, "Overview Text Details More");
}

#[test]
fn keeps_knowledge_metadata_aligned_with_literal_html() {
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
fn de_duplicates_knowledge_heading_ids() {
    let output = compile_knowledge("# Repeat\n\n## Repeat\n\n### Repeat");

    assert_eq!(output.toc[0].id, "repeat");
    assert_eq!(output.toc[1].id, "repeat-2");
    assert_eq!(output.toc[2].id, "repeat-3");
    assert!(output.html.contains("<h2 id=\"repeat-2\">Repeat</h2>"));
}

#[test]
fn removes_front_matter_from_knowledge_content() {
    let source = "---\ntitle: Daily\ntags:\n  - newspaper\n  - daily\n---\n## Today\n\nBriefing";
    let output = compile_knowledge(source);

    assert_eq!(knowledge_body(source), "## Today\n\nBriefing");
    assert!(!output.html.contains("title: Daily"));
    assert!(output.html.contains("<h2 id=\"today\">Today</h2>"));
    assert_eq!(output.excerpt, "Today Briefing");
}

#[test]
fn keeps_knowledge_without_complete_front_matter_unchanged() {
    assert_eq!(knowledge_body("---\nunfinished"), "---\nunfinished");
    assert_eq!(knowledge_body("# Article"), "# Article");
}
