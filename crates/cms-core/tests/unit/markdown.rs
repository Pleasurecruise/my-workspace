use super::{equivalent, render, render_memo};

#[test]
fn milkdown_roundtrip_corpus() {
    #[derive(serde::Deserialize)]
    struct Fixture {
        name: String,
        source: String,
        candidate: String,
        protected: Vec<String>,
        ranges: serde_json::Value,
    }
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("../fixtures/editor.json")).unwrap();
    for fixture in fixtures {
        assert!(
            equivalent(&fixture.source, &fixture.candidate),
            "{}",
            fixture.name
        );
        let ranges = super::source_spans(&fixture.source);
        assert_eq!(
            serde_json::to_value(&ranges).unwrap(),
            fixture.ranges,
            "{}",
            fixture.name
        );
        let source: Vec<_> = fixture.source.encode_utf16().collect();
        for range in &ranges {
            let preserved = String::from_utf16(&source[range.start..range.end]).unwrap();
            assert!(
                fixture
                    .protected
                    .iter()
                    .any(|text| preserved.contains(text) || text.contains(preserved.trim())),
                "{}: {preserved:?}",
                fixture.name
            );
        }
        for protected in fixture.protected {
            let start = fixture.source.find(&protected).unwrap();
            let start = fixture.source[..start].encode_utf16().count();
            let end = start + protected.encode_utf16().count();
            assert!(
                ranges
                    .iter()
                    .any(|range| range.start < end && range.end > start),
                "{}: {protected}",
                fixture.name
            );
        }
    }
    assert!(!equivalent(
        "[link][ref]\n\n[ref]: https://example.com",
        "[link](https://different.example.com)"
    ));
    assert!(!equivalent(
        "![alt][ref]\n\n[ref]: https://example.com/image.png",
        "![changed](https://example.com/image.png)"
    ));
}

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
        (
            "[**bold** plain](https://example.com)",
            "[bold plain](https://example.com)",
        ),
        (
            "[**bold** plain](https://example.com)",
            "**[bold](https://other.example.com)** [plain](https://example.com)",
        ),
        ("[](https://example.com)", ""),
        ("[ ](https://example.com)", " "),
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

#[test]
fn image_shortcodes_reach_all_markdown_entrypoints() {
    for compiled in [
        super::render("Before :suzume5_01: after :baishengnv_117: :suzume_思考: :suzume_期待:"),
        super::render_memo(
            "Before :suzume5_01: after :baishengnv_117: :suzume_思考: :suzume_期待:",
        ),
    ] {
        assert_eq!(compiled.matches("class=\"markdown-emoji\"").count(), 4);
        assert!(compiled.contains("Before "));
        assert!(compiled.contains(" after "));
    }
}

#[test]
fn fragment_retains_document_reference_definitions() {
    let context = "$x$ and [site][ref] and [^note]\n\n[ref]: https://example.com\n\n[^note]: Footnote body\n    continuation\n";
    let fragment = super::fragment("$x$ and [site][ref] and [^note]", context);
    let html = render(&fragment);
    assert!(html.contains("href=\"https://example.com\""));
    assert!(html.contains("Footnote body"));
    assert!(html.contains("continuation"));
}

#[test]
fn fragment_omits_unreferenced_footnotes() {
    let context = "Text[^1]\n\n[^1]: Unrelated footnote";
    assert_eq!(super::fragment("$x$", context), "$x$");
}

#[test]
fn shortcodes_preserve_renderer_protection() {
    assert_eq!(super::source_spans(":suzume_思考:").len(), 1);
    assert!(super::source_spans("[:suzume_思考:](https://example.com)").is_empty());
    assert!(super::source_spans("![:suzume_思考:](https://example.com/image.png)").is_empty());
}
