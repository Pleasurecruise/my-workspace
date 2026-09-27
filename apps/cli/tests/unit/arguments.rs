use super::*;

#[test]
fn preserves_feature_arguments_and_normalizes_options() {
    let cases = [
        (
            vec!["todo", "list", "--date", "2026-09-20"],
            vec!["todo", "--date", "2026-09-20", "list"],
        ),
        (
            vec!["memo", "create", "hello", "world"],
            vec!["memo", "create", "hello world"],
        ),
        (
            vec!["memo", "update", "id", "--file", "note.md"],
            vec!["memo", "update", "id", "--file", "note.md"],
        ),
        (
            vec!["memo", "create", "--", "--help"],
            vec!["memo", "create", "--", "--help"],
        ),
        (
            vec![
                "photo",
                "upload",
                "photo.heic",
                "--metadata",
                "metadata.json",
            ],
            vec!["photo", "upload", "--file", "metadata.json", "photo.heic"],
        ),
        (
            vec!["photo", "object", "put", "moment/original/a.png", "a.png"],
            vec!["photo", "object-put", "moment/original/a.png", "a.png"],
        ),
    ];
    for (input, expected) in cases {
        let parsed = parse(
            std::iter::once("vesper")
                .chain(input.clone())
                .map(OsString::from),
        )
        .unwrap_or_else(|error| panic!("{input:?}: {error}"));
        assert_eq!(parsed, expected, "{input:?}");
    }
}

#[test]
fn knowledge_delete_requires_and_preserves_both_version_fields() {
    let input = [
        "vesper",
        "knowledge",
        "delete",
        "019c1234-1234-7000-8000-123456789abc",
        "hash",
        "2026-09-20T11:00:00.000Z",
    ];
    assert_eq!(parse(input.map(OsString::from)).unwrap(), input[1..]);
    let error = parse(input[..5].iter().map(OsString::from)).unwrap_err();
    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
}

#[test]
fn photo_upload_and_filters_use_existing_inputs() {
    for (args, expected) in [
        (
            vec![
                "photo",
                "upload",
                "photo.heic",
                "--title",
                "Weekend",
                "--tag",
                "walk",
                "--tag",
                "city",
            ],
            serde_json::json!({"title":"Weekend", "description":null, "date":null, "tags":["walk","city"]}),
        ),
        (
            vec!["photo", "list", "--tag", "walk", "--from", "2026-09-01"],
            serde_json::json!({"limit":20,"tags":["walk"],"fromDate":"2026-09-01"}),
        ),
        (
            vec![
                "memo",
                "list",
                "--limit",
                "5",
                "--cursor",
                "next",
                "--favorites",
            ],
            serde_json::json!({"limit":5,"cursor":"next","favoritesOnly":true}),
        ),
        (
            vec![
                "knowledge",
                "list",
                "--tag",
                "rust",
                "--visibility",
                "private",
            ],
            serde_json::json!({"limit":20,"tags":["rust"],"visibility":"private"}),
        ),
    ] {
        let parsed = parse(std::iter::once("vesper").chain(args).map(OsString::from)).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&parsed[2]).unwrap(),
            expected
        );
    }
    for args in [
        vec![
            "photo",
            "upload",
            "photo.heic",
            "--metadata",
            "metadata.json",
            "--title",
            "conflict",
        ],
        vec!["photo", "list", "--search", "cat", "--tag", "animal"],
        vec!["memo", "list", "--archived", "--favorites"],
        vec!["memo", "list", "--limit", "26"],
    ] {
        assert!(parse(std::iter::once("vesper").chain(args).map(OsString::from)).is_err());
    }
}
