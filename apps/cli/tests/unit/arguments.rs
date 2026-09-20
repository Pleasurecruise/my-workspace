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
            vec!["moment", "upload-photo", "{}", "photo.heic"],
            vec!["moment", "upload-photo", "{}", "photo.heic"],
        ),
        (
            vec![
                "moment",
                "upload-photo",
                "--file",
                "metadata.json",
                "photo.heic",
            ],
            vec![
                "moment",
                "upload-photo",
                "--file",
                "metadata.json",
                "photo.heic",
            ],
        ),
        (
            vec!["moment", "upload-photo", "--stdin", "photo.heic"],
            vec!["moment", "upload-photo", "--stdin", "photo.heic"],
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
