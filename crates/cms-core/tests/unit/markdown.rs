use super::{equivalent, render, render_memo};

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
fn blocks_unsafe_destinations() {
    let html = render_memo(
        "[script](JaVaScRiPt:alert(1)) ![payload](data:image/svg+xml,unsafe) [mail](mailto:me@example.com) [local](/memo/1)",
    );

    assert!(!html.contains("javascript:"));
    assert!(!html.contains("data:image"));
    assert!(html.contains("<a href=\"#\">script</a>"));
    assert!(html.contains("<img src=\"\" alt=\"payload\" />"));
    assert!(html.contains("href=\"mailto:me@example.com\""));
    assert!(html.contains("href=\"/memo/1\""));
}

#[test]
fn rich_roundtrip() {
    assert!(equivalent(
        "* **Hello**\n* World\n",
        "- __Hello__\n- World\n\n"
    ));
    assert!(equivalent("Title\n=====\n", "# Title\n"));
    for (source, candidate) in [
        ("$x$", r"\$x\$"),
        ("[[hello]]", r"\[\[hello\]\]"),
        ("![cover](https://example.com/a.png)", ""),
        ("| A | B |\n| - | - |\n| 1 | 2 |", "A B\n1 2"),
        (
            "```embed:link\nurl: https://example.com\n```",
            "```\nurl: https://example.com\n```",
        ),
        ("```rust\nlet a = 1;\n```", "```rust\nlet a = 2;\n```"),
        ("<iframe src=\"https://example.com\"></iframe>", ""),
    ] {
        assert!(!equivalent(source, candidate), "{source}");
    }
}
